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

use interweave_human_store::HumanStore;
use interweave_human_ui_model::KeyProblem;
use interweave_profile_config::endpoint_overlay::{EndpointOverlay, EndpointOverlayError};
use interweave_profile_config::{
    EndpointSelector, ProfileConfig, ProfilePaths, ResolveEndpointError, resolve_endpoint,
};
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

/// Why [`resolve_identity`] named no endpoint: each a refused start, named
/// for what the person typed, never a hello sent with it (ENDPOINTS.md,
/// an alias is resolved by the client).
#[derive(Debug)]
pub enum IdentityRefused {
    /// An alias was given and the endpoint overlay, which it resolves
    /// over, did not read.
    Overlay(EndpointOverlayError),
    /// No configured endpoint has this alias or this id.
    Unknown(ResolveEndpointError),
    /// The endpoint it names is configured, and does not admit
    /// [`CLIENT_KIND`]: another client's, `transportctl`'s to administer.
    NotLeasable(EndpointId),
}

impl core::fmt::Display for IdentityRefused {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Overlay(e) => write!(f, "an alias needs the endpoint overlay: {e}"),
            Self::Unknown(e) => e.fmt(f),
            Self::NotLeasable(_) => {
                f.write_str("that endpoint is not one a human client may use in this profile")
            }
        }
    }
}

/// The `EndpointId` of the human endpoint `selector` names -- an alias or
/// an id, as a person types it -- in `config`: the one place a human
/// client turns what it was given into the id its session claims, called
/// once per identity (ADR-0067 decision 3). Every wire field takes the
/// id it returns, never the alias.
///
/// The endpoint overlay is read for an alias alone, read-only
/// ([`EndpointOverlay::read_within`]): an alias resolves in the set the
/// daemon composes, so an entry the overlay adds is found once overlays
/// add entries, and a client leaves the overlay as it found it. An id is
/// judged against `config.yaml`'s entries and never waits on the overlay
/// (`an_id_resolves_without_reading_the_overlay`). No file under
/// `endpoint-keys/` is read: an entry names its id in its `key` value.
///
/// # Errors
/// [`IdentityRefused`]: the overlay unread for an alias, an alias or id
/// no entry carries, an endpoint that does not admit [`CLIENT_KIND`].
pub fn resolve_identity(
    paths: &ProfilePaths,
    config: &ProfileConfig,
    selector: &EndpointSelector,
) -> Result<EndpointId, IdentityRefused> {
    let overlay = match selector {
        EndpointSelector::Id(_) => EndpointOverlay::default(),
        EndpointSelector::Alias(_) => EndpointOverlay::read_within(
            &EndpointOverlay::path_for(paths),
            &config.endpoints,
            paths.boundary(),
        )
        .map(|(overlay, _normalised)| overlay)
        .map_err(IdentityRefused::Overlay)?,
    };
    let id = resolve_endpoint(&config.endpoints, &overlay, &selector.to_string())
        .map_err(IdentityRefused::Unknown)?;
    let composed = overlay.apply(&config.endpoints);
    let leasable = composed.entries.iter().any(|e| {
        e.id == id
            && e.allowed_client_kinds
                .iter()
                .any(|k| k.as_str() == CLIENT_KIND)
    });
    if leasable {
        Ok(id)
    } else {
        Err(IdentityRefused::NotLeasable(id))
    }
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
    store: &HumanStore,
) -> Result<Option<IdentityKey>, IdentityKeyFileError> {
    identity_key_under(paths, endpoints, &FilesystemCustody, store)
}

/// The own key of the identity in front under `cipher`'s custody: the
/// first of `endpoints`', opened from its file, or made when it has none
/// and no conversation holds or may hold its card (a share `store`
/// records, or an own card row it holds pending). The desktop's custody is
/// [`identity_key`]'s; Android's service passes the Keystore, whose alias
/// is the identity's own (ADR-0063 decision 2). `Ok(None)` when
/// `endpoints` is empty.
///
/// # Errors
/// A file this build refuses -- another identity's, a seed record, one
/// below or above `cipher`'s custody, one whose wrapping key is gone --
/// or one that cannot be made; [`IdentityKeyFileError::Missing`] when the
/// file is gone after a share or while a card is on its way, or the
/// store cannot say neither (architect-cto's rulings of 2026-10-24 and
/// 2026-10-27: counterparties hold or may receive that key, so a new
/// one is the person's confirmed act). The file is left as it
/// is: a new key is the person's act, never this function's
/// (`a_key_file_this_build_refuses_is_left_as_it_is`,
/// `a_record_below_the_platforms_custody_is_refused_and_left`,
/// `a_key_file_gone_after_a_share_is_missing_and_none_is_made`,
/// and `tests/cards.rs`'s `a_key_file_gone_while_a_card_is_pending_is_missing`).
pub fn identity_key_under(
    paths: &ProfilePaths,
    endpoints: &[EndpointId],
    cipher: &dyn KeyCipher,
    store: &HumanStore,
) -> Result<Option<IdentityKey>, IdentityKeyFileError> {
    let Some(endpoint) = endpoints.first() else {
        return Ok(None);
    };
    if let Some(key) = identity_key::open(paths, endpoint, cipher)? {
        return Ok(Some(key));
    }
    if crate::cards::card_may_be_held(store) {
        return Err(IdentityKeyFileError::Missing);
    }
    identity_key::create(paths, endpoint, cipher).map(Some)
}

/// Where the identity in front's key lives and under whose custody: what
/// lets the facade open it again and make a new one
/// (design/2026-10-24-identity-key.md sections 5 and 6).
pub struct KeySource {
    paths: ProfilePaths,
    endpoint: EndpointId,
    cipher: Box<dyn KeyCipher + Send>,
}

impl KeySource {
    /// `endpoint`'s key under `cipher`, in `paths`' state directory.
    #[must_use]
    pub fn new(
        paths: ProfilePaths,
        endpoint: EndpointId,
        cipher: Box<dyn KeyCipher + Send>,
    ) -> Self {
        Self {
            paths,
            endpoint,
            cipher,
        }
    }

    /// The identity.
    #[must_use]
    pub const fn endpoint(&self) -> &EndpointId {
        &self.endpoint
    }

    /// Open the key, or make the first one, by [`identity_key_under`]'s
    /// rule.
    ///
    /// # Errors
    /// As [`identity_key_under`]'s.
    pub fn open(&self, store: &HumanStore) -> Result<IdentityKey, IdentityKeyFileError> {
        identity_key_under(
            &self.paths,
            std::slice::from_ref(&self.endpoint),
            &*self.cipher,
            store,
        )?
        .ok_or(IdentityKeyFileError::Missing)
    }

    /// Replace the key, on the person's confirmation
    /// ([`identity_key::replace`]).
    ///
    /// # Errors
    /// As [`identity_key::replace`]'s.
    pub fn replace(&self) -> Result<IdentityKey, identity_key::NewKeyFailed> {
        identity_key::replace(&self.paths, &self.endpoint, &*self.cipher)
    }

    /// The person's terms for a refusal: its kind, the key folder named
    /// where the folder is the problem. A refusal no open gives is told as
    /// one that may pass.
    #[must_use]
    pub fn problem(&self, refused: &IdentityKeyFileError) -> KeyProblem {
        use identity_key::KeyUnusable as K;
        match refused.unusable() {
            Some(K::LostOnDevice) => KeyProblem::LostOnDevice,
            Some(K::BelowDevice) => KeyProblem::BelowDevice,
            Some(K::OtherDevice) => KeyProblem::OtherDevice,
            Some(K::NewerVersion) => KeyProblem::NewerVersion,
            Some(K::Damaged) => KeyProblem::Damaged,
            Some(K::Exposed) => KeyProblem::Exposed,
            Some(K::DirUnsafe) => KeyProblem::DirUnsafe {
                folder: self
                    .paths
                    .state_dir()
                    .join(identity_key::IDENTITY_KEYS_DIR)
                    .display()
                    .to_string(),
            },
            Some(K::Missing) => KeyProblem::Missing,
            Some(K::NotNow) | None => KeyProblem::NotNow,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    use interweave_test_support::endpoint_keys::endpoint_id;

    /// A profile of three entries by key: `second` and `human` open to
    /// the human client, `claude` the bridge's alone.
    fn three() -> ProfileConfig {
        let text = format!(
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
    - {{ key: {second}.key, alias: second, enabled: false, advertise: false, allowed_client_kinds: [human-client] }}
    - {{ key: {claude}.key, alias: claude, enabled: true, advertise: true, allowed_client_kinds: [claude-channel] }}
    - {{ key: {human}.key, alias: human, enabled: true, advertise: true, allowed_client_kinds: [claude-channel, human-client] }}
channels: {{ desired: [] }}
",
            second = endpoint_id("second").as_str(),
            claude = endpoint_id("claude").as_str(),
            human = endpoint_id("human").as_str(),
        );
        ProfileConfig::parse_yaml(&text).expect("parses")
    }

    /// Paths whose state directory holds `overlay`, when given, as the
    /// endpoint overlay; the directory is private, as a profile's is.
    fn paths_with(dir: &std::path::Path, overlay: Option<&str>) -> ProfilePaths {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).expect("chmod");
        let paths = ProfilePaths::resolve_embedded(
            "default",
            interweave_profile_config::TrustBoundary::new(dir).expect("a boundary"),
        )
        .expect("paths");
        if let Some(text) = overlay {
            let file = EndpointOverlay::path_for(&paths);
            let state = file.parent().expect("a state dir");
            std::fs::create_dir_all(state).expect("mkdir");
            std::fs::set_permissions(state, std::fs::Permissions::from_mode(0o700)).expect("chmod");
            std::fs::write(&file, text).expect("writes");
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("chmod");
        }
        paths
    }

    fn selector(text: &str) -> EndpointSelector {
        EndpointSelector::parse(text).expect("a selector")
    }

    #[test]
    fn every_entry_naming_the_human_kind_in_file_order_disabled_too() {
        assert_eq!(
            leasable_endpoints(&three()),
            [endpoint_id("second"), endpoint_id("human")]
        );
    }

    /// An alias and the id it stands for name the same endpoint; the id
    /// returned is the entry's derived one, never the alias.
    #[test]
    fn an_alias_and_its_id_resolve_to_the_same_derived_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = paths_with(dir.path(), None);
        let config = three();
        let human = endpoint_id("human");
        assert_eq!(
            resolve_identity(&paths, &config, &selector("human")).expect("resolves"),
            human
        );
        assert_eq!(
            resolve_identity(&paths, &config, &selector(human.as_str())).expect("resolves"),
            human
        );
        // A disabled entry resolves: whether it may be leased is the
        // daemon's to answer at open.
        assert_eq!(
            resolve_identity(&paths, &config, &selector("second")).expect("resolves"),
            endpoint_id("second")
        );
    }

    #[test]
    fn an_unknown_alias_or_id_is_refused_by_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = paths_with(dir.path(), None);
        let config = three();
        assert!(matches!(
            resolve_identity(&paths, &config, &selector("nobody")),
            Err(IdentityRefused::Unknown(
                ResolveEndpointError::UnknownAlias(_)
            ))
        ));
        assert!(matches!(
            resolve_identity(&paths, &config, &selector(endpoint_id("nobody").as_str())),
            Err(IdentityRefused::Unknown(ResolveEndpointError::UnknownId(_)))
        ));
    }

    /// The bridge's endpoint is configured and resolves, and is not one a
    /// human client may claim, by its alias or by its id.
    #[test]
    fn another_clients_endpoint_is_not_leasable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = paths_with(dir.path(), None);
        let config = three();
        let claude = endpoint_id("claude");
        for given in ["claude", claude.as_str()] {
            match resolve_identity(&paths, &config, &selector(given)) {
                Err(IdentityRefused::NotLeasable(id)) => assert_eq!(id, claude),
                other => panic!("{given}: {other:?}"),
            }
        }
    }

    /// An overlay that does not read refuses an alias, which resolves over
    /// it, and never an id, which is judged on `config.yaml` alone.
    #[test]
    fn an_id_resolves_without_reading_the_overlay() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = paths_with(dir.path(), Some("not an overlay: ["));
        let config = three();
        assert!(matches!(
            resolve_identity(&paths, &config, &selector("human")),
            Err(IdentityRefused::Overlay(_))
        ));
        assert_eq!(
            resolve_identity(&paths, &config, &selector(endpoint_id("human").as_str()))
                .expect("an id needs no overlay"),
            endpoint_id("human")
        );
    }

    /// Resolving writes nothing in the profile: an overlay normalising
    /// would change is left byte for byte as it was.
    #[test]
    fn resolving_an_alias_leaves_the_overlay_as_it_found_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        // `enabled` naming an endpoint the profile lacks: normalising
        // drops it, and a load would write that back.
        let text = format!(
            "{{\"enabled\": {{\"{}\": true}}}}",
            endpoint_id("gone").as_str()
        );
        let paths = paths_with(dir.path(), Some(&text));
        let before = std::fs::read(EndpointOverlay::path_for(&paths)).expect("reads");
        resolve_identity(&paths, &three(), &selector("human")).expect("resolves");
        assert_eq!(
            std::fs::read(EndpointOverlay::path_for(&paths)).expect("reads"),
            before
        );
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

    fn store(dir: &std::path::Path) -> HumanStore {
        let state = dir.join("store");
        std::fs::create_dir_all(&state).expect("mkdir");
        std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).expect("chmod");
        HumanStore::open(
            &state.join("human.sqlite3"),
            interweave_human_store::StoreOptions::default(),
        )
        .expect("opens")
    }

    fn human() -> Vec<EndpointId> {
        vec![interweave_test_support::endpoint_keys::endpoint_id("human")]
    }

    #[test]
    fn the_first_start_makes_the_identitys_key_and_every_later_one_opens_it() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let store = store(dir.path());
        let first = identity_key(&paths, &human(), &store)
            .expect("made")
            .expect("a key");
        let file = identity_key::key_path(&paths, &human()[0]);
        assert!(file.is_file(), "under the state root, beside the store");
        let again = identity_key(&paths, &human(), &store)
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
        let store = store(dir.path());
        identity_key(&paths, &human(), &store).expect("made");
        let file = identity_key::key_path(&paths, &human()[0]);
        std::fs::write(&file, "not a record\n").expect("overwrite");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("chmod");
        assert!(
            identity_key(&paths, &human(), &store).is_err(),
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
        let store = store(dir.path());
        let first = identity_key_under(&paths, &human(), &PadKeystore, &store)
            .expect("made")
            .expect("a key");
        let again = identity_key_under(&paths, &human(), &PadKeystore, &store)
            .expect("opened")
            .expect("a key");
        assert_eq!(again.public_key(), first.public_key());
        assert!(
            identity_key(&paths, &human(), &store).is_err(),
            "the record is sealed: the filesystem floor cannot read it"
        );
    }

    #[test]
    fn a_record_below_the_platforms_custody_is_refused_and_left() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let store = store(dir.path());
        identity_key(&paths, &human(), &store).expect("a filesystem-only record");
        let file = identity_key::key_path(&paths, &human()[0]);
        let before = std::fs::read(&file).expect("read");
        assert_eq!(
            identity_key_under(&paths, &human(), &PadKeystore, &store).expect_err("a downgrade"),
            IdentityKeyFileError::Key(IdentityKeyError::CustodyDowngrade),
        );
        assert_eq!(
            std::fs::read(&file).expect("read"),
            before,
            "neither re-sealed nor replaced at open"
        );
    }

    /// architect-cto's ruling of 2026-10-24: once any conversation
    /// recorded a share of the card, a missing file is the key-missing
    /// state and no key is made; before any share it is a first key.
    #[test]
    fn a_key_file_gone_after_a_share_is_missing_and_none_is_made() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let mut store = store(dir.path());
        identity_key(&paths, &human(), &store).expect("a first key");
        let file = identity_key::key_path(&paths, &human()[0]);
        std::fs::remove_file(&file).expect("lost");
        identity_key(&paths, &human(), &store).expect("never shared: a first key again");
        store
            .mark_card_shared(
                &interweave_transport_api::TransportIdentity::parse(
                    "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN",
                )
                .expect("a PeerId"),
                &human()[0],
                10,
            )
            .expect("shared");
        std::fs::remove_file(&file).expect("lost again");
        assert_eq!(
            identity_key(&paths, &human(), &store).expect_err("missing"),
            IdentityKeyFileError::Missing
        );
        assert!(!file.exists(), "no key made in its place");
    }

    #[test]
    fn a_profile_offering_no_endpoint_has_no_key_and_makes_none() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let store = store(dir.path());
        assert!(
            identity_key(&paths, &[], &store)
                .expect("nothing to do")
                .is_none()
        );
    }
}
