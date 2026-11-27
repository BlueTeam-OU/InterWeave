// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The pending-add record (schema v15; STATE.md's pending-add paragraph,
//! ADR-0061 decision 7, A 2026-11-26): one row per local `EndpointId` the
//! add form asked for, the id and nothing else, at most
//! `PENDING_ADD_CAP` rows with none evicted, one record for every handle
//! of an open.
//!
//! Every key here is TEST-ONLY (`interweave_test_support::endpoint_keys`).

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use interweave_human_store::{
    HumanStore, PENDING_ADD_CAP, SCHEMA_VERSION, StorageHealth, StoreError, StoreOptions,
};
use interweave_test_support::endpoint_keys::endpoint_id;

fn open(path: &std::path::Path) -> HumanStore {
    HumanStore::open(path, &endpoint_id("human"), StoreOptions::default()).expect("opens")
}

fn version(path: &std::path::Path) -> i64 {
    rusqlite::Connection::open(path)
        .expect("a connection")
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .expect("the version")
}

/// The version this build writes.
#[test]
fn this_build_writes_v15() {
    assert_eq!(SCHEMA_VERSION, 15);
}

/// A store at v14 opens at v15 with the table, empty, and keeps the rows
/// it held. Mutation: `run_migrations` without `migration_15` -- the open
/// is refused, "table `pending_adds` is missing".
#[test]
fn a_v14_store_gains_the_pending_add_table_and_keeps_its_rows() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let mut store = open(&path);
    store
        .set_last_front(&endpoint_id("second"))
        .expect("a row v14 holds");
    drop(store);
    rusqlite::Connection::open(&path)
        .expect("a second connection")
        .execute_batch(common::UNDO_V15)
        .expect("back to v14");
    assert_eq!(version(&path), 14);

    let store = open(&path);
    assert_eq!(version(&path), 15);
    assert_eq!(store.pending_adds().expect("read"), Vec::new());
    assert_eq!(
        store.last_front().expect("read"),
        Some(endpoint_id("second")),
        "the row v14 held is kept"
    );
}

/// A row with a column beyond the id -- an alias, a name, a time -- is
/// refused at the next open: the record holds the id and nothing else.
/// Mutation: no `TableShape` for `pending_adds` -- the open succeeds.
#[test]
fn the_pending_add_table_holds_the_id_and_nothing_else() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    drop(open(&path));
    rusqlite::Connection::open(&path)
        .expect("a second connection")
        .execute_batch("ALTER TABLE pending_adds ADD COLUMN alias TEXT;")
        .expect("a column added by hand");
    match HumanStore::open(&path, &endpoint_id("human"), StoreOptions::default()) {
        Err(StoreError::Migration(why)) => assert!(why.contains("pending_adds"), "{why}"),
        other => panic!("a second column is refused: {other:?}"),
    }
}

/// Written, listed in the order written, kept across a reopen, and
/// forgotten: the delete answers whether there was a row, and a second
/// delete is no error. Mutation: `forget_pending_add` deleting nothing --
/// the row is still listed.
#[test]
fn a_pending_add_is_written_listed_kept_across_a_reopen_and_forgotten() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let mut store = open(&path);
    assert_eq!(store.pending_adds().expect("read"), Vec::new());
    store.add_pending_add(&endpoint_id("b")).expect("written");
    store.add_pending_add(&endpoint_id("a")).expect("written");
    assert_eq!(
        store.pending_adds().expect("read"),
        [endpoint_id("b"), endpoint_id("a")],
        "in the order written"
    );
    drop(store);

    let mut store = open(&path);
    assert_eq!(
        store.pending_adds().expect("read"),
        [endpoint_id("b"), endpoint_id("a")],
        "kept across a restart"
    );
    assert!(
        store
            .forget_pending_add(&endpoint_id("b"))
            .expect("deleted")
    );
    assert!(
        !store.forget_pending_add(&endpoint_id("b")).expect("again"),
        "no row the second time"
    );
    assert_eq!(store.pending_adds().expect("read"), [endpoint_id("a")]);
    drop(store);
    assert_eq!(
        open(&path).pending_adds().expect("read"),
        [endpoint_id("a")]
    );
}

/// A write retried with the same id keeps one row. Mutation: a plain
/// `INSERT` -- the second write fails on the primary key.
#[test]
fn writing_the_same_pending_add_twice_keeps_one_row() {
    let mut store =
        HumanStore::open_in_memory(&endpoint_id("human"), StoreOptions::default()).expect("opens");
    store.add_pending_add(&endpoint_id("a")).expect("written");
    store.add_pending_add(&endpoint_id("a")).expect("again");
    assert_eq!(store.pending_adds().expect("read"), [endpoint_id("a")]);
}

/// With `PENDING_ADD_CAP` rows a new id is refused, nothing is evicted,
/// the store stays healthy, and a row already held is accepted again.
/// Mutations: `>` for `>=` in the bound -- the 65th is written; the
/// oldest deleted before the insert -- the first row is gone.
#[test]
fn the_65th_pending_add_is_refused_and_no_row_is_evicted() {
    assert_eq!(PENDING_ADD_CAP, 64);
    let mut store =
        HumanStore::open_in_memory(&endpoint_id("human"), StoreOptions::default()).expect("opens");
    let ids: Vec<_> = (0..PENDING_ADD_CAP)
        .map(|n| endpoint_id(&format!("pending-{n}")))
        .collect();
    for id in &ids {
        store.add_pending_add(id).expect("under the bound");
    }

    let refused = store.add_pending_add(&endpoint_id("one-too-many"));
    assert!(
        matches!(refused, Err(StoreError::PendingAddBound)),
        "{refused:?}"
    );
    let err = refused.expect_err("refused");
    assert!(!err.needs_recovery(), "the bound is not recovery");
    assert_eq!(store.health(), StorageHealth::Healthy, "nor degradation");
    assert_eq!(store.pending_adds().expect("read"), ids, "no row evicted");

    store
        .add_pending_add(&ids[0])
        .expect("a row already held is accepted at the bound");
    store
        .add_pending_add(&ids[PENDING_ADD_CAP - 1])
        .expect("and the newest too");
    assert_eq!(
        store.pending_adds().expect("read"),
        ids,
        "still one row each"
    );

    assert!(store.forget_pending_add(&ids[10]).expect("one settled"));
    store
        .add_pending_add(&endpoint_id("one-too-many"))
        .expect("room again after a delete");
}

/// The record is the client's: a row written through one identity's
/// handle is what every other handle reads and deletes. Mutation: the
/// rows scoped by the handle's identity -- the second handle reads none.
#[test]
fn a_pending_add_is_one_for_every_handle() {
    let mut first =
        HumanStore::open_in_memory(&endpoint_id("human"), StoreOptions::default()).expect("opens");
    let mut second = first.handle_speaking_as(endpoint_id("second"));
    second
        .add_pending_add(&endpoint_id("new"))
        .expect("written by the second");
    assert_eq!(first.pending_adds().expect("read"), [endpoint_id("new")]);
    assert!(
        first
            .forget_pending_add(&endpoint_id("new"))
            .expect("deleted by the first")
    );
    assert_eq!(second.pending_adds().expect("read"), Vec::new());
}

/// A row that is not a derived `EndpointId` -- which this build does not
/// write -- is answered `Corrupt`. Mutation: a row that does not parse
/// skipped -- the read answers `Ok`.
#[test]
fn a_pending_add_row_that_is_no_endpoint_is_corrupt() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let store = open(&path);
    rusqlite::Connection::open(&path)
        .expect("a second connection")
        .execute(
            "INSERT INTO pending_adds (endpoint_id) VALUES ('human')",
            [],
        )
        .expect("a label, as no build of the store writes it");
    assert!(
        matches!(store.pending_adds(), Err(StoreError::Corrupt(_))),
        "{:?}",
        store.pending_adds()
    );
}
