// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The profile document a fresh install starts from, form 1 of the two
//! (no identity; the first identity's seal moves it to form 2), written in every
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

/// Write `profile`'s configuration under `app_data_dir` as form 1 (no
/// identity yet) if there is none (M1), and replace an earlier release's
/// first-run document with form 1 (M2), before anything loads it: the
/// earlier document names its entry `- id: human`, which the loader now
/// refuses (ADR-0067; ENDPOINTS.md, the two forms). Every start and the
/// Activity's identity check call it first, so M2 runs before the
/// reseal's and the log level's loads
/// (`an_earlier_releases_document_is_replaced_once_by_form_1`).
///
/// Any other document -- form 1, form 2, one without the marker, one a
/// person changed -- is kept without a write
/// (`the_first_run_profile_is_written_once_and_never_rewritten`). A
/// full or read-only data directory must not refuse a phone already
/// provisioned (`a_provisioned_profile_is_kept_without_a_write`):
/// `provision_embedded` reads the present document before it writes
/// anything (profile-config c63a9db5b).
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
