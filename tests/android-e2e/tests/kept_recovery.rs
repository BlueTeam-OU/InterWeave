// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A K1 that a refused start kept while Show my phrase's S1 showed, on a
//! device through the app's own screens (`src/screen.rs`, the debug
//! build's trace), carried from #615's review (F3 partly, F4): S1's Back
//! before the service was taken offline starts nothing, so its end raises
//! K1 at once (`ready_check::at_flow_end`'s `Raise`), and K1 begins in the
//! same turn of the event loop as S1's end, before the frame whose off S1
//! armed. That off finds K1 showing and asks nothing
//! (`onboard::secure_off`), so the window is `SECURE` from K1's rise and
//! stays so while K1 shows.
//!
//! The refused start is the debug build's hooks (`DebugHooks.kt`):
//! `REFUSE_IDENTITY` arms one `RecoveryRequired`, as for a key the
//! platform invalidated, the record left as it is, and `RESTART` runs the
//! start a platform restart runs, in the running process. The case ends
//! by a launch, which force-stops the app: the kept K1 is the process's,
//! so the phone is left on Chats as it was found. No person.
//!
//! What this does NOT prove: K1's own end clearing the flag (K1 ends by a
//! restore, which needs the phrase, or by Back, which finishes the
//! Activity); a K1 kept while S1 shows whose end asks a start (`Keep`:
//! the words page under a user-presence profile, a person's prompt); a
//! K1 raised over an open Show my address, whose end leaves the flag to
//! Show's own off (`ready_check::secure_may_clear`, host-tested), again
//! ended only by a restore; and the frames between the trace line and the
//! window read, as `identity_lines.rs` says.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use interweave_android_e2e_cases::keys;
use interweave_android_e2e_tests::adb::PACKAGE;
use interweave_android_e2e_tests::platform;
use interweave_android_e2e_tests::screen::{Screens, Step};

/// How long a screen's answer may take over the phone's own runtime.
const PATIENCE: Duration = Duration::from_secs(30);

/// How long K1 is watched for the flag going off.
const QUIET: Duration = Duration::from_secs(8);

/// The debug build's hooks (`DebugHooks.kt`).
const REFUSE_IDENTITY: &str = "org.interweave.human.debug.REFUSE_IDENTITY";
const RESTART: &str = "org.interweave.human.debug.RESTART";

#[test]
#[ignore = "a phone with the debug APK and an identity, ANDROID_SERIAL naming it"]
fn k1_kept_while_s1_shows_rises_secured_at_its_back() {
    let mut s =
        Screens::new(&std::env::var("ANDROID_SERIAL").expect("ANDROID_SERIAL names the phone"));
    passed(&s.launch(PATIENCE), &s);
    passed(&s.open_trust(PATIENCE), &s);
    passed(&s.open_phrase(PATIENCE), &s);

    hook(REFUSE_IDENTITY);
    app_line(&mut s, "identity hook: the next start answers", 0);
    let from = s.app_log().len();
    hook(RESTART);
    app_line(
        &mut s,
        "restart hook: restarting as the platform would",
        from,
    );
    // The refused start's K1, held back because S1 shows: the positive
    // control that the case is the one it names, not a K1 raised at once.
    app_line(&mut s, "no identity: RecoveryRequired", from);
    app_line(&mut s, "K1 kept: an identity flow shows", from);
    let after = s.mark();
    passed(
        &s.none_after(after, "identity=", Duration::from_secs(2)),
        &s,
    );
    assert!(
        !secure(),
        "S1 before the words: no SECURE window {:?}",
        platform::windows()
    );

    let from = s.app_log().len();
    passed(
        &s.identity_back(&["identity=closed", "identity=k1"], PATIENCE),
        &s,
    );
    let raised = s.mark();
    // S1's off ran after K1 began, and asked nothing.
    app_line(&mut s, "an identity flow shows: FLAG_SECURE stays on", from);
    assert!(
        secure(),
        "K1 raised: a SECURE window {:?}",
        platform::windows()
    );
    let deadline = Instant::now() + QUIET;
    while Instant::now() < deadline {
        assert!(
            secure(),
            "K1 still showing: the flag went off {:?}",
            platform::windows()
        );
        std::thread::sleep(Duration::from_millis(500));
    }
    passed(&s.none_after(raised, "identity=", Duration::ZERO), &s);

    // The kept K1 goes with the process: the next launch reads the record,
    // which the hook left as it was, and settles on Chats.
    passed(&s.launch(PATIENCE), &s);
    assert!(
        !secure(),
        "Chats again: no SECURE window {:?}",
        platform::windows()
    );
}

/// Fire one of the debug build's hooks; the Service hears it while it
/// runs.
fn hook(action: &str) {
    platform::shell(&["am", "broadcast", "-a", action, "-p", PACKAGE]);
}

/// Wait for an app line (not the trace) containing `needle` among those
/// from index `from` on.
fn app_line(s: &mut Screens, needle: &str, from: usize) {
    let deadline = Instant::now() + PATIENCE;
    loop {
        assert!(
            Instant::now() < deadline,
            "{needle:?} not said within {PATIENCE:?}: {:#?}",
            s.app_log()
        );
        let at = s.mark();
        let _ = s.lines_after(at, &[], Duration::ZERO);
        if s.app_log().iter().skip(from).any(|l| l.contains(needle)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn secure() -> bool {
    platform::windows().iter().any(|(_, s)| *s)
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
