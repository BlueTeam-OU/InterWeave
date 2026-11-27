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
use interweave_human_store::key_custody::FilesystemCustody;
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
