// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Start-up, in the order that keeps a refusal cheap and the store safe:
//! the command line, the platform, the profile, the single-instance lock,
//! the store -- and only then the app's window. A refusal a person can
//! act on is shown on the refusal screen as well as said on stderr.
//!
//! Exit codes are sysexits(3)'s, so a launcher or a test can tell why.

use std::process::ExitCode;
use std::time::Duration;

use interweave_human_ui_slint::{PlatformProblem, platform_check};
use interweave_profile_config::{HumanClientLock, PersistError, XdgRoots};

use crate::refusal::{self, Refusal, Retry};
use crate::startup::{Opened, StartError, open_store, parse_args, resolve};
use crate::window;

/// The command line was wrong.
pub const EX_USAGE: u8 = 64;
/// The store needs recovery.
pub const EX_DATAERR: u8 = 65;
/// A platform service is missing: fontconfig, a window, or locking on
/// this platform.
pub const EX_UNAVAILABLE: u8 = 69;
/// The store or the single-instance lock could not be opened now: an I/O
/// failure that may pass.
pub const EX_IOERR: u8 = 74;
/// Another window holds this profile's store.
pub const EX_TEMPFAIL: u8 = 75;
/// The store, the lock, or a directory they live in is not private to
/// this user.
pub const EX_NOPERM: u8 = 77;
/// The profile's directories or configuration are wrong.
pub const EX_CONFIG: u8 = 78;

/// Run the client with `args` (the command line after the program name).
pub fn run(args: impl IntoIterator<Item = String>) -> ExitCode {
    let refuse = |code: u8, what: &dyn std::fmt::Display| {
        eprintln!("human-desktop: {what}");
        ExitCode::from(code)
    };
    // A refusal a person can act on: the stderr line, then the screen
    // where a window can be shown, then the exit class.
    let refused = |refusal: Refusal| {
        eprintln!("human-desktop: {refusal}");
        // Without Try again the screen can only close, with this class.
        let _ = refusal::show::<()>(&refusal, None);
        ExitCode::from(refusal.code())
    };

    let launch = match parse_args(args) {
        Ok(launch) => launch,
        Err(usage) => return refuse(EX_USAGE, &usage),
    };
    if let Err(PlatformProblem::NoFontconfig) = platform_check() {
        return refuse(
            EX_UNAVAILABLE,
            &"the fontconfig library could not be loaded: without it no text can be shown. \
              Install fontconfig and start again.",
        );
    }
    let profile = match XdgRoots::from_env()
        .map_err(StartError::Directories)
        .and_then(|roots| resolve(&launch, &roots))
    {
        Ok(profile) => profile,
        Err(e) => return refused(Refusal::Profile(e)),
    };
    // Before the store: two windows on one store would fight over the
    // endpoint lease and the rows. Held until the process ends, through
    // the refusal screen too, so a Try again opens the store under it.
    let _instance = match HumanClientLock::acquire(&profile.paths, Duration::ZERO) {
        Ok(lock) => lock,
        Err(PersistError::InstanceLocked { .. }) => return refused(Refusal::AlreadyOpen),
        Err(e @ PersistError::UnsupportedPlatform) => return refuse(EX_UNAVAILABLE, &e),
        Err(e) => return refused(Refusal::Lock(e)),
    };
    let store = match open_store(&profile, launch.store_options()) {
        Opened::Ready(store) => store,
        Opened::Blocked(blocked) => {
            let refusal = Refusal::Store(blocked);
            eprintln!("human-desktop: {refusal}");
            let (profile, options) = (profile.clone(), launch.store_options());
            let retry: Retry<_> = Box::new(move || match open_store(&profile, options) {
                Opened::Ready(store) => Ok(store),
                Opened::Blocked(blocked) => Err(Refusal::Store(blocked)),
            });
            match refusal::show(&refusal, Some(retry)) {
                Ok(store) => store,
                Err(code) => return ExitCode::from(code),
            }
        }
    };
    match window::run(&profile, store) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => refuse(
            EX_UNAVAILABLE,
            &format_args!("no window could be opened: {e}"),
        ),
    }
}
