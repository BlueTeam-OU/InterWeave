// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The profile's identity at a start (plan section 20 step 6): unwrapped
//! from its Keystore-wrapped record by the embedded custody path, through
//! the platform's cipher ([`SeedCipher`], the app's Keystore adapter) --
//! or why there is none, which the Service answers as a refusal to start
//! (android-key-custody.md, "Failure behavior").
//!
//! A refusal of the record never leads to a new identity: a key the
//! platform invalidated, a record that does not open or a `PeerId` that
//! does not match is [`IdentityRefused::RecoveryRequired`], where the app's
//! recovery flow (step 7) takes over, and the record is left as it was.
//!
//! A profile with no record at all is a first run. Creating its identity
//! or restoring one from a phrase is the person's choice in step 7's
//! onboarding, which does not exist yet; until it does, the debug build
//! (`dev-stand-ins`) creates one and provisions it under the profile's
//! configured unlock policy, so the debug app keeps one `PeerId` across
//! restarts, and a build without the stand-ins answers
//! [`IdentityRefused::Unprovisioned`].

use std::path::Path;

use interweave_profile_config::{ProfilePaths, TrustBoundary};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_embedded::custody::{self, RecoveryCause, SeedCipher, UnlockRefused};

/// Why a start has no identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityRefused {
    /// The profile's paths could not be resolved under the app data
    /// directory.
    Paths(String),
    /// No record, and this build creates none: step 7's onboarding is
    /// where the person creates or restores one.
    Unprovisioned,
    /// The record exists and cannot give its identity back: the app
    /// enters recovery. The record is untouched.
    RecoveryRequired(RecoveryCause),
    /// The key needs the person to authenticate: stay offline until they
    /// do.
    UserNotAuthenticated,
    /// The cipher or the storage failed for a reason that says nothing
    /// about the record: try again, decide nothing.
    Unavailable(String),
    /// The first run's provisioning wrote nothing, and why.
    Provisioning(String),
}

/// The identity `profile` starts with, under `app_data_dir`.
///
/// # Errors
/// [`IdentityRefused`].
pub fn start_identity(
    app_data_dir: &Path,
    profile: &str,
    cipher: &dyn SeedCipher,
) -> Result<ProfileIdentity, IdentityRefused> {
    let paths = TrustBoundary::new(app_data_dir)
        .and_then(|boundary| ProfilePaths::resolve_embedded(profile, boundary))
        .map_err(|e| IdentityRefused::Paths(e.to_string()))?;
    match custody::unlock(&paths, cipher) {
        Ok(identity) => Ok(identity),
        Err(UnlockRefused::Unprovisioned) => first_run(&paths, cipher),
        Err(UnlockRefused::RecoveryRequired(cause)) => {
            Err(IdentityRefused::RecoveryRequired(cause))
        }
        Err(UnlockRefused::UserNotAuthenticated) => Err(IdentityRefused::UserNotAuthenticated),
        Err(UnlockRefused::Unavailable(why)) => Err(IdentityRefused::Unavailable(why)),
    }
}

/// A profile with no record: the debug build's stand-in for step 7's
/// onboarding creates its identity and provisions it.
#[cfg(feature = "dev-stand-ins")]
fn first_run(
    paths: &ProfilePaths,
    cipher: &dyn SeedCipher,
) -> Result<ProfileIdentity, IdentityRefused> {
    let policy = interweave_profile_config::ProfileConfig::load(paths)
        .map_err(|e| IdentityRefused::Provisioning(format!("the profile: {e}")))?
        .runtime
        .android
        .key_unlock_policy;
    let identity = ProfileIdentity::generate();
    custody::provision(paths, cipher, &identity, policy)
        .map_err(|e| IdentityRefused::Provisioning(format!("{e:?}")))?;
    Ok(identity)
}

/// A profile with no record, in a build that creates none.
#[cfg(not(feature = "dev-stand-ins"))]
fn first_run(
    _paths: &ProfilePaths,
    _cipher: &dyn SeedCipher,
) -> Result<ProfileIdentity, IdentityRefused> {
    Err(IdentityRefused::Unprovisioned)
}
