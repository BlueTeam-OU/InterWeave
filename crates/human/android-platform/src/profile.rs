// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The profile document a fresh install starts from, written in every
//! build: profile-config's embedded-android first run
//! (`provision_embedded`, the seam agreed with p2p-network-dev-01 in
//! relay 01a12499-ee78).
//!
//! The release build once wrote none -- the debug build's own stand-in
//! did, behind `dev-stand-ins` -- so a release first run found no
//! document and its onboarding could not seal an identity (j96, v0.1.0
//! to v0.2.0). One writer for both builds is what keeps the debug runs
//! evidence for the release (`a_release_first_run_writes_the_profile_onboarding_seals_under`).

use std::path::Path;

use interweave_profile_config::provision::provision_embedded;
use interweave_profile_config::{PersistError, ProfilePaths, TrustBoundary};

/// Write `profile`'s configuration under `app_data_dir` if there is none.
/// A profile once written is never rewritten, so what a person or the
/// admin overlay changed survives: the write is exclusive, and a document
/// already there is success.
///
/// # Errors
/// The paths are refused by the app's trust boundary, or the directory
/// or the file could not be made.
pub fn provision_profile(app_data_dir: &Path, profile: &str) -> Result<(), PersistError> {
    let paths = ProfilePaths::resolve_embedded(profile, TrustBoundary::new(app_data_dir)?)?;
    match provision_embedded(&paths) {
        Ok(()) | Err(PersistError::AlreadyExists) => Ok(()),
        Err(e) => Err(e),
    }
}
