// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade side's half of contact cards (plan §29 step 2): what a
//! received card would do, read from the store for its proposal; the
//! contacts and the identity in front, as the model shows them; and the
//! card a read row held, verified again before Accept writes anything.

use interweave_human_chat_protocol::{
    Card, CardRoute, Classified, classify, decode_envelope_bytes, is_card, parse_media_type,
};
use interweave_human_client_api::ReceivedCard;
use interweave_human_store::identity_key::{Fingerprint, IdentityKey};
use interweave_human_store::{
    AcceptedCard, CardStanding, Contact, HumanStore, InboundOrigin, KeyStanding, ReadEphemeral,
    StoreError, ValueSource,
};
use interweave_human_ui_model::{
    ContactRef, ContactView, IdentityView, KeyView, Proposal, ProposalContext, RouteView,
};
use interweave_transport_api::{EndpointId, TransportIdentity};

/// The card as the parser gave it, as Accept passes it to the store.
pub(crate) fn accepted<'a>(
    peer: &'a TransportIdentity,
    endpoint: &'a EndpointId,
    card: &'a Card,
) -> AcceptedCard<'a> {
    AcceptedCard {
        peer,
        endpoint,
        name: &card.name,
        surname: card.surname.as_deref(),
        identity_key: card.identity_key,
        photo: card.photo.as_ref().map(|p| p.bytes.as_slice()),
    }
}

fn fingerprint(key: &[u8; 32]) -> String {
    Fingerprint::of(key).to_string()
}

fn refs(contacts: &[Contact]) -> Vec<ContactRef> {
    contacts
        .iter()
        .map(|c| ContactRef {
            contact_id: c.contact_id.clone(),
            name: c.display_name.clone(),
        })
        .collect()
}

/// The proposal for a card committed unread: what Accept would do.
pub(crate) fn proposal(store: &HumanStore, received: ReceivedCard) -> Result<Proposal, StoreError> {
    let context = context(store, &received.peer, &received.endpoint, &received.card)?;
    Ok(Proposal {
        row: received.row,
        peer: received.peer,
        endpoint: received.endpoint,
        card: received.card,
        received_at: received.received_at,
        context,
    })
}

/// What Accept of `card` on its route would do, as the store stands now.
pub(crate) fn context(
    store: &HumanStore,
    peer: &TransportIdentity,
    endpoint: &EndpointId,
    card: &Card,
) -> Result<ProposalContext, StoreError> {
    let standing = store.card_standing(&accepted(peer, endpoint, card))?;
    let CardStanding {
        updates,
        shares_node,
        same_name,
        key,
        key_matches,
    } = standing;
    let context = ProposalContext {
        updates: updates.as_ref().map(|c| ContactRef {
            contact_id: c.contact_id.clone(),
            name: c.display_name.clone(),
        }),
        shares_node: refs(&shares_node),
        same_name: refs(&same_name),
        key_matches: refs(&key_matches),
        key: match key {
            KeyStanding::Unrecorded => KeyView::New,
            KeyStanding::Same => KeyView::Same,
            KeyStanding::Changed { recorded } => KeyView::Changed {
                recorded: fingerprint(&recorded),
            },
        },
        fingerprint: fingerprint(&card.identity_key),
    };
    Ok(context)
}

/// The photo Accept keeps: the card's, as received, when it decodes with
/// the memory-safe decoder within 256 × 256, and none when it does not
/// (HUMAN-CHAT.md: such a photo is discarded and the card kept without
/// it). The bytes kept are decoded again at display (STATE.md).
pub(crate) fn kept_photo(card: &Card) -> Option<&[u8]> {
    card.photo
        .as_ref()
        .filter(|p| crate::card_photo::decode_received(p).is_some())
        .map(|p| p.bytes.as_slice())
}

/// Every contact, as the model shows it.
pub(crate) fn contact_views(store: &HumanStore) -> Result<Vec<ContactView>, StoreError> {
    Ok(store
        .contacts()?
        .into_iter()
        .map(|c| ContactView {
            contact_id: c.contact_id,
            name: c.display_name,
            name_from_card: c.name_source == ValueSource::Card,
            name_at: None,
            photo_from_card: c.photo_source.map(|s| s == ValueSource::Card),
            photo_at: None,
            photo: None,
            routes: c
                .routes
                .into_iter()
                .map(|r| RouteView {
                    fingerprint: r.identity_key.as_ref().map(fingerprint),
                    peer: r.peer,
                    endpoint: r.endpoint,
                })
                .collect(),
        })
        .collect())
}

/// The identity in front, as the model shows it.
pub(crate) fn identity_view(
    store: &mut HumanStore,
    key: &IdentityKey,
    peer: Option<&TransportIdentity>,
    at: u64,
) -> Result<IdentityView, StoreError> {
    let card = store.ensure_own_card(key.endpoint(), at)?;
    Ok(IdentityView {
        endpoint: key.endpoint().clone(),
        name: card
            .surname
            .as_ref()
            .map_or_else(|| card.name.clone(), |s| format!("{} {s}", card.name)),
        given: card.name.clone(),
        surname: card.surname.clone(),
        has_photo: card.photo.is_some(),
        photo: None,
        fingerprint: key.fingerprint().to_string(),
        peer: peer.cloned(),
    })
}

/// Whether a read row holds a contact card by its kind, verified or not:
/// what Keep refuses (RETENTION.md: a card has no Keep), so a card that
/// would no longer verify is not keepable either (`tests/cards.rs`,
/// `a_keep_of_a_card_that_no_longer_verifies_is_refused_by_its_kind`).
pub(crate) fn holds_card(held: &ReadEphemeral) -> bool {
    held.media_type()
        .and_then(|m| parse_media_type(m.as_str()).ok())
        .and_then(|info| decode_envelope_bytes(held.payload(), info.encoding).ok())
        .is_some_and(|text| is_card(&text))
}

/// The route and card a read row held, verified again on that route: a
/// row the facade committed as a card, and nothing else, gives one.
pub(crate) fn card_of(held: &ReadEphemeral) -> Option<(TransportIdentity, EndpointId, Card)> {
    let InboundOrigin {
        peer,
        endpoint: Some(endpoint),
        channel: None,
    } = held.origin()
    else {
        return None;
    };
    let info = parse_media_type(held.media_type()?.as_str()).ok()?;
    let text = decode_envelope_bytes(held.payload(), info.encoding).ok()?;
    match classify(
        &text,
        CardRoute {
            source_peer: peer,
            source_endpoint: endpoint,
        },
    ) {
        Classified::Card(card) => Some((peer.clone(), endpoint.clone(), card)),
        Classified::NotCard | Classified::MalformedCard(_) => None,
    }
}
