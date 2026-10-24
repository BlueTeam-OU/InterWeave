// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! STAND-INS for what the app runs on before the parts that own them
//! land, compiled under the `dev-stand-ins` feature, which the app turns
//! on for its debug build:
//!
//! - the first-run profile: profile-config's own provisioning
//!   (`provision_embedded`, p2p-network-dev-01's, agreed in relay
//!   01a12499-ee78) replaces [`provision`];
//!
//! A stand-in profile trusts nobody. The identity is no stand-in: it is
//! Keystore-wrapped through the embedded custody path (step 6), and what
//! a first run does is the caller's [`crate::FirstRun`].

use std::path::Path;

use interweave_profile_config::{
    PersistError, ProfilePaths, TrustBoundary, create_private_dir_within,
};

/// The profile's document: the embedded deployment, wildcard listeners,
/// the human endpoint open to this client's kind and kept out of the
/// endpoint directory, no IPC, and no trusted peer, relay or AutoNAT
/// server -- those name infrastructure a default cannot know.
///
/// The endpoint is not advertised because no control in the app changes
/// it, and `human-client-ui.md` §13 asks every identity's endpoint to
/// have advertise off (`the_stand_in_profile_advertises_no_endpoint`). A
/// profile already written keeps what it says: [`provision`] never
/// rewrites one, and ADR-0028's overlay is what changes it.
fn document(profile: &str) -> String {
    format!(
        "schema_version: 2
profile:
  name: {profile}
runtime:
  deployment: embedded-android
ipc:
  enabled: false
transport:
  listen:
    addresses: [\"/ip4/0.0.0.0/tcp/0\"]
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  registration_policy: configured-only
  default_direct_endpoint: human
  entries:
    - id: human
      enabled: true
      advertise: false
      allowed_client_kinds: [human-client]
      inbound: inherit_profile_trust
      outbound: inherit_profile_trust
channels:
  desired: []
"
    )
}

/// Write `profile`'s configuration under `app_data_dir` if there is none:
/// a profile once written is never rewritten, so what a person changed
/// survives.
///
/// # Errors
/// The directory or the file could not be made, or is refused by the
/// app's trust boundary.
pub fn provision(app_data_dir: &Path, profile: &str) -> Result<(), PersistError> {
    let paths = ProfilePaths::resolve_embedded(profile, TrustBoundary::new(app_data_dir)?)?;
    if paths.config_file().exists() {
        return Ok(());
    }
    create_private_dir_within(paths.config_dir(), paths.boundary())?;
    std::fs::write(paths.config_file(), document(profile)).map_err(PersistError::Io)
}

/// Write `document` as `profile`'s configuration under `app_data_dir`,
/// replacing what is there: what the android-e2e cases' `provision`
/// asks of the app, since the host cannot reach the app's private
/// directories (the seam agreed with p2p-network-dev-02, 01a127b3).
/// The app's instrumentation calls it as the cases' `provision`
/// (apps/human-android/src/e2e.rs). The app's own start keeps
/// [`provision`]'s never-rewrite rule.
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
    create_private_dir_within(paths.config_dir(), paths.boundary())?;
    std::fs::write(paths.config_file(), document).map_err(PersistError::Io)
}
