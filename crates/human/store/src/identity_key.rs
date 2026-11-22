// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A user identity's own key pair, which is its endpoint's key (ADR-0063
//! decisions 1 and 2, ADR-0067 decisions 1 to 5).
//!
//! Every identity -- an `EndpointId` of the node, ADR-0061 -- is an
//! Ed25519 key pair that signs its contact card, generated from the
//! system's CSPRNG and never derived from the node's seed, the recovery
//! phrase or any transport material: a phrase restores the node, not the
//! identities, and the same identity can later live on a second node
//! (ADR-0064). Its `EndpointId` is DERIVED from the public key
//! ([`EndpointId::from_ed25519_public`]), so the key is the identity: a
//! new key is a new `EndpointId` and another identity, and nothing here
//! replaces a key under an id (ADR-0067 decision 5;
//! [`IdentityKeyFileError::KeyIsTheIdentity`]). Its [`Fingerprint`] is
//! what a person compares.
//!
//! # Where it is kept, and where it never is
//!
//! In one file per identity, `<state_dir>/endpoint-keys/<EndpointId>.key`
//! ([`key_path`], profile-config's `endpoint_key::key_path`), owner-only
//! in an owner-only directory, under the profile's state root and BESIDE
//! the human store, never inside it: Discard and recreate of the store
//! leaves every key, and nothing of a key enters the store, a log, IPC or
//! a backup. The seed's custody (`identity.iwk1`, the embedded host's) is
//! another file, another magic and, on Android, another alias namespace,
//! and nothing here touches it.
//!
//! # The IDK1 record
//!
//! Its layout is profile-config's (`endpoint_key`: [`header_bytes`],
//! [`encode_record`], [`RawRecord`]), the one the daemon reads the public
//! part of; this module adds the body's seal and open under a
//! [`KeyCipher`]. Opening refuses another magic (a seed record among
//! them), a version this build does not read, a header naming another
//! `EndpointId` than the one asked for (a renamed file), a header id the
//! record's public key does not derive ([`IdentityKeyError::NotDerived`]),
//! and a public key that is not the secret's: ADR-0063 decision 2's
//! associated data, cryptographic under a wrapping key and checked field
//! by field at the desktop's filesystem floor (ADR-0060 decision 3),
//! where no wrapping key exists.
//!
//! # The record a release before Stage 22 wrote
//!
//! Before ADR-0067 an identity's key was sealed under its LABEL:
//! `<state_dir>/identity-keys/<label>.key` ([`legacy_key_path`]), its
//! header naming the label, its Keystore body sealed under
//! `interweave.idk1.<profile>.<label>` ([`legacy_alias`]) with that
//! header as associated data. [`open_legacy`] reads such a record,
//! [`install`] writes the same key's record under its derived id, and
//! [`remove_legacy`] retires the old file and platform key: Android's
//! embedded host re-seals that way at the first start of the release that
//! carries ADR-0067, in process, since the header is the seal's
//! associated data and cannot be rewritten in place; on the desktop
//! `transportctl endpoint adopt` brings a filesystem-custody record over
//! offline. Nothing else reads `identity-keys/`.
//!
//! # The custody byte is a floor, never a choice
//!
//! The platform's [`KeyCipher`] says which custody it holds. A
//! record below it -- a filesystem-only record where a wrapping key
//! exists -- is refused as a downgrade, and one above it -- a sealed record
//! on a desktop -- as unreadable here. Nothing re-seals a record into
//! another custody: the person creates a new identity (ADR-0067
//! decision 5).

use ed25519_dalek::{Signer as _, SigningKey};
use std::path::PathBuf;

use interweave_profile_config::PersistError;
use interweave_profile_config::endpoint_alias::EndpointAlias;
use interweave_profile_config::endpoint_key::{
    self, RawRecord, RecordError, encode_record, header_bytes,
};
use interweave_profile_config::paths::ProfilePaths;
use interweave_profile_config::persist;
use interweave_transport_api::EndpointId;
use sha2::{Digest as _, Sha256};
use zeroize::Zeroizing;

pub use crate::key_custody::{CipherFailure, Custody, FilesystemCustody, KeyCipher};
pub use interweave_profile_config::endpoint_key::{
    ENDPOINT_KEYS_DIR, KEY_FILE_SUFFIX, MAGIC, MAX_RECORD_BYTES, VERSION,
};

/// The directory a release before Stage 22 kept an identity's key in,
/// named by its label ([`legacy_key_path`]); profile-config's
/// `LEGACY_IDENTITY_KEYS_DIR`. A key is kept in [`ENDPOINT_KEYS_DIR`].
pub const IDENTITY_KEYS_DIR: &str = endpoint_key::LEGACY_IDENTITY_KEYS_DIR;

/// The platform key's alias for one identity of one profile (ADR-0063
/// decision 2): its own namespace, never one of the seed's
/// `interweave.iwk1.<profile>.<policy>` aliases.
#[must_use]
pub fn alias(profile: &str, endpoint: &EndpointId) -> String {
    format!("interweave.idk1.{profile}.{}", endpoint.as_str())
}

/// The alias a release before Stage 22 sealed an identity's key under,
/// named by its label: what [`open_legacy`] opens and [`remove_legacy`]
/// retires.
#[must_use]
pub fn legacy_alias(profile: &str, label: &str) -> String {
    format!("interweave.idk1.{profile}.{label}")
}

/// The first 8 bytes of the SHA-256 of a public key, shown as 16
/// lowercase hex characters (ADR-0063 decision 1).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint([u8; 8]);

impl Fingerprint {
    /// The fingerprint of `public_key`.
    #[must_use]
    pub fn of(public_key: &[u8; 32]) -> Self {
        let digest = Sha256::digest(public_key);
        let mut first = [0u8; 8];
        first.copy_from_slice(&digest[..8]);
        Self(first)
    }
}

impl core::fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl core::fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Fingerprint({self})")
    }
}

/// One identity's key pair, held in memory while the client runs.
///
/// Its secret is zeroized when it drops, and no formatter prints it. Its
/// [`Self::endpoint`] is the id its public key derives, by construction.
pub struct IdentityKey {
    endpoint: EndpointId,
    signing: SigningKey,
}

impl core::fmt::Debug for IdentityKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IdentityKey")
            .field("endpoint", &self.endpoint.as_str())
            .field("fingerprint", &self.fingerprint())
            .finish_non_exhaustive()
    }
}

impl IdentityKey {
    /// A new key pair from the system's CSPRNG, and with it a new
    /// identity: [`Self::endpoint`] is the id its public key derives.
    ///
    /// # Errors
    /// [`IdentityKeyError::Random`] when the system cannot supply the
    /// bytes: no key is made from anything else.
    pub fn generate() -> Result<Self, IdentityKeyError> {
        use rand::TryRng as _;
        let mut secret = Zeroizing::new([0u8; 32]);
        rand::rngs::SysRng
            .try_fill_bytes(secret.as_mut())
            .map_err(|e| IdentityKeyError::Random(e.to_string()))?;
        Ok(Self::from_signing(SigningKey::from_bytes(&secret)))
    }

    /// The key `signing` is, under the id its public key derives.
    fn from_signing(signing: SigningKey) -> Self {
        let endpoint = EndpointId::from_ed25519_public(&signing.verifying_key().to_bytes());
        Self { endpoint, signing }
    }

    /// The identity this key is: the id its public key derives.
    #[must_use]
    pub fn endpoint(&self) -> &EndpointId {
        &self.endpoint
    }

    /// The public key.
    #[must_use]
    pub fn public_key(&self) -> [u8; 32] {
        self.signing.verifying_key().to_bytes()
    }

    /// The public key's fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of(&self.public_key())
    }

    /// The Ed25519 signature of `message`.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing.sign(message).to_bytes()
    }

    /// The record for this key under `cipher`'s custody, as the file's
    /// one line: its header names the derived id, and a sealed body is
    /// sealed under [`alias`]`(profile, id)` binding that header.
    ///
    /// # Errors
    /// [`IdentityKeyError::Cipher`] when the platform will not seal.
    pub fn to_record(
        &self,
        profile: &str,
        cipher: &dyn KeyCipher,
    ) -> Result<Zeroizing<String>, IdentityKeyError> {
        let custody = cipher.custody();
        let header = header_bytes(
            record_custody(custody),
            self.endpoint.as_str().as_bytes(),
            &self.public_key(),
        )
        .map_err(record_refusal)?;
        let secret = Zeroizing::new(self.signing.to_bytes());
        let body = Zeroizing::new(
            cipher
                .seal(&alias(profile, &self.endpoint), &header, &secret)
                .map_err(IdentityKeyError::Cipher)?,
        );
        Ok(encode_record(&header, &body))
    }

    /// The key a record holds, for the identity `endpoint` of `profile`,
    /// under `cipher`'s custody.
    ///
    /// # Errors
    /// One [`IdentityKeyError`] per refusal the module documents: the
    /// header must name `endpoint` ([`IdentityKeyError::EndpointMismatch`])
    /// and `endpoint` must be the id the record's public key derives
    /// ([`IdentityKeyError::NotDerived`]).
    pub fn from_record(
        line: &str,
        profile: &str,
        endpoint: &EndpointId,
        cipher: &dyn KeyCipher,
    ) -> Result<Self, IdentityKeyError> {
        let bytes = endpoint_key::decode_line(line).map_err(record_refusal)?;
        let raw = RawRecord::parse(&bytes).map_err(record_refusal)?;
        if raw.endpoint != endpoint.as_str().as_bytes() {
            return Err(IdentityKeyError::EndpointMismatch);
        }
        if EndpointId::from_ed25519_public(&raw.public_key) != *endpoint {
            return Err(IdentityKeyError::NotDerived);
        }
        open_body(&raw, &alias(profile, endpoint), cipher).map(Self::from_signing)
    }

    /// The key a record of a release before Stage 22 holds: its header
    /// names `label`, and a sealed body was sealed under
    /// [`legacy_alias`]`(profile, label)` binding that header. The key's
    /// [`Self::endpoint`] is the id its public key derives.
    fn from_legacy_record(
        line: &str,
        profile: &str,
        label: &str,
        cipher: &dyn KeyCipher,
    ) -> Result<Self, IdentityKeyError> {
        let bytes = endpoint_key::decode_line(line).map_err(record_refusal)?;
        let raw = RawRecord::parse(&bytes).map_err(record_refusal)?;
        if raw.endpoint != label.as_bytes() {
            return Err(IdentityKeyError::EndpointMismatch);
        }
        open_body(&raw, &legacy_alias(profile, label), cipher).map(Self::from_signing)
    }
}

/// The secret a parsed record holds under `cipher`'s custody, opened
/// under `alias`: the custody floor first, then the platform's open, then
/// the public key compared with the secret's.
fn open_body(
    raw: &RawRecord<'_>,
    alias: &str,
    cipher: &dyn KeyCipher,
) -> Result<SigningKey, IdentityKeyError> {
    let held = Custody::from_byte(raw.custody.byte()).ok_or(IdentityKeyError::UnknownCustody)?;
    let floor = cipher.custody();
    if held < floor {
        return Err(IdentityKeyError::CustodyDowngrade);
    }
    if held > floor {
        return Err(IdentityKeyError::SealedUnreadableHere);
    }
    let secret = cipher
        .open(alias, raw.header, raw.body)
        .map_err(IdentityKeyError::Cipher)?;
    let signing = SigningKey::from_bytes(&secret);
    if signing.verifying_key().to_bytes() != raw.public_key {
        return Err(IdentityKeyError::KeyMismatch);
    }
    Ok(signing)
}

/// The record's custody byte for the platform's custody: the two enums
/// name the same two custodies by the same bytes.
fn record_custody(custody: Custody) -> endpoint_key::Custody {
    match custody {
        Custody::FilesystemOnly => endpoint_key::Custody::FilesystemOnly,
        Custody::Keystore => endpoint_key::Custody::Keystore,
    }
}

/// profile-config's record refusal in this module's terms.
const fn record_refusal(e: RecordError) -> IdentityKeyError {
    match e {
        RecordError::NotARecord => IdentityKeyError::NotARecord,
        RecordError::UnsupportedVersion(v) => IdentityKeyError::UnsupportedVersion(v),
        RecordError::UnknownCustody => IdentityKeyError::UnknownCustody,
        RecordError::Truncated => IdentityKeyError::Truncated,
        RecordError::HeaderNotDerived => IdentityKeyError::NotDerived,
    }
}

/// Where `endpoint`'s key file is: `<state_dir>/endpoint-keys/<EndpointId>.key`
/// (profile-config's `endpoint_key::key_path`, the file the daemon reads
/// the public part of).
#[must_use]
pub fn key_path(paths: &ProfilePaths, endpoint: &EndpointId) -> PathBuf {
    endpoint_key::key_path(paths, endpoint)
}

/// Where a release before Stage 22 kept the key of the identity labelled
/// `label`: `<state_dir>/identity-keys/<label>.key`. [`open_legacy`] and
/// [`remove_legacy`] take only a label of the alias grammar, which has no
/// separator and no leading dot, so the name stays one entry of that
/// directory.
#[must_use]
pub fn legacy_key_path(paths: &ProfilePaths, label: &str) -> PathBuf {
    paths
        .state_dir()
        .join(IDENTITY_KEYS_DIR)
        .join(format!("{label}{KEY_FILE_SUFFIX}"))
}

/// Mint a NEW identity: a key from the CSPRNG ([`IdentityKey::generate`]),
/// written to `endpoint-keys/<derived id>.key` under `cipher`'s custody
/// ([`install`]).
///
/// # Errors
/// A key or persistence error, after which no file was made;
/// [`IdentityKeyFileError::Exists`] only if a file already stood at the
/// fresh id's name, which only a copy of this key could have put there.
pub fn create(
    paths: &ProfilePaths,
    cipher: &dyn KeyCipher,
) -> Result<IdentityKey, IdentityKeyFileError> {
    let key = IdentityKey::generate()?;
    install(paths, &key, cipher)?;
    Ok(key)
}

/// Write `key`'s record at [`key_path`]`(paths, key.endpoint())`, sealed
/// under [`alias`]`(profile, derived id)`, owner-only, never over a file
/// that exists: what a re-seal of a [`open_legacy`] key writes.
///
/// # Errors
/// [`IdentityKeyFileError::Exists`] when a file stands there -- told apart
/// from every other failure, so a re-seal interrupted after its write can
/// tell that it is finished; a key or persistence error otherwise, after
/// which no file was made.
pub fn install(
    paths: &ProfilePaths,
    key: &IdentityKey,
    cipher: &dyn KeyCipher,
) -> Result<(), IdentityKeyFileError> {
    let path = key_path(paths, key.endpoint());
    // REFUSED BEFORE THE SEAL: a sealing platform generates a new key at
    // the alias on every seal, so sealing first would replace the key an
    // existing record needs (#377 review F2). The exclusive create below
    // still decides a race; the caller holds the profile's client lock.
    if file_present(&path)? {
        return Err(IdentityKeyFileError::Exists);
    }
    let line = key.to_record(paths.profile(), cipher)?;
    match persist::create_private_exclusive_within(&path, line.as_bytes(), paths.boundary()) {
        Ok(()) => Ok(()),
        Err(PersistError::AlreadyExists) => {
            // Only a race past the check above reaches here: the seal may
            // then have made a platform key under the alias the existing
            // record also uses, and it is the existing one's to keep --
            // nothing is retired here.
            Err(IdentityKeyFileError::Exists)
        }
        Err(e) => Err(IdentityKeyFileError::Persist(e.to_string())),
    }
}

/// Open `endpoint`'s key from its file in `endpoint-keys/`, under
/// `cipher`'s custody: `None` when the identity has no key file.
///
/// The file is read by profile-config's one reader of private state
/// files: regular, not a link, this uid's, readable by nobody else,
/// bounded, its directory private under the boundary. Every refusal of
/// [`IdentityKey::from_record`] applies after it.
///
/// # Errors
/// A persistence refusal of the file, or the record's refusal.
pub fn open(
    paths: &ProfilePaths,
    endpoint: &EndpointId,
    cipher: &dyn KeyCipher,
) -> Result<Option<IdentityKey>, IdentityKeyFileError> {
    let Some(line) = read_record(paths, ENDPOINT_KEYS_DIR, &key_path(paths, endpoint))? else {
        return Ok(None);
    };
    Ok(Some(IdentityKey::from_record(
        &line,
        paths.profile(),
        endpoint,
        cipher,
    )?))
}

/// Open the key a release before Stage 22 sealed under `label`, from
/// `identity-keys/<label>.key`, under `cipher`'s custody: `None` when
/// there is no such file. The record's header must name `label`, and a
/// sealed body opens under [`legacy_alias`]; the custody floor and the
/// public key's check are [`open`]'s. The key's `endpoint()` is the id
/// its public key derives.
///
/// # Errors
/// [`IdentityKeyFileError::NotALabel`] for a `label` outside the alias
/// grammar; otherwise as [`open`]'s, a header naming another label being
/// [`IdentityKeyError::EndpointMismatch`].
pub fn open_legacy(
    paths: &ProfilePaths,
    label: &str,
    cipher: &dyn KeyCipher,
) -> Result<Option<IdentityKey>, IdentityKeyFileError> {
    let path = checked_legacy_path(paths, label)?;
    let Some(line) = read_record(paths, IDENTITY_KEYS_DIR, &path)? else {
        return Ok(None);
    };
    Ok(Some(IdentityKey::from_legacy_record(
        &line,
        paths.profile(),
        label,
        cipher,
    )?))
}

/// Delete `endpoint`'s key file and retire its platform key (ADR-0063
/// A 2026-10-22: Remove of an added identity deletes both). Whether
/// there was a file.
///
/// # Errors
/// A persistence or cipher error; the file is removed before the platform
/// key is retired, so a failed retire leaves a key that opens no record.
pub fn remove(
    paths: &ProfilePaths,
    endpoint: &EndpointId,
    cipher: &dyn KeyCipher,
) -> Result<bool, IdentityKeyFileError> {
    remove_file_then_retire(
        paths,
        &key_path(paths, endpoint),
        &alias(paths.profile(), endpoint),
        cipher,
    )
}

/// Delete the file a release before Stage 22 kept `label`'s key in and
/// retire [`legacy_alias`]`(profile, label)`, the file first as
/// [`remove`] does: what a re-seal does once [`install`] wrote the key
/// under its derived id. Whether there was a file.
///
/// # Errors
/// [`IdentityKeyFileError::NotALabel`], or as [`remove`]'s.
pub fn remove_legacy(
    paths: &ProfilePaths,
    label: &str,
    cipher: &dyn KeyCipher,
) -> Result<bool, IdentityKeyFileError> {
    let path = checked_legacy_path(paths, label)?;
    remove_file_then_retire(paths, &path, &legacy_alias(paths.profile(), label), cipher)
}

/// The ids whose key files stand in `endpoint-keys/`, sorted, read from
/// the file NAMES alone (profile-config's `parse_key_file_name`): no file
/// is opened, and a name that is not `<derived EndpointId>.key` is
/// skipped. No directory is no key.
///
/// # Errors
/// [`IdentityKeyFileError::Read`] for a directory that is a link, not a
/// directory or another account's, or one that cannot be listed.
pub fn sealed(paths: &ProfilePaths) -> Result<Vec<EndpointId>, IdentityKeyFileError> {
    keys_dir_is_safe(paths, ENDPOINT_KEYS_DIR)?;
    let io = |e: std::io::Error| IdentityKeyFileError::Read(KeyFileRead::Io(e.kind()));
    let entries = match std::fs::read_dir(paths.state_dir().join(ENDPOINT_KEYS_DIR)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io(e)),
    };
    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io)?;
        if let Some(id) = entry
            .file_name()
            .to_str()
            .and_then(|name| endpoint_key::parse_key_file_name(name).ok())
        {
            ids.push(id);
        }
    }
    ids.sort_unstable();
    Ok(ids)
}

/// The legacy path of `label`, a label of the alias grammar alone.
fn checked_legacy_path(paths: &ProfilePaths, label: &str) -> Result<PathBuf, IdentityKeyFileError> {
    EndpointAlias::parse(label).map_err(|_| IdentityKeyFileError::NotALabel)?;
    Ok(legacy_key_path(paths, label))
}

/// The record line at `path`, under the state root's `dir`, judged as a
/// directory first ([`keys_dir_is_safe`]) and read by profile-config's
/// one reader of private state files: `None` when there is no file.
fn read_record(
    paths: &ProfilePaths,
    dir: &str,
    path: &std::path::Path,
) -> Result<Option<Zeroizing<String>>, IdentityKeyFileError> {
    keys_dir_is_safe(paths, dir)?;
    match interweave_profile_config::read_private_within(path, paths.boundary(), MAX_RECORD_BYTES) {
        Ok(line) => Ok(line.map(Zeroizing::new)),
        Err(e) => Err(IdentityKeyFileError::Read(KeyFileRead::of(&e))),
    }
}

/// Remove `path`, then retire `alias`: a failed retire leaves a platform
/// key that opens no record, never a record whose platform key is gone.
fn remove_file_then_retire(
    paths: &ProfilePaths,
    path: &std::path::Path,
    alias: &str,
    cipher: &dyn KeyCipher,
) -> Result<bool, IdentityKeyFileError> {
    let removed = persist::remove_private_within(path, paths.boundary())
        .map_err(|e| IdentityKeyFileError::Persist(e.to_string()))?;
    cipher.retire(alias).map_err(IdentityKeyError::Cipher)?;
    Ok(removed)
}

/// The key directory `dir` under the state root, judged by what it is and
/// whose before any read: a link, something that is not a directory, or a
/// directory another account owns is [`KeyFileRead::DirUnsafe`] -- never
/// followed, changed or replaced, since the client changes nothing it
/// does not own (design/2026-10-24-identity-key.md section 3). Owner is
/// compared with the profile's state directory, this account's by the
/// profile lock; the read judges a directory's mode and not its owner. No
/// directory is no key file, left to the read.
fn keys_dir_is_safe(paths: &ProfilePaths, dir: &str) -> Result<(), IdentityKeyFileError> {
    use std::os::unix::fs::MetadataExt as _;
    let dir = paths.state_dir().join(dir);
    let unsafe_dir = || IdentityKeyFileError::Read(KeyFileRead::DirUnsafe);
    let meta = match std::fs::symlink_metadata(&dir) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(IdentityKeyFileError::Read(KeyFileRead::Io(e.kind()))),
    };
    if !meta.file_type().is_dir() {
        return Err(unsafe_dir());
    }
    let owner = std::fs::metadata(paths.state_dir())
        .map_err(|e| IdentityKeyFileError::Read(KeyFileRead::Io(e.kind())))?
        .uid();
    if meta.uid() != owner {
        return Err(unsafe_dir());
    }
    Ok(())
}

/// Why a new key was not made in an identity's place, and whether the
/// old file is still there: the shape app-core's `KeySource::replace`
/// answers in. Under ADR-0067 decision 5 no key is ever replaced, so it
/// is [`NewKeyFailed::Kept`] with [`IdentityKeyFileError::KeyIsTheIdentity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewKeyFailed {
    /// Stopped before the old file was removed: the identity is as it was.
    Kept(IdentityKeyFileError),
    /// Stopped after: the saved key is gone, and no new one was made.
    Gone(IdentityKeyFileError),
}

/// Whether a file, a link or anything else stands at `path`.
fn file_present(path: &std::path::Path) -> Result<bool, IdentityKeyFileError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(IdentityKeyFileError::Persist(e.to_string())),
    }
}

/// Why an identity's key file cannot be made, opened or removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityKeyFileError {
    /// A key file already stands at the name.
    Exists,
    /// The key file could not be read, before its record was judged.
    Read(KeyFileRead),
    /// The identity's key file is absent: the key-missing state (ADR-0067
    /// decision 5), in which the identity signs nothing. Never made again
    /// under the same id -- a new key is a new identity.
    Missing,
    /// The key or its record refused.
    Key(IdentityKeyError),
    /// The file or its directory could not be written, read or removed,
    /// or is not private; the persistence layer's own message.
    Persist(String),
    /// A new key under an identity's id was asked for: the key is the
    /// identity, since its id derives from the public key, and is never
    /// replaced; a new key is a new identity (ADR-0067 decision 5).
    KeyIsTheIdentity,
    /// A legacy label outside the alias grammar, which could name no file
    /// of `identity-keys/`.
    NotALabel,
}

impl From<IdentityKeyError> for IdentityKeyFileError {
    fn from(e: IdentityKeyError) -> Self {
        Self::Key(e)
    }
}

impl core::fmt::Display for IdentityKeyFileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Exists => f.write_str("a key file already stands at that identity's name"),
            Self::Missing => f.write_str("the identity's key file is missing"),
            Self::Key(e) => e.fmt(f),
            Self::Read(r) => write!(f, "the identity's key file could not be read: {r:?}"),
            Self::Persist(e) => write!(f, "the identity's key file: {e}"),
            Self::KeyIsTheIdentity => f.write_str(
                "an identity's key is never replaced: its id is derived from the key, so a new key is a new identity",
            ),
            Self::NotALabel => f.write_str("the label is not an endpoint alias"),
        }
    }
}

/// Why a key file could not be read, before its record was judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFileRead {
    /// Larger than any record.
    TooLarge,
    /// The file, or the key directory this account owns, is open to
    /// another account.
    NotPrivate,
    /// The key directory is a link, not a directory, or another account's.
    DirUnsafe,
    /// The read failed with this kind.
    Io(std::io::ErrorKind),
}

impl KeyFileRead {
    fn of(e: &interweave_profile_config::PrivateReadError) -> Self {
        use interweave_profile_config::PrivateReadError;
        match e {
            PrivateReadError::TooLarge => Self::TooLarge,
            PrivateReadError::NotPrivate { .. } => Self::NotPrivate,
            PrivateReadError::Read(e) => Self::Io(e.kind()),
            // A refusal this build does not know: told as a read that may
            // pass, never as damage it cannot name.
            _ => Self::Io(std::io::ErrorKind::Other),
        }
    }
}

/// What kind of refusal an identity's key met at open, in the person's
/// terms (design/2026-10-24-identity-key.md section 3): what the client
/// says and offers is chosen by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyUnusable {
    /// The platform holds the key no longer, or refuses it.
    LostOnDevice,
    /// Protected less than this device requires.
    BelowDevice,
    /// Sealed by another device.
    OtherDevice,
    /// Possibly a newer version's record.
    NewerVersion,
    /// Not a valid record for this identity.
    Damaged,
    /// Open to another account.
    Exposed,
    /// The key folder is not one this account alone owns.
    DirUnsafe,
    /// A platform or read error that may pass.
    NotNow,
    /// The key file is missing.
    Missing,
}

impl IdentityKeyFileError {
    /// The kind of refusal this is at open, or `None` for one that is not
    /// an open's (`Exists`; a write's or removal's `Persist`; `Random`;
    /// `KeyIsTheIdentity`; `NotALabel`).
    #[must_use]
    pub const fn unusable(&self) -> Option<KeyUnusable> {
        use std::io::ErrorKind;
        Some(match self {
            Self::Missing => KeyUnusable::Missing,
            Self::Read(KeyFileRead::DirUnsafe) => KeyUnusable::DirUnsafe,
            Self::Read(KeyFileRead::NotPrivate) => KeyUnusable::Exposed,
            // A file its owner cannot read, or bytes that are not text, is
            // damage: trying again would repeat it for ever.
            Self::Read(
                KeyFileRead::TooLarge
                | KeyFileRead::Io(ErrorKind::InvalidData | ErrorKind::PermissionDenied),
            ) => KeyUnusable::Damaged,
            Self::Read(KeyFileRead::Io(_)) => KeyUnusable::NotNow,
            Self::Key(e) => match e {
                IdentityKeyError::Cipher(
                    CipherFailure::KeyInvalidated | CipherFailure::KeyUnavailable,
                ) => KeyUnusable::LostOnDevice,
                IdentityKeyError::Cipher(CipherFailure::Platform(_)) => KeyUnusable::NotNow,
                IdentityKeyError::CustodyDowngrade => KeyUnusable::BelowDevice,
                IdentityKeyError::SealedUnreadableHere => KeyUnusable::OtherDevice,
                IdentityKeyError::UnsupportedVersion(v) if *v > VERSION => {
                    KeyUnusable::NewerVersion
                }
                IdentityKeyError::UnknownCustody => KeyUnusable::NewerVersion,
                IdentityKeyError::NotARecord
                | IdentityKeyError::Truncated
                | IdentityKeyError::UnsupportedVersion(_)
                | IdentityKeyError::EndpointMismatch
                | IdentityKeyError::NotDerived
                | IdentityKeyError::KeyMismatch
                | IdentityKeyError::Cipher(CipherFailure::Unauthentic) => KeyUnusable::Damaged,
                IdentityKeyError::Random(_) => return None,
            },
            Self::Exists | Self::Persist(_) | Self::KeyIsTheIdentity | Self::NotALabel => {
                return None;
            }
        })
    }
}

impl core::error::Error for IdentityKeyFileError {}

/// Why an identity key cannot be created, opened or removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityKeyError {
    /// The system's CSPRNG supplied no bytes.
    Random(String),
    /// Not an IDK1 record: another magic (a seed record among them), or
    /// not base64 at all.
    NotARecord,
    /// An IDK1 record of a version this build does not read.
    UnsupportedVersion(u8),
    /// A custody byte this build does not know.
    UnknownCustody,
    /// The record ends before its fields do.
    Truncated,
    /// The record names another identity: a file renamed or copied over.
    EndpointMismatch,
    /// The id the record's header names is not the one its public key
    /// derives: one of the two was written by hand.
    NotDerived,
    /// The public key is not the secret's.
    KeyMismatch,
    /// A filesystem-only record where the platform holds a wrapping key:
    /// refused, never re-sealed; the person creates a new identity.
    CustodyDowngrade,
    /// A sealed record where the platform holds no wrapping key.
    SealedUnreadableHere,
    /// The platform cipher's refusal.
    Cipher(CipherFailure),
}

impl core::fmt::Display for IdentityKeyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Random(e) => write!(f, "the system could not supply random bytes for a key: {e}"),
            Self::NotARecord => f.write_str("the file is not an identity-key record"),
            Self::UnsupportedVersion(v) => write!(f, "the identity-key record is version {v}, which this build does not read"),
            Self::UnknownCustody => f.write_str("the identity-key record names a custody this build does not know"),
            Self::Truncated => f.write_str("the identity-key record is truncated"),
            Self::EndpointMismatch => f.write_str("the identity-key record belongs to another identity"),
            Self::NotDerived => f.write_str("the identity-key record's id does not derive from its public key"),
            Self::KeyMismatch => f.write_str("the identity-key record's public key is not its secret's"),
            Self::CustodyDowngrade => f.write_str(
                "the identity key is held below this platform's custody; create a new identity, which contacts will see as another identity",
            ),
            Self::SealedUnreadableHere => f.write_str("the identity key is sealed by a platform key this computer does not hold"),
            Self::Cipher(e) => write!(f, "the platform refused the identity key: {e:?}"),
        }
    }
}

impl core::error::Error for IdentityKeyError {}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;

    fn hex(bytes: &[u8]) -> String {
        use core::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
    }

    #[test]
    fn a_record_round_trips_at_the_filesystem_floor() {
        let key = IdentityKey::generate().expect("a key");
        let line = key
            .to_record("default", &FilesystemCustody)
            .expect("a record");
        let back = IdentityKey::from_record(&line, "default", key.endpoint(), &FilesystemCustody)
            .expect("opens");
        assert_eq!(back.public_key(), key.public_key());
        assert_eq!(back.endpoint(), key.endpoint());
        assert_eq!(back.sign(b"m"), key.sign(b"m"));
    }

    #[test]
    fn a_generated_keys_endpoint_is_the_id_its_public_key_derives() {
        let key = IdentityKey::generate().expect("a key");
        assert_eq!(
            key.endpoint(),
            &EndpointId::from_ed25519_public(&key.public_key())
        );
        assert!(EndpointId::parse_derived(key.endpoint().as_str()).is_ok());
    }

    #[test]
    fn the_fingerprint_is_the_first_eight_bytes_of_the_keys_sha256_in_lowercase_hex() {
        let public = [7u8; 32];
        let want = hex(&Sha256::digest(public)[..8]);
        assert_eq!(Fingerprint::of(&public).to_string(), want);
        assert_eq!(want.len(), 16);
    }

    #[test]
    fn the_secret_is_in_no_formatted_output() {
        let key = IdentityKey::generate().expect("a key");
        let shown = format!("{key:?}");
        let secret = key.signing.to_bytes();
        assert!(!shown.contains(&STANDARD.encode(secret)));
        assert!(!shown.contains(&hex(&secret)));
        assert!(shown.contains(&key.fingerprint().to_string()));
    }

    #[test]
    fn the_alias_is_the_identity_keys_own_namespace() {
        let key = IdentityKey::generate().expect("a key");
        let id = key.endpoint();
        assert_eq!(
            alias("default", id),
            format!("interweave.idk1.default.{}", id.as_str())
        );
        assert!(!alias("default", id).starts_with("interweave.iwk1."));
        assert_eq!(
            legacy_alias("default", "human"),
            "interweave.idk1.default.human"
        );
    }

    #[test]
    fn the_legacy_directory_is_profile_configs() {
        assert_eq!(IDENTITY_KEYS_DIR, "identity-keys");
        assert_ne!(IDENTITY_KEYS_DIR, ENDPOINT_KEYS_DIR);
    }
}
