// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Several identities on one open (Stage 22, ADR-0061): the store is opened
//! once and each identity holds a handle on its one connection
//! (`HumanStore::handle_speaking_as`). Each handle reads and writes its own
//! identity's rows, a release under one still empties the log while the
//! other lives, and the store's health is one for every handle. That the
//! handles are one connection, and that their calls never interleave, is
//! pinned beside the handle (`store.rs`, `handle_tests`).
//!
//! Every key here is TEST-ONLY (`interweave_test_support::endpoint_keys`).

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::path::{Path, PathBuf};

use interweave_human_core::retention::{StorageHealth, TerminalCause};
use interweave_human_store::{
    AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, NewOutbound,
    OutboundDestination, StoreError, StoreOptions,
};
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::{DirectDestination, EndpointId, MessageId, TransportIdentity};

/// The first identity: the id the client hands to the open.
fn me() -> EndpointId {
    endpoint_id("me")
}

/// Another identity of the same node.
fn other() -> EndpointId {
    endpoint_id("other")
}

fn peer() -> TransportIdentity {
    TransportIdentity::parse(endpoint_id("node-p").as_str()).expect("a PeerId")
}

fn id(n: u32) -> AppMessageId {
    AppMessageId::parse(format!("{n:032x}")).expect("an id")
}

fn inbound(n: u32) -> NewInbound {
    NewInbound {
        app_message_id: id(n),
        origin: InboundOrigin {
            peer: peer(),
            endpoint: Some(endpoint_id("alice")),
            channel: None,
        },
        media_type: None,
        payload: n.to_be_bytes().to_vec(),
        received_at: u64::from(n),
        kind: InboundKind::Text,
    }
}

fn outbound(n: u32, payload: Vec<u8>) -> NewOutbound {
    NewOutbound {
        app_message_id: id(n),
        transport_message_id: MessageId::parse_hex(&format!("{:032x}", u128::from(n) + (1 << 100)))
            .expect("an id"),
        destination: OutboundDestination::Direct(DirectDestination {
            peer: peer(),
            endpoint: Some(endpoint_id("alice")),
        }),
        media_type: None,
        payload,
        created_at: u64::from(n),
    }
}

fn store_path(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("state").join("human.sqlite3")
}

fn wal(path: &Path) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push("-wal");
    PathBuf::from(p)
}

fn unread_ids(store: &HumanStore) -> Vec<AppMessageId> {
    let mut ids: Vec<_> = store
        .unread_inbound()
        .expect("unread")
        .into_iter()
        .map(|r| r.app_message_id)
        .collect();
    ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    ids
}

fn pending_ids(store: &HumanStore) -> Vec<AppMessageId> {
    let mut ids: Vec<_> = store
        .pending_outbound()
        .expect("pending")
        .into_iter()
        .map(|r| r.app_message_id)
        .collect();
    ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    ids
}

/// Two handles of one open, both alive and their calls alternating: each
/// lists what it wrote and none of the other's, and the first, reopened
/// alone, finds its own rows as they were. Mutation: `HumanStore::lock`
/// without the line that scopes the core to the calling handle's identity
/// -- the second handle's rows are written as the first's.
#[test]
fn two_live_handles_read_and_write_their_own_identitys_rows_alone() {
    let dir = common::private_tempdir();
    let path = store_path(&dir);
    let mut first = HumanStore::open(&path, &me(), StoreOptions::default()).expect("me");
    let mut second = first.handle_speaking_as(other());
    assert_eq!(first.identity(), &me());
    assert_eq!(second.identity(), &other());

    first.commit_unread_inbound(&inbound(1)).expect("mine");
    second
        .commit_unread_inbound(&inbound(2))
        .expect("the other's");
    first
        .commit_pending_outbound(&outbound(3, vec![3]))
        .expect("my send");
    second
        .commit_pending_outbound(&outbound(4, vec![4]))
        .expect("the other's send");
    // The same message reaching both identities is a row for each.
    second
        .commit_unread_inbound(&inbound(1))
        .expect("the other's copy");

    assert_eq!(unread_ids(&first), vec![id(1)], "mine alone");
    assert_eq!(unread_ids(&second), vec![id(1), id(2)], "the other's alone");
    assert_eq!(pending_ids(&first), vec![id(3)]);
    assert_eq!(pending_ids(&second), vec![id(4)]);

    // A release under one leaves the other's copy of the same message.
    let mine = first.unread_inbound().expect("unread")[0].row_id;
    first.mark_read(mine, 10).expect("read");
    assert!(unread_ids(&first).is_empty());
    assert_eq!(unread_ids(&second), vec![id(1), id(2)]);
    drop((first, second));

    let first = HumanStore::open(&path, &me(), StoreOptions::default()).expect("me again");
    assert!(unread_ids(&first).is_empty(), "my read row stays read");
    assert_eq!(pending_ids(&first), vec![id(3)], "my send alone");
    let second = first.handle_speaking_as(other());
    assert_eq!(unread_ids(&second), vec![id(1), id(2)]);
    assert_eq!(pending_ids(&second), vec![id(4)]);
}

/// A release under one handle while the other is alive -- and has just
/// written, so the log holds its pages -- still checkpoints and truncates
/// the log before it returns: the `-wal` file is empty. Mutation: the
/// release's truncate (`release_done`) made a no-op -- the log keeps the
/// pages.
#[test]
fn a_release_under_one_handle_truncates_the_log_while_another_lives() {
    let dir = common::private_tempdir();
    let path = store_path(&dir);
    let mut first = HumanStore::open(&path, &me(), StoreOptions::default()).expect("me");
    let mut second = first.handle_speaking_as(other());

    let mine = first.commit_unread_inbound(&inbound(1)).expect("mine");
    second
        .commit_unread_inbound(&inbound(2))
        .expect("the other's");
    assert!(
        std::fs::metadata(wal(&path)).expect("a log").len() > 0,
        "control: the commits are in the log"
    );
    first.mark_read(mine, 10).expect("released");
    assert_eq!(
        std::fs::metadata(wal(&path)).expect("a log").len(),
        0,
        "the release truncated the log with the other handle alive"
    );
    assert_eq!(unread_ids(&second), vec![id(2)], "the other's row stays");
}

/// The store's health is one: a send under the first handle that fills
/// the quota degrades the store for the second too, which then takes no
/// unread content; and a recheck under the second, once the first's sends
/// are released, clears it for the first. Mutation: `handle_speaking_as`
/// building its handle on a fresh core of its own (a second connection,
/// with its own health) -- the second reads healthy.
#[test]
fn a_degraded_store_is_degraded_for_every_handle() {
    let dir = common::private_tempdir();
    let path = store_path(&dir);
    let mut first = HumanStore::open(
        &path,
        &me(),
        StoreOptions {
            max_pages: Some(64),
        },
    )
    .expect("opens");
    let mut second = first.handle_speaking_as(other());

    let mut sent = Vec::new();
    let mut refused = None;
    for n in 1..=8 {
        let payload = vec![0_u8; interweave_transport_api::MAX_PAYLOAD_BYTES];
        match first.commit_pending_outbound(&outbound(n, payload)) {
            Ok(row) => sent.push(row),
            Err(e) => {
                refused = Some(e);
                break;
            }
        }
    }
    let refused = refused.expect("8 x 48 KiB cannot fit 64 pages");
    assert!(matches!(refused, StoreError::Sql(_)), "{refused:?}");
    assert!(!sent.is_empty(), "control: the quota admitted a send first");
    assert_eq!(first.health(), StorageHealth::Degraded);

    assert_eq!(
        second.health(),
        StorageHealth::Degraded,
        "the other identity's handle sees the store degraded"
    );
    let next = second
        .commit_unread_inbound(&inbound(100))
        .expect_err("a degraded store takes no unread content for any identity");
    assert!(matches!(next, StoreError::Degraded), "{next:?}");

    for row in sent {
        first
            .transport_terminal(row, TerminalCause::Accepted)
            .expect("released");
    }
    assert_eq!(
        second.recheck_health().expect("the probe commits"),
        StorageHealth::Healthy
    );
    assert_eq!(
        first.health(),
        StorageHealth::Healthy,
        "a recheck under one handle clears it for every handle"
    );
}
