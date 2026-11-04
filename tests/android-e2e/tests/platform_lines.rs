// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Two of plan §20's Platform-tests lines as instrumented cases, where
//! the stage closed them by dated device-run records
//! (`device-runs/2026-10-20-gate-b.md`, read by hand) and carried the
//! cases to the owner's release gate by the names rust-ui-dev gave them
//! on 2026-10-22, `backup_exclusion` and `fgs_lifecycle`. Each reads the
//! platform's own services (`src/platform.rs`), never the app's account
//! of itself.
//!
//! - `backup_exclusion`: `bmgr backupnow` for the app and, as the
//!   positive control, for `com.android.providers.settings`, under the
//!   local transport and, where the phone has it, GMS's device-transfer
//!   transport. The control must be `Success`, or Backup Manager took
//!   nothing and the app's refusal would measure nothing (the Pixel's
//!   first run on 2026-10-20 was void that way). The app must be `Backup
//!   is not allowed`. The transport and Backup Manager's state are put
//!   back as found, on a failing run's unwind too.
//! - `fgs_lifecycle`: under `foreground-only`, the window open binds the
//!   service without making it foreground, and leaving the window ends
//!   it; under `stay-reachable`, the service is foreground in the
//!   `reachable` channel, stays foreground behind Home, survives
//!   `am kill`, and after `am crash` is restarted in a new process,
//!   foreground again. At API 34+ its type must read `remoteMessaging`.
//!
//! Device only (`#[ignore]`d), as `src/adb.rs` says. `fgs_lifecycle`
//! provisions the phone's own profile with only its authored
//! availability mode changed, and puts the profile back as found when it
//! ends (`platform::ProfileKept`). It needs a `background-compatible`
//! key, and refuses another, since a restart under `user-presence` waits
//! for the person and the crash restart below would measure that wait;
//! and the foreground-only case refuses a phone whose person chose to
//! stay reachable, a choice that overrides any mode provisioned and that
//! the case does not change.
//!
//! What this does NOT prove: device transfer under `dataExtractionRules`
//! at API 31+, which the record carries: GMS's D2D transport is asserted
//! only below API 31, where `allowBackup` governs it, and reported above;
//! a person's view of the foreground notice; and a real low-memory kill
//! (`am crash` is not one).

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use interweave_android_e2e_tests::adb::{AdbDevice, PACKAGE};
use interweave_android_e2e_tests::platform::{
    self, BackupResult, ProfileKept, REMOTE_MESSAGING, ServiceRecord, app_pid, network_service,
    sdk, shell,
};
use interweave_android_e2e_tests::{Device, PATIENCE};
use interweave_profile_config::runtime::{AvailabilityMode, KeyUnlockPolicy};

/// The positive control: a package that allows backup on every phone.
const CONTROL: &str = "com.android.providers.settings";

const LOCAL_TRANSPORT: &str = "com.android.localtransport/.LocalTransport";
const D2D_TRANSPORT: &str = "com.google.android.gms/.backup.migrate.service.D2dTransport";

/// API 31: `dataExtractionRules` replaces `allowBackup` for device
/// transfer.
const EXTRACTION_RULES_FROM: u32 = 31;

/// API 34: a foreground service must declare its type.
const TYPES_REQUIRED_FROM: u32 = 34;

#[test]
#[ignore = "needs a device with the app installed: ANDROID_SERIAL (src/adb.rs); one phone, so --test-threads 1"]
fn backup_exclusion() {
    let api = sdk();
    let _restore = BackupState::take();
    shell(&["bmgr", "enable", "true"]);
    let (listed, _) = platform::transports(&shell(&["bmgr", "list", "transports"]));
    assert!(
        listed.iter().any(|t| t == LOCAL_TRANSPORT),
        "the local transport is on every build: {listed:?}"
    );
    let mut measured = Vec::new();
    for transport in [LOCAL_TRANSPORT, D2D_TRANSPORT] {
        if !listed.iter().any(|t| t == transport) {
            eprintln!("backup_exclusion: no {transport} on this phone (API {api})");
            continue;
        }
        shell(&["bmgr", "transport", transport]);
        let out = shell(&["bmgr", "backupnow", PACKAGE, CONTROL]);
        assert_eq!(
            platform::backup_result(&out, CONTROL),
            Some(BackupResult::Success),
            "control: {CONTROL} backs up under {transport}, or nothing was measured:\n{out}"
        );
        let app = platform::backup_result(&out, PACKAGE);
        if transport == D2D_TRANSPORT && api >= EXTRACTION_RULES_FROM {
            // The carried doubt, reported and not asserted.
            eprintln!("backup_exclusion: API {api}, {transport}: the app {app:?} (not asserted)");
            continue;
        }
        assert_eq!(
            app,
            Some(BackupResult::NotAllowed),
            "the app refused under {transport} (API {api}):\n{out}"
        );
        measured.push(transport);
    }
    eprintln!("backup_exclusion: API {api}, refused under {measured:?}");
}

#[test]
#[ignore = "needs a device with the app's androidTest build: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); one phone, so --test-threads 1"]
fn fgs_lifecycle_foreground_only_binds_and_ends_with_the_window() {
    let kept = background_compatible();
    assert!(
        !kept.stay_reachable_chosen(),
        "the person chose to stay reachable on this phone, which overrides foreground-only"
    );
    let mut phone = AdbDevice::connect();
    phone.start(&config(&kept, AvailabilityMode::ForegroundOnly));
    let log = || phone.log();

    launch();
    let bound = until_some(&log, "the service bound", network_service);
    assert!(
        !bound.foreground && !bound.start_requested,
        "open under foreground-only: bound, neither foreground nor started: {bound:?}"
    );

    home();
    until(&log, "the service ended with the window", || {
        network_service().is_none()
    });

    // Back to the window: bound again, so the end above was the window's
    // leaving and not a service that never came back.
    launch();
    let again = until_some(&log, "the service bound again", network_service);
    assert!(!again.foreground, "{again:?}");
    phone.stop();
}

#[test]
#[ignore = "needs a device with the app's androidTest build: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); one phone, so --test-threads 1"]
fn fgs_lifecycle_stay_reachable_is_foreground_and_restarts_after_a_crash() {
    let api = sdk();
    let kept = background_compatible();
    let mut phone = AdbDevice::connect();
    phone.start(&config(&kept, AvailabilityMode::StayReachable));
    let log = || phone.log();

    launch();
    let open = until_some(&log, "the service foreground", foreground);
    reachable(&open, api);

    home();
    // Behind Home for a while, foreground throughout: one read could fall
    // before a demotion that came later.
    let hold = Instant::now() + Duration::from_secs(5);
    while Instant::now() < hold {
        let r = network_service().expect("the service behind Home");
        assert!(r.foreground, "foreground behind Home: {r:?}");
        std::thread::sleep(Duration::from_millis(500));
    }

    let pid = app_pid().expect("the app's process");
    shell(&["am", "kill", PACKAGE]);
    std::thread::sleep(Duration::from_secs(2));
    assert_eq!(
        app_pid(),
        Some(pid),
        "am kill does not end a foreground service's process"
    );

    shell(&["am", "crash", PACKAGE]);
    let after = until_some(&log, "a new process for the service", || {
        app_pid().filter(|p| *p != pid)
    });
    let restarted = until_some(&log, "the restarted service foreground", foreground);
    reachable(&restarted, api);
    assert_eq!(app_pid(), Some(after), "one restart, no second");
    phone.stop();
}

/// The phone's profile, kept to be put back, when its key does not wait
/// for the person.
fn background_compatible() -> ProfileKept {
    let kept = ProfileKept::take();
    assert_eq!(
        kept.unlock_policy(),
        KeyUnlockPolicy::BackgroundCompatible,
        "fgs_lifecycle needs a phone whose key does not wait for the person"
    );
    kept
}

/// The phone's own profile under the authored `mode`, its key policy
/// and peers as they are.
fn config(kept: &ProfileKept, mode: AvailabilityMode) -> String {
    platform::with_availability(kept.found(), mode).unwrap_or_else(|e| panic!("{e}"))
}

fn foreground() -> Option<ServiceRecord> {
    network_service().filter(|r| r.foreground)
}

/// The stay-reachable service's record: started, in its channel, and of
/// its declared type where the level requires one.
fn reachable(r: &ServiceRecord, api: u32) {
    assert!(r.start_requested, "started, not only bound: {r:?}");
    assert_eq!(r.channel.as_deref(), Some("reachable"), "{r:?}");
    if api >= TYPES_REQUIRED_FROM {
        assert_eq!(r.types, Some(REMOTE_MESSAGING), "API {api}: {r:?}");
    } else if let Some(t) = r.types {
        assert_eq!(t, REMOTE_MESSAGING, "API {api}: {r:?}");
    }
}

fn launch() {
    shell(&[
        "am",
        "start",
        "-W",
        "-n",
        &format!("{PACKAGE}/.MainActivity"),
    ]);
}

fn home() {
    shell(&["input", "keyevent", "KEYCODE_HOME"]);
}

/// Backup Manager's state and transport as found, put back when dropped.
struct BackupState {
    enabled: bool,
    transport: String,
}

impl BackupState {
    /// The state as found; one this cannot read fails the case before
    /// anything is changed, so that the drop never puts back a guess.
    fn take() -> Self {
        let said = shell(&["bmgr", "enabled"]);
        let enabled = match (
            said.contains("currently enabled"),
            said.contains("currently disabled"),
        ) {
            (true, false) => true,
            (false, true) => false,
            _ => panic!("bmgr enabled said neither enabled nor disabled: {said}"),
        };
        let listed = shell(&["bmgr", "list", "transports"]);
        let (_, transport) = platform::transports(&listed);
        let transport =
            transport.unwrap_or_else(|| panic!("no selected transport to put back:\n{listed}"));
        Self { enabled, transport }
    }
}

impl Drop for BackupState {
    fn drop(&mut self) {
        // Never a second panic while unwinding: not asserted.
        let run = |args: &[&str]| {
            let _ = std::process::Command::new("adb")
                .arg("shell")
                .args(args)
                .output();
        };
        run(&["bmgr", "transport", &self.transport]);
        run(&[
            "bmgr",
            "enable",
            if self.enabled { "true" } else { "false" },
        ]);
    }
}

fn until(log: &impl Fn() -> String, what: &str, done: impl Fn() -> bool) {
    until_some(log, what, || done().then_some(()));
}

fn until_some<T>(log: &impl Fn() -> String, what: &str, found: impl Fn() -> Option<T>) -> T {
    let deadline = Instant::now() + PATIENCE;
    loop {
        if let Some(t) = found() {
            return t;
        }
        assert!(
            Instant::now() < deadline,
            "{what} did not happen in time\n{}",
            log()
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}
