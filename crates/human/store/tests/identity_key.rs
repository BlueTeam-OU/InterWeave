// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An identity's own key (ADR-0063 decisions 1 and 2, A 2026-10-22):
//! generated from the CSPRNG and never from anything else, held in an
//! IDK1 record whose fields refuse another record in its place, at a
//! custody floor that is never a choice, in a file beside the store and
//! never inside it, deleted with its platform key on Remove.
//!
//! The Keystore's half is a software AES-256-GCM cipher here, holding the
//! custody Android's `AndroidKeyStore` key does; the JNI half and the
//! phone run are device rows.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::cell::RefCell;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt as _;

use aes_gcm::aead::{Aead as _, KeyInit as _, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use interweave_human_store::identity_key::{
    self, CipherFailure, Custody, FilesystemCustody, IdentityKey, IdentityKeyCipher,
    IdentityKeyError, IdentityKeyFileError,
};
use interweave_profile_config::TrustBoundary;
use interweave_profile_config::paths::ProfilePaths;
use interweave_transport_api::EndpointId;
use zeroize::Zeroizing;

fn endpoint(name: &str) -> EndpointId {
    EndpointId::parse(name).expect("an endpoint")
}

/// A software stand-in for the Keystore: one AES-256-GCM key per alias,
/// the IV beside the ciphertext, the header as associated data. It
/// records every alias it is asked for and every one it retires.
#[derive(Default)]
struct SoftKeystore {
    keys: RefCell<HashMap<String, [u8; 32]>>,
    used: RefCell<Vec<String>>,
    retired: RefCell<Vec<String>>,
}

impl IdentityKeyCipher for SoftKeystore {
    fn custody(&self) -> Custody {
        Custody::Keystore
    }

    fn seal(&self, alias: &str, aad: &[u8], secret: &[u8; 32]) -> Result<Vec<u8>, CipherFailure> {
        self.used.borrow_mut().push(alias.to_owned());
        let key = *self
            .keys
            .borrow_mut()
            .entry(alias.to_owned())
            .or_insert_with(rand::random);
        let iv: [u8; 12] = rand::random();
        let sealed = Aes256Gcm::new(&key.into())
            .encrypt(Nonce::from_slice(&iv), Payload { msg: secret, aad })
            .map_err(|_| CipherFailure::Platform("seal".to_owned()))?;
        Ok([iv.as_slice(), &sealed].concat())
    }

    fn open(
        &self,
        alias: &str,
        aad: &[u8],
        sealed: &[u8],
    ) -> Result<Zeroizing<[u8; 32]>, CipherFailure> {
        self.used.borrow_mut().push(alias.to_owned());
        let key = *self
            .keys
            .borrow()
            .get(alias)
            .ok_or(CipherFailure::KeyUnavailable)?;
        let (iv, ct) = sealed
            .split_at_checked(12)
            .ok_or(CipherFailure::Unauthentic)?;
        let plain = Zeroizing::new(
            Aes256Gcm::new(&key.into())
                .decrypt(Nonce::from_slice(iv), Payload { msg: ct, aad })
                .map_err(|_| CipherFailure::Unauthentic)?,
        );
        let secret: [u8; 32] = plain
            .as_slice()
            .try_into()
            .map_err(|_| CipherFailure::Unauthentic)?;
        Ok(Zeroizing::new(secret))
    }

    fn retire(&self, alias: &str) -> Result<(), CipherFailure> {
        self.keys.borrow_mut().remove(alias);
        self.retired.borrow_mut().push(alias.to_owned());
        Ok(())
    }
}

/// Rewrite a record's bytes and give back its line.
fn edit_record(line: &str, f: impl FnOnce(&mut Vec<u8>)) -> String {
    let mut bytes = STANDARD.decode(line).expect("base64");
    f(&mut bytes);
    STANDARD.encode(bytes)
}

/// The offset of the public key in a record for `name`.
fn public_key_at(name: &str) -> usize {
    7 + name.len()
}

#[test]
fn every_key_comes_from_the_csprng_and_none_from_anything_the_identity_shares() {
    // IdentityKey::generate takes the endpoint and nothing else: no seed,
    // no phrase, no transport material is an input. And the same input
    // twice gives two keys, which a derivation from that input could not.
    let keys: Vec<[u8; 32]> = (0..16)
        .map(|_| {
            IdentityKey::generate(endpoint("human"))
                .expect("a key")
                .public_key()
        })
        .collect();
    let mut distinct = keys.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        keys.len(),
        "sixteen keys for one identity, all different"
    );
}

#[test]
fn a_record_refuses_another_record_in_its_place() {
    let key = IdentityKey::generate(endpoint("human")).expect("a key");
    let line = key
        .to_record("default", &FilesystemCustody)
        .expect("a record");
    let open = |l: &str, name: &str| {
        IdentityKey::from_record(l, "default", &endpoint(name), &FilesystemCustody)
            .map(|k| k.public_key())
    };
    assert_eq!(
        open(&line, "human"),
        Ok(key.public_key()),
        "the control opens"
    );

    // A seed record (the embedded host's IWK1) in its place.
    let seed = edit_record(&line, |b| b[..4].copy_from_slice(b"IWK1"));
    assert_eq!(open(&seed, "human"), Err(IdentityKeyError::NotARecord));
    // A version this build does not read.
    let v2 = edit_record(&line, |b| b[4] = 2);
    assert_eq!(
        open(&v2, "human"),
        Err(IdentityKeyError::UnsupportedVersion(2))
    );
    // Another identity's file renamed into this one's name.
    assert_eq!(open(&line, "work"), Err(IdentityKeyError::EndpointMismatch));
    // A public key that is not the secret's.
    let swapped = edit_record(&line, |b| b[public_key_at("human")] ^= 1);
    assert_eq!(open(&swapped, "human"), Err(IdentityKeyError::KeyMismatch));
    // And what is not a record at all.
    assert_eq!(
        open("not base64!", "human"),
        Err(IdentityKeyError::NotARecord)
    );
    let short = edit_record(&line, |b| b.truncate(10));
    assert_eq!(open(&short, "human"), Err(IdentityKeyError::Truncated));
    let custody = edit_record(&line, |b| b[5] = 9);
    assert_eq!(
        open(&custody, "human"),
        Err(IdentityKeyError::UnknownCustody)
    );
}

#[test]
fn a_sealed_record_binds_its_header_as_associated_data() {
    let keystore = SoftKeystore::default();
    let key = IdentityKey::generate(endpoint("human")).expect("a key");
    let line = key
        .to_record("default", &keystore)
        .expect("a sealed record");
    let back =
        IdentityKey::from_record(&line, "default", &endpoint("human"), &keystore).expect("opens");
    assert_eq!(back.public_key(), key.public_key());
    // The public key is in the clear header: changed, the body no longer
    // authenticates, before any key comparison could run.
    let swapped = edit_record(&line, |b| b[public_key_at("human")] ^= 1);
    assert_eq!(
        IdentityKey::from_record(&swapped, "default", &endpoint("human"), &keystore)
            .map(|k| k.public_key()),
        Err(IdentityKeyError::Cipher(CipherFailure::Unauthentic))
    );
}

#[test]
fn the_custody_byte_is_a_floor_never_a_choice() {
    let keystore = SoftKeystore::default();
    let key = IdentityKey::generate(endpoint("human")).expect("a key");
    // A filesystem-only record where a wrapping key exists: a downgrade.
    let plain = key
        .to_record("default", &FilesystemCustody)
        .expect("a record");
    assert_eq!(
        IdentityKey::from_record(&plain, "default", &endpoint("human"), &keystore)
            .map(|k| k.public_key()),
        Err(IdentityKeyError::CustodyDowngrade)
    );
    // Its custody byte raised to claim a seal it never had: the body is
    // no AES-GCM output, so it does not authenticate.
    let claimed = edit_record(&plain, |b| b[5] = 2);
    assert!(
        IdentityKey::from_record(&claimed, "default", &endpoint("human"), &keystore).is_err(),
        "a raised custody byte is not a seal"
    );
    // A sealed record on a desktop: unreadable here.
    let sealed = key
        .to_record("default", &keystore)
        .expect("a sealed record");
    assert_eq!(
        IdentityKey::from_record(&sealed, "default", &endpoint("human"), &FilesystemCustody)
            .map(|k| k.public_key()),
        Err(IdentityKeyError::SealedUnreadableHere)
    );
}

#[test]
fn the_platform_key_is_in_the_identity_keys_own_namespace() {
    let keystore = SoftKeystore::default();
    let key = IdentityKey::generate(endpoint("work")).expect("a key");
    let line = key.to_record("alice", &keystore).expect("a sealed record");
    IdentityKey::from_record(&line, "alice", &endpoint("work"), &keystore).expect("opens");
    {
        let used = keystore.used.borrow();
        assert!(
            used.iter().all(|a| a == "interweave.idk1.alice.work"),
            "{used:?}"
        );
    }
    // Another profile's alias does not open it.
    assert_eq!(
        IdentityKey::from_record(&line, "bob", &endpoint("work"), &keystore)
            .map(|k| k.public_key()),
        Err(IdentityKeyError::Cipher(CipherFailure::KeyUnavailable))
    );
}

fn desktop_paths() -> (tempfile::TempDir, ProfilePaths) {
    let dir = common::private_tempdir();
    let boundary = TrustBoundary::new(dir.path()).expect("a boundary");
    let paths = ProfilePaths::resolve_embedded("default", boundary).expect("paths");
    (dir, paths)
}

fn mode(path: &std::path::Path) -> u32 {
    std::fs::metadata(path).expect("stat").permissions().mode() & 0o777
}

#[test]
fn create_writes_an_owner_only_file_beside_the_store_and_never_over_a_key() {
    let (_dir, paths) = desktop_paths();
    let human = endpoint("human");
    let key = identity_key::create(&paths, &human, &FilesystemCustody).expect("created");
    let file = identity_key::key_path(&paths, &human);
    assert_eq!(
        file,
        paths.state_dir().join("identity-keys").join("human.key")
    );
    assert_eq!(mode(&file), 0o600, "the file is owner-only");
    assert_eq!(
        mode(file.parent().expect("dir")),
        0o700,
        "its directory too"
    );
    assert!(
        !file.starts_with(paths.human_dir()),
        "beside the store's directory, never inside it, so a discard of the store leaves it"
    );
    let line = std::fs::read_to_string(&file).expect("read");
    assert_eq!(
        IdentityKey::from_record(&line, "default", &human, &FilesystemCustody)
            .map(|k| k.public_key()),
        Ok(key.public_key())
    );

    // A second create is refused, and the first key is still the file's.
    assert_eq!(
        identity_key::create(&paths, &human, &FilesystemCustody).map(|k| k.public_key()),
        Err(IdentityKeyFileError::Exists)
    );
    assert_eq!(
        std::fs::read_to_string(&file).expect("read"),
        line,
        "unchanged"
    );
}

#[test]
fn the_store_directory_removed_and_made_again_leaves_every_key() {
    // What Discard and recreate does to the store's own directory (plan
    // §29 step 3's action) cannot reach the key, which is outside it.
    let (_dir, paths) = desktop_paths();
    let human = endpoint("human");
    let key = identity_key::create(&paths, &human, &FilesystemCustody).expect("created");
    let store = interweave_human_store::HumanStore::open_profile(
        &paths,
        interweave_human_store::StoreOptions::default(),
    )
    .expect("a store");
    drop(store);
    std::fs::remove_dir_all(paths.human_dir()).expect("the store's directory removed");
    drop(
        interweave_human_store::HumanStore::open_profile(
            &paths,
            interweave_human_store::StoreOptions::default(),
        )
        .expect("a new store"),
    );
    let line = std::fs::read_to_string(identity_key::key_path(&paths, &human))
        .expect("the key file stands");
    let back =
        IdentityKey::from_record(&line, "default", &human, &FilesystemCustody).expect("opens");
    assert_eq!(back.public_key(), key.public_key(), "the same key");
    assert_eq!(
        back.fingerprint(),
        key.fingerprint(),
        "and the same fingerprint"
    );
}

#[test]
fn remove_deletes_the_file_and_retires_the_platform_key() {
    let (_dir, paths) = desktop_paths();
    let keystore = SoftKeystore::default();
    let work = endpoint("work");
    identity_key::create(&paths, &work, &keystore).expect("created");
    let file = identity_key::key_path(&paths, &work);
    assert!(file.is_file());
    assert_eq!(identity_key::remove(&paths, &work, &keystore), Ok(true));
    assert!(!file.exists(), "the file is gone");
    assert_eq!(
        *keystore.retired.borrow(),
        vec!["interweave.idk1.default.work".to_owned()],
        "and its platform key retired"
    );
    assert!(
        keystore.keys.borrow().is_empty(),
        "no platform key left behind"
    );
    // Removing again is no error and finds nothing.
    assert_eq!(identity_key::remove(&paths, &work, &keystore), Ok(false));
}

#[test]
fn open_finds_no_key_where_none_was_made_and_refuses_a_file_others_can_read() {
    let (_dir, paths) = desktop_paths();
    let human = endpoint("human");
    assert_eq!(
        identity_key::open(&paths, &human, &FilesystemCustody).map(|k| k.is_some()),
        Ok(false),
        "no key file is no key, never an error"
    );
    identity_key::create(&paths, &human, &FilesystemCustody).expect("created");
    let file = identity_key::key_path(&paths, &human);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).expect("chmod");
    assert!(
        matches!(
            identity_key::open(&paths, &human, &FilesystemCustody),
            Err(IdentityKeyFileError::Persist(_))
        ),
        "a key file others can read is refused, not opened"
    );
}

#[test]
fn a_renamed_key_file_is_refused_at_open() {
    // human.key copied over work.key: the record names `human`.
    let (_dir, paths) = desktop_paths();
    let (human, work) = (endpoint("human"), endpoint("work"));
    identity_key::create(&paths, &human, &FilesystemCustody).expect("created");
    let from = identity_key::key_path(&paths, &human);
    let to = identity_key::key_path(&paths, &work);
    std::fs::copy(&from, &to).expect("copy");
    std::fs::set_permissions(&to, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    assert_eq!(
        identity_key::open(&paths, &work, &FilesystemCustody).map(|k| k.map(|k| k.public_key())),
        Err(IdentityKeyFileError::Key(
            IdentityKeyError::EndpointMismatch
        ))
    );
}

#[test]
fn a_key_file_refused_as_a_downgrade_is_left_as_it_was() {
    // A desktop's key file carried to a platform with a wrapping key:
    // refused at open, named, and never re-sealed there (ADR-0063
    // decision 2, A 2026-10-22).
    let (_dir, paths) = desktop_paths();
    let human = endpoint("human");
    identity_key::create(&paths, &human, &FilesystemCustody).expect("created");
    let file = identity_key::key_path(&paths, &human);
    let before = std::fs::read(&file).expect("read");
    let keystore = SoftKeystore::default();
    assert_eq!(
        identity_key::open(&paths, &human, &keystore).map(|k| k.is_some()),
        Err(IdentityKeyFileError::Key(
            IdentityKeyError::CustodyDowngrade
        ))
    );
    assert_eq!(std::fs::read(&file).expect("read"), before, "not re-sealed");
    assert!(
        keystore.used.borrow().is_empty(),
        "no wrapping key made for it"
    );
}
