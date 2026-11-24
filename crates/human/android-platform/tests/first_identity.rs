// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The phone's first identity (`src/first_identity.rs`): Create and Restore
//! seal its endpoint key and write form 2 in the step that seals the
//! node's seed, and every start finishes what onboarding or an earlier
//! release left -- over real files in a private app directory, the real
//! custody and key-file paths, and stand-ins of the Keystore that bind
//! each seal to its alias and associated data (`ENDPOINTS.md` §Configured
//! endpoint set, the two forms and the three moves; ADR-0067 decision 4).

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::cast_possible_truncation
)]
#![cfg(unix)]

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use interweave_human_android_platform::first_identity::{self, Finished, FirstIdentityRefused};
use interweave_human_android_platform::onboarding::{self, Creation};
use interweave_human_android_platform::{
    Expected, KeyUnlockPolicy, ServiceHost, provision_profile, stand_in,
};
use interweave_human_store::identity_key::{
    self, CipherFailure, Custody, IdentityKeyFileError, KeyCipher, KeyFileRead,
};
use interweave_profile_config::endpoint_key::{self, encode_record, header_bytes};
use interweave_profile_config::provision::{embedded_document, embedded_identity_document};
use interweave_profile_config::{ProfileConfig, ProfileLock, ProfilePaths, TrustBoundary};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::EndpointId;
use interweave_transport_embedded::custody::{
    self, CipherFailure as SeedFailure, IV_LEN, KeyRef, SEALED_LEN, SEED_LEN, Sealed, Seed,
    SeedCipher,
};

mod common;

const PROFILE: &str = "human";
const GRACE: Duration = Duration::from_secs(1);

/// A keyed check over a key, the associated data and a ciphertext:
/// enough for a stand-in to refuse a body opened under another alias or
/// header.
fn tag(key: &[u8; 32], aad: &[u8], ct: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (i, b) in key.iter().chain(aad).chain(ct).enumerate() {
        out[i % 16] = out[i % 16].rotate_left(3) ^ b ^ (i as u8);
    }
    out
}

/// A Keystore stand-in for the endpoint keys: a new key per alias at each
/// seal, as the Keystore makes one, a keystream XOR for the secret and a
/// tag binding the header. `seals` counts the seals asked; `retired`
/// lists the aliases retired.
#[derive(Default)]
struct KeyKeystore {
    keys: Mutex<HashMap<String, [u8; 32]>>,
    seals: Mutex<u32>,
    retired: Mutex<Vec<String>>,
}

impl KeyKeystore {
    fn seals(&self) -> u32 {
        *self.seals.lock().unwrap()
    }
}

impl KeyCipher for KeyKeystore {
    fn custody(&self) -> Custody {
        Custody::Keystore
    }

    fn seal(&self, alias: &str, aad: &[u8], secret: &[u8; 32]) -> Result<Vec<u8>, CipherFailure> {
        let mut seals = self.seals.lock().unwrap();
        *seals += 1;
        let mut key = [0u8; 32];
        for (i, b) in key.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(37) ^ (*seals as u8) ^ alias.len() as u8;
        }
        self.keys.lock().unwrap().insert(alias.to_owned(), key);
        let mut body: Vec<u8> = secret.iter().zip(key).map(|(s, k)| s ^ k).collect();
        let t = tag(&key, aad, &body);
        body.extend(t);
        Ok(body)
    }

    fn open(
        &self,
        alias: &str,
        aad: &[u8],
        sealed: &[u8],
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, CipherFailure> {
        let key = *self
            .keys
            .lock()
            .unwrap()
            .get(alias)
            .ok_or(CipherFailure::KeyUnavailable)?;
        if sealed.len() != 48 {
            return Err(CipherFailure::Unauthentic);
        }
        let (ct, t) = sealed.split_at(32);
        if tag(&key, aad, ct) != t {
            return Err(CipherFailure::Unauthentic);
        }
        let mut secret = zeroize::Zeroizing::new([0u8; 32]);
        for (i, b) in secret.iter_mut().enumerate() {
            *b = ct[i] ^ key[i];
        }
        Ok(secret)
    }

    fn retire(&self, alias: &str) -> Result<(), CipherFailure> {
        self.keys.lock().unwrap().remove(alias);
        self.retired.lock().unwrap().push(alias.to_owned());
        Ok(())
    }
}

/// A Keystore stand-in for the seed, as tests/identity.rs's: one key per
/// alias, made at each seal.
#[derive(Default)]
struct SeedKeystore {
    keys: Mutex<HashMap<String, [u8; 32]>>,
}

impl SeedCipher for SeedKeystore {
    fn seal(&self, key: &KeyRef, aad: &[u8], seed: &Seed) -> Result<Sealed, SeedFailure> {
        let mut fresh = [0u8; 32];
        for (i, b) in fresh.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(29) ^ 0x5a;
        }
        self.keys.lock().unwrap().insert(key.alias(), fresh);
        let ct: Vec<u8> = seed
            .expose()
            .iter()
            .zip(fresh)
            .map(|(s, k)| s ^ k)
            .collect();
        let mut sealed = ct.clone();
        sealed.extend(tag(&fresh, aad, &ct));
        Ok(Sealed {
            iv: vec![7; IV_LEN],
            sealed,
        })
    }

    fn retire(&self, key: &KeyRef) -> Result<(), SeedFailure> {
        self.keys.lock().unwrap().remove(&key.alias());
        Ok(())
    }

    fn open(
        &self,
        key: &KeyRef,
        _iv: &[u8; IV_LEN],
        sealed: &[u8; SEALED_LEN],
        aad: &[u8],
    ) -> Result<Seed, SeedFailure> {
        let k = *self
            .keys
            .lock()
            .unwrap()
            .get(&key.alias())
            .ok_or(SeedFailure::KeyMissing)?;
        let (ct, t) = sealed.split_at(SEED_LEN);
        if tag(&k, aad, ct) != t {
            return Err(SeedFailure::Authentication);
        }
        let mut seed = [0u8; SEED_LEN];
        for (i, b) in seed.iter_mut().enumerate() {
            *b = ct[i] ^ k[i];
        }
        Ok(Seed::new(seed))
    }
}

struct App {
    _root: tempfile::TempDir,
    dir: std::path::PathBuf,
}

/// The app's data directory as a fresh install leaves it: nothing in it.
fn fresh_install() -> App {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let dir = root.path().join("app");
    std::fs::create_dir(&dir).expect("mkdir");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod");
    App { _root: root, dir }
}

fn paths(app: &App) -> ProfilePaths {
    ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b")).expect("p")
}

fn document(app: &App) -> String {
    std::fs::read_to_string(paths(app).config_file()).expect("the document")
}

/// The node's seed sealed, as a Create or an earlier release left it.
fn seal_a_seed(app: &App, seed: &SeedKeystore) {
    custody::provision(
        &paths(app),
        seed,
        &ProfileIdentity::generate(),
        KeyUnlockPolicy::default(),
    )
    .expect("the seed sealed");
}

/// `identity-keys/human.key` as a release before Stage 22 sealed it: a
/// header naming the label, the body sealed under the label's alias
/// binding that header. The id the key derives.
fn write_legacy_key(app: &App, keys: &KeyKeystore) -> EndpointId {
    let paths = paths(app);
    let secret = [0x42u8; 32];
    let public = ed25519_dalek::SigningKey::from_bytes(&secret)
        .verifying_key()
        .to_bytes();
    let header = header_bytes(endpoint_key::Custody::Keystore, b"human", &public).expect("header");
    let body = keys
        .seal(
            &identity_key::legacy_alias(PROFILE, "human"),
            &header,
            &secret,
        )
        .expect("sealed");
    let file = identity_key::legacy_key_path(&paths, "human");
    interweave_profile_config::persist::create_private_dir_within(
        file.parent().expect("a directory"),
        paths.boundary(),
    )
    .expect("identity-keys/");
    interweave_profile_config::create_private_exclusive_within(
        &file,
        encode_record(&header, &body).as_bytes(),
        paths.boundary(),
    )
    .expect("written");
    EndpointId::from_ed25519_public(&public)
}

/// The document every release before ADR-0067 wrote at first run, as
/// tests/service.rs's `earlier_release_document` has it.
fn earlier_release_document() -> String {
    format!(
        "{}: the embedded-android
# defaults. Trust, relays and AutoNAT servers arrive through the admin
# trust surface or onboarding, not here.
schema_version: 2
profile:
  name: {PROFILE}
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
",
        interweave_profile_config::provision::EMBEDDED_DOCUMENT_MARKER
    )
}

/// Form 2 naming `id`, which loads with `id` as the service's endpoint.
fn assert_form_2(app: &App, id: &EndpointId) {
    assert_eq!(
        document(app),
        embedded_identity_document(PROFILE, id),
        "form 2 naming the sealed key's id"
    );
    let config = ProfileConfig::load(&paths(app)).expect("form 2 loads");
    assert_eq!(config.runtime.android.endpoint.as_ref(), Some(id));
}

/// The acceptance's first test: a fresh install through Create -- M1 at
/// the app's provisioning, the words, the seal under the flow's lock --
/// ends with the first identity's key sealed under the Keystore as
/// `endpoint-keys/<id>.key` and the document in form 2 naming that id, in
/// the one step. The flow's lock is still the flow's after it. On
/// 73f5bc1e9 the seal left form 1 and no key, which every start refused
/// (`AndroidNoIdentity`). Mutation: the seal's M3 call dropped leaves
/// form 1 and fails here.
#[test]
fn create_on_a_fresh_install_seals_the_first_key_and_writes_form_2_naming_it() {
    let app = fresh_install();
    let (seed, keys) = (SeedKeystore::default(), KeyKeystore::default());
    provision_profile(&app.dir, PROFILE).expect("M1");
    assert_eq!(document(&app), embedded_document(PROFILE), "the control");
    let lock = onboarding::hold(&app.dir, PROFILE).expect("held");
    let (creation, _, peer, _) = Creation::new().expect("a creation");

    let sealed =
        onboarding::seal(&app.dir, PROFILE, &seed, &keys, &creation, Some(&lock)).expect("sealed");

    let Ok(Finished::Minted(id)) = sealed.first_identity else {
        panic!("{sealed:?}");
    };
    assert_form_2(&app, &id);
    let paths = paths(&app);
    assert_eq!(
        identity_key::sealed(&paths).expect("listed"),
        vec![id.clone()]
    );
    let key = identity_key::open(&paths, &id, &keys)
        .expect("opens")
        .expect("there");
    assert_eq!(EndpointId::from_ed25519_public(&key.public_key()), id);
    assert_eq!(
        custody::recorded_identity(&paths).expect("read"),
        Some(peer),
        "the seed is the creation's"
    );
    assert!(
        ProfileLock::is_held(&paths).expect("asked"),
        "the flow's lock is still held"
    );
    drop(lock);
    assert!(
        !ProfileLock::is_held(&paths).expect("asked"),
        "and released"
    );
}

/// Restore onto fresh state brings no endpoint key back (ADR-0063
/// decision 1): its step mints the first identity's key as Create's does.
/// Mutation: restore's M3 call dropped fails here.
#[test]
fn a_restore_onto_fresh_state_mints_the_first_key_and_writes_form_2() {
    let app = fresh_install();
    let (seed, keys) = (SeedKeystore::default(), KeyKeystore::default());
    provision_profile(&app.dir, PROFILE).expect("M1");
    let (_, words, peer, _) = Creation::new().expect("an identity to restore");
    let restored = onboarding::restore(
        &app.dir,
        PROFILE,
        &seed,
        &keys,
        words.join(" "),
        Expected::Confirmed(&peer),
        None,
    )
    .expect("restored");
    let Ok(Finished::Minted(id)) = restored.first_identity else {
        panic!("{restored:?}");
    };
    assert_form_2(&app, &id);
    assert!(
        !ProfileLock::is_held(&paths(&app)).expect("asked"),
        "the lock M3 took is released"
    );
}

/// The acceptance's second test: a phone upgraded from a release before
/// Stage 22 -- its `id: human` document, its sealed seed and its key in
/// `identity-keys/human.key` -- STARTS: the start replaces the document
/// with form 1 (M2), re-seals the key under the id it derives, reads it
/// back, retires the old file and platform key, and writes form 2 naming
/// that id (M3), before the client loads the profile. On 73f5bc1e9 the
/// start answered `NoHumanEndpoint`, form 1 naming nothing. Mutations: the
/// start's M3 call dropped fails the start; `remove_legacy` dropped keeps
/// the old file.
#[test]
fn the_upgraded_id_human_document_starts_with_its_key_resealed_under_its_id() {
    let app = fresh_install();
    let (seed, keys) = (SeedKeystore::default(), Arc::new(KeyKeystore::default()));
    stand_in::provision_replacing(&app.dir, PROFILE, &earlier_release_document())
        .expect("the earlier release's document");
    seal_a_seed(&app, &seed);
    let id = write_legacy_key(&app, &keys);
    let legacy = identity_key::legacy_key_path(&paths(&app), "human");

    // What the app's start runs: the provisioning, then the start.
    provision_profile(&app.dir, PROFILE).expect("M2");
    let service = ServiceHost::new();
    service
        .start_without_network(&app.dir, PROFILE, keys.clone())
        .expect("the upgraded phone starts");
    let _ = service.stop(GRACE);

    assert_form_2(&app, &id);
    let paths = paths(&app);
    assert_eq!(
        identity_key::sealed(&paths).expect("listed"),
        vec![id.clone()]
    );
    let key = identity_key::open(&paths, &id, &*keys)
        .expect("opens under its id")
        .expect("there");
    assert_eq!(EndpointId::from_ed25519_public(&key.public_key()), id);
    assert!(!legacy.exists(), "the earlier release's file is removed");
    assert!(
        keys.retired
            .lock()
            .unwrap()
            .contains(&identity_key::legacy_alias(PROFILE, "human")),
        "and its platform key retired"
    );
}

/// A re-seal interrupted after its new file was written and before the
/// old one went: the next start finds both, mints nothing, seals nothing
/// again, and finishes -- the old file retired, form 2 written. Mutation:
/// `install`'s `Exists` answered as a refusal fails here.
#[test]
fn an_interrupted_reseal_is_finished_and_mints_nothing() {
    let app = fresh_install();
    let (seed, keys) = (SeedKeystore::default(), KeyKeystore::default());
    provision_profile(&app.dir, PROFILE).expect("M1");
    seal_a_seed(&app, &seed);
    let id = write_legacy_key(&app, &keys);
    let paths = paths(&app);
    let old = identity_key::open_legacy(&paths, "human", &keys)
        .expect("opens")
        .expect("there");
    identity_key::install(&paths, &old, &keys).expect("the first half");
    let seals = keys.seals();

    assert_eq!(
        first_identity::at_start(&app.dir, PROFILE, &keys),
        Ok(Finished::Resealed(id.clone()))
    );
    assert_eq!(keys.seals(), seals, "nothing sealed again");
    assert_eq!(
        identity_key::sealed(&paths).expect("listed"),
        vec![id.clone()]
    );
    assert!(!identity_key::legacy_key_path(&paths, "human").exists());
    assert_form_2(&app, &id);
}

/// A re-sealed key that does not open to the earlier key is not taken for
/// it: the earlier file stays, form 1 stays, and the start says why.
/// Here the file at the earlier key's id is another key's, as a hand-made
/// file could be, so the install answers `Exists` and the read-back is
/// what refuses. Mutation: the read-back dropped (an `Exists` taken as
/// done) answers `Resealed` and removes the earlier file.
#[test]
fn a_reseal_whose_read_back_fails_keeps_the_earlier_key() {
    let app = fresh_install();
    let (seed, keys) = (SeedKeystore::default(), KeyKeystore::default());
    provision_profile(&app.dir, PROFILE).expect("M1");
    seal_a_seed(&app, &seed);
    let id = write_legacy_key(&app, &keys);
    let paths = paths(&app);
    // Another key's record at the earlier key's name: its header names
    // the name's id, so the read-back's open gets as far as the keys.
    let other = [0x17u8; 32];
    let public = ed25519_dalek::SigningKey::from_bytes(&other)
        .verifying_key()
        .to_bytes();
    let header = header_bytes(
        endpoint_key::Custody::Keystore,
        id.as_str().as_bytes(),
        &public,
    )
    .expect("header");
    let body = keys
        .seal(&identity_key::alias(PROFILE, &id), &header, &other)
        .expect("sealed");
    interweave_profile_config::persist::create_private_dir_within(
        &paths.state_dir().join(identity_key::ENDPOINT_KEYS_DIR),
        paths.boundary(),
    )
    .expect("endpoint-keys/");
    interweave_profile_config::create_private_exclusive_within(
        &identity_key::key_path(&paths, &id),
        encode_record(&header, &body).as_bytes(),
        paths.boundary(),
    )
    .expect("written");

    assert!(matches!(
        first_identity::at_start(&app.dir, PROFILE, &keys),
        Err(FirstIdentityRefused::Key(_) | FirstIdentityRefused::ReadBack)
    ));
    assert!(
        identity_key::legacy_key_path(&paths, "human").exists(),
        "the earlier key is kept"
    );
    assert_eq!(document(&app), embedded_document(PROFILE), "form 1 stays");
}

/// Form 1 and no seed record -- the app's provisioning ran, the person
/// has not chosen Create or Restore: no key is minted, nothing written.
/// Mutation: the seed check dropped mints here.
#[test]
fn form_1_with_no_seed_record_mints_nothing() {
    let app = fresh_install();
    let keys = KeyKeystore::default();
    provision_profile(&app.dir, PROFILE).expect("M1");
    assert_eq!(
        first_identity::at_start(&app.dir, PROFILE, &keys),
        Ok(Finished::NoSeed)
    );
    assert_eq!(keys.seals(), 0, "nothing sealed");
    assert!(
        !paths(&app)
            .state_dir()
            .join(identity_key::ENDPOINT_KEYS_DIR)
            .exists(),
        "no endpoint-keys/"
    );
    assert_eq!(document(&app), embedded_document(PROFILE));
}

/// Form 1 beside a seed and no key -- a Create whose mint failed after
/// its seed seal: the next start mints the key and writes form 2, the
/// ruling's recovery for a mint left undone. Mutation: the mint arm
/// answering `LeftToHost` fails here.
#[test]
fn form_1_beside_a_seed_and_no_key_mints_at_the_start() {
    let app = fresh_install();
    let (seed, keys) = (SeedKeystore::default(), KeyKeystore::default());
    provision_profile(&app.dir, PROFILE).expect("M1");
    seal_a_seed(&app, &seed);
    let Ok(Finished::Minted(id)) = first_identity::at_start(&app.dir, PROFILE, &keys) else {
        panic!("minted");
    };
    assert_eq!(keys.seals(), 1, "one key");
    assert_form_2(&app, &id);
}

/// Form 1 beside one sealed key and no earlier release's -- a mint whose
/// form-2 write failed: the start writes nothing and mints nothing; the
/// embedded host's start finishes M3 with that key. Mutation: the
/// `sealed` check dropped mints a second key here.
#[test]
fn a_sealed_key_beside_form_1_is_left_to_the_host() {
    let app = fresh_install();
    let (seed, keys) = (SeedKeystore::default(), KeyKeystore::default());
    provision_profile(&app.dir, PROFILE).expect("M1");
    seal_a_seed(&app, &seed);
    let paths = paths(&app);
    let id = identity_key::create(&paths, &keys)
        .expect("a key")
        .endpoint()
        .clone();
    let before = std::fs::read(identity_key::key_path(&paths, &id)).expect("the key file");
    assert_eq!(
        first_identity::at_start(&app.dir, PROFILE, &keys),
        Ok(Finished::LeftToHost)
    );
    assert_eq!(keys.seals(), 1, "nothing sealed by the start");
    assert_eq!(
        identity_key::sealed(&paths).expect("listed"),
        vec![id.clone()]
    );
    assert_eq!(
        std::fs::read(identity_key::key_path(&paths, &id)).expect("the key file"),
        before
    );
    assert_eq!(document(&app), embedded_document(PROFILE), "form 1 stays");
}

/// Form 2: the start reads no key and writes nothing. Mutation: the form-1
/// check dropped answers `LeftToHost` here.
#[test]
fn form_2_is_left_as_it_is() {
    let app = fresh_install();
    let (seed, keys) = (SeedKeystore::default(), KeyKeystore::default());
    let id = common::provision_first_identity(&app.dir, PROFILE);
    seal_a_seed(&app, &seed);
    let before = document(&app);
    assert_eq!(
        first_identity::at_start(&app.dir, PROFILE, &keys),
        Ok(Finished::NotForm1)
    );
    assert_eq!(keys.seals(), 0);
    assert_eq!(document(&app), before);
    assert_eq!(
        identity_key::sealed(&paths(&app)).expect("listed"),
        vec![id]
    );
}

/// The lock [`first_identity::finish`] takes is released on every path,
/// a refusal's included, and a lock another holds is waited for by
/// nobody: nothing is tried. Mutation: the lock kept past the return
/// (`std::mem::forget`) fails the first acquire.
#[test]
fn the_lock_finish_takes_is_released_on_every_path() {
    let keys = KeyKeystore::default();
    let released = |app: &App| {
        drop(ProfileLock::acquire(&paths(app), Duration::ZERO).expect("released"));
    };
    // NoSeed.
    let app = fresh_install();
    provision_profile(&app.dir, PROFILE).expect("M1");
    assert_eq!(
        first_identity::at_start(&app.dir, PROFILE, &keys),
        Ok(Finished::NoSeed)
    );
    released(&app);
    // A refusal: endpoint-keys/ open to the group.
    seal_a_seed(&app, &SeedKeystore::default());
    let dir = paths(&app)
        .state_dir()
        .join(identity_key::ENDPOINT_KEYS_DIR);
    std::fs::create_dir(&dir).expect("mkdir");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o750)).expect("chmod");
    assert_eq!(
        first_identity::at_start(&app.dir, PROFILE, &keys),
        Err(FirstIdentityRefused::Key(IdentityKeyFileError::Read(
            KeyFileRead::DirUnsafe
        )))
    );
    released(&app);
    // Minted.
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod");
    assert!(matches!(
        first_identity::at_start(&app.dir, PROFILE, &keys),
        Ok(Finished::Minted(_))
    ));
    released(&app);
    // NotForm1.
    assert_eq!(
        first_identity::at_start(&app.dir, PROFILE, &keys),
        Ok(Finished::NotForm1)
    );
    released(&app);
    // Held: another holder, nothing tried.
    let app = fresh_install();
    provision_profile(&app.dir, PROFILE).expect("M1");
    seal_a_seed(&app, &SeedKeystore::default());
    let lock = onboarding::hold(&app.dir, PROFILE).expect("held");
    assert_eq!(
        first_identity::at_start(&app.dir, PROFILE, &keys),
        Ok(Finished::Held)
    );
    assert_eq!(keys.seals(), 1, "the earlier mint's alone");
    assert_eq!(document(&app), embedded_document(PROFILE));
    drop(lock);
}
