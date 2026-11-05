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
//! a PIN; and Keystore invalidation on a lock-screen change, which needs
//! the person's lock settings and leaves the profile in recovery.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::process::Command;
use std::time::{Duration, Instant};

use interweave_android_e2e_cases::keys;
use interweave_android_e2e_tests::adb::PACKAGE;
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
