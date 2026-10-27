// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the per-peer endpoint bound counts (HUMAN-CHAT.md's malformed
//! rule, STATE.md): every direct conversation the store holds rows of is
//! indexed with its first row, or at open for the rows already stored, and
//! a peer's endpoints are those conversations and its contacts' routes
//! together. The check that refuses a 65th is the transport client's.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use interweave_human_store::{
    AcceptedCard, AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, NewOutbound,
    OutboundDestination, StoreOptions,
};
use interweave_transport_api::{
    ChannelId, DirectDestination, EndpointId, MessageId, TransportIdentity,
};

const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
const Q: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";

fn peer(s: &str) -> TransportIdentity {
    TransportIdentity::parse(s).expect("a PeerId")
}

fn endpoint(s: &str) -> EndpointId {
    EndpointId::parse(s).expect("an endpoint")
}

fn id(n: u32) -> AppMessageId {
    AppMessageId::parse(format!("{n:032x}")).expect("an id")
}

fn from(n: u32, p: &str, e: Option<&str>, channel: Option<&str>) -> NewInbound {
    NewInbound {
        app_message_id: id(n),
        origin: InboundOrigin {
            peer: peer(p),
            endpoint: e.map(endpoint),
            channel: channel.map(|c| ChannelId::parse(c).expect("a channel")),
        },
        media_type: None,
        payload: vec![1],
        received_at: u64::from(n),
        kind: InboundKind::Text,
    }
}

fn to(n: u32, p: &str, e: Option<&str>) -> NewOutbound {
    NewOutbound {
        app_message_id: id(n),
        transport_message_id: MessageId::parse_hex(&format!("{:032x}", u128::from(n) + (1 << 100)))
            .expect("an id"),
        destination: OutboundDestination::Direct(DirectDestination {
            peer: peer(p),
            endpoint: e.map(endpoint),
        }),
        media_type: None,
        payload: vec![1],
        created_at: 1,
    }
}

fn store() -> (tempfile::TempDir, std::path::PathBuf, HumanStore) {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let store = HumanStore::open(&path, StoreOptions::default()).expect("opens");
    (dir, path, store)
}

/// A direct inbound row and a direct send to a named endpoint index their
/// conversation; a channel row, a send to the default endpoint, and a copy
/// refused as already read do not; a peer's endpoints are its indexed
/// conversations and its contacts' routes, each counted once, and another
/// peer's are not. Mutations: the inbound commit without the index
/// insert; the contact routes left out of the union.
#[test]
fn a_peers_endpoints_are_its_conversations_and_its_contacts_routes() {
    let (_dir, _path, mut store) = store();
    let (p, q) = (peer(P), peer(Q));
    assert_eq!(store.remote_endpoints(&p).expect("count"), 0);

    store
        .commit_unread_inbound(&from(1, P, Some("human"), None))
        .expect("direct");
    store
        .commit_unread_inbound(&from(2, P, Some("human"), None))
        .expect("the same conversation again");
    store
        .commit_unread_inbound(&from(3, P, None, Some("room")))
        .expect("a channel");
    store
        .commit_pending_outbound(&to(4, P, Some("phone")))
        .expect("a send");
    store
        .commit_pending_outbound(&to(5, P, None))
        .expect("to the default");
    store
        .commit_unread_inbound(&from(6, Q, Some("human"), None))
        .expect("Q's");
    assert_eq!(
        store.remote_endpoints(&p).expect("count"),
        2,
        "human and phone"
    );
    assert!(store.holds_endpoint(&p, &endpoint("human")).expect("held"));
    assert!(store.holds_endpoint(&p, &endpoint("phone")).expect("held"));
    assert!(
        !store
            .holds_endpoint(&p, &endpoint("tablet"))
            .expect("not held")
    );

    // A contact's route on a third endpoint, and one on a held one.
    store
        .accept_card(
            &AcceptedCard {
                peer: &p,
                endpoint: &endpoint("tablet"),
                name: "Ada",
                surname: None,
                identity_key: [1; 32],
                photo: None,
            },
            10,
        )
        .expect("a contact");
    assert_eq!(
        store.remote_endpoints(&p).expect("count"),
        3,
        "each endpoint once"
    );
    assert!(
        store
            .holds_endpoint(&p, &endpoint("tablet"))
            .expect("a route")
    );
    assert_eq!(store.remote_endpoints(&q).expect("count"), 1, "Q's own");
}

/// A store whose rows came before the index was written at commit -- every
/// store before this build -- has their direct conversations indexed at
/// open, once, from the rows' own columns. Mutation: the backfill skipped.
#[test]
fn rows_stored_before_the_index_are_indexed_at_open() {
    let (_dir, path, mut store) = store();
    store
        .commit_unread_inbound(&from(1, P, Some("human"), None))
        .expect("unread");
    let kept = store
        .commit_unread_inbound(&from(2, P, Some("phone"), None))
        .expect("to keep");
    let read = store.mark_read(kept, 3).expect("read");
    store.keep(&read, 4).expect("kept");
    store
        .commit_pending_outbound(&to(5, P, Some("tablet")))
        .expect("pending");
    drop(store);
    // As an earlier build left it: rows, no index, no version.
    let conn = rusqlite::Connection::open(&path).expect("a second connection");
    conn.execute_batch(
        "DELETE FROM conversation_index;
         DELETE FROM settings WHERE key = 'conversation_index_version';",
    )
    .expect("back before the index");
    drop(conn);

    let store = HumanStore::open(&path, StoreOptions::default()).expect("reopens");
    assert_eq!(store.remote_endpoints(&peer(P)).expect("count"), 3);
}
