// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The bridge's one parsing exception (CHANNEL-EVENT.md §Content, A
//! 2026-10-21, ADR-0059 decision 7): a `HumanChatV2` payload chat-protocol's
//! parser classifies as a contact card reaches the model as `meta` alone,
//! and a malformed or broadcast card is dropped under one reason of a
//! closed set.
//!
//! The bridge decides nothing about a card itself: whether a text is a
//! card, and whether it is well-formed and verified, is `classify`'s
//! answer and no other. What the bridge adds is the presentation -- the
//! card's own fields as `meta`, never the photo's bytes -- and the
//! reason a dropped card is counted under. It binds nothing and stores
//! nothing: the card is data to the model.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use interweave_human_chat_protocol::{
    Card, CardFault, CardRoute, Classified, MAX_CARD_NAME_SCALARS, classify,
};
use interweave_transport_api::{EndpointId, TransportIdentity};

use crate::meta::{ChannelMeta, MAX_META_VALUE_BYTES, MetaError, MetaKey};

// A legal name or surname is at most this many scalar values, each at most
// four bytes, so it always fits a meta value: a card the parser accepts
// is never refused by the bridge's own bound.
const _: () = assert!(MAX_CARD_NAME_SCALARS * 4 <= MAX_META_VALUE_BYTES);

/// Why a contact card was dropped: CHANNEL-EVENT.md §Content's closed set,
/// one reason per fault class it lists. A new class is a change to that
/// sentence, and to this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardDrop {
    /// The photo is over its decoded bound.
    PhotoSize,
    /// The photo's base64 is not RFC 4648 §4 standard, padded.
    PhotoEncoding,
    /// The photo's media type is not one of the two listed.
    PhotoMediaType,
    /// The name or the surname is outside its bounds.
    Name,
    /// `card` or `card.name` is missing.
    MissingField,
    /// A `text` or `reply_to` member is present.
    StrayMember,
    /// A member is duplicated.
    DuplicateMember,
    /// A text the parser cannot hold to one JSON value.
    Unreadable,
    /// The identity key's encoding or length.
    IdentityKey,
    /// The signature's encoding or length, or its verification.
    Signature,
    /// An envelope-wide fault: `v`, `app_message_id`, `sent_at_ms` or
    /// `from_endpoint`.
    Envelope,
    /// A card received over broadcast, which HUMAN-CHAT.md ignores.
    Broadcast,
}

impl CardDrop {
    /// Every reason, in the order `status` lists them.
    pub const ALL: [Self; 12] = [
        Self::PhotoSize,
        Self::PhotoEncoding,
        Self::PhotoMediaType,
        Self::Name,
        Self::MissingField,
        Self::StrayMember,
        Self::DuplicateMember,
        Self::Unreadable,
        Self::IdentityKey,
        Self::Signature,
        Self::Envelope,
        Self::Broadcast,
    ];

    /// The reason as `status` and stderr name it: fixed text, never
    /// anything the card carried.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PhotoSize => "photo_size",
            Self::PhotoEncoding => "photo_encoding",
            Self::PhotoMediaType => "photo_media_type",
            Self::Name => "name",
            Self::MissingField => "missing_field",
            Self::StrayMember => "stray_member",
            Self::DuplicateMember => "duplicate_member",
            Self::Unreadable => "unreadable",
            Self::IdentityKey => "identity_key",
            Self::Signature => "signature",
            Self::Envelope => "envelope",
            Self::Broadcast => "broadcast",
        }
    }

    /// Its place in [`CardDrop::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The reason a parser fault is counted under: a surname with the
    /// name, both key faults as one, the three signature faults as one,
    /// the envelope-wide faults as one.
    #[must_use]
    pub const fn of(fault: CardFault) -> Self {
        match fault {
            CardFault::PhotoTooLarge => Self::PhotoSize,
            CardFault::PhotoEncoding => Self::PhotoEncoding,
            CardFault::PhotoMediaType => Self::PhotoMediaType,
            CardFault::Name | CardFault::Surname => Self::Name,
            CardFault::MissingCard => Self::MissingField,
            CardFault::TextPresent | CardFault::ReplyToPresent => Self::StrayMember,
            CardFault::DuplicateMember => Self::DuplicateMember,
            CardFault::Unreadable => Self::Unreadable,
            CardFault::IdentityKeyEncoding | CardFault::IdentityKeyLength => Self::IdentityKey,
            CardFault::SignatureEncoding
            | CardFault::SignatureLength
            | CardFault::SignatureFails => Self::Signature,
            CardFault::Version
            | CardFault::AppMessageId
            | CardFault::SentAtMs
            | CardFault::FromEndpoint => Self::Envelope,
        }
    }
}

/// What a `HumanChatV2` text is to the bridge.
#[derive(Debug)]
pub enum Kind {
    /// Not a card: forwarded as text, unparsed, as any other payload.
    Text,
    /// A well-formed card that verified under its key, received direct.
    Card(Box<Card>),
}

/// `text`, a `HumanChatV2` envelope's decoded text, received direct from
/// `source_peer`'s `source_endpoint`.
///
/// # Errors
/// The [`CardDrop`] reason for a card the parser calls malformed.
pub fn direct(
    text: &str,
    source_peer: &TransportIdentity,
    source_endpoint: &EndpointId,
) -> Result<Kind, CardDrop> {
    match classify(
        text,
        CardRoute {
            source_peer,
            source_endpoint,
        },
    ) {
        Classified::NotCard => Ok(Kind::Text),
        Classified::Card(card) => Ok(Kind::Card(Box::new(card))),
        Classified::MalformedCard(fault) => Err(CardDrop::of(fault)),
    }
}

/// `text`, a `HumanChatV2` envelope's decoded text, received over broadcast
/// from `publisher`. A card of any shape is dropped as
/// [`CardDrop::Broadcast`]: a broadcast has no source endpoint, and
/// whether a text is a card is decided before the route is read (the
/// parser's `classify`), so the endpoint given it here only reaches a
/// signature check whose answer is not used.
///
/// # Errors
/// [`CardDrop::Broadcast`] for any card, malformed or not.
pub fn broadcast(text: &str, publisher: &TransportIdentity) -> Result<(), CardDrop> {
    let unused = EndpointId::parse("broadcast").map_err(|_| CardDrop::Broadcast)?;
    match direct(text, publisher, &unused) {
        Ok(Kind::Text) => Ok(()),
        Ok(Kind::Card(_)) | Err(_) => Err(CardDrop::Broadcast),
    }
}

/// The card's own fields as `meta`, after the keys every direct event
/// carries (CHANNEL-EVENT.md §Metadata). The photo is its length and its
/// type, never its bytes; the identity key is as it came on the wire,
/// since the parser's base64 is canonical and so re-encodes to the same
/// 44 characters; nothing is derived from either.
///
/// # Errors
/// [`MetaError`], which a card the parser accepted does not reach.
pub fn set_meta(meta: &mut ChannelMeta, card: &Card) -> Result<(), MetaError> {
    meta.set(MetaKey::AppKind, "contact_card")?;
    meta.set(MetaKey::CardName, card.name.as_str())?;
    if let Some(surname) = &card.surname {
        meta.set(MetaKey::CardSurname, surname.as_str())?;
    }
    meta.set(MetaKey::CardPhotoBytes, card.photo_len().to_string())?;
    if let Some(photo) = &card.photo {
        meta.set(MetaKey::CardPhotoType, photo.media_type)?;
    }
    meta.set(MetaKey::CardIdentityKey, STANDARD.encode(card.identity_key))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every reason is in `ALL` once, at its own index.
    #[test]
    fn all_is_every_reason_once_in_declaration_order() {
        for (i, reason) in CardDrop::ALL.iter().enumerate() {
            assert_eq!(reason.index(), i, "{reason:?}");
        }
        let names: std::collections::BTreeSet<_> =
            CardDrop::ALL.iter().map(|r| r.as_str()).collect();
        assert_eq!(names.len(), CardDrop::ALL.len(), "names are distinct");
    }

    /// The grouping CHANNEL-EVENT.md §Content states, fault by fault.
    #[test]
    fn each_fault_is_counted_under_its_class() {
        use CardFault as F;
        let cases = [
            (F::PhotoTooLarge, CardDrop::PhotoSize),
            (F::PhotoEncoding, CardDrop::PhotoEncoding),
            (F::PhotoMediaType, CardDrop::PhotoMediaType),
            (F::Name, CardDrop::Name),
            (F::Surname, CardDrop::Name),
            (F::MissingCard, CardDrop::MissingField),
            (F::TextPresent, CardDrop::StrayMember),
            (F::ReplyToPresent, CardDrop::StrayMember),
            (F::DuplicateMember, CardDrop::DuplicateMember),
            (F::Unreadable, CardDrop::Unreadable),
            (F::IdentityKeyEncoding, CardDrop::IdentityKey),
            (F::IdentityKeyLength, CardDrop::IdentityKey),
            (F::SignatureEncoding, CardDrop::Signature),
            (F::SignatureLength, CardDrop::Signature),
            (F::SignatureFails, CardDrop::Signature),
            (F::Version, CardDrop::Envelope),
            (F::AppMessageId, CardDrop::Envelope),
            (F::SentAtMs, CardDrop::Envelope),
            (F::FromEndpoint, CardDrop::Envelope),
        ];
        for (fault, reason) in cases {
            assert_eq!(CardDrop::of(fault), reason, "{fault:?}");
        }
    }
}
