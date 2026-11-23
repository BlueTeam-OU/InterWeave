// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Four of the cases Stage 17's closing record (plan §20) carried to
//! the owner's release gate unbuilt, where the stage met their lines by
//! the gate-b record: `secure_span` and `presence_prompt_and_cancel`
//! (rust-ui-dev's names, 2026-10-22) and the Keystore user-presence
//! case. They step the app's own screens by key events
//! (`src/screen.rs`, the debug build's trace) and read the platform
//! beside each step (`src/platform.rs`): which of the app's windows
//! carry `FLAG_SECURE`, whether the system's credential prompt shows,
//! and whether the network service runs.
//!
//! - `secure_span_show_my_address`: no window of the app is `SECURE` on
//!   the conversation list; Show my address drawn makes one so; closed,
//!   none is again. No person.
//! - `secure_span_recovery_words`: the same around the recovery words.
//!   On Trust none is `SECURE`; after the person's grant the words page
//!   is drawn with a window `SECURE`; Back closes the flow and none is
//!   again. A person grants the prompt.
//! - `presence_prompt_and_cancel`: a cold launch past the key's
//!   validity raises the prompt before the service runs; leaving it by
//!   Recents is the person's cancel; on return no prompt is raised again
//!   and the service still does not run. No person.
//! - `keystore_user_presence_unlock`: from the locked state, Unlock asks
//!   the prompt, the person's grant unwraps the key, and the service
//!   then runs. A person grants the prompt.
//! - `r1_warns_that_the_phrase_may_be_live_elsewhere` and
//!   `k1_warns_that_the_phrase_may_be_live_elsewhere`: TM-48's case, the
//!   warning on R1 and on K1 read off the trace's `warns=` line, which
//!   says what the page handed to the window carries. No person.
//!
//! The presence cases need a phone whose app holds a `user-presence`
//! identity a person made, as `device-runs/2026-10-20-gate-b.md` set it
//! up: no instrumentation can wrap a seed under a key that needs the
//! person. A case with a person waits for them, naming the step, and
//! needs `INTERWEAVE_ANDROID_PERSON` set, so a run with no one there
//! fails at once instead of timing out.
//!
//! What this does NOT prove: what the person sees (`FLAG_SECURE` turns
//! screencap black, so the flag is read and not the frame); that the
//! flag was on before the first frame with a word, rather than at the
//! read after the trace line (the trace line comes after the Activity's
//! flag, as rust-ui-dev's onboard.rs orders it, but the read cannot see
//! the frames between); the Recents thumbnail; a biometric rather than
//! a PIN. Keystore invalidation on a lock-screen change is
//! `an_invalidated_user_presence_key_opens_on_k1_and_mints_nothing`,
//! S21-12's case: it needs the person's lock settings and leaves the
//! profile in recovery. It passed on the Samsung A40, API 30
//! (`runs/2026-11-23-samsung-a40-s21-12-invalidated-key.md`).

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::io::Write as _;
use std::process::Command;
use std::time::{Duration, Instant};

use interweave_android_e2e_cases::keys;
use interweave_android_e2e_tests::adb::{self, PACKAGE};
use interweave_android_e2e_tests::platform::{
    self, CONFIG, ScreenAwake, network_service, prompt_shown,
};
use interweave_android_e2e_tests::screen::{Screens, Step};
use interweave_profile_config::runtime::KeyUnlockPolicy;

/// How long a screen's answer may take over the phone's own runtime.
const PATIENCE: Duration = Duration::from_secs(30);

/// How long a person is given for a prompt.
const PERSON: Duration = Duration::from_secs(180);

/// The key's validity after an authentication is 60 s: a launch after
/// longer than this finds it locked.
const PAST_VALIDITY: Duration = Duration::from_secs(70);

/// How long a return is watched for a prompt raised again.
const QUIET: Duration = Duration::from_secs(8);

/// Set when a person is at the phone to grant a prompt.
const PERSON_ENV: &str = "INTERWEAVE_ANDROID_PERSON";

#[test]
#[ignore = "a phone with the debug APK and an identity, ANDROID_SERIAL naming it"]
fn secure_span_show_my_address() {
    let mut s = screens();
    passed(&s.launch(PATIENCE), &s);
    until("no SECURE window on the conversation list", || !secure());
    passed(&s.open_peers(PATIENCE), &s);
    passed(&s.hold_show(PATIENCE), &s);
    assert!(
        secure(),
        "Show my address drawn: a SECURE window {:?}",
        platform::windows()
    );
    passed(&s.close_show(PATIENCE), &s);
    until("no SECURE window after Show closed", || !secure());
}

#[test]
#[ignore = "a phone with the debug APK and an identity, ANDROID_SERIAL naming it, and a person: INTERWEAVE_ANDROID_PERSON set"]
fn secure_span_recovery_words() {
    person();
    let mut s = screens();
    passed(&s.launch(PATIENCE), &s);
    passed(&s.open_trust(PATIENCE), &s);
    until("no SECURE window on Trust", || !secure());
    passed(&s.open_phrase(PATIENCE), &s);
    passed(&s.ask_words(PATIENCE), &s);
    let from = s.mark();
    ask_person("grant the prompt for the recovery words");
    passed(&s.words_shown(from, PERSON), &s);
    assert!(
        secure(),
        "the words drawn: a SECURE window {:?}",
        platform::windows()
    );
    passed(&s.close_words(PATIENCE), &s);
    until("no SECURE window after the words closed", || !secure());
}

#[test]
#[ignore = "a phone with the debug APK and a user-presence identity a person made, ANDROID_SERIAL naming it"]
fn presence_prompt_and_cancel() {
    user_presence();
    let mut s = screens();
    let _awake = locked_launch(&mut s);
    // Each "no service" is read off a dump that says it is not running
    // (`platform::service_record`): one that does not say so fails the
    // read, so these are not passed by a dump this does not recognise.
    assert!(
        network_service().is_none(),
        "no service before the person: {:?}",
        network_service()
    );

    passed(&s.leave_by_recents(PATIENCE), &s);
    let from = s.mark();
    passed(&s.bring_back(), &s);
    // The positive control for the quiet below: the prompt line the
    // trace carried at the cancel, read above.
    passed(&s.none_after(from, "prompt=", QUIET), &s);
    assert!(!prompt_shown(), "no prompt raised again on return");
    assert!(
        network_service().is_none(),
        "the service still not running after a cancel: {:?}",
        network_service()
    );
}

#[test]
#[ignore = "a phone with the debug APK and a user-presence identity a person made, ANDROID_SERIAL naming it, and a person: INTERWEAVE_ANDROID_PERSON set"]
fn keystore_user_presence_unlock() {
    person();
    user_presence();
    let mut s = screens();
    let _awake = locked_launch(&mut s);
    // The prompt the launch raised, ended as a cancel, so that the one
    // granted below is Unlock's.
    passed(&s.leave_by_recents(PATIENCE), &s);
    passed(&s.bring_back(), &s);
    assert!(network_service().is_none(), "control: locked, no service");

    passed(&s.unlock(PATIENCE), &s);
    let from = s.mark();
    ask_person("grant the Unlock prompt");
    passed(&s.lines_after(from, &["prompt=granted"], PERSON), &s);
    let deadline = Instant::now() + PATIENCE;
    while network_service().is_none() {
        assert!(Instant::now() < deadline, "the service after the grant");
        std::thread::sleep(Duration::from_millis(500));
    }
    passed(&s.lines_after(from, &["notice=none"], PATIENCE), &s);
}

/// TM-48's device case: R1, reached from Welcome on a first run, carries
/// ADR-0043's warning before R2 takes a word, R2 does not, and Back to R1
/// carries it again. Needs a phone whose app has no identity (its data
/// cleared): the case makes none, so it leaves the phone at Welcome.
#[test]
#[ignore = "a phone with the debug APK and no identity (the app's data cleared), ANDROID_SERIAL naming it"]
fn r1_warns_that_the_phrase_may_be_live_elsewhere() {
    let mut s = screens();
    passed(&s.launch_to_identity("w", PATIENCE), &s);
    let from = s.mark();
    passed(&s.open_restore(PATIENCE), &s);
    passed(
        &s.lines_after(from, &["identity=r1", "warns=active-elsewhere"], PATIENCE),
        &s,
    );
    let from = s.mark();
    passed(&s.open_phrase_entry(PATIENCE), &s);
    passed(&s.lines_after(from, &["warns=none"], PATIENCE), &s);
    passed(
        &s.identity_back(&["identity=r1", "warns=active-elsewhere"], PATIENCE),
        &s,
    );
}

/// TM-48's device case on K1: an identity whose key the Keystore no
/// longer holds opens on K1 with a restore offered, which carries the
/// warning, and R2 after it does not. Needs the seed key dropped first
/// (`E2eCase`'s `drop-seed-key`); the case restores nothing, so the
/// phone stays on K1.
#[test]
#[ignore = "a phone with the debug APK and an identity whose seed key was dropped (drop-seed-key), ANDROID_SERIAL naming it"]
fn k1_warns_that_the_phrase_may_be_live_elsewhere() {
    let mut s = screens();
    passed(&s.launch_to_identity("k1", PATIENCE), &s);
    passed(&s.lines_after(0, &["warns=active-elsewhere"], PATIENCE), &s);
    let from = s.mark();
    passed(&s.open_phrase_entry(PATIENCE), &s);
    passed(&s.lines_after(from, &["warns=none"], PATIENCE), &s);
    passed(
        &s.identity_back(&["identity=k1", "warns=active-elsewhere"], PATIENCE),
        &s,
    );
}

/// S21-12's case: a `user-presence` seed key invalidated on a
/// phone, and what the app shows for it. The person removes the screen
/// lock and sets it again, which the platform answers by invalidating
/// (or deleting) every key bound to the person's authentication; the
/// app then opens on K1, the recovery page, with the warning, no prompt
/// and no service; and the identity record is the one found, so nothing was
/// minted over it (the custody seam's
/// `an_invalidated_or_missing_key_requires_recovery_and_mints_nothing`,
/// on the phone).
///
/// The controls, before the change: the profile is `user-presence`, the
/// phone has a lock, the seed's key is bound to authentication (on the
/// A40 a lock change invalidated nothing, no key being bound then:
/// TM-54's record), and a locked launch opens the identity, offline,
/// with the prompt up — the state the change takes away. `key-info`'s
/// reading after the change says which the platform did, invalidated
/// (the alias kept, refused) or deleted (the alias gone): both open on
/// K1 (`RecoveryReason::Key`), so the case prints it and asserts neither.
///
/// It leaves the phone on K1 with its identity unusable: putting the
/// app back is a discard or `pm clear`, on the owner's word, which this
/// case does not do. Needs the androidTest APK
/// (`INTERWEAVE_ANDROID_INSTRUMENTATION`) for `key-info`.
#[test]
#[ignore = "a phone with the debug APK and its androidTest APK, a user-presence identity a person made, ANDROID_SERIAL naming it, and a person who may remove and set the screen lock: INTERWEAVE_ANDROID_PERSON set"]
fn an_invalidated_user_presence_key_opens_on_k1_and_mints_nothing() {
    person();
    user_presence();
    assert!(
        platform::screen_lock_now(),
        "control: the phone has a screen lock to remove"
    );
    let alias = format!("interweave.iwk1.{PROFILE}.user-presence");
    let before = seed_key(&alias).unwrap_or_else(|| panic!("control: no key at {alias}"));
    assert_eq!(
        before
            .get("user_authentication_required")
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "control: the seed's key is bound to the person's authentication: {before}"
    );
    let record = identity_record();

    let mut s = screens();
    {
        let _awake = locked_launch(&mut s);
        passed(&s.leave_by_recents(PATIENCE), &s);
    }
    let _ = Command::new("adb")
        .args(["shell", "am", "force-stop", PACKAGE])
        .output();

    let lock = LockPutBack;
    ask_person("open Settings > Lock screen > Screen lock type, choose None, and confirm");
    until_within("the screen lock removed", PERSON, || {
        !platform::screen_lock_now()
    });
    eprintln!("key-info, the lock removed: {:?}", seed_key(&alias));
    ask_person("set the same screen lock again");
    until_within(
        "the screen lock set again",
        PERSON,
        platform::screen_lock_now,
    );
    drop(lock);
    let after = seed_key(&alias);
    eprintln!("key-info, the lock set again: {after:?}");

    passed(&s.launch_to_identity("k1", PATIENCE), &s);
    passed(&s.lines_after(0, &["warns=active-elsewhere"], PATIENCE), &s);
    // A prompt or a service may come after the page's trace line, so the
    // app's own `prompt=` line is waited out first. Its positive control
    // is the prompt the locked launch above raised, read the same way.
    passed(&s.none_after(0, "prompt=", QUIET), &s);
    assert!(!prompt_shown(), "no prompt raised on K1");
    // No run of this case sees the service running, so this "no" leans on
    // `platform::service_record`, which refuses a dump it does not
    // recognise rather than reading it as not running.
    assert!(
        network_service().is_none(),
        "no service on K1: {:?}",
        network_service()
    );
    assert!(
        identity_record() == record,
        "the identity record is the one found: nothing minted over it"
    );
}

/// Past the key's validity, then a cold launch: locked, the prompt up.
/// The person unlocks the phone before the wait starts, since an unlock
/// after it opens the key's window again; the screen is then held on so
/// that no keyguard comes up meanwhile, for as long as the returned
/// hold lives (the case's whole body).
fn locked_launch(s: &mut Screens) -> ScreenAwake {
    platform::until_unlocked();
    let awake = ScreenAwake::hold();
    let _ = Command::new("adb")
        .args(["shell", "am", "force-stop", PACKAGE])
        .output();
    std::thread::sleep(PAST_VALIDITY);
    passed(&s.launch_locked(PATIENCE), s);
    until("the credential prompt raised at the launch", prompt_shown);
    awake
}

fn screens() -> Screens {
    Screens::new(&std::env::var("ANDROID_SERIAL").expect("ANDROID_SERIAL names the phone"))
}

fn secure() -> bool {
    platform::windows().iter().any(|(_, s)| *s)
}

/// The case needs a person, and one is there.
fn person() {
    assert!(
        std::env::var_os(PERSON_ENV).is_some(),
        "this case needs a person at the phone: set {PERSON_ENV}"
    );
}

fn ask_person(what: &str) {
    eprintln!("PERSON: on the phone, {what} (within {PERSON:?})");
}

/// The app's profile asks the person for its key.
fn user_presence() {
    let out = Command::new("adb")
        .args(["exec-out", "run-as", PACKAGE, "cat", CONFIG])
        .output()
        .expect("adb");
    let config = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success()
            && platform::unlock_policy(&config) == Ok(KeyUnlockPolicy::UserPresence),
        "the app's profile is not user-presence (gate-b's setup): {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The embedded profile's name, as the app's paths and aliases carry it.
const PROFILE: &str = "human-android";

/// The identity record under the app's data directory, as `run-as`
/// opens it (`ProfilePaths::resolve_embedded`'s `data` role and
/// `custody::CUSTODY_FILE`).
const RECORD: &str = "interweave/data/profiles/human-android/identity.iwk1";

/// `E2eCase`'s reading of the app's Keystore aliases
/// (`E2eCase.kt` `KEY_INFO`): Kotlin's, so no Rust case names it.
const KEY_INFO: &str = "key-info";

/// The identity record's bytes now.
fn identity_record() -> Vec<u8> {
    let out = platform::adb()
        .args(["exec-out", "run-as", PACKAGE, "cat", RECORD])
        .output()
        .expect("adb");
    assert!(
        out.status.success() && !out.stdout.is_empty(),
        "the identity record at {RECORD}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// `key-info`'s entry for `alias`, `None` when the Keystore lists no
/// such alias.
fn seed_key(alias: &str) -> Option<serde_json::Value> {
    let instrumentation = std::env::var(adb::INSTRUMENTATION_ENV)
        .unwrap_or_else(|_| panic!("{} names the androidTest APK", adb::INSTRUMENTATION_ENV));
    let argv = adb::instrument_argv(
        &instrumentation,
        KEY_INFO,
        &serde_json::Value::Object(serde_json::Map::new()),
    );
    let out = platform::adb().args(&argv).output().expect("adb");
    let read = adb::parse(&String::from_utf8_lossy(&out.stdout));
    assert_eq!(
        read.get(keys::RESULT).and_then(serde_json::Value::as_str),
        Some(keys::PASS),
        "key-info: {read:?}"
    );
    let listed = read
        .get("keys")
        .and_then(serde_json::Value::as_str)
        .and_then(|t| serde_json::from_str::<Vec<serde_json::Value>>(t).ok())
        .unwrap_or_else(|| panic!("key-info listed no keys array: {read:?}"));
    listed
        .into_iter()
        .find(|k| k.get("alias").and_then(serde_json::Value::as_str) == Some(alias))
}

/// Says, if the case ends with the phone's lock removed, that the
/// person sets it again: the case removes nothing itself, so it can put
/// nothing back.
struct LockPutBack;

impl Drop for LockPutBack {
    fn drop(&mut self) {
        // Read without panicking: this runs in a failing case's unwind.
        let lock_set = platform::adb()
            .arg("shell")
            .args(platform::dump_command(&["lock_settings"]))
            .output()
            .ok()
            .and_then(|o| platform::dump_ended(&String::from_utf8_lossy(&o.stdout)).ok())
            .and_then(|d| platform::screen_lock_set(&d).ok());
        if lock_set == Some(false) {
            // `eprintln!` panics when stderr is gone, and a panic here
            // would abort the unwind.
            let _ = writeln!(
                std::io::stderr(),
                "PERSON: the phone has no screen lock now: set it again"
            );
        }
    }
}

fn until_within(what: &str, patience: Duration, done: impl Fn() -> bool) {
    let deadline = Instant::now() + patience;
    while !done() {
        assert!(Instant::now() < deadline, "{what}: not within {patience:?}");
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn passed(step: &Step, s: &Screens) {
    assert_eq!(
        step.get(keys::RESULT).and_then(|v| v.as_str()),
        Some(keys::PASS),
        "{step:?}\nthe trace: {:#?}\nthe app's other lines: {:#?}",
        s.trace(),
        s.app_log()
    );
}

fn until(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !done() {
        assert!(Instant::now() < deadline, "{what}: not within {PATIENCE:?}");
        std::thread::sleep(Duration::from_millis(500));
    }
}
