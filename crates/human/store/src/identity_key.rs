// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A user identity's own key pair (ADR-0063 decisions 1 and 2).
//!
//! Every identity — an `EndpointId` of the node, ADR-0061 — has an
//! Ed25519 key pair that signs its contact card. It is generated with the
//! identity from the system's CSPRNG and never derived from the node's
//! seed, the recovery phrase or any transport material: a phrase restores
//! the node, not the identities, and the same identity can later live on a
//! second node (ADR-0064). Its public key is the identity's cryptographic
//! name, and its [`Fingerprint`] what a person compares.
//!
//! # Where it is kept, and where it never is
//!
//! In one file per identity, `<state_dir>/identity-keys/<EndpointId>.key`,
//! owner-only in an owner-only directory, under the profile's state root
//! and BESIDE the human store, never inside it: Discard and recreate of
//! the store leaves every key, and nothing of a key enters the store, a
//! log, IPC or a backup. The seed's custody (`identity.iwk1`, the
//! embedded host's) is another file, another magic and, on Android,
//! another alias namespace, and nothing here touches it.
//!
//! # The IDK1 record
//!
//! The file holds one line: the standard base64 of
//!
//! ```text
//! b"IDK1" | version (1) | custody (1) | len(EndpointId) (1) | EndpointId
//!         | public key (32) | body
//! ```
//!
//! where the body is the 32-byte secret at [`Custody::FilesystemOnly`]
//! and, at [`Custody::Keystore`], the secret sealed by the platform's
//! wrapping key with every byte before it as associated data. Opening
//! refuses another magic (a seed record among them), a version this
//! build does not read, an `EndpointId` other than the one asked for (a
//! renamed file), and a public key that is not the secret's: ADR-0063
//! decision 2's associated data, cryptographic under a wrapping key and
//! checked field by field at the desktop's filesystem floor (ADR-0060
//! decision 3), where no wrapping key exists.
//!
//! # The custody byte is a floor, never a choice
//!
//! The platform's [`KeyCipher`] says which custody it holds. A
//! record below it — a filesystem-only record where a wrapping key
//! exists — is refused as a downgrade, and one above it — a sealed record
//! on a desktop — as unreadable here. Nothing re-seals a record into
//! another custody: the person creates a new key, which counterparties
//! see as a key change (ADR-0063 A 2026-10-22).

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer as _, SigningKey};
use std::path::PathBuf;

use interweave_profile_config::PersistError;
use interweave_profile_config::paths::ProfilePaths;
use interweave_profile_config::persist;
use interweave_transport_api::EndpointId;
use sha2::{Digest as _, Sha256};
use zeroize::Zeroizing;

pub use crate::key_custody::{CipherFailure, Custody, FilesystemCustody, KeyCipher};

/// The record's magic.
pub const MAGIC: &[u8; 4] = b"IDK1";
/// The one version this build writes and reads.
pub const VERSION: u8 = 1;
/// The directory under the profile's state root that holds the files.
pub const IDENTITY_KEYS_DIR: &str = "identity-keys";
/// A file larger than this is no record of this build's.
pub const MAX_RECORD_BYTES: u64 = 1024;

/// The platform key's alias for one identity of one profile (ADR-0063
/// decision 2): its own namespace, never one of the seed's
/// `interweave.iwk1.<profile>.<policy>` aliases.
#[must_use]
pub fn alias(profile: &str, endpoint: &EndpointId) -> String {
    format!("interweave.idk1.{profile}.{}", endpoint.as_str())
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
/// Its secret is zeroized when it drops, and no formatter prints it.
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
    /// A new key pair for `endpoint`, from the system's CSPRNG.
    ///
    /// # Errors
    /// [`IdentityKeyError::Random`] when the system cannot supply the
    /// bytes: no key is made from anything else.
    pub fn generate(endpoint: EndpointId) -> Result<Self, IdentityKeyError> {
        use rand::TryRng as _;
        let mut secret = Zeroizing::new([0u8; 32]);
        rand::rngs::SysRng
            .try_fill_bytes(secret.as_mut())
            .map_err(|e| IdentityKeyError::Random(e.to_string()))?;
        Ok(Self {
            endpoint,
            signing: SigningKey::from_bytes(&secret),
        })
    }

    /// The identity this key is.
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

    /// The record's bytes before the body: the associated data a sealed
    /// body binds.
    fn header(&self, custody: Custody) -> Vec<u8> {
        header(custody, &self.endpoint, &self.public_key())
    }

    /// The record for this key under `cipher`'s custody, as the file's
    /// one line.
    ///
    /// # Errors
    /// [`IdentityKeyError::Cipher`] when the platform will not seal.
    pub fn to_record(
        &self,
        profile: &str,
        cipher: &dyn KeyCipher,
    ) -> Result<Zeroizing<String>, IdentityKeyError> {
        let custody = cipher.custody();
        let mut record = Zeroizing::new(self.header(custody));
        let secret = Zeroizing::new(self.signing.to_bytes());
        let body = Zeroizing::new(
            cipher
                .seal(&alias(profile, &self.endpoint), &record, &secret)
                .map_err(IdentityKeyError::Cipher)?,
        );
        record.extend_from_slice(&body);
        Ok(Zeroizing::new(STANDARD.encode(record.as_slice())))
    }

    /// The key a record holds, for the identity `endpoint` of `profile`,
    /// under `cipher`'s custody.
    ///
    /// # Errors
    /// One [`IdentityKeyError`] per refusal the module documents.
    pub fn from_record(
        line: &str,
        profile: &str,
        endpoint: &EndpointId,
        cipher: &dyn KeyCipher,
    ) -> Result<Self, IdentityKeyError> {
        let bytes = Zeroizing::new(
            STANDARD
                .decode(line.trim_end_matches('\n'))
                .map_err(|_| IdentityKeyError::NotARecord)?,
        );
        let fields = Fields::parse(&bytes)?;
        if fields.endpoint != endpoint.as_str().as_bytes() {
            return Err(IdentityKeyError::EndpointMismatch);
        }
        let floor = cipher.custody();
        if fields.custody < floor {
            return Err(IdentityKeyError::CustodyDowngrade);
        }
        if fields.custody > floor {
            return Err(IdentityKeyError::SealedUnreadableHere);
        }
        let secret = cipher
            .open(&alias(profile, endpoint), fields.header, fields.body)
            .map_err(IdentityKeyError::Cipher)?;
        let signing = SigningKey::from_bytes(&secret);
        if signing.verifying_key().to_bytes() != fields.public_key {
            return Err(IdentityKeyError::KeyMismatch);
        }
        Ok(Self {
            endpoint: endpoint.clone(),
            signing,
        })
    }
}

/// Where `endpoint`'s key file is: `<state_dir>/identity-keys/<EndpointId>.key`.
///
/// An `EndpointId`'s grammar (a lowercase letter, then lowercase letters,
/// digits, `.`, `_` and `-`) has no separator and no leading dot, so the
/// name stays one entry of that directory.
#[must_use]
pub fn key_path(paths: &ProfilePaths, endpoint: &EndpointId) -> PathBuf {
    paths
        .state_dir()
        .join(IDENTITY_KEYS_DIR)
        .join(format!("{}.key", endpoint.as_str()))
}

/// Generate `endpoint`'s key and write its file, owner-only, never over
/// one that exists: an identity's key changes only by the person's act.
///
/// # Errors
/// [`IdentityKeyFileError::Exists`] when the identity already has a key;
/// a key or persistence error otherwise, after which no file was made.
pub fn create(
    paths: &ProfilePaths,
    endpoint: &EndpointId,
    cipher: &dyn KeyCipher,
) -> Result<IdentityKey, IdentityKeyFileError> {
    // REFUSED BEFORE THE SEAL: a sealing platform generates a new key at
    // the alias on every seal, so sealing first would replace the key an
    // existing record needs (#377 review F2). The exclusive create below
    // still decides a race; the caller holds the profile's client lock.
    if file_present(&key_path(paths, endpoint))? {
        return Err(IdentityKeyFileError::Exists);
    }
    let key = IdentityKey::generate(endpoint.clone())?;
    let line = key.to_record(paths.profile(), cipher)?;
    match persist::create_private_exclusive_within(
        &key_path(paths, endpoint),
        line.as_bytes(),
        paths.boundary(),
    ) {
        Ok(()) => Ok(key),
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

/// Open `endpoint`'s key from its file, under `cipher`'s custody: `None`
/// when the identity has no key file.
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
    let Some(line) = interweave_profile_config::read_private_within(
        &key_path(paths, endpoint),
        paths.boundary(),
        MAX_RECORD_BYTES,
    )
    .map_err(|e| IdentityKeyFileError::Persist(e.to_string()))?
    else {
        return Ok(None);
    };
    let line = Zeroizing::new(line);
    Ok(Some(IdentityKey::from_record(
        &line,
        paths.profile(),
        endpoint,
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
    let removed = persist::remove_private_within(&key_path(paths, endpoint), paths.boundary())
        .map_err(|e| IdentityKeyFileError::Persist(e.to_string()))?;
    cipher
        .retire(&alias(paths.profile(), endpoint))
        .map_err(IdentityKeyError::Cipher)?;
    Ok(removed)
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
    /// The identity already has a key file.
    Exists,
    /// The identity's key file is gone after its card was shared, so
    /// counterparties hold a key this device no longer has: never made
    /// again silently, only by the person's confirmed new key, which they
    /// see as a key change (ADR-0063 decision 1, architect-cto's ruling
    /// of 2026-10-24).
    Missing,
    /// The key or its record refused.
    Key(IdentityKeyError),
    /// The file or its directory could not be written, read or removed,
    /// or is not private; the persistence layer's own message.
    Persist(String),
}

impl From<IdentityKeyError> for IdentityKeyFileError {
    fn from(e: IdentityKeyError) -> Self {
        Self::Key(e)
    }
}

impl core::fmt::Display for IdentityKeyFileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Exists => f.write_str("the identity already has a key"),
            Self::Missing => {
                f.write_str("the identity's key file is gone, and its card was shared")
            }
            Self::Key(e) => e.fmt(f),
            Self::Persist(e) => write!(f, "the identity's key file: {e}"),
        }
    }
}

impl core::error::Error for IdentityKeyFileError {}

fn header(custody: Custody, endpoint: &EndpointId, public_key: &[u8; 32]) -> Vec<u8> {
    let name = endpoint.as_str().as_bytes();
    let mut out = Vec::with_capacity(4 + 3 + name.len() + 32);
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(custody.byte());
    // An EndpointId is at most 64 ASCII bytes (its grammar), so its
    // length fits the one byte with room to spare.
    out.push(u8::try_from(name.len()).unwrap_or(u8::MAX));
    out.extend_from_slice(name);
    out.extend_from_slice(public_key);
    out
}

/// A record's fields, borrowed from its bytes.
struct Fields<'a> {
    custody: Custody,
    endpoint: &'a [u8],
    public_key: [u8; 32],
    header: &'a [u8],
    body: &'a [u8],
}

impl<'a> Fields<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, IdentityKeyError> {
        let magic = bytes.get(..4).ok_or(IdentityKeyError::NotARecord)?;
        if magic != MAGIC {
            return Err(IdentityKeyError::NotARecord);
        }
        let version = *bytes.get(4).ok_or(IdentityKeyError::Truncated)?;
        if version != VERSION {
            return Err(IdentityKeyError::UnsupportedVersion(version));
        }
        let custody = bytes
            .get(5)
            .and_then(|b| Custody::from_byte(*b))
            .ok_or(IdentityKeyError::UnknownCustody)?;
        let name_len = usize::from(*bytes.get(6).ok_or(IdentityKeyError::Truncated)?);
        let name_end = 7 + name_len;
        let endpoint = bytes.get(7..name_end).ok_or(IdentityKeyError::Truncated)?;
        let key_end = name_end + 32;
        let public_key: [u8; 32] = bytes
            .get(name_end..key_end)
            .ok_or(IdentityKeyError::Truncated)?
            .try_into()
            .map_err(|_| IdentityKeyError::Truncated)?;
        Ok(Self {
            custody,
            endpoint,
            public_key,
            header: &bytes[..key_end],
            body: &bytes[key_end..],
        })
    }
}

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
    /// The public key is not the secret's.
    KeyMismatch,
    /// A filesystem-only record where the platform holds a wrapping key:
    /// refused, never re-sealed; the person creates a new key.
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
            Self::KeyMismatch => f.write_str("the identity-key record's public key is not its secret's"),
            Self::CustodyDowngrade => f.write_str(
                "the identity key is held below this platform's custody; create a new key, which contacts will see as a key change",
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

    fn human() -> EndpointId {
        EndpointId::parse("human").expect("an endpoint")
    }

    fn hex(bytes: &[u8]) -> String {
        use core::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
    }

    #[test]
    fn a_record_round_trips_at_the_filesystem_floor() {
        let key = IdentityKey::generate(human()).expect("a key");
        let line = key
            .to_record("default", &FilesystemCustody)
            .expect("a record");
        let back = IdentityKey::from_record(&line, "default", &human(), &FilesystemCustody)
            .expect("opens");
        assert_eq!(back.public_key(), key.public_key());
        assert_eq!(back.sign(b"m"), key.sign(b"m"));
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
        let key = IdentityKey::generate(human()).expect("a key");
        let shown = format!("{key:?}");
        let secret = key.signing.to_bytes();
        assert!(!shown.contains(&STANDARD.encode(secret)));
        assert!(!shown.contains(&hex(&secret)));
        assert!(shown.contains(&key.fingerprint().to_string()));
    }

    #[test]
    fn the_alias_is_the_identity_keys_own_namespace() {
        assert_eq!(alias("default", &human()), "interweave.idk1.default.human");
        assert!(!alias("default", &human()).starts_with("interweave.iwk1."));
    }
}
