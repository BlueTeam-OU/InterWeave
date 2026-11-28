// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The roster carries an add form's Create, its leaving and the settle
//! of pending-add records (`design/2026-11-04-identities.md` §6.2;
//! ADR-0061 decision 7; architect-cto's rulings of 2026-11-27): over the
//! fake network for the sessions, an admin binding whose endpoint list,
//! add, remove, seed and advertise calls are scripted beside the fake's
//! own, a real store and a real `endpoint-keys/` under a private
//! directory.
//!
//! What this does NOT prove: the daemon's add and remove (the scripted
//! port answers as the contract says; `crates/transport/composition`
//! proves the daemon's), a window, the desktop's facade thread.
//!
//! Every key here is TEST-ONLY (`interweave_test_support::endpoint_keys`,
//! or generated into a temporary directory).

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use interweave_human_app_core::{
    AddAct, AddFactory, AddTold, CardSaved, Command, Roster, RosterConfig, Update,
};
use interweave_human_store::identity_key::{self, FilesystemCustody};
use interweave_human_store::{HumanStore, PENDING_ADD_CAP, StoreOptions};
use interweave_human_transport_client::{ADD_WAIT_MS, AddOutcome};
use interweave_local_client_api::{
    AdminBinding, AdminCapability, AdminPort, AdminStatus, DataCapability, DataSessionBinding,
    DataSessionPort as _, EndpointAddChange, EndpointAdminView, EndpointRemoveChange,
    EndpointSource, Generation, LocalAdminPort, SessionRequest, TrustAdminView, TrustChange,
    TrustSeed,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode, FakeSession};
use interweave_profile_config::paths::ProfilePaths;
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::{
    DirectDestination, EndpointId, MediaType, MessageId, Payload, TransportError, TransportIdentity,
};

const WALL: u64 = 1_786_600_000_000;

fn wall() -> u64 {
    WALL
}

fn human() -> EndpointId {
    endpoint_id("human")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: ProfileIdentity::generate()
            .transport_identity()
            .expect("peer id"),
        endpoints: vec![FakeEndpoint::open(human(), false)],
        default_endpoint: Some(human()),
        queue_bound: 16,
    }
}

/// One thing asked of the runtime, in the order asked.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    /// An admin connection opened, holding these.
    Opened(BTreeSet<AdminCapability>),
    /// `admin.endpoints.list` asked.
    Listed,
    /// `admin.endpoints.add` asked with this alias and these kinds.
    Added { alias: String, kinds: Vec<String> },
    /// `admin.endpoints.remove` asked.
    Removed(EndpointId),
    /// `admin.trust.seed` asked.
    Seeded(EndpointId),
    /// An endpoint's `advertise` set.
    Advertised(EndpointId, bool),
    /// A session open naming this endpoint.
    Open(EndpointId),
}

/// How the next call of a kind is answered.
#[derive(Debug, Clone)]
enum Reply<T> {
    Now(T),
    /// Never answered.
    Holds,
}

#[derive(Default)]
struct Script {
    steps: Vec<Step>,
    /// What a list answers.
    listed: Vec<EndpointAdminView>,
    /// The next adds' answers; once empty, `Added`.
    adds: VecDeque<Reply<Result<EndpointAddChange, TransportError>>>,
    /// The next seeds' answers; once empty, `seeded`.
    seeds: VecDeque<Reply<Result<TrustSeed, TransportError>>>,
}

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Script>>);

impl Log {
    fn with<T>(&self, f: impl FnOnce(&mut Script) -> T) -> T {
        f(&mut self.0.lock().unwrap())
    }

    fn steps(&self) -> Vec<Step> {
        self.with(|s| s.steps.clone())
    }

    fn push(&self, step: Step) {
        self.with(|s| s.steps.push(step));
    }

    fn count(&self, f: impl Fn(&Step) -> bool) -> usize {
        self.steps().iter().filter(|s| f(s)).count()
    }
}

async fn answer<T>(reply: Reply<T>) -> T {
    match reply {
        Reply::Now(answer) => answer,
        Reply::Holds => std::future::pending().await,
    }
}

/// The fake's data sessions, each open logged.
#[derive(Clone)]
struct Data {
    node: FakeNode,
    log: Log,
}

impl DataSessionBinding for Data {
    type Session = FakeSession;

    async fn open(&self, request: SessionRequest) -> Result<FakeSession, TransportError> {
        if let Some(endpoint) = request.endpoint() {
            self.log.push(Step::Open(endpoint.clone()));
        }
        self.node.open(request).await
    }
}

/// The fake's admin connections, with the endpoint list, add, remove,
/// seed and advertise answered as scripted.
#[derive(Clone)]
struct Admins {
    node: FakeNode,
    log: Log,
}

struct Port {
    inner: <FakeNode as AdminBinding>::Admin,
    log: Log,
}

impl AdminBinding for Admins {
    type Admin = Port;

    async fn admin(&self, capabilities: BTreeSet<AdminCapability>) -> Result<Port, TransportError> {
        self.log.push(Step::Opened(capabilities.clone()));
        Ok(Port {
            inner: self.node.admin(capabilities).await?,
            log: self.log.clone(),
        })
    }
}

impl AdminPort for Port {
    fn port(&self) -> &LocalAdminPort {
        self.inner.port()
    }
    async fn status(&self) -> Result<AdminStatus, TransportError> {
        self.inner.status().await
    }
    fn leases(
        &self,
    ) -> impl Future<Output = Result<Vec<EndpointAdminView>, TransportError>> + Send {
        std::future::ready(self.log.with(|s| {
            s.steps.push(Step::Listed);
            Ok(s.listed.clone())
        }))
    }
    async fn add_endpoint(
        &self,
        key: String,
        alias: String,
        allowed_client_kinds: Vec<String>,
    ) -> Result<EndpointAddChange, TransportError> {
        let id = interweave_profile_config::endpoint_key::parse_key_file_name(&key)
            .expect("a key file's name");
        let reply = self.log.with(|s| {
            s.steps.push(Step::Added {
                alias,
                kinds: allowed_client_kinds,
            });
            s.adds
                .pop_front()
                .unwrap_or(Reply::Now(Ok(EndpointAddChange::Added(id))))
        });
        answer(reply).await
    }
    fn remove_endpoint(
        &self,
        endpoint: EndpointId,
    ) -> impl Future<Output = Result<EndpointRemoveChange, TransportError>> + Send {
        self.log.push(Step::Removed(endpoint));
        std::future::ready(Ok(EndpointRemoveChange::Removed {
            revoked_epoch: None,
        }))
    }
    fn set_endpoint_advertise(
        &self,
        endpoint: EndpointId,
        advertise: bool,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        self.log.push(Step::Advertised(endpoint, advertise));
        std::future::ready(Ok(()))
    }
    async fn seed_endpoint_trust(&self, endpoint: EndpointId) -> Result<TrustSeed, TransportError> {
        let reply = self.log.with(|s| {
            s.steps.push(Step::Seeded(endpoint));
            s.seeds
                .pop_front()
                .unwrap_or(Reply::Now(Ok(TrustSeed::Seeded)))
        });
        answer(reply).await
    }
    async fn revoke_endpoint(&self, endpoint: EndpointId) -> Result<(), TransportError> {
        self.inner.revoke_endpoint(endpoint).await
    }
    async fn set_endpoint_enabled(
        &self,
        endpoint: EndpointId,
        enabled: bool,
    ) -> Result<Option<Generation>, TransportError> {
        self.inner.set_endpoint_enabled(endpoint, enabled).await
    }
    async fn set_default_endpoint(
        &self,
        endpoint: Option<EndpointId>,
    ) -> Result<(), TransportError> {
        self.inner.set_default_endpoint(endpoint).await
    }
    async fn trust(&self) -> Result<TrustAdminView, TransportError> {
        self.inner.trust().await
    }
    async fn set_trust(
        &self,
        peer: TransportIdentity,
        allowed: bool,
    ) -> Result<TrustChange, TransportError> {
        self.inner.set_trust(peer, allowed).await
    }
    async fn shutdown(&self, grace: Duration) -> Result<(), TransportError> {
        self.inner.shutdown(grace).await
    }
}

/// One test's device: the node, the log, a private profile directory.
struct Device {
    node: FakeNode,
    peer: FakeNode,
    log: Log,
    _dir: tempfile::TempDir,
    paths: ProfilePaths,
    /// A handle of the roster's store open, read past the roster.
    rows: Option<HumanStore>,
}

impl Device {
    fn new() -> Self {
        let (node, peer) = FakeNetwork::pair(node(), node());
        let dir = common::private_tempdir();
        let paths = ProfilePaths::resolve_embedded(
            "default",
            interweave_profile_config::TrustBoundary::new(dir.path()).expect("a boundary"),
        )
        .expect("paths");
        Self {
            node,
            peer,
            log: Log::default(),
            _dir: dir,
            paths,
            rows: None,
        }
    }

    /// Every id the pending-add record names now.
    fn records(&self) -> Vec<EndpointId> {
        self.rows
            .as_ref()
            .expect("a roster built")
            .pending_adds()
            .expect("the record reads")
    }

    /// An identity's own card, as the store holds it now.
    fn card(
        &self,
        endpoint: &EndpointId,
    ) -> Result<Option<interweave_human_store::OwnCard>, interweave_human_store::StoreError> {
        self.rows
            .as_ref()
            .expect("a roster built")
            .own_card(endpoint)
    }

    fn data(&self) -> Data {
        Data {
            node: self.node.clone(),
            log: self.log.clone(),
        }
    }

    fn admins(&self) -> Admins {
        Admins {
            node: self.node.clone(),
            log: self.log.clone(),
        }
    }

    /// The roster over `store`, the identities in `others` besides the
    /// first, able to hold an identity made while it runs.
    fn roster(&mut self, store: HumanStore, others: Vec<EndpointId>) -> Roster<Data, Admins> {
        self.rows = Some(store.handle_speaking_as(human()));
        let (data, admins) = (self.data(), self.admins());
        let mut endpoints = vec![human()];
        endpoints.extend(others.iter().cloned());
        let (d, a) = (data.clone(), admins.clone());
        Roster::new(
            RosterConfig {
                client_kind: "human-client".to_owned(),
                endpoints,
                channels: Vec::new(),
                max_payload_bytes: 49_152,
                others,
                default: None,
                aliases: BTreeMap::new(),
            },
            store,
            move |_| (d.clone(), a.clone()),
            wall,
            0,
        )
        .expect("a roster")
        .adding(AddFactory {
            bindings: Box::new(move |_| (data.clone(), admins.clone())),
            admin: Arc::new(self.admins()),
            paths: self.paths.clone(),
            cipher: Arc::new(FilesystemCustody),
        })
    }

    fn script(&self, f: impl FnOnce(&mut Script)) {
        self.log.with(f);
    }

    fn key_file(&self, endpoint: &EndpointId) -> bool {
        identity_key::key_path(&self.paths, endpoint).exists()
    }

    /// A key sealed in `endpoint-keys/` and a record naming it, as a
    /// form whose add ended unknown left them.
    fn left_by_a_form(&self, store: &mut HumanStore) -> EndpointId {
        let id = identity_key::create(&self.paths, &FilesystemCustody)
            .expect("a key")
            .endpoint()
            .clone();
        store.add_pending_add(&id).expect("a record");
        id
    }
}

fn memory() -> HumanStore {
    HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store")
}

fn view(endpoint: &EndpointId, alias: &str) -> EndpointAdminView {
    EndpointAdminView {
        endpoint: endpoint.clone(),
        enabled: true,
        default: false,
        persisted: true,
        lease: None,
        alias: Some(alias.to_owned()),
        source: EndpointSource::Added,
        advertise: false,
    }
}

fn create(name: &str) -> AddAct {
    AddAct::Create {
        name: name.to_owned(),
        surname: Some("Rossi".to_owned()),
        photo: None,
    }
}

/// The roster's first turns, done: the first identity open, and the
/// settle at the first turn over.
async fn started(roster: &mut Roster<Data, Admins>) {
    for now in 0..3 {
        roster.turn(now).await;
    }
}

/// An add form opened and Create offered, the seed answered at once.
fn offered(roster: &mut Roster<Data, Admins>, now: u64) {
    roster.add(AddAct::Opened, now);
    assert_eq!(
        roster.take_add_told(),
        [AddTold::WaitsSeed, AddTold::Offered],
        "asked and answered at once"
    );
    roster.add(AddAct::Opened, now);
    assert_eq!(roster.take_add_told(), [AddTold::Offered], "seeded now");
}

fn created(told: &[AddTold]) -> (EndpointId, Option<String>, Option<CardSaved>) {
    told.iter()
        .find_map(|t| match t {
            AddTold::Ended {
                outcome: AddOutcome::Created { endpoint, alias },
                card,
            } => Some((endpoint.clone(), alias.clone(), *card)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("created: {told:?}"))
}

/// Create is the gated control, not the form: the form opens in a run
/// where no seed answered, asks for the seed itself on an `admin.trust`
/// connection of its own, and Create is not offered meanwhile -- a Create
/// then makes nothing, writes no record and sends no add. The ask
/// unanswered for the wait ends not offered; opening the form again asks
/// once more, and once that answers Create is offered, the run's flag
/// set, and a Create runs. Mutations: `AddAct::Create` carried out
/// without the gate -- an add is sent before the seed answered; the open
/// telling `Offered` whatever the seed -- the first told is wrong.
#[tokio::test]
async fn add_is_offered_only_once_the_first_seed_answered() {
    let mut device = Device::new();
    device.script(|s| s.seeds.push_back(Reply::Holds));
    let mut roster = device.roster(memory(), Vec::new());
    started(&mut roster).await;
    assert!(!roster.first_seeded(), "the premise: no seed answered yet");

    roster.add(AddAct::Opened, 10);
    assert_eq!(roster.take_add_told(), [AddTold::WaitsSeed]);
    assert_eq!(
        device
            .log
            .count(|s| matches!(s, Step::Seeded(e) if e == &human())),
        1,
        "{:?}",
        device.log.steps()
    );
    roster.add(create("Anna"), 11);
    assert_eq!(roster.take_add_told(), [AddTold::WaitsSeed]);
    roster.turn(10 + ADD_WAIT_MS - 1).await;
    assert!(roster.take_add_told().is_empty(), "still waited for");
    assert_eq!(device.log.count(|s| matches!(s, Step::Added { .. })), 0);
    assert!(device.records().is_empty(), "no record written");

    roster.turn(10 + ADD_WAIT_MS).await;
    assert_eq!(roster.take_add_told(), [AddTold::NoSeed]);
    roster.add(create("Anna"), 10 + ADD_WAIT_MS);
    assert_eq!(roster.take_add_told(), [AddTold::NoSeed]);
    assert_eq!(device.log.count(|s| matches!(s, Step::Added { .. })), 0);

    // Opened again: asked once more, answered this time.
    let now = 20 + ADD_WAIT_MS;
    roster.add(AddAct::Opened, now);
    assert_eq!(
        roster.take_add_told(),
        [AddTold::WaitsSeed, AddTold::Offered]
    );
    assert!(roster.first_seeded(), "the run's flag");
    roster.add(create("Anna"), now);
    let (endpoint, _, _) = created(&roster.take_add_told());
    assert!(roster.held().any(|e| e == &endpoint));
}

/// F25's second case at the engine (identities §6.2, "Create waits for
/// the first identity's seed"): after the form's own ask ended `NoSeed`,
/// a trust act of the first identity answers the seed on its own
/// connection, and the next poll tells `Offered`, once -- later polls
/// tell nothing more -- with no second ask of the form's; a Create then
/// runs. Mutation: the check for a seed answered elsewhere removed --
/// nothing is told after the trust act.
#[tokio::test]
async fn a_seed_answered_on_another_connection_offers_create_on_the_open_form() {
    let mut device = Device::new();
    device.script(|s| s.seeds.push_back(Reply::Holds));
    let mut roster = device.roster(memory(), Vec::new());
    started(&mut roster).await;
    roster.add(AddAct::Opened, 10);
    assert_eq!(roster.take_add_told(), [AddTold::WaitsSeed]);
    roster.turn(10 + ADD_WAIT_MS).await;
    assert_eq!(roster.take_add_told(), [AddTold::NoSeed]);
    assert!(!roster.first_seeded(), "the premise: nothing answered yet");

    let now = 20 + ADD_WAIT_MS;
    roster
        .execute(&human(), Command::ReadTrust, now)
        .await
        .expect("held");
    assert!(
        roster.first_seeded(),
        "answered on the trust act's connection"
    );
    let seeds = device.log.count(|s| matches!(s, Step::Seeded(_)));
    roster.turn(now + 1).await;
    assert_eq!(roster.take_add_told(), [AddTold::Offered]);
    roster.turn(now + 2).await;
    assert!(roster.take_add_told().is_empty(), "told once");
    assert_eq!(
        device.log.count(|s| matches!(s, Step::Seeded(_))),
        seeds,
        "the form asked nothing more"
    );
    roster.add(create("Anna"), now + 3);
    let (endpoint, _, _) = created(&roster.take_add_told());
    assert!(roster.held().any(|e| e == &endpoint));
}

/// A Create the daemon answers `added` deletes the record, writes the new
/// identity's card -- name, surname -- holds it at once and leaves the
/// front as it was; the add sent the alias the name makes and the
/// client's kind, and the new identity is asserted unadvertised before
/// its lease. Mutations: the card not written -- the card reads none;
/// `hold_added` not called -- not held; the front set to it -- the front
/// changes.
#[tokio::test]
async fn a_create_that_lands_deletes_the_record_holds_the_identity_and_writes_its_card() {
    let mut device = Device::new();
    let mut roster = device.roster(memory(), Vec::new());
    started(&mut roster).await;
    offered(&mut roster, 5);
    roster.add(create("Anna"), 5);
    let told = roster.take_add_told();
    let (endpoint, alias, card) = created(&told);
    assert_eq!(alias.as_deref(), Some("anna"));
    assert_eq!(card, Some(CardSaved::Saved));
    assert!(told.contains(&AddTold::Held(endpoint.clone())), "{told:?}");
    assert!(device.log.steps().contains(&Step::Added {
        alias: "anna".to_owned(),
        kinds: vec!["human-client".to_owned()],
    }));
    assert!(device.records().is_empty(), "the record is deleted");
    assert!(device.key_file(&endpoint), "its key is kept");
    let card = device
        .card(&endpoint)
        .expect("the card reads")
        .expect("a card");
    assert_eq!(
        (card.name.as_str(), card.surname.as_deref()),
        ("Anna", Some("Rossi"))
    );
    assert_eq!(roster.front(), &human(), "the front does not change");
    let more = roster.take_held_more();
    assert_eq!(more.len(), 1);
    assert_eq!(more[0].0, endpoint);

    let before = device.log.steps().len();
    for now in 6..9 {
        roster.turn(now).await;
    }
    let steps = device.log.steps()[before..].to_vec();
    let asserted = steps
        .iter()
        .position(|s| s == &Step::Advertised(endpoint.clone(), false))
        .expect("asserted unadvertised");
    let opened = steps
        .iter()
        .position(|s| s == &Step::Open(endpoint.clone()))
        .expect("leased");
    assert!(asserted < opened, "{steps:?}");
    assert_eq!(roster.front(), &human());
}

/// While a record names the id -- its add not answered -- nothing is
/// written under it and nothing leases it: no card, no session, not held,
/// however many turns run. A Create while one runs does not start and is
/// told so. Mutations: the card written as the Create starts -- a card
/// stands under the id; a Create that does not start telling nothing --
/// no `NotStarted`.
#[tokio::test]
async fn while_a_record_names_the_id_nothing_is_written_under_it() {
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(Reply::Holds));
    let mut roster = device.roster(memory(), Vec::new());
    started(&mut roster).await;
    offered(&mut roster, 5);
    roster.add(create("Anna"), 5);
    assert!(roster.take_add_told().is_empty(), "the add is waited for");
    roster.add(create("Anna"), 5);
    assert_eq!(roster.take_add_told(), [AddTold::NotStarted], "one runs");
    let records = device.records();
    assert_eq!(records.len(), 1, "the record names the form's key");
    let id = records[0].clone();
    for now in 6..12 {
        roster.turn(now).await;
    }
    assert_eq!(device.card(&id).expect("the card reads"), None);
    assert!(!device.log.steps().contains(&Step::Open(id.clone())));
    assert!(!roster.held().any(|e| e == &id));
    assert!(roster.take_held_more().is_empty());
}

/// A card that cannot be written still ends Created: the identity is
/// held, its record deleted, and the card is marked not saved. The store
/// refuses every write to the own cards through a trigger laid by a
/// second connection. Mutation: a failed card write ending the Create as
/// failed -- no Created is told.
#[tokio::test]
async fn a_card_that_cannot_be_written_still_ends_created_with_the_card_not_saved() {
    let mut device = Device::new();
    let dir = common::private_tempdir();
    let path = dir.path().join("human.sqlite3");
    let store = HumanStore::open(&path, &human(), StoreOptions::default()).expect("a store");
    rusqlite::Connection::open(&path)
        .expect("a second connection")
        .execute_batch(
            "CREATE TRIGGER no_card BEFORE INSERT ON own_cards
                 BEGIN SELECT RAISE(ABORT, 'refused'); END;
             CREATE TRIGGER no_card_change BEFORE UPDATE ON own_cards
                 BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        )
        .expect("the triggers");
    let mut roster = device.roster(store, Vec::new());
    started(&mut roster).await;
    offered(&mut roster, 5);
    roster.add(create("Anna"), 5);
    let told = roster.take_add_told();
    let (endpoint, _, card) = created(&told);
    assert_eq!(card, Some(CardSaved::NotSaved));
    assert!(told.contains(&AddTold::Held(endpoint.clone())), "{told:?}");
    assert_eq!(device.card(&endpoint).expect("the card reads"), None);
    assert!(device.records().is_empty());
}

/// A Create the daemon does not answer holds no other identity's turn:
/// while its add is waited for, the first identity's session takes a
/// message and commits it unread. Run on a thread of its own and waited
/// for with `recv_timeout`, so a Create that blocks the turn fails here
/// instead of hanging the suite. Mutation: the roster polling the form
/// until it ends -- the turn never returns.
#[test]
fn a_create_never_holds_another_identitys_turn() {
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        runtime.block_on(async {
            let mut device = Device::new();
            device.script(|s| s.adds.push_back(Reply::Holds));
            let store = memory();
            let rows = store.handle_speaking_as(human());
            let mut roster = device.roster(store, Vec::new());
            started(&mut roster).await;
            offered(&mut roster, 5);
            roster.add(create("Anna"), 5);
            let from = device
                .peer
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
            let id = format!("{:032x}", 7);
            from.send_direct(
                DirectDestination {
                    peer: device.node.peer().clone(),
                    endpoint: Some(human()),
                },
                MessageId::from_bytes([7; 16]),
                Payload::at_ceiling(
                    Some(
                        MediaType::parse("application/vnd.interweave-human-chat+json;v=2")
                            .expect("media"),
                    ),
                    format!(r#"{{"v":2,"kind":"text","app_message_id":"{id}","text":"hi"}}"#)
                        .into_bytes(),
                )
                .expect("bounds"),
            )
            .await
            .expect("delivered");
            let mut unread = 0;
            for now in 6..40 {
                let routed = roster.turn(now).await;
                assert!(
                    routed
                        .iter()
                        .all(|(_, u)| u.iter().all(|u| !matches!(u, Update::Failed { .. }))),
                    "{routed:?}"
                );
                unread = rows
                    .unread_counts()
                    .expect("counts")
                    .iter()
                    .map(|c| c.unread)
                    .sum::<u64>();
                if unread > 0 {
                    break;
                }
            }
            assert_eq!(unread, 1, "the first identity drained its message");
            assert!(
                roster.take_add_told().is_empty(),
                "the add still waited for"
            );
        });
        let _ = done.send(());
    });
    finished
        .recv_timeout(Duration::from_secs(60))
        .expect("the turns ran while the add was waited for");
}

/// The settle runs at the first turn, at the list's opening and at a
/// write refused at the bound, and at no other turn: a record left by an
/// earlier run, unlisted, is discarded at the first turn; one planted
/// later stays through idle turns and is discarded when the list opens;
/// with the record full, a Create settles the others and writes once
/// more. Mutations: the first turn's settle not owed -- the earlier run's
/// record stays; the list's opening not a moment -- the planted one
/// stays.
#[tokio::test]
async fn the_settle_runs_at_the_first_turn_at_the_lists_opening_and_at_the_bound() {
    let mut device = Device::new();
    let mut store = memory();
    let earlier = device.left_by_a_form(&mut store);
    let mut rows = store.handle_speaking_as(human());
    let mut roster = device.roster(store, Vec::new());
    roster.turn(0).await;
    assert!(
        device.log.steps().contains(&Step::Removed(earlier.clone())),
        "{:?}",
        device.log.steps()
    );
    assert!(device.records().is_empty());
    assert!(!device.key_file(&earlier), "its key path emptied");

    let later = device.left_by_a_form(&mut rows);
    for now in 1..6 {
        roster.turn(now).await;
    }
    assert_eq!(
        device.records(),
        std::slice::from_ref(&later),
        "no moment yet"
    );
    roster.add(AddAct::ListOpened, 6);
    roster.turn(7).await;
    assert!(device.log.steps().contains(&Step::Removed(later.clone())));
    assert!(device.records().is_empty());

    let full: Vec<EndpointId> = (0..PENDING_ADD_CAP)
        .map(|_| device.left_by_a_form(&mut rows))
        .collect();
    offered(&mut roster, 8);
    roster.add(create("Anna"), 8);
    let (endpoint, _, _) = created(&roster.take_add_told());
    for id in &full {
        assert!(device.log.steps().contains(&Step::Removed(id.clone())));
    }
    assert!(device.records().is_empty(), "settled and made");
    assert!(roster.held().any(|e| e == &endpoint));
}

/// An identity a settle at the first turn finds -- a record whose id is
/// listed with its key file there, which the profile's list names
/// besides the first -- is not held at construction and is held at the
/// settle, with no restart: told held, its record deleted, handed to the
/// root, then asserted and leased. Mutation: a `Settled::Identity` not
/// held -- it is not told.
#[tokio::test]
async fn an_identity_the_start_settle_finds_is_held_without_a_restart() {
    let mut device = Device::new();
    let mut store = memory();
    let found = device.left_by_a_form(&mut store);
    device.script(|s| s.listed = vec![view(&found, "found")]);
    let mut roster = device.roster(store, vec![found.clone()]);
    assert!(!roster.held().any(|e| e == &found), "a record names it");
    roster.turn(0).await;
    assert_eq!(roster.take_add_told(), [AddTold::Held(found.clone())]);
    assert!(device.records().is_empty());
    assert!(device.key_file(&found));
    assert_eq!(roster.take_held_more().len(), 1);
    for now in 1..4 {
        roster.turn(now).await;
    }
    let steps = device.log.steps();
    assert!(steps.contains(&Step::Advertised(found.clone(), false)));
    assert!(steps.contains(&Step::Open(found.clone())), "{steps:?}");
    assert_eq!(roster.front(), &human());
}

/// A form's closing settles its own record, with one identity held:
/// the add ended unknown, so the form keeps its key and record while it
/// is open -- a settle at the list's opening leaves that record alone --
/// and its leaving is a settle moment that covers it: listed nowhere, it
/// is discarded, key path first and the remove after. Mutations: the
/// leaving owing no settle -- the record stays; the list's settle taking
/// the open form's record -- it is removed while the form is open.
#[tokio::test]
async fn a_forms_closing_settles_its_own_record_with_one_identity_held() {
    let mut device = Device::new();
    device.script(|s| {
        s.adds.push_back(Reply::Now(Err(TransportError::Internal)));
    });
    let mut roster = device.roster(memory(), Vec::new());
    started(&mut roster).await;
    assert_eq!(roster.held().len(), 1);
    offered(&mut roster, 5);
    roster.add(create("Anna"), 5);
    let told = roster.take_add_told();
    assert!(
        told.contains(&AddTold::Ended {
            outcome: AddOutcome::Unknown,
            card: None
        }),
        "{told:?}"
    );
    let records = device.records();
    assert_eq!(records.len(), 1);
    let own = records[0].clone();
    assert!(device.key_file(&own), "kept while the form is open");

    roster.add(AddAct::ListOpened, 6);
    roster.turn(7).await;
    assert!(!device.log.steps().contains(&Step::Removed(own.clone())));
    assert_eq!(
        device.records(),
        std::slice::from_ref(&own),
        "the open form's"
    );

    roster.add(AddAct::Left, 8);
    roster.turn(9).await;
    assert!(device.records().is_empty(), "settled at the closing");
    assert!(!device.key_file(&own), "discarded");
    assert!(device.log.steps().contains(&Step::Removed(own.clone())));
    assert_eq!(roster.held().len(), 1, "nothing held");
}

/// An identity whose hold is refused after its record is deleted is told
/// [`AddTold::NotHeld`] and held at a later poll, with no restart. The
/// record is put behind a view by a second connection, so that every
/// read of it fails once a delete ran, until the test clears that: the
/// Create's delete then fails the hold's read of the record. Mutations:
/// the refusal told nothing -- no `NotHeld`; the hold not tried again --
/// never held.
#[tokio::test]
async fn an_identity_whose_hold_is_refused_is_told_and_held_at_a_later_poll() {
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(Reply::Now(Err(TransportError::Internal))));
    let dir = common::private_tempdir();
    let path = dir.path().join("human.sqlite3");
    let store = HumanStore::open(&path, &human(), StoreOptions::default()).expect("a store");
    let mut roster = device.roster(store, Vec::new());
    started(&mut roster).await;
    offered(&mut roster, 5);
    roster.add(create("Anna"), 5);
    let told = roster.take_add_told();
    assert!(
        told.contains(&AddTold::Ended {
            outcome: AddOutcome::Unknown,
            card: None
        }),
        "{told:?}"
    );
    let records = device.records();
    assert_eq!(records.len(), 1);
    let id = records[0].clone();

    // abs() of i64::MIN is an error, so the view fails every read while
    // `fail_reads` holds a row, rows or none (the second SELECT runs once
    // and yields nothing); the record's delete writes one.
    let second = rusqlite::Connection::open(&path).expect("a second connection");
    second
        .execute_batch(
            "CREATE TABLE fail_reads (n INTEGER);
             ALTER TABLE pending_adds RENAME TO pending_adds_rows;
             CREATE VIEW pending_adds AS
                 SELECT rowid AS rowid, endpoint_id FROM pending_adds_rows
                 UNION ALL SELECT 0, NULL
                 WHERE abs(-9223372036854775807 - (SELECT COUNT(*) FROM fail_reads)) < 0;
             CREATE TRIGGER pending_adds_delete INSTEAD OF DELETE ON pending_adds
                 BEGIN
                     DELETE FROM pending_adds_rows WHERE endpoint_id = OLD.endpoint_id;
                     INSERT INTO fail_reads VALUES (1);
                 END;",
        )
        .expect("the view");
    assert_eq!(
        device.records(),
        std::slice::from_ref(&id),
        "reads until a delete"
    );

    device.script(|s| s.listed = vec![view(&id, "anna")]);
    roster.add(create("Anna"), 6);
    let told = roster.take_add_told();
    let (endpoint, _, _) = created(&told);
    assert_eq!(endpoint, id);
    assert!(told.contains(&AddTold::NotHeld(id.clone())), "{told:?}");
    assert!(!told.contains(&AddTold::Held(id.clone())), "{told:?}");
    assert!(!roster.held().any(|e| e == &id), "not held yet");
    roster.turn(7).await;
    assert!(
        roster.take_add_told().is_empty(),
        "still refused, told once"
    );
    assert!(!roster.held().any(|e| e == &id));

    second
        .execute("DELETE FROM fail_reads", [])
        .expect("reads again");
    roster.turn(8).await;
    assert_eq!(roster.take_add_told(), [AddTold::Held(id.clone())]);
    assert!(roster.held().any(|e| e == &id), "held with no restart");
    assert_eq!(roster.take_held_more().len(), 1);
}

/// Done closes the form (identities §6.2; contact cards §5.2, "A way of
/// leaving"): a Create that ends Created leaves no form open, so a
/// Create then is told not started; the next opening makes a new form,
/// whose Create makes a second key and tells a second outcome; and the
/// closing is a settle moment, so a record planted after Done is settled
/// at the next turn. Mutations: the form not cleared at Created -- the
/// second Create is told not started and nothing is made; the closing
/// owing no settle -- the planted record stays.
#[tokio::test]
async fn done_closes_the_form_and_the_next_opens_a_new_one() {
    let mut device = Device::new();
    let store = memory();
    let mut rows = store.handle_speaking_as(human());
    let mut roster = device.roster(store, Vec::new());
    started(&mut roster).await;
    offered(&mut roster, 5);
    roster.add(create("Anna"), 5);
    let (first, _, _) = created(&roster.take_add_told());

    let planted = device.left_by_a_form(&mut rows);
    roster.turn(6).await;
    assert!(
        device.log.steps().contains(&Step::Removed(planted.clone())),
        "settled at Done's closing: {:?}",
        device.log.steps()
    );
    assert!(device.records().is_empty());

    roster.add(create("Bruno"), 7);
    assert_eq!(
        roster.take_add_told(),
        [AddTold::NotStarted],
        "no form open"
    );

    roster.add(AddAct::Opened, 8);
    assert_eq!(roster.take_add_told(), [AddTold::Offered]);
    roster.add(create("Bruno"), 8);
    let told = roster.take_add_told();
    let (second, alias, _) = created(&told);
    assert_ne!(second, first, "a key of its own");
    assert_eq!(alias.as_deref(), Some("bruno"));
    assert!(told.contains(&AddTold::Held(second.clone())), "{told:?}");
    assert!(device.key_file(&first) && device.key_file(&second));
    assert_eq!(device.log.count(|s| matches!(s, Step::Added { .. })), 2);
    assert_eq!(roster.held().len(), 3);
}

/// A PNG of `w` × `h`, as a person's file might hold one.
fn png(w: u32, h: u32) -> Vec<u8> {
    let image = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([
            u8::try_from(x % 256).unwrap_or(0),
            u8::try_from(y % 256).unwrap_or(0),
            90,
        ])
    });
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .expect("a PNG");
    out
}

/// Turns until the form's photo is told, every update they routed kept.
async fn photo_told(roster: &mut Roster<Data, Admins>, from: u64) -> (Vec<AddTold>, Vec<Update>) {
    let mut routed = Vec::new();
    for now in from..from + 500 {
        for (_, updates) in roster.turn(now).await {
            routed.extend(updates);
        }
        let told = roster.take_add_told();
        if told
            .iter()
            .any(|t| matches!(t, AddTold::PhotoPrepared { .. }))
        {
            return (told, routed);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the photo was never told");
}

/// The add form's photo is the roster's to prepare, for no identity
/// (identities §6.2; cards §5.2's photo rules): told by its number with
/// the re-encoded bytes and their decoded square, while no update reaches
/// any held identity -- no own photo prepared -- and the first identity's
/// own card is as it was. A Create then carries those bytes, and they are
/// the new identity's card's photo. Mutations: the photo written to the
/// first identity's own card; the photo told to the front as its own
/// photo's update.
#[tokio::test]
async fn a_forms_photo_is_prepared_and_touches_no_identitys_card() {
    let mut device = Device::new();
    let mut roster = device.roster(memory(), Vec::new());
    started(&mut roster).await;
    let before = device.card(&human()).expect("the card reads");
    roster.add(
        AddAct::PreparePhoto {
            chosen: png(40, 30),
            seq: 7,
        },
        5,
    );
    let (told, routed) = photo_told(&mut roster, 6).await;
    let photo = match told.as_slice() {
        [
            AddTold::PhotoPrepared {
                seq: 7,
                photo: Some(photo),
            },
        ] => photo.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!((photo.shown.width, photo.shown.height), (30, 30), "square");
    assert_eq!(&photo.bytes[..2], &[0xFF, 0xD8], "re-encoded as JPEG");
    assert!(
        !routed
            .iter()
            .any(|u| matches!(u, Update::OwnPhotoPrepared { .. })),
        "{routed:?}"
    );
    assert_eq!(device.card(&human()).expect("the card reads"), before);

    offered(&mut roster, 2_000);
    roster.add(
        AddAct::Create {
            name: "Anna".to_owned(),
            surname: None,
            photo: Some(photo.bytes.clone()),
        },
        2_000,
    );
    let (endpoint, _, card) = created(&roster.take_add_told());
    assert_eq!(card, Some(CardSaved::Saved));
    let card = device
        .card(&endpoint)
        .expect("the card reads")
        .expect("a card");
    assert_eq!(card.photo, Some(photo.bytes));
    assert_eq!(device.card(&human()).expect("the card reads"), before);
}

/// A file that is no image a card can carry is told so -- the form shows
/// `PhotoUnusable` -- by its own number, and nothing else is told.
/// Mutation: an unusable file told as prepared.
#[tokio::test]
async fn a_photo_that_cannot_be_used_is_told_so() {
    let mut device = Device::new();
    let mut roster = device.roster(memory(), Vec::new());
    started(&mut roster).await;
    roster.add(
        AddAct::PreparePhoto {
            chosen: b"not an image".to_vec(),
            seq: 3,
        },
        5,
    );
    let (told, _) = photo_told(&mut roster, 6).await;
    assert_eq!(
        told,
        [AddTold::PhotoPrepared {
            seq: 3,
            photo: None
        }]
    );
}
