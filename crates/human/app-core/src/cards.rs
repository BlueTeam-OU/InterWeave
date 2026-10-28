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
use interweave_human_store::identity_key::Fingerprint;
use interweave_human_store::{
    AcceptedCard, CardStanding, Contact, ContactName, ContactRoute, HumanStore, InboundOrigin,
    KeyStanding, OutboundDestination, PendingOutbound, ReadEphemeral, RowId, StoreError,
    ValueSource,
};
use interweave_human_ui_model::{
    ContactRef, ContactView, DirectoryContact, IdentityView, KeyView, OwnKey, Photo, PhotoChange,
    Proposal, ProposalContext, RouteView, ValueChanges,
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

fn refs(contacts: &[ContactName]) -> Vec<ContactRef> {
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
    let card_as_accepted = accepted(peer, endpoint, card);
    let standing = store.card_standing(&card_as_accepted)?;
    let CardStanding {
        updates,
        shares_node,
        same_name,
        key,
        key_matches,
    } = standing;
    // What Accept would do to the photo (design 5.4.1, as #443 left it).
    // A local choice stays -- the person's photo, or no photo by their
    // Remove -- whatever the card carries. Any other photo becomes the
    // card's whenever the card carries one, since a photo not yet decoded
    // cannot be compared, and goes when the card carries none.
    let card_carries = card.photo.is_some();
    let photo_change = |c: &Contact| match (
        c.photo_source == Some(ValueSource::Person),
        c.photo.is_some(),
    ) {
        (true, true) => PhotoChange::Kept,
        (true, false) if card_carries => PhotoChange::KeptNone,
        (false, _) if card_carries => PhotoChange::New,
        (false, true) => PhotoChange::Removed,
        (true | false, false) => PhotoChange::Unchanged,
    };
    // What Accept would leave as it is, judged as `HumanStore::accept_card`
    // writes: a person's value stays, any other becomes the card's, and an
    // unrecorded key is recorded.
    let changes_nothing = matches!(key, KeyStanding::Same)
        && updates.as_ref().is_some_and(|c| {
            (c.name_source == ValueSource::Person
                || c.display_name == card_as_accepted.display_name())
                && !matches!(photo_change(c), PhotoChange::New | PhotoChange::Removed)
        });
    // The values design 5.4.1 names (cases U, E and S): a person's name
    // stays, any other becomes the card's, and the photo as above.
    let values = updates.as_ref().map_or_else(ValueChanges::default, |c| {
        let name_differs = c.display_name != card_as_accepted.display_name();
        let person_name = c.name_source == ValueSource::Person;
        ValueChanges {
            new_name: (name_differs && !person_name).then(|| card_as_accepted.display_name()),
            kept_name: name_differs && person_name,
            photo: photo_change(c),
        }
    });
    let context = ProposalContext {
        values,
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
        changes_nothing,
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

/// A kept photo decoded for the model, or nothing when it does not
/// decode within the card's bound.
pub(crate) fn shown(bytes: &[u8]) -> Option<Photo> {
    crate::card_photo::decode_kept(bytes).map(|p| Photo {
        width: p.width,
        height: p.height,
        rgba: p.rgba,
    })
}

/// The card one share sent, for its own card row (contact-cards section
/// 5.5): its name and surname as one line, its photo decoded for display.
pub(crate) fn sent_card(
    name: &str,
    surname: Option<&str>,
    photo: Option<&[u8]>,
) -> interweave_human_ui_model::SentCard {
    interweave_human_ui_model::SentCard {
        name: surname.map_or_else(|| name.to_owned(), |s| format!("{name} {s}")),
        photo: photo.and_then(shown),
    }
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
            name_at: c.name_at,
            photo_from_card: c.photo_source.map(|s| s == ValueSource::Card),
            photo_at: c.photo_at,
            photo: c.photo.as_deref().and_then(shown),
            routes: route_views(c.routes),
        })
        .collect())
}

fn route_views(routes: Vec<ContactRoute>) -> Vec<RouteView> {
    routes
        .into_iter()
        .map(|r| RouteView {
            fingerprint: r.identity_key.as_ref().map(fingerprint),
            peer: r.peer,
            endpoint: r.endpoint,
        })
        .collect()
}

/// Every contact as the windowed mode reads them: names and routes, and
/// whether each has a photo, never a photo's bytes -- a photo is read when
/// a view shows it (design/2026-10-27-start-window.md section 11).
pub(crate) fn contact_directory(store: &HumanStore) -> Result<Vec<DirectoryContact>, StoreError> {
    /// Contacts per page: a page holds no photo, so a count bounds it.
    const DIRECTORY_PAGE: usize = 256;
    let mut contacts = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let page = store.contact_directory_page(after.as_deref(), DIRECTORY_PAGE)?;
        contacts.extend(page.items.into_iter().map(|c| DirectoryContact {
            has_photo: c.has_photo,
            view: ContactView {
                contact_id: c.contact_id,
                name: c.display_name,
                name_from_card: c.name_source == ValueSource::Card,
                name_at: c.name_at,
                photo_from_card: c.photo_source.map(|s| s == ValueSource::Card),
                photo_at: c.photo_at,
                photo: None,
                routes: route_views(c.routes),
            },
        }));
        match page.next {
            Some(next) => after = Some(next),
            None => return Ok(contacts),
        }
    }
}

/// The identity in front, as the model shows it.
pub(crate) fn identity_view(
    store: &mut HumanStore,
    endpoint: &EndpointId,
    key: OwnKey,
    peer: Option<&TransportIdentity>,
    at: u64,
) -> Result<IdentityView, StoreError> {
    let card = store.ensure_own_card(endpoint, at)?;
    Ok(IdentityView {
        endpoint: endpoint.clone(),
        name: card
            .surname
            .as_ref()
            .map_or_else(|| card.name.clone(), |s| format!("{} {s}", card.name)),
        given: card.name.clone(),
        surname: card.surname.clone(),
        has_photo: card.photo.is_some(),
        photo: card.photo.as_deref().and_then(shown),
        key,
        peer: peer.cloned(),
    })
}

/// The identity's own card rows still pending, by row, with the route
/// each goes to: a card is told from a message by its payload, since it
/// is no `HumanChatV2` envelope.
///
/// # Errors
/// The store's, when a page cannot be read.
pub(crate) fn own_cards_pending(
    store: &HumanStore,
) -> Result<Vec<(RowId, TransportIdentity, EndpointId)>, StoreError> {
    let rows =
        crate::listing::walk(|after| store.pending_outbound_page(after, crate::listing::limits()))?;
    Ok(rows
        .iter()
        .filter_map(|row| own_card_route(row).map(|(peer, endpoint)| (row.row_id, peer, endpoint)))
        .collect())
}

/// The route of a pending row that holds the identity's own card, `None`
/// for any other row.
pub(crate) fn own_card_route(row: &PendingOutbound) -> Option<(TransportIdentity, EndpointId)> {
    let OutboundDestination::Direct(direct) = &row.destination else {
        return None;
    };
    let endpoint = direct.endpoint.clone()?;
    let info = parse_media_type(row.media_type.as_ref()?.as_str()).ok()?;
    let text = decode_envelope_bytes(&row.payload, info.encoding).ok()?;
    is_card(&text).then(|| (direct.peer.clone(), endpoint))
}

/// Whether any conversation holds or may hold the identity's card: a
/// share recorded (`card_shared_at`, written at Accepted) or an own card
/// row still pending outbound, Sending or Not sent -- the two sources
/// Make a new key's share-again list unions (`new_key_start`). A read
/// that fails counts as held, so a key is never made silently on a guess
/// (design/2026-10-24-identity-key.md section 7, ADR-0063 A 2026-10-27;
/// `tests/cards.rs`: `a_key_file_gone_while_a_card_is_pending_is_missing`;
/// a failed read: `a_failed_read_counts_as_held`).
pub(crate) fn card_may_be_held(store: &HumanStore) -> bool {
    held(store.card_ever_shared(), || {
        own_cards_pending(store).map(|rows| rows.len())
    })
}

/// [`card_may_be_held`] from its two reads: whether a share was recorded,
/// and how many own card rows are pending, read only when none was.
fn held(
    shared: Result<bool, StoreError>,
    pending: impl FnOnce() -> Result<usize, StoreError>,
) -> bool {
    shared.unwrap_or(true) || pending().map_or(true, |rows| rows > 0)
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

#[cfg(test)]
mod held_tests {
    use super::{StoreError, held};

    #[test]
    fn a_failed_read_counts_as_held() {
        let failed = || StoreError::Corrupt("told to".to_owned());
        assert!(held(Err(failed()), || Ok(0)), "the shares unread");
        assert!(held(Ok(false), || Err(failed())), "the pending rows unread");
        assert!(held(Ok(true), || Ok(0)), "a share recorded");
        assert!(held(Ok(false), || Ok(1)), "a card on its way");
        assert!(!held(Ok(false), || Ok(0)), "neither");
    }
}
