// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! One conversation, walked oldest first through the windowed model, as
//! architect-cto ruled it (01a18d00, 01a18ecf): the model holds a window
//! of a conversation, not all of it, so a verb that prints a whole
//! conversation walks it in two passes.
//!
//! **Up** pages from the newest row to the top, focus held on the oldest
//! row so the held-row ceiling lets go of rows below it -- leaving gaps --
//! rather than moving the top away. It prints nothing and marks nothing
//! read: no view event is scripted while it runs.
//!
//! **Down** hands the verb the drawn rows oldest first, in chunks, from a
//! cursor that only moves forward, and fills each gap below the cursor
//! before it hands over anything past it. A row is handed over once: one
//! the ceiling let go and a gap's page brought back stands at or before
//! the cursor and is skipped. A row the drain hands over while the walk
//! runs is kept apart, wherever it draws, and handed over after the walk's
//! last row, from its own update
//! (`a_row_committed_mid_walk_prints_after_the_walks_last_row`,
//! `a_row_committed_mid_walk_drawn_before_the_cursor_prints_last`).
//!
//! The walk advances one step of the session at a time, so a verb that
//! waits on other things -- `listen` -- can interleave it with them.

use interweave_human_app_core::Update;
use interweave_human_client_api::Origin;
use interweave_human_ui_model::{
    Anchor, ConversationKey, DrawnAt, Edge, ItemKey, MarkKind, MessageItem, WindowInput,
};
use interweave_local_client_api::{AdminBinding, DataSessionBinding};

use crate::Failure;
use crate::session::Session;

/// The most rows handed over at once: the facade keeps at most
/// `READ_COPY_CAP` (256) read copies a Keep can still act on, so a verb
/// that reads a chunk, then keeps from it, stays well inside that bound;
/// and the chunk is printed and flushed before the next is read
/// (architect-cto's 01a18ecf, Q5).
pub(crate) const CHUNK: usize = 64;

/// How many advances in a row may change nothing before the walk gives
/// up: a page arrives in the step that asks it, so a walk that stands
/// still this long will not move again.
const STALL: usize = 64;

/// Which pass the walk is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pass {
    Up,
    Down,
    Done,
}

/// What one advance of the walk came to.
#[derive(Debug)]
pub(crate) enum Progress {
    /// A step taken; nothing to hand over yet.
    Working,
    /// The next rows, oldest first: each a message item or a card
    /// proposal's row, as the conversation draws them.
    Chunk(Vec<Anchor>),
    /// Rows the drain handed over during the walk: after every other row,
    /// from their own updates.
    Late(Vec<MessageItem>),
    /// The walk is over.
    Done,
}

/// One conversation's walk.
pub(crate) struct Walk {
    key: ConversationKey,
    pass: Pass,
    opening: Option<u64>,
    cursor: Option<DrawnAt>,
    late: Vec<MessageItem>,
    /// The items of `late`, left out of every chunk.
    arrived: std::collections::HashSet<ItemKey>,
    stalled: usize,
    last: Option<(Option<DrawnAt>, usize, usize, usize)>,
    /// The updates the walk's steps took, kept for a caller that prints
    /// them -- `listen` -- or `None`, dropped: a walk of a long
    /// conversation takes every page it reads.
    passed: Option<Vec<Update>>,
}

impl Walk {
    /// Open `key` and start walking it.
    pub(crate) fn open<B: DataSessionBinding, A: AdminBinding>(
        s: &mut Session<B, A>,
        key: &ConversationKey,
    ) -> Self {
        s.side().window(WindowInput::Opened(key.clone()));
        Self {
            key: key.clone(),
            pass: Pass::Up,
            opening: None,
            cursor: None,
            late: Vec::new(),
            arrived: std::collections::HashSet::new(),
            stalled: 0,
            last: None,
            passed: None,
        }
    }

    /// Open `key` and start walking it, keeping every update the walk's
    /// steps take for [`Self::take_passed`]: a verb that prints the
    /// session's events while it walks loses none of them
    /// (`listen_consume_drains_a_conversation_past_the_held_row_ceiling`).
    pub(crate) fn open_passing<B: DataSessionBinding, A: AdminBinding>(
        s: &mut Session<B, A>,
        key: &ConversationKey,
    ) -> Self {
        Self {
            passed: Some(Vec::new()),
            ..Self::open(s, key)
        }
    }

    /// The updates the walk's steps took since the last take, in order.
    pub(crate) fn take_passed(&mut self) -> Vec<Update> {
        self.passed.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// The conversation walked.
    pub(crate) const fn conversation(&self) -> &ConversationKey {
        &self.key
    }

    /// Whether the first pass is over: from here rows are handed over.
    pub(crate) const fn past_the_top(&self) -> bool {
        !matches!(self.pass, Pass::Up)
    }

    /// Walk the first pass alone: every page of the conversation read, to
    /// its top, nothing handed over and nothing read. The model holds an
    /// open conversation's proposals whatever the held-row ceiling lets go
    /// of, so they are all held once it returns: a card past the ceiling
    /// from either end is found
    /// (`proposals_read_finds_a_card_older_than_the_held_row_ceiling`).
    ///
    /// # Errors
    /// As [`Self::advance`].
    pub(crate) async fn up_to_the_top<B: DataSessionBinding, A: AdminBinding>(
        &mut self,
        s: &mut Session<B, A>,
    ) -> Result<(), Failure> {
        while !self.past_the_top() {
            self.advance(s).await?;
        }
        Ok(())
    }

    /// Close the conversation: the rows it held are let go, and stay in
    /// the store.
    pub(crate) fn close<B: DataSessionBinding, A: AdminBinding>(self, s: &mut Session<B, A>) {
        s.side().window(WindowInput::Closed(self.key));
    }

    /// Take one step of the walk.
    ///
    /// # Errors
    /// [`Failure::Store`] when a page of the conversation cannot be read,
    /// naming the conversation; nothing of the pass that failed has been
    /// handed over by then.
    pub(crate) async fn advance<B: DataSessionBinding, A: AdminBinding>(
        &mut self,
        s: &mut Session<B, A>,
    ) -> Result<Progress, Failure> {
        match self.pass {
            Pass::Up => self.up(s).await,
            Pass::Down => self.down(s).await,
            Pass::Done => Ok(Progress::Done),
        }
    }

    async fn up<B: DataSessionBinding, A: AdminBinding>(
        &mut self,
        s: &mut Session<B, A>,
    ) -> Result<Progress, Failure> {
        let model = s.model();
        if model
            .window_edges(&self.key)
            .iter()
            .any(|(e, _)| *e == Edge::Top)
        {
            // Focus on the oldest row held: the ceiling lets go of rows
            // far from it, below, and the top stays where the pages left it.
            let oldest = model.messages(&self.key).first().map(|m| m.key);
            let key = self.key.clone();
            let side = s.side();
            side.window(WindowInput::Place {
                conversation: key.clone(),
                focused: oldest,
                in_view: oldest.into_iter().collect(),
            });
            // A side that brought its page waits to be left before it is
            // reached again.
            side.window(WindowInput::Left {
                conversation: key.clone(),
                edge: Edge::Top,
            });
            side.window(WindowInput::Reached {
                conversation: key,
                edge: Edge::Top,
            });
        }
        self.step(s).await?;
        if self.reopened(s) {
            return Ok(Progress::Working);
        }
        let marks = s.model().window_marks(&self.key);
        if marks
            .iter()
            .any(|m| matches!(m.kind, MarkKind::AtLimit(Edge::Top)))
        {
            s.side().window(WindowInput::ShowAnyway {
                conversation: self.key.clone(),
                edge: Edge::Top,
            });
            return Ok(Progress::Working);
        }
        if !s
            .model()
            .window_edges(&self.key)
            .iter()
            .any(|(e, _)| *e == Edge::Top)
        {
            self.pass = Pass::Down;
        }
        Ok(Progress::Working)
    }

    async fn down<B: DataSessionBinding, A: AdminBinding>(
        &mut self,
        s: &mut Session<B, A>,
    ) -> Result<Progress, Failure> {
        let model = s.model();
        let after = |at: &DrawnAt, cursor: Option<DrawnAt>| cursor.is_none_or(|c| *at > c);
        // The first gap below the cursor bounds what can be handed over:
        // nothing drawn past where it starts, until it is filled.
        let gaps: Vec<_> = model
            .window_gaps(&self.key)
            .into_iter()
            .filter(|g| after(&g.below, self.cursor))
            .collect();
        let barrier = gaps.iter().map(|g| g.above).min();
        let chunk: Vec<(DrawnAt, Anchor)> = model
            .drawn_order(&self.key)
            .into_iter()
            .filter(|(at, _)| after(at, self.cursor) && barrier.is_none_or(|b| *at <= b))
            .filter(|(_, a)| !matches!(a, Anchor::Item(k) if self.arrived.contains(k)))
            .take(CHUNK)
            .collect();
        if let Some((last, _)) = chunk.last() {
            self.cursor = Some(*last);
            self.stalled = 0;
            // Focus on the last row handed over: the ceiling lets go of the
            // rows already handed over, and of rows far below.
            if let Some(key) = chunk.iter().rev().find_map(|(_, a)| match a {
                Anchor::Item(key) => Some(*key),
                Anchor::Card(_) => None,
            }) {
                self.place(s, key);
            }
            return Ok(Progress::Chunk(chunk.into_iter().map(|(_, a)| a).collect()));
        }
        let Some(gap) = gaps.into_iter().min_by_key(|g| g.above) else {
            self.pass = Pass::Done;
            s.side().window(WindowInput::Closed(self.key.clone()));
            let late = std::mem::take(&mut self.late);
            return Ok(if late.is_empty() {
                Progress::Done
            } else {
                Progress::Late(late)
            });
        };
        let marks = s.model().window_marks(&self.key);
        if marks
            .iter()
            .any(|m| matches!(m.kind, MarkKind::AtLimit(e) if e == gap.upper))
        {
            s.side().window(WindowInput::ShowAnyway {
                conversation: self.key.clone(),
                edge: gap.upper,
            });
        } else {
            // The gap's upper side reads it oldest first, from the row
            // above it: the order it is handed over in.
            let side = s.side();
            side.window(WindowInput::Left {
                conversation: self.key.clone(),
                edge: gap.upper,
            });
            side.window(WindowInput::Reached {
                conversation: self.key.clone(),
                edge: gap.upper,
            });
        }
        self.step(s).await?;
        if self.reopened(s) {
            // Opened again at the newest row: up to the top again, the
            // cursor kept, so nothing handed over is handed over twice.
            self.pass = Pass::Up;
        }
        Ok(Progress::Working)
    }

    fn place<B: DataSessionBinding, A: AdminBinding>(&self, s: &mut Session<B, A>, key: ItemKey) {
        s.side().window(WindowInput::Place {
            conversation: self.key.clone(),
            focused: Some(key),
            in_view: vec![key],
        });
    }

    /// One step of the session, its failures and late rows taken, and the
    /// walk's progress checked.
    async fn step<B: DataSessionBinding, A: AdminBinding>(
        &mut self,
        s: &mut Session<B, A>,
    ) -> Result<(), Failure> {
        s.step().await;
        self.take_arrivals(s);
        if s.model().window_marks(&self.key).iter().any(|m| {
            matches!(
                m.kind,
                MarkKind::Failed(Edge::Top | Edge::GapUpper(_) | Edge::GapLower(_))
            )
        }) {
            return Err(Failure::Store(format!(
                "the messages of conversation {} could not be read from the message store",
                crate::cli::conversation_text(&self.key)
            )));
        }
        let model = s.model();
        let now = (
            self.cursor,
            model.drawn_order(&self.key).len(),
            model.window_gaps(&self.key).len(),
            model.window_edges(&self.key).len(),
        );
        if self.last == Some(now) {
            self.stalled += 1;
            if self.stalled >= STALL {
                return Err(Failure::Store(format!(
                    "the messages of conversation {} could not be walked: the message store stopped answering",
                    crate::cli::conversation_text(&self.key)
                )));
            }
        } else {
            self.stalled = 0;
        }
        self.last = Some(now);
        Ok(())
    }

    /// Take what the session's last step reported, a step the caller took
    /// itself while the walk is open included: a row it brought for the
    /// conversation is handed over after the walk's last row. Every step
    /// taken while a walk is open is taken here before the next, since a
    /// step drops what the one before it reported
    /// (`read_prints_a_row_arriving_during_its_own_step_last_and_unread`).
    pub(crate) fn take_arrivals<B: DataSessionBinding, A: AdminBinding>(
        &mut self,
        s: &mut Session<B, A>,
    ) {
        for update in s.take_updates() {
            // By its origin, held or not: one that draws far from focus is
            // let go by the ceiling in the step that brings it.
            if let Update::Received(received) = &update
                && conversation_of(&received.origin) == self.key
            {
                let item = s.model().message_of(received);
                if self.arrived.insert(item.key) {
                    self.late.push(item);
                }
            }
            if let Some(passed) = self.passed.as_mut() {
                passed.push(update);
            }
        }
    }

    /// Whether the window opened again since the walk last looked: the
    /// opening is recorded at the first look.
    fn reopened<B: DataSessionBinding, A: AdminBinding>(&mut self, s: &Session<B, A>) -> bool {
        let now = s
            .model()
            .window_opening()
            .filter(|(k, _)| *k == self.key)
            .map(|(_, seq)| seq);
        let again = self.opening.is_some() && now != self.opening;
        self.opening = now;
        again
    }
}

/// The conversation a received row belongs to.
pub(crate) fn conversation_of(origin: &Origin) -> ConversationKey {
    match origin {
        Origin::Direct { peer, endpoint } => ConversationKey::Direct {
            peer: peer.clone(),
            endpoint: Some(endpoint.clone()),
        },
        Origin::Channel { channel, .. } => ConversationKey::Channel(channel.clone()),
    }
}
