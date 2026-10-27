// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An inbound row's kind (schema v10, STATE.md): written at commit as the
//! client classified it, carried with the message from unread to kept,
//! and classified again, all at once and once per version, when the
//! client's classification changes.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use interweave_human_store::{
    AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, StoreOptions,
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
