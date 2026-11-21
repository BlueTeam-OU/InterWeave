// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The embedded-android profile a fresh install starts from (plan §20
//! steps 2-4, the seam agreed with `rust-ui-dev`, `GZCoord` seq 95202),
//! in its two forms (architect-cto's ruling for j185, ADR-0067).
//!
//! `EmbeddedHost::start` loads `config.yaml` from under the host's
//! configuration root, and on a fresh install nothing has written it.
//! The app calls [`provision_embedded`] before every start, through
//! `android-platform`'s `provision_profile`, which writes nothing when
//! `config.yaml` is already there and takes `AlreadyExists` as done; the
//! write's own refusal is what keeps a start that passes that check
//! beside another from replacing the profile the other wrote.
//!
//! # The two forms
//!
//! Both open with [`EMBEDDED_DOCUMENT_MARKER`], and both are the minimum
//! a host starts on with the rest the schema's defaults: the
//! embedded-android deployment, a WILDCARD listener (the embedded-android
//! rule, `ConfigError::AndroidListenerNotWildcard`), the peer cache as
//! its only discovery, IPC off, and NO trust entries, relays or AutoNAT
//! servers -- those name operator infrastructure a default cannot know,
//! and arrive through the admin trust surface or onboarding.
//!
//! FORM 1, NO IDENTITY YET ([`embedded_document`]): no endpoint entry, no
//! `runtime.android.endpoint` and no `default_direct_endpoint`. An
//! endpoint is named by the id its own key derives (ADR-0067), so no
//! document can name one before onboarding has minted and sealed that
//! key. Form 1 LOADS -- the Android lease rule binds a start, not a load
//! -- and a start on it is refused `ConfigError::AndroidNoIdentity`.
//!
//! FORM 2, FIRST IDENTITY SEALED ([`embedded_identity_document`]): form 1
//! plus the one entry `key: <id>.key` aliased `human`, the service's
//! `runtime.android.endpoint: <id>`, and `default_direct_endpoint: human`.
//! [`provision_identity`] moves form 1 to form 2 once the caller has
//! sealed the key; it never writes a key itself.
//!
//! The identity's endpoint is not advertised -- `human-client-ui.md` §13
//! asks every identity's endpoint to have advertise off with no control
//! that changes it -- and is leased by the human client's kind alone.
//!
//! # An earlier release's document
//!
//! Releases before ADR-0067's key-named entries wrote one document that
//! named its endpoint `- id: human`, which a load now refuses
//! `EndpointIdNotDerived`. [`provision_embedded`] replaces such a
//! document whole with form 1 when it carries the marker -- the app wrote
//! it, and nothing in it is the person's, since an overlay never rewrites
//! `config.yaml` -- and leaves every document without the marker alone:
//! the person authored it, and its load refuses as on the desktop.

use interweave_transport_api::EndpointId;

use crate::paths::ProfilePaths;
use crate::persist::{create_private_exclusive_within, write_private_atomic_within};
use crate::private_read::{PrivateReadError, read_private_within};
use crate::{MAX_PROFILE_BYTES, PersistError, ProfileConfig};

/// The text both forms open with. `apps/human-android`'s
/// `verifyReleaseNative` fails the release build when the native library
/// lacks it, as its evidence that the release carries the first-run
/// writer: a change here is a change there, in the same change.
pub const EMBEDDED_DOCUMENT_MARKER: &str = "# Written on first run by the Android client";

/// The alias form 2 gives the first identity's entry, and the alias its
/// `default_direct_endpoint` names.
pub const FIRST_IDENTITY_ALIAS: &str = "human";

/// The document opened by the marker, up to its endpoints: what both
/// forms share. `android` is the `runtime.android` block, empty in form 1.
fn document_head(profile: &str, android: &str) -> String {
    format!(
        "{EMBEDDED_DOCUMENT_MARKER}: the embedded-android
# defaults. Trust, relays and AutoNAT servers arrive through the admin
# trust surface or onboarding, not here.
schema_version: 2
profile:
  name: {profile}
runtime:
  deployment: embedded-android
{android}ipc:
  enabled: false
transport:
  listen:
    addresses: [\"/ip4/0.0.0.0/tcp/0\"]
trust:
  policy: static-allowlist
  allowed_peers: []
"
    )
}

/// The discovery block both forms end with.
const DISCOVERY: &str = "discovery:
  providers:
    - type: peer-cache
      enabled: true
      priority: 10
      config: {}
";

/// FORM 1, the document [`provision_embedded`] writes for `profile`: no
/// endpoint, no `runtime.android.endpoint`, no default. It opens with
/// [`EMBEDDED_DOCUMENT_MARKER`].
#[must_use]
pub fn embedded_document(profile: &str) -> String {
    format!(
        "{}endpoints:
  entries: []
{DISCOVERY}",
        document_head(profile, "")
    )
}

/// FORM 2, the document [`provision_identity`] writes for `profile` once
/// `endpoint`'s key is sealed: form 1 plus that one entry, aliased
/// [`FIRST_IDENTITY_ALIAS`], as the service's endpoint and the default.
#[must_use]
pub fn embedded_identity_document(profile: &str, endpoint: &EndpointId) -> String {
    let id = endpoint.as_str();
    format!(
        "{}endpoints:
  default_direct_endpoint: {FIRST_IDENTITY_ALIAS}
  entries:
    - key: {id}.key
      alias: {FIRST_IDENTITY_ALIAS}
      enabled: true
      advertise: false
      allowed_client_kinds: [human-client]
{DISCOVERY}",
        document_head(profile, &format!("  android:\n    endpoint: {id}\n"))
    )
}

/// Whether `text` is an earlier release's first-run document: it opens
/// with the marker AND names an entry by the retired `id` member, as
/// every release before ADR-0067 wrote it (`- id: human`). Neither alone
/// is enough: without the marker the person wrote it, and a marker
/// document without `- id:` is form 1 or form 2.
fn is_retired_embedded_document(text: &str) -> bool {
    text.starts_with(EMBEDDED_DOCUMENT_MARKER)
        && text
            .lines()
            .any(|line| line.trim_start().starts_with("- id:"))
}

/// Write form 1 for `paths`' profile at its `config.yaml`, owner-only,
/// under the trust boundary the paths carry, when there is none; replace
/// an earlier release's first-run document with it; and otherwise
/// refuse, leaving what is there as it was.
///
/// - ABSENT: written in the same filesystem operation that refuses a
///   file already there (`create_private_exclusive_within`), so a second
///   first run never replaces what the first wrote
///   (`tests/provision.rs`,
///   `a_first_run_writes_form_1_and_a_second_is_refused`).
/// - AN EARLIER RELEASE'S DOCUMENT -- the marker and an entry by the
///   retired `- id:` member -- is replaced whole by form 1, atomically
///   and owner-only (`write_private_atomic_within`). No key file is read
///   (`an_earlier_releases_document_is_replaced_by_form_1`).
/// - ANYTHING ELSE -- form 1, form 2, a document without the marker, one
///   that is not private or past the size bound -- is
///   [`PersistError::AlreadyExists`] and is not touched
///   (`a_document_without_the_marker_is_never_replaced`).
///
/// # Errors
/// [`PersistError::AlreadyExists`] as above; otherwise what the write or
/// the read of the present document meets.
pub fn provision_embedded(paths: &ProfilePaths) -> Result<(), PersistError> {
    let form_1 = embedded_document(paths.profile());
    match create_private_exclusive_within(&paths.config_file(), form_1.as_bytes(), paths.boundary())
    {
        Err(PersistError::AlreadyExists) => {}
        other => return other,
    }
    let present =
        match read_private_within(&paths.config_file(), paths.boundary(), MAX_PROFILE_BYTES) {
            Ok(Some(text)) => text,
            // Removed since the exclusive create found it: the next start
            // writes form 1.
            Ok(None) => return Err(PersistError::AlreadyExists),
            // Not the app's own owner-only document: not the app's to
            // replace, and its load refuses it by its own rule.
            Err(PrivateReadError::NotPrivate { .. } | PrivateReadError::TooLarge) => {
                return Err(PersistError::AlreadyExists);
            }
            Err(PrivateReadError::Read(e)) => return Err(PersistError::Io(e)),
        };
    if !is_retired_embedded_document(&present) {
        return Err(PersistError::AlreadyExists);
    }
    write_private_atomic_within(&paths.config_file(), form_1.as_bytes(), paths.boundary())
}

/// Why [`provision_identity`] wrote nothing.
#[derive(Debug)]
pub enum ProvisionIdentityError {
    /// No `config.yaml`: [`provision_embedded`] writes form 1 first.
    Absent,
    /// The document does not open with [`EMBEDDED_DOCUMENT_MARKER`]: the
    /// person wrote it, and it is never replaced.
    NotEmbedded,
    /// The document is form 2 for another endpoint: the profile already
    /// has its first identity.
    OtherIdentity {
        /// The endpoint the document's `runtime.android.endpoint` names.
        found: EndpointId,
    },
    /// The document carries the marker and is neither form 1 nor form 2
    /// for this profile -- an earlier release's, which
    /// [`provision_embedded`] replaces first, or one changed since it was
    /// written.
    Unrecognised,
    /// The document could not be read: not private, past
    /// [`MAX_PROFILE_BYTES`], or a read failure.
    Read(PrivateReadError),
    /// Form 2 could not be written; a failure before the rename leaves
    /// form 1 in place.
    Write(PersistError),
}

impl core::fmt::Display for ProvisionIdentityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Absent => f.write_str("the profile has no config.yaml to give an identity"),
            Self::NotEmbedded => f.write_str(
                "config.yaml was not written by the Android client's first run; it is never replaced",
            ),
            Self::OtherIdentity { found } => write!(
                f,
                "the profile already has its first identity, {}",
                found.as_str()
            ),
            Self::Unrecognised => f.write_str(
                "config.yaml carries the first-run marker but is neither of its forms for this profile",
            ),
            Self::Read(e) => write!(f, "config.yaml {e}"),
            Self::Write(e) => write!(f, "config.yaml cannot be written: {e}"),
        }
    }
}

impl core::error::Error for ProvisionIdentityError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Read(e) => Some(e),
            Self::Write(e) => Some(e),
            _ => None,
        }
    }
}

/// Replace `paths`' form-1 document whole with form 2 for `endpoint`,
/// atomically and owner-only under the paths' trust boundary. Form 2
/// for `endpoint` already there is success with nothing written, so a
/// retried onboarding step is idempotent.
///
/// NEVER WRITES A KEY: the caller minted and sealed `endpoint`'s key
/// before this, and the document names it by the id that key derives.
/// Form 1 and form 2 are recognised by their exact text for `paths`'
/// profile, so a document changed since it was written is refused
/// rather than overwritten.
///
/// # Errors
/// [`ProvisionIdentityError`], nothing written.
pub fn provision_identity(
    paths: &ProfilePaths,
    endpoint: &EndpointId,
) -> Result<(), ProvisionIdentityError> {
    let text = read_private_within(&paths.config_file(), paths.boundary(), MAX_PROFILE_BYTES)
        .map_err(ProvisionIdentityError::Read)?
        .ok_or(ProvisionIdentityError::Absent)?;
    if !text.starts_with(EMBEDDED_DOCUMENT_MARKER) {
        return Err(ProvisionIdentityError::NotEmbedded);
    }
    let form_2 = embedded_identity_document(paths.profile(), endpoint);
    if text == form_2 {
        return Ok(());
    }
    if text != embedded_document(paths.profile()) {
        return Err(
            match ProfileConfig::parse_yaml(&text)
                .ok()
                .and_then(|config| config.runtime.android.endpoint)
            {
                Some(found) if &found != endpoint => {
                    ProvisionIdentityError::OtherIdentity { found }
                }
                _ => ProvisionIdentityError::Unrecognised,
            },
        );
    }
    write_private_atomic_within(&paths.config_file(), form_2.as_bytes(), paths.boundary())
        .map_err(ProvisionIdentityError::Write)
}
