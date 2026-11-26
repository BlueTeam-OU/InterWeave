// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Which of a profile's endpoints a desktop-hosted human client may lease:
//! one rule for every root that connects to a shared daemon -- the window
//! and the command line -- so the two never disagree on which identity a
//! profile offers them. Android's service keeps its own rule for its
//! first identity -- the endpoint its profile names
//! (`crates/human/android-platform/src/service.rs` `human_endpoint`) --
//! and holds after it the added entries [`held_list`] reads.

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

/// One identity a desktop-hosted human client holds: its endpoint and
/// the alias its entry carries, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldEntry {
    /// The identity's endpoint.
    pub endpoint: EndpointId,
    /// Its entry's alias, as the entry carries it: `None` for an entry
    /// without one, never one made up in its place (identities §6.1).
    pub alias: Option<String>,
}

/// The identities a desktop-hosted human client holds, and the profile's
/// default among them (plan §30; architect-cto's ruling A5 of
/// 2026-11-25 and REPLY 01a216b6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldList {
    /// The first identity first -- the first authored entry admitting
    /// [`CLIENT_KIND`], what [`leasable_endpoints`]`(..).first()` names
    /// -- then every entry the endpoint overlay `added` that admits it,
    /// in the overlay's order. Empty when no authored entry admits it.
    pub held: Vec<HeldEntry>,
    /// The effective default -- the overlay's, else `config.yaml`'s --
    /// when it is held; else none, and the roster falls back.
    pub default: Option<EndpointId>,
}

/// The identities `config` and its endpoint overlay hold for a human
/// client ([`HeldList`]). A later AUTHORED entry admitting [`CLIENT_KIND`]
/// is not an identity: an endpoint authored by hand for this client kind
/// is a route (ADR-0061 decision 1), so only the first is held
/// (`a_second_authored_human_entry_is_not_held`). The overlay is read
/// read-only ([`EndpointOverlay::read_within`]), as
/// [`resolve_identity`] reads it; an absent overlay is no added entry.
///
/// # Errors
/// The overlay's, when it is present and cannot be read: NEVER read as
/// "no added entries" (architect-cto's REPLY 01a216b6, item 4), since
/// the client would then show a held list the file may contradict
/// (`a_present_overlay_that_cannot_be_read_is_an_error_and_an_absent_one_is_none`).
pub fn held_list(
    paths: &ProfilePaths,
    config: &ProfileConfig,
) -> Result<HeldList, EndpointOverlayError> {
    let admits = |e: &interweave_profile_config::EndpointConfig| {
        e.allowed_client_kinds
            .iter()
            .any(|k| k.as_str() == CLIENT_KIND)
    };
    let entry = |e: &interweave_profile_config::EndpointConfig| HeldEntry {
        endpoint: e.id.clone(),
        alias: e.alias.as_ref().map(ToString::to_string),
    };
    let (overlay, _normalised) = EndpointOverlay::read_within(
        &EndpointOverlay::path_for(paths),
        &config.endpoints,
        paths.boundary(),
    )?;
    let Some(first) = config.endpoints.entries.iter().find(|e| admits(e)) else {
        return Ok(HeldList {
            held: Vec::new(),
            default: None,
        });
    };
    let mut held = vec![entry(first)];
    held.extend(overlay.added().filter(|e| admits(e)).map(entry));
    let default = overlay
        .effective(&config.endpoints)
        .default
        .filter(|d| held.iter().any(|h| &h.endpoint == d));
    Ok(HeldList { held, default })
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

/// The own key of the identity in front (ADR-0063, ADR-0067): the first
/// leasable endpoint's, opened from its file at the desktop's filesystem
/// floor (ADR-0060 decision 3). `Ok(None)` when the profile offers no
/// endpoint.
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
/// first of `endpoints`', opened from its file. The desktop's custody is
/// [`identity_key`]'s; Android's service passes the Keystore, whose alias
/// is the identity's own (ADR-0063 decision 2). `Ok(None)` when
/// `endpoints` is empty.
///
/// NO KEY IS MADE HERE (ADR-0067 decision 5): the identity's id derives
/// from its key, so a key minted here would name an id the configuration
/// does not. The desktop's key is minted by `transportctl endpoint init`
/// or brought over by `endpoint adopt`, Android's at onboarding; a
/// missing file is the key-missing state whatever `store` holds.
/// `store` is kept for the callers' shape.
///
/// # Errors
/// A file this build refuses -- another identity's, a seed record, one
/// below or above `cipher`'s custody, one whose wrapping key is gone --
/// and [`IdentityKeyFileError::Missing`] when there is no file. The file
/// is left as it is (`a_key_file_this_build_refuses_is_left_as_it_is`,
/// `a_record_below_the_platforms_custody_is_refused_and_left`,
/// `a_missing_key_file_is_missing_and_none_is_made`).
pub fn identity_key_under(
    paths: &ProfilePaths,
    endpoints: &[EndpointId],
    cipher: &dyn KeyCipher,
    _store: &HumanStore,
) -> Result<Option<IdentityKey>, IdentityKeyFileError> {
    let Some(endpoint) = endpoints.first() else {
        return Ok(None);
    };
    identity_key::open(paths, endpoint, cipher)?
        .map(Some)
        .ok_or(IdentityKeyFileError::Missing)
}

/// Where the identity in front's key lives and under whose custody: what
/// lets the facade open it again (design/2026-10-24-identity-key.md
/// section 6). Nothing here makes a key in the identity's place: its id
/// derives from the key, so a new key is a new identity (ADR-0067
/// decision 5).
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

    /// Open the key by [`identity_key_under`]'s rule.
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
                    .join(identity_key::ENDPOINT_KEYS_DIR)
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

    /// An overlay whose `added` holds `added` (human-client) and `bot`
    /// (claude-channel alone), each with its empty own set, and whose
    /// default is `default` when given.
    fn overlay_with_added(default: Option<&EndpointId>) -> String {
        let (added, bot) = (endpoint_id("added"), endpoint_id("bot"));
        let default =
            default.map_or_else(String::new, |d| format!(r#""default":"{}","#, d.as_str()));
        format!(
            r#"{{{default}"enabled":{{}},"added":[{{"key":"{a}.key","alias":"added","enabled":true,"advertise":false,"allowed_client_kinds":["human-client"]}},{{"key":"{b}.key","alias":"bot","enabled":true,"advertise":false,"allowed_client_kinds":["claude-channel"]}}],"inbound":{{"{a}":[],"{b}":[]}},"outbound":{{"{a}":[],"{b}":[]}}}}"#,
            a = added.as_str(),
            b = bot.as_str(),
        )
    }

    fn ids(list: &HeldList) -> Vec<EndpointId> {
        list.held.iter().map(|h| h.endpoint.clone()).collect()
    }

    /// The first authored human-client entry is the first identity, with
    /// its alias; a second authored one is a route, not an identity; an
    /// added human-client entry is held, an added entry of another kind
    /// is not. Mutations: `held_list` taking every authored human-client
    /// entry -- `human` is held; not filtering the added by kind -- `bot`
    /// is held; dropping the added -- `added` is missing.
    #[test]
    fn a_second_authored_human_entry_is_not_held() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = paths_with(dir.path(), Some(&overlay_with_added(None)));
        let list = held_list(&paths, &three()).expect("reads");
        assert_eq!(ids(&list), [endpoint_id("second"), endpoint_id("added")]);
        assert_eq!(list.held[0].alias.as_deref(), Some("second"));
        assert_eq!(list.held[1].alias.as_deref(), Some("added"));
    }

    /// A present overlay that cannot be read is an error, never "no
    /// added entries"; an absent one is the first identity alone.
    /// Mutation: `held_list` reading a failed overlay as the empty one --
    /// the corrupt case answers `Ok`.
    #[test]
    fn a_present_overlay_that_cannot_be_read_is_an_error_and_an_absent_one_is_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = paths_with(dir.path(), Some("{ not json"));
        assert!(matches!(
            held_list(&paths, &three()),
            Err(EndpointOverlayError::Parse(_))
        ));
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = paths_with(dir.path(), None);
        let list = held_list(&paths, &three()).expect("reads");
        assert_eq!(ids(&list), [endpoint_id("second")]);
        assert_eq!(list.default, None);
    }

    /// The default is the overlay's effective one, an added entry
    /// included, when it is held; one not held is none. Mutations:
    /// `held_list` taking `config.yaml`'s default alone -- the added
    /// default is lost; not filtering by held -- `bot` is the default.
    #[test]
    fn the_default_is_the_overlays_when_held() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = paths_with(
            dir.path(),
            Some(&overlay_with_added(Some(&endpoint_id("added")))),
        );
        let list = held_list(&paths, &three()).expect("reads");
        assert_eq!(list.default, Some(endpoint_id("added")));
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = paths_with(
            dir.path(),
            Some(&overlay_with_added(Some(&endpoint_id("bot")))),
        );
        let list = held_list(&paths, &three()).expect("reads");
        assert_eq!(list.default, None, "not held");
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
            &interweave_test_support::endpoint_keys::endpoint_id("human"),
            interweave_human_store::StoreOptions::default(),
        )
        .expect("opens")
    }

    /// A key minted as `transportctl endpoint init` does, under `cipher`,
    /// and the endpoint list naming its derived id.
    fn minted(paths: &ProfilePaths, cipher: &dyn KeyCipher) -> (IdentityKey, Vec<EndpointId>) {
        let key = identity_key::create(paths, cipher).expect("minted");
        let endpoints = vec![key.endpoint().clone()];
        (key, endpoints)
    }

    /// ADR-0067 decision 5: a first run with no key file is the
    /// key-missing state, and no key is made under the configured id.
    #[test]
    fn a_missing_key_file_is_missing_and_none_is_made() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let store = store(dir.path());
        let unminted = vec![IdentityKey::generate().expect("a key").endpoint().clone()];
        assert_eq!(
            identity_key(&paths, &unminted, &store).expect_err("missing"),
            IdentityKeyFileError::Missing
        );
        assert!(
            !identity_key::key_path(&paths, &unminted[0]).exists(),
            "no key made in its place"
        );
        assert_eq!(
            identity_key_under(&paths, &unminted, &PadKeystore, &store).expect_err("missing"),
            IdentityKeyFileError::Missing,
            "under a wrapping key too"
        );
        assert_eq!(
            identity_key::sealed(&paths).expect("listed"),
            Vec::<EndpointId>::new(),
            "no key file anywhere"
        );
    }

    #[test]
    fn a_minted_key_is_opened_and_every_later_start_opens_the_same() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let store = store(dir.path());
        let (minted, endpoints) = minted(&paths, &FilesystemCustody);
        let first = identity_key(&paths, &endpoints, &store)
            .expect("opened")
            .expect("a key");
        assert_eq!(first.public_key(), minted.public_key());
        let again = identity_key(&paths, &endpoints, &store)
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
        let (_, endpoints) = minted(&paths, &FilesystemCustody);
        let file = identity_key::key_path(&paths, &endpoints[0]);
        std::fs::write(&file, "not a record\n").expect("overwrite");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("chmod");
        assert!(
            identity_key(&paths, &endpoints, &store).is_err(),
            "refused, so nothing is signed"
        );
        assert_eq!(
            std::fs::read_to_string(&file).expect("read"),
            "not a record\n",
            "nothing replaces a refused file: a new key is a new identity"
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
    fn under_a_wrapping_key_a_sealed_key_opens_and_the_filesystem_floor_cannot_read_it() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let store = store(dir.path());
        let (minted, endpoints) = minted(&paths, &PadKeystore);
        let first = identity_key_under(&paths, &endpoints, &PadKeystore, &store)
            .expect("opened")
            .expect("a key");
        assert_eq!(first.public_key(), minted.public_key());
        assert_eq!(
            identity_key(&paths, &endpoints, &store).expect_err("sealed"),
            IdentityKeyFileError::Key(IdentityKeyError::SealedUnreadableHere),
            "the record is sealed: the filesystem floor cannot read it"
        );
    }

    #[test]
    fn a_record_below_the_platforms_custody_is_refused_and_left() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let store = store(dir.path());
        let (_, endpoints) = minted(&paths, &FilesystemCustody);
        let file = identity_key::key_path(&paths, &endpoints[0]);
        let before = std::fs::read(&file).expect("read");
        assert_eq!(
            identity_key_under(&paths, &endpoints, &PadKeystore, &store).expect_err("a downgrade"),
            IdentityKeyFileError::Key(IdentityKeyError::CustodyDowngrade),
        );
        assert_eq!(
            std::fs::read(&file).expect("read"),
            before,
            "neither re-sealed nor replaced at open"
        );
    }

    /// The key folder a `DirUnsafe` refusal names is `endpoint-keys/`, the
    /// directory the key is read from.
    #[test]
    fn an_unsafe_key_folder_is_named_as_endpoint_keys() {
        let dir = private_dir();
        let paths = paths(dir.path());
        let (_, endpoints) = minted(&paths, &FilesystemCustody);
        let source = KeySource::new(
            paths.clone(),
            endpoints[0].clone(),
            Box::new(FilesystemCustody),
        );
        let refused = IdentityKeyFileError::Read(identity_key::KeyFileRead::DirUnsafe);
        assert_eq!(
            source.problem(&refused),
            KeyProblem::DirUnsafe {
                folder: paths
                    .state_dir()
                    .join("endpoint-keys")
                    .display()
                    .to_string()
            }
        );
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
