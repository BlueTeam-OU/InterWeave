// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The embedded-android profile a fresh install starts from (plan §20
//! steps 2-4, the seam agreed with `rust-ui-dev`, `GZCoord` seq 95202).
//!
//! `EmbeddedHost::start` loads `config.yaml` from under the host's
//! configuration root, and on a fresh install nothing has written it.
//! The app calls [`provision_embedded`] before every start, through
//! `android-platform`'s `provision_profile`, which writes nothing when
//! `config.yaml` is already there and takes `AlreadyExists` as done; the
//! write's own refusal is what keeps a start that passes that check
//! beside another from replacing the profile the other wrote.
//!
//! The document is the minimum a host starts on, and the rest is the
//! schema's defaults: the embedded-android deployment with the `human`
//! endpoint it leases, a WILDCARD listener (the embedded-android rule,
//! `ConfigError::AndroidListenerNotWildcard`), the peer cache as its
//! only discovery, IPC off, and NO trust entries, relays or AutoNAT
//! servers -- those name operator infrastructure a default cannot know,
//! and arrive through the admin trust surface or onboarding. The
//! identity is not here: it is onboarding's (§20 step 6).
//!
//! The `human` endpoint is not advertised -- `human-client-ui.md` §13 asks
//! every identity's endpoint to have advertise off with no control that
//! changes it -- and is leased by the human client's kind alone. The
//! debug build's stand-in profile, which this document replaces, leased
//! it the same way, but advertised it until 2026-10-24 and carried
//! neither the `android:` block nor the peer-cache provider.

use crate::PersistError;
use crate::paths::ProfilePaths;
use crate::persist::create_private_exclusive_within;

/// The text [`embedded_document`] opens with. `apps/human-android`'s
/// `verifyReleaseNative` fails the release build when the native library
/// lacks it, as its evidence that the release carries the first-run
/// writer: a change here is a change there, in the same change.
pub const EMBEDDED_DOCUMENT_MARKER: &str = "# Written on first run by the Android client";

/// The document [`provision_embedded`] writes for `profile`; it opens
/// with [`EMBEDDED_DOCUMENT_MARKER`].
#[must_use]
pub fn embedded_document(profile: &str) -> String {
    format!(
        "{EMBEDDED_DOCUMENT_MARKER}: the embedded-android
# defaults. Trust, relays and AutoNAT servers arrive through the admin
# trust surface or onboarding, not here.
schema_version: 2
profile:
  name: {profile}
runtime:
  deployment: embedded-android
  android:
    endpoint: human
ipc:
  enabled: false
transport:
  listen:
    addresses: [\"/ip4/0.0.0.0/tcp/0\"]
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  default_direct_endpoint: human
  entries:
    - id: human
      enabled: true
      advertise: false
      allowed_client_kinds: [human-client]
discovery:
  providers:
    - type: peer-cache
      enabled: true
      priority: 10
      config: {{}}
"
    )
}

/// Write the embedded-android profile document for `paths`' profile at
/// its `config.yaml`, owner-only, under the trust boundary the paths
/// carry; REFUSED if one is already there, in the same filesystem
/// operation that installs it (`create_private_exclusive_within`), so a
/// second first run never replaces what the first wrote
/// (`tests/provision.rs`,
/// `a_first_run_writes_the_profile_and_a_second_is_refused`).
///
/// # Errors
/// [`PersistError::AlreadyExists`] when the profile is already
/// provisioned; otherwise what the write meets
/// (`create_private_exclusive_within`).
pub fn provision_embedded(paths: &ProfilePaths) -> Result<(), PersistError> {
    create_private_exclusive_within(
        &paths.config_file(),
        embedded_document(paths.profile()).as_bytes(),
        paths.boundary(),
    )
}
