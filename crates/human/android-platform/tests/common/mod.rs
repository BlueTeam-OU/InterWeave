// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The first run as far as the Service's start needs it, for the test
//! binaries that start one.

use std::path::Path;

use interweave_human_android_platform::provision_profile;
use interweave_profile_config::provision::provision_identity;
use interweave_profile_config::{ProfilePaths, TrustBoundary, endpoint_key};
use interweave_transport_api::EndpointId;

/// Form 1, then M3: the first identity's endpoint key minted and form 2
/// written naming it, whose id this answers. The embedded start refuses
/// form 1 as `AndroidNoIdentity` (the ruling of 2026-11-21), so a test
/// that starts the Service runs this where the app has run onboarding.
///
/// The key is minted at FILESYSTEM custody, as the embedded host's own
/// tests do (`crates/transport/embedded/tests/host.rs`,
/// `a_start_on_form_1_is_refused_no_identity`): the host has no
/// Keystore. The app seals the key at onboarding under the Keystore and
/// only then calls `provision_identity`; that order is not what these
/// tests prove.
pub(crate) fn provision_first_identity(app_data_dir: &Path, profile: &str) -> EndpointId {
    provision_profile(app_data_dir, profile).expect("form 1");
    let paths = ProfilePaths::resolve_embedded(
        profile,
        TrustBoundary::new(app_data_dir).expect("a boundary"),
    )
    .expect("paths");
    let id = endpoint_key::mint_filesystem(&paths).expect("the first identity's key");
    provision_identity(&paths, &id).expect("form 2");
    id
}
