// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `messages` and `read`: one conversation walked oldest first
//! ([`crate::walk`]), written out a chunk at a time as it is walked, as
//! architect-cto ruled them (01a18ecf Q5, Q6, Q8).
//!
//! `read` checks every `--keep` id against the whole conversation in the
//! walk's first pass, before anything is marked read; then each chunk is
//! marked read, its `--keep` rows kept, and it is written and flushed
//! before the next chunk is marked read, so a failure of the store leaves
//! no row read and unwritten. A page that cannot be read stops the walk
//! with the store's failure: the rows written are read, the later ones
//! stay unread, and a second `read` goes on from them. stdout failing
//! after a chunk was read -- a reader gone, a full disk -- leaves that
//! chunk read and unwritten, at most [`crate::walk::CHUNK`] rows: a row
//! is read before it is printed, so its status prints as read
//! (#519 review, round 1 risk 2).

use std::collections::HashMap;
use std::io::Write;

use interweave_human_ui_model::{Anchor, ConversationKey, Direction, ItemKey, MessageItem};
use interweave_local_client_api::{AdminBinding, DataSessionBinding};

use crate::Failure;
use crate::output;
use crate::session::Session;
use crate::verbs::{Run, keep_after_read, look};
use crate::walk::{Progress, Walk};

/// `messages`: the conversation's rows, oldest first, read by no one.
///
/// # Errors
/// [`Failure::Store`] when a page of the conversation cannot be read;
/// what was written before stays written.
pub(crate) async fn messages<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    key: &ConversationKey,
    out: &mut dyn Write,
) -> Result<(), Failure> {
    look(s, r).await;
    let mut walk = Walk::open(s, key);
    loop {
        let written = match walk.advance(s).await? {
            Progress::Working => true,
            Progress::Chunk(anchors) => write(out, &shown(s, key, &anchors), r.json)?,
            Progress::Late(items) => write(out, &items, r.json)?,
            Progress::Done => return Ok(()),
        };
        // The reader went away: nothing more is worth walking.
        if !written {
            return Ok(());
        }
    }
}

/// `read`: the conversation's rows, oldest first, each unread one marked
/// read as its chunk is written, and the `--keep` ones kept.
///
/// A row that arrives while the walk runs is written after the walk's
/// last row, as `messages` writes it, and stays unread: the reader was not
/// shown it when the read began.
///
/// # Errors
/// [`Failure::Refused`] for a `--keep` id that names no message received
/// in the conversation, before anything is read or written; a page that
/// cannot be read, [`Failure::Store`]; a read or keep the store refuses,
/// its failure, once the chunk it was in is written.
pub(crate) async fn read<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    key: &ConversationKey,
    keep: &[String],
    out: &mut dyn Write,
) -> Result<(), Failure> {
    look(s, r).await;
    let mut walk = Walk::open(s, key);
    // Every --keep names a message received in this conversation, checked
    // over the whole of it in the first pass, before anything is marked
    // read: a mistyped id must not cost the read, wherever the id stands
    // (`a_keep_past_the_held_row_ceiling_is_checked_before_anything_is_read`).
    let mut to_keep: Vec<Option<ItemKey>> = vec![None; keep.len()];
    found(s, key, keep, &mut to_keep);
    while !walk.past_the_top() {
        walk.advance(s).await?;
        found(s, key, keep, &mut to_keep);
    }
    if let Some(n) = to_keep.iter().position(Option::is_none) {
        return Err(Failure::Refused(format!(
            "--keep number {}: no message received in this conversation has that id",
            n + 1
        )));
    }
    let to_keep: HashMap<ItemKey, usize> = to_keep
        .into_iter()
        .enumerate()
        .filter_map(|(n, k)| k.map(|k| (k, n)))
        .collect();
    loop {
        match walk.advance(s).await? {
            Progress::Working => {}
            Progress::Chunk(anchors) => {
                let items: Vec<ItemKey> = anchors
                    .iter()
                    .filter_map(|a| match a {
                        Anchor::Item(k) => Some(*k),
                        Anchor::Card(_) => None,
                    })
                    .collect();
                let reads = s.model().items_viewed(&items);
                if !reads.is_empty() {
                    s.will(move |_| {
                        reads
                            .into_iter()
                            .map(interweave_human_ui_model::ViewEvent::Intent)
                            .collect()
                    });
                    s.step().await;
                    // What this step brought is the walk's: a row that
                    // arrived in it is handed over last.
                    walk.take_arrivals(s);
                }
                let keeping: Vec<(usize, ItemKey)> = items
                    .iter()
                    .filter_map(|k| to_keep.get(k).map(|n| (*n, *k)))
                    .collect();
                // From here a row may be read and not kept, which this run
                // alone holds (RETENTION.md): whatever fails after, the
                // chunk is written first, or the person never sees it
                // (`a_keep_that_fails_after_the_read_still_prints_what_was_read`).
                let kept = keep_after_read(s, &keeping).await;
                // And the keep's step, which a message from a daemon can
                // reach as well; the in-memory node delivers in the read
                // step, so no test times one into this step.
                walk.take_arrivals(s);
                if !write(out, &shown(s, key, &anchors), r.json)? {
                    return Ok(());
                }
                kept?;
            }
            Progress::Late(items) => {
                if !write(out, &items, r.json)? {
                    return Ok(());
                }
            }
            Progress::Done => return Ok(()),
        }
    }
}

/// Note each `--keep` id the model now holds as a message received in
/// `key`, by the rule the id is printed under.
fn found<B: DataSessionBinding, A: AdminBinding>(
    s: &Session<B, A>,
    key: &ConversationKey,
    keep: &[String],
    to_keep: &mut [Option<ItemKey>],
) {
    if to_keep.iter().all(Option::is_some) {
        return;
    }
    let model = s.model();
    let received: Vec<ItemKey> = model
        .messages(key)
        .into_iter()
        .filter(|m| m.direction == Direction::Inbound)
        .map(|m| m.key)
        .collect();
    for (id, slot) in keep.iter().zip(to_keep.iter_mut()) {
        if slot.is_none() {
            *slot = model.item_named(id).filter(|k| received.contains(k));
        }
    }
}

/// The messages `anchors` name, in their order: a card's row is the
/// proposals' verbs', not these.
fn shown<B: DataSessionBinding, A: AdminBinding>(
    s: &Session<B, A>,
    key: &ConversationKey,
    anchors: &[Anchor],
) -> Vec<MessageItem> {
    let mut held: HashMap<ItemKey, MessageItem> = s
        .model()
        .messages(key)
        .into_iter()
        .map(|m| (m.key, m))
        .collect();
    anchors
        .iter()
        .filter_map(|a| match a {
            Anchor::Item(k) => held.remove(k),
            Anchor::Card(_) => None,
        })
        .collect()
}

/// Write `items` and flush: `false` when the reader went away.
fn write(out: &mut dyn Write, items: &[MessageItem], json: bool) -> Result<bool, Failure> {
    let mut text = String::new();
    for m in items {
        text.push_str(&output::message(m, json));
    }
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(false),
        Err(e) => Err(Failure::Refused(format!("stdout: {e}"))),
    }
}
