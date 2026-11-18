// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//
// What every test binary of this crate shares.
#![allow(dead_code, clippy::expect_used)]

use std::path::Path;
use std::time::{Duration, Instant};

/// A temporary directory made owner-only AT CREATION, whatever the umask.
///
/// `tempfile` creates with the process umask, so under umask 002 with a
/// shared primary group a bare `tempfile::tempdir()` is group-writable,
/// and the store's ancestor rule (ADR-0028 A 2026-10-08) refuses it: the
/// test fails at its setup, not at what it tests. Under umask 022 a bare
/// tempdir is 0755, which the rule accepts, so nothing else would notice
/// the mode going missing: the helper checks its own result, and every
/// test that uses it fails if it is not 0700.
pub(crate) fn private_tempdir() -> tempfile::TempDir {
    private_tempdir_in(std::env::temp_dir())
}

/// [`private_tempdir`] under `dir`.
pub(crate) fn private_tempdir_in(dir: impl AsRef<Path>) -> tempfile::TempDir {
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    let temp = builder.tempdir_in(dir).expect("tempdir");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(temp.path())
            .expect("stat")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "a private tempdir is owner-only");
    }
    temp
}

/// How long a test waits for what it awaits before it fails. Far past
/// what any wait here takes on a loaded host: it exists only so a lost
/// event cannot hang the suite, never to time the client. A wait whose
/// bound sat near its real cost failed under the workspace suite's load
/// and passed alone -- a 10 s wait over two turns of 4 to 6 s each in a
/// debug build (j296) -- so the test said "timed out" about a client that
/// had done nothing wrong.
pub(crate) const WAIT_BOUND: Duration = Duration::from_secs(120);

/// A wait for one thing, ended by the caller's loop once it holds; it
/// fails the test when [`WAIT_BOUND`] runs out first, naming the wait
/// and the bound (`running_out_fails_naming_the_wait_and_its_bound`).
pub(crate) struct Wait {
    what: String,
    bound: Duration,
    deadline: Instant,
}

impl Wait {
    pub(crate) fn on(what: impl Into<String>) -> Self {
        Self::within(WAIT_BOUND, what)
    }

    fn within(bound: Duration, what: impl Into<String>) -> Self {
        Self {
            what: what.into(),
            bound,
            deadline: Instant::now() + bound,
        }
    }

    /// # Panics
    /// When the bound has run out.
    pub(crate) fn check(&self) {
        self.check_showing(String::new);
    }

    /// [`Wait::check`], whose failure also says `detail` -- what the
    /// test saw, which the assertions after the wait would have shown.
    ///
    /// # Panics
    /// When the bound has run out.
    pub(crate) fn check_showing(&self, detail: impl FnOnce() -> String) {
        if Instant::now() >= self.deadline {
            let detail = detail();
            let sep = if detail.is_empty() { "" } else { "; " };
            panic!(
                "timed out waiting for {} within {:?}{sep}{detail}",
                self.what, self.bound
            );
        }
    }
}

#[test]
#[should_panic(expected = "timed out waiting for the far end within 0ns; it said nothing")]
fn running_out_fails_naming_the_wait_and_its_bound() {
    Wait::within(Duration::ZERO, "the far end").check_showing(|| "it said nothing".to_owned());
}

#[test]
fn a_wait_inside_its_bound_passes() {
    Wait::on("anything").check();
}
