// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The roster of identities (plan §30 step 1; architect-cto's rulings A1,
//! A2, A5 and A7 of 2026-11-25): a facade side, a client and a store
//! handle per held identity, over the fake network, with a spy around the
//! fake's bindings that records what each identity asked of the runtime.
//!
//! What this does NOT prove: a real daemon's connection limit (the spy
//! answers `Overloaded` where told), a real window, a model per identity.
//!
//! Every key here is TEST-ONLY (`interweave_test_support::endpoint_keys`).

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use interweave_human_app_core::{
    Command, FacadeSide, FrontChange, Roster, RosterConfig, RosterError, SwitchOutcome, Update,
};
use interweave_human_client_api::{ClientEvent, Connectivity, SessionState};
use interweave_human_store::{HumanStore, OwnCard, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{ConversationKey, IDENTITY_CAP};
use interweave_local_client_api::{
    AdminBinding, AdminCapability, DataSessionBinding, DataSessionPort, SessionEvent,
    SessionLosses, SessionRequest,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode, FakeSession};
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::{
    BroadcastMessageV1, ChannelId, DirectDestination, EndpointDirectoryV1, EndpointId, MessageId,
    Payload, TransportError, TransportIdentity,
};

const WALL: u64 = 1_786_600_000_000;

fn wall() -> u64 {
    WALL
}

fn human() -> EndpointId {
    endpoint_id("human")
}

fn second() -> EndpointId {
    endpoint_id("second")
}

fn third() -> EndpointId {
    endpoint_id("third")
}

fn room() -> ChannelId {
    ChannelId::parse("room").expect("valid")
}

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn node(endpoints: &[EndpointId]) -> FakeConfig {
    FakeConfig {
        peer: peer(),
        endpoints: endpoints
            .iter()
            .map(|e| FakeEndpoint::open(e.clone(), false))
            .collect(),
        default_endpoint: endpoints.first().cloned(),
        queue_bound: 16,
    }
}

/// What the runtime was asked, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Asked {
    /// A session open naming this endpoint (answered or not).
    Open(EndpointId),
    /// A session leasing this endpoint joined this channel.
    Join(EndpointId, ChannelId),
    /// A session leasing this endpoint was closed.
    Close(EndpointId),
    /// An admin connection asking these capabilities.
    Admin(BTreeSet<AdminCapability>),
}

/// The fake's bindings, recording every ask, and answering `Overloaded`
/// to an open of an endpoint in `overloaded`.
#[derive(Clone)]
struct Spy {
    node: FakeNode,
    asked: Arc<Mutex<Vec<Asked>>>,
    overloaded: Arc<Mutex<BTreeSet<EndpointId>>>,
}

impl Spy {
    fn new(node: FakeNode) -> Self {
        Self {
            node,
            asked: Arc::default(),
            overloaded: Arc::default(),
        }
    }

    fn asked(&self) -> Vec<Asked> {
        self.asked.lock().expect("log").clone()
    }

    fn opens(&self) -> Vec<EndpointId> {
        self.asked()
            .into_iter()
            .filter_map(|a| match a {
                Asked::Open(e) => Some(e),
                _ => None,
            })
            .collect()
    }

    fn status_admins(&self) -> usize {
        self.asked()
            .iter()
            .filter(|a| matches!(a, Asked::Admin(c) if c.contains(&AdminCapability::Status)))
            .count()
    }

    fn overload(&self, endpoint: &EndpointId, on: bool) {
        let mut set = self.overloaded.lock().expect("set");
        if on {
            set.insert(endpoint.clone());
        } else {
            set.remove(endpoint);
        }
    }

    fn log(&self, asked: Asked) {
        self.asked.lock().expect("log").push(asked);
    }
}

struct SpySession {
    inner: FakeSession,
    endpoint: Option<EndpointId>,
    asked: Arc<Mutex<Vec<Asked>>>,
}

impl DataSessionBinding for Spy {
    type Session = SpySession;

    async fn open(&self, request: SessionRequest) -> Result<SpySession, TransportError> {
        let endpoint = request.endpoint().cloned();
        if let Some(e) = &endpoint {
            self.log(Asked::Open(e.clone()));
            if self.overloaded.lock().expect("set").contains(e) {
                return Err(TransportError::Overloaded);
            }
        }
        let inner = self.node.open(request).await?;
        Ok(SpySession {
            inner,
            endpoint,
            asked: Arc::clone(&self.asked),
        })
    }
}

impl AdminBinding for Spy {
    type Admin = <FakeNode as AdminBinding>::Admin;

    async fn admin(
        &self,
        capabilities: BTreeSet<AdminCapability>,
    ) -> Result<Self::Admin, TransportError> {
        self.log(Asked::Admin(capabilities.clone()));
        self.node.admin(capabilities).await
    }
}

impl DataSessionPort for SpySession {
    fn session(&self) -> &interweave_local_client_api::LocalDataSession {
        self.inner.session()
    }

    async fn join(&self, channel: ChannelId) -> Result<(), TransportError> {
        if let Some(e) = &self.endpoint {
            self.asked
                .lock()
                .expect("log")
                .push(Asked::Join(e.clone(), channel.clone()));
        }
        self.inner.join(channel).await
    }

    async fn leave(&self, channel: ChannelId) -> Result<(), TransportError> {
        self.inner.leave(channel).await
    }

    async fn broadcast(
        &self,
        channel: ChannelId,
        message: BroadcastMessageV1,
    ) -> Result<(), TransportError> {
        self.inner.broadcast(channel, message).await
    }

    async fn send_direct(
        &self,
        destination: DirectDestination,
        message_id: MessageId,
        payload: Payload,
    ) -> Result<EndpointId, TransportError> {
        self.inner
            .send_direct(destination, message_id, payload)
            .await
    }

    async fn events(&self, max: usize) -> Result<Vec<SessionEvent>, TransportError> {
        self.inner.events(max).await
    }

    async fn ready(&self) -> Result<(), TransportError> {
        self.inner.ready().await
    }

    async fn query_endpoints(
        &self,
        peer: TransportIdentity,
    ) -> Result<EndpointDirectoryV1, TransportError> {
        self.inner.query_endpoints(peer).await
    }

    async fn losses(&self) -> Result<SessionLosses, TransportError> {
        self.inner.losses().await
    }

    async fn close(self) -> Result<(), TransportError> {
        if let Some(e) = &self.endpoint {
            self.asked
                .lock()
                .expect("log")
                .push(Asked::Close(e.clone()));
        }
        self.inner.close().await
    }
}

fn memory() -> HumanStore {
    HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store")
}

fn config(others: Vec<EndpointId>, default: Option<EndpointId>) -> RosterConfig {
    let mut endpoints = vec![human()];
    endpoints.extend(others.iter().cloned());
    RosterConfig {
        client_kind: "human-client".to_owned(),
        endpoints,
        channels: vec![room()],
        max_payload_bytes: 49_152,
        others,
        default,
    }
}

fn build(spy: &Spy, store: HumanStore, config: RosterConfig) -> Roster<Spy, Spy> {
    Roster::new(config, store, |_| (spy.clone(), spy.clone()), wall, 0).expect("a roster")
}

/// A card for `endpoint` made at `at`, written past the roster.
fn card(store: &mut HumanStore, endpoint: &EndpointId, at: u64) {
    store
        .set_own_card(
            &OwnCard {
                endpoint: endpoint.clone(),
                name: "x".to_owned(),
                surname: None,
                photo: None,
            },
            at,
        )
        .expect("a card");
}

fn session(roster: &Roster<Spy, Spy>, endpoint: &EndpointId) -> SessionState {
    roster
        .side(endpoint)
        .expect("held")
        .client()
        .session_state()
        .clone()
}

fn connectivity(updates: &[Update]) -> Vec<Update> {
    updates
        .iter()
        .filter(|u| matches!(u, Update::Client(ClientEvent::Connectivity(_))))
        .cloned()
        .collect()
}

/// One status reader, the first identity's, whatever the number held:
/// three identities over many status intervals open one admin connection
/// for status, and each identity is told the first's connectivity, none
/// its own. Mutations: the roster building every client with status --
/// three admin connections; `route` keeping each identity's own
/// connectivity and giving none of the first's -- the others are told
/// `Unknown` alone.
#[tokio::test]
async fn one_status_reader_whatever_the_number_held() {
    let (a, _b) = FakeNetwork::pair(node(&[human(), second(), third()]), node(&[human()]));
    let spy = Spy::new(a);
    let mut roster = build(&spy, memory(), config(vec![second(), third()], None));
    let mut told: Vec<(EndpointId, Vec<Update>)> = Vec::new();
    let mut now = 0;
    for _ in 0..6 {
        for (identity, updates) in roster.turn(now).await {
            match told.iter_mut().find(|(e, _)| e == &identity) {
                Some((_, all)) => all.extend(connectivity(&updates)),
                None => told.push((identity, connectivity(&updates))),
            }
        }
        now += 5_001;
    }
    assert_eq!(spy.status_admins(), 1, "{:?}", spy.asked());
    let first = told
        .iter()
        .find(|(e, _)| e == &human())
        .map(|(_, u)| u.clone())
        .expect("the first's");
    assert!(
        first.iter().any(|u| !matches!(
            u,
            Update::Client(ClientEvent::Connectivity(Connectivity::Unknown))
        )),
        "the first read status past `Unknown`: {first:?}"
    );
    for (identity, updates) in &told {
        assert_eq!(updates, &first, "{identity:?} is told the first's");
    }
}

/// The first identity's session joins the profile's channels and no other
/// identity's joins any. Mutation: every client given the channels --
/// three joins.
#[tokio::test]
async fn the_first_identity_alone_joins_the_channels() {
    let (a, _b) = FakeNetwork::pair(node(&[human(), second(), third()]), node(&[human()]));
    let spy = Spy::new(a);
    let mut roster = build(&spy, memory(), config(vec![second(), third()], None));
    for now in 0..3 {
        roster.turn(now).await;
    }
    for e in [human(), second(), third()] {
        assert!(
            matches!(session(&roster, &e), SessionState::Ready { .. }),
            "{e:?} is ready"
        );
    }
    let joins: Vec<Asked> = spy
        .asked()
        .into_iter()
        .filter(|a| matches!(a, Asked::Join(..)))
        .collect();
    assert_eq!(joins, [Asked::Join(human(), room())]);
}

/// The held order: the first identity, then by own card's creation, an
/// identity with no card after those with one, the id breaking a tie.
/// Mutation: the roster not sorting -- the order given is kept.
#[tokio::test]
async fn the_held_order_is_the_first_then_card_creation_then_id() {
    let (node_a, _node_b) = FakeNetwork::pair(node(&[human()]), node(&[human()]));
    let spy = Spy::new(node_a);
    let no_card = endpoint_id("no-card");
    let (tied, tied_too) = (endpoint_id("tied"), endpoint_id("tied-too"));
    let earliest = endpoint_id("earliest");
    let mut store = memory();
    card(&mut store, &tied, 5);
    card(&mut store, &tied_too, 5);
    card(&mut store, &earliest, 1);
    let roster = build(
        &spy,
        store,
        config(
            vec![
                no_card.clone(),
                tied.clone(),
                human(),
                tied_too.clone(),
                earliest.clone(),
                tied.clone(),
            ],
            None,
        ),
    );
    let (tie_first, tie_second) = if tied.as_str() < tied_too.as_str() {
        (tied, tied_too)
    } else {
        (tied_too, tied)
    };
    let held: Vec<EndpointId> = roster.held().cloned().collect();
    assert_eq!(held, [human(), earliest, tie_first, tie_second, no_card]);
}

/// Sessions open in one order: the identity in front, then the default,
/// then the rest in held order; with no row and no default, the held
/// order. Mutation: `order` answering the held order alone -- the first
/// case opens the first identity first.
#[tokio::test]
async fn sessions_open_front_then_default_then_held_order() {
    let all = [human(), second(), third()];
    // Held: human, third (card at 5), second (card at 10).
    let (a, _b) = FakeNetwork::pair(node(&all), node(&[human()]));
    let spy = Spy::new(a);
    let mut store = memory();
    card(&mut store, &second(), 10);
    card(&mut store, &third(), 5);
    store.set_last_front(&second()).expect("a row");
    let mut roster = build(&spy, store, config(vec![second(), third()], Some(third())));
    assert_eq!(roster.front(), &second());
    assert_eq!(roster.open_order(), [second(), third(), human()]);
    roster.turn(0).await;
    assert_eq!(spy.opens(), [second(), third(), human()]);

    let (a, _b) = FakeNetwork::pair(node(&all), node(&[human()]));
    let spy = Spy::new(a);
    let mut store = memory();
    card(&mut store, &second(), 10);
    card(&mut store, &third(), 5);
    let mut roster = build(&spy, store, config(vec![second(), third()], None));
    roster.turn(0).await;
    assert_eq!(spy.opens(), [human(), third(), second()]);
}

/// An identity whose open is answered `Overloaded` cannot act now: it
/// reconnects at the pace a lone client reconnects at, and no faster,
/// while the others are ready, and no other identity's session is closed
/// to make room; once a slot is free it opens. Mutation: `turn` stopping
/// at the first identity that is not ready -- the third never opens.
#[tokio::test]
async fn an_overloaded_identity_retries_at_its_pace_and_the_others_act() {
    let all = [human(), second(), third()];
    // What a lone client does with the same answer: the pace to hold to.
    let (a, _b) = FakeNetwork::pair(node(&all), node(&[human()]));
    let lone = Spy::new(a);
    lone.overload(&second(), true);
    let mut client = TransportClient::new(
        lone.clone(),
        lone.clone(),
        memory().speaking_as(second()),
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: all.to_vec(),
            channels: Vec::new(),
            max_payload_bytes: 49_152,
        },
        Box::new(wall),
        0,
    )
    .expect("a client")
    .without_status();
    client.tick(0).await;
    let SessionState::Reconnecting { next_at, .. } = client.session_state().clone() else {
        panic!("a lone client retries: {:?}", client.session_state());
    };
    assert!(next_at > 0);

    let (a, _b) = FakeNetwork::pair(node(&all), node(&[human()]));
    let node_a = a.clone();
    let spy = Spy::new(a);
    spy.overload(&second(), true);
    let mut roster = build(&spy, memory(), config(vec![second(), third()], None));
    roster.turn(0).await;
    assert_eq!(&session(&roster, &second()), client.session_state());
    assert!(matches!(
        session(&roster, &human()),
        SessionState::Ready { .. }
    ));
    assert!(matches!(
        session(&roster, &third()),
        SessionState::Ready { .. }
    ));

    let tries = |spy: &Spy| spy.opens().iter().filter(|e| **e == second()).count();
    roster.turn(next_at - 1).await;
    assert_eq!(tries(&spy), 1, "no faster than a lone client");
    roster.turn(next_at).await;
    assert_eq!(tries(&spy), 2, "retried when due");
    assert_eq!(node_a.open_sessions(), 2, "the others hold theirs");

    spy.overload(&second(), false);
    let SessionState::Reconnecting { next_at, .. } = session(&roster, &second()) else {
        panic!("still retrying");
    };
    roster.turn(next_at).await;
    for e in &all {
        assert!(
            matches!(session(&roster, e), SessionState::Ready { .. }),
            "{e:?}"
        );
    }
    assert!(
        !spy.asked().iter().any(|a| matches!(a, Asked::Close(_))),
        "no session closed for another: {:?}",
        spy.asked()
    );
    assert_eq!(node_a.open_sessions(), 3);
}

/// The identity in front at start: the last-front row's when it is held,
/// else the default when it is held, else the first; an unreadable row
/// is no row. Mutations: `resolve` taking the row's id whether or not it
/// is held -- the unknown row's case; `resolve` skipping the default --
/// the no-row case.
#[tokio::test]
async fn the_front_at_start_is_the_last_held_else_the_default_else_the_first() {
    let front = |row: Option<EndpointId>, default: Option<EndpointId>| {
        let (a, _b) = FakeNetwork::pair(node(&[human()]), node(&[human()]));
        let spy = Spy::new(a);
        let mut store = memory();
        if let Some(row) = row {
            store.set_last_front(&row).expect("a row");
        }
        build(&spy, store, config(vec![second(), third()], default))
            .front()
            .clone()
    };
    assert_eq!(front(Some(third()), Some(second())), third(), "held");
    assert_eq!(
        front(Some(endpoint_id("gone")), Some(second())),
        second(),
        "a row naming no identity held: the default"
    );
    assert_eq!(front(None, Some(second())), second(), "no row: the default");
    assert_eq!(
        front(None, Some(endpoint_id("route"))),
        human(),
        "a default that is no identity held: the first"
    );
    assert_eq!(front(None, None), human(), "neither: the first");

    // A row this build does not write, read as no row.
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let store = HumanStore::open(&path, &human(), StoreOptions::default()).expect("opens");
    rusqlite::Connection::open(&path)
        .expect("a second connection")
        .execute(
            "INSERT INTO settings (key, value) VALUES ('last_front', 'human')",
            [],
        )
        .expect("a label");
    let (a, _b) = FakeNetwork::pair(node(&[human()]), node(&[human()]));
    let spy = Spy::new(a);
    let roster = build(&spy, store, config(vec![second()], Some(second())));
    assert_eq!(roster.front(), &second(), "an unreadable row: the default");
}

/// The hooks of a switch, recording the order they ran in and what the
/// last-front row said when the surface was told.
struct Hooks {
    log: Vec<String>,
    store: HumanStore,
    rows_when_told: Vec<Option<EndpointId>>,
}

impl FrontChange for Hooks {
    fn drain(&mut self, old: &EndpointId) -> Vec<Command> {
        self.log.push(format!("drain {}", name(old)));
        vec![Command::Reopen]
    }

    fn answered(&mut self, identity: &EndpointId, _updates: Vec<Update>) {
        self.log.push(format!("answered {}", name(identity)));
    }

    fn close_switch_snackbar(&mut self, old: &EndpointId) {
        self.log.push(format!("close {}", name(old)));
    }

    fn front_changed(&mut self, old: &EndpointId, new: &EndpointId) {
        self.log
            .push(format!("front_changed {} {}", name(old), name(new)));
        self.rows_when_told
            .push(self.store.last_front().expect("read"));
    }
}

fn name(e: &EndpointId) -> &'static str {
    [("human", human()), ("second", second()), ("third", third())]
        .into_iter()
        .find(|(_, id)| id == e)
        .map_or("?", |(n, _)| n)
}

/// A switch drains the old front's pending acts and carries them out as
/// the old front, closes its switch snackbar, tells the surface, and only
/// then swaps and writes the row; a switch to the identity in front or to
/// one not held runs no hook. Mutations: the row written before the hooks
/// -- the surface is told with the new row in place; the snackbar closed
/// before the drained acts run -- the order differs.
#[tokio::test]
async fn a_switch_drains_acts_closes_tells_then_swaps() {
    let (a, _b) = FakeNetwork::pair(node(&[human(), second()]), node(&[human()]));
    let spy = Spy::new(a);
    let store = memory();
    let mut hooks = Hooks {
        log: Vec::new(),
        store: store.handle_speaking_as(human()),
        rows_when_told: Vec::new(),
    };
    let mut roster = build(&spy, store, config(vec![second()], None));
    roster.turn(0).await;
    assert_eq!(roster.front(), &human());

    assert!(matches!(
        roster.switch(&human(), &mut hooks, 1).await,
        SwitchOutcome::Unchanged
    ));
    assert!(matches!(
        roster.switch(&endpoint_id("gone"), &mut hooks, 1).await,
        SwitchOutcome::Unchanged
    ));
    assert!(hooks.log.is_empty(), "{:?}", hooks.log);

    let outcome = roster.switch(&second(), &mut hooks, 1).await;
    assert!(
        matches!(outcome, SwitchOutcome::Switched { remembered: Ok(()) }),
        "{outcome:?}"
    );
    assert_eq!(
        hooks.log,
        [
            "drain human",
            "answered human",
            "close human",
            "front_changed human second"
        ]
    );
    assert_eq!(hooks.rows_when_told, [None], "the row follows the hooks");
    assert_eq!(roster.front(), &second());
    assert_eq!(hooks.store.last_front().expect("read"), Some(second()));
    assert_eq!(roster.open_order()[0], second(), "the front opens first");
}

/// A roster holds at most `IDENTITY_CAP` identities, the first among
/// them. Mutation: the bound check off by one (`> IDENTITY_CAP + 1`) --
/// one more is built.
#[test]
fn more_identities_than_the_bound_are_refused() {
    let (a, _b) = FakeNetwork::pair(node(&[human()]), node(&[human()]));
    let spy = Spy::new(a);
    let others =
        |n: usize| -> Vec<EndpointId> { (0..n).map(|i| endpoint_id(&format!("id-{i}"))).collect() };
    let at_bound = Roster::new(
        config(others(IDENTITY_CAP - 1), None),
        memory(),
        |_| (spy.clone(), spy.clone()),
        wall,
        0,
    );
    assert_eq!(at_bound.expect("at the bound").held().len(), IDENTITY_CAP);
    let past = Roster::new(
        config(others(IDENTITY_CAP), None),
        memory(),
        |_| (spy.clone(), spy.clone()),
        wall,
        0,
    );
    assert!(matches!(past, Err(RosterError::OverBound)));
}

/// A value's variant name, from its debug form.
fn variant(debug: &str) -> String {
    let end = debug.find(['(', ' ', '{']).unwrap_or(debug.len());
    debug[..end].to_owned()
}

/// What a run answered, by kind and in order: each update's variant, a
/// client event's by its own. The values carry what differs between two
/// runs by construction -- peers, ids, times -- and the kinds do not.
fn shape(updates: &[Update]) -> Vec<String> {
    updates
        .iter()
        .map(|u| match u {
            Update::Client(event) => format!("Client::{}", variant(&format!("{event:?}"))),
            _ => variant(&format!("{u:?}")),
        })
        .collect()
}

/// Drives one identity through status reads, a session with its channel,
/// a send and further status intervals, by `step`, and says what it
/// answered and what the runtime was asked.
async fn run<F>(mut step: F) -> (Vec<String>, Vec<Asked>)
where
    F: AsyncStep,
{
    let mut all = Vec::new();
    let mut now = 0;
    for _ in 0..3 {
        all.extend(step.turn(now).await);
        now += 1;
    }
    let to = ConversationKey::Direct {
        peer: step.remote().clone(),
        endpoint: None,
    };
    all.extend(
        step.execute(
            Command::Send {
                key: to,
                draft: "hello".to_owned(),
            },
            now,
        )
        .await,
    );
    for _ in 0..3 {
        now += 5_001;
        all.extend(step.turn(now).await);
    }
    (shape(&all), step.asked())
}

/// One identity's way of being driven: alone, or as a roster of one.
trait AsyncStep {
    async fn turn(&mut self, now: u64) -> Vec<Update>;
    async fn execute(&mut self, command: Command, now: u64) -> Vec<Update>;
    fn remote(&self) -> &TransportIdentity;
    fn asked(&self) -> Vec<Asked>;
}

struct Alone {
    side: FacadeSide<Spy, Spy>,
    spy: Spy,
    remote: TransportIdentity,
}

impl AsyncStep for Alone {
    async fn turn(&mut self, now: u64) -> Vec<Update> {
        self.side.turn(now).await
    }
    async fn execute(&mut self, command: Command, now: u64) -> Vec<Update> {
        self.side.execute(command, now).await
    }
    fn remote(&self) -> &TransportIdentity {
        &self.remote
    }
    fn asked(&self) -> Vec<Asked> {
        self.spy.asked()
    }
}

struct OfOne {
    roster: Roster<Spy, Spy>,
    spy: Spy,
    remote: TransportIdentity,
}

impl OfOne {
    fn only(routed: Vec<(EndpointId, Vec<Update>)>) -> Vec<Update> {
        assert!(routed.len() <= 1, "one identity: {routed:?}");
        routed
            .into_iter()
            .flat_map(|(e, updates)| {
                assert_eq!(e, human());
                updates
            })
            .collect()
    }
}

impl AsyncStep for OfOne {
    async fn turn(&mut self, now: u64) -> Vec<Update> {
        Self::only(self.roster.turn(now).await)
    }
    async fn execute(&mut self, command: Command, now: u64) -> Vec<Update> {
        Self::only(
            self.roster
                .execute(&human(), command, now)
                .await
                .expect("held"),
        )
    }
    fn remote(&self) -> &TransportIdentity {
        &self.remote
    }
    fn asked(&self) -> Vec<Asked> {
        self.spy.asked()
    }
}

/// A roster of one, as the composition builds it in this batch, answers
/// what the facade side `main` composes answers -- the same updates in
/// the same order, the same asks of the runtime -- through status reads,
/// a session with its channel, a send and a status interval. Mutation:
/// the first identity's client built `without_status` -- no status read,
/// no connectivity past `Unknown`.
#[tokio::test]
async fn a_roster_of_one_behaves_as_its_facade_side() {
    let main_config = |channels| ClientConfig {
        client_kind: "human-client".to_owned(),
        endpoints: vec![human()],
        channels,
        max_payload_bytes: 49_152,
    };
    let (a, b) = FakeNetwork::pair(node(&[human()]), node(&[human()]));
    let spy = Spy::new(a);
    let client = TransportClient::new(
        spy.clone(),
        spy.clone(),
        memory(),
        main_config(vec![room()]),
        Box::new(wall),
        0,
    )
    .expect("a client");
    let alone = run(Alone {
        side: FacadeSide::new(client, wall),
        spy,
        remote: b.peer().clone(),
    })
    .await;

    let (a, b) = FakeNetwork::pair(node(&[human()]), node(&[human()]));
    let spy = Spy::new(a);
    let mut config = config(Vec::new(), Some(human()));
    config.endpoints = vec![human()];
    let of_one = run(OfOne {
        roster: build(&spy, memory(), config),
        spy,
        remote: b.peer().clone(),
    })
    .await;
    assert!(
        alone.0.iter().any(|s| s == "Sent"),
        "the run sends: {:?}",
        alone.0
    );
    assert_eq!(of_one, alone);
}
