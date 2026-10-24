// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Which of a profile's endpoints a desktop-hosted human client may lease:
//! one rule for every root that connects to a shared daemon -- the window
//! and the command line -- so the two never disagree on which identity a
//! profile offers them. Android's service leases the one endpoint its
//! profile names, and keeps its own rule
//! (`crates/human/android-platform/src/service.rs` `human_endpoint`).

use interweave_human_store::identity_key::{
    self, FilesystemCustody, IdentityKey, IdentityKeyFileError, KeyCipher,
};

use interweave_profile_config::{ProfileConfig, ProfilePaths};
use interweave_transport_api::EndpointId;

/// The client kind a human client leases under. A profile's endpoint
/// admits it by naming it in `allowed_client_kinds`.
pub const CLIENT_KIND: &str = "human-client";

/// The endpoints `config` lets a human client lease, in the file's order:
/// every entry naming [`CLIENT_KIND`], whatever `enabled` says -- the
/// daemon's effective state decides at open, and an administrator may have
/// enabled one since the file was written.
#[must_use]
pub fn leasable_endpoints(config: &ProfileConfig) -> Vec<EndpointId> {
    config
        .endpoints
        .entries
        .iter()
        .filter(|e| {
            e.allowed_client_kinds
                .iter()
                .any(|k| k.as_str() == CLIENT_KIND)
        })
        .map(|e| e.id.clone())
        .collect()
}

/// The own key of the identity in front (ADR-0063): the first leasable
/// endpoint's, opened from its file, or made with the identity the first
/// time a desktop-hosted client runs, at the desktop's filesystem floor
/// (ADR-0060 decision 3). `Ok(None)` when the profile offers no endpoint.
///
/// # Errors
/// As [`identity_key_under`]'s.
pub fn identity_key(
    paths: &ProfilePaths,
    endpoints: &[EndpointId],
) -> Result<Option<IdentityKey>, IdentityKeyFileError> {
    identity_key_under(paths, endpoints, &FilesystemCustody)
}

/// The own key of the identity in front under `cipher`'s custody: the
/// first of `endpoints`', opened from its file or made the first time.
/// The desktop's custody is [`identity_key`]'s; Android's service passes
/// the Keystore, whose alias is the identity's own (ADR-0063 decision 2).
/// `Ok(None)` when `endpoints` is empty.
///
/// # Errors
/// A file this build refuses -- another identity's, a seed record, one
/// below or above `cipher`'s custody, one whose wrapping key is gone --
/// or one that cannot be made. The file is left as it is: a new key is
/// the person's act, never this function's
/// (`a_key_file_this_build_refuses_is_left_as_it_is`,
/// `a_record_below_the_platforms_custody_is_refused_and_left`).
pub fn identity_key_under(
    paths: &ProfilePaths,
    endpoints: &[EndpointId],
    cipher: &dyn KeyCipher,
) -> Result<Option<IdentityKey>, IdentityKeyFileError> {
    let Some(endpoint) = endpoints.first() else {
        return Ok(None);
    };
    match identity_key::open(paths, endpoint, cipher)? {
        Some(key) => Ok(Some(key)),
        None => identity_key::create(paths, endpoint, cipher).map(Some),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn every_entry_naming_the_human_kind_in_file_order_disabled_too() {
        let config = ProfileConfig::parse_yaml(
            "schema_version: 2
profile:
  name: test
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
  entries:
    - { id: second, enabled: false, advertise: false, allowed_client_kinds: [human-client] }
    - { id: claude, enabled: true, advertise: true, allowed_client_kinds: [claude-channel] }
    - { id: human, enabled: true, advertise: true, allowed_client_kinds: [claude-channel, human-client] }
channels: { desired: [] }
",
        )
        .expect("parses");
        let ids: Vec<String> = leasable_endpoints(&config)
            .into_iter()
            .map(|e| e.as_str().to_owned())
            .collect();
        assert_eq!(ids, ["second", "human"]);
    }
}

#[cfg(test)]
mod identity_tests {
    #![allow(clippy::expect_used)]

    use std::os::unix::fs::PermissionsExt as _;

    use interweave_profile_config::TrustBoundary;

    use super::*;
    use interweave_human_store::identity_key::{CipherFailure, Custody, IdentityKeyError};

    fn paths(dir: &std::path::Path) -> ProfilePaths {
        ProfilePaths::resolve_embedded("default", TrustBoundary::new(dir).expect("a boundary"))
            .expect("paths")
    }

    fn private_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("chmod");
        dir
    }

    fn human() -> Vec<EndpointId> {
        vec![EndpointId::parse("human").expect("an endpoint")]
    }

    #[test]
    fn the_first_start_makes_the_identitys_key_and_every_later_one_opens_it() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let first = identity_key(&paths, &human())
            .expect("made")
            .expect("a key");
        let file = identity_key::key_path(&paths, &human()[0]);
        assert!(file.is_file(), "under the state root, beside the store");
        let again = identity_key(&paths, &human())
            .expect("opened")
            .expect("a key");
        assert_eq!(
            again.public_key(),
            first.public_key(),
            "the same key, never a new one"
        );
    }

    #[test]
    fn a_key_file_this_build_refuses_is_left_as_it_is() {
        let dir = private_dir();
        let paths = paths(dir.path());
        identity_key(&paths, &human()).expect("made");
        let file = identity_key::key_path(&paths, &human()[0]);
        std::fs::write(&file, "not a record\n").expect("overwrite");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("chmod");
        assert!(
            identity_key(&paths, &human()).is_err(),
            "refused, so nothing is signed"
        );
        assert_eq!(
            std::fs::read_to_string(&file).expect("read"),
            "not a record\n",
            "nothing replaces a refused file: a new key is the person's act"
        );
    }

    /// A sealing stand-in: custody `Keystore`, each byte of the secret masked by a
    /// pad of the alias's length, so a body opens only under its alias.
    /// It proves the custody floor's use here, not the Keystore.
    struct PadKeystore;

    impl KeyCipher for PadKeystore {
        fn custody(&self) -> Custody {
            Custody::Keystore
        }

        fn seal(&self, alias: &str, _: &[u8], secret: &[u8; 32]) -> Result<Vec<u8>, CipherFailure> {
            Ok(secret.iter().map(|b| b ^ pad(alias)).collect())
        }

        fn open(
            &self,
            alias: &str,
            _: &[u8],
            sealed: &[u8],
        ) -> Result<zeroize::Zeroizing<[u8; 32]>, CipherFailure> {
            let opened: Vec<u8> = sealed.iter().map(|b| b ^ pad(alias)).collect();
            let secret: [u8; 32] = opened.try_into().map_err(|_| CipherFailure::Unauthentic)?;
            Ok(zeroize::Zeroizing::new(secret))
        }

        fn retire(&self, _: &str) -> Result<(), CipherFailure> {
            Ok(())
        }
    }

    fn pad(alias: &str) -> u8 {
        u8::try_from(alias.len() % 251).expect("below 251") | 1
    }

    #[test]
    fn under_a_wrapping_key_the_first_start_makes_a_sealed_key_and_later_ones_open_it() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let first = identity_key_under(&paths, &human(), &PadKeystore)
            .expect("made")
            .expect("a key");
        let again = identity_key_under(&paths, &human(), &PadKeystore)
            .expect("opened")
            .expect("a key");
        assert_eq!(again.public_key(), first.public_key());
        assert!(
            identity_key(&paths, &human()).is_err(),
            "the record is sealed: the filesystem floor cannot read it"
        );
    }

    #[test]
    fn a_record_below_the_platforms_custody_is_refused_and_left() {
        let dir = private_dir();
        let paths = paths(dir.path());
        identity_key(&paths, &human()).expect("a filesystem-only record");
        let file = identity_key::key_path(&paths, &human()[0]);
        let before = std::fs::read(&file).expect("read");
        assert_eq!(
            identity_key_under(&paths, &human(), &PadKeystore).expect_err("a downgrade"),
            IdentityKeyFileError::Key(IdentityKeyError::CustodyDowngrade),
        );
        assert_eq!(
            std::fs::read(&file).expect("read"),
            before,
            "neither re-sealed nor replaced at open"
        );
    }

    #[test]
    fn a_profile_offering_no_endpoint_has_no_key_and_makes_none() {
        let dir = private_dir();
        let paths = paths(dir.path());
        assert!(identity_key(&paths, &[]).expect("nothing to do").is_none());
    }
}
