// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identity a start takes (src/identity.rs), over the real custody
//! path and a fake of the platform's cipher: the first run provisions
//! under the profile's configured policy and every later start unlocks the
//! same `PeerId`; a record that cannot give its identity back is refused
//! for recovery, left as it was, and never replaced.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::cast_possible_truncation
)]
#![cfg(unix)]

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt as _;
use std::sync::Mutex;

use interweave_human_android_platform::{
    Expected, FirstRun, IdentityRefused, RestoreRefused, phrase_peer, restore_identity, stand_in,
    start_identity,
};
use interweave_profile_config::{ProfilePaths, TrustBoundary};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_embedded::custody::{
    self, CipherFailure, IV_LEN, KeyRef, RecoveryCause, SEALED_LEN, SEED_LEN, Sealed, Seed,
    SeedCipher,
};

const PROFILE: &str = "human";

/// A stand-in for the Keystore: one key per alias, a keystream XOR for
/// the seed and a keyed check over the associated data and ciphertext
/// for the tag -- enough for custody to tell a right key and AAD from a
/// wrong one. `fail` makes the next open fail as the platform would.
#[derive(Default)]
struct FakeKeystore {
    keys: Mutex<HashMap<String, [u8; 32]>>,
    generated: Mutex<Vec<String>>,
    fail: Mutex<Option<CipherFailure>>,
    retire_fails: Mutex<bool>,
}

fn tag(key: &[u8; 32], aad: &[u8], ct: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (i, b) in key.iter().chain(aad).chain(ct).enumerate() {
        out[i % 16] = out[i % 16].rotate_left(3) ^ b ^ (i as u8);
    }
    out
}

impl SeedCipher for FakeKeystore {
    fn seal(&self, key: &KeyRef, aad: &[u8], seed: &Seed) -> Result<Sealed, CipherFailure> {
        let alias = key.alias();
        let mut fresh = [0u8; 32];
        for (i, b) in fresh.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31) ^ (self.generated.lock().unwrap().len() as u8);
        }
        self.keys.lock().unwrap().insert(alias.clone(), fresh);
        self.generated.lock().unwrap().push(alias);
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

    fn retire(&self, key: &KeyRef) -> Result<(), CipherFailure> {
        if *self.retire_fails.lock().unwrap() {
            return Err(CipherFailure::Unavailable("busy".to_owned()));
        }
        self.keys.lock().unwrap().remove(&key.alias());
        Ok(())
    }

    fn open(
        &self,
        key: &KeyRef,
        _iv: &[u8; IV_LEN],
        sealed: &[u8; SEALED_LEN],
        aad: &[u8],
    ) -> Result<Seed, CipherFailure> {
        if let Some(failure) = self.fail.lock().unwrap().take() {
            return Err(failure);
        }
        let k = *self
            .keys
            .lock()
            .unwrap()
            .get(&key.alias())
            .ok_or(CipherFailure::KeyMissing)?;
        let (ct, t) = sealed.split_at(SEED_LEN);
        if tag(&k, aad, ct) != t {
            return Err(CipherFailure::Authentication);
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

fn app() -> App {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let dir = root.path().join("app");
    std::fs::create_dir(&dir).expect("mkdir");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod");
    stand_in::provision(&dir, PROFILE).expect("the profile");
    App { _root: root, dir }
}

fn record(app: &App) -> Vec<u8> {
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("p");
    std::fs::read(custody::custody_file(&paths)).expect("the record")
}

#[test]
fn the_first_run_provisions_and_every_later_start_unlocks_the_same_peer() {
    let app = app();
    let keystore = FakeKeystore::default();
    let first =
        start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("the first run");
    let peer = first.transport_identity().expect("a PeerId");
    assert_eq!(
        *keystore.generated.lock().unwrap(),
        vec![format!("interweave.iwk1.{PROFILE}.background-compatible")],
        "one key, under the profile's configured policy (the default)"
    );
    let stored = record(&app);
    for _ in 0..2 {
        let again =
            start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).expect("a later start");
        assert_eq!(again.transport_identity().expect("a PeerId"), peer);
    }
    assert_eq!(
        keystore.generated.lock().unwrap().len(),
        1,
        "no later start seals"
    );
    assert_eq!(record(&app), stored, "no later start rewrites the record");
}

#[test]
fn a_record_that_cannot_give_its_identity_back_is_refused_left_as_it_was_and_never_replaced() {
    let app = app();
    let keystore = FakeKeystore::default();
    let peer = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create)
        .expect("provisioned")
        .transport_identity()
        .expect("a PeerId");
    let stored = record(&app);
    for (failure, refused) in [
        (
            CipherFailure::KeyInvalidated,
            IdentityRefused::RecoveryRequired(RecoveryCause::KeyInvalidated),
        ),
        (
            CipherFailure::KeyMissing,
            IdentityRefused::RecoveryRequired(RecoveryCause::KeyMissing),
        ),
        (
            CipherFailure::Authentication,
            IdentityRefused::RecoveryRequired(RecoveryCause::Authentication),
        ),
        (
            CipherFailure::UserNotAuthenticated,
            IdentityRefused::UserNotAuthenticated,
        ),
        (
            CipherFailure::Unavailable("busy".to_owned()),
            IdentityRefused::Unavailable("busy".to_owned()),
        ),
    ] {
        *keystore.fail.lock().unwrap() = Some(failure.clone());
        assert_eq!(
            start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).map(|_| ()),
            Err(refused),
            "{failure:?}"
        );
        assert_eq!(
            record(&app),
            stored,
            "{failure:?} leaves the record as it was"
        );
        assert_eq!(
            keystore.generated.lock().unwrap().len(),
            1,
            "{failure:?} seals nothing"
        );
    }
    let again = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create)
        .expect("the key answers again");
    assert_eq!(again.transport_identity().expect("a PeerId"), peer);
}

#[test]
fn a_record_a_new_key_does_not_open_is_recovery_never_a_second_identity() {
    let app = app();
    start_identity(
        &app.dir,
        PROFILE,
        &FakeKeystore::default(),
        FirstRun::Create,
    )
    .expect("provisioned");
    // A keystore that holds none of this record's keys: the app reinstalled
    // over its old data, or the key deleted under it.
    let empty = FakeKeystore::default();
    assert_eq!(
        start_identity(&app.dir, PROFILE, &empty, FirstRun::Create).map(|_| ()),
        Err(IdentityRefused::RecoveryRequired(RecoveryCause::KeyMissing))
    );
    assert!(
        empty.generated.lock().unwrap().is_empty(),
        "nothing generated"
    );
}

#[test]
fn a_build_that_refuses_first_runs_answers_unprovisioned_and_writes_nothing() {
    let app = app();
    let keystore = FakeKeystore::default();
    assert_eq!(
        start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse).map(|_| ()),
        Err(IdentityRefused::Unprovisioned)
    );
    assert!(
        keystore.generated.lock().unwrap().is_empty(),
        "nothing generated"
    );
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("p");
    assert!(!custody::custody_file(&paths).exists(), "no record written");
}

#[test]
fn a_first_run_whose_old_key_could_not_be_retired_is_provisioned_and_starts() {
    let app = app();
    let keystore = FakeKeystore::default();
    *keystore.retire_fails.lock().unwrap() = true;
    let first = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create)
        .expect("the record is written, so the first run starts");
    let peer = first.transport_identity().expect("a PeerId");
    *keystore.retire_fails.lock().unwrap() = false;
    let again = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse).expect("unlocks");
    assert_eq!(again.transport_identity().expect("a PeerId"), peer);
}

/// Plan section 20 gate (a), the client clauses: a key the platform lost
/// enters recovery, mints nothing, and the person's phrase restores the
/// `PeerId` the record names -- which then unlocks at every later start.
#[test]
fn recovery_restores_the_recorded_peer_from_the_phrase_and_it_unlocks_after() {
    let app = app();
    let identity = start_identity(
        &app.dir,
        PROFILE,
        &FakeKeystore::default(),
        FirstRun::Create,
    )
    .expect("provisioned");
    let peer = identity.transport_identity().expect("a PeerId");
    let phrase = identity.recovery_phrase().expect("its phrase");
    // The keys are gone: the app reinstalled over its data, or the key
    // deleted under it.
    let keystore = FakeKeystore::default();
    assert_eq!(
        start_identity(&app.dir, PROFILE, &keystore, FirstRun::Create).map(|_| ()),
        Err(IdentityRefused::RecoveryRequired(RecoveryCause::KeyMissing))
    );

    let restored = restore_identity(&app.dir, PROFILE, &keystore, &phrase, Expected::Recorded)
        .expect("the phrase restores");
    assert_eq!(restored.transport_identity().expect("a PeerId"), peer);
    let again = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse)
        .expect("the restored record unlocks");
    assert_eq!(again.transport_identity().expect("a PeerId"), peer);
}

/// BIP-39's checksum is eight bits: a phrase that parses can be another
/// identity's. It is refused and the record stays as it was, so the
/// person's own phrase still restores it.
#[test]
fn another_identitys_phrase_is_refused_and_the_record_is_left_as_it_was() {
    let app = app();
    let identity = start_identity(
        &app.dir,
        PROFILE,
        &FakeKeystore::default(),
        FirstRun::Create,
    )
    .expect("provisioned");
    let before = record(&app);
    let keystore = FakeKeystore::default();
    let stranger = ProfileIdentity::generate()
        .recovery_phrase()
        .expect("a phrase");

    assert_eq!(
        restore_identity(&app.dir, PROFILE, &keystore, &stranger, Expected::Recorded).map(|_| ()),
        Err(RestoreRefused::OtherIdentity)
    );
    assert_eq!(record(&app), before, "the record is untouched");
    assert!(keystore.generated.lock().unwrap().is_empty(), "no key made");
    // Control: the right phrase still restores over the same record.
    restore_identity(
        &app.dir,
        PROFILE,
        &keystore,
        &identity.recovery_phrase().expect("its phrase"),
        Expected::Recorded,
    )
    .expect("the person's own phrase restores");
}

/// A fresh install has no record to say which `PeerId` to restore, and
/// none is taken on trust: nothing is written.
#[test]
fn with_no_record_nothing_is_restored_without_a_confirmed_peer() {
    let app = app();
    let keystore = FakeKeystore::default();
    let phrase = ProfileIdentity::generate()
        .recovery_phrase()
        .expect("a phrase");

    assert_eq!(
        restore_identity(&app.dir, PROFILE, &keystore, &phrase, Expected::Recorded).map(|_| ()),
        Err(RestoreRefused::NoRecord)
    );
    assert!(
        !custody::custody_file(&paths(&app)).exists(),
        "no record written"
    );
    assert!(keystore.generated.lock().unwrap().is_empty(), "no key made");
}

/// The fresh install's restore: the app shows the `PeerId` the phrase
/// derives to, the person confirms it, and that is what is restored and
/// unlocks after. A confirmation of another `PeerId` restores nothing.
#[test]
fn a_fresh_install_restores_the_peer_the_person_confirmed_and_no_other() {
    let app = app();
    let keystore = FakeKeystore::default();
    let original = ProfileIdentity::generate();
    let phrase = original.recovery_phrase().expect("a phrase");
    let shown = phrase_peer(&phrase).expect("the derived PeerId");
    assert_eq!(shown, original.transport_identity().expect("a PeerId"));

    let other = ProfileIdentity::generate()
        .transport_identity()
        .expect("a PeerId");
    assert_eq!(
        restore_identity(
            &app.dir,
            PROFILE,
            &keystore,
            &phrase,
            Expected::Confirmed(&other)
        )
        .map(|_| ()),
        Err(RestoreRefused::OtherIdentity)
    );
    assert!(
        !custody::custody_file(&paths(&app)).exists(),
        "no record written"
    );

    let restored = restore_identity(
        &app.dir,
        PROFILE,
        &keystore,
        &phrase,
        Expected::Confirmed(&shown),
    )
    .expect("the confirmed PeerId restores");
    assert_eq!(restored.transport_identity().expect("a PeerId"), shown);
    let again = start_identity(&app.dir, PROFILE, &keystore, FirstRun::Refuse)
        .expect("the new record unlocks");
    assert_eq!(again.transport_identity().expect("a PeerId"), shown);
}

/// A confirmed `PeerId` never stands in for a record: over an established
/// identity, even its own confirmed `PeerId` is refused and the record is
/// left as it was -- that profile restores as its record names it.
#[test]
fn a_confirmed_peer_never_replaces_an_established_record() {
    let app = app();
    let identity = start_identity(
        &app.dir,
        PROFILE,
        &FakeKeystore::default(),
        FirstRun::Create,
    )
    .expect("provisioned");
    let before = record(&app);
    let phrase = identity.recovery_phrase().expect("its phrase");
    let peer = identity.transport_identity().expect("a PeerId");

    assert_eq!(
        restore_identity(
            &app.dir,
            PROFILE,
            &FakeKeystore::default(),
            &phrase,
            Expected::Confirmed(&peer)
        )
        .map(|_| ()),
        Err(RestoreRefused::RecordExists)
    );
    assert_eq!(record(&app), before, "the record is untouched");
}

fn paths(app: &App) -> ProfilePaths {
    ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b")).expect("p")
}
