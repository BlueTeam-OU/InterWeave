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
use interweave_human_store::{
    AppMessageId, HumanStore, InboundOrigin, NewInbound, NewOutbound, OutboundDestination,
    SCHEMA_VERSION, StoreError, StoreOptions,
};
use interweave_transport_api::{DirectDestination, MessageId, TransportIdentity};

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
        kind: interweave_human_store::InboundKind::Text,
    }
}

fn outbound(n: u32, payload: Vec<u8>) -> NewOutbound {
    NewOutbound {
        app_message_id: id(n),
        // Distinct from the application id: the two identities stay apart.
        transport_message_id: MessageId::parse_hex(&format!("{:032x}", u128::from(n) + (1 << 100)))
            .expect("32 hex characters"),
        destination: OutboundDestination::Direct(DirectDestination::to_default(peer())),
        media_type: None,
        payload,
        created_at: 1_000,
    }
}

fn raw(path: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(path).expect("an independent connection")
}

fn count(conn: &rusqlite::Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        .expect("count")
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

/// A v5 file whose migration to this build's schema fails after its first
/// step has already rebuilt a table. The migration is one transaction, so
/// the file is left at v5 with every row it had; a person who clears the
/// obstacle opens it migrated, with nothing lost.
#[test]
fn a_migration_that_fails_part_way_leaves_the_file_at_its_old_version_with_every_row() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    {
        let mut store = HumanStore::open(&path, StoreOptions::default()).expect("opens");
        store
            .commit_pending_outbound(&outbound(1, b"pending before the upgrade".to_vec()))
            .expect("pending");
        store
            .commit_unread_inbound(&inbound(2, b"unread before the upgrade"))
            .expect("unread");
    }
    // Back to v5: pending_outbound without its transport id (migration 6
    // rebuilds the table to add it), no read_pairs (migration 7 creates
    // it) and no walk indexes (migration 8's). In read_pairs' place, an index of that name: migration 7's
    // CREATE TABLE fails on it, AFTER migration 6 has rebuilt
    // pending_outbound inside the same transaction.
    raw(&path)
        .execute_batch(common::UNDO_V10)
        .expect("back before v10");
    raw(&path)
        .execute_batch(
            "CREATE TABLE pending_outbound_v5 (
                 row_id                INTEGER PRIMARY KEY AUTOINCREMENT,
                 app_message_id        TEXT    NOT NULL UNIQUE,
                 destination_peer      TEXT    NOT NULL,
                 destination_endpoint  TEXT,
                 channel_id            TEXT,
                 media_type            TEXT,
                 payload               BLOB    NOT NULL,
                 created_at            INTEGER NOT NULL,
                 last_attempt_at       INTEGER,
                 attempts              INTEGER NOT NULL DEFAULT 0
             );
             INSERT INTO pending_outbound_v5
                 SELECT row_id, app_message_id, destination_peer, destination_endpoint,
                        channel_id, media_type, payload, created_at, last_attempt_at, attempts
                   FROM pending_outbound;
             DROP TABLE pending_outbound;
             ALTER TABLE pending_outbound_v5 RENAME TO pending_outbound;
             DROP TABLE read_pairs;
             DROP INDEX unread_inbound_by_received;
             DROP INDEX kept_inbound_by_received;
             DROP TABLE own_cards;
             ALTER TABLE contacts DROP COLUMN name_source;
             ALTER TABLE contacts DROP COLUMN name_at;
             ALTER TABLE contacts DROP COLUMN photo_source;
             ALTER TABLE contacts DROP COLUMN photo_at;
             ALTER TABLE contacts DROP COLUMN photo;
             ALTER TABLE contacts ADD COLUMN avatar_ref TEXT;
             ALTER TABLE contact_routes DROP COLUMN identity_key;
             ALTER TABLE conversation_index DROP COLUMN card_shared_at;
             CREATE INDEX read_pairs ON settings(key);
             PRAGMA user_version = 5;",
        )
        .expect("a v5 file with an obstacle in migration 7's way");
    let v5_columns = |conn: &rusqlite::Connection| -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM pragma_table_info('pending_outbound')")
            .expect("table info");
        stmt.query_map([], |r| r.get(0))
            .expect("columns")
            .map(|c| c.expect("column"))
            .collect()
    };
    let before = v5_columns(&raw(&path));
    assert!(!before.iter().any(|c| c == "transport_message_id"));

    let error = HumanStore::open(&path, StoreOptions::default())
        .expect_err("a migration that cannot apply refuses the open");
    assert!(
        matches!(&error, StoreError::Migration(m) if m.contains("already an index named read_pairs")),
        "{error:?}"
    );
    assert!(error.needs_recovery(), "{error:?}");

    // NOTHING OF THE FAILED MIGRATION REMAINS: still v5, still v5's shape,
    // every row where it was.
    let conn = raw(&path);
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .expect("version");
    assert_eq!(
        version, 5,
        "the version moves only with the whole migration"
    );
    assert_eq!(
        v5_columns(&conn),
        before,
        "migration 6's rebuild was rolled back with migration 7's failure"
    );
    assert_eq!(count(&conn, "pending_outbound"), 1);
    assert_eq!(count(&conn, "unread_inbound"), 1);

    // The obstacle cleared, the same file migrates with both rows.
    conn.execute_batch("DROP INDEX read_pairs;")
        .expect("clear it");
    drop(conn);
    let store = HumanStore::open(&path, StoreOptions::default()).expect("migrates now");
    let pending = store.pending_outbound().expect("read");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].payload, b"pending before the upgrade");
    let unread = store.unread_inbound().expect("read");
    assert_eq!(unread.len(), 1);
    assert_eq!(unread[0].payload, b"unread before the upgrade");
    drop(store);
    let version: i64 = raw(&path)
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .expect("version");
    assert_eq!(version, SCHEMA_VERSION);
}

/// A medium that fills while the person is SENDING. The full-medium cases
/// beside this one fill it with inbound; a send is the other road to the
/// same disk, and its refusal must degrade the store too, since the
/// receiver shares the file: an outbound that hit the ceiling leaves no
/// room for the next unread message either.
///
/// The ceiling is a page quota, so the failure is the `SQLITE_FULL` a full
/// filesystem returns (SQLite maps `ENOSPC` to it), through the code a full
/// filesystem takes.
#[test]
fn a_send_that_fills_the_medium_is_refused_degrades_the_store_and_keeps_every_acknowledged_row() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let mut store = HumanStore::open(
        &path,
        StoreOptions {
            max_pages: Some(64),
        },
    )
    .expect("opens");

    let maximal = |n: u32| {
        let mut payload = vec![0_u8; interweave_transport_api::MAX_PAYLOAD_BYTES];
        payload[..4].copy_from_slice(&n.to_be_bytes());
        payload
    };
    let mut acknowledged = Vec::new();
    let mut refused = None;
    for n in 1..=8 {
        match store.commit_pending_outbound(&outbound(n, maximal(n))) {
            Ok(_) => acknowledged.push(n),
            Err(e) => {
                refused = Some((n, e));
                break;
            }
        }
    }
    let (refused_n, refused) = refused.expect("8 x 48 KiB cannot fit 64 pages");
    assert!(
        !acknowledged.is_empty(),
        "the ceiling admits a send first, so the store filled rather than never fitting"
    );
    assert!(
        matches!(&refused, StoreError::Sql(_)),
        "the refusal is the medium's own: {refused:?}"
    );

    // THE CLAIM: the send's refusal degrades the store, as an inbound's
    // does, so the receiver stops presenting as durable.
    assert_eq!(
        store.health(),
        StorageHealth::Degraded,
        "a full medium met by a send degrades the store"
    );
    assert!(
        store
            .health()
            .degraded_response()
            .is_some_and(|r| r.release_human_endpoint && r.suspend_broadcast_joins),
    );
    let next = store
        .commit_unread_inbound(&inbound(100, b"tiny"))
        .expect_err("a degraded store takes no unread content, however small");
    assert!(matches!(next, StoreError::Degraded), "{next:?}");
    let next = store
        .commit_pending_outbound(&outbound(101, b"tiny".to_vec()))
        .expect_err("nor a new send");
    assert!(matches!(next, StoreError::Degraded), "{next:?}");
    drop(store);

    // NEVER SILENTLY: reopened without the ceiling, the file holds every
    // acknowledged send, byte for byte under its own ids, and nothing of
    // the refused one.
    let store = HumanStore::open(&path, StoreOptions::default()).expect("reopens");
    let pending = store.pending_outbound().expect("read");
    let held: Vec<u32> = pending
        .iter()
        .map(|p| {
            let n = u32::from_be_bytes(p.payload[..4].try_into().expect("four bytes"));
            assert_eq!(p.payload, maximal(n), "send {n} is intact");
            assert_eq!(p.app_message_id, id(n));
            assert_eq!(
                p.transport_message_id,
                outbound(n, Vec::new()).transport_message_id
            );
            n
        })
        .collect();
    assert_eq!(
        held, acknowledged,
        "every acknowledged send, and only those"
    );
    assert!(!held.contains(&refused_n));
}
