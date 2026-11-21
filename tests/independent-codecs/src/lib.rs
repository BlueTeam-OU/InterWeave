// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

//! Independent codecs for InterWeave's frozen wire shapes.
//!
//! **Test-only**, and independent by construction: every rule here is read
//! from the contract text — `architecture/transport/libp2p/DIRECT.md`,
//! `PUBSUB.md`, `architecture/contracts/ENDPOINTS.md`, `TRANSPORT.md`,
//! `LOCAL-IPC.md` and the schemas under `architecture/contracts/schemas/` —
//! and nothing from a production crate. `tools/checks/check_independent_codecs.sh`
//! refuses any workspace package in this crate's dependency graph,
//! dev-dependencies included.
//!
//! What follows from that: when a codec here and production disagree, the
//! contract text is the oracle and the disagreement is a contract finding
//! (testing.md §Compatibility fixtures), never a reason to edit one side
//! until the tests agree.
//!
//! The shapes, one module each:
//!
//! - [`direct_v2`] — the `DirectMessageV2` request frame, on
//!   `/interweave/direct/3.0.0`;
//! - [`direct_response_v2`] — its `AcceptedV2`/`RejectedV2` response;
//! - [`endpoints_v1`] — the endpoint directory's request and response;
//! - [`broadcast_v1`] — the GossipSub `BroadcastMessageV1` envelope;
//! - [`fingerprint`] — `DirectContentFingerprintV1`;
//! - [`gossipsub`] — the topic key and the mesh message id;
//! - [`ipc_v2`] — the IPC v2 length-prefixed frame and its envelope classes.
//!
//! The response codec is written from `DIRECT.md` §Response byte layout,
//! which this codec's writing caused to be stated (architect-cto,
//! 2026-10-08): until then the layout lived only in production's codec.

pub mod broadcast_v1;
pub mod direct_response_v2;
pub mod direct_v2;
pub mod endpoints_v1;
pub mod fingerprint;
pub mod gossipsub;
pub mod ipc_v2;
pub mod json;
mod wire;

/// Why a frame was refused. The message names the rule, for a test's
/// failure output. Where a reader must branch (more bytes or a refusal),
/// the codec returns a type for it: [`ipc_v2::Split`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(pub String);

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DecodeError {}

/// `DIRECT.md` and `PUBSUB.md`: the payload ceiling both wires enforce at
/// decode, the profile's `max_payload_bytes` never above it.
pub const MAX_PAYLOAD_BYTES: usize = 49_152;

/// `TRANSPORT.md` §Direct send: a present media type is 1..128 bytes.
pub const MAX_MEDIA_TYPE_BYTES: usize = 128;

/// `ENDPOINTS.md`, the retired label grammar: `^[a-z][a-z0-9._-]{0,63}$`.
/// The direct codecs no longer use it (they take [`is_derived_endpoint_id`]);
/// the shapes that still do follow their own contract's major.
#[must_use]
pub fn is_endpoint_id(s: &str) -> bool {
    let b = s.as_bytes();
    match b.split_first() {
        Some((first, rest)) => {
            first.is_ascii_lowercase()
                && rest.len() <= 63
                && rest.iter().all(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'_' | b'-')
                })
        }
        None => false,
    }
}

/// The length of a derived `EndpointId`'s text, and so of every endpoint
/// field on `/interweave/direct/3.0.0` (`DIRECT.md` §Request).
pub const DERIVED_ENDPOINT_ID_BYTES: usize = 52;

const BASE58BTC: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// `00 24 08 01 12 20`: the identity multihash of 36 bytes, then the
/// protobuf header of an Ed25519 public key (`ENDPOINTS.md` §`EndpointId`).
const DERIVED_PREFIX: [u8; 6] = [0x00, 0x24, 0x08, 0x01, 0x12, 0x20];

fn base58btc(bytes: &[u8]) -> String {
    let zeros = bytes.iter().take_while(|b| **b == 0).count();
    // Little-endian base-58 digits of the big-endian number.
    let mut digits: Vec<u8> = Vec::new();
    for &byte in &bytes[zeros..] {
        let mut carry = u32::from(byte);
        for d in &mut digits {
            carry += u32::from(*d) << 8;
            *d = u8::try_from(carry % 58).unwrap_or(0);
            carry /= 58;
        }
        while carry > 0 {
            digits.push(u8::try_from(carry % 58).unwrap_or(0));
            carry /= 58;
        }
    }
    let mut out = "1".repeat(zeros);
    out.extend(
        digits
            .iter()
            .rev()
            .map(|d| char::from(BASE58BTC[usize::from(*d)])),
    );
    out
}

fn base58btc_decode(text: &str) -> Option<Vec<u8>> {
    let zeros = text.bytes().take_while(|b| *b == b'1').count();
    let mut bytes: Vec<u8> = Vec::new(); // little-endian
    for c in text.bytes().skip(zeros) {
        let mut carry = u32::try_from(BASE58BTC.iter().position(|a| *a == c)?).ok()?;
        for b in &mut bytes {
            carry += u32::from(*b) * 58;
            *b = u8::try_from(carry & 0xff).unwrap_or(0);
            carry >>= 8;
        }
        while carry > 0 {
            bytes.push(u8::try_from(carry & 0xff).unwrap_or(0));
            carry >>= 8;
        }
    }
    let mut out = vec![0; zeros];
    out.extend(bytes.iter().rev());
    Some(out)
}

/// The derived `EndpointId` of an Ed25519 public key (`ENDPOINTS.md`
/// §`EndpointId`): base58btc of `00 24 08 01 12 20 || key`.
#[must_use]
pub fn derive_endpoint_id(public_key: &[u8; 32]) -> String {
    let mut bytes = DERIVED_PREFIX.to_vec();
    bytes.extend_from_slice(public_key);
    base58btc(&bytes)
}

/// `ENDPOINTS.md` §`EndpointId`: 52 characters that base58btc-decode to the
/// 38 bytes `00 24 08 01 12 20 || key` and re-encode to themselves, the
/// one canonical form.
#[must_use]
pub fn is_derived_endpoint_id(s: &str) -> bool {
    if s.len() != DERIVED_ENDPOINT_ID_BYTES {
        return false;
    }
    base58btc_decode(s)
        .is_some_and(|raw| raw.len() == 38 && raw[..6] == DERIVED_PREFIX && base58btc(&raw) == s)
}

/// A present media type: 1..128 bytes, each in 0x20..=0x7E.
///
/// PRINTABLE: the two schemas that carry the field pin it
/// (`ipc/payload`, `endpoints/message-received`: `^[\x20-\x7E]+$`), and
/// since architect-cto's ruling of 2026-10-08 (01a11c8d) the wire and
/// fingerprint prose says so too. Before that the prose said "ASCII".
#[must_use]
pub fn is_media_type(s: &str) -> bool {
    (1..=MAX_MEDIA_TYPE_BYTES).contains(&s.len()) && s.bytes().all(|b| (0x20..=0x7e).contains(&b))
}
