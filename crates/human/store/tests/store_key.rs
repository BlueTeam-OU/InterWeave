// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The store key's record and file (ADR-0060 decisions 2 and 3).
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;
#[path = "common/keystore.rs"]
mod keystore;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use interweave_human_store::identity_key::{self, IdentityKey};
use interweave_human_store::key_custody::{CipherFailure, FilesystemCustody};
use interweave_human_store::store_key::{
    self, STORE_KEY_FILE, StoreKey, StoreKeyError, StoreKeyFileError,
};
use interweave_profile_config::TrustBoundary;
use interweave_profile_config::paths::ProfilePaths;
use interweave_transport_api::EndpointId;
use keystore::SoftKeystore;

fn paths() -> (tempfile::TempDir, ProfilePaths) {
    let dir = common::private_tempdir();
    let boundary = TrustBoundary::new(dir.path()).expect("a boundary");
    let paths = ProfilePaths::resolve_embedded("default", boundary).expect("paths");
    (dir, paths)
}

fn edit(line: &str, f: impl FnOnce(&mut Vec<u8>)) -> String {
    let mut bytes = STANDARD.decode(line).expect("base64");
    f(&mut bytes);
    STANDARD.encode(bytes)
}

/// The two literals of one key, compared without printing either.
fn same(a: &StoreKey, b: &StoreKey) -> bool {
    format!("{a:?}") == format!("{b:?}") && {
        let (x, y) = (
            a.to_record("p", &FilesystemCustody),
            b.to_record("p", &FilesystemCustody),
        );
        x.expect("record").as_str() == y.expect("record").as_str()
    }
}

#[test]
fn a_desktop_record_round_trips_and_its_file_is_owner_only_beside_the_store_directory() {
    let (_dir, paths) = paths();
    let key = store_key::create(&paths, &FilesystemCustody).expect("created");
    let path = store_key::key_path(&paths);
    // Outside the store's own directory, so a copy of it leaves the key
    // behind, and not in the daemon's identity directory.
    assert_eq!(path, paths.state_dir().join(STORE_KEY_FILE));
    assert!(!path.starts_with(paths.human_dir()), "{}", path.display());
    assert!(
        !path.starts_with(paths.identity_dir()),
        "{}",
        path.display()
    );
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&path).expect("stat").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    let opened = store_key::open(&paths, &FilesystemCustody)
        .expect("opens")
        .expect("a key");
    assert!(same(&key, &opened));
}

#[test]
fn a_second_create_is_refused_and_the_first_key_survives() {
    let (_dir, paths) = paths();
    let key = store_key::create(&paths, &FilesystemCustody).expect("created");
    assert_eq!(
        store_key::create(&paths, &FilesystemCustody).map(|_| ()),
        Err(StoreKeyFileError::Exists)
    );
    let opened = store_key::open(&paths, &FilesystemCustody)
        .unwrap()
        .unwrap();
    assert!(same(&key, &opened));
}

#[test]
fn no_file_is_no_key_and_not_an_error() {
    let (_dir, paths) = paths();
    assert!(!store_key::exists(&paths).unwrap());
    assert!(
        store_key::open(&paths, &FilesystemCustody)
            .unwrap()
            .is_none()
    );
}

#[test]
fn two_keys_differ() {
    let (a, b) = (StoreKey::generate().unwrap(), StoreKey::generate().unwrap());
    assert!(!same(&a, &b));
}

#[test]
fn debug_prints_no_byte_of_the_key() {
    let key = StoreKey::generate().unwrap();
    assert_eq!(format!("{key:?}"), "StoreKey(..)");
}

#[test]
fn a_sealed_record_opens_under_its_own_alias_and_no_other() {
    let keystore = SoftKeystore::default();
    let key = StoreKey::generate().unwrap();
    let line = key.to_record("alice", &keystore).unwrap();
    let opened = StoreKey::from_record(&line, "alice", &keystore).unwrap();
    assert!(same(&key, &opened));
    assert!(
        keystore
            .used
            .borrow()
            .iter()
            .all(|a| a == "interweave.store.alice"),
        "{:?}",
        keystore.used.borrow()
    );
    // Another profile's alias holds no key.
    assert_eq!(
        StoreKey::from_record(&line, "bob", &keystore).map(|_| ()),
        Err(StoreKeyError::Cipher(CipherFailure::KeyUnavailable))
    );
}

#[test]
fn the_store_alias_is_apart_from_the_seed_and_every_identity() {
    let store = store_key::alias("alice");
    let identity = identity_key::alias("alice", &EndpointId::parse("human").unwrap());
    assert_eq!(store, "interweave.store.alice");
    assert!(!store.starts_with("interweave.iwk1."));
    assert!(!store.starts_with("interweave.idk1."));
    assert_ne!(store, identity);
}

#[test]
fn a_changed_header_does_not_authenticate() {
    let keystore = SoftKeystore::default();
    let line = StoreKey::generate()
        .unwrap()
        .to_record("p", &keystore)
        .unwrap();
    // The custody byte is the only header byte a record of this version
    // can change and still parse, so flip the body instead: AES-GCM
    // refuses either.
    let tampered = edit(&line, |b| {
        let last = b.len() - 1;
        b[last] ^= 1;
    });
    assert_eq!(
        StoreKey::from_record(&tampered, "p", &keystore).map(|_| ()),
        Err(StoreKeyError::Cipher(CipherFailure::Unauthentic))
    );
}

#[test]
fn an_invalidated_platform_key_is_told_apart() {
    let keystore = SoftKeystore::default();
    let line = StoreKey::generate()
        .unwrap()
        .to_record("p", &keystore)
        .unwrap();
    keystore.invalidate("interweave.store.p");
    let err = StoreKey::from_record(&line, "p", &keystore)
        .map(|_| ())
        .unwrap_err();
    assert_eq!(err, StoreKeyError::Cipher(CipherFailure::KeyInvalidated));
    assert!(err.is_invalidated());
    assert!(!StoreKeyError::Cipher(CipherFailure::KeyUnavailable).is_invalidated());
}

#[test]
fn records_of_another_kind_version_or_custody_are_refused_by_name() {
    let line = StoreKey::generate()
        .unwrap()
        .to_record("p", &FilesystemCustody)
        .unwrap();
    let open = |l: &str| StoreKey::from_record(l, "p", &FilesystemCustody).map(|_| ());
    assert_eq!(open(&line), Ok(()));
    assert_eq!(
        open(&edit(&line, |b| b[4] = 2)),
        Err(StoreKeyError::Version(2))
    );
    assert_eq!(
        open(&edit(&line, |b| b.truncate(3))),
        Err(StoreKeyError::NotARecord)
    );
    assert_eq!(open("not base64 at all!"), Err(StoreKeyError::NotARecord));
    // A sealed record on the desktop is unreadable here.
    assert_eq!(
        open(&edit(&line, |b| b[5] = 2)),
        Err(StoreKeyError::SealedUnreadableHere)
    );
    // A filesystem-only record where a wrapping key exists is a downgrade.
    assert_eq!(
        StoreKey::from_record(&line, "p", &SoftKeystore::default()).map(|_| ()),
        Err(StoreKeyError::CustodyDowngrade)
    );
    // An identity's IDK1 record is not a store key.
    let idk1 = IdentityKey::generate(EndpointId::parse("human").unwrap())
        .unwrap()
        .to_record("p", &FilesystemCustody)
        .unwrap();
    assert_eq!(open(&idk1), Err(StoreKeyError::NotARecord));
    // A short body is not 32 bytes of key.
    assert_eq!(
        open(&edit(&line, |b| b.truncate(6 + 31))),
        Err(StoreKeyError::Cipher(CipherFailure::Unauthentic))
    );
}

#[test]
fn remove_deletes_the_file_and_retires_the_alias() {
    let (_dir, paths) = paths();
    let keystore = SoftKeystore::default();
    store_key::create(&paths, &keystore).unwrap();
    assert!(keystore.holds("interweave.store.default"));
    assert_eq!(store_key::remove(&paths, &keystore), Ok(true));
    assert!(!store_key::exists(&paths).unwrap());
    assert!(!keystore.holds("interweave.store.default"));
    assert_eq!(*keystore.retired.borrow(), ["interweave.store.default"]);
    assert_eq!(store_key::remove(&paths, &keystore), Ok(false));
}

/// Under a sealing custody, a second create is refused BEFORE it seals:
/// the Keystore generates a new key at the alias on every seal, so a seal
/// first would have replaced the key the existing record needs (#377
/// review F2).
#[test]
fn a_second_create_under_the_keystore_leaves_the_first_record_openable() {
    let (_dir, paths) = paths();
    let keystore = SoftKeystore::default();
    let key = store_key::create(&paths, &keystore).expect("created");
    assert_eq!(
        store_key::create(&paths, &keystore).map(|_| ()),
        Err(StoreKeyFileError::Exists)
    );
    let opened = store_key::open(&paths, &keystore)
        .expect("the first record still opens")
        .expect("a key");
    let filesystem = |k: &StoreKey| k.to_record("p", &FilesystemCustody).unwrap();
    assert_eq!(filesystem(&key).as_str(), filesystem(&opened).as_str());
}
