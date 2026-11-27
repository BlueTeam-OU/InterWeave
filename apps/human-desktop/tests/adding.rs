// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade thread carries the roster's add acts, headless
//! (`design/2026-11-04-identities.md` §6.2; ADR-0061 decision 7): an add
//! act crosses to the roster and what came of it comes back; an identity
//! held while the client runs -- here found by the settle of a record an
//! earlier run left -- reaches the window before anything of it does, so
//! it gets a model of its own and is told the held list; and what the
//! window logs of an add is a class, never an id, an alias or a name.
//! Over the fake network with a real store, a real `endpoint-keys/`
//! under a private directory and the real facade thread; the add answered
//! by a port scripted beside the fake's own, which has no add.
//!
//! What this does NOT prove: a screen (none sends an add act yet), the
//! daemon's add (`crates/transport/composition`'s), a real window.
//!
//! Every key here is TEST-ONLY (`interweave_test_support::endpoint_keys`,
//! or generated into a temporary directory).

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use interweave_human_app_core::{
    AddAct, AddFactory, AddTold, Opener, Roster, RosterConfig, Update,
};
use interweave_human_desktop::app::{App, SlintSurface};
use interweave_human_desktop::facade_thread::{FacadeFault, FacadeThread, FromFacade};
use interweave_human_store::identity_key::{self, FilesystemCustody};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_ui_model::{IdentityKey, RuntimeHost};
use interweave_human_ui_slint::View;
use interweave_local_client_api::{
    AdminBinding, AdminCapability, AdminPort, AdminStatus, EndpointAddChange, EndpointAdminView,
    EndpointTrustView, Generation, LocalAdminPort, TrustAdminView, TrustChange, TrustDirection,
    TrustSeed,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_config::TrustBoundary;
use interweave_profile_config::paths::ProfilePaths;
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::{EndpointId, TransportError, TransportIdentity};

fn human() -> EndpointId {
    endpoint_id("human")
}

fn node(endpoints: &[EndpointId]) -> FakeConfig {
    FakeConfig {
        peer: ProfileIdentity::generate()
            .transport_identity()
            .expect("peer"),
        endpoints: endpoints
            .iter()
            .map(|e| FakeEndpoint::open(e.clone(), false))
            .collect(),
        default_endpoint: endpoints.first().cloned(),
        queue_bound: 16,
    }
}

struct NoLinks;

impl Opener for NoLinks {
    fn open(&mut self, _destination: &str) {}
    fn ask_network_access(&mut self) {}
    fn authenticate(&mut self) {}
}

/// The fake's admin connections, with an add answered `added` -- the
/// fake answers it unsupported -- and every other call the fake's.
#[derive(Clone)]
struct Admins(FakeNode);

struct Port(<FakeNode as AdminBinding>::Admin);

impl AdminBinding for Admins {
    type Admin = Port;

    async fn admin(&self, capabilities: BTreeSet<AdminCapability>) -> Result<Port, TransportError> {
        Ok(Port(self.0.admin(capabilities).await?))
    }
}

impl AdminPort for Port {
    fn port(&self) -> &LocalAdminPort {
        self.0.port()
    }
    async fn status(&self) -> Result<AdminStatus, TransportError> {
        self.0.status().await
    }
    async fn leases(&self) -> Result<Vec<EndpointAdminView>, TransportError> {
        self.0.leases().await
    }
    fn add_endpoint(
        &self,
        key: String,
        _alias: String,
        _allowed_client_kinds: Vec<String>,
    ) -> impl Future<Output = Result<EndpointAddChange, TransportError>> + Send {
        let id = interweave_profile_config::endpoint_key::parse_key_file_name(&key)
            .expect("a key file's name");
        std::future::ready(Ok(EndpointAddChange::Added(id)))
    }
    async fn set_endpoint_advertise(
        &self,
        endpoint: EndpointId,
        advertise: bool,
    ) -> Result<(), TransportError> {
        self.0.set_endpoint_advertise(endpoint, advertise).await
    }
    async fn seed_endpoint_trust(&self, endpoint: EndpointId) -> Result<TrustSeed, TransportError> {
        self.0.seed_endpoint_trust(endpoint).await
    }
    async fn endpoint_trust(
        &self,
        endpoint: EndpointId,
        direction: TrustDirection,
    ) -> Result<EndpointTrustView, TransportError> {
        self.0.endpoint_trust(endpoint, direction).await
    }
    async fn revoke_endpoint(&self, endpoint: EndpointId) -> Result<(), TransportError> {
        self.0.revoke_endpoint(endpoint).await
    }
    async fn set_endpoint_enabled(
        &self,
        endpoint: EndpointId,
        enabled: bool,
    ) -> Result<Option<Generation>, TransportError> {
        self.0.set_endpoint_enabled(endpoint, enabled).await
    }
    async fn set_default_endpoint(
        &self,
        endpoint: Option<EndpointId>,
    ) -> Result<(), TransportError> {
        self.0.set_default_endpoint(endpoint).await
    }
    async fn trust(&self) -> Result<TrustAdminView, TransportError> {
        self.0.trust().await
    }
    async fn set_trust(
        &self,
        peer: TransportIdentity,
        allowed: bool,
    ) -> Result<TrustChange, TransportError> {
        self.0.set_trust(peer, allowed).await
    }
    async fn shutdown(&self, grace: Duration) -> Result<(), TransportError> {
        self.0.shutdown(grace).await
    }
}

/// A private profile directory and its paths.
fn profile() -> (tempfile::TempDir, ProfilePaths) {
    let dir = common::private_tempdir();
    let paths = ProfilePaths::resolve_embedded(
        "default",
        TrustBoundary::new(dir.path()).expect("a boundary"),
    )
    .expect("paths");
    (dir, paths)
}

/// What the desktop's `ipc.rs` builds, over `data` for the sessions and
/// `admin` for the administrative calls: the first identity and
/// `others`, able to hold an identity made while it runs.
fn roster<A>(
    data: FakeNode,
    admin: A,
    store: HumanStore,
    others: Vec<EndpointId>,
    paths: ProfilePaths,
) -> impl FnOnce() -> Result<Roster<FakeNode, A>, FacadeFault> + Send + 'static
where
    A: AdminBinding<Admin: Send> + Clone + Send + Sync + 'static,
{
    move || {
        let mut endpoints = vec![human()];
        endpoints.extend(others.iter().cloned());
        let (d, a) = (data.clone(), admin.clone());
        Ok(Roster::new(
            RosterConfig {
                client_kind: "human-client".to_owned(),
                endpoints,
                channels: vec![],
                max_payload_bytes: 49_152,
                others,
                default: None,
                aliases: BTreeMap::new(),
            },
            store,
            move |_| (d.clone(), a.clone()),
            || 1_786_600_000_000,
            0,
        )
        .map_err(FacadeFault::from)?
        .adding(AddFactory {
            bindings: Box::new({
                let admin = admin.clone();
                move |_| (data.clone(), admin.clone())
            }),
            admin: Arc::new(admin),
            paths,
            cipher: Arc::new(FilesystemCustody),
        }))
    }
}

/// A key sealed in `endpoint-keys/` and a record naming it, as an earlier
/// run's form whose add ended unknown left them.
fn left_by_an_earlier_run(paths: &ProfilePaths, store: &mut HumanStore) -> EndpointId {
    let id = identity_key::create(paths, &FilesystemCustody)
        .expect("a key")
        .endpoint()
        .clone();
    store.add_pending_add(&id).expect("a record");
    id
}

/// The window over the real view, headless, fed by `thread`.
fn app(thread: FacadeThread, log: fn(&str)) -> App<SlintSurface, NoLinks> {
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Daemon).expect("a window");
    App::new(SlintSurface(view), NoLinks, thread).with_log(log)
}

fn pump_until(
    app: &mut App<SlintSurface, NoLinks>,
    what: &str,
    done: impl Fn(&App<SlintSurface, NoLinks>) -> bool,
) {
    let wait = common::Wait::on(what);
    while !done(app) {
        wait.check();
        app.pump();
        std::thread::sleep(Duration::from_millis(20));
    }
}

static SETTLE_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn logged(log: &Mutex<Vec<String>>) -> Vec<String> {
    log.lock().expect("the log").clone()
}

/// A record an earlier run left, whose id the daemon lists with its key
/// file at its key path, is settled at the first turn through the real
/// thread: the id is held with no restart. The thread sends the id held,
/// then its start, then the held list naming it, in that order, and the
/// handle's held identities name it before the first of these is sent;
/// through the window, it gets a model of its own, that model is told the
/// held list, its session is leased, the front is unchanged and the log
/// says an identity is held. The settle lands before the window's first
/// pump, so here the window names the identity from the handle; one held
/// after the window was named reaches it by `HeldMore` alone, which the
/// add's test below holds. Mutations: the thread not taking the
/// identities held after a turn -- no `HeldMore`; the thread forwarding
/// the turn's updates before it takes them -- the first identity's
/// connectivity, which the turn routes to the identity it held, comes
/// first and the order fails; the handle's held identities not grown --
/// they do not name it.
#[test]
fn a_record_left_by_an_earlier_run_is_settled_at_start_through_the_real_thread() {
    // The thread's own order, read off its queue.
    {
        let (_dir, paths) = profile();
        let mut store =
            HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store");
        let found = left_by_an_earlier_run(&paths, &mut store);
        let rows = store.handle_speaking_as(human());
        let (a, _b) = FakeNetwork::pair(node(&[human(), found.clone()]), node(&[human()]));
        let thread = FacadeThread::spawn(
            roster(
                a.clone(),
                a.clone(),
                store,
                vec![found.clone()],
                paths.clone(),
            ),
            || Ok(true),
            Arc::new(|| {}),
            |_| {},
        )
        .expect("the facade thread");
        let key = IdentityKey::new(found.as_str());
        let mut seen: Vec<&'static str> = Vec::new();
        let mut named_at_more = None;
        let wait = common::Wait::on("the found identity told");
        while !seen.contains(&"held list") {
            wait.check_showing(|| format!("{seen:?}"));
            for message in thread.take() {
                match message {
                    FromFacade::HeldMore(more) if more.contains(&found) => {
                        named_at_more = Some(thread.held().expect("named").held.contains(&found));
                        seen.push("held more");
                    }
                    FromFacade::Update(identity, update) if identity == found => {
                        seen.push(if matches!(*update, Update::Started(_)) {
                            "started"
                        } else {
                            "update"
                        });
                    }
                    FromFacade::Held(held) if held.iter().any(|h| h.key == key) => {
                        seen.push("held list");
                    }
                    FromFacade::Add(AddTold::Held(held)) if held == found => {
                        seen.push("told held");
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let at = |what: &str| seen.iter().position(|s| *s == what);
        assert!(at("held more").is_some(), "{seen:?}");
        assert!(at("held more") < at("started"), "{seen:?}");
        assert!(at("started") < at("held list"), "{seen:?}");
        assert!(
            at("told held").is_some_and(|i| i < at("held list").expect("seen")),
            "{seen:?}"
        );
        assert!(
            !seen[..at("started").expect("seen")].contains(&"update"),
            "{seen:?}"
        );
        assert_eq!(named_at_more, Some(true), "the handle names it first");
        assert!(
            rows.pending_adds().expect("read").is_empty(),
            "record deleted"
        );
        assert!(identity_key::key_path(&paths, &found).exists(), "key kept");
        assert!(thread.close(Duration::from_secs(5)));
    }

    // Through the window.
    let (_dir, paths) = profile();
    let mut store = HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store");
    let found = left_by_an_earlier_run(&paths, &mut store);
    let (a, _b) = FakeNetwork::pair(node(&[human(), found.clone()]), node(&[human()]));
    let thread = FacadeThread::spawn(
        roster(a.clone(), a.clone(), store, vec![found.clone()], paths),
        || Ok(true),
        Arc::new(|| {}),
        |_| {},
    )
    .expect("the facade thread");
    let mut app = app(thread, |line| {
        SETTLE_LOG.lock().expect("the log").push(line.to_owned());
    });
    pump_until(
        &mut app,
        "the found identity held, leased and told",
        |app| {
            a.open_sessions() == 2
                && app
                    .side()
                    .model_of(&found)
                    .is_some_and(|m| m.identities().held().len() == 2)
        },
    );
    assert_eq!(app.side().front(), Some(&human()), "the front unchanged");
    assert!(
        logged(&SETTLE_LOG).contains(&"add: an identity is held from now on".to_owned()),
        "{:?}",
        logged(&SETTLE_LOG)
    );
    assert!(app.close(Duration::from_secs(5)));
}

static ADD_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// TM-45's rule for the add (`tests/log_lines.rs`): an add form opened
/// and a Create carried through the real thread to an identity made and
/// held, every line the window writes on the way names a class and none
/// the new identity's id, its alias -- made from the name -- the name or
/// the surname, nor the first identity's id; and the identity, held
/// after the window was named, gets a model of its own. Mutations: the
/// add's line carrying `{told:?}` -- the id, alias and name reach the
/// log; `App::pump` ignoring `HeldMore` -- no model.
#[test]
fn the_add_logs_classes_and_never_an_id_an_alias_or_a_name() {
    let (_dir, paths) = profile();
    let store = HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store");
    let (a, _b) = FakeNetwork::pair(node(&[human()]), node(&[human()]));
    let thread = FacadeThread::spawn(
        roster(a.clone(), Admins(a.clone()), store, vec![], paths.clone()),
        || Ok(true),
        Arc::new(|| {}),
        |_| {},
    )
    .expect("the facade thread");
    let mut app = app(thread, |line| {
        ADD_LOG.lock().expect("the log").push(line.to_owned());
    });
    // By its start: what a line goes on to say is what this test holds.
    let has = |line: &str| logged(&ADD_LOG).iter().any(|l| l.starts_with(line));
    pump_until(&mut app, "the first identity leased", |app| {
        app.side().front().is_some() && a.open_sessions() == 1
    });

    assert!(app.add(AddAct::Opened));
    pump_until(&mut app, "Create offered", |_| has("add: create offered"));
    assert!(app.add(AddAct::Create {
        name: "Zelinda".to_owned(),
        surname: Some("Quarantotti".to_owned()),
        photo: None,
    }));
    pump_until(&mut app, "the identity made and held", |_| {
        has("add: an identity is held from now on") && has("add: created")
    });
    let made = identity_key::sealed(&paths).expect("the keys");
    assert_eq!(made.len(), 1, "one key made");
    let made = made[0].clone();
    pump_until(&mut app, "its model", |app| {
        app.side().model_of(&made).is_some()
    });
    assert!(app.close(Duration::from_secs(5)));

    let lines = logged(&ADD_LOG);
    let planted = [
        made.as_str().to_lowercase(),
        human().as_str().to_lowercase(),
        "zelinda".to_owned(),
        "quarantotti".to_owned(),
    ];
    for line in &lines {
        let lower = line.to_lowercase();
        for word in &planted {
            assert!(!lower.contains(word.as_str()), "{line:?} names {word:?}");
        }
    }
}

static IPC_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Over the desktop's own composition (`ipc.rs`) and the real IPC
/// binding, no daemon: an add form's opening sent through the window and
/// the real thread reaches the roster -- given its add factory -- and
/// what came of it comes back: Create waits for the first identity's
/// seed, which no daemon answers. Mutations: `ipc.rs` building the roster
/// without `adding` -- nothing is told; the thread dropping
/// `ToFacade::Add` -- nothing is told.
#[test]
fn an_add_act_sent_through_the_thread_reaches_the_roster_and_its_answer_comes_back() {
    use interweave_human_desktop::daemon;
    use interweave_human_desktop::ipc::facade_over_ipc;
    use interweave_human_desktop::startup::Profile;
    use interweave_profile_config::XdgRoots;

    let dir = common::private_tempdir();
    let roots = XdgRoots {
        config_home: dir.path().join("config"),
        data_home: dir.path().join("data"),
        state_home: dir.path().join("state"),
        cache_home: dir.path().join("cache"),
        runtime_dir: Some(dir.path().join("run")),
    };
    let paths = ProfilePaths::resolve("desk", &roots).expect("paths");
    let profile = Profile {
        endpoints: vec![human()],
        held: vec![interweave_human_app_core::HeldEntry {
            endpoint: human(),
            alias: None,
        }],
        default: None,
        channels: vec![],
        data_socket: paths.data_socket().expect("data socket"),
        admin_socket: paths.admin_socket().expect("admin socket"),
        paths: paths.clone(),
    };
    let store =
        HumanStore::open(&profile.store_path(), &human(), StoreOptions::default()).expect("store");
    let thread = FacadeThread::spawn(
        facade_over_ipc(&profile, store),
        move || daemon::present(&paths),
        Arc::new(|| {}),
        |_| {},
    )
    .expect("the facade thread");
    let mut app = app(thread, |line| {
        IPC_LOG.lock().expect("the log").push(line.to_owned());
    });
    pump_until(&mut app, "the identities named", |app| {
        app.side().front().is_some()
    });
    assert!(app.add(AddAct::Opened));
    pump_until(&mut app, "the form's answer", |_| {
        logged(&IPC_LOG)
            .iter()
            .any(|l| l == "add: create waits for the first identity's seed")
    });
    assert!(app.close(Duration::from_secs(5)));
}
