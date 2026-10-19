// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Stage 18's S10, the store's half (plan section 21; `tests/SECURITY-MATRIX.md`
//! row S21-10): a corrupt file, a failed migration and a full medium. Each
//! fails safe, degrades the human endpoint, and never drops content silently
//! (ADR-0044; `RETENTION.md` section 9 rule 14).
//!
//! "Silently" is what each case pins. A store that cannot do what it was
//! asked says so in the answer, and what the file holds afterwards is
//! exactly what the store acknowledged: no refused row half-written, no
//! acknowledged row lost on the way to the refusal, and no read of a
//! damaged table answered as an empty one.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::os::unix::fs::FileExt as _;
use std::path::Path;

use interweave_human_core::retention::StorageHealth;
use interweave_human_store::{AppMessageId, HumanStore, InboundOrigin, NewInbound, StoreOptions};
use interweave_transport_api::TransportIdentity;

const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

fn peer() -> TransportIdentity {
    TransportIdentity::parse(PEER).expect("the fixture peer id is canonical")
}

fn id(n: u32) -> AppMessageId {
    AppMessageId::parse(format!("{n:032x}")).expect("canonical")
}

fn inbound(n: u32, payload: &[u8]) -> NewInbound {
    NewInbound {
        app_message_id: id(n),
        origin: InboundOrigin {
            peer: peer(),
            endpoint: None,
            channel: None,
        },
        media_type: None,
        payload: payload.to_vec(),
        received_at: 2_000,
    }
}

fn raw(path: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(path).expect("an independent connection")
}

/// The byte offset of `table`'s root page in the file: where its b-tree
/// header starts. Read from the closed file, which the store's close left
/// with an empty log, so the file is the whole database.
fn root_page_offset(path: &Path, table: &str) -> u64 {
    let conn = raw(path);
    let page_size: i64 = conn
        .query_row("PRAGMA page_size", [], |r| r.get(0))
        .expect("page size");
    let root: i64 = conn
        .query_row(
            "SELECT rootpage FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |r| r.get(0),
        )
        .expect("root page");
    assert!(root > 1, "page 1 carries the file header before its b-tree");
    u64::try_from((root - 1) * page_size).expect("a file offset")
}

/// A file whose header is sound and whose `unread_inbound` b-tree is not:
/// the damage a torn write or a failing sector leaves, which the header
/// check at open cannot see. What a person is owed is a store that reads
/// as damaged, not one that reads as empty: an empty unread list would
/// tell them they have no messages, when three are on disk.
#[test]
fn a_store_damaged_under_its_unread_rows_needs_recovery_and_never_reads_as_empty() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    {
        let mut store = HumanStore::open(&path, StoreOptions::default()).expect("opens");
        for n in 1..=3 {
            store
                .commit_unread_inbound(&inbound(n, b"on disk before the damage"))
                .expect("a healthy medium commits");
        }
    }

    // THE CONTROL: before the damage, a fresh open reads all three.
    assert_eq!(
        HumanStore::open(&path, StoreOptions::default())
            .expect("reopens")
            .unread_inbound()
            .expect("reads")
            .len(),
        3
    );

    // THE DAMAGE: the root page's type byte, a value no b-tree page has.
    let offset = root_page_offset(&path, "unread_inbound");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("the file");
    file.write_all_at(&[0xFF], offset).expect("damage the page");
    drop(file);

    let mut store = HumanStore::open(&path, StoreOptions::default())
        .expect("the header is sound, so the store opens: the damage is met at the first read");

    // NEVER AS EMPTY: the read is an error, and one a client turns into
    // its recovery screen rather than a retry.
    let read = store
        .unread_inbound()
        .expect_err("a damaged table cannot answer with a list");
    assert!(read.needs_recovery(), "{read:?}");
    assert!(
        store
            .unread_inbound_page(None, interweave_human_store::PageLimits::default())
            .is_err(),
        "the paged read the client walks fails the same way"
    );

    // A new message cannot be promised to this file: the commit is refused
    // and the store degrades, so the human endpoint is released rather
    // than accepting a stream it would lose.
    let commit = store
        .commit_unread_inbound(&inbound(4, b"after the damage"))
        .expect_err("a damaged table takes no new content");
    assert!(commit.needs_recovery(), "{commit:?}");
    assert_eq!(store.health(), StorageHealth::Degraded);
    assert!(
        store
            .health()
            .degraded_response()
            .is_some_and(|r| r.release_human_endpoint && r.surface_to_user),
        "a degraded store owes the endpoint's release and a word to the person"
    );
    drop(store);

    // AND LEFT AS IT IS: no open, read or commit "repaired" the file by
    // dropping what it could not read (STATE.md Migrations: a client never
    // renames, moves or deletes a file needing recovery).
    let mut byte = [0_u8];
    std::fs::File::open(&path)
        .expect("the file")
        .read_exact_at(&mut byte, offset)
        .expect("read back");
    assert_eq!(byte, [0xFF], "the damaged page is still the damaged page");
}
