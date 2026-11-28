// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade thread carries the roster's add acts, headless
//! (`design/2026-11-04-identities.md` §6.2; ADR-0061 decision 7): an add
//! act crosses to the roster and what came of it comes back; an identity
//! held while the client runs -- here found by the settle of a record an
//! earlier run left -- reaches the window before anything of it does, so
//! it gets a model of its own and is told the held list; and what the
//! window logs of an add is a class, never an id, an alias or a name.
//! And the window carries the acts (plan §30 PR 3 commit 14): the form
//! and the list, pressed by their accessible names in the headless view,
//! reach the roster, and what it tells shows on them.
//! Over the fake network with a real store, a real `endpoint-keys/`
//! under a private directory and the real facade thread; the add, the
//! seed and the remove answered by a port scripted beside the fake's
//! own, which has no add.
//!
//! What this does NOT prove: the daemon's add
//! (`crates/transport/composition`'s; desktop-e2e `adding.rs` drives it),
//! a real window, a screen reader.
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
use interweave_human_ui_model::{IdentityKey, RuntimeHost, UiText, placeholder_en};
use interweave_human_ui_slint::{CardsPage, View};
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
use slint::Model as _;

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

/// What the scripted admin answers and what it was asked.
#[derive(Default)]
struct Script {
    /// The next adds' answers, in order; past them, `added`.
    adds: std::collections::VecDeque<Result<EndpointAddChange, TransportError>>,
    /// The alias of each add sent, in order.
    aliases: Vec<String>,
    /// The first identity's seed is not answered while set.
    seed_fails: bool,
    /// Each endpoint a remove was asked for: answered `EndpointUnknown`,
    /// the fake never having held it.
    removes: Vec<EndpointId>,
}

/// The fake's admin connections, with the add, the seed and the remove
/// scripted -- the fake answers an add and a remove unsupported -- and
/// every other call the fake's.
#[derive(Clone)]
struct Admins(FakeNode, Arc<Mutex<Script>>);

impl Admins {
    fn of(node: FakeNode) -> Self {
        Self(node, Arc::new(Mutex::new(Script::default())))
    }

    fn script(&self) -> std::sync::MutexGuard<'_, Script> {
        self.1.lock().expect("the script")
    }
}

struct Port(<FakeNode as AdminBinding>::Admin, Arc<Mutex<Script>>);

impl AdminBinding for Admins {
    type Admin = Port;

    async fn admin(&self, capabilities: BTreeSet<AdminCapability>) -> Result<Port, TransportError> {
        Ok(Port(self.0.admin(capabilities).await?, Arc::clone(&self.1)))
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
        alias: String,
        _allowed_client_kinds: Vec<String>,
    ) -> impl Future<Output = Result<EndpointAddChange, TransportError>> + Send {
        let id = interweave_profile_config::endpoint_key::parse_key_file_name(&key)
            .expect("a key file's name");
        let mut script = self.1.lock().expect("the script");
        script.aliases.push(alias);
        let answer = script
            .adds
            .pop_front()
            .unwrap_or(Ok(EndpointAddChange::Added(id)));
        std::future::ready(answer)
    }
    fn remove_endpoint(
        &self,
        endpoint: EndpointId,
    ) -> impl Future<
        Output = Result<interweave_local_client_api::EndpointRemoveChange, TransportError>,
    > + Send {
        self.1.lock().expect("the script").removes.push(endpoint);
        std::future::ready(Err(TransportError::EndpointUnknown))
    }
    async fn set_endpoint_advertise(
        &self,
        endpoint: EndpointId,
        advertise: bool,
    ) -> Result<(), TransportError> {
        self.0.set_endpoint_advertise(endpoint, advertise).await
    }
    async fn seed_endpoint_trust(&self, endpoint: EndpointId) -> Result<TrustSeed, TransportError> {
        let fails = self.1.lock().expect("the script").seed_fails;
        if fails {
            return Err(TransportError::BackendUnavailable);
        }
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
    thread_local! {
        static PLATFORM: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    // Once per thread: a case that opens a window more than once.
    if !PLATFORM.replace(true) {
        i_slint_backend_testing::init_no_event_loop();
    }
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
        roster(
            a.clone(),
            Admins::of(a.clone()),
            store,
            vec![],
            paths.clone(),
        ),
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

// The window carries the acts (plan §30 PR 3 commit 14): the person's
// presses on the headless view, by accessible name, through the model
// side and the real facade thread to the roster, and back.

/// One identity held, its store a file (so a test can make its writes
/// fail by trigger), over the scripted admin.
struct Desk {
    app: App<SlintSurface, NoLinks>,
    admins: Admins,
    paths: ProfilePaths,
    rows: HumanStore,
    _peer: FakeNode,
    _dir: tempfile::TempDir,
}

static DESK_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

impl Desk {
    /// The window open, `prepare` run on the admin script and on the
    /// store's file before the roster starts.
    fn open(prepare: impl FnOnce(&Admins, &rusqlite::Connection)) -> Self {
        let dir = common::private_tempdir();
        let paths = ProfilePaths::resolve_embedded(
            "default",
            TrustBoundary::new(dir.path()).expect("a boundary"),
        )
        .expect("paths");
        let store_path = dir.path().join("human.sqlite3");
        let store =
            HumanStore::open(&store_path, &human(), StoreOptions::default()).expect("store");
        let rows = store.handle_speaking_as(human());
        let (a, b) = FakeNetwork::pair(node(&[human()]), node(&[human()]));
        let admins = Admins::of(a.clone());
        prepare(
            &admins,
            &rusqlite::Connection::open(&store_path).expect("a second connection"),
        );
        let thread = FacadeThread::spawn(
            roster(a.clone(), admins.clone(), store, vec![], paths.clone()),
            || Ok(true),
            Arc::new(|| {}),
            |_| {},
        )
        .expect("the facade thread");
        let app = app(thread, |line| {
            DESK_LOG.lock().expect("the log").push(line.to_owned());
        });
        let mut desk = Self {
            app,
            admins,
            paths,
            rows,
            _peer: b,
            _dir: dir,
        };
        desk.until("the first identity leased", |d| {
            d.app.side().front().is_some()
        });
        desk
    }

    fn window(&self) -> &interweave_human_ui_slint::AppWindow {
        self.app.side().surface().0.window()
    }

    /// One turn of the window, with the timers its focus requests wait on.
    fn turn(&mut self) {
        self.app.pump();
        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(5));
        slint::platform::update_timers_and_animations();
    }

    fn until(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        let wait = common::Wait::on(what);
        while !done(self) {
            wait.check_showing(|| format!("{:?}", logged(&DESK_LOG)));
            self.turn();
            std::thread::sleep(Duration::from_millis(10));
        }
        self.turn();
    }

    /// The control named `label` pressed by its default action.
    fn press(&mut self, label: &str) {
        let found: Vec<_> =
            i_slint_backend_testing::ElementHandle::find_by_accessible_label(self.window(), label)
                .collect();
        assert!(!found.is_empty(), "a control named {label:?}");
        found[0].invoke_accessible_default_action();
        self.turn();
    }

    fn shows(&self, label: &str) -> bool {
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(self.window(), label)
            .next()
            .is_some()
    }

    /// `typed` keyed into the focused control.
    fn type_text(&mut self, typed: &str) {
        for c in typed.chars() {
            let text: slint::SharedString = c.to_string().into();
            let window = slint::ComponentHandle::window(self.window());
            window.dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
            window.dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
        }
        self.turn();
    }

    fn focus(&self) -> String {
        self.window().get_focus_id().to_string()
    }

    /// The form opened from Your card's overflow (one identity held),
    /// its seed answered unless the script holds it back, the name typed
    /// into its Name field, which has focus (F5).
    fn form_with(&mut self, name: &str) {
        let options = self.window().get_app_options_label().to_string();
        self.press(&options);
        self.press(text(UiText::OwnCardTitle));
        self.press(text(UiText::IdentityOptions));
        self.press(text(UiText::AddAnotherIdentity));
        self.until("the form, its Name focused", |d| {
            d.window().get_cards_page() == CardsPage::AddIdentity && d.focus() == "add-name"
        });
        self.type_text(name);
    }

    fn create_offered(&mut self) {
        self.until("Create offered", |d| {
            let form = d.window().get_add_form();
            form.create_shown && !form.create_checking && !form.create_running
        });
    }

    fn records(&self) -> Vec<EndpointId> {
        self.rows.pending_adds().expect("the records read")
    }

    fn keys(&self) -> Vec<EndpointId> {
        identity_key::sealed(&self.paths).expect("the keys")
    }

    fn line_above(&self) -> String {
        self.window().get_add_form().line_above.to_string()
    }
}

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

/// Acceptance 7, through the window: Your card's overflow, Add another
/// identity, a name typed and Create; the form closes on Your
/// identities, the new row focused (F6), `ConfirmIdentityAdded` shown,
/// the identity in front unchanged, the record gone and the key kept.
/// Mutations: `App::pump` not handing the model side's add acts to the
/// facade -- nothing is created; not giving `FromFacade::Add` to the
/// model side -- the form never ends.
#[test]
fn create_through_the_window_ends_on_the_new_row_with_the_front_unchanged() {
    let mut desk = Desk::open(|_, _| {});
    desk.form_with("Zelinda");
    desk.create_offered();
    desk.press(text(UiText::CreateIdentity));
    desk.until("Your identities on the new row", |d| {
        d.window().get_cards_page() == CardsPage::Identities && d.focus() == "identities-row:1"
    });
    assert_eq!(desk.app.side().front(), Some(&human()), "front unchanged");
    let made = desk.keys();
    assert_eq!(made.len(), 1, "one key made");
    assert!(desk.records().is_empty(), "the record deleted");
    assert_eq!(desk.admins.script().aliases, ["zelinda"]);
    let snack = desk.window().get_id_snack_text().to_string();
    assert!(
        snack.starts_with("\u{2068}Zelinda\u{2069} was created"),
        "{snack:?}"
    );
    assert!(desk.app.close(Duration::from_secs(5)));
}

/// Run (t)'s tail: an add whose answer is lost ends
/// `IdentityAddUnknown`; Back asks first (the Name holds text) and Close
/// without saving leaves with the key and the record kept, so the
/// closing's settle finds them: the list lacks the id, the remove is
/// sent, and only then are the key and the record gone. Mutation: the
/// form's leaving deleting the key whatever the outcome -- no remove is
/// ever sent.
#[test]
fn an_unknown_outcome_then_back_keeps_the_key_and_the_record_and_the_closing_settles() {
    let mut desk = Desk::open(|admins, _| {
        admins
            .script()
            .adds
            .push_back(Err(TransportError::BackendUnavailable));
    });
    desk.form_with("Zelinda");
    desk.create_offered();
    desk.press(text(UiText::CreateIdentity));
    desk.until("the outcome unknown", |d| {
        d.line_above().starts_with("InterWeave could not check")
    });
    assert_eq!(desk.focus(), "add-line-above", "F7");
    let made = desk.keys();
    assert_eq!(made.len(), 1);
    assert_eq!(desk.records(), made, "the record kept");
    desk.press(text(UiText::BackToCard));
    assert!(
        desk.window().get_own_discard_open(),
        "asks: the Name holds text"
    );
    desk.press(text(UiText::DiscardChanges));
    desk.until("the closing's settle", |d| {
        d.keys().is_empty() && d.records().is_empty()
    });
    assert_eq!(desk.admins.script().removes, made, "removed by the settle");
    assert_eq!(
        desk.window().get_cards_page(),
        CardsPage::OwnCard,
        "back on the card"
    );
    assert!(desk.app.close(Duration::from_secs(5)));
}

/// Run (k), 11 and 11a (a) to (c), (f), through the window: the record
/// cannot be written -- `IdentityAddFailed`, no key sealed; its delete
/// fails -- `IdentityAddNotFinished`, the record kept; the card cannot be
/// written -- Done, the row named by its alias with
/// `IdentityCardNotSaved` and no `ConfirmIdentityAdded`. Mutation: an
/// ending's told not given to the model side -- the form never shows it.
#[test]
fn the_records_write_its_delete_and_the_cards_write_each_made_to_fail() {
    let trigger = |sql: &'static str| {
        move |_: &Admins, conn: &rusqlite::Connection| {
            conn.execute_batch(sql).expect("the trigger");
        }
    };

    let mut desk = Desk::open(trigger(
        "CREATE TRIGGER no_record BEFORE INSERT ON pending_adds
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    ));
    desk.form_with("Zelinda");
    desk.create_offered();
    desk.press(text(UiText::CreateIdentity));
    desk.until("refused, nothing created", |d| {
        d.line_above() == text(UiText::IdentityAddFailed)
    });
    assert!(desk.keys().is_empty(), "nothing sealed");
    assert!(desk.admins.script().aliases.is_empty(), "no add sent");
    assert!(desk.app.close(Duration::from_secs(5)));

    let mut desk = Desk::open(trigger(
        "CREATE TRIGGER no_delete BEFORE DELETE ON pending_adds
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    ));
    desk.form_with("Zelinda");
    desk.create_offered();
    desk.press(text(UiText::CreateIdentity));
    desk.until("created, not finished", |d| {
        d.line_above().contains("could not finish setting it up")
    });
    assert_eq!(desk.records().len(), 1, "the record stands");
    assert_eq!(desk.focus(), "add-line-above", "F7");
    assert!(desk.app.close(Duration::from_secs(5)));

    let mut desk = Desk::open(trigger(
        "CREATE TRIGGER no_card BEFORE INSERT ON own_cards
             BEGIN SELECT RAISE(ABORT, 'refused'); END;
         CREATE TRIGGER no_card_change BEFORE UPDATE ON own_cards
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    ));
    desk.form_with("Zelinda");
    desk.create_offered();
    desk.press(text(UiText::CreateIdentity));
    desk.until("Done, the card not saved", |d| {
        d.window().get_cards_page() == CardsPage::Identities && d.focus() == "identities-row:1"
    });
    let row = desk
        .window()
        .get_id_list()
        .rows
        .row_data(1)
        .expect("the new row");
    assert_eq!(row.name, "zelinda", "named by its alias");
    assert_eq!(row.card_not_saved, text(UiText::IdentityCardNotSaved));
    assert!(
        !desk.window().get_id_snack_shown(),
        "no ConfirmIdentityAdded"
    );
    assert!(desk.app.close(Duration::from_secs(5)));
}

/// Run (u), its first three variants, through the window: with the seed
/// not answered, Create is read as checking, then `IdentityAddWaitsSeed`
/// and Try again stand in its place; Try again with the seed still not
/// answered asks again and leaves focus on it; the seed answered, Try
/// again brings Create back with focus on it, and Create ends Done.
/// Mutation: the form's acts not handed to the facade -- no seed is
/// asked again, and Create never comes.
#[test]
fn a_run_with_no_seed_shows_the_line_then_try_again_then_create() {
    let mut desk = Desk::open(|admins, _| admins.script().seed_fails = true);
    desk.form_with("Zelinda");
    desk.until("the seed not answered", |d| {
        d.shows(text(UiText::IdentityAddWaitsSeed))
    });
    assert!(!desk.window().get_add_form().create_shown);
    desk.press(text(UiText::TryAgain));
    desk.until("asked again, not answered", |d| {
        let form = d.window().get_add_form();
        form.try_enabled && !form.try_progress
    });
    assert!(desk.shows(text(UiText::IdentityAddWaitsSeed)));
    desk.admins.script().seed_fails = false;
    desk.press(text(UiText::TryAgain));
    desk.create_offered();
    assert!(!desk.shows(text(UiText::IdentityAddWaitsSeed)));
    desk.press(text(UiText::CreateIdentity));
    desk.until("Done", |d| {
        d.window().get_cards_page() == CardsPage::Identities
    });
    assert!(desk.app.close(Duration::from_secs(5)));
}

/// Run (i) and (v), through the window: a first Create refused
/// `alias_taken` shows `IdentityAddFailed`; a second sends the next free
/// alias; refused again, Back and Close without saving delete the key
/// and the record, since nothing was created. Mutation: the form's acts
/// taken after the turn -- the dialog never shows on Back.
#[test]
fn a_first_create_refused_alias_taken_then_back_deletes_the_key_and_the_record() {
    let taken = || {
        Ok(EndpointAddChange::Refused {
            reason: interweave_local_client_api::EndpointAddRefusal::AliasTaken,
            detail: String::new(),
        })
    };
    let mut desk = Desk::open(|admins, _| {
        let mut script = admins.script();
        script.adds.push_back(taken());
        script.adds.push_back(taken());
    });
    desk.form_with("Zelinda");
    desk.create_offered();
    desk.press(text(UiText::CreateIdentity));
    desk.until("refused", |d| {
        d.line_above() == text(UiText::IdentityAddFailed)
    });
    desk.press(text(UiText::CreateIdentity));
    desk.until("refused again", |d| d.admins.script().aliases.len() == 2);
    desk.until("the line again", |d| {
        d.line_above() == text(UiText::IdentityAddFailed)
            && !d.window().get_add_form().create_running
    });
    assert_eq!(desk.admins.script().aliases, ["zelinda", "zelinda-2"]);
    assert_eq!(desk.keys().len(), 1, "one key for the form");
    desk.press(text(UiText::BackToCard));
    assert!(desk.window().get_own_discard_open());
    desk.press(text(UiText::DiscardChanges));
    desk.until("the key and the record deleted", |d| {
        d.keys().is_empty() && d.records().is_empty()
    });
    assert!(desk.admins.script().removes.is_empty(), "nothing to remove");
    assert!(desk.app.close(Duration::from_secs(5)));
}
