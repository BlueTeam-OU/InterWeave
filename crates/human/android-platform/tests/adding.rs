// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Creating an identity through the embedded host (plan section 30,
//! Stage 22 step 3; ADR-0061 decision 7; `design/2026-11-04-identities.md`
//! section 6.2): the Service's roster holds an identity made while it
//! runs with no restart, and the view hears it before anything of it.
//!
//! Over the real embedded host -- its real administrative port, real
//! `endpoint-keys/` and the real store -- in this process, and the real
//! facade thread; the view is a hub link read directly. What this does
//! NOT prove: a screen (Android offers no Add control yet), a device, or
//! the Keystore: every key here is TEST-ONLY, sealed under the
//! filesystem custody.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use interweave_human_android_platform::{
    Held, ServiceHost, ServiceLaunch, StartRefused, ToView, ViewLink,
};
use interweave_human_app_core::{AddAct, AddTold, Update};
use interweave_human_store::identity_key;
use interweave_human_store::key_custody::{FilesystemCustody, KeyCipher};
use interweave_human_transport_client::AddOutcome;
use interweave_human_ui_model::HeldIdentity;
use interweave_profile_config::{ProfilePaths, TrustBoundary};
use interweave_transport_api::EndpointId;

mod common;

const PROFILE: &str = "human";
const WAIT: Duration = Duration::from_secs(40);
const GRACE: Duration = Duration::from_secs(1);

/// An app data directory as the platform hands it over: private, under a
/// parent that is not.
struct App {
    _root: tempfile::TempDir,
    dir: PathBuf,
}

fn app() -> App {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let open = root.path().join("open");
    let dir = open.join("app");
    for (path, mode) in [(&open, 0o777), (&dir, 0o700)] {
        std::fs::create_dir(path).expect("mkdir");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
    }
    App { _root: root, dir }
}

fn paths(app: &App) -> ProfilePaths {
    ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("a boundary"))
        .expect("paths")
}

fn start(app: &App, service: &ServiceHost) -> Result<(), StartRefused> {
    service.start(ServiceLaunch {
        app_data_dir: app.dir.clone(),
        profile: PROFILE.to_owned(),
        identity: interweave_profile_identity::ProfileIdentity::generate(),
        store_cipher: Box::new(FilesystemCustody),
        unlock: None,
    })
}

fn start_offline(app: &App, service: &ServiceHost) -> Result<(), StartRefused> {
    let cipher: Arc<dyn KeyCipher + Send + Sync> = Arc::new(FilesystemCustody);
    service.start_without_network(&app.dir, PROFILE, cipher)
}

/// What a view was sent, in the order it was sent, reduced to what these
/// tests hold.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Said {
    Held(Vec<EndpointId>),
    HeldMore(Vec<EndpointId>),
    /// An update for the identity: whether it is `Update::Started`.
    Update(EndpointId, bool),
    /// The held rows' identities.
    Rows(Vec<EndpointId>),
    Add(AddTold),
}

/// A view attached to the hub, and what it has been sent.
struct View {
    link: ViewLink,
    said: Vec<Said>,
    rows: Vec<HeldIdentity>,
    held: Option<Held>,
}

impl View {
    fn attach(service: &ServiceHost) -> Self {
        Self {
            link: service.hub().attach(Arc::new(|| {})),
            said: Vec::new(),
            rows: Vec::new(),
            held: None,
        }
    }

    fn take(&mut self) {
        for message in self.link.take() {
            match message {
                ToView::Held(held) => {
                    self.said.push(Said::Held(held.held.clone()));
                    self.held = Some(held);
                }
                ToView::HeldMore(more) => self.said.push(Said::HeldMore(more)),
                ToView::Update(identity, update) => self.said.push(Said::Update(
                    identity,
                    matches!(*update, Update::Started(_)),
                )),
                ToView::Identities(rows) => {
                    self.said.push(Said::Rows(
                        rows.iter()
                            .map(|r| EndpointId::parse_derived(r.key.as_str()).expect("an id"))
                            .collect(),
                    ));
                    self.rows = rows;
                }
                ToView::Add(told) => self.said.push(Said::Add(told)),
                _ => {}
            }
        }
    }

    /// Wait for `done`, taking what is sent meanwhile.
    fn until(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + WAIT;
        loop {
            self.take();
            if done(self) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "{what} within {WAIT:?}: {:?}",
                self.said
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn told(&self, told: &AddTold) -> bool {
        self.said.contains(&Said::Add(told.clone()))
    }

    fn at(&self, said: &Said) -> Option<usize> {
        self.said.iter().position(|s| s == said)
    }

    /// The identity a Create ended Created with.
    fn created(&self) -> Option<EndpointId> {
        self.said.iter().find_map(|s| match s {
            Said::Add(AddTold::Ended {
                outcome: AddOutcome::Created { endpoint, .. },
                ..
            }) => Some(endpoint.clone()),
            _ => None,
        })
    }
}

/// A started network service over a provisioned profile, a view attached
/// and told the first identity, and the add form opened and Create
/// offered.
fn offered(app: &App) -> (ServiceHost, View, EndpointId) {
    let first = common::provision_first_identity(&app.dir, PROFILE);
    let service = ServiceHost::new();
    start(app, &service).expect("the service starts");
    let mut view = View::attach(&service);
    view.until("the first identity's rows", |v| {
        v.said.iter().any(|s| matches!(s, Said::Rows(_)))
    });
    assert_eq!(service.hub().add(AddAct::Opened), Ok(()));
    view.until("Create offered", |v| v.told(&AddTold::Offered));
    (service, view, first)
}

fn create(service: &ServiceHost, name: &str) {
    assert_eq!(
        service.hub().add(AddAct::Create {
            name: name.to_owned(),
            surname: None,
            photo: None,
        }),
        Ok(())
    );
}

/// A Create through the real embedded host makes the identity, seals its
/// key under `endpoint-keys/`, holds it with no restart and leaves the
/// front as it was; the view hears it held before it hears any update
/// for it and before a held list naming it, then its start, then the
/// held list. Mutations (run, each failing it): the network roster built
/// without `.adding(factory)` -- the form is never answered and the wait
/// times out; `HeldMore` told after the identity's start update -- the
/// order assert fails; the loop never taking the identities held -- no
/// `HeldMore` and the wait times out. The turn's own updates published
/// before the held identities are told is held by the facade's unit test
/// (`a_created_identity_is_named_before_any_update_for_it`), since the integration run did not fail on it.
#[test]
fn a_create_through_the_embedded_host_holds_the_identity_before_any_update_for_it() {
    let app = app();
    let (service, mut view, first) = offered(&app);
    create(&service, "Work");
    view.until("the identity held and its rows told", |v| {
        v.created().is_some_and(|made| {
            v.said
                .iter()
                .any(|s| matches!(s, Said::Rows(rows) if rows.contains(&made)))
        })
    });
    let made = view.created().expect("created");
    assert_ne!(made, first);

    let more = view
        .at(&Said::HeldMore(vec![made.clone()]))
        .expect("told it held");
    let first_for_it = view
        .said
        .iter()
        .position(|s| matches!(s, Said::Update(id, _) if *id == made))
        .expect("an update for it");
    let rows_naming_it = view
        .said
        .iter()
        .position(|s| matches!(s, Said::Rows(rows) if rows.contains(&made)))
        .expect("rows naming it");
    assert!(more < first_for_it, "{:?}", view.said);
    assert!(more < rows_naming_it, "{:?}", view.said);
    assert_eq!(
        view.at(&Said::Update(made.clone(), true)),
        Some(first_for_it),
        "its start is its first update: {:?}",
        view.said
    );
    assert!(
        view.told(&AddTold::Held(made.clone())),
        "told held: {:?}",
        view.said
    );

    let rows = view.rows.clone();
    assert_eq!(rows.len(), 2, "two identities held now");
    assert!(rows[0].is_first && rows[0].key.as_str() == first.as_str());
    assert!(!rows[1].is_first && rows[1].key.as_str() == made.as_str());
    assert_eq!(
        view.held.as_ref().map(|h| h.front.clone()),
        Some(first),
        "the front unchanged"
    );
    assert!(
        identity_key::key_path(&paths(&app), &made).exists(),
        "its key file is under endpoint-keys"
    );
    let _ = service.stop(GRACE);
}

/// A view that attaches after the Create is told a held list that
/// includes the new identity, in front of nothing new, and the rows
/// naming it. Mutation: the hub's `held_more` not growing the list it
/// keeps -- the late view's list names the first identity alone.
#[test]
fn a_view_attached_after_a_create_is_told_the_new_identity_held() {
    let app = app();
    let (service, mut view, first) = offered(&app);
    create(&service, "Work");
    view.until("the identity held", |v| {
        v.created().is_some_and(|made| {
            v.said
                .iter()
                .any(|s| matches!(s, Said::Rows(rows) if rows.contains(&made)))
        })
    });
    let made = view.created().expect("created");

    let mut late = View::attach(&service);
    late.until("the late view's rows", |v| {
        v.said.iter().any(|s| matches!(s, Said::Rows(_)))
    });
    assert_eq!(
        late.said.first(),
        Some(&Said::Held(vec![first.clone(), made.clone()])),
        "{:?}",
        late.said
    );
    assert_eq!(
        late.held.as_ref().map(|h| h.front.clone()),
        Some(first),
        "the front is the first"
    );
    assert_eq!(late.rows.len(), 2);
    let _ = service.stop(GRACE);
}

/// A pending-add record naming a key sealed in `endpoint-keys/` and
/// listed by the daemon -- as an earlier run's form whose add ended
/// unknown left them: the service has run once, nothing holds the store,
/// and the record is written under the identity the store opens as.
fn left_by_an_earlier_run(app: &App) -> (EndpointId, EndpointId) {
    use interweave_human_store::{HumanStore, StoreOptions};
    use interweave_profile_config::endpoint_overlay::{Added, EndpointOverlay};
    use interweave_profile_config::{ClientKind, ProfileConfig, endpoint_key};
    let first = common::provision_first_identity(&app.dir, PROFILE);
    let paths = paths(app);
    let service = ServiceHost::new();
    start_offline(app, &service).expect("the posture starts");
    service.stop(GRACE).expect("it ran");
    let key = identity_key::create(&paths, &FilesystemCustody).expect("a key");
    let found = key.endpoint().clone();
    let mut store =
        HumanStore::open_profile(&paths, &FilesystemCustody, &first, StoreOptions::default())
            .expect("the store");
    store.add_pending_add(&found).expect("a record");
    drop(store);
    let config = ProfileConfig::load(&paths).expect("the profile");
    match EndpointOverlay::default().add(
        &config.endpoints,
        Some(&paths),
        &endpoint_key::key_file_name(&found),
        "found",
        vec![ClientKind::parse("human-client").expect("a kind")],
    ) {
        Added::Added { overlay, .. } => overlay
            .write_within(&EndpointOverlay::path_for(&paths), paths.boundary())
            .expect("written"),
        other => panic!("{other:?}"),
    }
    (first, found)
}

/// What the store of the profile names as pending adds, read once the
/// service has stopped and nothing holds the store.
fn pending_adds(app: &App, first: &EndpointId) -> Vec<EndpointId> {
    use interweave_human_store::{HumanStore, StoreOptions};
    HumanStore::open_profile(
        &paths(app),
        &FilesystemCustody,
        first,
        StoreOptions::default(),
    )
    .expect("the store")
    .pending_adds()
    .expect("the record")
}

/// A record an earlier run left, whose id the daemon lists with its key
/// file at its key path, is settled at the service's start with no
/// restart: the id is held after the start, named to the view before any
/// update for it, and the record is gone and the key kept. Through the
/// real embedded host and the real facade thread. Mutations (run, each
/// failing it): the roster built without `.adding(factory)` -- nothing is
/// settled; `HeldMore` told after the identity's start update -- the
/// order assert fails.
#[test]
fn a_record_left_by_an_earlier_run_is_settled_at_start_through_the_real_thread() {
    let app = app();
    let (first, found) = left_by_an_earlier_run(&app);
    assert_eq!(
        pending_adds(&app, &first),
        std::slice::from_ref(&found),
        "the control"
    );
    let service = ServiceHost::new();
    let mut view = View::attach(&service);
    start(&app, &service).expect("the service starts");
    view.until("the found identity held and its rows told", |v| {
        v.told(&AddTold::Held(found.clone()))
            && v.said
                .iter()
                .any(|s| matches!(s, Said::Rows(rows) if rows.contains(&found)))
    });
    assert_eq!(
        view.said.first(),
        Some(&Said::Held(vec![first.clone()])),
        "the start named the first alone: a record names no identity yet"
    );
    let more = view
        .at(&Said::HeldMore(vec![found.clone()]))
        .expect("told it held");
    let first_for_it = view
        .said
        .iter()
        .position(|s| matches!(s, Said::Update(id, _) if *id == found))
        .expect("an update for it");
    assert!(more < first_for_it, "{:?}", view.said);
    assert_eq!(
        view.at(&Said::Update(found.clone(), true)),
        Some(first_for_it),
        "its start is its first update: {:?}",
        view.said
    );
    let held = view
        .at(&Said::Add(AddTold::Held(found.clone())))
        .expect("told");
    let rows = view
        .said
        .iter()
        .position(|s| matches!(s, Said::Rows(rows) if rows.contains(&found)))
        .expect("rows");
    assert!(more < held && more < rows, "{:?}", view.said);
    assert_eq!(
        view.held.as_ref().map(|h| h.front.clone()),
        Some(first.clone()),
        "the front unchanged"
    );
    let _ = service.stop(GRACE);
    assert!(
        pending_adds(&app, &first).is_empty(),
        "the record is deleted"
    );
    assert!(
        identity_key::key_path(&paths(&app), &found).exists(),
        "the key is kept"
    );
}

/// Over the network-denied posture the offline binding's administrative
/// port answers `BackendUnavailable`, so no add or settle goes any
/// further than its first call: a record an earlier run left is neither
/// settled nor discarded at the start or at the list's opening, the key
/// stays beside it for the next start that has the network and nothing
/// is held or told; the form's seed ask ends unanswered, `NoSeed`, so
/// Create is not offered, and a Create made nothing, wrote no key and
/// holds no identity. Mutation (run, failing it): the offline roster
/// built without `.adding(factory)` -- the form is never answered and the
/// wait for `NoSeed` times out.
#[test]
fn over_the_offline_posture_a_settle_and_a_create_stop_at_their_first_call() {
    let app = app();
    let (first, found) = left_by_an_earlier_run(&app);
    let service = ServiceHost::new();
    let mut view = View::attach(&service);
    start_offline(&app, &service).expect("the posture starts");
    view.until("the first identity's rows", |v| {
        v.said.iter().any(|s| matches!(s, Said::Rows(_)))
    });

    assert_eq!(service.hub().add(AddAct::Opened), Ok(()));
    view.until("the seed ask ended", |v| v.told(&AddTold::NoSeed));
    assert!(view.told(&AddTold::WaitsSeed), "it waited first");
    assert!(!view.told(&AddTold::Offered), "Create is not offered");
    let told_before = view
        .said
        .iter()
        .filter(|s| matches!(s, Said::Add(_)))
        .count();
    create(&service, "Work");
    assert_eq!(service.hub().add(AddAct::ListOpened), Ok(()));
    view.until("the Create answered", |v| {
        v.said.iter().filter(|s| matches!(s, Said::Add(_))).count() > told_before
    });
    // The list's settle has had its turns by then: the Create's answer
    // went through the same loop after it.
    std::thread::sleep(Duration::from_millis(500));
    view.take();
    let tails: Vec<_> = view
        .said
        .iter()
        .filter_map(|s| match s {
            Said::Add(told) => Some(told.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        tails,
        [AddTold::WaitsSeed, AddTold::NoSeed, AddTold::NoSeed],
        "nothing made, ended or held"
    );
    assert!(
        !view.said.iter().any(|s| matches!(s, Said::HeldMore(_))),
        "{:?}",
        view.said
    );
    assert_eq!(
        view.held.as_ref().map(|h| h.held.clone()),
        Some(vec![first.clone()])
    );
    let _ = service.stop(GRACE);
    assert_eq!(
        pending_adds(&app, &first),
        std::slice::from_ref(&found),
        "the record is kept for the next start"
    );
    assert!(identity_key::key_path(&paths(&app), &found).exists());
    let mut keys = identity_key::sealed(&paths(&app)).expect("the keys");
    keys.sort();
    let mut expected = vec![first, found];
    expected.sort();
    assert_eq!(keys, expected, "the first's and the found, none made");
}
