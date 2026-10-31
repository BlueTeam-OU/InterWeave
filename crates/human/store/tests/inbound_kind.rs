// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An inbound row's kind (schema v10, STATE.md): written at commit as the
//! client classified it, carried with the message from unread to kept,
//! and classified again, all at once and once per version, when the
//! client's classification changes.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use interweave_human_store::{
    AppMessageId, HumanStore, InboundCounts, InboundKind, InboundOrigin, NewInbound, PageLimits,
    StoreConversation, StoreOptions,
};
use interweave_transport_api::{EndpointId, TransportIdentity};

const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

fn store() -> (tempfile::TempDir, std::path::PathBuf, HumanStore) {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let store = HumanStore::open(&path, StoreOptions::default()).expect("opens");
    (dir, path, store)
}

fn inbound(n: u8, kind: InboundKind) -> NewInbound {
    NewInbound {
        app_message_id: AppMessageId::parse(format!("{n:032x}")).expect("an id"),
        origin: InboundOrigin {
            peer: TransportIdentity::parse(P).expect("a PeerId"),
            endpoint: Some(EndpointId::parse("human").expect("an endpoint")),
            channel: None,
        },
        media_type: None,
        payload: vec![n],
        received_at: u64::from(n),
        kind,
    }
}

/// The kind written at commit is the one read back, and keeping a message
/// carries it to the kept row, which an unkeep reads back too. Mutation:
/// `keep` writing the column's default.
#[test]
fn a_rows_kind_is_kept_from_commit_through_keep() {
    let (_dir, _path, mut store) = store();
    let text = store
        .commit_unread_inbound(&inbound(1, InboundKind::Text))
        .expect("text");
    let card = store
        .commit_unread_inbound(&inbound(2, InboundKind::Card))
        .expect("card");
    let kinds: Vec<_> = store
        .unread_inbound()
        .expect("read")
        .iter()
        .map(|r| r.kind)
        .collect();
    assert_eq!(kinds, [InboundKind::Text, InboundKind::Card]);

    let read = store.mark_read(card, 10).expect("read");
    store.keep(&read, 11).expect("kept");
    let kept = store.kept_inbound().expect("kept rows");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].kind, InboundKind::Card, "the card stays a card");
    assert_eq!(
        store.unread_inbound().expect("read")[0].row_id,
        text,
        "control: the text row is untouched"
    );
}

/// The kept count is of the kinds this build shows, a text and a card,
/// and a kept row of another kind is counted hidden instead. Mutations:
/// the kind filter dropped from the kept count; the shown kinds counted
/// hidden.
#[test]
fn the_kept_count_is_of_the_kinds_shown() {
    let (_dir, path, mut store) = store();
    for (n, kind) in [
        (1, InboundKind::Text),
        (2, InboundKind::Card),
        (3, InboundKind::Text),
    ] {
        let row = store
            .commit_unread_inbound(&inbound(n, kind))
            .expect("committed");
        let read = store.mark_read(row, 10).expect("read");
        store.keep(&read, 11).expect("kept");
    }
    store
        .commit_unread_inbound(&inbound(4, InboundKind::Text))
        .expect("unread, not kept");
    assert_eq!(
        store.inbound_counts().expect("counts"),
        InboundCounts {
            hidden_unread: 0,
            hidden_kept: 0,
            kept: 3
        }
    );
    rusqlite::Connection::open(&path)
        .expect("a second connection")
        .execute(
            "UPDATE kept_inbound SET kind = 'image' WHERE kind = 'card'",
            [],
        )
        .expect("a newer build's kind");
    assert_eq!(
        store.inbound_counts().expect("counts"),
        InboundCounts {
            hidden_unread: 0,
            hidden_kept: 1,
            kept: 2
        }
    );
}

/// The client's run at open: every row classified again, only a kind
/// that changed written, and the version recorded so the next open does
/// not run it again. Before any client ran, there is no version (the v10
/// migration leaves none). Mutation: the version not written.
#[test]
fn a_reclassification_rewrites_every_row_and_records_its_version() {
    let (_dir, path, mut store) = store();
    assert_eq!(store.inbound_kind_version().expect("version"), None);
    for n in 1..=5 {
        store
            .commit_unread_inbound(&inbound(n, InboundKind::Other))
            .expect("committed");
    }
    let held = store
        .commit_unread_inbound(&inbound(6, InboundKind::Text))
        .expect("committed");
    let read = store.mark_read(held, 20).expect("read");
    store.keep(&read, 21).expect("kept");

    let mut seen = 0;
    store
        .reclassify_inbound("2", |row| {
            seen += 1;
            if row.payload[0] % 2 == 0 {
                InboundKind::Card
            } else {
                InboundKind::Text
            }
        })
        .expect("reclassified");
    assert_eq!(seen, 6, "every unread and kept row, once");
    assert_eq!(
        store.inbound_kind_version().expect("version").as_deref(),
        Some("2")
    );
    let unread: Vec<_> = store
        .unread_inbound()
        .expect("read")
        .iter()
        .map(|r| (r.payload[0], r.kind))
        .collect();
    assert_eq!(
        unread,
        [
            (1, InboundKind::Text),
            (2, InboundKind::Card),
            (3, InboundKind::Text),
            (4, InboundKind::Card),
            (5, InboundKind::Text),
        ]
    );
    assert_eq!(
        store.kept_inbound().expect("kept")[0].kind,
        InboundKind::Card
    );
    drop(store);

    // Durable: a reopened store reads the new kinds and the version.
    let store = HumanStore::open(&path, StoreOptions::default()).expect("reopens");
    assert_eq!(
        store.inbound_kind_version().expect("version").as_deref(),
        Some("2")
    );
    assert_eq!(
        store.unread_inbound().expect("read")[1].kind,
        InboundKind::Card
    );
}

/// A reclassification that cannot finish leaves every kind and the old
/// version: the next open runs it again from the start. Mutation: the
/// updates committed outside the transaction.
#[test]
fn a_reclassification_that_fails_changes_nothing() {
    let (_dir, path, mut store) = store();
    for n in 1..=3 {
        store
            .commit_unread_inbound(&inbound(n, InboundKind::Other))
            .expect("committed");
    }
    store
        .reclassify_inbound("1", |_| InboundKind::Other)
        .expect("a first version");
    let conn = rusqlite::Connection::open(&path).expect("a second connection");
    conn.execute_batch(
        "CREATE TRIGGER refuse_third BEFORE UPDATE OF kind ON unread_inbound
          WHEN OLD.received_at = 3 BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    )
    .expect("the obstacle, on the open store");
    drop(conn);

    let failed = store.reclassify_inbound("2", |_| InboundKind::Text);
    assert!(failed.is_err(), "the third update refused: {failed:?}");
    assert_eq!(
        store.inbound_kind_version().expect("version").as_deref(),
        Some("1"),
        "the old version stays"
    );
    assert!(
        store
            .unread_inbound()
            .expect("read")
            .iter()
            .all(|r| r.kind == InboundKind::Other),
        "no row kept the two updates made before the failure"
    );
}

/// A kind only a newer build writes (here `image`) is neither text nor
/// card: hidden, never counted in a badge nor paged, and a
/// reclassification writes it back as this build's `other` rather than
/// leaving it because it reads back as `Other` (#489 review F3).
/// Mutations: the counts filtering `<> 'other'`; the reclassification not
/// resetting an unknown kind.
#[test]
fn a_kind_this_build_does_not_know_is_hidden_and_reset_by_reclassification() {
    let (_dir, path, mut store) = store();
    store
        .commit_unread_inbound(&inbound(1, InboundKind::Text))
        .expect("committed");
    rusqlite::Connection::open(&path)
        .expect("a second connection")
        .execute("UPDATE unread_inbound SET kind = 'image'", [])
        .expect("a newer build's kind");

    assert!(
        store.unread_counts().expect("counts").is_empty(),
        "not counted"
    );
    assert_eq!(
        store.inbound_counts().expect("counts"),
        InboundCounts {
            hidden_unread: 1,
            hidden_kept: 0,
            kept: 0
        }
    );
    let conversation = StoreConversation::Direct {
        peer: TransportIdentity::parse(P).expect("a PeerId"),
        endpoint: EndpointId::parse("human").expect("an endpoint"),
    };
    let page = store
        .conversation_page(&conversation, None, PageLimits::default())
        .expect("a page");
    assert!(page.items.is_empty(), "not paged");

    store
        .reclassify_inbound("2", |_| InboundKind::Other)
        .expect("reclassified");
    let stored: String = rusqlite::Connection::open(&path)
        .expect("a second connection")
        .query_row("SELECT kind FROM unread_inbound", [], |r| r.get(0))
        .expect("the kind");
    assert_eq!(stored, "other", "written back as this build's");
}

/// The unread cards are read by their kind alone, oldest first and a page
/// at a time: no message, no row of a kind this build cannot show, and no
/// kept card. Mutations: the kind filter dropped; the page's cursor not
/// carried to the next.
#[test]
fn the_unread_cards_are_read_by_their_kind_a_page_at_a_time() {
    let (_dir, _path, mut store) = store();
    for (n, kind) in [
        (1, InboundKind::Text),
        (2, InboundKind::Card),
        (3, InboundKind::Other),
        (4, InboundKind::Card),
        (5, InboundKind::Text),
        (6, InboundKind::Card),
    ] {
        store
            .commit_unread_inbound(&inbound(n, kind))
            .expect("a row");
    }
    let kept = store
        .commit_unread_inbound(&inbound(7, InboundKind::Card))
        .expect("a card to keep");
    let read = store.mark_read(kept, 10).expect("read");
    store.keep(&read, 11).expect("kept");

    let two = PageLimits::new(2, 1024).expect("limits");
    let first = store.unread_cards_page(None, two).expect("a page");
    let second = store
        .unread_cards_page(first.next, two)
        .expect("the next page");
    let at = |rows: &[interweave_human_store::StoredInbound]| -> Vec<u64> {
        rows.iter().map(|r| r.received_at).collect()
    };
    assert_eq!(at(&first.items), [2, 4], "the first two cards");
    assert!(first.next.is_some(), "more to read");
    assert_eq!(at(&second.items), [6], "the last card, the kept one not");
    assert!(second.next.is_none(), "nothing after it");
}
