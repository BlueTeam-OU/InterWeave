// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The screen driving (`src/screen.rs`) on one phone, through the steps
//! that write nothing: launch, Peers, Show drawn and closed, the add page
//! opened and left. The two-phone row that adds and trusts through the
//! screens is `pairing.rs`'s; this is the check that the driver reaches
//! every control it needs on the build at hand, without touching the
//! profile.
//!
//! `#[ignore]`d: run with `--ignored` against the debug APK (the trace is
//! the `ui-trace` feature's), `ANDROID_SERIAL` naming the phone.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_android_e2e_cases::keys;
use interweave_android_e2e_tests::screen::{Screens, Step};

/// How long one screen's answer may take: Peers reads two lists and Show
/// one, over the phone's own runtime.
const PATIENCE: Duration = Duration::from_secs(30);

fn passed(s: &Step, trace: &[String]) {
    assert_eq!(
        s.get(keys::RESULT).and_then(|v| v.as_str()),
        Some(keys::PASS),
        "{s:?}\nthe trace: {trace:#?}"
    );
}

#[test]
#[ignore = "a phone with the debug APK, ANDROID_SERIAL naming it"]
fn the_driver_reaches_show_and_the_add_page_without_writing() {
    let serial = std::env::var("ANDROID_SERIAL").expect("ANDROID_SERIAL names the phone");
    let mut screens = Screens::new(&serial);
    let s = screens.launch(PATIENCE);
    passed(&s, screens.trace());
    let s = screens.open_peers(PATIENCE);
    passed(&s, screens.trace());
    let s = screens.show(PATIENCE);
    passed(&s, screens.trace());
    let s = screens.open_add(PATIENCE);
    passed(&s, screens.trace());
    let s = screens.leave_add(PATIENCE);
    passed(&s, screens.trace());
    println!("{:#?}", screens.trace());
}
