// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The two-pass walk against a real store file, as architect-cto's rulings
//! pin it (01a18d00, 01a18ecf): a conversation with more rows than the
//! held-row ceiling is handed over whole, each row once, oldest first; the
//! first pass hands over nothing and marks nothing read; a row committed
//! during the walk comes after the walk's last row; the session bound is
//! passed by Show more messages.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use interweave_human_app_core::FacadeSide;
use interweave_human_app_core::test_hooks::{Fault, StoreFaults};
use interweave_human_store::{
    AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, StoreOptions,
};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{
    Anchor, ConversationKey, HELD_ROW_CAP, READ_PROPOSAL_CAP, SESSION_ITEM_CAP,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, MediaType, TransportIdentity};

use crate::Failure;
use crate::session::Session;
use crate::walk::{Progress, Walk};

const WALL: u64 = 1_792_652_603_000;

fn endpoint() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: ProfileIdentity::generate()
            .transport_identity()
            .expect("peer id"),
        endpoints: vec![FakeEndpoint::open(endpoint(), false)],
        default_endpoint: Some(endpoint()),
        queue_bound: 16,
    }
}

fn private_dir() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt as _;
    let mut builder = tempfile::Builder::new();
    builder.permissions(std::fs::Permissions::from_mode(0o700));
    builder.tempdir().expect("tempdir")
}

fn store_at(dir: &Path) -> HumanStore {
    HumanStore::open(&dir.join("human.sqlite3"), StoreOptions::default()).expect("store")
}

fn to(peer: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: peer.clone(),
        endpoint: Some(endpoint()),
    }
}

/// One row seeded: its text, when it arrived, and whether it is kept.
struct Row {
    text: String,
    at: u64,
    kept: bool,
}

/// Commit `rows` from `from` to the store in `dir`, and answer the order
/// a conversation draws them in: by when they arrived, an unread row
/// before a kept one at the same instant, then by row id.
fn seed(dir: &Path, from: &TransportIdentity, rows: &[Row]) -> Vec<String> {
    let mut store = store_at(dir);
    let mut drawn = Vec::new();
    for (n, row) in rows.iter().enumerate() {
        let id = format!("{n:032x}");
        let stored = store
            .commit_unread_inbound(&NewInbound {
                app_message_id: AppMessageId::parse(id.clone()).expect("an id"),
                origin: InboundOrigin {
                    peer: from.clone(),
                    endpoint: Some(endpoint()),
                    channel: None,
                },
                media_type: Some(
                    MediaType::parse("application/vnd.interweave-human-chat+json;v=2")
                        .expect("media"),
                ),
                payload: format!(
                    r#"{{"v":2,"kind":"text","app_message_id":"{id}","text":"{}"}}"#,
                    row.text
                )
                .into_bytes(),
                received_at: row.at,
                kind: InboundKind::Text,
            })
            .expect("committed");
        let (table, id) = if row.kept {
            let read = store.mark_read(stored, row.at).expect("read");
            (2, store.keep(&read, row.at).expect("kept").get())
        } else {
            (1, stored.get())
        };
        drawn.push(((row.at, table, id), row.text.clone()));
    }
    drawn.sort();
    drawn.into_iter().map(|(_, text)| text).collect()
}

fn session(
    node: &FakeNode,
    dir: &Path,
    faults: Option<Arc<StoreFaults>>,
) -> Session<FakeNode, FakeNode> {
    let client = TransportClient::new(
        node.clone(),
        node.clone(),
        store_at(dir),
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![endpoint()],
            channels: vec![],
            max_payload_bytes: 49_152,
        },
        Box::new(|| WALL),
        0,
    )
    .expect("facade");
    let mut facade = FacadeSide::new(client, || WALL);
    if let Some(faults) = faults {
        facade = facade.with_store_faults(faults);
    }
    Session::started(facade, || Some(true)).expect("session")
}

/// What a walk handed over, as texts, and how it ended.
struct Walked {
    texts: Vec<String>,
    chunks_before_down: usize,
    end: Result<(), Failure>,
}

/// The texts of `anchors`' messages.
fn texts_of(
    s: &Session<FakeNode, FakeNode>,
    key: &ConversationKey,
    anchors: Vec<Anchor>,
) -> Vec<String> {
    let shown = s.model().messages(key);
    anchors
        .into_iter()
        .filter_map(|anchor| match anchor {
            Anchor::Item(k) => Some(
                shown
                    .iter()
                    .find(|m| m.key == k)
                    .expect("handed over held")
                    .source
                    .clone(),
            ),
            Anchor::Card(_) => None,
        })
        .collect()
}

/// Walk `key` to its end, at most `bound` advances, calling `between`
/// with the advance's number after each.
async fn walk(
    s: &mut Session<FakeNode, FakeNode>,
    key: &ConversationKey,
    bound: usize,
    mut between: impl FnMut(usize),
) -> Walked {
    let mut w = Walk::open(s, key);
    let mut walked = Walked {
        texts: Vec::new(),
        chunks_before_down: 0,
        end: Ok(()),
    };
    for n in 0..bound {
        match w.advance(s).await {
            Err(e) => {
                walked.end = Err(e);
                return walked;
            }
            Ok(Progress::Working) => {}
            Ok(Progress::Chunk(anchors)) => {
                if !w.past_the_top() {
                    walked.chunks_before_down += 1;
                }
                walked.texts.extend(texts_of(s, key, anchors));
            }
            Ok(Progress::Late(items)) => {
                walked.texts.extend(items.into_iter().map(|m| m.source));
            }
            Ok(Progress::Done) => return walked,
        }
        between(n);
    }
    panic!("the walk did not end within {bound} advances");
}

fn rows(n: usize, kept_every: usize) -> Vec<Row> {
    (1..=n)
        .map(|i| Row {
            text: format!("row {i}"),
            // Every tenth row arrives at the same instant as the one
            // before it: the drawn order then falls to the table and row.
            at: 10_000 + u64::try_from(i - usize::from(i % 10 == 0)).expect("small"),
            kept: kept_every != 0 && i % kept_every == 0,
        })
        .collect()
}

/// architect-cto's (b): a conversation with more rows than the held-row
/// ceiling is handed over whole, every row once, in the order it draws
/// -- ties broken by table and row, kept rows among unread ones -- and
/// nothing is handed over before the top is reached. Mutations: the
/// barrier ignored; the cursor check removed.
#[tokio::test]
async fn a_conversation_past_the_held_row_ceiling_prints_every_row_once_oldest_first() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let dir = private_dir();
    let expected = seed(dir.path(), b.peer(), &rows(HELD_ROW_CAP + 300, 33));
    let mut s = session(&a, dir.path(), None);
    let walked = walk(&mut s, &to(b.peer()), 20_000, |_| {}).await;
    walked.end.expect("walked");
    assert_eq!(walked.chunks_before_down, 0, "nothing before the top");
    assert_eq!(walked.texts.len(), expected.len(), "each row once");
    assert_eq!(walked.texts, expected, "oldest first");
}

/// The same at bounds small enough that gaps form, merge and fill many
/// times over.
#[tokio::test]
async fn at_small_bounds_every_row_is_printed_once_oldest_first() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let dir = private_dir();
    let expected = seed(dir.path(), b.peer(), &rows(60, 7));
    let mut s = session(&a, dir.path(), None);
    s.side().set_bounds_for_test(SESSION_ITEM_CAP, 8, 4);
    let walked = walk(&mut s, &to(b.peer()), 20_000, |_| {}).await;
    walked.end.expect("walked");
    assert_eq!(walked.texts, expected);
}

/// architect-cto's (a): the first pass hands over nothing and marks
/// nothing read. A page read failing in it ends the walk with the store's
/// failure, nothing handed over, and every row still unread in the store.
/// Mutation: the first pass handing over what its pages bring.
#[tokio::test]
async fn the_first_pass_marks_nothing_read_and_prints_nothing() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let dir = private_dir();
    seed(dir.path(), b.peer(), &rows(300, 0));
    let faults = Arc::new(StoreFaults::default());
    let mut s = session(&a, dir.path(), Some(Arc::clone(&faults)));
    let pages = Arc::clone(&faults);
    let walked = walk(&mut s, &to(b.peer()), 1_000, move |n| {
        if n == 2 {
            pages.set_pages(Fault::Fail);
        }
    })
    .await;
    assert!(walked.texts.is_empty(), "{:?}", walked.texts);
    assert!(
        matches!(&walked.end, Err(Failure::Store(why)) if why.contains("could not be read")),
        "{:?}",
        walked.end.as_ref().err().map(Failure::message)
    );
    drop(s);
    let unread: i64 = rusqlite::Connection::open(dir.path().join("human.sqlite3"))
        .expect("the store file")
        .query_row("SELECT COUNT(*) FROM unread_inbound", [], |r| r.get(0))
        .expect("a count");
    assert_eq!(unread, 300, "nothing read");
}

/// Walk B's conversation in A's store, seeded with `rows`, B sending
/// "late" to A once the first chunk is handed over; what was handed over.
async fn walked_with_a_send_midway(rows: &[Row]) -> (Vec<String>, Vec<String>) {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let expected = seed(dir_a.path(), b.peer(), rows);
    let mut alice = session(&a, dir_a.path(), None);
    let mut bob = session(&b, dir_b.path(), None);
    let until = Instant::now() + Duration::from_secs(5);
    alice.open(until).await;
    bob.open(until).await;
    let key = to(b.peer());

    let mut w = Walk::open(&mut alice, &key);
    let mut texts = Vec::new();
    let mut sent = false;
    for _ in 0..20_000 {
        match w.advance(&mut alice).await.expect("walked") {
            Progress::Working => {}
            Progress::Chunk(anchors) => {
                texts.extend(texts_of(&alice, &key, anchors));
                if !sent {
                    sent = true;
                    let r = crate::verbs::Run {
                        profile: "test",
                        json: true,
                        until: Instant::now() + Duration::from_secs(5),
                    };
                    let mut out = String::new();
                    crate::verbs::run(
                        &mut bob,
                        &crate::cli::Verb::Send {
                            conversation: to(a.peer()),
                            text: Some("late".to_owned()),
                            pending: false,
                        },
                        &r,
                        &mut out,
                    )
                    .await
                    .expect("sent");
                }
            }
            Progress::Late(items) => texts.extend(items.into_iter().map(|m| m.source)),
            Progress::Done => return (texts, expected),
        }
    }
    panic!("the walk did not end");
}

/// architect-cto's (b): a row committed while the walk runs comes after
/// the walk's last row, and every stored row once before it. Mutations:
/// the row also handed over where it draws (twice); held back and never
/// handed over.
#[tokio::test]
async fn a_row_committed_mid_walk_prints_after_the_walks_last_row() {
    let (texts, mut expected) = walked_with_a_send_midway(&rows(HELD_ROW_CAP + 100, 0)).await;
    expected.push("late".to_owned());
    assert_eq!(texts, expected);
}

/// The same when the row draws before rows not yet handed over -- the
/// stored rows arrived after this device's clock now reads -- still after
/// the walk's last row, though the ceiling let it go in the step that
/// brought it. Mutation: held back and never handed over.
#[tokio::test]
async fn a_row_committed_mid_walk_drawn_before_the_cursor_prints_last() {
    let ahead: Vec<Row> = (1..=u64::try_from(HELD_ROW_CAP + 100).expect("small"))
        .map(|i| Row {
            text: format!("ahead {i}"),
            at: WALL + 1_000 + i,
            kept: false,
        })
        .collect();
    let (texts, mut expected) = walked_with_a_send_midway(&ahead).await;
    expected.push("late".to_owned());
    assert_eq!(texts, expected);
}

/// At the session bound the walk presses Show more messages and goes on,
/// every row handed over once, oldest first. Mutation: the walk stopping
/// at the limit row.
#[tokio::test]
async fn at_the_session_bound_the_walk_presses_show_anyway() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let dir = private_dir();
    let expected = seed(dir.path(), b.peer(), &rows(30, 0));
    let mut s = session(&a, dir.path(), None);
    s.side()
        .set_bounds_for_test(8, HELD_ROW_CAP, READ_PROPOSAL_CAP);
    let walked = walk(&mut s, &to(b.peer()), 5_000, |_| {}).await;
    walked.end.expect("walked");
    assert_eq!(walked.texts, expected);
}
