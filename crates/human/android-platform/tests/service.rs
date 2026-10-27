// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Service's hold on the client, against the real embedded runtime on
//! the host: what the Service starts, what a view attached to it sees,
//! and what a stop leaves behind.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use interweave_human_android_platform::stand_in;
use interweave_human_android_platform::{
    AvailabilityMode, Ended, Link, NetworkReport, ServiceHost, ServiceLaunch, StartRefused,
    StayReachable, StoreAnswer, ToView, ViewLink,
};
use interweave_human_app_core::{Command, Update};
use interweave_human_client_api::{ClientEvent, SessionState};
use interweave_profile_config::{ProfileConfig, ProfilePaths, TrustBoundary};
use interweave_transport_embedded::EmbeddedRefused;

const PROFILE: &str = "human";
const WAIT: Duration = Duration::from_secs(20);
const GRACE: Duration = Duration::from_secs(1);

/// An app data directory as the platform hands it over: private, under a
/// parent that is not (Android's `/data/user/0` is the system's).
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

fn launch(app: &App) -> ServiceLaunch {
    ServiceLaunch {
        app_data_dir: app.dir.clone(),
        profile: PROFILE.to_owned(),
        identity: interweave_profile_identity::ProfileIdentity::generate(),
        store_cipher: Box::new(interweave_human_store::key_custody::FilesystemCustody),
    }
}

fn started(app: &App) -> ServiceHost {
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let service = ServiceHost::new();
    service.start(launch(app)).expect("started");
    service
}

fn attach(service: &ServiceHost) -> ViewLink {
    service.hub().attach(Arc::new(|| {}))
}

/// Take from `link` until `done` says so, or fail after [`WAIT`]. What
/// arrived after the message that satisfied it is kept in `held` for the
/// next call: a take hands over everything that waits, so the next thing
/// a test waits for may have come in the same take.
fn until(
    link: &mut ViewLink,
    held: &mut Vec<ToView>,
    what: &str,
    mut done: impl FnMut(&ToView) -> bool,
) {
    let deadline = Instant::now() + WAIT;
    loop {
        let mut waiting = std::mem::take(held);
        waiting.extend(link.take());
        if let Some(at) = waiting.iter().position(&mut done) {
            *held = waiting.split_off(at + 1);
            return;
        }
        assert!(Instant::now() < deadline, "no {what} within {WAIT:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn is_listing(message: &ToView) -> bool {
    matches!(message, ToView::Update(u) if matches!(**u, Update::Listed(_)))
}

fn is_ready(message: &ToView) -> bool {
    matches!(
        message,
        ToView::Update(u)
            if matches!(**u, Update::Client(ClientEvent::Session(SessionState::Ready { .. })))
    )
}

#[test]
fn a_started_service_gives_a_view_its_listing_then_a_ready_session() {
    let app = app();
    let service = started(&app);
    assert!(service.is_running());
    let mut link = attach(&service);
    let first = link.take();
    assert!(
        matches!(first.first(), Some(ToView::Running(true))),
        "the view hears the service runs first: {first:?}"
    );
    let mut listed = first.iter().any(is_listing);
    let mut ready = false;
    let deadline = Instant::now() + WAIT;
    while !(listed && ready) && Instant::now() < deadline {
        for message in link.take() {
            if is_listing(&message) {
                listed = true;
            } else if is_ready(&message) {
                assert!(listed, "an update never precedes the view's listing");
                ready = true;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(listed && ready, "listed {listed}, ready {ready}");
    let stopped = service.stop(GRACE).expect("it ran");
    assert!(
        stopped.session_closed,
        "the session closed before the runtime stopped"
    );
    assert!(stopped.runtime.is_ok(), "{:?}", stopped.runtime);
}

#[test]
fn a_start_while_the_client_runs_changes_nothing() {
    let app = app();
    let service = started(&app);
    let mut link = attach(&service);
    let mut held = Vec::new();
    until(&mut link, &mut held, "listing", is_listing);
    service
        .start(launch(&app))
        .expect("a second start succeeds");
    assert!(service.is_running());
    let mut again = held;
    again.extend(link.take());
    assert!(
        !again
            .iter()
            .any(|m| matches!(m, ToView::Running(_)) || is_listing(m)),
        "the facade was not restarted under the view: {again:?}"
    );
    let _ = service.stop(GRACE);
}

#[test]
fn a_stop_releases_the_waiter_tells_the_view_and_frees_the_profile() {
    let app = app();
    let service = Arc::new(started(&app));
    let mut link = attach(&service);
    let mut held = Vec::new();
    until(&mut link, &mut held, "listing", is_listing);

    let waiting = Arc::clone(&service);
    let waiter = std::thread::spawn(move || waiting.wait_ended());
    // The waiter is parked in the runtime before the stop asks.
    std::thread::sleep(Duration::from_millis(200));
    let stopped = service.stop(GRACE).expect("it ran");
    assert!(stopped.runtime.is_ok(), "{:?}", stopped.runtime);
    assert_eq!(
        waiter.join().expect("the waiter returned"),
        Ended::ShutdownRequested { grace: GRACE }
    );
    until(&mut link, &mut held, "the service stopping", |m| {
        matches!(m, ToView::Running(false))
    });
    assert!(!service.is_running());
    assert_eq!(service.wait_ended(), Ended::NotRunning);
    assert!(service.stop(GRACE).is_none(), "nothing left to stop");

    // The profile lock went with the runtime: the same profile starts.
    service.start(launch(&app)).expect("started again");
    until(&mut link, &mut held, "the service running again", |m| {
        matches!(m, ToView::Running(true))
    });
    until(&mut link, &mut held, "a fresh listing", is_listing);
    let _ = service.stop(GRACE);
}

/// Owner's finding P2-2, through the running service: every command the
/// hub accepted before a stop is carried out and answered before the view
/// hears the service stopped -- none is dropped unanswered -- and one sent
/// once it stopped is handed back. Whether any is still queued when the
/// close is seen is timing: with the close's drain removed this fails in
/// about two runs of five. The drain itself is held without timing by
/// facade.rs's `closing_hands_back_every_command_accepted_before_it`.
#[test]
fn a_command_accepted_before_a_stop_is_answered() {
    let app = app();
    let service = started(&app);
    let mut link = attach(&service);
    let mut held = Vec::new();
    until(&mut link, &mut held, "listing", is_listing);
    let rows: Vec<_> = (1_000..1_200)
        .map(interweave_human_ui_model::RowId::from_stored)
        .collect();
    for row in &rows {
        assert!(service.hub().command(Command::MarkRead(*row)).is_ok());
    }
    let _ = service.stop(GRACE).expect("it ran");
    let mut answered = std::collections::BTreeSet::new();
    let deadline = Instant::now() + WAIT;
    'heard: loop {
        let mut waiting = std::mem::take(&mut held);
        waiting.extend(link.take());
        for message in waiting {
            match message {
                ToView::Update(update) => match *update {
                    Update::Done(Command::MarkRead(row))
                    | Update::Failed {
                        command: Command::MarkRead(row),
                        ..
                    } => {
                        answered.insert(row);
                    }
                    _ => {}
                },
                ToView::Running(false) => break 'heard,
                _ => {}
            }
        }
        assert!(Instant::now() < deadline, "no stop within {WAIT:?}");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        answered.len(),
        rows.len(),
        "every accepted command answered before the stop"
    );
    assert_eq!(
        service.hub().command(Command::Reopen),
        Err(Command::Reopen),
        "handed back once stopped"
    );
}

/// The network callback's reports (plan section 20 step 5) are taken
/// while a start runs, without waiting for it: the callback runs on the
/// platform's thread, and a start holds the client for as long as the
/// runtime, the store and the facade take to come up.
#[test]
fn a_network_report_never_waits_on_a_start() {
    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let service = Arc::new(ServiceHost::new());
    let _ = service.network_report(NetworkReport::Watching);
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let begun = Arc::new(std::sync::Barrier::new(2));
    let starting = {
        let (service, done, launch) = (Arc::clone(&service), Arc::clone(&done), launch(&app));
        let begun = Arc::clone(&begun);
        std::thread::spawn(move || {
            begun.wait();
            service.start(launch).expect("started");
            done.store(true, std::sync::atomic::Ordering::SeqCst);
        })
    };
    // The start takes the client first thing and holds it for tens of
    // milliseconds at the least, and a report takes microseconds: counted
    // from a moment the start surely holds it, a report that waited on
    // the start lets none through.
    begun.wait();
    std::thread::sleep(Duration::from_millis(5));
    let mut during_the_start = 0_u32;
    let network = 1;
    while !done.load(std::sync::atomic::Ordering::SeqCst) {
        let _ = service.network_report(NetworkReport::Link {
            network,
            link: Link {
                addresses: vec![std::net::Ipv4Addr::new(10, 0, 0, 5).into()],
                ..Link::default()
            },
        });
        if !done.load(std::sync::atomic::Ordering::SeqCst) {
            during_the_start += 1;
        }
    }
    starting.join().expect("the start returned");
    assert!(
        during_the_start >= 100,
        "{during_the_start} reports overlapped the start: one waited on it"
    );
    // The slot's hold on the runtime does not keep it from its stop.
    let stopped = service.stop(GRACE).expect("it ran");
    assert!(stopped.runtime.is_ok(), "{:?}", stopped.runtime);
    let _ = service.network_report(NetworkReport::Lost { network });
}

/// The level the app's platform log is set to before a start is the
/// profile's own, as the start will read it.
#[test]
fn the_log_level_is_the_profiles_and_none_without_one() {
    let app = app();
    assert_eq!(
        interweave_human_android_platform::profile_log_level(&app.dir, PROFILE),
        None,
        "no profile yet"
    );
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let paths = ProfilePaths::resolve_embedded(
        PROFILE,
        TrustBoundary::new(&app.dir).expect("the boundary"),
    )
    .expect("the paths");
    let config = std::fs::read_to_string(paths.config_file()).expect("the config");
    assert_eq!(
        interweave_human_android_platform::profile_log_level(&app.dir, PROFILE),
        Some(interweave_human_android_platform::LogLevel::Info),
        "the stand-in names none: the default"
    );
    std::fs::write(
        paths.config_file(),
        format!("{config}\nobservability: {{ log_level: warn }}\n"),
    )
    .expect("the config written");
    assert_eq!(
        interweave_human_android_platform::profile_log_level(&app.dir, PROFILE),
        Some(interweave_human_android_platform::LogLevel::Warn)
    );
}

/// The level the platform log follows once a start ran is the one the
/// runtime loaded: a start redelivered while it runs starts nothing, so
/// a file changed since does not speak for it.
#[test]
fn the_running_log_level_is_the_one_the_runtime_loaded() {
    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let service = ServiceHost::new();
    assert_eq!(service.log_level(), None, "nothing runs");
    service.start(launch(&app)).expect("started");
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("paths");
    let config = std::fs::read_to_string(paths.config_file()).expect("read");
    std::fs::write(
        paths.config_file(),
        format!("{config}\nobservability: {{ log_level: warn }}\n"),
    )
    .expect("write");
    assert_eq!(
        interweave_human_android_platform::profile_log_level(&app.dir, PROFILE),
        Some(interweave_human_android_platform::LogLevel::Warn),
        "the control: the file now names warn"
    );
    service.start(launch(&app)).expect("redelivered");
    assert_eq!(
        service.log_level(),
        Some(interweave_human_android_platform::LogLevel::Info),
        "the running runtime's level, not the file's"
    );
    let _ = service.stop(GRACE);
    assert_eq!(service.log_level(), None, "nothing runs again");
}

#[test]
fn a_profile_no_endpoint_admits_is_refused_and_leaves_nothing_running() {
    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("paths");
    let good = std::fs::read_to_string(paths.config_file()).expect("read");
    let bad = good.replace(
        "allowed_client_kinds: [human-client]",
        "allowed_client_kinds: []",
    );
    assert_ne!(good, bad, "the control: the document names the kind");
    std::fs::write(paths.config_file(), &bad).expect("write");

    let service = ServiceHost::new();
    assert_eq!(
        service.start(launch(&app)),
        Err(StartRefused::NoHumanEndpoint)
    );
    assert!(!service.is_running());
    // The endpoint the Android profile names for its service decides,
    // not the first entry open to the kind; one that names no enabled
    // entry is refused by the profile's own validation, before the kind.
    let elsewhere = good.replace(
        "deployment: embedded-android",
        "deployment: embedded-android\n  android:\n    endpoint: phone",
    );
    assert_ne!(
        good, elsewhere,
        "the control: the document names the deployment"
    );
    std::fs::write(paths.config_file(), &elsewhere).expect("write");
    assert!(matches!(
        service.start(launch(&app)),
        Err(StartRefused::Runtime(EmbeddedRefused::ProfileInvalid(_)))
    ));
    // Nor does another entry open to the kind stand in for it.
    let other = bad.replace(
        "channels:",
        "    - id: other\n      enabled: true\n      advertise: false\n      \
         allowed_client_kinds: [human-client]\nchannels:",
    );
    assert_ne!(bad, other, "the control: the second entry was added");
    std::fs::write(paths.config_file(), &other).expect("write");
    assert_eq!(
        service.start(launch(&app)),
        Err(StartRefused::NoHumanEndpoint)
    );
    assert!(!service.is_running());
    // The runtime it started was stopped: its lock is free again.
    std::fs::write(paths.config_file(), &good).expect("write");
    service
        .start(launch(&app))
        .expect("starts once the profile admits it");
    let _ = service.stop(GRACE);
}

/// The availability the Service decides on: the stand-in profile's
/// default, foreground-only (ADR-0041: staying reachable is an opt-in),
/// and the profile's own field when it says stay-reachable.
#[test]
fn the_availability_is_the_profiles() {
    let app = app();
    let service = started(&app);
    assert_eq!(
        service.availability(),
        Some(AvailabilityMode::ForegroundOnly)
    );
    let _ = service.stop(GRACE);
    assert_eq!(service.availability(), None, "nothing runs");

    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("paths");
    let text = std::fs::read_to_string(paths.config_file()).expect("read");
    let reachable = text.replace(
        "deployment: embedded-android",
        "deployment: embedded-android\n  android:\n    availability_mode: stay-reachable",
    );
    assert_ne!(text, reachable, "the control: the document was changed");
    std::fs::write(paths.config_file(), reachable).expect("write");
    service.start(launch(&app)).expect("started");
    assert_eq!(
        service.availability(),
        Some(AvailabilityMode::StayReachable)
    );
    let _ = service.stop(GRACE);
}

/// A runtime that ends on its own releases the Service's wait with that
/// outcome, not as a request; the stop after it releases the profile.
#[test]
fn a_runtime_that_ends_on_its_own_is_said_as_its_end() {
    let app = app();
    let service = Arc::new(started(&app));
    let waiting = Arc::clone(&service);
    let waiter = std::thread::spawn(move || waiting.wait_ended());
    std::thread::sleep(Duration::from_millis(200));
    service.end_runtime_for_test();
    assert_eq!(
        waiter.join().expect("the waiter returned"),
        Ended::RuntimeEnded
    );
    let stopped = service.stop(GRACE).expect("it ran");
    assert!(stopped.runtime.is_ok(), "{:?}", stopped.runtime);
    service
        .start(launch(&app))
        .expect("a fresh start after the end");
    let _ = service.stop(GRACE);
}

/// The person's Stay-reachable choice moves the effective availability,
/// and the Service's wait answers the new mode; removing the choice
/// brings the authored default back (ADR-0041 A 2026-10-10).
#[test]
fn the_persons_choice_changes_the_availability_and_says_so() {
    let app = app();
    let service = Arc::new(started(&app));
    assert_eq!(
        service.availability(),
        Some(AvailabilityMode::ForegroundOnly)
    );

    let waiting = Arc::clone(&service);
    let waiter = std::thread::spawn(move || waiting.wait_ended());
    std::thread::sleep(Duration::from_millis(200));
    service
        .set_availability(Some(StayReachable))
        .expect("the choice is written");
    assert_eq!(
        waiter.join().expect("the waiter returned"),
        Ended::AvailabilityChanged(AvailabilityMode::StayReachable)
    );
    assert_eq!(
        service.availability(),
        Some(AvailabilityMode::StayReachable)
    );

    // The Service restarts in the new mode; removing the choice there
    // returns to the authored default, said again.
    let _ = service.stop(GRACE);
    service.start(launch(&app)).expect("restarted");
    assert_eq!(
        service.availability(),
        Some(AvailabilityMode::StayReachable)
    );
    service
        .set_availability(None)
        .expect("the choice is removed");
    assert_eq!(
        service.wait_ended(),
        Ended::AvailabilityChanged(AvailabilityMode::ForegroundOnly)
    );
    let _ = service.stop(GRACE);
    assert!(
        service.set_availability(None).is_err(),
        "nothing to record a choice against while nothing runs"
    );
}

#[test]
fn the_stand_in_profile_is_written_once_and_never_rewritten() {
    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("paths");
    let mode = std::fs::metadata(paths.config_dir())
        .expect("stat")
        .permissions()
        .mode();
    assert_eq!(
        mode & 0o777,
        0o700,
        "the configuration directory is private"
    );
    let changed = format!(
        "{}# a person's change\n",
        std::fs::read_to_string(paths.config_file()).expect("read")
    );
    std::fs::write(paths.config_file(), &changed).expect("write");
    stand_in::provision(&app.dir, PROFILE).expect("again");
    assert_eq!(
        std::fs::read_to_string(paths.config_file()).expect("read"),
        changed
    );
}

/// No identity's endpoint is in the directory: the app has no control
/// that would take it out (`human-client-ui.md` §13).
#[test]
fn the_stand_in_profile_advertises_no_endpoint() {
    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("paths");
    let profile = ProfileConfig::load(&paths).expect("the stand-in loads");
    assert!(
        !profile.endpoints.entries.is_empty(),
        "an endpoint is written"
    );
    for endpoint in &profile.endpoints.entries {
        assert!(!endpoint.advertise, "{:?} is advertised", endpoint.id);
    }
}

/// The android-e2e cases' provisioning replaces what is there -- the
/// host names each start's profile -- where the app's own never does,
/// and it writes under the same private directory.
#[test]
fn the_cases_provisioning_replaces_the_profile_where_the_apps_keeps_it() {
    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    stand_in::provision_replacing(&app.dir, PROFILE, "# the host's profile\n").expect("replaced");
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("paths");
    assert_eq!(
        std::fs::read_to_string(paths.config_file()).expect("read"),
        "# the host's profile\n"
    );

    let fresh = self::app();
    stand_in::provision_replacing(&fresh.dir, PROFILE, "# first\n").expect("on nothing");
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&fresh.dir).expect("b"))
        .expect("paths");
    let mode = std::fs::metadata(paths.config_dir())
        .expect("stat")
        .permissions()
        .mode();
    assert_eq!(
        mode & 0o777,
        0o700,
        "the configuration directory is private"
    );
}

/// The network-denied posture (human-client-android.md, "Runtime
/// permissions and the network-denied state"): the store opens and a view
/// gets its listing, told access is withheld; no runtime runs, so there is
/// no availability and no wait, and no session ever opens. A stop, then a
/// full start, is how a grant is answered: the view hears access again
/// and the session opens.
#[test]
fn without_network_the_store_opens_and_no_runtime_runs() {
    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let service = ServiceHost::new();
    service
        .start_without_network(
            &app.dir,
            PROFILE,
            std::sync::Arc::new(interweave_human_store::key_custody::FilesystemCustody),
        )
        .expect("the posture starts");
    assert!(service.is_running());
    assert!(service.is_network_denied());
    assert_eq!(service.availability(), None, "no runtime: no availability");
    assert_eq!(
        service.wait_ended(),
        Ended::NotRunning,
        "no runtime to wait on"
    );
    assert_eq!(
        service.start_without_network(
            &app.dir,
            PROFILE,
            std::sync::Arc::new(interweave_human_store::key_custody::FilesystemCustody)
        ),
        Ok(()),
        "a second start changes nothing"
    );

    let mut link = attach(&service);
    let mut held = Vec::new();
    until(&mut link, &mut held, "running", |m| {
        matches!(m, ToView::Running(true))
    });
    until(&mut link, &mut held, "access withheld", |m| {
        matches!(m, ToView::NetworkAccess(false))
    });
    until(&mut link, &mut held, "the listing", is_listing);
    // Long enough for a facade with a runtime behind it to be Ready (the
    // first test's start reaches it well inside this).
    let watched = Instant::now() + Duration::from_secs(2);
    while Instant::now() < watched {
        held.extend(link.take());
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !held.iter().any(is_ready),
        "no session opens without a runtime: {held:?}"
    );

    let stopped = service.stop(GRACE).expect("it ran");
    assert_eq!(stopped.runtime, Ok(0), "no runtime to stop");
    assert!(!service.is_running());

    service
        .start(launch(&app))
        .expect("the full start after a grant");
    assert!(!service.is_network_denied(), "the control: a runtime runs");
    let mut held = Vec::new();
    until(&mut link, &mut held, "access again", |m| {
        matches!(m, ToView::NetworkAccess(true))
    });
    until(&mut link, &mut held, "a ready session", is_ready);
    let _ = service.stop(GRACE);
}

/// The posture refuses what the runtime would have refused, before it
/// opens anything: here a profile that was never written.
#[test]
fn without_network_a_missing_profile_is_refused_and_nothing_runs() {
    let app = app();
    let service = ServiceHost::new();
    let refused = service.start_without_network(
        &app.dir,
        PROFILE,
        std::sync::Arc::new(interweave_human_store::key_custody::FilesystemCustody),
    );
    assert!(
        matches!(
            refused,
            Err(StartRefused::Runtime(EmbeddedRefused::ProfileInvalid(_)))
        ),
        "{refused:?}"
    );
    assert!(!service.is_running());
}

/// The posture's promise, step 3: reading and Keep are LOCAL writes and
/// work exactly as ADR-0044 defines them with no runtime -- a read
/// deletes the unread row unless it is kept, and a kept read becomes a
/// kept row.
#[test]
fn without_network_a_message_is_read_and_kept_as_with_one() {
    use interweave_human_chat_protocol::{HumanChatV2, MessageKind, encode_outbound};
    use interweave_human_store::{
        AppMessageId, HumanStore, InboundOrigin, NewInbound, StoreOptions,
    };
    use interweave_human_ui_model::{Intent, UiModel};
    use interweave_transport_api::MediaType;

    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("paths");
    // A first posture start lays the profile's directories out; the store
    // is then seeded with two unread messages while nothing holds it.
    let service = ServiceHost::new();
    service
        .start_without_network(
            &app.dir,
            PROFILE,
            std::sync::Arc::new(interweave_human_store::key_custody::FilesystemCustody),
        )
        .expect("the posture starts");
    service.stop(GRACE).expect("it ran");
    {
        let mut store = HumanStore::open_profile(
            &paths,
            &interweave_human_store::key_custody::FilesystemCustody,
            StoreOptions::default(),
        )
        .expect("the store");
        let peer = interweave_profile_identity::ProfileIdentity::generate()
            .transport_identity()
            .expect("a PeerId");
        for serial in [1_u8, 2] {
            let envelope = HumanChatV2 {
                v: 2,
                kind: MessageKind::Text,
                app_message_id: format!("{serial:032x}"),
                text: format!("message {serial}"),
                reply_to: None,
                sent_at_ms: None,
                from_endpoint: None,
            };
            let encoded = encode_outbound(&envelope, interweave_transport_api::MAX_PAYLOAD_BYTES)
                .expect("encodes");
            store
                .commit_unread_inbound(&NewInbound {
                    app_message_id: AppMessageId::parse(envelope.app_message_id.clone())
                        .expect("id"),
                    // A direct row records the endpoint its sender asserted,
                    // as the facade writes one.
                    origin: InboundOrigin {
                        peer: peer.clone(),
                        endpoint: Some(
                            interweave_transport_api::EndpointId::parse("human").expect("endpoint"),
                        ),
                        channel: None,
                    },
                    media_type: Some(MediaType::parse(&encoded.media_type).expect("media type")),
                    payload: encoded.bytes,
                    received_at: u64::from(serial),
                    kind: interweave_human_store::InboundKind::Text,
                })
                .expect("committed unread");
        }
    }

    service
        .start_without_network(
            &app.dir,
            PROFILE,
            std::sync::Arc::new(interweave_human_store::key_custody::FilesystemCustody),
        )
        .expect("the posture starts again");
    let mut link = attach(&service);
    let mut held = Vec::new();
    let mut model = UiModel::new();
    let mut listing = None;
    until(&mut link, &mut held, "the listing", |m| match m {
        ToView::Update(u) => match &**u {
            Update::Listed(l) => {
                listing = Some(l.clone());
                true
            }
            _ => false,
        },
        _ => false,
    });
    let listing = listing.expect("listed");
    assert_eq!(listing.unread.len(), 2, "both unread rows listed");
    let (kept_row, read_row) = (listing.unread[0].row, listing.unread[1].row);
    model.unread_listed(listing.unread);

    // Read both; keep one.
    for row in [kept_row, read_row] {
        assert!(
            service.hub().command(Command::MarkRead(row)).is_ok(),
            "a facade takes commands"
        );
        until(
            &mut link,
            &mut held,
            "the read",
            |m| matches!(m, ToView::Update(u) if matches!(**u, Update::Read(r) if r == row)),
        );
        model.read(row);
    }
    let keep = model
        .conversations()
        .iter()
        .flat_map(|c| model.messages(&c.key))
        .flat_map(|item| model.actions(item.key))
        .find_map(|intent| match intent {
            Intent::Keep { item, from } if from.1 == kept_row => Some(Command::Keep { item, from }),
            _ => None,
        })
        .expect("Keep offered on the read message");
    assert!(service.hub().command(keep).is_ok());
    until(
        &mut link,
        &mut held,
        "the keep",
        |m| matches!(m, ToView::Update(u) if matches!(**u, Update::Kept { .. })),
    );
    service.stop(GRACE).expect("it ran");

    let store = HumanStore::open_profile(
        &paths,
        &interweave_human_store::key_custody::FilesystemCustody,
        StoreOptions::default(),
    )
    .expect("the store");
    assert!(
        store.unread_inbound().expect("unread").is_empty(),
        "a read deletes the unread row"
    );
    let kept = store.kept_inbound().expect("kept");
    assert_eq!(kept.len(), 1, "only the kept read survives");
    assert_eq!(kept[0].app_message_id.as_str(), format!("{:032x}", 1));
}

/// #309 F8: a view attaching to a facade that has run is told each pending
/// row's status as it is now, after its listing, without waiting for the
/// row's next attempt: the listing alone reads a pending row as a first
/// attempt ("Sending"), and the facade reports a row only when it changes.
#[test]
fn a_view_attaching_late_is_told_a_pending_rows_status_at_once() {
    use interweave_human_client_api::OutboundStatus;
    use interweave_human_ui_model::ConversationKey;

    let app = app();
    let service = started(&app);
    let mut first = attach(&service);
    let mut held = Vec::new();
    until(&mut first, &mut held, "a ready session", is_ready);
    // A peer this profile does not trust: the send needs the person and
    // the facade never retries it, so after this no event can stand in
    // for the late view's replay (#309 round 2, risk).
    let stranger = interweave_profile_identity::ProfileIdentity::generate()
        .transport_identity()
        .expect("a PeerId");
    assert!(
        service
            .hub()
            .command(Command::Send {
                key: ConversationKey::Direct {
                    peer: stranger,
                    endpoint: None,
                },
                draft: "to nobody".to_owned(),
            })
            .is_ok()
    );
    let settled = |status: &OutboundStatus| matches!(status, OutboundStatus::NeedsAttention { .. });
    let mut row = None;
    until(&mut first, &mut held, "a settled pending row", |message| {
        if let ToView::Update(update) = message
            && let Update::Client(ClientEvent::Outbound(outbound)) = &**update
            && settled(&outbound.status)
        {
            row = Some(outbound.row);
            return true;
        }
        false
    });
    let row = row.expect("a row");

    let mut late = attach(&service);
    let mut held = Vec::new();
    until(&mut late, &mut held, "the late view's listing", is_listing);
    let deadline = Instant::now() + Duration::from_millis(1_000);
    let told = loop {
        held.extend(late.take());
        let told = held.iter().any(|message| {
            matches!(message, ToView::Update(update)
                if matches!(&**update, Update::Client(ClientEvent::Outbound(o)) if o.row == row))
        });
        if told || Instant::now() > deadline {
            break told;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(told, "the row's status came with the listing");
    service.stop(GRACE).expect("it ran");
}

/// A view attaching to a facade whose session is ready is told this
/// node's `PeerId` with its listing, as the Ready it missed would have
/// told it: Peers reads this device's own row by it.
#[test]
fn a_view_attaching_late_is_told_this_device() {
    let app = app();
    let service = started(&app);
    let mut first = attach(&service);
    let mut held = Vec::new();
    until(&mut first, &mut held, "a ready session", is_ready);
    let mut late = attach(&service);
    let mut held = Vec::new();
    until(
        &mut late,
        &mut held,
        "this device's PeerId",
        |message| matches!(message, ToView::Update(u) if matches!(**u, Update::ThisDevice(_))),
    );
    service.stop(GRACE).expect("it ran");
}

/// A stand-in for the Keystore's store key, sealing nothing (the secret
/// is the body) so it can be told to refuse as invalidated: what the
/// Service does with that refusal, not the Keystore, is what is tested
/// here (the Keystore is the device row's).
struct StoreKeystore {
    invalidated: std::sync::atomic::AtomicBool,
}

impl interweave_human_store::key_custody::KeyCipher for StoreKeystore {
    fn custody(&self) -> interweave_human_store::key_custody::Custody {
        interweave_human_store::key_custody::Custody::Keystore
    }

    fn seal(
        &self,
        _: &str,
        _: &[u8],
        secret: &[u8; 32],
    ) -> Result<Vec<u8>, interweave_human_store::key_custody::CipherFailure> {
        self.invalidated
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(secret.to_vec())
    }

    fn open(
        &self,
        _: &str,
        _: &[u8],
        sealed: &[u8],
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, interweave_human_store::key_custody::CipherFailure>
    {
        if self.invalidated.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(interweave_human_store::key_custody::CipherFailure::KeyInvalidated);
        }
        let secret: [u8; 32] = sealed
            .try_into()
            .map_err(|_| interweave_human_store::key_custody::CipherFailure::Unauthentic)?;
        Ok(zeroize::Zeroizing::new(secret))
    }

    fn retire(&self, _: &str) -> Result<(), interweave_human_store::key_custody::CipherFailure> {
        Ok(())
    }
}

/// ADR-0060 decision 6 on Android: an invalidated store key is its own
/// refusal, never recovery mode and never an identity condition; Discard
/// and recreate is refused while the client holds the store and, once it
/// is stopped and the person confirmed, removes the store and its key so
/// the next start recreates it.
#[test]
fn an_invalidated_store_key_refuses_the_start_and_discard_recreates_the_store() {
    use interweave_human_store::at_rest::DiscardScope;
    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let keystore = std::sync::Arc::new(StoreKeystore {
        invalidated: std::sync::atomic::AtomicBool::new(false),
    });
    let service = ServiceHost::new();
    service
        .start_without_network(&app.dir, PROFILE, keystore.clone())
        .expect("the first start makes the store and its key");
    assert!(matches!(
        service.plan_store_discard(&app.dir, PROFILE, &keystore),
        Err(interweave_human_android_platform::StoreActionRefused::Running)
    ));
    let _ = service.stop(GRACE);

    keystore
        .invalidated
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        service.start_without_network(&app.dir, PROFILE, keystore.clone()),
        Err(StartRefused::StoreKeyInvalidated)
    );
    assert!(!service.is_running());
    // A view attached now hears which screen the refusal calls for, at
    // the hub's count: the opened first start, then this refusal.
    let mut link = service.hub().attach(Arc::new(|| {}));
    let refused = store_answers(&mut link);
    assert_eq!(
        refused,
        [(
            StoreAnswer::Refused(interweave_human_android_platform::StoreRefusal::KeyLost),
            2
        )],
        "the key-lost screen"
    );
    let health = service
        .store_health(&app.dir, PROFILE, &keystore)
        .expect("health");
    assert_eq!(
        health.key,
        interweave_human_store::at_rest::KeyPresence::Invalidated
    );

    let plan = service
        .plan_store_discard(&app.dir, PROFILE, &keystore)
        .expect("a plan");
    assert_eq!(plan.scope(), DiscardScope::Everything);
    assert!(plan.removes_key());
    let discarded = service
        .discard_store(&app.dir, PROFILE, &keystore, plan.confirm())
        .expect("discarded");
    assert_eq!(
        discarded.answered.count(),
        2,
        "the discard ends after the refusal's answer"
    );
    service
        .start_without_network(&app.dir, PROFILE, keystore.clone())
        .expect("the store is recreated under a new key");
    let opened = store_answers(&mut link);
    assert_eq!(
        opened,
        [(StoreAnswer::Opened, 3)],
        "the screen goes once a start opened the store, counted past the discard"
    );
    let _ = service.stop(GRACE);
}

/// The store answers a view took from `link`, with the hub's count.
fn store_answers(link: &mut ViewLink) -> Vec<(StoreAnswer, u64)> {
    link.take()
        .into_iter()
        .filter_map(|m| match m {
            ToView::Store(answer, generation) => Some((answer, generation.count())),
            _ => None,
        })
        .collect()
}

/// A start refused for a reason no store screen shows answers so, never
/// as an opened store, whether the host refused it -- here no profile --
/// or the platform side did before reaching it (#377 review r3,
/// pre-existing); a start while the client runs answers nothing new.
#[test]
fn a_start_refused_elsewhere_answers_so_and_never_as_an_opened_store() {
    let app = app();
    let keystore = std::sync::Arc::new(StoreKeystore {
        invalidated: std::sync::atomic::AtomicBool::new(false),
    });
    let service = ServiceHost::new();
    let mut link = service.hub().attach(Arc::new(|| {}));
    assert!(store_answers(&mut link).is_empty(), "no start, no answer");
    assert!(
        service
            .start_without_network(&app.dir, PROFILE, keystore.clone())
            .is_err(),
        "no profile"
    );
    service.refused_before_start();
    assert_eq!(
        store_answers(&mut link),
        [
            (StoreAnswer::OtherRefusal, 1),
            (StoreAnswer::OtherRefusal, 2)
        ]
    );

    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    service
        .start_without_network(&app.dir, PROFILE, keystore.clone())
        .expect("started");
    service
        .start_without_network(&app.dir, PROFILE, keystore.clone())
        .expect("a start while running does nothing");
    service.refused_before_start();
    assert_eq!(
        store_answers(&mut link),
        [(StoreAnswer::Opened, 3)],
        "the second start and the late refusal changed nothing"
    );
    let _ = service.stop(GRACE);
}

/// The identity in front as the first listing after a start reports it.
fn listed_identity(service: &ServiceHost) -> Option<interweave_human_ui_model::IdentityView> {
    let mut link = attach(service);
    let mut held = Vec::new();
    let mut identity = None;
    until(&mut link, &mut held, "the listing", |m| match m {
        ToView::Update(u) => match &**u {
            Update::Listed(l) => {
                identity = Some(l.identity.clone());
                true
            }
            _ => false,
        },
        _ => false,
    });
    identity.expect("listed")
}

/// ADR-0063 decision 2 on Android: the first start makes the identity's
/// key sealed under the platform's custody, beside the store and outside
/// it, and every later start opens that key, never a new one; the record
/// is the Keystore's, which the desktop's filesystem floor cannot read.
#[test]
fn the_service_makes_the_identitys_key_under_the_keystore_and_opens_it_after() {
    use interweave_human_store::identity_key::{
        self, FilesystemCustody, IdentityKeyError, IdentityKeyFileError,
    };
    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let keystore = std::sync::Arc::new(StoreKeystore {
        invalidated: std::sync::atomic::AtomicBool::new(false),
    });
    let service = ServiceHost::new();
    service
        .start_without_network(&app.dir, PROFILE, keystore.clone())
        .expect("started");
    let first = listed_identity(&service).expect("the identity has its key");
    let _ = service.stop(GRACE);

    service
        .start_without_network(&app.dir, PROFILE, keystore.clone())
        .expect("started again");
    let again = listed_identity(&service).expect("the key opened");
    assert!(first.key.fingerprint().is_some(), "a key that opens");
    assert_eq!(again.key, first.key, "the same key");
    let _ = service.stop(GRACE);

    let paths =
        ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("a boundary"))
            .expect("paths");
    assert_eq!(
        identity_key::open(&paths, &first.endpoint, &FilesystemCustody).expect_err("sealed"),
        IdentityKeyFileError::Key(IdentityKeyError::SealedUnreadableHere),
        "the record is held at the Keystore's custody"
    );
}

/// #414 F3: a start that refuses the identity's key file runs with no
/// key and keeps the refusal's class for the platform log; the file is
/// left as it is, and a start that opens a key clears the refusal.
#[test]
fn a_refused_key_file_is_kept_for_the_log_and_left_as_it_is() {
    use interweave_human_store::identity_key;
    let app = app();
    stand_in::provision(&app.dir, PROFILE).expect("provisioned");
    let keystore = std::sync::Arc::new(StoreKeystore {
        invalidated: std::sync::atomic::AtomicBool::new(false),
    });
    let service = ServiceHost::new();
    service
        .start_without_network(&app.dir, PROFILE, keystore.clone())
        .expect("started");
    let first = listed_identity(&service).expect("the identity");
    assert!(first.key.fingerprint().is_some(), "a key");
    assert_eq!(service.identity_key_refused(), None);
    let _ = service.stop(GRACE);

    let paths =
        ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("a boundary"))
            .expect("paths");
    let file = identity_key::key_path(&paths, &first.endpoint);
    let good = std::fs::read(&file).expect("read");
    std::fs::write(&file, "not a record\n").expect("overwrite");
    service
        .start_without_network(&app.dir, PROFILE, keystore.clone())
        .expect("started with no key");
    assert_eq!(
        listed_identity(&service).expect("the identity").key,
        interweave_human_ui_model::OwnKey::Unusable(interweave_human_app_core::KeyProblem::Damaged),
        "no key, nothing signed, and its page says why"
    );
    assert_eq!(
        service.identity_key_refused(),
        Some(interweave_human_app_core::KeyProblem::Damaged)
    );
    let _ = service.stop(GRACE);
    assert_eq!(
        std::fs::read(&file).expect("read"),
        b"not a record\n",
        "left as it is"
    );

    std::fs::write(&file, &good).expect("restore");
    service
        .start_without_network(&app.dir, PROFILE, keystore.clone())
        .expect("started");
    assert_eq!(service.identity_key_refused(), None, "cleared");
    let _ = service.stop(GRACE);
}
