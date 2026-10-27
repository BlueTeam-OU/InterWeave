// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The start window's reads (`design/2026-10-27-start-window.md`, j68
//! P2-1): a conversation's rows a page at a time, newest first, every one
//! reachable and none twice; the badge's counts; the hidden rows.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use interweave_human_store::{
    AppMessageId, ConversationUnread, HiddenInbound, HumanStore, InboundKind, InboundOrigin,
    NewInbound, PageLimits, RowId, StoreConversation, StoreOptions, WindowCursor,
};
use interweave_transport_api::{ChannelId, EndpointId, TransportIdentity};

const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
const Q: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";

fn peer(s: &str) -> TransportIdentity {
    TransportIdentity::parse(s).expect("a PeerId")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("an endpoint")
}

fn store() -> (tempfile::TempDir, HumanStore) {
    let dir = common::private_tempdir();
    let store = HumanStore::open(
        &dir.path().join("state").join("human.sqlite3"),
        StoreOptions::default(),
    )
    .expect("opens");
    (dir, store)
}

/// Message `n` from `from` on `endpoint`, received at `at`.
fn direct(n: u32, from: &str, at: u64, kind: InboundKind) -> NewInbound {
    NewInbound {
        app_message_id: AppMessageId::parse(format!("{n:032x}")).expect("an id"),
        origin: InboundOrigin {
            peer: peer(from),
            endpoint: Some(human()),
            channel: None,
        },
        media_type: None,
        payload: n.to_be_bytes().to_vec(),
        received_at: at,
        kind,
    }
}

fn on_channel(n: u32, publisher: &str, channel: &ChannelId) -> NewInbound {
    NewInbound {
        origin: InboundOrigin {
            peer: peer(publisher),
            endpoint: None,
            channel: Some(channel.clone()),
        },
        ..direct(n, publisher, u64::from(n), InboundKind::Text)
    }
}

fn number(payload: &[u8]) -> u32 {
    u32::from_be_bytes(payload.try_into().expect("four bytes"))
}

/// Every page of `conversation`, as numbers, walked to the end.
fn walk(store: &HumanStore, conversation: &StoreConversation, records: usize) -> Vec<Vec<u32>> {
    let limits = PageLimits::new(records, 1024 * 1024).expect("limits");
    let mut pages = Vec::new();
    let mut before: Option<WindowCursor> = None;
    loop {
        let page = store
            .conversation_page(conversation, before, limits)
            .expect("a page");
        pages.push(page.items.iter().map(|r| number(&r.payload)).collect());
        // A walk that does not end fails rather than hanging the test.
        assert!(pages.len() <= 64, "the walk does not end: {pages:?}");
        match page.next {
            Some(next) => before = Some(next),
            None => return pages,
        }
    }
}

/// P2-1's test (2): rows past the first window stay in the store and are
/// fetched later. Walked a page at a time, a conversation gives every row
/// it shows exactly once, newest first, unread and kept together -- rows
/// received at the same instant in both tables included -- and nothing of
/// another conversation, nor any row this build cannot show. Mutations:
/// the bound `<` read as `<=` (a row twice); the kept table not read.
#[test]
fn a_conversation_is_paged_newest_first_with_every_row_once() {
    let (_dir, mut store) = store();
    // Unread 1..=7 at times 10..=70; 4 is read and kept, so kept and unread
    // share instants with each other's neighbours; 8 at the same instant as
    // 7; 9 cannot be shown; 100 is Q's.
    for n in 1..=7 {
        store
            .commit_unread_inbound(&direct(n, P, u64::from(n) * 10, InboundKind::Text))
            .expect("committed");
    }
    store
        .commit_unread_inbound(&direct(8, P, 70, InboundKind::Card))
        .expect("same instant");
    store
        .commit_unread_inbound(&direct(9, P, 80, InboundKind::Other))
        .expect("hidden");
    store
        .commit_unread_inbound(&direct(100, Q, 90, InboundKind::Text))
        .expect("another conversation");
    let four = store
        .unread_inbound()
        .expect("read")
        .into_iter()
        .find(|r| number(&r.payload) == 4)
        .expect("four")
        .row_id;
    let read = store.mark_read(four, 1).expect("read");
    store.keep(&read, 2).expect("kept");

    let conversation = StoreConversation::Direct {
        peer: peer(P),
        endpoint: human(),
    };
    for records in [1, 2, 3, 8, 64] {
        let pages = walk(&store, &conversation, records);
        let all: Vec<u32> = pages.iter().flatten().copied().collect();
        assert_eq!(
            all,
            [8, 7, 6, 5, 4, 3, 2, 1],
            "pages of {records}: {pages:?}"
        );
        assert!(
            pages.iter().all(|p| p.len() <= records),
            "pages of {records}: {pages:?}"
        );
    }
}

/// A page holds no more payload than its byte budget, but always one row,
/// so a large row cannot stop the walk. Mutation: the byte budget ignored.
#[test]
fn a_page_keeps_to_its_byte_budget() {
    let (_dir, mut store) = store();
    for n in 1..=4 {
        store
            .commit_unread_inbound(&NewInbound {
                payload: vec![0; 1000],
                ..direct(n, P, u64::from(n), InboundKind::Text)
            })
            .expect("committed");
    }
    let conversation = StoreConversation::Direct {
        peer: peer(P),
        endpoint: human(),
    };
    let page = store
        .conversation_page(
            &conversation,
            None,
            PageLimits::new(64, 2500).expect("limits"),
        )
        .expect("a page");
    assert_eq!(page.items.len(), 2, "two rows of 1000 bytes fit 2500");
    assert!(page.next.is_some());
    let one = store
        .conversation_page(
            &conversation,
            None,
            PageLimits::new(64, 10).expect("limits"),
        )
        .expect("a page");
    assert_eq!(one.items.len(), 1, "one row, whatever its size");
}

/// The badge's counts (architect-cto's A3): distinct message ids among a
/// conversation's unread rows it shows, a channel being one conversation
/// whoever published on it; a row this build cannot show is not counted
/// but counted apart, and a kept row is not unread. Mutation: `COUNT(*)`
/// for the distinct count.
#[test]
fn the_unread_counts_are_the_badges() {
    let (_dir, mut store) = store();
    for n in 1..=3 {
        store
            .commit_unread_inbound(&direct(n, P, u64::from(n), InboundKind::Text))
            .expect("committed");
    }
    store
        .commit_unread_inbound(&direct(4, P, 4, InboundKind::Card))
        .expect("a card");
    store
        .commit_unread_inbound(&direct(5, P, 5, InboundKind::Other))
        .expect("hidden");
    store
        .commit_unread_inbound(&direct(6, Q, 6, InboundKind::Text))
        .expect("Q's");
    let room = ChannelId::parse("room").expect("a channel");
    // One message id published by two peers on the channel: one item.
    store
        .commit_unread_inbound(&on_channel(7, P, &room))
        .expect("on the channel");
    store
        .commit_unread_inbound(&on_channel(7, Q, &room))
        .expect("the same id, from Q");
    store
        .commit_unread_inbound(&on_channel(8, Q, &room))
        .expect("another");
    let two = store
        .unread_inbound()
        .expect("read")
        .into_iter()
        .find(|r| number(&r.payload) == 2)
        .expect("two")
        .row_id;
    let read = store.mark_read(two, 1).expect("read");
    store.keep(&read, 2).expect("kept");

    let mut counts = store.unread_counts().expect("counts");
    counts.sort_by(|a, b| a.conversation.cmp(&b.conversation));
    let mut expected = vec![
        ConversationUnread {
            conversation: StoreConversation::Direct {
                peer: peer(P),
                endpoint: human(),
            },
            unread: 3,
        },
        ConversationUnread {
            conversation: StoreConversation::Direct {
                peer: peer(Q),
                endpoint: human(),
            },
            unread: 1,
        },
        ConversationUnread {
            conversation: StoreConversation::Channel(room),
            unread: 2,
        },
    ];
    expected.sort_by(|a, b| a.conversation.cmp(&b.conversation));
    assert_eq!(counts, expected);
    assert_eq!(
        store.hidden_inbound().expect("hidden"),
        HiddenInbound { unread: 1, kept: 0 }
    );
}

/// One unread row by id, while it is unread. Mutation: the id ignored.
#[test]
fn an_unread_row_is_read_by_its_id_while_it_is_unread() {
    let (_dir, mut store) = store();
    let first = store
        .commit_unread_inbound(&direct(1, P, 1, InboundKind::Text))
        .expect("committed");
    let second = store
        .commit_unread_inbound(&direct(2, P, 2, InboundKind::Card))
        .expect("committed");
    let row = store
        .unread_inbound_row(second)
        .expect("read")
        .expect("there");
    assert_eq!((number(&row.payload), row.kind), (2, InboundKind::Card));
    store.mark_read(first, 3).expect("read");
    assert!(store.unread_inbound_row(first).expect("read").is_none());
    assert!(
        store
            .unread_inbound_row(RowId::from_stored(999))
            .expect("read")
            .is_none()
    );
}
