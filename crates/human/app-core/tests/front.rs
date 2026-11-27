// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! One model per held identity behind one surface (plan §30; ux ruling U2
//! of 2026-11-25): a read the identity behind the front asked lands in its
//! own model and draws nothing; switched back, its model shows it at once;
//! switched back while its read is still on its way, no second read is
//! asked. Over a real store and the fake network, the facade side's
//! answers handed to the model side by identity, as the desktop's facade
//! thread hands them.
//!
//! What this does NOT prove: a second identity's facade (the identity
//! switched to asks nothing here), a real window, the desktop's threads.

#![allow(clippy::expect_used, clippy::panic)]

use interweave_human_app_core::{
    Command, Crossing, FacadeSide, ModelSide, Opener, Surface, Update, WindowCommand,
};
use interweave_human_store::{
    AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, StoreOptions,
};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{ConversationKey, UiModel, ViewEvent, WindowInput};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::{EndpointId, MediaType, TransportIdentity};

const WALL: u64 = 1_786_600_000_000;

fn human() -> EndpointId {
    endpoint_id("human")
}

fn second() -> EndpointId {
    endpoint_id("second")
}

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: peer(),
        endpoints: vec![FakeEndpoint::open(human(), false)],
        default_endpoint: Some(human()),
        queue_bound: 16,
    }
}

/// `count` text messages from `from` to `human`, unread.
fn texts(from: &TransportIdentity, count: u32) -> HumanStore {
    let mut store = HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store");
    for n in 1..=count {
        let id = format!("{n:032x}");
        store
            .commit_unread_inbound(&NewInbound {
                app_message_id: AppMessageId::parse(id.clone()).expect("an id"),
                origin: InboundOrigin {
                    peer: from.clone(),
                    endpoint: Some(human()),
                    channel: None,
                },
                media_type: Some(
                    MediaType::parse("application/vnd.interweave-human-chat+json;v=2")
                        .expect("media"),
                ),
                payload: format!(
                    r#"{{"v":2,"kind":"text","app_message_id":"{id}","text":"message {n}"}}"#
                )
                .into_bytes(),
                received_at: u64::from(n),
                kind: InboundKind::Text,
            })
            .expect("committed");
    }
    store
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

/// Draws nothing, and remembers how many messages of `key` each render
/// showed.
struct Drawn {
    key: ConversationKey,
    shown: Vec<usize>,
}

impl Surface for Drawn {
    fn render(&mut self, model: &UiModel) {
        self.shown.push(model.messages(&self.key).len());
    }
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

fn pages(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|c| matches!(c, Command::Window(WindowCommand::FetchPage(_))))
        .count()
}

fn switch(model: &mut ModelSide<Drawn, NoOpener>, to: &EndpointId) -> Vec<Crossing> {
    let mut sent = Vec::new();
    assert!(model.switch(to, |c| sent.push(c)), "switched");
    sent
}

/// The first identity opens a conversation and its page read goes out;
/// the person switches to the second identity before it is answered,
/// and back while it is still on its way: no second read is asked. The
/// answer lands while the second is in front: into the first's model,
/// drawing nothing. Switched back, the first's model shows the page at
/// the first render. Mutations: `apply_as` applying to the front's model
/// -- the second's model holds the rows; the switch not carrying the
/// first's model (`exchange` leaving `model` out) -- the page is never
/// shown in front. What is in flight is each identity's own too
/// (`a_command_in_flight_is_its_identitys_own`).
#[tokio::test]
async fn a_read_behind_lands_in_its_model_and_a_switch_back_asks_no_second() {
    let from = peer();
    let key = ConversationKey::Direct {
        peer: from.clone(),
        endpoint: Some(human()),
    };
    let (a, _b) = FakeNetwork::pair(node(), node());
    let mut side = facade(&a, texts(&from, 10));
    let mut model = ModelSide::new(
        Drawn {
            key: key.clone(),
            shown: Vec::new(),
        },
        NoOpener,
    );
    assert!(model.hold(&[human(), second()], &human()));
    assert!(model.apply_as(
        &human(),
        Update::Started(Box::new(side.start().expect("started")))
    ));
    model.window(WindowInput::Opened(key.clone()));
    let asked = model.turn();
    assert_eq!(pages(&asked), 1, "the opening's page: {asked:?}");
    let page = asked
        .into_iter()
        .find(|c| matches!(c, Command::Window(WindowCommand::FetchPage(_))))
        .expect("asked");

    // To the second, and back, with the read still on its way.
    let there = switch(&mut model, &second());
    assert_eq!(there, [Crossing::Front(second())], "nothing else owed");
    let back = switch(&mut model, &human());
    assert_eq!(back, [Crossing::Front(human())]);
    assert_eq!(pages(&model.turn()), 0, "no second read");

    // Away again; the answer comes as the first's while the second shows.
    switch(&mut model, &second());
    let renders = model.surface().shown.len();
    for update in side.execute(page, 1).await {
        assert!(model.apply_as(&human(), update));
    }
    assert_eq!(model.surface().shown.len(), renders, "nothing drawn");
    assert!(model.model().messages(&key).is_empty(), "not the second's");
    assert_eq!(
        model.model_of(&human()).expect("held").messages(&key).len(),
        10,
        "the first's model holds the page"
    );

    switch(&mut model, &human());
    let asked = model.turn();
    assert_eq!(pages(&asked), 0, "answered: nothing to read again");
    assert_eq!(
        model.surface().shown.last(),
        Some(&10),
        "shown at the first render in front"
    );
}

/// A trust act made with the first identity in front is that identity's
/// (ADR-0067 decision 6): its set gains the peer, while the second
/// identity's and the operator's list do not; so the peer's direct to the
/// first is delivered and its direct to the second refused. Mutation:
/// the facade side's `SetTrust` calling the endpoint-less `set_trust`
/// -- the peer lands in the operator's list, which the second inherits.
#[tokio::test]
async fn trusting_in_front_of_a_lists_the_peer_under_a_alone() {
    use interweave_human_app_core::{Roster, RosterConfig};
    use interweave_local_client_api::{
        AdminBinding as _, AdminCapability, AdminPort as _, DataCapability,
        DataSessionBinding as _, DataSessionPort as _, SessionRequest, TrustChange as Change,
    };
    use interweave_transport_api::{DirectDestination, MessageId, Payload, TransportError};
    let (a, b) = FakeNetwork::pair(
        FakeConfig {
            endpoints: vec![
                FakeEndpoint::open(human(), false),
                FakeEndpoint::open(second(), false),
            ],
            ..node()
        },
        node(),
    );
    // The peer starts untrusted: the operator's list let go of it.
    let operator = a
        .admin([AdminCapability::Trust].into())
        .await
        .expect("a port");
    assert_eq!(
        operator.set_trust(b.peer().clone(), false).await,
        Ok(Change::Set)
    );
    let store = HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store");
    let mut roster = Roster::new(
        RosterConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![human(), second()],
            channels: Vec::new(),
            max_payload_bytes: 49_152,
            others: vec![second()],
            default: None,
            aliases: std::collections::BTreeMap::new(),
        },
        store,
        |_| (a.clone(), a.clone()),
        || WALL,
        0,
    )
    .expect("a roster");
    roster.turn(0).await;
    assert_eq!(roster.front(), &human());
    let routed = roster
        .execute(
            &human(),
            Command::SetTrust(interweave_human_ui_model::TrustChange {
                peer: b.peer().clone(),
                allowed: true,
            }),
            1,
        )
        .await
        .expect("held");
    let answer = routed[0].1.iter().find_map(|update| match update {
        Update::TrustSet { answer, .. } => Some(answer.clone()),
        _ => None,
    });
    assert!(
        matches!(&answer, Some(Ok(list)) if list.allows(b.peer())),
        "{answer:?}"
    );
    let routed = roster
        .execute(&second(), Command::ReadTrust, 2)
        .await
        .expect("held");
    let read = routed[0].1.iter().find_map(|update| match update {
        Update::TrustRead(read) => Some(read.clone()),
        _ => None,
    });
    assert!(
        matches!(&read, Some(Ok(list)) if !list.allows(b.peer())),
        "the second identity's set does not gain it: {read:?}"
    );
    assert!(a.operator_list().is_empty(), "nor the operator's list");

    let from = b
        .open(
            SessionRequest::new(
                "raw",
                Some(human()),
                [DataCapability::Commands, DataCapability::Events],
            )
            .expect("bounds"),
        )
        .await
        .expect("leases");
    let send = |to: EndpointId, n: u8| {
        from.send_direct(
            DirectDestination {
                peer: a.peer().clone(),
                endpoint: Some(to),
            },
            MessageId::from_bytes([n; 16]),
            Payload::at_ceiling(None, b"x".to_vec()).expect("bounds"),
        )
    };
    assert_eq!(send(human(), 1).await, Ok(human()), "to the first");
    assert_eq!(
        send(second(), 2).await,
        Err(TransportError::RemoteEndpointUnavailable),
        "to the second"
    );
}
