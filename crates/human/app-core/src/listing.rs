// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The store's rows, decoded for the model: what a root shows at start,
//! and the unread rows again when the facade held content it could not
//! hand over.

use interweave_human_chat_protocol::{
    CardRoute, Classified, HumanChatV2, classify, decode_envelope_bytes, parse_media_type,
};
use interweave_human_client_api::{Destination, Origin, ReceivedCard};
use interweave_human_store::{
    Cursor, HumanStore, InboundOrigin, OutboundDestination, Page, PageLimits, PendingOutbound,
    StoreError, StoredInbound,
};
use interweave_human_ui_model::{ListedInbound, ListedOutbound};
use interweave_transport_api::MediaType;

use crate::protocol::Listing;

/// One page of a listing: records, and bytes, at most. Both ceilings are
/// the store's own rule -- a record count says nothing about 48 KiB
/// payloads -- and the walk continues page by page.
const PAGE_RECORDS: usize = 64;
const PAGE_BYTES: usize = 1024 * 1024;

pub(crate) fn limits() -> PageLimits {
    // Both non-zero: `new` refuses only a zero ceiling.
    PageLimits::new(PAGE_RECORDS, PAGE_BYTES).unwrap_or_else(|_| unreachable!())
}

/// Every pending, unread and kept row, decoded.
///
/// # Errors
/// The store's, when a page cannot be read.
pub(crate) fn list_all(store: &HumanStore) -> Result<Listing, StoreError> {
    let mut listing = Listing::default();
    for row in walk(|after| store.pending_outbound_page(after, limits()))? {
        match outbound(&row) {
            Some(listed) => listing.pending.push(listed),
            None => listing.undecodable_other += 1,
        }
    }
    let (unread, cards, undecodable) = list_unread(store)?;
    listing.unread = unread;
    listing.undecodable_unread = undecodable;
    for card in cards {
        listing
            .unread_cards
            .push(crate::cards::proposal(store, card)?);
    }
    listing.contacts = crate::cards::contact_views(store)?;
    for row in walk(|after| store.kept_inbound_page(after, limits()))? {
        match inbound(&row) {
            Some(listed) => listing.kept.push(listed),
            None => listing.undecodable_other += 1,
        }
    }
    Ok(listing)
}

/// Every unread row, decoded: the messages, the cards, and how many rows
/// were neither.
///
/// # Errors
/// The store's, when a page cannot be read.
pub(crate) fn list_unread(
    store: &HumanStore,
) -> Result<(Vec<ListedInbound>, Vec<ReceivedCard>, usize), StoreError> {
    let mut listed = Vec::new();
    let mut cards = Vec::new();
    let mut undecodable = 0;
    for row in walk(|after| store.unread_inbound_page(after, limits()))? {
        if let Some(item) = inbound(&row) {
            listed.push(item);
        } else if let Some(card) = card(&row) {
            cards.push(card);
        } else {
            undecodable += 1;
        }
    }
    Ok((listed, cards, undecodable))
}

/// A row that holds a card, verified on the route it was stored with:
/// the same steps the facade takes on receipt, so a card shown as a
/// proposal when it arrived is one again after a restart.
fn card(row: &StoredInbound) -> Option<ReceivedCard> {
    let InboundOrigin {
        peer,
        endpoint: Some(endpoint),
        channel: None,
    } = &row.origin
    else {
        return None;
    };
    let info = parse_media_type(row.media_type.as_ref()?.as_str()).ok()?;
    let text = decode_envelope_bytes(&row.payload, info.encoding).ok()?;
    let Classified::Card(card) = classify(
        &text,
        CardRoute {
            source_peer: peer,
            source_endpoint: endpoint,
        },
    ) else {
        return None;
    };
    Some(ReceivedCard {
        row: row.row_id,
        peer: peer.clone(),
        endpoint: endpoint.clone(),
        card,
        received_at: row.received_at,
    })
}

pub(crate) fn walk<T>(
    mut page: impl FnMut(Option<Cursor>) -> Result<Page<T>, StoreError>,
) -> Result<Vec<T>, StoreError> {
    let mut rows = Vec::new();
    let mut after = None;
    loop {
        let Page { items, next } = page(after)?;
        rows.extend(items);
        match next {
            Some(cursor) => after = Some(cursor),
            None => return Ok(rows),
        }
    }
}

fn outbound(row: &PendingOutbound) -> Option<ListedOutbound> {
    let destination = match &row.destination {
        OutboundDestination::Direct(direct) => Destination::Direct {
            peer: direct.peer.clone(),
            endpoint: direct.endpoint.clone(),
        },
        OutboundDestination::Broadcast(channel) => Destination::Broadcast(channel.clone()),
    };
    if let Some(envelope) = decode(row.media_type.as_ref(), &row.payload) {
        return Some(ListedOutbound {
            row: row.row_id,
            destination,
            envelope,
            created_at: row.created_at,
            card: None,
        });
    }
    // The identity's own card, read back to show it as it went (contact-
    // cards section 5.5): its own write, unverified, since this node's
    // PeerId is not known before a session (HUMAN-CHAT.md section Contact
    // card, A 2026-10-27). One the grammar refuses stays undecodable.
    let info = parse_media_type(row.media_type.as_ref()?.as_str()).ok()?;
    let text = decode_envelope_bytes(&row.payload, info.encoding).ok()?;
    let own = interweave_human_chat_protocol::read_own_card_unverified(&text)?;
    let card = crate::cards::sent_card(
        &own.name,
        own.surname.as_deref(),
        own.photo.as_ref().map(|p| p.bytes.as_slice()),
    );
    Some(ListedOutbound::own_card(
        row.row_id,
        destination,
        own.app_message_id,
        card,
        row.created_at,
    ))
}

fn inbound(row: &StoredInbound) -> Option<ListedInbound> {
    let InboundOrigin {
        peer,
        endpoint,
        channel,
    } = &row.origin;
    // A direct row records the endpoint the sender asserted, a broadcast
    // row its channel; a row with neither, or both, is not one the
    // facade wrote.
    let origin = match (endpoint, channel) {
        (Some(endpoint), None) => Origin::Direct {
            peer: peer.clone(),
            endpoint: endpoint.clone(),
        },
        (None, Some(channel)) => Origin::Channel {
            channel: channel.clone(),
            publisher: peer.clone(),
        },
        _ => return None,
    };
    Some(ListedInbound {
        row: row.row_id,
        origin,
        envelope: decode(row.media_type.as_ref(), &row.payload)?,
        received_at: row.received_at,
    })
}

/// The envelope a stored payload carries: the same steps the facade
/// takes on receipt (`HUMAN-CHAT.md` §Consumers), so a row that was
/// shown when it arrived is shown again after a restart.
fn decode(media_type: Option<&MediaType>, payload: &[u8]) -> Option<HumanChatV2> {
    let info = parse_media_type(media_type?.as_str()).ok()?;
    let text = decode_envelope_bytes(payload, info.encoding).ok()?;
    HumanChatV2::parse(&text).ok()
}
