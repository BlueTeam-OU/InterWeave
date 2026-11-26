// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identity last in front (STATE.md, one client process, several
//! identities, A 2026-11-25): one content-free `settings` row, the
//! `EndpointId` and nothing else, written at the switch. The client's, not
//! an identity's: every handle of one open reads the one row. What a
//! missing row, or one naming an identity the client does not hold,
//! resolves to is the root's (`interweave-human-app-core`'s roster), not
//! the store's.
//!
//! Every key here is TEST-ONLY (`interweave_test_support::endpoint_keys`).

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use interweave_human_store::{HumanStore, StoreError, StoreOptions};
use interweave_test_support::endpoint_keys::endpoint_id;

fn open(path: &std::path::Path) -> HumanStore {
    HumanStore::open(path, &endpoint_id("human"), StoreOptions::default()).expect("opens")
}

/// A store never switched holds no row; a write is read back, a second
/// write replaces it, and the row outlives a reopen. Mutation:
/// `set_last_front` writing under another key -- the read finds none.
#[test]
fn the_last_front_row_is_absent_then_written_replaced_and_kept() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let mut store = open(&path);
    assert_eq!(store.last_front().expect("read"), None, "never switched");

    store
        .set_last_front(&endpoint_id("second"))
        .expect("written");
    assert_eq!(
        store.last_front().expect("read"),
        Some(endpoint_id("second"))
    );
    store.set_last_front(&endpoint_id("human")).expect("again");
    assert_eq!(
        store.last_front().expect("read"),
        Some(endpoint_id("human")),
        "the newer switch replaces the row"
    );
    drop(store);

    let store = open(&path);
    assert_eq!(
        store.last_front().expect("read"),
        Some(endpoint_id("human")),
        "kept across a restart"
    );
    let rows: i64 = rusqlite::Connection::open(&path)
        .expect("a second connection")
        .query_row(
            "SELECT COUNT(*) FROM settings WHERE key = 'last_front'",
            [],
            |r| r.get(0),
        )
        .expect("count");
    assert_eq!(rows, 1, "one row, replaced in place");
}

/// The row is the client's: written through one identity's handle, it is
/// what every other handle of the open reads. Mutation: scoping the row
/// by the handle's identity (a key per identity) -- the second handle
/// reads none.
#[test]
fn the_last_front_row_is_one_for_every_handle() {
    let mut first =
        HumanStore::open_in_memory(&endpoint_id("human"), StoreOptions::default()).expect("opens");
    let mut second = first.handle_speaking_as(endpoint_id("second"));
    second
        .set_last_front(&endpoint_id("second"))
        .expect("written by the second");
    assert_eq!(
        first.last_front().expect("read"),
        Some(endpoint_id("second"))
    );
    first.set_last_front(&endpoint_id("human")).expect("back");
    assert_eq!(
        second.last_front().expect("read"),
        Some(endpoint_id("human"))
    );
}

/// A row that does not parse as a derived `EndpointId` -- which this build
/// does not write -- is answered `Corrupt`, never taken as an identity.
/// Mutation: `last_front` mapping a parse failure to `None` -- the read
/// answers `Ok(None)`.
#[test]
fn a_last_front_row_that_is_no_endpoint_is_corrupt() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let store = open(&path);
    rusqlite::Connection::open(&path)
        .expect("a second connection")
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('last_front', 'human')",
            [],
        )
        .expect("a label, as no build of the store writes it");
    assert!(
        matches!(store.last_front(), Err(StoreError::Corrupt(_))),
        "{:?}",
        store.last_front()
    );
}
