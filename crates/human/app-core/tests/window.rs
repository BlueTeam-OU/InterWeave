// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The windowed start (design/2026-10-27-start-window.md sections 3 and
//! 11) over a real store and the fake network: what the facade side reads
//! at a start, counted in rows, and what the model is given.
//!
//! What this does NOT prove: a memory figure, a real daemon, a real
//! window. A row count is what a test can hold; the time a start takes on
//! the phone is the device run's.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use interweave_human_app_core::{FacadeSide, ModelSide, Opener, Surface, Update};
use interweave_human_store::{
    AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, StoreOptions,
};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{ConversationKey, UiModel, ViewEvent};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, MediaType, TransportIdentity};

const WALL: u64 = 1_786_600_000_000;

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: peer(),
        endpoints: vec![FakeEndpoint::open(human(), false)],
        default_endpoint: Some(human()),
        queue_bound: 16,
    }
}

fn facade(node: &FakeNode, store: HumanStore) -> FacadeSide<FakeNode, FakeNode> {
    let client = TransportClient::new(
        node.clone(),
        node.clone(),
        store,
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![human()],
            channels: vec![],
            max_payload_bytes: 49_152,
        },
        Box::new(|| WALL),
        0,
    )
    .expect("facade");
    FacadeSide::new(client, || WALL)
}

/// Message `n` from `from`, a text envelope as the facade commits one. Its
/// kind is left `Other`, as a store no client has classified holds it.
fn message(n: u32, from: &TransportIdentity, at: u64) -> NewInbound {
    let id = format!("{n:032x}");
    NewInbound {
        app_message_id: AppMessageId::parse(id.clone()).expect("an id"),
        origin: InboundOrigin {
            peer: from.clone(),
            endpoint: Some(human()),
            channel: None,
        },
        media_type: Some(
            MediaType::parse("application/vnd.interweave-human-chat+json;v=2").expect("media"),
        ),
        payload: format!(r#"{{"v":2,"kind":"text","app_message_id":"{id}","text":"message {n}"}}"#)
            .into_bytes(),
        received_at: at,
        kind: InboundKind::Other,
    }
}

struct Blind;

impl Surface for Blind {
    fn render(&mut self, _model: &UiModel) {}
    fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
        Vec::new()
    }
}

struct NoOpener;

impl Opener for NoOpener {
    fn open(&mut self, _destination: &str) {}
    fn ask_network_access(&mut self) {}
    fn authenticate(&mut self) {}
}

const CONVERSATIONS: u32 = 3;
const PER_CONVERSATION: u32 = 300;

/// A start reads one row per conversation, its newest, and none of a
/// backlog: three conversations of 300 unread rows are started from three
/// rows, every badge the store's count, and the model holds no row of
/// them. The first start pays the kinds' classification once, every row;
/// a whole listing of the same store reads every row, the control that
/// shows the count is live. Mutation: the start summarising by listing
/// every row.
#[test]
fn a_start_reads_a_row_per_conversation_never_the_backlog() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let peers: Vec<TransportIdentity> = (0..CONVERSATIONS).map(|_| peer()).collect();
    {
        let mut store = HumanStore::open(&path, StoreOptions::default()).expect("store");
        let mut n = 0;
        for (i, from) in peers.iter().enumerate() {
            for at in 0..PER_CONVERSATION {
                n += 1;
                let at = u64::from(at) * 10 + u64::try_from(i).expect("small");
                store
                    .commit_unread_inbound(&message(n, from, at))
                    .expect("committed");
            }
        }
        let mut hidden = message(n + 1, &peers[0], 1);
        hidden.payload = b"no envelope".to_vec();
        store
            .commit_unread_inbound(&hidden)
            .expect("a row not shown");
    }
    let total = u64::from(CONVERSATIONS * PER_CONVERSATION) + 1;
    let (a, _b) = FakeNetwork::pair(node(), node());

    let mut first = facade(
        &a,
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
    );
    first.start().expect("started");
    assert!(
        first.rows_read() >= total,
        "the first start classifies every row: {}",
        first.rows_read()
    );
    drop(first);

    let mut side = facade(
        &a,
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
    );
    let start = side.start().expect("started");
    assert_eq!(
        side.rows_read(),
        u64::from(CONVERSATIONS),
        "one row per conversation"
    );
    assert_eq!(start.summaries.len(), peers.len());
    assert_eq!(start.undecodable_unread, 1);
    assert!(start.contacts.is_ok());

    let mut model = ModelSide::new(Blind, NoOpener);
    model.apply(Update::Started(Box::new(start)));
    assert!(model.model().windowed());
    assert!(model.model().is_empty(), "no row of a conversation is held");
    for from in &peers {
        let key = ConversationKey::Direct {
            peer: from.clone(),
            endpoint: Some(human()),
        };
        let listed = model
            .model()
            .conversations()
            .into_iter()
            .find(|c| c.key == key)
            .expect("listed");
        assert_eq!(
            listed.unread,
            usize::try_from(PER_CONVERSATION).expect("small")
        );
        assert!(!listed.last.preview.shown().is_empty());
    }
    assert_eq!(model.hidden_rows(), 1);

    let before = side.rows_read();
    side.listing().expect("listed");
    assert!(
        side.rows_read() - before >= total,
        "a whole listing reads every row"
    );
}
