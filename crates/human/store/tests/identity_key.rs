// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An identity's own key (ADR-0063 decisions 1 and 2, A 2026-10-22;
//! ADR-0067 decisions 1 to 5): generated from the CSPRNG and never from
//! anything else, its `EndpointId` the one its public key derives, held in
//! profile-config's IDK1 record whose fields refuse another record in its
//! place, at a custody floor that is never a choice, in a file of
//! `endpoint-keys/` beside the store and never inside it, deleted with its
//! platform key on Remove; and a record a release before Stage 22 sealed
//! under a label, read and brought over.
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
    self, CipherFailure, Custody, FilesystemCustody, IdentityKey, IdentityKeyError,
    IdentityKeyFileError, KeyCipher,
};
use interweave_profile_config::TrustBoundary;
use interweave_profile_config::endpoint_key;
use interweave_profile_config::paths::ProfilePaths;
use interweave_transport_api::EndpointId;
use zeroize::Zeroizing;

/// A software stand-in for the Keystore: one AES-256-GCM key per alias,
/// the IV beside the ciphertext, the header as associated data. It
/// records every alias it is asked for and every one it retires.
#[derive(Default)]
struct SoftKeystore {
    keys: RefCell<HashMap<String, [u8; 32]>>,
    used: RefCell<Vec<String>>,
    retired: RefCell<Vec<String>>,
}

impl KeyCipher for SoftKeystore {
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

/// A hand-made key: its secret, public key and the id that derives.
fn handmade() -> (Zeroizing<[u8; 32]>, [u8; 32], EndpointId) {
    let secret = Zeroizing::new(rand::random::<[u8; 32]>());
    let public = ed25519_dalek::SigningKey::from_bytes(&secret)
        .verifying_key()
        .to_bytes();
    let id = EndpointId::from_ed25519_public(&public);
    (secret, public, id)
}

/// A record's custody byte for the platform's custody.
fn record_custody(custody: Custody) -> endpoint_key::Custody {
    match custody {
        Custody::FilesystemOnly => endpoint_key::Custody::FilesystemOnly,
        Custody::Keystore => endpoint_key::Custody::Keystore,
    }
}

/// A record line written field by field: a header naming `name` over
/// `public` at `cipher`'s custody, and `secret` sealed by `cipher` under
/// `alias` with that header as associated data -- the way every release
/// wrote one, a release before Stage 22's naming a label.
fn record_of(
    cipher: &dyn KeyCipher,
    alias: &str,
    name: &[u8],
    public: &[u8; 32],
    secret: &[u8; 32],
) -> String {
    let header = endpoint_key::header_bytes(record_custody(cipher.custody()), name, public)
        .expect("a header");
    let body = cipher.seal(alias, &header, secret).expect("sealed");
    endpoint_key::encode_record(&header, &body).to_string()
}

/// Rewrite a record's bytes and give back its line.
fn edit_record(line: &str, f: impl FnOnce(&mut Vec<u8>)) -> String {
    let mut bytes = STANDARD.decode(line).expect("base64");
    f(&mut bytes);
    STANDARD.encode(bytes)
}

/// The offset of the public key in a record for `id`.
fn public_key_at(id: &EndpointId) -> usize {
    7 + id.as_str().len()
}

#[test]
fn every_key_comes_from_the_csprng_and_none_from_anything_the_identity_shares() {
    // IdentityKey::generate takes nothing: no seed, no phrase, no
    // transport material is an input. Sixteen calls give sixteen keys,
    // and each is named by the id its own public key derives.
    let keys: Vec<IdentityKey> = (0..16)
        .map(|_| IdentityKey::generate().expect("a key"))
        .collect();
    let mut distinct: Vec<[u8; 32]> = keys.iter().map(IdentityKey::public_key).collect();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), keys.len(), "sixteen keys, all different");
    for key in &keys {
        assert_eq!(
            key.endpoint(),
            &EndpointId::from_ed25519_public(&key.public_key())
        );
    }
}

#[test]
fn a_record_refuses_another_record_in_its_place() {
    let key = IdentityKey::generate().expect("a key");
    let id = key.endpoint().clone();
    let line = key
        .to_record("default", &FilesystemCustody)
        .expect("a record");
    let open = |l: &str, id: &EndpointId| {
        IdentityKey::from_record(l, "default", id, &FilesystemCustody).map(|k| k.public_key())
    };
    assert_eq!(open(&line, &id), Ok(key.public_key()), "the control opens");

    // A seed record (the embedded host's IWK1) in its place.
    let seed = edit_record(&line, |b| b[..4].copy_from_slice(b"IWK1"));
    assert_eq!(open(&seed, &id), Err(IdentityKeyError::NotARecord));
    // A version this build does not read.
    let v2 = edit_record(&line, |b| b[4] = 2);
    assert_eq!(open(&v2, &id), Err(IdentityKeyError::UnsupportedVersion(2)));
    // Another identity's file renamed into this one's name.
    let other = IdentityKey::generate().expect("another");
    assert_eq!(
        open(&line, other.endpoint()),
        Err(IdentityKeyError::EndpointMismatch)
    );
    // A public key that is not the secret's, under a header whose id that
    // key does derive: a body from another key.
    let (secret, _, _) = handmade();
    let foreign = record_of(
        &FilesystemCustody,
        "unused",
        id.as_str().as_bytes(),
        &key.public_key(),
        &secret,
    );
    assert_eq!(open(&foreign, &id), Err(IdentityKeyError::KeyMismatch));
    // And what is not a record at all.
    assert_eq!(open("not base64!", &id), Err(IdentityKeyError::NotARecord));
    let short = edit_record(&line, |b| b.truncate(10));
    assert_eq!(open(&short, &id), Err(IdentityKeyError::Truncated));
    let custody = edit_record(&line, |b| b[5] = 9);
    assert_eq!(open(&custody, &id), Err(IdentityKeyError::UnknownCustody));
}

/// ADR-0067 decision 2: the header's id must be the one the record's own
/// public key derives. A header naming the id asked for over another key
/// is refused as not derived, and the same record asked for by the id
/// its key does derive names another identity.
#[test]
fn a_header_whose_id_its_key_does_not_derive_is_refused() {
    let (secret, public, derived) = handmade();
    let named = IdentityKey::generate().expect("another").endpoint().clone();
    let line = record_of(
        &FilesystemCustody,
        "unused",
        named.as_str().as_bytes(),
        &public,
        &secret,
    );
    let open = |id: &EndpointId| {
        IdentityKey::from_record(&line, "default", id, &FilesystemCustody).map(|k| k.public_key())
    };
    assert_eq!(open(&named), Err(IdentityKeyError::NotDerived));
    assert_eq!(open(&derived), Err(IdentityKeyError::EndpointMismatch));
    // The control: the same key under its own id opens.
    let good = record_of(
        &FilesystemCustody,
        "unused",
        derived.as_str().as_bytes(),
        &public,
        &secret,
    );
    assert_eq!(
        IdentityKey::from_record(&good, "default", &derived, &FilesystemCustody)
            .map(|k| k.public_key()),
        Ok(public)
    );
    // A flipped byte of the public key is the same refusal: the id no
    // longer derives, before any key comparison runs.
    let flipped = edit_record(&good, |b| b[public_key_at(&derived)] ^= 1);
    assert_eq!(
        IdentityKey::from_record(&flipped, "default", &derived, &FilesystemCustody)
            .map(|k| k.public_key()),
        Err(IdentityKeyError::NotDerived)
    );
}

#[test]
fn a_sealed_record_binds_its_header_as_associated_data() {
    let keystore = SoftKeystore::default();
    let key = IdentityKey::generate().expect("a key");
    let id = key.endpoint().clone();
    let line = key
        .to_record("default", &keystore)
        .expect("a sealed record");
    let back = IdentityKey::from_record(&line, "default", &id, &keystore).expect("opens");
    assert_eq!(back.public_key(), key.public_key());
    // Every header byte is checked before the seal opens, so the binding
    // is shown with a body sealed under the right alias over OTHER
    // associated data: it does not authenticate under the record's header.
    let (secret, public, derived) = handmade();
    let alias = identity_key::alias("default", &derived);
    let real = endpoint_key::header_bytes(
        endpoint_key::Custody::Keystore,
        derived.as_str().as_bytes(),
        &public,
    )
    .expect("a header");
    let body = keystore.seal(&alias, &real, &secret).expect("sealed");
    let control = endpoint_key::encode_record(&real, &body).to_string();
    assert_eq!(
        IdentityKey::from_record(&control, "default", &derived, &keystore).map(|k| k.public_key()),
        Ok(public),
        "the control: sealed over its own header"
    );
    let mut other_aad = real.clone();
    other_aad.push(0);
    let body = keystore.seal(&alias, &other_aad, &secret).expect("sealed");
    let unbound = endpoint_key::encode_record(&real, &body).to_string();
    assert_eq!(
        IdentityKey::from_record(&unbound, "default", &derived, &keystore).map(|k| k.public_key()),
        Err(IdentityKeyError::Cipher(CipherFailure::Unauthentic))
    );
}

#[test]
fn the_custody_byte_is_a_floor_never_a_choice() {
    let keystore = SoftKeystore::default();
    let key = IdentityKey::generate().expect("a key");
    let id = key.endpoint().clone();
    // A filesystem-only record where a wrapping key exists: a downgrade.
    let plain = key
        .to_record("default", &FilesystemCustody)
        .expect("a record");
    assert_eq!(
        IdentityKey::from_record(&plain, "default", &id, &keystore).map(|k| k.public_key()),
        Err(IdentityKeyError::CustodyDowngrade)
    );
    // Its custody byte raised to claim a seal it never had: the body is
    // no AES-GCM output, so it does not authenticate.
    let claimed = edit_record(&plain, |b| b[5] = 2);
    assert!(
        IdentityKey::from_record(&claimed, "default", &id, &keystore).is_err(),
        "a raised custody byte is not a seal"
    );
    // A sealed record on a desktop: unreadable here.
    let sealed = key
        .to_record("default", &keystore)
        .expect("a sealed record");
    assert_eq!(
        IdentityKey::from_record(&sealed, "default", &id, &FilesystemCustody)
            .map(|k| k.public_key()),
        Err(IdentityKeyError::SealedUnreadableHere)
    );
}

#[test]
fn the_platform_key_is_in_the_identity_keys_own_namespace() {
    let keystore = SoftKeystore::default();
    let key = IdentityKey::generate().expect("a key");
    let id = key.endpoint().clone();
    let line = key.to_record("alice", &keystore).expect("a sealed record");
    IdentityKey::from_record(&line, "alice", &id, &keystore).expect("opens");
    {
        let used = keystore.used.borrow();
        let want = format!("interweave.idk1.alice.{}", id.as_str());
        assert!(used.iter().all(|a| *a == want), "{used:?}");
    }
    // Another profile's alias does not open it.
    assert_eq!(
        IdentityKey::from_record(&line, "bob", &id, &keystore).map(|k| k.public_key()),
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
fn create_writes_an_owner_only_file_in_endpoint_keys_beside_the_store() {
    let (_dir, paths) = desktop_paths();
    let key = identity_key::create(&paths, &FilesystemCustody).expect("created");
    let id = key.endpoint().clone();
    let file = identity_key::key_path(&paths, &id);
    assert_eq!(
        file,
        paths
            .state_dir()
            .join("endpoint-keys")
            .join(format!("{}.key", id.as_str()))
    );
    assert_eq!(
        file,
        endpoint_key::key_path(&paths, &id),
        "profile-config's"
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
        IdentityKey::from_record(&line, "default", &id, &FilesystemCustody).map(|k| k.public_key()),
        Ok(key.public_key())
    );
    // The daemon's reader reads the same file's public part.
    let header = endpoint_key::KeyHeader::from_line(&line).expect("a header");
    assert_eq!(header.endpoint(), &id);
    assert_eq!(header.public_key(), &key.public_key());
    // A second create is a second identity, never the first one's file.
    let second = identity_key::create(&paths, &FilesystemCustody).expect("another");
    assert_ne!(second.endpoint(), &id);
    assert_eq!(
        std::fs::read_to_string(&file).expect("read"),
        line,
        "unchanged"
    );
}

#[test]
fn create_then_open_round_trips_at_both_custodies() {
    let (_dir, paths) = desktop_paths();
    let key = identity_key::create(&paths, &FilesystemCustody).expect("created");
    let opened = identity_key::open(&paths, key.endpoint(), &FilesystemCustody)
        .expect("opens")
        .expect("a key");
    assert_eq!(opened.public_key(), key.public_key());
    assert_eq!(opened.endpoint(), key.endpoint());

    let (_dir, paths) = desktop_paths();
    let keystore = keystore::SoftKeystore::default();
    let key = identity_key::create(&paths, &keystore).expect("sealed");
    let file = identity_key::key_path(&paths, key.endpoint());
    assert_eq!(mode(&file), 0o600);
    let opened = identity_key::open(&paths, key.endpoint(), &keystore)
        .expect("opens")
        .expect("a key");
    assert_eq!(opened.public_key(), key.public_key());
    assert_eq!(opened.sign(b"m"), key.sign(b"m"));
}

#[test]
fn the_store_directory_removed_and_made_again_leaves_every_key() {
    // What Discard and recreate does to the store's own directory (plan
    // §29 step 3's action) cannot reach the key, which is outside it.
    let (_dir, paths) = desktop_paths();
    let key = identity_key::create(&paths, &FilesystemCustody).expect("created");
    let id = key.endpoint().clone();
    let store = interweave_human_store::HumanStore::open_profile(
        &paths,
        &FilesystemCustody,
        interweave_human_store::StoreOptions::default(),
    )
    .expect("a store");
    drop(store);
    std::fs::remove_dir_all(paths.human_dir()).expect("the store's directory removed");
    drop(
        interweave_human_store::HumanStore::open_profile(
            &paths,
            &FilesystemCustody,
            interweave_human_store::StoreOptions::default(),
        )
        .expect("a new store"),
    );
    let line =
        std::fs::read_to_string(identity_key::key_path(&paths, &id)).expect("the key file stands");
    let back = IdentityKey::from_record(&line, "default", &id, &FilesystemCustody).expect("opens");
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
    let work = identity_key::create(&paths, &keystore)
        .expect("created")
        .endpoint()
        .clone();
    let file = identity_key::key_path(&paths, &work);
    assert!(file.is_file());
    assert_eq!(identity_key::remove(&paths, &work, &keystore), Ok(true));
    assert!(!file.exists(), "the file is gone");
    assert_eq!(
        *keystore.retired.borrow(),
        vec![format!("interweave.idk1.default.{}", work.as_str())],
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
    let unmade = IdentityKey::generate().expect("a key").endpoint().clone();
    assert_eq!(
        identity_key::open(&paths, &unmade, &FilesystemCustody).map(|k| k.is_some()),
        Ok(false),
        "no key file is no key, never an error"
    );
    let human = identity_key::create(&paths, &FilesystemCustody)
        .expect("created")
        .endpoint()
        .clone();
    let file = identity_key::key_path(&paths, &human);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).expect("chmod");
    assert!(
        matches!(
            identity_key::open(&paths, &human, &FilesystemCustody),
            Err(IdentityKeyFileError::Read(
                identity_key::KeyFileRead::NotPrivate
            ))
        ),
        "a key file others can read is refused, not opened"
    );
}

#[test]
fn a_renamed_key_file_is_refused_at_open() {
    // One identity's file copied over another's name: the record names the
    // first.
    let (_dir, paths) = desktop_paths();
    let human = identity_key::create(&paths, &FilesystemCustody)
        .expect("created")
        .endpoint()
        .clone();
    let work = IdentityKey::generate().expect("a key").endpoint().clone();
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
    let human = identity_key::create(&paths, &FilesystemCustody)
        .expect("created")
        .endpoint()
        .clone();
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

/// A platform keystore whose retire fails, noting first whether the key
/// file was still there when it was asked.
struct RetireFails {
    inner: SoftKeystore,
    file: std::path::PathBuf,
    file_there_at_retire: RefCell<Option<bool>>,
}

impl KeyCipher for RetireFails {
    fn custody(&self) -> Custody {
        self.inner.custody()
    }

    fn seal(&self, alias: &str, aad: &[u8], secret: &[u8; 32]) -> Result<Vec<u8>, CipherFailure> {
        self.inner.seal(alias, aad, secret)
    }

    fn open(
        &self,
        alias: &str,
        aad: &[u8],
        sealed: &[u8],
    ) -> Result<Zeroizing<[u8; 32]>, CipherFailure> {
        self.inner.open(alias, aad, sealed)
    }

    fn retire(&self, _alias: &str) -> Result<(), CipherFailure> {
        *self.file_there_at_retire.borrow_mut() = Some(self.file.exists());
        Err(CipherFailure::Platform("retire".to_owned()))
    }
}

#[test]
fn remove_deletes_the_file_before_it_retires_the_platform_key() {
    // A retire that fails leaves a platform key that opens no record,
    // never a record whose platform key is gone.
    let (_dir, paths) = desktop_paths();
    let key = IdentityKey::generate().expect("a key");
    let work = key.endpoint().clone();
    let cipher = RetireFails {
        inner: SoftKeystore::default(),
        file: identity_key::key_path(&paths, &work),
        file_there_at_retire: RefCell::new(None),
    };
    identity_key::install(&paths, &key, &cipher).expect("installed");
    assert!(
        identity_key::remove(&paths, &work, &cipher).is_err(),
        "the failed retire is said"
    );
    assert_eq!(
        *cipher.file_there_at_retire.borrow(),
        Some(false),
        "the file went first"
    );
    assert!(!cipher.file.exists());
}

/// design/2026-10-24-identity-key.md section 3: every refusal at open has
/// its kind, by which the client says what happened and what it offers
/// (acceptance items 2, 14, 15, 16 and 18, the store's half).
#[test]
fn every_open_refusal_has_its_kind() {
    use identity_key::KeyUnusable as K;
    let kind = |paths: &ProfilePaths, id: &EndpointId, cipher: &dyn KeyCipher| {
        identity_key::open(paths, id, cipher)
            .expect_err("refused")
            .unusable()
    };
    let fresh = || {
        let (dir, paths) = desktop_paths();
        let id = identity_key::create(&paths, &FilesystemCustody)
            .expect("created")
            .endpoint()
            .clone();
        let file = identity_key::key_path(&paths, &id);
        (dir, paths, id, file)
    };
    let rewrite = |file: &std::path::Path, f: &dyn Fn(&mut Vec<u8>)| {
        let line = std::fs::read_to_string(file).expect("read");
        std::fs::write(file, edit_record(line.trim(), |b| f(b))).expect("write");
    };

    // Another identity's file in this one's place.
    let (_d, paths, id, file) = fresh();
    let work = identity_key::create(&paths, &FilesystemCustody)
        .expect("work")
        .endpoint()
        .clone();
    std::fs::copy(identity_key::key_path(&paths, &work), &file).expect("copy");
    assert_eq!(
        kind(&paths, &id, &FilesystemCustody),
        Some(K::Damaged),
        "another identity's"
    );
    // A header id its key does not derive: damage.
    let (_d, paths, id, file) = fresh();
    rewrite(&file, &|b| b[public_key_at(&id)] ^= 1);
    assert_eq!(
        identity_key::open(&paths, &id, &FilesystemCustody).map(|k| k.is_some()),
        Err(IdentityKeyFileError::Key(IdentityKeyError::NotDerived))
    );
    assert_eq!(kind(&paths, &id, &FilesystemCustody), Some(K::Damaged));

    // A version above this build's is a newer version's; below, damage.
    let (_d, paths, id, file) = fresh();
    rewrite(&file, &|b| b[4] = identity_key::VERSION + 1);
    assert_eq!(kind(&paths, &id, &FilesystemCustody), Some(K::NewerVersion));
    rewrite(&file, &|b| b[4] = 0);
    assert_eq!(
        kind(&paths, &id, &FilesystemCustody),
        Some(K::Damaged),
        "below"
    );
    // An unknown custody byte may be a newer version's.
    let (_d, paths, id, file) = fresh();
    rewrite(&file, &|b| b[5] = 9);
    assert_eq!(kind(&paths, &id, &FilesystemCustody), Some(K::NewerVersion));

    // Over the record bound, not text, unreadable by its owner: damage.
    let (_d, paths, id, file) = fresh();
    std::fs::write(&file, "A".repeat(64 * 1024)).expect("write");
    assert_eq!(
        kind(&paths, &id, &FilesystemCustody),
        Some(K::Damaged),
        "too large"
    );
    std::fs::write(&file, [0xff_u8, 0xfe, 0x00]).expect("write");
    assert_eq!(
        kind(&paths, &id, &FilesystemCustody),
        Some(K::Damaged),
        "not text"
    );
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    assert_eq!(
        kind(&paths, &id, &FilesystemCustody),
        Some(K::Damaged),
        "mode 0000"
    );

    // Readable by the group, or a key folder others can read: exposed.
    let (_d, paths, id, file) = fresh();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).expect("chmod");
    assert_eq!(
        kind(&paths, &id, &FilesystemCustody),
        Some(K::Exposed),
        "the file"
    );
    let (_d, paths, id, file) = fresh();
    let keys = file.parent().expect("the folder").to_owned();
    std::fs::set_permissions(&keys, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    assert_eq!(
        kind(&paths, &id, &FilesystemCustody),
        Some(K::Exposed),
        "the folder"
    );

    // The folder a link to a private folder, or a file: never followed.
    let (_d, paths, id, file) = fresh();
    let keys = file.parent().expect("the folder").to_owned();
    let elsewhere = keys.with_file_name("elsewhere");
    std::fs::rename(&keys, &elsewhere).expect("move");
    std::os::unix::fs::symlink(&elsewhere, &keys).expect("link");
    assert_eq!(
        kind(&paths, &id, &FilesystemCustody),
        Some(K::DirUnsafe),
        "a link"
    );
    std::fs::remove_file(&keys).expect("unlink");
    std::fs::write(&keys, "not a folder").expect("a file");
    assert_eq!(
        kind(&paths, &id, &FilesystemCustody),
        Some(K::DirUnsafe),
        "a file"
    );
    assert_eq!(
        std::fs::read(&keys).expect("read"),
        b"not a folder",
        "left as it is"
    );

    // Custody: below the platform's, sealed elsewhere, its key gone.
    let (_d, paths, id, _) = fresh();
    assert_eq!(
        kind(&paths, &id, &keystore::SoftKeystore::default()),
        Some(K::BelowDevice)
    );
    let (_d, paths) = desktop_paths();
    let soft = keystore::SoftKeystore::default();
    let id = identity_key::create(&paths, &soft)
        .expect("sealed")
        .endpoint()
        .clone();
    assert_eq!(kind(&paths, &id, &FilesystemCustody), Some(K::OtherDevice));
    soft.invalidate(&identity_key::alias("default", &id));
    assert_eq!(kind(&paths, &id, &soft), Some(K::LostOnDevice));

    assert_eq!(IdentityKeyFileError::Missing.unusable(), Some(K::Missing));
    for not_an_open in [
        IdentityKeyFileError::Exists,
        IdentityKeyFileError::KeyIsTheIdentity,
        IdentityKeyFileError::NotALabel,
    ] {
        assert_eq!(not_an_open.unusable(), None, "{not_an_open:?}");
    }
}

#[path = "common/keystore.rs"]
mod keystore;

/// Under a custody that makes a new key at the alias on every seal, as
/// `KeystoreCipher.kt` does, an install over a file that exists is
/// refused before it seals: `Exists`, told apart from every write
/// failure, the file unchanged and its record still openable (#377
/// review F2) -- what an interrupted re-seal's second run relies on.
#[test]
fn install_over_an_existing_file_is_exists_and_leaves_it_openable() {
    let (_dir, paths) = desktop_paths();
    let keystore = keystore::SoftKeystore::default();
    let key = identity_key::create(&paths, &keystore).expect("created");
    let file = identity_key::key_path(&paths, key.endpoint());
    let before = std::fs::read(&file).expect("read");
    assert_eq!(
        identity_key::install(&paths, &key, &keystore),
        Err(IdentityKeyFileError::Exists)
    );
    assert_eq!(std::fs::read(&file).expect("read"), before, "unchanged");
    let opened = identity_key::open(&paths, key.endpoint(), &keystore)
        .expect("the first record still opens")
        .expect("a key");
    assert_eq!(opened.public_key(), key.public_key());
}

/// Write `identity-keys/<label>.key` as a release before Stage 22 did: a
/// header naming the label, sealed under the label's alias. The public
/// key and the id it derives.
fn write_legacy(
    paths: &ProfilePaths,
    label: &str,
    cipher: &dyn KeyCipher,
) -> ([u8; 32], EndpointId) {
    let (secret, public, derived) = handmade();
    let line = record_of(
        cipher,
        &identity_key::legacy_alias(paths.profile(), label),
        label.as_bytes(),
        &public,
        &secret,
    );
    let file = identity_key::legacy_key_path(paths, label);
    assert_eq!(
        file,
        paths
            .state_dir()
            .join("identity-keys")
            .join(format!("{label}.key"))
    );
    interweave_profile_config::create_private_exclusive_within(
        &file,
        line.as_bytes(),
        paths.boundary(),
    )
    .expect("written");
    (public, derived)
}

#[test]
fn open_legacy_reads_a_label_record_at_both_custodies_under_its_derived_id() {
    let filesystem = FilesystemCustody;
    let soft = SoftKeystore::default();
    for cipher in [&filesystem as &dyn KeyCipher, &soft] {
        let (_dir, paths) = desktop_paths();
        let (public, derived) = write_legacy(&paths, "human", cipher);
        let key = identity_key::open_legacy(&paths, "human", cipher)
            .expect("opens")
            .expect("a key");
        assert_eq!(key.public_key(), public);
        assert_eq!(key.endpoint(), &derived, "the derived id, never the label");
        // Under another label the file is absent; the same file read
        // under another label's name is another identity's.
        assert_eq!(
            identity_key::open_legacy(&paths, "work", cipher).map(|k| k.is_some()),
            Ok(false)
        );
        std::fs::copy(
            identity_key::legacy_key_path(&paths, "human"),
            identity_key::legacy_key_path(&paths, "work"),
        )
        .expect("copy");
        std::fs::set_permissions(
            identity_key::legacy_key_path(&paths, "work"),
            std::fs::Permissions::from_mode(0o600),
        )
        .expect("chmod");
        assert_eq!(
            identity_key::open_legacy(&paths, "work", cipher).map(|k| k.is_some()),
            Err(IdentityKeyFileError::Key(
                IdentityKeyError::EndpointMismatch
            ))
        );
        // Nothing of the new directory reads it.
        assert_eq!(
            identity_key::open(&paths, &derived, cipher).map(|k| k.is_some()),
            Ok(false)
        );
    }
    // The seal is the label's alias: the Keystore was asked for it alone.
    let want = identity_key::legacy_alias("default", "human");
    assert!(
        soft.used
            .borrow()
            .iter()
            .filter(|a| a.rsplit('.').next() == Some("human"))
            .all(|a| *a == want),
        "{:?}",
        soft.used.borrow()
    );
}

#[test]
fn open_legacy_takes_a_label_and_never_a_path() {
    let (_dir, paths) = desktop_paths();
    for label in ["../endpoint-keys/x", "a/b", ".hidden", ""] {
        assert_eq!(
            identity_key::open_legacy(&paths, label, &FilesystemCustody).map(|k| k.is_some()),
            Err(IdentityKeyFileError::NotALabel),
            "{label:?}"
        );
        assert_eq!(
            identity_key::remove_legacy(&paths, label, &FilesystemCustody),
            Err(IdentityKeyFileError::NotALabel),
            "{label:?}"
        );
    }
}

/// The re-seal's three steps, the store's half: open the label's record,
/// install the same key under its derived id, remove the old file and
/// retire the label's alias -- and the new record opens with the same
/// key, under its own alias, after the old one is retired.
#[test]
fn a_legacy_key_installed_then_removed_opens_under_its_derived_id() {
    let (_dir, paths) = desktop_paths();
    let soft = SoftKeystore::default();
    let (public, derived) = write_legacy(&paths, "human", &soft);
    let key = identity_key::open_legacy(&paths, "human", &soft)
        .expect("opens")
        .expect("a key");
    identity_key::install(&paths, &key, &soft).expect("installed");
    assert_eq!(
        identity_key::install(&paths, &key, &soft),
        Err(IdentityKeyFileError::Exists),
        "a second run finds it written"
    );
    let legacy = identity_key::legacy_key_path(&paths, "human");
    assert_eq!(
        identity_key::remove_legacy(&paths, "human", &soft),
        Ok(true)
    );
    assert!(!legacy.exists(), "the old file is gone");
    assert_eq!(
        *soft.retired.borrow(),
        vec![identity_key::legacy_alias("default", "human")],
        "and the label's alias alone retired"
    );
    assert!(
        !soft
            .keys
            .borrow()
            .contains_key(&identity_key::legacy_alias("default", "human"))
    );
    let opened = identity_key::open(&paths, &derived, &soft)
        .expect("opens")
        .expect("a key");
    assert_eq!(opened.public_key(), public, "the same key");
    assert_eq!(
        identity_key::remove_legacy(&paths, "human", &soft),
        Ok(false),
        "again: nothing to remove"
    );
}

#[test]
fn sealed_lists_the_ids_by_file_name_and_opens_no_file() {
    let (_dir, paths) = desktop_paths();
    assert_eq!(identity_key::sealed(&paths), Ok(Vec::new()), "no directory");
    let mut made: Vec<EndpointId> = (0..3)
        .map(|_| {
            identity_key::create(&paths, &FilesystemCustody)
                .expect("created")
                .endpoint()
                .clone()
        })
        .collect();
    made.sort();
    let dir = paths.state_dir().join("endpoint-keys");
    // Names that are no derived id's key file are skipped.
    for stray in ["notes.txt", "human.key", ".tmp-x"] {
        std::fs::write(dir.join(stray), "x").expect("a stray file");
    }
    // A file nobody can read is still listed: its name is what is read.
    std::fs::set_permissions(
        identity_key::key_path(&paths, &made[0]),
        std::fs::Permissions::from_mode(0o000),
    )
    .expect("chmod");
    assert_eq!(identity_key::sealed(&paths), Ok(made.clone()));
    // A legacy file is not an endpoint key.
    write_legacy(&paths, "human", &FilesystemCustody);
    assert_eq!(identity_key::sealed(&paths), Ok(made));
}
