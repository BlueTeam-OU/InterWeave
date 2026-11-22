// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An endpoint's key file: the IDK1 record and its custody on the
//! filesystem (ADR-0067 decisions 1 to 4, ADR-0063 decision 2).
//!
//! Every endpoint is an Ed25519 key pair generated from the system's
//! CSPRNG, never from the node's seed, and its `EndpointId` is derived
//! from the public key ([`EndpointId::from_ed25519_public`]). The key is
//! kept in one file per endpoint, `<state_dir>/endpoint-keys/<EndpointId>.key`
//! ([`ENDPOINT_KEYS_DIR`]), owner-only in an owner-only directory, and a
//! configuration entry names that file by its name alone
//! ([`parse_key_file_name`]).
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
//! Every byte before the body is the HEADER ([`header_bytes`]): the
//! public part, and the associated data a sealed body binds. The body is
//! the 32-byte secret at [`Custody::FilesystemOnly`] and, at
//! [`Custody::Keystore`], the secret sealed by a platform key the file
//! does not hold -- which this crate never holds: sealing and opening a
//! Keystore body is the human store's (`key_custody.rs`), which encodes
//! and decodes the record with [`encode_record`] and [`RawRecord`].
//!
//! # What a transport process reads of it
//!
//! The PUBLIC PART ALONE ([`KeyHeader::from_line`], [`read_header`]): no
//! transport process unseals an endpoint key or signs with one
//! (ENDPOINTS.md section Configured endpoint set). The header reader
//! decodes only the base64 characters that cover the header, so a body
//! that is garbage, truncated or sealed is never decoded
//! (`the_header_reader_never_decodes_the_body`).
//!
//! # What lives here, and what does not
//!
//! The record's layout, the key file's name and path, the header reader
//! and the filesystem-custody mint ([`mint_filesystem`], what
//! `transportctl endpoint init` calls). The Keystore cipher, the
//! fingerprint, signing, and the identity's removal and replacement stay
//! with the human store.

use std::path::PathBuf;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::SigningKey;
use interweave_transport_api::{EndpointId, IdError};
use zeroize::Zeroizing;

use crate::PersistError;
use crate::paths::ProfilePaths;
use crate::private_read::PrivateReadError;

/// The record's magic.
pub const MAGIC: &[u8; 4] = b"IDK1";
/// The one version this build writes and reads.
pub const VERSION: u8 = 1;
/// The directory under the profile's state root that holds endpoint keys
/// (ADR-0067 decision 3).
pub const ENDPOINT_KEYS_DIR: &str = "endpoint-keys";
/// The directory a release before Stage 22's kept an identity's key in,
/// named by a label (ADR-0063 decision 2). Only its file's PRESENCE is
/// asked here, to name the command that adopts it
/// ([`crate::EndpointRemedy`]); nothing of it is read.
pub const LEGACY_IDENTITY_KEYS_DIR: &str = "identity-keys";
/// The suffix of a key file's name.
pub const KEY_FILE_SUFFIX: &str = ".key";
/// A file larger than this is no record of this build's.
pub const MAX_RECORD_BYTES: u64 = 1024;

/// Bytes before the endpoint name: magic, version, custody, the name's
/// length.
const FIXED_PREFIX: usize = 7;
/// The public key's length.
const PUBLIC_KEY_LEN: usize = 32;

/// Who holds the secret at rest, in increasing order of protection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Custody {
    /// An owner-only file and nothing more: ADR-0038's `filesystem-only`,
    /// the desktop's.
    FilesystemOnly,
    /// Sealed by a platform key the file does not hold: Android's
    /// `AndroidKeyStore`.
    Keystore,
}

impl Custody {
    /// The record's custody byte.
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Self::FilesystemOnly => 1,
            Self::Keystore => 2,
        }
    }

    /// The custody a record's byte names, `None` for one this build does
    /// not know.
    #[must_use]
    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(Self::FilesystemOnly),
            2 => Some(Self::Keystore),
            _ => None,
        }
    }
}

/// Why a record is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordError {
    /// Not an IDK1 record: another magic (a seed record among them), not
    /// base64, or a line longer than any record.
    NotARecord,
    /// An IDK1 record of a version this build does not read.
    UnsupportedVersion(u8),
    /// A custody byte this build does not know.
    UnknownCustody,
    /// The record ends before its header does.
    Truncated,
    /// The header's endpoint id is not the one its own public key derives
    /// -- a label's header among them: one of the two was written by
    /// hand.
    HeaderNotDerived,
}

impl core::fmt::Display for RecordError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotARecord => f.write_str("the file is not an endpoint-key record"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "the endpoint-key record is version {v}, which this build does not read"
            ),
            Self::UnknownCustody => {
                f.write_str("the endpoint-key record names a custody this build does not know")
            }
            Self::Truncated => f.write_str("the endpoint-key record is truncated"),
            Self::HeaderNotDerived => f.write_str(
                "the endpoint-key record's id does not derive from the public key beside it",
            ),
        }
    }
}

impl core::error::Error for RecordError {}

/// The record's bytes before the body for `endpoint` (its id's text, or
/// for a record a release before Stage 22's wrote, its label) and
/// `public_key` under `custody`: the associated data a sealed body binds.
///
/// # Errors
/// [`RecordError::NotARecord`] for an endpoint name longer than the one
/// length byte can say; every `EndpointId` fits.
pub fn header_bytes(
    custody: Custody,
    endpoint: &[u8],
    public_key: &[u8; 32],
) -> Result<Vec<u8>, RecordError> {
    let len = u8::try_from(endpoint.len()).map_err(|_| RecordError::NotARecord)?;
    let mut out = Vec::with_capacity(FIXED_PREFIX + endpoint.len() + PUBLIC_KEY_LEN);
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(custody.byte());
    out.push(len);
    out.extend_from_slice(endpoint);
    out.extend_from_slice(public_key);
    Ok(out)
}

/// The file's one line for a record of `header` (from [`header_bytes`])
/// and `body`.
#[must_use]
pub fn encode_record(header: &[u8], body: &[u8]) -> Zeroizing<String> {
    let mut record = Zeroizing::new(Vec::with_capacity(header.len() + body.len()));
    record.extend_from_slice(header);
    record.extend_from_slice(body);
    Zeroizing::new(STANDARD.encode(record.as_slice()))
}

/// The bytes of a record's line, decoded whole -- body included, for the
/// human store's open; a transport process reads [`KeyHeader::from_line`]
/// instead.
///
/// # Errors
/// [`RecordError::NotARecord`] for a line that is not base64.
pub fn decode_line(line: &str) -> Result<Zeroizing<Vec<u8>>, RecordError> {
    STANDARD
        .decode(line.trim_end_matches('\n'))
        .map(Zeroizing::new)
        .map_err(|_| RecordError::NotARecord)
}

/// A record's fields, borrowed from its decoded bytes, judged for their
/// layout and nothing else: the endpoint name is whatever the header
/// says (a label in a release before Stage 22's), and the public key is
/// not compared with it -- [`KeyHeader`] does that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawRecord<'a> {
    /// The custody byte, known to this build.
    pub custody: Custody,
    /// The endpoint name in the header, as bytes.
    pub endpoint: &'a [u8],
    /// The public key.
    pub public_key: [u8; 32],
    /// Every byte before the body: the associated data.
    pub header: &'a [u8],
    /// The body: the secret, or the sealed secret.
    pub body: &'a [u8],
}

impl<'a> RawRecord<'a> {
    /// The fields of `bytes`, a record's decoded line.
    ///
    /// # Errors
    /// [`RecordError::NotARecord`] for another magic,
    /// [`RecordError::UnsupportedVersion`], [`RecordError::UnknownCustody`]
    /// and [`RecordError::Truncated`].
    pub fn parse(bytes: &'a [u8]) -> Result<Self, RecordError> {
        let header_len = header_len(bytes)?;
        if bytes.len() < header_len {
            return Err(RecordError::Truncated);
        }
        let (header, body) = bytes.split_at(header_len);
        let custody = Custody::from_byte(header[5]).ok_or(RecordError::UnknownCustody)?;
        let endpoint = &header[FIXED_PREFIX..header_len - PUBLIC_KEY_LEN];
        let mut public_key = [0_u8; PUBLIC_KEY_LEN];
        public_key.copy_from_slice(&header[header_len - PUBLIC_KEY_LEN..]);
        Ok(Self {
            custody,
            endpoint,
            public_key,
            header,
            body,
        })
    }
}

/// The header's length as a record's fixed fields say it, each judged on
/// the way. `bytes` may be a prefix of the record: only its first
/// [`FIXED_PREFIX`] bytes are read.
fn header_len(bytes: &[u8]) -> Result<usize, RecordError> {
    let magic = bytes.get(..MAGIC.len()).ok_or(RecordError::NotARecord)?;
    if magic != MAGIC {
        return Err(RecordError::NotARecord);
    }
    let version = *bytes.get(4).ok_or(RecordError::Truncated)?;
    if version != VERSION {
        return Err(RecordError::UnsupportedVersion(version));
    }
    let custody = *bytes.get(5).ok_or(RecordError::Truncated)?;
    if Custody::from_byte(custody).is_none() {
        return Err(RecordError::UnknownCustody);
    }
    let name_len = usize::from(*bytes.get(6).ok_or(RecordError::Truncated)?);
    Ok(FIXED_PREFIX + name_len + PUBLIC_KEY_LEN)
}

/// A key file's public part: what a transport process reads of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyHeader {
    custody: Custody,
    endpoint: EndpointId,
    public_key: [u8; 32],
}

impl KeyHeader {
    /// The header of a record's line, read without decoding its body.
    ///
    /// Only the base64 characters covering the header are decoded -- at
    /// most two bytes of the body ride in its last group -- so the
    /// secret, sealed or not, is never decoded here, whatever the custody
    /// byte says.
    ///
    /// # Errors
    /// [`RecordError::NotARecord`] for another magic, a line that is not
    /// base64 where the header is, or one longer than any record;
    /// [`RecordError::UnsupportedVersion`], [`RecordError::UnknownCustody`],
    /// [`RecordError::Truncated`]; and [`RecordError::HeaderNotDerived`]
    /// for a header whose id is not the one its public key derives.
    pub fn from_line(line: &str) -> Result<Self, RecordError> {
        let text = line.trim_end_matches('\n');
        if text.len() as u64 > MAX_RECORD_BYTES {
            return Err(RecordError::NotARecord);
        }
        // The fixed fields first: seven bytes are in the first twelve
        // characters, whose third group ends at byte nine.
        let header_len = header_len(&decode_prefix(text, FIXED_PREFIX)?)?;
        let header = decode_prefix(text, header_len)?;
        let raw = RawRecord::parse(&header)?;
        let endpoint = core::str::from_utf8(raw.endpoint)
            .ok()
            .and_then(|s| EndpointId::parse_derived(s).ok())
            .ok_or(RecordError::HeaderNotDerived)?;
        if EndpointId::from_ed25519_public(&raw.public_key) != endpoint {
            return Err(RecordError::HeaderNotDerived);
        }
        Ok(Self {
            custody: raw.custody,
            endpoint,
            public_key: raw.public_key,
        })
    }

    /// The custody the record's byte names.
    #[must_use]
    pub const fn custody(&self) -> Custody {
        self.custody
    }

    /// The endpoint id, derived from [`Self::public_key`] by construction.
    #[must_use]
    pub const fn endpoint(&self) -> &EndpointId {
        &self.endpoint
    }

    /// The public key.
    #[must_use]
    pub const fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }
}

/// The first `want` bytes of `text`'s base64, decoding only the
/// characters that cover them (or all of `text` when it is shorter).
fn decode_prefix(text: &str, want: usize) -> Result<Zeroizing<Vec<u8>>, RecordError> {
    let chars = want.div_ceil(3) * 4;
    let prefix = text
        .get(..chars.min(text.len()))
        .ok_or(RecordError::NotARecord)?;
    let mut bytes = STANDARD
        .decode(prefix)
        .map(Zeroizing::new)
        .map_err(|_| RecordError::NotARecord)?;
    bytes.truncate(want);
    Ok(bytes)
}

/// Why a `key` value is not the name of an endpoint key file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyNameError {
    /// It holds a path separator: the directory is fixed, and a `key` is
    /// a file's name, never a path.
    Separator,
    /// It holds `..`.
    ParentDir,
    /// It does not end in [`KEY_FILE_SUFFIX`] -- a bare id among them.
    Suffix,
    /// Its stem is not a derived `EndpointId`.
    Stem(IdError),
}

impl core::fmt::Display for KeyNameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Separator => f.write_str("it holds a path separator, and a key is a file name"),
            Self::ParentDir => f.write_str("it holds `..`"),
            Self::Suffix => write!(f, "it does not end in `{KEY_FILE_SUFFIX}`"),
            Self::Stem(e) => write!(f, "its stem is not a derived EndpointId: {e}"),
        }
    }
}

/// The `EndpointId` a configuration entry's `key` value names: exactly
/// `<EndpointId>.key`, the stem passing [`EndpointId::parse_derived`].
///
/// # Errors
/// One [`KeyNameError`] per shape refused, in the order they are tested:
/// a separator, a `..`, the suffix, the stem.
pub fn parse_key_file_name(name: &str) -> Result<EndpointId, KeyNameError> {
    if name.contains('/') || name.contains('\\') {
        return Err(KeyNameError::Separator);
    }
    if name.contains("..") {
        return Err(KeyNameError::ParentDir);
    }
    let stem = name
        .strip_suffix(KEY_FILE_SUFFIX)
        .ok_or(KeyNameError::Suffix)?;
    EndpointId::parse_derived(stem).map_err(KeyNameError::Stem)
}

/// The key file's name for `endpoint`: `<EndpointId>.key`.
#[must_use]
pub fn key_file_name(endpoint: &EndpointId) -> String {
    format!("{}{KEY_FILE_SUFFIX}", endpoint.as_str())
}

/// Where `endpoint`'s key file is: `<state_dir>/endpoint-keys/<EndpointId>.key`.
#[must_use]
pub fn key_path(paths: &ProfilePaths, endpoint: &EndpointId) -> PathBuf {
    paths
        .state_dir()
        .join(ENDPOINT_KEYS_DIR)
        .join(key_file_name(endpoint))
}

/// Why a key file's header could not be read. Its text names what is
/// wrong with the file, never anything read from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFileError {
    /// The file, or its directory, is open to another account, a link,
    /// or not a regular file.
    NotPrivate,
    /// Larger than [`MAX_RECORD_BYTES`].
    TooLarge,
    /// The read failed with this kind.
    Io(std::io::ErrorKind),
    /// The record's header was refused.
    Record(RecordError),
}

impl core::fmt::Display for KeyFileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotPrivate => f.write_str("the key file or its directory is not private"),
            Self::TooLarge => write!(f, "the key file is larger than {MAX_RECORD_BYTES} bytes"),
            Self::Io(kind) => write!(f, "the key file could not be read: {kind}"),
            Self::Record(e) => e.fmt(f),
        }
    }
}

impl core::error::Error for KeyFileError {}

impl From<PrivateReadError> for KeyFileError {
    fn from(e: PrivateReadError) -> Self {
        match e {
            PrivateReadError::NotPrivate { .. } => Self::NotPrivate,
            PrivateReadError::TooLarge => Self::TooLarge,
            PrivateReadError::Read(e) => Self::Io(e.kind()),
        }
    }
}

/// The header of `endpoint`'s key file, its public part alone: `None`
/// when the file, or its directory, is absent.
///
/// Read by profile-config's one reader of private state files (regular,
/// not a link, this uid's, readable by nobody else, bounded, its
/// directory private under the boundary); [`KeyHeader::from_line`]
/// judges what it read. The header's id is NOT compared with `endpoint`
/// here: the loader names that refusal (`EndpointKeyMismatch`).
///
/// # Errors
/// A [`KeyFileError`].
pub fn read_header(
    paths: &ProfilePaths,
    endpoint: &EndpointId,
) -> Result<Option<KeyHeader>, KeyFileError> {
    let Some(text) = crate::read_private_within(
        &key_path(paths, endpoint),
        paths.boundary(),
        MAX_RECORD_BYTES,
    )?
    else {
        return Ok(None);
    };
    let text = Zeroizing::new(text);
    KeyHeader::from_line(&text)
        .map(Some)
        .map_err(KeyFileError::Record)
}

/// Why [`mint_filesystem`] made no key.
#[derive(Debug)]
pub enum MintError {
    /// The system's CSPRNG supplied no bytes: no key is made from
    /// anything else.
    Random(String),
    /// The file could not be written; nothing was installed under its
    /// name unless the error is [`PersistError::Unsynced`].
    Persist(PersistError),
}

impl core::fmt::Display for MintError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Random(e) => write!(
                f,
                "the system could not supply random bytes for an endpoint key: {e}"
            ),
            Self::Persist(e) => write!(f, "the endpoint key file could not be written: {e}"),
        }
    }
}

impl core::error::Error for MintError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Random(_) => None,
            Self::Persist(e) => Some(e),
        }
    }
}

/// Mint a new endpoint key at filesystem custody for `paths`' profile
/// and return the `EndpointId` it derives (ADR-0067 decision 4, what
/// `transportctl endpoint init` calls).
///
/// The secret comes from the system's CSPRNG and from nothing else --
/// never the node's seed. The record is written owner-only in an
/// owner-only `endpoint-keys/`, by the exclusive create that never
/// replaces a file: a fresh key's id has no file yet, and if one stood
/// there the write is refused rather than another key put in its place.
///
/// # Errors
/// [`MintError`].
pub fn mint_filesystem(paths: &ProfilePaths) -> Result<EndpointId, MintError> {
    mint_filesystem_at(paths, |endpoint| key_path(paths, endpoint))
}

/// The file a key is minted into under `alias` before its id has a name
/// anyone reads: `<state_dir>/endpoint-keys/.pending-<alias>.key`.
///
/// Nothing enumerates `endpoint-keys/` -- the daemon opens the one file
/// an entry names, by [`key_path`] -- and an alias has no leading dot, so
/// this is no key's name; `transportctl endpoint init --alias` finds a
/// run it interrupted by this name, and publishes the file under its id
/// ([`crate::persist::rename_private_no_replace_within`]).
#[must_use]
pub fn pending_key_path(paths: &ProfilePaths, alias: &crate::EndpointAlias) -> PathBuf {
    paths
        .state_dir()
        .join(ENDPOINT_KEYS_DIR)
        .join(format!(".pending-{alias}{KEY_FILE_SUFFIX}"))
}

/// [`mint_filesystem`] into the path `target` names for the id the new
/// key derives: [`pending_key_path`]'s file for `init --alias`, where the
/// id is known only once the key exists.
///
/// # Errors
/// [`MintError`]; a file already at the target is
/// `Persist(AlreadyExists)`, never replaced.
pub fn mint_filesystem_at(
    paths: &ProfilePaths,
    target: impl FnOnce(&EndpointId) -> PathBuf,
) -> Result<EndpointId, MintError> {
    use rand::TryRng as _;
    let mut secret = Zeroizing::new([0_u8; 32]);
    rand::rngs::SysRng
        .try_fill_bytes(secret.as_mut())
        .map_err(|e| MintError::Random(e.to_string()))?;
    let signing = SigningKey::from_bytes(&secret);
    let public_key = signing.verifying_key().to_bytes();
    let endpoint = EndpointId::from_ed25519_public(&public_key);
    let header = header_bytes(
        Custody::FilesystemOnly,
        endpoint.as_str().as_bytes(),
        &public_key,
    )
    .map_err(|_| MintError::Persist(PersistError::Io(std::io::ErrorKind::InvalidData.into())))?;
    // The filesystem custody's seal is the identity: the file is the only
    // protection, so the body is the secret itself.
    let line = encode_record(&header, secret.as_slice());
    crate::create_private_exclusive_within(&target(&endpoint), line.as_bytes(), paths.boundary())
        .map_err(MintError::Persist)?;
    Ok(endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(custody: Custody, public_key: &[u8; 32], body: &[u8]) -> Zeroizing<String> {
        let id = EndpointId::from_ed25519_public(public_key);
        let header = header_bytes(custody, id.as_str().as_bytes(), public_key).expect("header");
        encode_record(&header, body)
    }

    #[test]
    fn a_header_round_trips_and_derives_its_id() {
        let pk = SigningKey::from_bytes(&[7; 32]).verifying_key().to_bytes();
        let line = record(Custody::FilesystemOnly, &pk, &[7; 32]);
        let header = KeyHeader::from_line(&line).expect("reads");
        assert_eq!(header.custody(), Custody::FilesystemOnly);
        assert_eq!(header.public_key(), &pk);
        assert_eq!(header.endpoint(), &EndpointId::from_ed25519_public(&pk));
    }

    /// THE HEADER READER NEVER DECODES THE BODY: a Keystore record whose
    /// body is not base64 at all, or cut off, still yields its header --
    /// and the control, the same garbage where the header is, is refused,
    /// so the reader does decode what it covers.
    #[test]
    fn the_header_reader_never_decodes_the_body() {
        let pk = [9_u8; 32];
        let id = EndpointId::from_ed25519_public(&pk);
        let header = header_bytes(Custody::Keystore, id.as_str().as_bytes(), &pk).expect("h");
        // 7 + 52 + 32 = 91 header bytes, covered by the first 31 groups
        // (124 characters, 93 bytes): what follows is body alone.
        assert_eq!(header.len(), 91);
        let clean = encode_record(&header, &[0xa5; 60]);
        let covered = &clean[..124];
        for body in ["!!!!not base64 at all~~", "", "QQ", "%"] {
            let line = format!("{covered}{body}");
            let read = KeyHeader::from_line(&line).expect("the header alone is read");
            assert_eq!(read.custody(), Custody::Keystore);
            assert_eq!(read.endpoint(), &id);
            // And the whole-line decode a store's open makes does refuse it.
            if !body.is_empty() {
                assert_eq!(decode_line(&line).err(), Some(RecordError::NotARecord));
            }
        }
        let broken = format!("{}!!!!{}", &clean[..8], &clean[12..]);
        assert_eq!(
            KeyHeader::from_line(&broken).err(),
            Some(RecordError::NotARecord)
        );
    }

    #[test]
    fn the_header_reader_refuses_each_malformed_shape() {
        let pk = [3_u8; 32];
        let id = EndpointId::from_ed25519_public(&pk);
        let good = header_bytes(Custody::FilesystemOnly, id.as_str().as_bytes(), &pk).expect("h");
        let line = |bytes: &[u8]| STANDARD.encode(bytes);
        assert!(KeyHeader::from_line(&line(&good)).is_ok(), "the control");

        let mut magic = good.clone();
        magic[..4].copy_from_slice(b"IWK1");
        assert_eq!(
            KeyHeader::from_line(&line(&magic)).err(),
            Some(RecordError::NotARecord)
        );
        let mut version = good.clone();
        version[4] = 2;
        assert_eq!(
            KeyHeader::from_line(&line(&version)).err(),
            Some(RecordError::UnsupportedVersion(2))
        );
        let mut custody = good.clone();
        custody[5] = 9;
        assert_eq!(
            KeyHeader::from_line(&line(&custody)).err(),
            Some(RecordError::UnknownCustody)
        );
        assert_eq!(
            KeyHeader::from_line(&line(&good[..good.len() - 1])).err(),
            Some(RecordError::Truncated)
        );
        assert_eq!(
            KeyHeader::from_line(&line(&good[..5])).err(),
            Some(RecordError::Truncated)
        );
        assert_eq!(
            KeyHeader::from_line(&"A".repeat(2000)).err(),
            Some(RecordError::NotARecord),
            "longer than any record"
        );
        // A header naming another key's id, and one naming a label.
        let other = EndpointId::from_ed25519_public(&[4; 32]);
        let swapped =
            header_bytes(Custody::FilesystemOnly, other.as_str().as_bytes(), &pk).expect("h");
        assert_eq!(
            KeyHeader::from_line(&line(&swapped)).err(),
            Some(RecordError::HeaderNotDerived)
        );
        let label = header_bytes(Custody::FilesystemOnly, b"human", &pk).expect("h");
        assert_eq!(
            KeyHeader::from_line(&line(&label)).err(),
            Some(RecordError::HeaderNotDerived)
        );
        // The raw parse reads a label's record whole: what adopt needs.
        let raw_bytes = decode_line(&encode_record(&label, &[1; 32])).expect("b64");
        let raw = RawRecord::parse(&raw_bytes).expect("raw");
        assert_eq!(raw.endpoint, b"human");
        assert_eq!(raw.body, &[1; 32]);
        assert_eq!(raw.header, label.as_slice());
    }

    #[test]
    fn a_key_value_is_exactly_a_derived_ids_file_name() {
        let id = EndpointId::from_ed25519_public(&[5; 32]);
        let name = key_file_name(&id);
        assert_eq!(parse_key_file_name(&name), Ok(id.clone()), "the control");
        assert_eq!(
            parse_key_file_name(&format!("endpoint-keys/{name}")),
            Err(KeyNameError::Separator)
        );
        assert_eq!(
            parse_key_file_name(&format!("..{name}")),
            Err(KeyNameError::ParentDir)
        );
        assert_eq!(
            parse_key_file_name(id.as_str()),
            Err(KeyNameError::Suffix),
            "a bare id"
        );
        assert_eq!(
            parse_key_file_name(&format!("{}.pem", id.as_str())),
            Err(KeyNameError::Suffix)
        );
        assert!(matches!(
            parse_key_file_name("human.key"),
            Err(KeyNameError::Stem(_))
        ));
    }
}
