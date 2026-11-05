// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! STAND-INS for what the app runs on in its debug build alone, compiled
//! under the `dev-stand-ins` feature, which the app turns on there: the
//! android-e2e cases' replacement of the profile ([`provision_replacing`]).
//!
//! The first-run profile is no stand-in: every build writes profile-config's
//! ([`crate::provision_profile`]). Nor is the identity: it is
//! Keystore-wrapped through the embedded custody path (step 6), and what a
//! first run does is the caller's [`crate::FirstRun`].

use std::path::Path;

use interweave_profile_config::{
    PersistError, ProfilePaths, TrustBoundary, create_private_dir_within,
    write_private_atomic_within,
};

/// Write `document` as `profile`'s configuration under `app_data_dir`,
/// replacing what is there: what the android-e2e cases' `provision`
/// asks of the app, since the host cannot reach the app's private
/// directories (the seam agreed with p2p-network-dev-02, 01a127b3).
/// The app's instrumentation calls it as the cases' `provision`
/// (apps/human-android/src/e2e.rs). The app's own start keeps
/// [`crate::provision_profile`]'s never-rewrite rule
/// (`the_cases_provisioning_replaces_the_profile_where_the_apps_keeps_it`).
///
/// # Errors
/// The directory or the file could not be made, or is refused by the
/// app's trust boundary.
pub fn provision_replacing(
    app_data_dir: &Path,
    profile: &str,
    document: &str,
) -> Result<(), PersistError> {
    let paths = ProfilePaths::resolve_embedded(profile, TrustBoundary::new(app_data_dir)?)?;
    write_config(&paths, document)
}

/// `document` as the configuration file, owner-only whatever the
/// process's umask and whatever mode a file already there had: the app's
/// start refuses a configuration another account can change, and the
/// instrumentation's process, which calls [`provision_replacing`], once
/// wrote one at 0666 on the A40 (2026-11-04, j89), so the next start
/// answered -3 (`a_provisioned_profile_is_owner_only_and_stays_so`).
fn write_config(paths: &ProfilePaths, document: &str) -> Result<(), PersistError> {
    create_private_dir_within(paths.config_dir(), paths.boundary())?;
    write_private_atomic_within(&paths.config_file(), document.as_bytes(), paths.boundary())
}
