// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The contact card (HUMAN-CHAT.md §Contact card, ADR-0059, ADR-0063).
//!
//! A `contact_card` envelope is the sender's own card: a name, an
//! optional surname and photo, the sending identity's Ed25519 key and its
//! signature over the canonical card bytes. [`classify`] gives every
//! `HumanChatV2` text the three-way answer its consumers need — not a
//! card, a card, or a malformed card and why — and a card it calls valid
//! has verified under its key over bytes that name the connection it
//! arrived on, never anything the envelope says about where it came from.
//!
//! Nothing here decodes the photo as an image: a client does that after
//! the person accepts the card (ADR-0059 decision 5). What a [`Card`]
//! carries is the decoded BYTES and their length, as the parser measured
//! them.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, VerifyingKey};
use interweave_transport_api::{EndpointId, TransportIdentity};
use sha2::{Digest as _, Sha256};
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_normalization::UnicodeNormalization as _;

use crate::envelope::{MAX_SENT_AT_MS, StrictValue, duplicate_member, is_canonical_id};

/// The kind a contact card carries.
pub const CONTACT_CARD_KIND: &str = "contact_card";
/// The ASCII prefix of the canonical card bytes, with no length prefix.
pub const CARD_BYTES_PREFIX: &[u8] = b"interweave-card-v1";
/// A name or surname is 1 to this many Unicode scalar values after NFC.
pub const MAX_CARD_NAME_SCALARS: usize = 64;
/// A photo is at most this many bytes decoded.
pub const MAX_CARD_PHOTO_BYTES: usize = 24_576;
/// The two media types a photo may have, and no other.
pub const CARD_PHOTO_MEDIA_TYPES: [&str; 2] = ["image/png", "image/jpeg"];

/// The connection a card arrived on: the authenticated source peer and
/// the source endpoint of the direct message that carried it.
///
/// The canonical bytes take both from here, never from the envelope, so
/// a card signed for one route does not verify on another (ADR-0063
/// decision 3). A card received over broadcast has no such route and is
/// not given to [`classify`]: it is ignored (HUMAN-CHAT.md).
#[derive(Debug, Clone, Copy)]
pub struct CardRoute<'a> {
    /// The authenticated `source_peer`.
    pub source_peer: &'a TransportIdentity,
    /// The peer-asserted `source_endpoint` the message was addressed from.
    pub source_endpoint: &'a EndpointId,
}

/// A photo as the card carried it: its media type and its decoded bytes,
/// at most [`MAX_CARD_PHOTO_BYTES`]. Never decoded as an image here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardPhoto {
    /// `image/png` or `image/jpeg`.
    pub media_type: &'static str,
    /// The decoded bytes.
    pub bytes: Vec<u8>,
}

/// A contact card that verified under its identity key over the canonical
/// bytes of the route it arrived on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// 32 lowercase hex characters.
    pub app_message_id: String,
    /// The name after NFC, within its bounds.
    pub name: String,
    /// The surname after NFC, within its bounds, if the card has one.
    pub surname: Option<String>,
    /// The photo, if the card has one.
    pub photo: Option<CardPhoto>,
    /// The sending identity's Ed25519 public key.
    pub identity_key: [u8; 32],
    /// Diagnostic timestamp, never authority.
    pub sent_at_ms: Option<u64>,
    /// An unauthenticated display hint.
    pub from_endpoint: Option<EndpointId>,
}

impl Card {
    /// The photo's decoded length as the parser measured it, `0` when the
    /// card has none (the bridge's `card_photo_bytes`).
    #[must_use]
    pub fn photo_len(&self) -> usize {
        self.photo.as_ref().map_or(0, |p| p.bytes.len())
    }
}

/// Why a contact card is malformed: one fixed reason per fault.
///
/// The variants carry no peer text, so a consumer may log or count one
/// as it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardFault {
    /// An object named the same member twice, anywhere in the envelope —
    /// a second top-level `kind` included.
    DuplicateMember,
    /// Well-formed JSON this parser cannot hold to one value: a number
    /// out of range, a lone surrogate escape, or nesting past the
    /// recursion limit. Other readers resolve each differently, so the
    /// card has no single meaning, as for a duplicate member.
    Unreadable,
    /// `v` was not 2.
    Version,
    /// `app_message_id` was not 32 lowercase hex characters.
    AppMessageId,
    /// `sent_at_ms` was outside the closed interval.
    SentAtMs,
    /// `from_endpoint` was not a valid `EndpointId`.
    FromEndpoint,
    /// The envelope carried `text`: a card is not a message with a caption.
    TextPresent,
    /// The envelope carried `reply_to`: a card answers nothing.
    ReplyToPresent,
    /// `card` was missing or not an object.
    MissingCard,
    /// `card.name` was missing, not a string, or outside its bounds.
    Name,
    /// `card.surname` was not a string, or outside its bounds.
    Surname,
    /// `photo` was not an object, or its media type not one of the two.
    PhotoMediaType,
    /// `photo.data` was not RFC 4648 §4 standard base64, padded.
    PhotoEncoding,
    /// The photo decoded to more than [`MAX_CARD_PHOTO_BYTES`].
    PhotoTooLarge,
    /// `identity_key` was missing or not RFC 4648 §4 standard base64.
    IdentityKeyEncoding,
    /// `identity_key` did not decode to 32 bytes.
    IdentityKeyLength,
    /// `signature` was missing or not RFC 4648 §4 standard base64.
    SignatureEncoding,
    /// `signature` did not decode to 64 bytes.
    SignatureLength,
    /// The signature failed strict verification under the identity key
    /// over the canonical bytes of the route the card arrived on.
    SignatureFails,
}

impl CardFault {
    /// A fixed, peer-text-free name for the fault.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DuplicateMember => "duplicate-member",
            Self::Unreadable => "unreadable",
            Self::Version => "version",
            Self::AppMessageId => "app-message-id",
            Self::SentAtMs => "sent-at-ms",
            Self::FromEndpoint => "from-endpoint",
            Self::TextPresent => "text-present",
            Self::ReplyToPresent => "reply-to-present",
            Self::MissingCard => "card",
            Self::Name => "name",
            Self::Surname => "surname",
            Self::PhotoMediaType => "photo-media-type",
            Self::PhotoEncoding => "photo-encoding",
            Self::PhotoTooLarge => "photo-size",
            Self::IdentityKeyEncoding => "identity-key-encoding",
            Self::IdentityKeyLength => "identity-key-length",
            Self::SignatureEncoding => "signature-encoding",
            Self::SignatureLength => "signature-length",
            Self::SignatureFails => "signature-fails",
        }
    }
}

impl core::fmt::Display for CardFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The three-way answer for one `HumanChatV2` text (plan §29 step 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classified {
    /// No top-level `kind` is `contact_card`: the text is not a card, and
    /// whether it is a valid message is [`crate::HumanChatV2::parse`]'s
    /// question.
    NotCard,
    /// A valid card.
    Card(Card),
    /// A card, malformed for this reason.
    MalformedCard(CardFault),
}

/// Classify one `HumanChatV2` text received on `route`.
///
/// An envelope ANY of whose top-level `kind` copies is `contact_card` is
/// a card, so a duplicate `kind` cannot hide a card from a consumer that
/// treats "not a card" differently from "a malformed card": it is
/// [`Classified::MalformedCard`] with [`CardFault::DuplicateMember`],
/// never [`Classified::NotCard`].
#[must_use]
pub fn classify(text: &str, route: CardRoute<'_>) -> Classified {
    if !any_top_level_kind_is_card(text) {
        return Classified::NotCard;
    }
    match parse_card(text, route) {
        Ok(card) => Classified::Card(card),
        Err(fault) => Classified::MalformedCard(fault),
    }
}

/// HUMAN-CHAT.md §Contact card's canonical card bytes: the prefix, then
/// six fields each as `u16be(len) || bytes` — the name and the surname
/// after NFC (an absent surname as length 0), the photo's media type and
/// the SHA-256 of its decoded bytes (each length 0 without a photo), the
/// source peer's multihash bytes and the source endpoint.
///
/// Every field is bounded far below `u16::MAX` by the card's own bounds
/// or by its type: a name is at most 64 scalars of at most 4 bytes, a
/// media type one of two, a hash 32 bytes, a `PeerId`'s multihash and an
/// `EndpointId` each a few dozen bytes. `None` is returned rather than a
/// truncated length should that ever stop being true.
#[must_use]
pub fn canonical_card_bytes(
    name: &str,
    surname: Option<&str>,
    photo: Option<(&str, &[u8])>,
    source_peer_multihash: &[u8],
    source_endpoint: &str,
) -> Option<Vec<u8>> {
    let photo_hash = photo.map(|(_, bytes)| Sha256::digest(bytes));
    let fields: [&[u8]; 6] = [
        name.as_bytes(),
        surname.unwrap_or("").as_bytes(),
        photo.map_or(&[][..], |(media_type, _)| media_type.as_bytes()),
        photo_hash.as_ref().map_or(&[][..], |h| h.as_slice()),
        source_peer_multihash,
        source_endpoint.as_bytes(),
    ];
    let mut out = Vec::with_capacity(
        CARD_BYTES_PREFIX.len() + fields.iter().map(|f| 2 + f.len()).sum::<usize>(),
    );
    out.extend_from_slice(CARD_BYTES_PREFIX);
    for field in fields {
        let len = u16::try_from(field.len()).ok()?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(field);
    }
    Some(out)
}

/// Whether `text` is a contact card at all, by the same rule [`classify`]
/// uses and before any of its faults is judged: a consumer that only
/// discards cards (a card received over broadcast, which is ignored)
/// asks this, with no route to verify a signature over.
#[must_use]
pub fn is_card(text: &str) -> bool {
    any_top_level_kind_is_card(text)
}

/// The fields of this node's OWN card as its pending row holds them, read
/// with [`read_own_card_unverified`]: never a [`Card`], so it cannot be
/// passed where a verified card is wanted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnCardFields {
    /// 32 lowercase hex characters.
    pub app_message_id: String,
    /// The name after NFC, within its bounds.
    pub name: String,
    /// The surname after NFC, within its bounds, if it has one.
    pub surname: Option<String>,
    /// The photo, if it has one.
    pub photo: Option<CardPhoto>,
    /// The identity key it carries.
    pub identity_key: [u8; 32],
}

/// Read back this node's OWN card from the text its pending outbound row
/// holds, for display, WITHOUT verifying its signature: `None` for a text
/// that is not a card, or one this grammar refuses.
///
/// For the sender's own stored rows only. A received text never enters
/// it -- a rule of its callers, held by review alone: its one caller is
/// app-core's listing of this node's pending outbound rows. The
/// receiver's path is [`classify`] alone, which verifies the card
/// over the route it arrived on (ADR-0063 decision 3). Reading one's own
/// write back verifies nothing the encrypted store does not, and cannot be
/// done before this node's `PeerId` is known (HUMAN-CHAT.md section Contact
/// card, A 2026-10-27). The grammar is [`classify`]'s up to the signature check,
/// one function for both (`tests/frozen_cards.rs`:
/// `the_own_reader_refuses_what_classify_refuses_before_the_signature`).
#[must_use]
pub fn read_own_card_unverified(text: &str) -> Option<OwnCardFields> {
    if !any_top_level_kind_is_card(text) {
        return None;
    }
    parse_fields(text).ok().map(|(fields, _)| fields.own)
}

/// What a person writes on their own card, before it is signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardDraft<'a> {
    /// The name, as typed; it is sent after NFC.
    pub name: &'a str,
    /// The surname, if any.
    pub surname: Option<&'a str>,
    /// The photo, already re-encoded with no metadata block: its media
    /// type and its bytes.
    pub photo: Option<(&'a str, &'a [u8])>,
}

/// Why a card cannot be built: the draft is outside the bounds a
/// receiver holds it to, so it would arrive malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardBuildError {
    /// The name is outside its bounds after NFC.
    Name,
    /// The surname is outside its bounds after NFC.
    Surname,
    /// The photo's media type is not one of the two.
    PhotoMediaType,
    /// The photo is over [`MAX_CARD_PHOTO_BYTES`].
    PhotoTooLarge,
    /// A canonical field is past `u16`: never for a draft within bounds.
    Unencodable,
}

/// Whether a receiver would hold `draft` within its bounds: the name and
/// surname after NFC, the photo's media type and size. What a person's own
/// card is held to when they edit it, so a card they share is never one a
/// receiver refuses.
///
/// # Errors
/// [`CardBuildError`] for the first bound it breaks.
pub fn check_draft(draft: CardDraft<'_>) -> Result<(), CardBuildError> {
    bounded_name(&serde_json::Value::from(draft.name)).ok_or(CardBuildError::Name)?;
    if let Some(s) = draft.surname {
        bounded_name(&serde_json::Value::from(s)).ok_or(CardBuildError::Surname)?;
    }
    if let Some((media_type, bytes)) = draft.photo {
        if !CARD_PHOTO_MEDIA_TYPES.contains(&media_type) {
            return Err(CardBuildError::PhotoMediaType);
        }
        if bytes.len() > MAX_CARD_PHOTO_BYTES {
            return Err(CardBuildError::PhotoTooLarge);
        }
    }
    Ok(())
}

/// The `contact_card` envelope `draft` makes, signed for the route it is
/// sent on: `sender` is this node's `PeerId` and `endpoint` the identity
/// the conversation speaks as, which the receiver takes from the
/// connection and verifies the signature over. `sign` is the identity
/// key's, and `identity_key` its public half.
///
/// What it returns is the raw JSON text, which [`classify`] answers with
/// [`Classified::Card`] on that route, and only there.
///
/// # Errors
/// [`CardBuildError`] for a draft a receiver would refuse.
pub fn build_card(
    app_message_id: &str,
    draft: CardDraft<'_>,
    identity_key: [u8; 32],
    sender: CardRoute<'_>,
    sent_at_ms: Option<u64>,
    sign: impl FnOnce(&[u8]) -> [u8; 64],
) -> Result<String, CardBuildError> {
    check_draft(draft)?;
    let name = bounded_name(&serde_json::Value::from(draft.name)).ok_or(CardBuildError::Name)?;
    let surname = match draft.surname {
        None => None,
        Some(s) => Some(bounded_name(&serde_json::Value::from(s)).ok_or(CardBuildError::Surname)?),
    };
    let photo = match draft.photo {
        None => None,
        Some((media_type, bytes)) => {
            let media_type = CARD_PHOTO_MEDIA_TYPES
                .into_iter()
                .find(|t| *t == media_type)
                .ok_or(CardBuildError::PhotoMediaType)?;
            if bytes.len() > MAX_CARD_PHOTO_BYTES {
                return Err(CardBuildError::PhotoTooLarge);
            }
            Some((media_type, bytes))
        }
    };
    let peer = bs58::decode(sender.source_peer.as_str())
        .into_vec()
        .map_err(|_| CardBuildError::Unencodable)?;
    let message = canonical_card_bytes(
        &name,
        surname.as_deref(),
        photo,
        &peer,
        sender.source_endpoint.as_str(),
    )
    .ok_or(CardBuildError::Unencodable)?;
    let signature = sign(&message);

    let mut card = serde_json::Map::new();
    card.insert("name".to_owned(), name.into());
    if let Some(surname) = surname {
        card.insert("surname".to_owned(), surname.into());
    }
    if let Some((media_type, bytes)) = photo {
        card.insert(
            "photo".to_owned(),
            serde_json::json!({ "media_type": media_type, "data": STANDARD.encode(bytes) }),
        );
    }
    card.insert(
        "identity_key".to_owned(),
        STANDARD.encode(identity_key).into(),
    );
    card.insert("signature".to_owned(), STANDARD.encode(signature).into());
    let mut envelope = serde_json::Map::new();
    envelope.insert("v".to_owned(), 2.into());
    envelope.insert("kind".to_owned(), CONTACT_CARD_KIND.into());
    envelope.insert("app_message_id".to_owned(), app_message_id.into());
    envelope.insert("card".to_owned(), card.into());
    if let Some(at) = sent_at_ms {
        envelope.insert("sent_at_ms".to_owned(), at.into());
    }
    Ok(serde_json::Value::Object(envelope).to_string())
}

/// Whether any top-level `kind` member is the string `contact_card`.
///
/// Read apart from the strict parse, which refuses a duplicate before it
/// could say which copy named a card: here every copy is seen. And read
/// by the JSON GRAMMAR, not by what `serde_json` can represent: a number
/// out of `f64`'s range, a lone surrogate escape or nesting past the
/// recursion limit are all well-formed JSON that other readers accept, so
/// none of them, in a key or in any value, may stop this pass from seeing
/// a `contact_card` beside it. Keys are read as raw bytes, which takes
/// `serde_json`'s unvalidated string path; a `kind` value is taken as a
/// [`serde_json::value::RawValue`], skipped as loosely as an ignored
/// member, and only then decoded; every other value is ignored. What
/// still fails is text that is not a JSON object at all, which no reader
/// sees a card in.
fn any_top_level_kind_is_card(text: &str) -> bool {
    /// Whether a member's name is `kind`, after its escapes.
    struct IsKind(bool);
    impl<'de> serde::Deserialize<'de> for IsKind {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct V;
            impl serde::de::Visitor<'_> for V {
                type Value = IsKind;
                fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str("a member name")
                }
                fn visit_bytes<E>(self, v: &[u8]) -> Result<IsKind, E> {
                    Ok(IsKind(v == b"kind"))
                }
                fn visit_str<E>(self, v: &str) -> Result<IsKind, E> {
                    Ok(IsKind(v == "kind"))
                }
            }
            d.deserialize_bytes(V)
        }
    }

    struct Kinds(bool);
    impl<'de> serde::Deserialize<'de> for Kinds {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct V;
            impl<'de> serde::de::Visitor<'de> for V {
                type Value = Kinds;
                fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str("a JSON object")
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> Result<Kinds, A::Error> {
                    let mut card = false;
                    while let Some(IsKind(is_kind)) = map.next_key::<IsKind>()? {
                        if is_kind {
                            let raw: &'de serde_json::value::RawValue = map.next_value()?;
                            // A copy that is no string, or one holding a
                            // lone surrogate, is not `contact_card`.
                            card |= serde_json::from_str::<String>(raw.get())
                                .is_ok_and(|kind| kind == CONTACT_CARD_KIND);
                        } else {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                    Ok(Kinds(card))
                }
            }
            d.deserialize_map(V)
        }
    }
    // The raw key path decodes escapes but skips the grammar's one rule
    // on string content, no unescaped control character, so it would
    // read a text that is not JSON. `IgnoredAny` over the whole text
    // keeps that rule in every string, key or value, and nothing of
    // representability (range, surrogates, depth): a text is a card only
    // if it is JSON at all.
    serde_json::from_str::<Kinds>(text).is_ok_and(|Kinds(card)| card)
        && serde_json::from_str::<serde::de::IgnoredAny>(text).is_ok()
}

/// A card's fields as its grammar reads them, before any signature check:
/// what [`parse_card`] verifies and [`read_own_card_unverified`] returns.
struct ParsedFields {
    own: OwnCardFields,
    sent_at_ms: Option<u64>,
    from_endpoint: Option<EndpointId>,
}

fn parse_card(text: &str, route: CardRoute<'_>) -> Result<Card, CardFault> {
    let (fields, signature) = parse_fields(text)?;
    let ParsedFields {
        own:
            OwnCardFields {
                app_message_id,
                name,
                surname,
                photo,
                identity_key,
            },
        sent_at_ms,
        from_endpoint,
    } = fields;
    let peer = bs58::decode(route.source_peer.as_str())
        .into_vec()
        .map_err(|_| CardFault::SignatureFails)?;
    let message = canonical_card_bytes(
        &name,
        surname.as_deref(),
        photo.as_ref().map(|p| (p.media_type, p.bytes.as_slice())),
        &peer,
        route.source_endpoint.as_str(),
    )
    .ok_or(CardFault::SignatureFails)?;
    if !verifies_strictly(&identity_key, &signature, &message) {
        return Err(CardFault::SignatureFails);
    }

    Ok(Card {
        app_message_id,
        name,
        surname,
        photo,
        identity_key,
        sent_at_ms,
        from_endpoint,
    })
}

/// The card grammar, every fault but the signature's: the fields and the
/// signature's bytes, decoded and of their length.
fn parse_fields(text: &str) -> Result<(ParsedFields, [u8; 64]), CardFault> {
    let StrictValue(raw) = serde_json::from_str::<StrictValue>(text).map_err(|e| {
        // The lenient pass read this text as a JSON object by the grammar,
        // so a strict failure is either a repeated member or JSON this
        // parser cannot hold to one value: a number out of range, a lone
        // surrogate, nesting past the recursion limit.
        if duplicate_member(&e).is_some() {
            CardFault::DuplicateMember
        } else {
            CardFault::Unreadable
        }
    })?;

    if raw.get("v").and_then(serde_json::Value::as_u64) != Some(2) {
        return Err(CardFault::Version);
    }
    let app_message_id = raw
        .get("app_message_id")
        .and_then(serde_json::Value::as_str)
        .filter(|s| is_canonical_id(s))
        .ok_or(CardFault::AppMessageId)?
        .to_owned();
    if raw.get("text").is_some() {
        return Err(CardFault::TextPresent);
    }
    if raw.get("reply_to").is_some() {
        return Err(CardFault::ReplyToPresent);
    }
    // An explicit null is not absence, as for a text message.
    let sent_at_ms = match raw.get("sent_at_ms") {
        None => None,
        Some(v) => Some(
            v.as_u64()
                .filter(|n| *n <= MAX_SENT_AT_MS)
                .ok_or(CardFault::SentAtMs)?,
        ),
    };
    let from_endpoint = match raw.get("from_endpoint") {
        None => None,
        Some(v) => Some(
            v.as_str()
                .and_then(|s| EndpointId::parse(s).ok())
                .ok_or(CardFault::FromEndpoint)?,
        ),
    };

    let card = raw
        .get("card")
        .and_then(serde_json::Value::as_object)
        .ok_or(CardFault::MissingCard)?;
    let name = card
        .get("name")
        .and_then(bounded_name)
        .ok_or(CardFault::Name)?;
    let surname = match card.get("surname") {
        None => None,
        Some(v) => Some(bounded_name(v).ok_or(CardFault::Surname)?),
    };
    let photo = match card.get("photo") {
        None => None,
        Some(v) => Some(card_photo(v)?),
    };
    let identity_key: [u8; 32] = strict_base64(card.get("identity_key"))
        .ok_or(CardFault::IdentityKeyEncoding)?
        .try_into()
        .map_err(|_| CardFault::IdentityKeyLength)?;
    let signature: [u8; 64] = strict_base64(card.get("signature"))
        .ok_or(CardFault::SignatureEncoding)?
        .try_into()
        .map_err(|_| CardFault::SignatureLength)?;
    Ok((
        ParsedFields {
            own: OwnCardFields {
                app_message_id,
                name,
                surname,
                photo,
                identity_key,
            },
            sent_at_ms,
            from_endpoint,
        },
        signature,
    ))
}

/// Strict as `ed25519-dalek`'s `verify_strict` defines it (HUMAN-CHAT.md):
/// a small-order public key and a small-order R are refused by its
/// checks, and a non-canonical S by its scalar decoding while the
/// crate's `legacy_compatibility` feature is off in the graph.
fn verifies_strictly(key: &[u8; 32], signature: &[u8; 64], message: &[u8]) -> bool {
    let Ok(key) = VerifyingKey::from_bytes(key) else {
        return false;
    };
    key.verify_strict(message, &Signature::from_bytes(signature))
        .is_ok()
}

/// A name or surname after NFC: 1 to 64 scalar values, none a control
/// (`Cc`), format (`Cf`), line separator (`Zl`) or paragraph separator
/// (`Zp`) character.
fn bounded_name(value: &serde_json::Value) -> Option<String> {
    let nfc: String = value.as_str()?.nfc().collect();
    let scalars = nfc.chars().count();
    let clean = nfc.chars().all(|c| {
        !matches!(
            get_general_category(c),
            GeneralCategory::Control
                | GeneralCategory::Format
                | GeneralCategory::LineSeparator
                | GeneralCategory::ParagraphSeparator
        )
    });
    ((1..=MAX_CARD_NAME_SCALARS).contains(&scalars) && clean).then_some(nfc)
}

fn card_photo(value: &serde_json::Value) -> Result<CardPhoto, CardFault> {
    let photo = value.as_object().ok_or(CardFault::PhotoMediaType)?;
    let media_type = photo
        .get("media_type")
        .and_then(serde_json::Value::as_str)
        .and_then(|m| CARD_PHOTO_MEDIA_TYPES.into_iter().find(|t| *t == m))
        .ok_or(CardFault::PhotoMediaType)?;
    let bytes = strict_base64(photo.get("data")).ok_or(CardFault::PhotoEncoding)?;
    if bytes.len() > MAX_CARD_PHOTO_BYTES {
        return Err(CardFault::PhotoTooLarge);
    }
    Ok(CardPhoto { media_type, bytes })
}

/// RFC 4648 §4 standard base64, padded, no whitespace, no other alphabet,
/// no stray bits in the last character; `None` for anything else, a
/// missing or non-string member included.
fn strict_base64(value: Option<&serde_json::Value>) -> Option<Vec<u8>> {
    STANDARD.decode(value?.as_str()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_canonical_field_is_length_prefixed_in_order() {
        let bytes = canonical_card_bytes("Ada", None, None, &[0, 1], "human").expect("fits");
        let mut want = b"interweave-card-v1".to_vec();
        want.extend_from_slice(&[0, 3, b'A', b'd', b'a', 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0, 5]);
        want.extend_from_slice(b"human");
        assert_eq!(bytes, want);
    }

    #[test]
    fn a_field_past_u16_is_refused_rather_than_truncated() {
        let long = "a".repeat(usize::from(u16::MAX) + 1);
        assert_eq!(canonical_card_bytes(&long, None, None, &[], "human"), None);
        let at = "a".repeat(usize::from(u16::MAX));
        assert!(canonical_card_bytes(&at, None, None, &[], "human").is_some());
    }

    #[test]
    fn a_name_refuses_each_excluded_category_and_counts_scalars_after_nfc() {
        for bad in ["a\u{0007}", "a\u{200B}", "a\u{2028}", "a\u{2029}", ""] {
            assert_eq!(bounded_name(&serde_json::Value::from(bad)), None, "{bad:?}");
        }
        // 64 decomposed é (128 scalars) compose to 64.
        let nfd = "e\u{0301}".repeat(64);
        assert_eq!(
            bounded_name(&serde_json::Value::from(nfd)).map(|n| n.chars().count()),
            Some(64)
        );
        assert_eq!(bounded_name(&serde_json::Value::from("a".repeat(65))), None);
        // An astral scalar is one, whatever its UTF-16 length.
        assert!(bounded_name(&serde_json::Value::from("\u{1D49C}".repeat(64))).is_some());
    }

    #[test]
    fn base64_is_standard_padded_and_canonical() {
        let ok = |s: &str| strict_base64(Some(&serde_json::Value::from(s)));
        assert_eq!(ok("QUJD"), Some(b"ABC".to_vec()));
        assert_eq!(ok("QQ=="), Some(b"A".to_vec()));
        for bad in ["QQ", "QR==", "Q Q==", "QQ==\n", "-_8=", "QUJD="] {
            assert_eq!(ok(bad), None, "{bad:?}");
        }
        assert_eq!(strict_base64(Some(&serde_json::Value::from(1))), None);
        assert_eq!(strict_base64(None), None);
    }
}
