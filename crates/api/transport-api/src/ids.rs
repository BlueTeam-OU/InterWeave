// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Identifier newtypes with their grammars enforced at construction.
//!
//! Every type here is validate-on-construction and immutable after. The
//! alternative — a bare `String` checked wherever someone remembers — is
//! how a 65-byte `EndpointId` reaches a wire encoder that assumed `u8` fit.
//! Parsing at the boundary means the rest of the codebase can hold one of
//! these and know the grammar already held.

use core::fmt;

use serde::{Deserialize, Serialize};

/// Why a value did not satisfy its grammar.
///
/// Deliberately carries the specifics. These errors surface as
/// `InvalidArgument` to a caller, but a local diagnostic that says only
/// "invalid" costs an operator the one fact they needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    /// The value was empty; every identifier here has a minimum of one byte.
    Empty,
    /// The value exceeded its byte ceiling.
    TooLong {
        /// Bytes supplied.
        got: usize,
        /// Bytes permitted.
        max: usize,
    },
    /// A byte outside the grammar, reported with its position.
    IllegalByte {
        /// Zero-based index of the offending byte.
        index: usize,
        /// The byte itself.
        byte: u8,
    },
    /// The first character is constrained more tightly than the rest.
    IllegalLeadingByte {
        /// The offending first byte.
        byte: u8,
    },
    /// A fixed-width identifier was the wrong length.
    WrongLength {
        /// Characters supplied.
        got: usize,
        /// Characters required.
        expected: usize,
    },
    /// A transport identity was not a canonical `PeerId` string.
    NotCanonicalPeerId,
    /// An endpoint id was not derived from an Ed25519 public key: the
    /// contract's `EndpointIdNotDerived` refusal (ENDPOINTS.md, section `EndpointId`).
    NotDerived,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "value is empty"),
            Self::TooLong { got, max } => write!(f, "value is {got} bytes; the limit is {max}"),
            Self::IllegalByte { index, byte } => {
                write!(
                    f,
                    "byte {byte:#04x} at index {index} is outside the grammar"
                )
            }
            Self::IllegalLeadingByte { byte } => {
                write!(f, "leading byte {byte:#04x} is outside the grammar")
            }
            Self::WrongLength { got, expected } => {
                write!(
                    f,
                    "value is {got} characters; exactly {expected} are required"
                )
            }
            Self::NotDerived => write!(
                f,
                "EndpointIdNotDerived: an endpoint id is the canonical PeerId text of an \
                 Ed25519 public key (12D3KooW followed by 44 base58btc characters)"
            ),
            Self::NotCanonicalPeerId => write!(
                f,
                "not a canonical PeerId: expected 12D3KooW or Qm followed by 44 base58btc characters"
            ),
        }
    }
}

impl core::error::Error for IdError {}

/// The name of an endpoint's own Ed25519 key pair, derived from the public
/// key exactly as a libp2p `PeerId` is from the node's (ADR-0067 decision 2,
/// ENDPOINTS.md §EndpointId): the identity multihash over the protobuf-encoded
/// key, in canonical base58btc, 52 characters matching
/// `^12D3KooW[1-9A-HJ-NP-Za-km-z]{44}$`.
///
/// Not a second `PeerId`, not a person, not a role, and not an authorization
/// principal. Received from the network it is peer-asserted routing metadata:
/// a remote `source_endpoint` proves only that the authenticated peer claimed
/// it.
///
/// TRANSITIONAL: [`Self::parse`] still accepts the retired label grammar
/// `^[a-z][a-z0-9._-]{0,63}$` through `parse_legacy_label`, because the rest
/// of the workspace still writes labels. That path goes when every caller
/// derives its ids; [`Self::parse_derived`] is the strict constructor and
/// refuses a label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct EndpointId(String);

/// Every byte of a derived id before the key: the identity multihash code
/// and length (`0x00`, 36), then the libp2p public-key protobuf header
/// (field 1 varint `Ed25519`, field 2 bytes of length 32).
const DERIVED_PREFIX: [u8; 6] = [0x00, 0x24, 0x08, 0x01, 0x12, 0x20];
const DERIVED_BYTES: usize = 38;
/// Text length of a derived id: base58btc of 38 bytes that start `0x00`.
const DERIVED_TEXT_LEN: usize = 52;

impl EndpointId {
    /// Maximum length in bytes (ADR-0026).
    pub const MAX_BYTES: usize = 64;

    /// Derive the id of an endpoint from its Ed25519 public key.
    #[must_use]
    pub fn from_ed25519_public(key: &[u8; 32]) -> Self {
        let mut bytes = [0_u8; DERIVED_BYTES];
        bytes[..DERIVED_PREFIX.len()].copy_from_slice(&DERIVED_PREFIX);
        bytes[DERIVED_PREFIX.len()..].copy_from_slice(key);
        Self(bs58::encode(bytes).into_string())
    }

    /// Parse a derived id strictly.
    ///
    /// Accepts exactly the text [`Self::from_ed25519_public`] writes: it must
    /// decode to the 38 bytes `00 24 08 01 12 20 ‖ key` and re-encode to the
    /// input byte for byte, so there is one canonical form.
    ///
    /// # Errors
    /// [`IdError::Empty`] for an empty value, [`IdError::TooLong`] past
    /// [`Self::MAX_BYTES`], [`IdError::IllegalByte`] for a byte outside the
    /// base58btc alphabet, [`IdError::WrongLength`] for any other length than
    /// 52 characters, and [`IdError::NotDerived`] for a `Qm…` sha256 `PeerId`
    /// and for 52 characters that are not the canonical encoding of an
    /// Ed25519 key.
    pub fn parse_derived(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.is_empty() {
            return Err(IdError::Empty);
        }
        if bytes.len() > Self::MAX_BYTES {
            return Err(IdError::TooLong {
                got: bytes.len(),
                max: Self::MAX_BYTES,
            });
        }
        for (index, &b) in bytes.iter().enumerate() {
            let ok = b.is_ascii_alphanumeric() && !matches!(b, b'0' | b'O' | b'I' | b'l');
            if !ok {
                return Err(IdError::IllegalByte { index, byte: b });
            }
        }
        // A `Qm…` PeerId is well-formed text of another multihash, not a
        // short derived id: it gets the contract's refusal, not a length one.
        if value.starts_with("Qm") && bytes.len() == 46 {
            return Err(IdError::NotDerived);
        }
        if bytes.len() != DERIVED_TEXT_LEN {
            return Err(IdError::WrongLength {
                got: bytes.len(),
                expected: DERIVED_TEXT_LEN,
            });
        }
        let mut buf = [0_u8; 64];
        let len = bs58::decode(bytes)
            .onto(&mut buf[..])
            .map_err(|_| IdError::NotDerived)?;
        let decoded = &buf[..len];
        if decoded.len() != DERIVED_BYTES || decoded[..DERIVED_PREFIX.len()] != DERIVED_PREFIX {
            return Err(IdError::NotDerived);
        }
        let mut key = [0_u8; 32];
        key.copy_from_slice(&decoded[DERIVED_PREFIX.len()..]);
        let canonical = Self::from_ed25519_public(&key);
        if canonical.0 != value {
            return Err(IdError::NotDerived);
        }
        Ok(canonical)
    }

    /// Parse an `EndpointId`: the derived form, or (TRANSITIONAL) a legacy
    /// label.
    ///
    /// Text beginning `12D3KooW` or `Qm` can only be meant as a derived id
    /// (a label starts with a lowercase letter) and is judged by
    /// [`Self::parse_derived`]; anything else is judged as a label.
    ///
    /// # Errors
    /// The [`Self::parse_derived`] errors for PeerId-shaped text, else the
    /// label grammar's: [`IdError::Empty`], [`IdError::TooLong`],
    /// [`IdError::IllegalLeadingByte`] or [`IdError::IllegalByte`].
    pub fn parse(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if value.starts_with("12D3KooW") || value.starts_with("Qm") {
            Self::parse_derived(value)
        } else {
            Self::parse_legacy_label(value)
        }
    }

    /// The Ed25519 public key a derived id names; `None` for a legacy label.
    #[must_use]
    pub fn ed25519_public(&self) -> Option<[u8; 32]> {
        let mut buf = [0_u8; 64];
        let len = bs58::decode(self.0.as_bytes()).onto(&mut buf[..]).ok()?;
        if len != DERIVED_BYTES || buf[..DERIVED_PREFIX.len()] != DERIVED_PREFIX {
            return None;
        }
        let mut key = [0_u8; 32];
        key.copy_from_slice(&buf[DERIVED_PREFIX.len()..len]);
        Some(key)
    }

    /// TRANSITIONAL: the retired label grammar `^[a-z][a-z0-9._-]{0,63}$`.
    /// Delete this function and its call in [`Self::parse`] when the last
    /// caller that writes a label moves to a derived id.
    fn parse_legacy_label(value: String) -> Result<Self, IdError> {
        let bytes = value.as_bytes();
        let Some(&first) = bytes.first() else {
            return Err(IdError::Empty);
        };
        if bytes.len() > Self::MAX_BYTES {
            return Err(IdError::TooLong {
                got: bytes.len(),
                max: Self::MAX_BYTES,
            });
        }
        if !first.is_ascii_lowercase() {
            return Err(IdError::IllegalLeadingByte { byte: first });
        }
        for (index, &b) in bytes.iter().enumerate() {
            let ok =
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-');
            if !ok {
                return Err(IdError::IllegalByte { index, byte: b });
            }
        }
        Ok(Self(value))
    }

    /// The identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A logical broadcast channel — `^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$`.
///
/// Case-sensitive, and ASCII-only so no Unicode normalization question
/// exists (ADR-0025). The transport attaches no application meaning to it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ChannelId(String);

impl ChannelId {
    /// Maximum length in bytes (ADR-0026).
    pub const MAX_BYTES: usize = 128;

    /// Parse a `ChannelId`, enforcing the contract grammar.
    ///
    /// # Errors
    /// Returns [`IdError`] when the value is empty, longer than
    /// [`Self::MAX_BYTES`], starts with a non-alphanumeric byte, or
    /// contains a byte outside `[A-Za-z0-9._:/-]`.
    pub fn parse(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        let bytes = value.as_bytes();
        let Some(&first) = bytes.first() else {
            return Err(IdError::Empty);
        };
        if bytes.len() > Self::MAX_BYTES {
            return Err(IdError::TooLong {
                got: bytes.len(),
                max: Self::MAX_BYTES,
            });
        }
        if !first.is_ascii_alphanumeric() {
            return Err(IdError::IllegalLeadingByte { byte: first });
        }
        for (index, &b) in bytes.iter().enumerate() {
            let ok = b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'/' | b'-');
            if !ok {
                return Err(IdError::IllegalByte { index, byte: b });
            }
        }
        Ok(Self(value))
    }

    /// The identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A 128-bit message identifier.
///
/// Stored as bytes, not as text: the canonical JSON form is 32 lowercase
/// hex characters, and keeping the bytes canonical means the printable
/// form is derived rather than trusted. A `MessageId` that round-trips
/// through JSON therefore cannot acquire an uppercase spelling on the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId([u8; 16]);

impl MessageId {
    /// Characters in the canonical printable form.
    pub const HEX_CHARS: usize = 32;
    /// Length in bytes.
    ///
    /// Named because the direct-v2 frame puts the id FIRST, and a reader
    /// recovering it from a frame that failed to parse needs to say how
    /// many bytes that is without restating 16.
    pub const LEN: usize = 16;

    /// Wrap raw bytes. Any 128-bit value is a legal identifier.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The raw 16 bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Parse the canonical form: exactly 32 **lowercase** hex characters.
    ///
    /// Uppercase is rejected rather than normalized. Accepting it would
    /// make two spellings of one identifier, and the contract pins the
    /// canonical form precisely so independent implementations compare
    /// equal without agreeing on a normalization step first.
    ///
    /// # Errors
    /// Returns [`IdError`] when the length is not 32 or a character is not
    /// `[0-9a-f]`.
    pub fn parse_hex(value: &str) -> Result<Self, IdError> {
        let bytes = value.as_bytes();
        if bytes.len() != Self::HEX_CHARS {
            return Err(IdError::WrongLength {
                got: bytes.len(),
                expected: Self::HEX_CHARS,
            });
        }
        let mut out = [0u8; 16];
        // `as_chunks::<2>()`: the length was checked above, so the
        // remainder is empty; the constant chunk size is what the
        // toolchain's clippy asks for and what the pinned MSRV provides.
        let (pairs, _) = bytes.as_chunks::<2>();
        for (i, [a, b]) in pairs.iter().enumerate() {
            let hi = lower_hex_value(*a).ok_or(IdError::IllegalByte {
                index: i * 2,
                byte: *a,
            })?;
            let lo = lower_hex_value(*b).ok_or(IdError::IllegalByte {
                index: i * 2 + 1,
                byte: *b,
            })?;
            out[i] = (hi << 4) | lo;
        }
        Ok(Self(out))
    }

    /// The canonical 32-lowercase-hex form.
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(Self::HEX_CHARS);
        for b in self.0 {
            // Written out rather than `format!("{b:02x}")` in a loop so the
            // canonical form has exactly one implementation and no format
            // string can drift it.
            s.push(char::from(HEX_DIGITS[usize::from(b >> 4)]));
            s.push(char::from(HEX_DIGITS[usize::from(b & 0x0f)]));
        }
        s
    }
}

const HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";

const fn lower_hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for MessageId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for MessageId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse_hex(&s).map_err(serde::de::Error::custom)
    }
}

/// An opaque stable transport identity (a `PeerId` in the libp2p backend).
///
/// Deliberately a validated-length opaque string rather than a parsed
/// multihash: the neutral contract must not acquire a libp2p type, and no
/// consumer above this layer inspects the structure. Backends that need
/// the bytes parse it themselves.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct TransportIdentity(String);

impl TransportIdentity {
    /// Maximum length in bytes for the normalized string form.
    pub const MAX_BYTES: usize = 256;

    /// Parse a transport identity in its canonical `PeerId` form.
    ///
    /// Accepts exactly what `common/peer-id.schema.json` accepts: a
    /// `12D3KooW`-prefixed Ed25519 identity or a `Qm`-prefixed multihash,
    /// each followed by 44 base58btc characters.
    ///
    /// **Validated here rather than deferred to the backend.** This type
    /// crosses the neutral boundary — destinations arrive from IPC as
    /// JSON — and accepting any non-empty string meant `"garbage"`, or a
    /// value with control characters in it, passed the validation layer
    /// and failed much later inside a backend parser, with the error
    /// surfacing far from the input that caused it. Checking the shape
    /// costs nothing and needs no libp2p type: the grammar is a prefix, an
    /// alphabet, and a length.
    ///
    /// This proves the string is *well-formed*, not that the peer exists,
    /// is reachable, or is trusted. Nothing here is an authorization step.
    ///
    /// # Errors
    /// Returns [`IdError`] when the value is empty, exceeds
    /// [`Self::MAX_BYTES`], or is not a canonical `PeerId` string.
    pub fn parse(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(IdError::Empty);
        }
        if value.len() > Self::MAX_BYTES {
            return Err(IdError::TooLong {
                got: value.len(),
                max: Self::MAX_BYTES,
            });
        }
        Self::check_canonical(&value)?;
        Ok(Self(value))
    }

    /// `^(12D3KooW[1-9A-HJ-NP-Za-km-z]{44}|Qm[1-9A-HJ-NP-Za-km-z]{44})$`,
    /// **and** the multihash those characters actually encode.
    ///
    /// The pattern is what `common/peer-id.schema.json` can express: a
    /// JSON Schema matches text, so the prefix, the alphabet and the
    /// length are the whole of what it can assert. They say a string
    /// LOOKS like a `PeerId`. They do not say it IS one — the 44 tail
    /// characters are not free, they are a base58btc number whose bytes
    /// have a fixed structure, and most strings the pattern admits decode
    /// to something no libp2p parser will accept.
    ///
    /// Checking only the pattern put that failure exactly where the doc
    /// on `parse` says this type exists to prevent it: past the neutral
    /// boundary, inside a backend parser, far from the input that caused
    /// it. So the decode happens here, where the value is still a string
    /// and the error still names the field it came from.
    fn check_canonical(value: &str) -> Result<(), IdError> {
        let ed25519 = value.starts_with("12D3KooW");
        let rest = value
            .strip_prefix("12D3KooW")
            .or_else(|| value.strip_prefix("Qm"))
            .ok_or(IdError::NotCanonicalPeerId)?;
        if rest.len() != 44 {
            return Err(IdError::NotCanonicalPeerId);
        }
        // base58btc omits 0, O, I and l precisely so the remaining glyphs
        // cannot be confused by a human reading one aloud. Checked before
        // the decode so a bad glyph reports the same error either way,
        // rather than depending on which layer noticed.
        for &b in rest.as_bytes() {
            let ok = b.is_ascii_alphanumeric() && !matches!(b, b'0' | b'O' | b'I' | b'l');
            if !ok {
                return Err(IdError::NotCanonicalPeerId);
            }
        }

        // A fixed buffer, so a decode cannot allocate on input: both
        // accepted forms are far under it and anything longer is not one
        // of them, which `into_vec`'s length error reports.
        let mut bytes = [0_u8; 64];
        let len = bs58::decode(value)
            .onto(&mut bytes[..])
            .map_err(|_| IdError::NotCanonicalPeerId)?;
        let decoded = &bytes[..len];

        if ed25519 {
            // IDENTITY MULTIHASH of a libp2p Ed25519 public-key protobuf:
            // 0x00 identity code, 0x24 = 36 bytes, then the protobuf —
            // field 1 varint KeyType::Ed25519, field 2 bytes of length
            // 0x20. Every byte here is fixed except the key itself, which
            // is why the pattern's 44 free characters overstate the space
            // by a wide margin.
            if decoded.len() != 38 || decoded[..6] != [0x00, 0x24, 0x08, 0x01, 0x12, 0x20] {
                return Err(IdError::NotCanonicalPeerId);
            }
        } else {
            // SHA-256 MULTIHASH: 0x12 code, 0x20 = 32 bytes of digest.
            // The digest itself is opaque and unconstrained — this says
            // the envelope is a sha2-256 multihash, not that any
            // particular key hashes to it.
            if decoded.len() != 34 || decoded[..2] != [0x12, 0x20] {
                return Err(IdError::NotCanonicalPeerId);
            }
        }
        Ok(())
    }

    /// The identity as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! deserialize_via_parse {
    ($t:ty) => {
        impl<'de> Deserialize<'de> for $t {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                Self::parse(s).map_err(serde::de::Error::custom)
            }
        }
    };
}

// Deserialization goes through `parse` for every one of these. A derived
// impl would build a value that never satisfied its grammar, which is the
// whole failure mode these newtypes exist to prevent — and JSON arriving
// over IPC is exactly the untrusted path where it would happen.
deserialize_via_parse!(EndpointId);
deserialize_via_parse!(ChannelId);
deserialize_via_parse!(TransportIdentity);

/// Where a directed message is going.
///
/// An absent endpoint means the receiver's configured default endpoint —
/// never fan-out (ADR-0030). The distinction is preserved in the type so
/// no call site can express "broadcast to all endpoints" by omission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// CLOSED. An unknown property here is how a caller would try to smuggle
// a second destination, a source claim, or a fan-out hint into a type
// whose entire purpose is resolving to exactly ONE endpoint.
#[serde(deny_unknown_fields)]
pub struct DirectDestination {
    /// The remote transport identity.
    pub peer: TransportIdentity,
    /// The remote endpoint, or `None` for its configured default.
    ///
    /// Absent or an `EndpointId`. NOT `null`: absence means the receiver's
    /// configured default, and an explicit null would be a third state
    /// the contract does not define — on a type where the difference
    /// decides where a message goes.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "absent_or_endpoint"
    )]
    pub endpoint: Option<EndpointId>,
}

/// An optional `EndpointId` that may be ABSENT but never explicitly `null`.
fn absent_or_endpoint<'de, D>(deserializer: D) -> Result<Option<EndpointId>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    Option::<EndpointId>::deserialize(deserializer)?
        .map(Some)
        .ok_or_else(|| D::Error::custom("must be an endpoint id or omitted entirely, not null"))
}

impl DirectDestination {
    /// Address the peer's configured default endpoint.
    #[must_use]
    pub const fn to_default(peer: TransportIdentity) -> Self {
        Self {
            peer,
            endpoint: None,
        }
    }

    /// Address one explicit endpoint.
    #[must_use]
    pub const fn to_endpoint(peer: TransportIdentity, endpoint: EndpointId) -> Self {
        Self {
            peer,
            endpoint: Some(endpoint),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TEST-ONLY keys: three fixed 32-byte patterns, not real identities.
    const KEYS: [[u8; 32]; 3] = [[0x00; 32], [0x7f; 32], [0xff; 32]];

    #[test]
    fn a_derived_id_has_the_peer_id_shape_and_round_trips_its_key() {
        for key in KEYS {
            let id = EndpointId::from_ed25519_public(&key);
            let text = id.as_str();
            assert_eq!(text.len(), 52);
            assert!(text.starts_with("12D3KooW"), "{text}");
            assert_eq!(EndpointId::parse_derived(text), Ok(id.clone()));
            assert_eq!(EndpointId::parse(text), Ok(id.clone()));
            assert_eq!(id.ed25519_public(), Some(key));
        }
    }

    #[test]
    fn a_legacy_label_has_no_key() {
        // Control for the derived case above: the same accessor answers Some.
        let label = EndpointId::parse("human").expect("label");
        assert_eq!(label.ed25519_public(), None);
        assert!(
            EndpointId::from_ed25519_public(&KEYS[1])
                .ed25519_public()
                .is_some()
        );
    }

    #[test]
    fn parse_derived_refuses_a_sha256_peer_id_as_not_derived() {
        // A real-shaped Qm PeerId (sha256 multihash of 32 bytes).
        let mut bytes = vec![0x12, 0x20];
        bytes.extend_from_slice(&[0x5a; 32]);
        let qm = bs58::encode(bytes).into_string();
        assert!(qm.starts_with("Qm"), "{qm}");
        assert_eq!(
            EndpointId::parse_derived(qm.as_str()),
            Err(IdError::NotDerived)
        );
        assert_eq!(EndpointId::parse(qm.as_str()), Err(IdError::NotDerived));
        assert!(
            EndpointId::parse_derived(EndpointId::from_ed25519_public(&KEYS[1]).as_str()).is_ok()
        );
    }

    #[test]
    fn parse_derived_refuses_a_different_key_type_or_prefix() {
        // Identity multihash, but KeyType 2 (secp256k1), or a key length of
        // 33: the 52-character text exists, the bytes are not ours.
        for prefix in [
            [0x00, 0x24, 0x08, 0x02, 0x12, 0x20],
            [0x00, 0x24, 0x08, 0x01, 0x12, 0x21],
        ] {
            let mut bytes = prefix.to_vec();
            bytes.extend_from_slice(&[0x11; 32]);
            let text = bs58::encode(bytes).into_string();
            assert_eq!(text.len(), 52, "{text}");
            assert_eq!(
                EndpointId::parse_derived(text.as_str()),
                Err(IdError::NotDerived),
                "{text}"
            );
        }
        // Without the leading zero byte the text is not even 52 characters.
        let mut bytes = vec![0x01, 0x24, 0x08, 0x01, 0x12, 0x20];
        bytes.extend_from_slice(&[0x11; 32]);
        let text = bs58::encode(bytes).into_string();
        assert!(EndpointId::parse_derived(text).is_err());
        let good = EndpointId::from_ed25519_public(&[0x11; 32]);
        assert!(EndpointId::parse_derived(good.as_str()).is_ok());
    }

    #[test]
    fn parse_derived_refuses_a_body_of_the_wrong_length() {
        // 52 characters that decode to something other than 38 bytes.
        let text = "1".repeat(52);
        assert_eq!(
            EndpointId::parse_derived(text.as_str()),
            Err(IdError::NotDerived)
        );
        // The right prefix but 39 bytes encodes to a different text length.
        let mut bytes = DERIVED_PREFIX.to_vec();
        bytes.extend_from_slice(&[0x11; 33]);
        let long = bs58::encode(bytes).into_string();
        assert!(EndpointId::parse_derived(long.as_str()).is_err());
        assert!(
            EndpointId::parse_derived(EndpointId::from_ed25519_public(&[0x11; 32]).as_str())
                .is_ok()
        );
    }

    #[test]
    fn parse_derived_keeps_the_shape_errors() {
        let good = EndpointId::from_ed25519_public(&KEYS[1]);
        let t = good.as_str();
        assert_eq!(EndpointId::parse_derived(""), Err(IdError::Empty));
        assert_eq!(
            EndpointId::parse_derived(&t[..51]),
            Err(IdError::WrongLength {
                got: 51,
                expected: 52
            })
        );
        assert_eq!(
            EndpointId::parse_derived(format!("{t}1")),
            Err(IdError::WrongLength {
                got: 53,
                expected: 52
            })
        );
        assert_eq!(
            EndpointId::parse_derived("1".repeat(65)),
            Err(IdError::TooLong { got: 65, max: 64 })
        );
        let mut zero = t.to_owned();
        zero.replace_range(10..11, "0");
        assert_eq!(
            EndpointId::parse_derived(zero),
            Err(IdError::IllegalByte {
                index: 10,
                byte: b'0'
            })
        );
        assert!(EndpointId::parse_derived(t).is_ok());
    }

    #[test]
    fn parse_derived_refuses_text_that_is_not_the_canonical_form() {
        let good = EndpointId::from_ed25519_public(&KEYS[1]);
        // A case change of the prefix is another number, never the same id.
        let lowered = good.as_str().replacen("12D3KooW", "12D3Koow", 1);
        assert_eq!(EndpointId::parse_derived(lowered), Err(IdError::NotDerived));
        // An extra leading '1' (one more zero byte) is another byte string.
        let padded = format!("1{}", &good.as_str()[..51]);
        assert!(EndpointId::parse_derived(padded).is_err());
        assert_eq!(
            EndpointId::parse_derived(good.as_str()).expect("control"),
            good
        );
    }

    #[test]
    fn parse_derived_refuses_a_label() {
        assert!(EndpointId::parse_derived("human").is_err());
        assert!(EndpointId::parse("human").is_ok());
    }

    #[test]
    fn a_derived_id_deserializes_through_parse() {
        let id = EndpointId::from_ed25519_public(&KEYS[1]);
        let json = serde_json::to_string(&id).expect("ser");
        assert_eq!(serde_json::from_str::<EndpointId>(&json).expect("de"), id);
        let bad = format!("\"{}\"", "Qm".to_owned() + &"1".repeat(44));
        assert!(serde_json::from_str::<EndpointId>(&bad).is_err());
    }

    #[test]
    fn endpoint_id_accepts_the_conventional_names() {
        for name in [
            "human",
            "claude",
            "automation.build",
            "a_b-c.d",
            "a",
            &"e".repeat(64),
        ] {
            assert!(EndpointId::parse(name).is_ok(), "{name} should parse");
        }
    }

    #[test]
    fn endpoint_id_rejects_everything_outside_the_grammar() {
        assert_eq!(EndpointId::parse(""), Err(IdError::Empty));
        assert_eq!(
            EndpointId::parse("e".repeat(65)),
            Err(IdError::TooLong { got: 65, max: 64 })
        );
        // A leading digit is reported as a LEADING error, not a generic
        // illegal byte: digits are legal everywhere else, so the position
        // is the whole explanation.
        assert_eq!(
            EndpointId::parse("1human"),
            Err(IdError::IllegalLeadingByte { byte: b'1' })
        );
        assert_eq!(
            EndpointId::parse("-human"),
            Err(IdError::IllegalLeadingByte { byte: b'-' })
        );
        assert_eq!(
            EndpointId::parse("Human"),
            Err(IdError::IllegalLeadingByte { byte: b'H' })
        );
        assert_eq!(
            EndpointId::parse("huMan"),
            Err(IdError::IllegalByte {
                index: 2,
                byte: b'M'
            })
        );
        assert!(EndpointId::parse("human client").is_err());
        assert!(EndpointId::parse("human/main").is_err());
        assert!(EndpointId::parse("hüman").is_err());
    }

    #[test]
    fn channel_id_is_case_sensitive_and_allows_its_wider_punctuation() {
        assert!(ChannelId::parse("general").is_ok());
        assert!(ChannelId::parse("General").is_ok());
        assert!(ChannelId::parse("team.eu:builds/nightly-1").is_ok());
        assert!(ChannelId::parse("c".repeat(128)).is_ok());
        assert!(ChannelId::parse("c".repeat(129)).is_err());
        assert_eq!(
            ChannelId::parse(".leading"),
            Err(IdError::IllegalLeadingByte { byte: b'.' })
        );
        // Distinct spellings stay distinct: no normalization happens here,
        // and the hashed wire topic would differ for these two.
        assert_ne!(
            ChannelId::parse("general").expect("valid"),
            ChannelId::parse("General").expect("valid")
        );
    }

    #[test]
    fn message_id_round_trips_through_its_canonical_form() {
        let id = MessageId::from_bytes([0xab; 16]);
        assert_eq!(id.to_hex(), "ab".repeat(16));
        assert_eq!(MessageId::parse_hex(&id.to_hex()), Ok(id));
        assert_eq!(
            MessageId::parse_hex(&"0".repeat(32)),
            Ok(MessageId::from_bytes([0; 16]))
        );
    }

    #[test]
    fn message_id_rejects_uppercase_rather_than_normalizing() {
        // Two spellings of one identifier is the bug; independent
        // implementations must compare equal without a normalization step.
        assert_eq!(
            MessageId::parse_hex(&"AB".repeat(16)),
            Err(IdError::IllegalByte {
                index: 0,
                byte: b'A'
            })
        );
        assert_eq!(
            MessageId::parse_hex(&"0".repeat(31)),
            Err(IdError::WrongLength {
                got: 31,
                expected: 32
            })
        );
        assert_eq!(
            MessageId::parse_hex(&"0".repeat(33)),
            Err(IdError::WrongLength {
                got: 33,
                expected: 32
            })
        );
        assert!(MessageId::parse_hex("0x00000000000000000000000000000000").is_err());
        assert!(MessageId::parse_hex("00000000-0000-0000-0000-000000000000").is_err());
    }

    /// A syntactically valid identity for tests. Not a real key.
    const TEST_PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

    #[test]
    fn a_destination_without_an_endpoint_means_default_not_fan_out() {
        let peer = TransportIdentity::parse(TEST_PEER).expect("valid");
        let d = DirectDestination::to_default(peer.clone());
        assert!(d.endpoint.is_none());
        let e = EndpointId::parse("human").expect("valid");
        assert_eq!(
            DirectDestination::to_endpoint(peer, e.clone()).endpoint,
            Some(e)
        );
    }

    #[test]
    fn transport_identity_requires_the_canonical_peer_id_form() {
        assert_eq!(TransportIdentity::parse(""), Err(IdError::Empty));
        assert!(TransportIdentity::parse(TEST_PEER).is_ok());
        // A Qm-form multihash is equally canonical.
        assert!(TransportIdentity::parse(format!("Qm{}", "a".repeat(44))).is_ok());

        // These all used to pass, and each one would have failed later
        // inside a backend parser instead of here at the boundary.
        for bad in [
            "garbage",
            "12D3KooW",
            "12D3KooWshort",
            "Qm",
            &format!("12D3KooW{}", "a".repeat(43)),
            &format!("12D3KooW{}", "a".repeat(45)),
            // base58btc excludes these four glyphs by design.
            &format!("12D3KooW{}0", "a".repeat(43)),
            &format!("12D3KooW{}O", "a".repeat(43)),
            &format!("12D3KooW{}I", "a".repeat(43)),
            &format!("12D3KooW{}l", "a".repeat(43)),
        ] {
            assert_eq!(
                TransportIdentity::parse(bad),
                Err(IdError::NotCanonicalPeerId),
                "{bad:?} should not parse"
            );
        }

        assert_eq!(
            TransportIdentity::parse("p".repeat(257)),
            Err(IdError::TooLong { got: 257, max: 256 })
        );
    }

    #[test]
    fn a_string_that_only_looks_like_a_peer_id_is_refused() {
        // The pattern is what the JSON Schema can express — text — and
        // it is not the whole rule. The 44 tail characters are a
        // base58btc number whose bytes have a fixed structure, so most
        // strings the pattern admits decode to something no libp2p
        // parser accepts, and used to reach one to find that out.

        // `Qm` + 44 valid glyphs, decoding to a multihash whose declared
        // length disagrees with the bytes that follow: 0x12 0x22 says 34
        // bytes of sha2-256 digest and 32 arrive.
        let shaped_only = format!("Qm{}", "z".repeat(44));
        assert_eq!(
            TransportIdentity::parse(shaped_only.clone()),
            Err(IdError::NotCanonicalPeerId),
            "a well-shaped string is not a well-formed multihash"
        );
        // It really does match the schema's pattern — otherwise this
        // would be testing the length check again.
        assert!(
            shaped_only.len() == 46
                && shaped_only.starts_with("Qm")
                && shaped_only[2..]
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() && !matches!(b, b'0' | b'O' | b'I' | b'l')),
            "the negative case must satisfy the pattern, or it proves nothing"
        );

        // The identity form, with the key-length byte one off: 0x21 says
        // 33 key bytes and 32 follow. Every other byte is correct.
        let mut bytes = [7_u8; 38];
        bytes[..6].copy_from_slice(&[0x00, 0x24, 0x08, 0x01, 0x12, 0x21]);
        let wrong_key_length = bs58::encode(bytes).into_string();
        assert!(wrong_key_length.starts_with("12D3KooW") && wrong_key_length.len() == 52);
        assert_eq!(
            TransportIdentity::parse(wrong_key_length),
            Err(IdError::NotCanonicalPeerId),
            "one byte wrong inside the envelope is still not a PeerId"
        );

        // And a sha2-256 multihash under the 12D3KooW prefix is not one
        // either: the prefix and the code have to agree.
        let mut swapped = [3_u8; 38];
        swapped[..6].copy_from_slice(&[0x12, 0x20, 0x08, 0x01, 0x12, 0x20]);
        assert_eq!(
            TransportIdentity::parse(bs58::encode(swapped).into_string()),
            Err(IdError::NotCanonicalPeerId)
        );

        // POSITIVE CONTROL: the same envelope, correct, parses — so this
        // is a decoder and not a blanket refusal.
        bytes[..6].copy_from_slice(&[0x00, 0x24, 0x08, 0x01, 0x12, 0x20]);
        let well_formed = bs58::encode(bytes).into_string();
        assert!(
            TransportIdentity::parse(well_formed.clone()).is_ok(),
            "{well_formed} is a canonical Ed25519 PeerId"
        );
    }
    #[test]
    fn a_direct_destination_refuses_unknown_fields_and_explicit_null() {
        // On a type whose whole purpose is resolving to exactly one
        // endpoint, an unknown property is how a caller smuggles in a
        // second destination or a source claim, and an explicit null is a
        // third state that decides where a message goes.
        let closed = format!(r#"{{"peer":"{TEST_PEER}","fanout":true}}"#);
        assert!(serde_json::from_str::<DirectDestination>(&closed).is_err());

        let nulled = format!(r#"{{"peer":"{TEST_PEER}","endpoint":null}}"#);
        assert!(serde_json::from_str::<DirectDestination>(&nulled).is_err());

        // Omitted still means the configured default, and a named
        // endpoint still parses.
        let omitted = format!(r#"{{"peer":"{TEST_PEER}"}}"#);
        let d: DirectDestination = serde_json::from_str(&omitted).expect("parses");
        assert_eq!(d.endpoint, None);
        let named = format!(r#"{{"peer":"{TEST_PEER}","endpoint":"human"}}"#);
        assert!(serde_json::from_str::<DirectDestination>(&named).is_ok());
    }
}
