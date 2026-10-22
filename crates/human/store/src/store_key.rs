// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The store's own key (ADR-0060 decisions 2 and 3).
//!
//! 32 bytes from the system's CSPRNG, one per profile, never derived from
//! the identity seed, the recovery phrase, an identity's key or any
//! transport material: losing it loses the store and nothing else. It is
//! held in memory only while the store is open, zeroized when dropped.
//!
//! # Where it is kept
//!
//! In one owner-only file, `<state_dir>/human-store.key` -- under the
//! profile's state root and OUTSIDE the store's own directory
//! `<state_dir>/human`, so a copy of that directory leaves the key behind;
//! and never in the daemon's identity directory. The platform's custody
//! seals it ([`KeyCipher`]): the Android Keystore at `background-compatible`
//! under the alias [`alias`] (`interweave.store.<profile>`), apart from the
//! seed's aliases and every identity's; on the v1 desktop the file alone,
//! ADR-0038's `filesystem-only`.
//!
//! # The IWSK record
//!
//! The file holds one line: the standard base64 of
//!
//! ```text
//! b"IWSK" | version (1) | custody (1) | body
//! ```
//!
//! where the body is the 32-byte key at [`Custody::FilesystemOnly`] and, at
//! [`Custody::Keystore`], the key sealed by the platform's wrapping key with
//! the six bytes before it as associated data. Opening refuses another
//! magic (an identity's IDK1 record or the seed's IWK1 among them), a
//! version this build does not read, and a custody other than the
//! platform's: below it a downgrade, above it unreadable here.

use std::path::PathBuf;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use zeroize::Zeroizing;

use interweave_profile_config::PersistError;
use interweave_profile_config::paths::ProfilePaths;
use interweave_profile_config::persist;

use crate::key_custody::{CipherFailure, Custody, KeyCipher};

/// The record's magic.
pub const MAGIC: &[u8; 4] = b"IWSK";
/// The one version this build writes and reads.
pub const VERSION: u8 = 1;
/// The key file's name in the profile's state directory.
pub const STORE_KEY_FILE: &str = "human-store.key";
/// A file larger than this is no record of this build's.
pub const MAX_RECORD_BYTES: u64 = 256;

const HEADER_LEN: usize = 6;

/// The platform key's alias for one profile's store key: its own
/// namespace, never a seed's `interweave.iwk1.<profile>.<policy>` nor an
/// identity's `interweave.idk1.<profile>.<EndpointId>`.
#[must_use]
pub fn alias(profile: &str) -> String {
    format!("interweave.store.{profile}")
}

/// Where `paths`' store key lives.
#[must_use]
pub fn key_path(paths: &ProfilePaths) -> PathBuf {
    paths.state_dir().join(STORE_KEY_FILE)
}

/// The store key, in memory. No `Clone`, and a `Debug` that prints no byte.
pub struct StoreKey(Zeroizing<[u8; 32]>);

impl core::fmt::Debug for StoreKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("StoreKey(..)")
    }
}

impl StoreKey {
    /// A new key from the system's CSPRNG.
    ///
    /// # Errors
    /// [`StoreKeyError::Random`] when the system has no randomness to give.
    pub fn generate() -> Result<Self, StoreKeyError> {
        use rand::TryRng as _;
        let mut key = Zeroizing::new([0u8; 32]);
        rand::rngs::SysRng
            .try_fill_bytes(key.as_mut())
            .map_err(|e| StoreKeyError::Random(e.to_string()))?;
        Ok(Self(key))
    }

    /// The key as `SQLCipher`'s raw-key literal, `x'<64 hex>'`.
    pub(crate) fn sqlcipher_literal(&self) -> Zeroizing<String> {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = Zeroizing::new(String::with_capacity(67));
        out.push_str("x'");
        for b in self.0.iter() {
            out.push(char::from(HEX[usize::from(b >> 4)]));
            out.push(char::from(HEX[usize::from(b & 0x0f)]));
        }
        out.push('\'');
        out
    }

    /// The record line sealed by `cipher` for `profile`.
    ///
    /// # Errors
    /// [`StoreKeyError::Cipher`].
    pub fn to_record(
        &self,
        profile: &str,
        cipher: &dyn KeyCipher,
    ) -> Result<Zeroizing<String>, StoreKeyError> {
        let header = header(cipher.custody());
        let body = Zeroizing::new(
            cipher
                .seal(&alias(profile), &header, &self.0)
                .map_err(StoreKeyError::Cipher)?,
        );
        let mut record = Zeroizing::new(header.to_vec());
        record.extend_from_slice(&body);
        Ok(Zeroizing::new(STANDARD.encode(record.as_slice())))
    }

    /// The key a record line holds, opened by `cipher` for `profile`.
    ///
    /// # Errors
    /// [`StoreKeyError`]: not a record, another version, a custody other
    /// than the platform's, or the platform's refusal.
    pub fn from_record(
        line: &str,
        profile: &str,
        cipher: &dyn KeyCipher,
    ) -> Result<Self, StoreKeyError> {
        let bytes = Zeroizing::new(
            STANDARD
                .decode(line.trim_end_matches('\n'))
                .map_err(|_| StoreKeyError::NotARecord)?,
        );
        if bytes.len() < HEADER_LEN || &bytes[..4] != MAGIC {
            return Err(StoreKeyError::NotARecord);
        }
        if bytes[4] != VERSION {
            return Err(StoreKeyError::Version(bytes[4]));
        }
        let custody = Custody::from_byte(bytes[5]).ok_or(StoreKeyError::NotARecord)?;
        let floor = cipher.custody();
        if custody < floor {
            return Err(StoreKeyError::CustodyDowngrade);
        }
        if custody > floor {
            return Err(StoreKeyError::SealedUnreadableHere);
        }
        let (header, body) = bytes.split_at(HEADER_LEN);
        let key = cipher
            .open(&alias(profile), header, body)
            .map_err(StoreKeyError::Cipher)?;
        Ok(Self(key))
    }
}

fn header(custody: Custody) -> [u8; HEADER_LEN] {
    let mut out = [0u8; HEADER_LEN];
    out[..4].copy_from_slice(MAGIC);
    out[4] = VERSION;
    out[5] = custody.byte();
    out
}

/// Why a record holds no key this build can use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreKeyError {
    /// Not an IWSK record: another magic, a truncated or damaged file.
    NotARecord,
    /// An IWSK record of a version this build does not read.
    Version(u8),
    /// A filesystem-only record where the platform holds a wrapping key.
    CustodyDowngrade,
    /// A sealed record on a platform with no wrapping key.
    SealedUnreadableHere,
    /// The platform's cipher refused.
    Cipher(CipherFailure),
    /// The system's CSPRNG failed.
    Random(String),
}

impl StoreKeyError {
    /// Whether the platform reports the key invalidated (ADR-0060
    /// decision 6's key-invalidated condition), as against missing,
    /// damaged or refused.
    #[must_use]
    pub const fn is_invalidated(&self) -> bool {
        matches!(self, Self::Cipher(CipherFailure::KeyInvalidated))
    }
}

impl core::fmt::Display for StoreKeyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotARecord => f.write_str("the store's key file is not a key record"),
            Self::Version(v) => write!(
                f,
                "the store's key file is of version {v}, which this build does not read"
            ),
            Self::CustodyDowngrade => f.write_str(
                "the store's key file is held below this platform's custody and is refused",
            ),
            Self::SealedUnreadableHere => {
                f.write_str("the store's key file is sealed by a platform key this device lacks")
            }
            Self::Cipher(CipherFailure::KeyInvalidated) => {
                f.write_str("the platform has invalidated the store's key")
            }
            Self::Cipher(CipherFailure::KeyUnavailable) => {
                f.write_str("the platform holds no key for the store")
            }
            Self::Cipher(CipherFailure::Unauthentic) => {
                f.write_str("the store's key file does not open under the platform's key")
            }
            Self::Cipher(CipherFailure::Platform(e)) => {
                write!(f, "the platform could not open the store's key: {e}")
            }
            Self::Random(e) => write!(f, "no randomness for the store's key: {e}"),
        }
    }
}

impl core::error::Error for StoreKeyError {}

/// The key file's failures: the record's, or the file's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreKeyFileError {
    /// [`create`] found a key file already there.
    Exists,
    /// The record holds no usable key.
    Key(StoreKeyError),
    /// The file could not be read, written or removed.
    Persist(String),
}

impl From<StoreKeyError> for StoreKeyFileError {
    fn from(e: StoreKeyError) -> Self {
        Self::Key(e)
    }
}

impl core::fmt::Display for StoreKeyFileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Exists => f.write_str("the store already has a key file"),
            Self::Key(e) => e.fmt(f),
            Self::Persist(e) => write!(f, "the store's key file: {e}"),
        }
    }
}

impl core::error::Error for StoreKeyFileError {}

/// Generate a key, seal it and write its file, synced, refusing to replace
/// one: a second key would orphan the store the first one opens.
///
/// # Errors
/// [`StoreKeyFileError`].
pub fn create(paths: &ProfilePaths, cipher: &dyn KeyCipher) -> Result<StoreKey, StoreKeyFileError> {
    let key = StoreKey::generate()?;
    let line = key.to_record(paths.profile(), cipher)?;
    match persist::create_private_exclusive_within(
        &key_path(paths),
        line.as_bytes(),
        paths.boundary(),
    ) {
        Ok(()) => Ok(key),
        Err(PersistError::AlreadyExists) => Err(StoreKeyFileError::Exists),
        Err(e) => Err(StoreKeyFileError::Persist(e.to_string())),
    }
}

/// Whether `paths` holds a key file, without opening it.
///
/// # Errors
/// [`StoreKeyFileError::Persist`] when the state directory cannot be read.
pub fn exists(paths: &ProfilePaths) -> Result<bool, StoreKeyFileError> {
    match std::fs::symlink_metadata(key_path(paths)) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(StoreKeyFileError::Persist(e.to_string())),
    }
}

/// The key in `paths`' key file, or `None` when there is no file.
///
/// # Errors
/// [`StoreKeyFileError`]: the file is not private, unreadable, or holds no
/// usable key.
pub fn open(
    paths: &ProfilePaths,
    cipher: &dyn KeyCipher,
) -> Result<Option<StoreKey>, StoreKeyFileError> {
    let Some(line) = interweave_profile_config::read_private_within(
        &key_path(paths),
        paths.boundary(),
        MAX_RECORD_BYTES,
    )
    .map_err(|e| StoreKeyFileError::Persist(e.to_string()))?
    else {
        return Ok(None);
    };
    let line = Zeroizing::new(line);
    Ok(Some(StoreKey::from_record(&line, paths.profile(), cipher)?))
}

/// Remove `paths`' key file and retire the platform key under it; `true`
/// when there was a file.
///
/// # Errors
/// [`StoreKeyFileError`].
pub fn remove(paths: &ProfilePaths, cipher: &dyn KeyCipher) -> Result<bool, StoreKeyFileError> {
    let removed = persist::remove_private_within(&key_path(paths), paths.boundary())
        .map_err(|e| StoreKeyFileError::Persist(e.to_string()))?;
    cipher
        .retire(&alias(paths.profile()))
        .map_err(StoreKeyError::Cipher)?;
    Ok(removed)
}
