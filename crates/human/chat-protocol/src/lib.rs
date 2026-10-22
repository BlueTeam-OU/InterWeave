// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `HumanChatV2`: the envelope, the contact card, the media type, the
//! bounded decode path, and the send-side encoder.
//!
//! Above transport and independent of it (ADR-0050). The same library
//! serves the desktop client, the Android client, and the Claude bridge,
//! which is what makes "decompression happens once, above transport"
//! true rather than aspirational.
//!
//! The markdown SUBSET is not enforced here. An out-of-subset construct
//! falls back to plain-text display rather than rejecting the message, so
//! subset conformance is a rendering contract and belongs with whatever
//! pins a CommonMark parser. What this crate does provide is the policy
//! primitives that rendering needs — the allowlisted link schemes and the
//! bounds — so every consumer applies one rule rather than its own
//! reading of the contract.

#![forbid(unsafe_code)]

pub mod card;
pub mod decode;
pub mod encode;
pub mod envelope;
#[cfg(feature = "markdown")]
pub mod render;

pub use card::{
    CARD_BYTES_PREFIX, CARD_PHOTO_MEDIA_TYPES, CONTACT_CARD_KIND, Card, CardBuildError, CardDraft,
    CardFault, CardPhoto, CardRoute, Classified, MAX_CARD_NAME_SCALARS, MAX_CARD_PHOTO_BYTES,
    build_card, canonical_card_bytes, classify, is_card,
};
pub use decode::{
    CONTENT_ENCODING_BROTLI, ContentEncoding, DecodeError, MAX_DECOMPRESSED_BYTES, MEDIA_TYPE_V2,
    MediaTypeInfo, decode_envelope_bytes, parse_media_type, sender_may_compress,
};
pub use encode::{EncodeError, Encoded, encode_outbound, encode_raw_envelope};
pub use envelope::{
    ALLOWED_LINK_SCHEMES, COMMONMARK_VERSION, EnvelopeError, GFM_VERSION, HumanChatV2,
    MAX_BLOCK_NESTING, MAX_INLINE_NESTING, MAX_SENT_AT_MS, MAX_TABLE_COLUMNS, MAX_TABLE_ROWS,
    MessageKind, is_allowed_link_scheme,
};
#[cfg(feature = "markdown")]
pub use render::{Alignment, Block, Inline, OverBound, Rendered, render};
