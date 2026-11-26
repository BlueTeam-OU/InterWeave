// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identities the Service holds (plan section 30 step 3; the note of
//! 2026-11-26 (ii)): the first, then every entry the endpoint overlay
//! added for the human client, each with its own key -- with the network
//! and without it -- and a start refused over an overlay that does not
//! read.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use interweave_human_android_platform::{
    Held, ServiceHost, ServiceLaunch, StartRefused, ToView, ViewLink,
};
use interweave_human_store::key_custody::{FilesystemCustody, KeyCipher};
use interweave_human_ui_model::HeldIdentity;
use interweave_profile_config::endpoint_overlay::{Added, EndpointOverlay};
use interweave_profile_config::{
    ClientKind, ProfileConfig, ProfilePaths, TrustBoundary, endpoint_key,
};
use interweave_transport_api::EndpointId;
use interweave_transport_embedded::EmbeddedRefused;

mod common;

const PROFILE: &str = "human";
const WAIT: Duration = Duration::from_secs(20);
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

/// The two ways the Service starts: with the network, over the embedded
/// runtime, and without it.
#[derive(Debug, Clone, Copy)]
enum Path {
    Network,
    Offline,
}

const BOTH: [Path; 2] = [Path::Network, Path::Offline];

fn start(app: &App, service: &ServiceHost, path: Path) -> Result<(), StartRefused> {
    match path {
        Path::Network => service.start(ServiceLaunch {
            app_data_dir: app.dir.clone(),
            profile: PROFILE.to_owned(),
            identity: interweave_profile_identity::ProfileIdentity::generate(),
            store_cipher: Box::new(FilesystemCustody),
            unlock: None,
        }),
        Path::Offline => {
            let cipher: Arc<dyn KeyCipher + Send + Sync> = Arc::new(FilesystemCustody);
            service.start_without_network(&app.dir, PROFILE, cipher)
        }
    }
}

/// The first identity provisioned, then an overlay adding `work`, a
/// human-client entry, and `bot`, a claude-channel one, each with a key
/// of its own: the first's id and the two added.
fn two_added(app: &App) -> (EndpointId, EndpointId, EndpointId) {
    let first = common::provision_first_identity(&app.dir, PROFILE);
    let paths = paths(app);
    let config = ProfileConfig::load(&paths).expect("the profile");
    let mut overlay = EndpointOverlay::default();
    let mut add = |alias: &str, kind: &str| {
        let minted = endpoint_key::mint_filesystem(&paths).expect("minted");
        match overlay.add(
            &config.endpoints,
            Some(&paths),
            &endpoint_key::key_file_name(&minted),
            alias,
            vec![ClientKind::parse(kind).expect("a kind")],
        ) {
            Added::Added {
                overlay: added,
                endpoint,
            } => {
                overlay = added;
                endpoint
            }
            other => panic!("{other:?}"),
        }
    };
    let work = add("work", "human-client");
    let bot = add("bot", "claude-channel");
    overlay
        .write_within(&EndpointOverlay::path_for(&paths), paths.boundary())
        .expect("written");
    (first, work, bot)
}

fn attach(service: &ServiceHost) -> ViewLink {
    service.hub().attach(Arc::new(|| {}))
}

/// What a view attached now is told of the identities: the held list and
/// the rows, waiting for the rows, which the facade tells from its own
/// thread.
fn told(service: &ServiceHost) -> (Held, Vec<HeldIdentity>) {
    let mut link = attach(service);
    let deadline = Instant::now() + WAIT;
    let mut held = None;
    loop {
        for message in link.take() {
            match message {
                ToView::Held(h) => held = Some(h),
                ToView::Identities(rows) => {
                    return (held.expect("the held list before the rows"), rows);
                }
                _ => {}
            }
        }
        assert!(Instant::now() < deadline, "no identities within {WAIT:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn row<'a>(rows: &'a [HeldIdentity], id: &EndpointId) -> &'a HeldIdentity {
    rows.iter()
        .find(|r| r.key.as_str() == id.as_str())
        .expect("a row for the identity")
}

/// With an overlay adding a human-client entry and a claude-channel one,
/// the Service holds the first identity and the human-client one, in that
/// order, and no other -- with the network and without it -- and the
/// added identity's row is named by its alias, not its short key.
/// Mutations: the roster's `others` left empty -- one held; the aliases
/// left empty -- `work` is named by its short key; the added taken before
/// the first -- the order fails.
#[test]
fn the_first_and_the_added_human_entry_are_held() {
    for path in BOTH {
        let app = app();
        let (first, work, bot) = two_added(&app);
        let service = ServiceHost::new();
        start(&app, &service, path).unwrap_or_else(|e| panic!("{path:?}: {e:?}"));
        let (held, rows) = told(&service);
        assert_eq!(held.held, [first.clone(), work.clone()], "{path:?}");
        assert!(!held.held.contains(&bot), "{path:?}: not the bridge's");
        assert_eq!(rows.len(), 2, "{path:?}");
        assert!(row(&rows, &first).is_first, "{path:?}");
        let added = row(&rows, &work);
        assert_eq!(added.alias, "work", "{path:?}");
        assert_eq!(
            added.name(),
            "work",
            "{path:?}: its alias, not its short key"
        );
        let _ = service.stop(GRACE);
    }
}

/// With no overlay the first identity is held alone.
#[test]
fn an_absent_overlay_holds_the_first_alone() {
    for path in BOTH {
        let app = app();
        let first = common::provision_first_identity(&app.dir, PROFILE);
        let service = ServiceHost::new();
        start(&app, &service, path).unwrap_or_else(|e| panic!("{path:?}: {e:?}"));
        let (held, rows) = told(&service);
        assert_eq!(held.held, [first], "{path:?}");
        assert_eq!(rows.len(), 1, "{path:?}");
        let _ = service.stop(GRACE);
    }
}

/// A present overlay that does not read refuses the start as a profile
/// that does not read, never read as one with no added entry; nothing is
/// left running. With the network the runtime's own start reads the
/// overlay first and refuses it the same way, so the Service's read is
/// reached only without it: the mutation runs there. Mutation: the
/// Service's read failing into "the first alone" -- the start without
/// network succeeds.
#[test]
fn an_overlay_that_does_not_read_refuses_the_start_without_network() {
    for path in BOTH {
        let app = app();
        common::provision_first_identity(&app.dir, PROFILE);
        let paths = paths(&app);
        let file = EndpointOverlay::path_for(&paths);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("mkdir");
        std::fs::write(&file, b"{ not json").expect("written");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("chmod");
        let service = ServiceHost::new();
        match (path, start(&app, &service, path)) {
            // The runtime's own read, which names a parse internal.
            (Path::Network, Err(StartRefused::Runtime(EmbeddedRefused::Internal(_))))
            | (Path::Offline, Err(StartRefused::Runtime(EmbeddedRefused::ProfileInvalid(_)))) => {}
            (_, other) => panic!("{path:?}: {other:?}"),
        }
        assert!(!service.is_running(), "{path:?}");
    }
}

/// Each held identity opens its own key: the added identity's damaged
/// key is its own problem, said by kind among the others', and the
/// first's, which opens, is not reported. Mutations: one source, the
/// first's, for every identity -- nothing is said of the other; the
/// other's problem kept as the first's -- `identity_key_refused` says it.
#[test]
fn each_held_identity_opens_its_own_key() {
    let app = app();
    let (_, work, _) = two_added(&app);
    let paths = paths(&app);
    std::fs::write(endpoint_key::key_path(&paths, &work), b"not a key").expect("damaged");
    let service = ServiceHost::new();
    start(&app, &service, Path::Offline).expect("started");
    let _ = told(&service);
    assert_eq!(service.identity_key_refused(), None, "the first's opens");
    assert_eq!(service.other_identity_keys_refused(), ["damaged"]);
    let _ = service.stop(GRACE);
}

/// Every held identity is asserted unadvertised before its lease (ADR-0061
/// decision 5; the note of 2026-11-26): an added identity the overlay
/// marks advertised is unadvertised in the file by the time its session
/// is Ready, through the runtime's own administrative port. The assertion
/// is the transport client's, made by every client the roster builds;
/// this pins that the Service's roster gives the added identity one.
/// Mutation: the roster's `others` left empty -- the added identity
/// takes no lease, and its session never reads Ready.
#[test]
fn an_added_identity_is_asserted_unadvertised_before_its_lease() {
    use interweave_human_app_core::Update;
    use interweave_human_client_api::{ClientEvent, SessionState};

    let app = app();
    let paths = paths(&app);
    let (_, work, _) = two_added(&app);
    let config = ProfileConfig::load(&paths).expect("the profile");
    let file = EndpointOverlay::path_for(&paths);
    let (overlay, _) =
        EndpointOverlay::read_within(&file, &config.endpoints, paths.boundary()).expect("reads");
    overlay
        .set_advertise(&config.endpoints, &work, true)
        .expect("held")
        .expect("changed")
        .write_within(&file, paths.boundary())
        .expect("written");
    let advertise = |endpoint: &EndpointId| {
        let (overlay, _) = EndpointOverlay::read_within(&file, &config.endpoints, paths.boundary())
            .expect("reads");
        overlay
            .added()
            .find(|e| &e.id == endpoint)
            .expect("added")
            .advertise
    };
    assert!(advertise(&work), "the control: advertised before the start");

    let service = ServiceHost::new();
    start(&app, &service, Path::Network).expect("started");
    let mut link = attach(&service);
    let deadline = Instant::now() + WAIT;
    'ready: loop {
        for message in link.take() {
            if let ToView::Update(identity, update) = message
                && identity == work
                && matches!(
                    *update,
                    Update::Client(ClientEvent::Session(SessionState::Ready { .. }))
                )
            {
                break 'ready;
            }
        }
        assert!(Instant::now() < deadline, "the added identity never Ready");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!advertise(&work), "asserted unadvertised before the lease");
    let _ = service.stop(GRACE);
}
