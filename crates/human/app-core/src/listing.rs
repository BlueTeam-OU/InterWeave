// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The store's rows, decoded for the model: what a root shows at start,
//! and the unread rows again when the facade held content it could not
//! hand over.

use interweave_human_chat_protocol::{
    CardRoute, Classified, HumanChatV2, classify, decode_envelope_bytes, parse_media_type,
};
use interweave_human_client_api::{Destination, Origin, ReceivedCard};
use std::collections::BTreeMap;

use interweave_human_store::{
    Cursor, HumanStore, InboundKind, InboundOrigin, Order, OutboundDestination, Page, PageLimits,
    PageRange, PendingOutbound, RowId, StoreConversation, StoreError, StoredInbound, WindowCursor,
    WindowLimits,
};
use interweave_human_ui_model::{
    ConversationKey, ListedInbound, ListedOutbound, NewestRow, PageAsk, PageEnd, PageMark,
    PageOrder, PageRows, StoredSummary, Table,
};
use interweave_transport_api::MediaType;

/// One page of a listing: records, and bytes, at most. Both ceilings are
/// the store's own rule -- a record count says nothing about 48 KiB
/// payloads -- and the walk continues page by page.
const PAGE_RECORDS: usize = 64;
const PAGE_BYTES: usize = 1024 * 1024;

pub(crate) fn limits() -> PageLimits {
    // Both non-zero: `new` refuses only a zero ceiling.
    PageLimits::new(PAGE_RECORDS, PAGE_BYTES).unwrap_or_else(|_| unreachable!())
}

/// The version of [`kind_of`], kept in the store as
/// `inbound_kind_version` (STATE.md schema v10). A build whose
/// classification of a stored row differs -- the envelope or card grammar
/// read another way -- bumps it, and its first listing classifies every
/// row again before anything is counted.
pub(crate) const INBOUND_KIND_VERSION: &str = "1";

/// A stored row's kind, by the same steps the listing takes to show it: a
/// message it shows as text, a card it shows as a proposal, and any other
/// row one this build cannot show.
#[must_use]
pub fn kind_of(row: &StoredInbound) -> InboundKind {
    if inbound(row).is_some() {
        InboundKind::Text
    } else if card(row).is_some() {
        InboundKind::Card
    } else {
        InboundKind::Other
    }
}

/// Classify every stored row again when the store's kinds were made by
/// another version of [`kind_of`], or by none, as after the v10
/// migration: before the first count is read.
///
/// # Errors
/// The store's; the kinds and their version are then as they were.
/// Each row classified is one read, counted in `read`.
pub(crate) fn reclassify_if_stale(
    store: &mut HumanStore,
    read: &mut u64,
) -> Result<(), StoreError> {
    if store.inbound_kind_version()?.as_deref() == Some(INBOUND_KIND_VERSION) {
        return Ok(());
    }
    store.reclassify_inbound(INBOUND_KIND_VERSION, |row| {
        *read += 1;
        kind_of(row)
    })
}

/// Every pending row, decoded, and how many could not be.
///
/// # Errors
/// The store's, when a page cannot be read.
pub(crate) fn list_pending(store: &HumanStore) -> Result<(Vec<ListedOutbound>, usize), StoreError> {
    let mut pending = Vec::new();
    let mut undecodable = 0;
    for row in walk(|after| store.pending_outbound_page(after, limits()))? {
        match outbound(&row) {
            Some(listed) => pending.push(listed),
            None => undecodable += 1,
        }
    }
    Ok((pending, undecodable))
}

/// What the Chats list needs of the store at a start, read without any
/// conversation's rows (design/2026-10-27-start-window.md section 3).
pub(crate) struct Summarised {
    /// Each conversation's count and newest row.
    pub(crate) summaries: Vec<StoredSummary>,
    /// The unread table's high-water mark, read with the counts.
    pub(crate) counted_through: Option<RowId>,
    /// The rows the store gave: one per conversation, its newest.
    pub(crate) read: u64,
}

/// Every conversation the store shows a row of, with its unread count and
/// its newest row: counts from the indexes, and one row read per
/// conversation. The facade alone writes the store and this runs on its
/// side, so no commit falls between the counts and the mark.
///
/// # Errors
/// The store's, when a count or a row cannot be read.
pub(crate) fn summaries(store: &HumanStore) -> Result<Summarised, StoreError> {
    let counts: BTreeMap<StoreConversation, u64> = store
        .unread_counts()?
        .into_iter()
        .map(|c| (c.conversation, c.unread))
        .collect();
    let counted_through = store.unread_high_water()?;
    let newest_only = PageLimits::new(1, PAGE_BYTES).unwrap_or_else(|_| unreachable!());
    let mut summaries = Vec::new();
    let mut read = 0;
    for conversation in store.shown_conversations()? {
        let page = store.conversation_page(&conversation, None, newest_only)?;
        read += u64::try_from(page.items.len()).unwrap_or(u64::MAX);
        let newest = page.items.first().and_then(|row| {
            let table = if row.kept_at.is_some() {
                Table::Kept
            } else {
                Table::Unread
            };
            if let Some(message) = inbound(row) {
                Some(NewestRow::message(
                    table,
                    row.row_id,
                    row.received_at,
                    &message.envelope.text,
                ))
            } else {
                card(row).map(|_| NewestRow::card(row.row_id, row.received_at))
            }
        });
        summaries.push(StoredSummary {
            unread: counts.get(&conversation).copied().unwrap_or(0),
            conversation: match conversation {
                StoreConversation::Direct { peer, endpoint } => ConversationKey::Direct {
                    peer,
                    endpoint: Some(endpoint),
                },
                StoreConversation::Channel(channel) => ConversationKey::Channel(channel),
            },
            newest,
        });
    }
    Ok(Summarised {
        summaries,
        counted_through,
        read,
    })
}

/// One page of the open conversation as `ask` names it: its rows decoded,
/// each table apart, its cards as proposals, and whether more remain -- at
/// the unread room said apart, since the limit row shows only then -- with
/// how many rows the store gave.
///
/// # Errors
/// The store's, when the page or a proposal's context cannot be read.
pub(crate) fn page(store: &HumanStore, ask: PageAsk) -> Result<(PageRows, u64), StoreError> {
    let mut rows = PageRows {
        ask,
        unread: Vec::new(),
        kept: Vec::new(),
        proposals: Vec::new(),
        end: PageEnd::Exhausted,
    };
    let conversation = match &rows.ask.conversation {
        ConversationKey::Direct {
            peer,
            endpoint: Some(endpoint),
        } => StoreConversation::Direct {
            peer: peer.clone(),
            endpoint: endpoint.clone(),
        },
        ConversationKey::Channel(channel) => StoreConversation::Channel(channel.clone()),
        // The peer's default endpoint: no stored row is under it, since
        // each records the endpoint its sender asserted.
        ConversationKey::Direct { endpoint: None, .. } => return Ok((rows, 0)),
    };
    let cursor = |m: PageMark| WindowCursor::at(m.received_at, m.kept, m.row);
    let window = store.conversation_window(
        &conversation,
        PageRange {
            older_than: rows.ask.older_than.map(cursor),
            newer_than: rows.ask.newer_than.map(cursor),
        },
        match rows.ask.order {
            PageOrder::NewestFirst => Order::NewestFirst,
            PageOrder::OldestFirst => Order::OldestFirst,
        },
        WindowLimits {
            page: limits(),
            unread: rows.ask.unread_room,
        },
    )?;
    let read = u64::try_from(window.page.items.len()).unwrap_or(u64::MAX);
    rows.end = match (window.page.next, window.room_used) {
        (None, _) => PageEnd::Exhausted,
        (Some(_), true) => PageEnd::RoomUsed,
        (Some(_), false) => PageEnd::More,
    };
    // A page holds the kinds this build shows, by the same steps as these
    // (`kind_of`, classified again at the start when another build made
    // them), so no row of it is left out here.
    for row in window.page.items {
        if row.kept_at.is_some() {
            rows.kept.extend(inbound(&row));
        } else if let Some(message) = inbound(&row) {
            rows.unread.push(message);
        } else if let Some(card) = card(&row) {
            rows.proposals.push(crate::cards::proposal(store, card)?);
        }
    }
    Ok((rows, read))
}

/// Every unread card, verified on its route, read by the store's kind
/// index rather than by a walk of the unread messages (j78); each row
/// read is counted in `read`.
///
/// # Errors
/// The store's, when a page cannot be read.
pub(crate) fn list_unread_cards(
    store: &HumanStore,
    read: &mut u64,
) -> Result<Vec<ReceivedCard>, StoreError> {
    let rows = walk(|after| store.unread_cards_page(after, limits()))?;
    *read += u64::try_from(rows.len()).unwrap_or(u64::MAX);
    Ok(rows.iter().filter_map(card).collect())
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

#[cfg(test)]
mod tests {
    use super::{INBOUND_KIND_VERSION, reclassify_if_stale};
    use interweave_human_store::{
        AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, StoreOptions,
    };
    use interweave_transport_api::{EndpointId, MediaType, TransportIdentity};

    fn row(n: u8, payload: &[u8]) -> NewInbound {
        NewInbound {
            app_message_id: AppMessageId::parse(format!("{n:032x}")).expect("an id"),
            origin: InboundOrigin {
                peer: TransportIdentity::parse(
                    "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN",
                )
                .expect("a PeerId"),
                endpoint: Some(EndpointId::parse("human").expect("an endpoint")),
                channel: None,
            },
            media_type: Some(
                MediaType::parse("application/vnd.interweave-human-chat+json;v=2").expect("media"),
            ),
            payload: payload.to_vec(),
            received_at: u64::from(n),
            // As the v10 migration leaves every row before a client ran.
            kind: InboundKind::Other,
        }
    }

    /// A store whose kinds no client has classified -- every row `other`,
    /// as v10 leaves them -- has them classified at the first read, and
    /// the version recorded, so the next start leaves them alone: it reads
    /// no row. Mutations: the stale store taken for a current one; the
    /// version not consulted, every start classifying again.
    #[test]
    fn a_store_classified_by_no_client_is_classified_once() {
        let mut store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
        let text = br#"{"v":2,"kind":"text","app_message_id":"00000000000000000000000000000001","text":"hi"}"#;
        store
            .commit_unread_inbound(&row(1, text))
            .expect("a message");
        store
            .commit_unread_inbound(&row(2, b"no envelope"))
            .expect("a row this build cannot show");
        let mut read = 0;
        reclassify_if_stale(&mut store, &mut read).expect("classified");
        assert_eq!(read, 2, "each row read once");
        let kinds: Vec<_> = store
            .unread_inbound()
            .expect("read")
            .iter()
            .map(|r| r.kind)
            .collect();
        assert_eq!(kinds, [InboundKind::Text, InboundKind::Other]);
        assert_eq!(
            store.inbound_kind_version().expect("version").as_deref(),
            Some(INBOUND_KIND_VERSION)
        );
        let mut again = 0;
        reclassify_if_stale(&mut store, &mut again).expect("the next start");
        assert_eq!(again, 0, "the next start reads no row");
    }
}
