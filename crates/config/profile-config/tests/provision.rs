// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The first-run profile of an embedded-android host, in its two forms
//! (`provision::provision_embedded`, `provision::provision_identity`):
//! form 1 is written once, owner-only, under the host's configuration
//! root, and loads with no identity; an earlier release's document is
//! replaced by it, one without the marker never is; form 2 names the
//! sealed first identity and loads with its key present.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(target_os = "linux")]

use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

use interweave_profile_config::endpoint_key::{Custody, ENDPOINT_KEYS_DIR, mint_filesystem};
use interweave_profile_config::provision::{
    EMBEDDED_DOCUMENT_MARKER, FIRST_IDENTITY_ALIAS, ProvisionIdentityError, embedded_document,
    embedded_identity_document, provision_embedded, provision_identity,
};
use interweave_profile_config::runtime::Deployment;
use interweave_profile_config::{
    ClientKind, ConfigError, EndpointAlias, EndpointKeyState, LoadError, NotDerivedEntry,
    PersistError, ProfileConfig, ProfilePaths, TrustBoundary,
};
use interweave_transport_api::EndpointId;

/// An earlier release's first-run document for profile `work`, BYTE FOR
/// BYTE as `embedded_document("work")` wrote it at 278709595 (and every
/// release before ADR-0067's key-named entries): the input M2 replaces.
const RETIRED_DOCUMENT_WORK: &str =
    "# Written on first run by the Android client: the embedded-android
# defaults. Trust, relays and AutoNAT servers arrive through the admin
# trust surface or onboarding, not here.
schema_version: 2
profile:
  name: work
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
      config: {}
";

/// An app data directory, owner-only, under an owner-only temporary root.
fn app() -> (tempfile::TempDir, std::path::PathBuf) {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let app = root.path().join("app");
    std::fs::create_dir(&app).expect("mkdir");
    std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o700)).expect("chmod");
    (root, app)
}

fn paths_in(app: &std::path::Path) -> ProfilePaths {
    ProfilePaths::resolve_embedded("work", TrustBoundary::new(app).expect("a boundary"))
        .expect("paths")
}

/// `paths`' `config.yaml` written as `text`, owner-only, as an earlier
/// release's first run wrote it.
fn plant(paths: &ProfilePaths, text: &str) {
    interweave_profile_config::create_private_dir_within(paths.config_dir(), paths.boundary())
        .expect("config dir");
    interweave_profile_config::write_private_atomic_within(
        &paths.config_file(),
        text.as_bytes(),
        paths.boundary(),
    )
    .expect("planted");
}

fn read(paths: &ProfilePaths) -> String {
    std::fs::read_to_string(paths.config_file()).expect("config.yaml")
}

#[test]
fn form_1_validates_as_an_embedded_android_profile_with_no_identity() {
    let config = ProfileConfig::parse_yaml(&embedded_document("work")).expect("parses");
    assert!(config.validate().is_empty(), "{:?}", config.validate());
    assert_eq!(config.runtime.deployment, Deployment::EmbeddedAndroid);
    assert_eq!(config.runtime.android.endpoint, None, "no service endpoint");
    assert!(config.endpoints.entries.is_empty(), "no endpoint entry");
    assert_eq!(config.endpoints.default_direct_endpoint, None, "no default");
    assert!(!config.ipc.enabled, "ipc off");
    assert!(
        config
            .transport
            .listen
            .addresses
            .iter()
            .all(|a| a.starts_with("/ip4/0.0.0.0/") || a.starts_with("/ip6/::/")),
        "wildcard listeners only: {:?}",
        config.transport.listen.addresses
    );
    assert!(
        config.trust.allowed_peers.is_empty(),
        "no trust is defaulted"
    );
    // The start's half of the lease rule refuses it by name.
    assert_eq!(
        config.runtime.require_android_identity(&config.endpoints),
        Err(ConfigError::AndroidNoIdentity)
    );
}

/// Form 2 is form 1 plus the one entry, the service endpoint and the
/// default -- and nothing else: the shared head and the discovery block
/// are the same bytes.
#[test]
fn form_2_is_form_1_plus_the_first_identity() {
    let id = EndpointId::parse_derived("12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr")
        .expect("a derived id");
    let form_1 = embedded_document("work");
    let form_2 = embedded_identity_document("work", &id);
    let config = ProfileConfig::parse_yaml(&form_2).expect("parses");
    assert!(config.validate().is_empty(), "{:?}", config.validate());
    assert_eq!(config.runtime.android.endpoint.as_ref(), Some(&id));
    assert_eq!(config.endpoints.default_direct_endpoint.as_ref(), Some(&id));
    let [entry] = config.endpoints.entries.as_slice() else {
        panic!("one entry: {:?}", config.endpoints.entries);
    };
    assert_eq!(entry.id, id);
    assert_eq!(
        entry.alias.as_ref().map(EndpointAlias::as_str),
        Some(FIRST_IDENTITY_ALIAS)
    );
    assert!(entry.enabled, "enabled");
    // `human-client-ui.md` §13: an identity's endpoint is never
    // advertised. The list is hygiene, not authority, and empty means
    // unrestricted, so the exact list is pinned.
    assert!(!entry.advertise, "not advertised");
    let kinds: Vec<&str> = entry
        .allowed_client_kinds
        .iter()
        .map(ClientKind::as_str)
        .collect();
    assert_eq!(kinds, ["human-client"]);
    assert_eq!(
        config.runtime.require_android_identity(&config.endpoints),
        Ok(()),
        "the start's rule admits it"
    );
    // Everything but the endpoint lines is form 1's.
    let strip = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|l| {
                !l.contains(id.as_str())
                    && !l.contains("alias:")
                    && !l.contains("default_direct_endpoint")
                    && !l.trim_start().starts_with("enabled: true")
                    && !l.trim_start().starts_with("advertise:")
                    && !l.trim_start().starts_with("allowed_client_kinds:")
                    && *l != "  android:"
                    && *l != "  entries:"
                    && *l != "  entries: []"
            })
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(strip(&form_1), strip(&form_2));
}

/// The release build's check reads the marker in the native library, so
/// both forms open with exactly it.
#[test]
fn both_forms_open_with_the_marker_the_release_check_reads() {
    let id = EndpointId::parse_derived("12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr")
        .expect("a derived id");
    for document in [
        embedded_document("work"),
        embedded_identity_document("work", &id),
    ] {
        assert!(document.starts_with(EMBEDDED_DOCUMENT_MARKER), "{document}");
    }
    assert_eq!(
        EMBEDDED_DOCUMENT_MARKER,
        "# Written on first run by the Android client"
    );
}

/// M1: an absent `config.yaml` is written as form 1, owner-only, under
/// the app's own directory, and it LOADS; a second first run is refused
/// `AlreadyExists` -- android-platform's `provision_profile` takes that
/// as done -- and leaves what is there as it was.
#[test]
fn a_first_run_writes_form_1_and_a_second_is_refused() {
    let (_root, app) = app();
    let paths = paths_in(&app);
    provision_embedded(&paths).expect("the first run writes it");
    let meta = std::fs::metadata(paths.config_file()).expect("written");
    assert_eq!(meta.mode() & 0o777, 0o600, "owner-only");
    assert!(
        paths.config_file().starts_with(&app),
        "under the app's own directory"
    );
    assert_eq!(read(&paths), embedded_document("work"), "form 1");
    let loaded = ProfileConfig::load(&paths).expect("form 1 loads");
    assert_eq!(loaded.runtime.deployment, Deployment::EmbeddedAndroid);
    assert_eq!(loaded.runtime.android.endpoint, None);

    // A second first run on form 1 is refused, and form 1 stands.
    assert!(matches!(
        provision_embedded(&paths),
        Err(PersistError::AlreadyExists)
    ));
    assert_eq!(read(&paths), embedded_document("work"), "nothing replaced");

    // And on a document the person edited.
    std::fs::write(paths.config_file(), "edited by the person\n").expect("an edit");
    assert!(matches!(
        provision_embedded(&paths),
        Err(PersistError::AlreadyExists)
    ));
    assert_eq!(read(&paths), "edited by the person\n", "nothing replaced");
}

/// M2: an earlier release's document -- the marker and an entry by the
/// retired `- id:` -- is replaced whole by form 1, owner-only, reading
/// no key: `endpoint-keys/` is shut (mode 000) and the replace still
/// goes through. Its load was refused `EndpointIdNotDerived` (the
/// control); form 1 loads.
#[test]
fn an_earlier_releases_document_is_replaced_by_form_1() {
    let (_root, app) = app();
    let paths = paths_in(&app);
    plant(&paths, RETIRED_DOCUMENT_WORK);
    assert!(
        matches!(
            ProfileConfig::load(&paths),
            Err(LoadError::Invalid(errors)) if errors.iter().any(|e| matches!(
                e,
                ConfigError::EndpointIdNotDerived { found: NotDerivedEntry::RetiredId { .. } }
            ))
        ),
        "the control: the earlier document is refused as retired"
    );
    let keys = paths.state_dir().join(ENDPOINT_KEYS_DIR);
    interweave_profile_config::create_private_dir_within(&keys, paths.boundary())
        .expect("keys dir");
    std::fs::set_permissions(&keys, std::fs::Permissions::from_mode(0o000)).expect("shut");
    let replaced = provision_embedded(&paths);
    std::fs::set_permissions(&keys, std::fs::Permissions::from_mode(0o700)).expect("open");
    replaced.expect("the earlier document is replaced");
    assert_eq!(read(&paths), embedded_document("work"), "by form 1, whole");
    assert_eq!(
        std::fs::metadata(paths.config_file())
            .expect("present")
            .mode()
            & 0o777,
        0o600,
        "owner-only"
    );
    ProfileConfig::load(&paths).expect("form 1 loads");
    // A third run finds form 1 and leaves it.
    assert!(matches!(
        provision_embedded(&paths),
        Err(PersistError::AlreadyExists)
    ));
}

/// M2's other side: the SAME retired entry without the marker is the
/// person's document, left byte for byte, and its load is still refused
/// `EndpointIdNotDerived` as on the desktop. A marker document that is
/// form 2 is left too.
#[test]
fn a_document_without_the_marker_is_never_replaced() {
    let (_root, app) = app();
    let paths = paths_in(&app);
    let authored = RETIRED_DOCUMENT_WORK
        .split_once('\n')
        .map(|(_, rest)| format!("# Mine.\n{rest}"))
        .expect("a first line");
    assert!(authored.contains("    - id: human\n"));
    plant(&paths, &authored);
    assert!(matches!(
        provision_embedded(&paths),
        Err(PersistError::AlreadyExists)
    ));
    assert_eq!(read(&paths), authored, "byte-identical");
    match ProfileConfig::load(&paths) {
        Err(LoadError::Invalid(errors)) => assert!(
            errors.iter().any(|e| matches!(
                e,
                ConfigError::EndpointIdNotDerived {
                    found: NotDerivedEntry::RetiredId { .. }
                }
            )),
            "{errors:?}"
        ),
        other => panic!("refused EndpointIdNotDerived: {:?}", other.map(|_| ())),
    }

    let id = EndpointId::parse_derived("12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr")
        .expect("a derived id");
    let form_2 = embedded_identity_document("work", &id);
    plant(&paths, &form_2);
    assert!(matches!(
        provision_embedded(&paths),
        Err(PersistError::AlreadyExists)
    ));
    assert_eq!(read(&paths), form_2, "form 2 left as it is");
}

/// M3: form 1 becomes form 2 for the minted id, owner-only, and loads
/// with that entry's key PRESENT at filesystem custody; run again it
/// writes nothing and succeeds; run for another id it is refused naming
/// the one already there, form 2 left as it was.
#[test]
fn provision_identity_moves_form_1_to_form_2_once() {
    let (_root, app) = app();
    let paths = paths_in(&app);
    assert!(
        matches!(
            provision_identity(
                &paths,
                &EndpointId::parse_derived("12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr")
                    .expect("id")
            ),
            Err(ProvisionIdentityError::Absent)
        ),
        "nothing provisioned"
    );
    provision_embedded(&paths).expect("form 1");
    let id = mint_filesystem(&paths).expect("a key minted");
    provision_identity(&paths, &id).expect("form 2");
    assert_eq!(read(&paths), embedded_identity_document("work", &id));
    assert_eq!(
        std::fs::metadata(paths.config_file())
            .expect("present")
            .mode()
            & 0o777,
        0o600,
        "owner-only"
    );
    let loaded = ProfileConfig::load(&paths).expect("form 2 loads");
    let [entry] = loaded.endpoints.entries.as_slice() else {
        panic!("one entry");
    };
    assert_eq!(entry.id, id);
    assert_eq!(
        entry.key_state,
        EndpointKeyState::Present {
            custody: Custody::FilesystemOnly
        }
    );

    // Idempotent: nothing written the second time (the inode stays).
    let before = std::fs::metadata(paths.config_file())
        .expect("present")
        .ino();
    provision_identity(&paths, &id).expect("again: done");
    assert_eq!(
        std::fs::metadata(paths.config_file())
            .expect("present")
            .ino(),
        before,
        "not rewritten"
    );

    let other = mint_filesystem(&paths).expect("another key");
    match provision_identity(&paths, &other) {
        Err(ProvisionIdentityError::OtherIdentity { found }) => assert_eq!(found, id),
        other => panic!("refused naming the first identity: {other:?}"),
    }
    assert_eq!(
        read(&paths),
        embedded_identity_document("work", &id),
        "kept"
    );
}

/// M3 replaces only the app's own form 1: a document without the marker
/// is the person's, and an earlier release's is M1's to replace first.
#[test]
fn provision_identity_refuses_what_is_not_form_1() {
    let (_root, app) = app();
    let paths = paths_in(&app);
    let id = EndpointId::parse_derived("12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr")
        .expect("id");
    plant(&paths, "schema_version: 2\n");
    assert!(matches!(
        provision_identity(&paths, &id),
        Err(ProvisionIdentityError::NotEmbedded)
    ));
    assert_eq!(read(&paths), "schema_version: 2\n");
    plant(&paths, RETIRED_DOCUMENT_WORK);
    assert!(matches!(
        provision_identity(&paths, &id),
        Err(ProvisionIdentityError::Unrecognised)
    ));
    assert_eq!(read(&paths), RETIRED_DOCUMENT_WORK);
}

/// A legal profile name YAML would read as another scalar -- a null, a
/// boolean, a number -- still round-trips: the first-run document writes
/// it plain, and the name's `String` field takes the scalar's text, so
/// the profile loads under its own name. Pinned because a change of YAML
/// reader or of the field's type would break it silently.
#[test]
fn a_name_yaml_reads_as_another_scalar_round_trips() {
    for name in ["null", "true", "no", "on", "123", "0x1F", "1e3", "-1"] {
        let (_root, app) = app();
        let paths =
            ProfilePaths::resolve_embedded(name, TrustBoundary::new(&app).expect("a boundary"))
                .expect("a legal name");
        provision_embedded(&paths).expect("written");
        let config = ProfileConfig::load(&paths)
            .unwrap_or_else(|e| panic!("{name}: the provisioned profile loads: {e}"));
        assert_eq!(
            config.profile.map(|profile| profile.name).as_deref(),
            Some(name),
            "{name}"
        );
    }
}
