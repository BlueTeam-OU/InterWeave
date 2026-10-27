// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The start window's reads (`design/2026-10-27-start-window.md`, j68
//! P2-1): a conversation's rows a page at a time, newest first, every one
//! reachable and none twice; the badge's counts; the hidden rows.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use interweave_human_store::{
    AppMessageId, ConversationUnread, HiddenInbound, HumanStore, InboundKind, InboundOrigin,
    NewInbound, Order, PageLimits, PageRange, RowId, StoreConversation, StoreOptions,
    StoredInbound, WindowCursor, WindowLimits,
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
/// another conversation, nor any row this build cannot show. At one
/// instant a kept row orders before an unread one. Mutations: the bound
/// `<` read as `<=` (a row twice); the kept table not read; the
/// cross-table bounds swapped.
#[test]
fn a_conversation_is_paged_newest_first_with_every_row_once() {
    let (_dir, mut store) = store();
    // Unread 1..=7 at times 10..=70; 4 is read and kept; 8 at the same
    // instant as 7 in the same table; 9 cannot be shown; 100 is Q's.
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
    // 11 at 40 beside the kept 4: one unread and one kept row at one
    // instant, so a cursor falls between the tables (#489 review F4).
    store
        .commit_unread_inbound(&direct(11, P, 40, InboundKind::Text))
        .expect("the same instant as the kept row");
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
            [8, 7, 6, 5, 4, 11, 3, 2, 1],
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

/// The badge's counts (STATE.md, A 2026-10-27 (ii)): a conversation's
/// unread rows it shows, one per row, a channel being one conversation
/// whoever published on it, so one id published by two peers on it is two
/// items; a row this build cannot show is not counted but counted apart,
/// and a kept row is not unread. Mutation: `COUNT(DISTINCT
/// app_message_id)` for the row count.
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
    // One message id published by two peers on the channel: two items,
    // as the conversation shows them.
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
            unread: 3,
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

/// The fixture the range tests share: in P's conversation, unread rows
/// 1..=6 at times 10..=60, then row 3 read and kept, and unread row 7 at
/// 30 beside it, so the kept row shares an instant with an unread one.
fn ranged() -> (tempfile::TempDir, HumanStore, StoreConversation) {
    let (dir, mut store) = store();
    for n in 1..=6 {
        store
            .commit_unread_inbound(&direct(n, P, u64::from(n) * 10, InboundKind::Text))
            .expect("committed");
    }
    store
        .commit_unread_inbound(&direct(7, P, 30, InboundKind::Text))
        .expect("the same instant");
    let three = store
        .unread_inbound()
        .expect("read")
        .into_iter()
        .find(|r| number(&r.payload) == 3)
        .expect("three")
        .row_id;
    let read = store.mark_read(three, 1).expect("read");
    store.keep(&read, 2).expect("kept");
    let conversation = StoreConversation::Direct {
        peer: peer(P),
        endpoint: human(),
    };
    (dir, store, conversation)
}

/// Every page of `range` in `order`, by `records`, as numbers.
fn walk_range(
    store: &HumanStore,
    conversation: &StoreConversation,
    range: PageRange,
    order: Order,
    records: usize,
) -> Vec<u32> {
    let limits = WindowLimits {
        page: PageLimits::new(records, 1024 * 1024).expect("limits"),
        unread: None,
    };
    let mut out = Vec::new();
    let mut range = range;
    for _ in 0..64 {
        let page = store
            .conversation_range(conversation, range, order, limits)
            .expect("a page");
        out.extend(page.items.iter().map(|r| number(&r.payload)));
        match (page.next, order) {
            (None, _) => return out,
            (Some(next), Order::NewestFirst) => range.older_than = Some(next),
            (Some(next), Order::OldestFirst) => range.newer_than = Some(next),
        }
    }
    panic!("the walk does not end: {out:?}");
}

/// The row numbered `n` as a page gives it.
fn row(store: &HumanStore, conversation: &StoreConversation, n: u32) -> StoredInbound {
    store
        .conversation_page(conversation, None, PageLimits::default())
        .expect("a page")
        .items
        .into_iter()
        .find(|r| number(&r.payload) == n)
        .expect("the row")
}

/// Oldest first is the newest-first order reversed, every row once, the
/// kept row before the unread one at their shared instant read the other
/// way. Mutation: the order not applied to the merge.
#[test]
fn a_conversation_reads_oldest_first_as_the_reverse_of_newest_first() {
    let (_dir, store, conversation) = ranged();
    for records in [1, 2, 64] {
        let newest = walk_range(
            &store,
            &conversation,
            PageRange::default(),
            Order::NewestFirst,
            records,
        );
        let mut oldest = walk_range(
            &store,
            &conversation,
            PageRange::default(),
            Order::OldestFirst,
            records,
        );
        assert_eq!(newest, [6, 5, 4, 3, 7, 2, 1], "pages of {records}");
        oldest.reverse();
        assert_eq!(oldest, newest, "pages of {records}");
    }
}

/// A gap between two held rows (design §5.1) is read from either side
/// and gives exactly the rows between them, never a row beyond either
/// bound, at a shared instant across the tables included. Mutations: the
/// newer bound dropped from the query; the cross-table bound ignored.
#[test]
fn a_gap_is_read_from_either_side_and_holds_only_its_rows() {
    let (_dir, store, conversation) = ranged();
    let above = WindowCursor::of(&row(&store, &conversation, 5));
    let below = WindowCursor::of(&row(&store, &conversation, 2));
    let gap = PageRange {
        older_than: Some(above),
        newer_than: Some(below),
    };
    for records in [1, 2, 64] {
        assert_eq!(
            walk_range(&store, &conversation, gap, Order::NewestFirst, records),
            [4, 3, 7],
            "from above, pages of {records}"
        );
        assert_eq!(
            walk_range(&store, &conversation, gap, Order::OldestFirst, records),
            [7, 3, 4],
            "from below, pages of {records}"
        );
    }
    // A bound on a kept row: its unread neighbour at the same instant is
    // below it, newest first.
    let kept = WindowCursor::of(&row(&store, &conversation, 3));
    let under_kept = PageRange {
        older_than: Some(kept),
        newer_than: None,
    };
    assert_eq!(
        walk_range(&store, &conversation, under_kept, Order::NewestFirst, 1),
        [7, 2, 1]
    );
}

/// A page shortened to the session's room (design §7.1) stops before its
/// room+1th unread row, kept rows taking no room; with no room left and
/// an unread row next it is empty, and its cursor says more remain so the
/// next page loses nothing. Mutations: kept rows counted against the
/// room; the empty page's cursor dropped.
#[test]
fn a_page_keeps_to_its_unread_room() {
    let (_dir, store, conversation) = ranged();
    let room = |unread: usize| WindowLimits {
        page: PageLimits::new(64, 1024 * 1024).expect("limits"),
        unread: Some(unread),
    };
    let page = store
        .conversation_range(
            &conversation,
            PageRange::default(),
            Order::NewestFirst,
            room(3),
        )
        .expect("a page");
    let numbers: Vec<u32> = page.items.iter().map(|r| number(&r.payload)).collect();
    assert_eq!(
        numbers,
        [6, 5, 4, 3],
        "three unread rows and the kept one beside them, not the fourth"
    );
    let next = page.next.expect("more remain");

    let empty = store
        .conversation_range(
            &conversation,
            PageRange {
                older_than: Some(next),
                newer_than: None,
            },
            Order::NewestFirst,
            room(0),
        )
        .expect("a page");
    assert!(empty.items.is_empty(), "no room, and the next row unread");
    let again = empty.next.expect("the cursor stays, so nothing is lost");
    let rest = store
        .conversation_range(
            &conversation,
            PageRange {
                older_than: Some(again),
                newer_than: None,
            },
            Order::NewestFirst,
            room(64),
        )
        .expect("a page");
    let numbers: Vec<u32> = rest.items.iter().map(|r| number(&r.payload)).collect();
    assert_eq!(numbers, [7, 2, 1]);
}

/// A first page opened with no unread room left, in either order: it
/// takes no unread row, and its cursor says more remain -- never `None`,
/// which reads as a finished conversation -- so the pages from it hold
/// every row once, in order (#492 review F1). Mutation: the first page's
/// cursor taken from its absent bound.
#[test]
fn a_first_page_with_no_room_still_says_more_remain() {
    let (_dir, store, conversation) = ranged();
    let limits = |unread: usize| WindowLimits {
        page: PageLimits::new(64, 1024 * 1024).expect("limits"),
        unread: Some(unread),
    };
    for (order, all) in [
        (Order::NewestFirst, [6, 5, 4, 3, 7, 2, 1]),
        (Order::OldestFirst, [1, 2, 7, 3, 4, 5, 6]),
    ] {
        let first = store
            .conversation_range(&conversation, PageRange::default(), order, limits(0))
            .expect("a page");
        let next = first.next.expect("more remain, with no room taken");
        let range = match order {
            Order::NewestFirst => PageRange {
                older_than: Some(next),
                newer_than: None,
            },
            Order::OldestFirst => PageRange {
                older_than: None,
                newer_than: Some(next),
            },
        };
        let rest = store
            .conversation_range(&conversation, range, order, limits(64))
            .expect("a page");
        let numbers: Vec<u32> = first
            .items
            .iter()
            .chain(&rest.items)
            .map(|r| number(&r.payload))
            .collect();
        assert_eq!(numbers, all, "{order:?}");
        assert!(rest.next.is_none(), "{order:?}: the end");
    }
}

/// The conversations the Chats list holds from the store: every one with
/// a row this build shows, unread or kept -- a kept-only one included --
/// and a channel once whoever published; one holding only rows this build
/// cannot show is not among them. Mutation: the kept table not read.
#[test]
fn the_shown_conversations_include_a_kept_only_one() {
    let (_dir, mut store) = store();
    store
        .commit_unread_inbound(&direct(1, P, 1, InboundKind::Text))
        .expect("P's unread");
    let kept = store
        .commit_unread_inbound(&direct(2, Q, 2, InboundKind::Card))
        .expect("Q's, to keep");
    let read = store.mark_read(kept, 3).expect("read");
    store.keep(&read, 4).expect("kept");
    let room = ChannelId::parse("room").expect("a channel");
    store
        .commit_unread_inbound(&on_channel(3, P, &room))
        .expect("on the channel");
    store
        .commit_unread_inbound(&on_channel(4, Q, &room))
        .expect("on the channel, from Q");
    store
        .commit_unread_inbound(&NewInbound {
            origin: InboundOrigin {
                peer: peer(P),
                endpoint: Some(EndpointId::parse("agent").expect("an endpoint")),
                channel: None,
            },
            ..direct(5, P, 5, InboundKind::Other)
        })
        .expect("a row this build cannot show");
    let mut shown = store.shown_conversations().expect("conversations");
    shown.sort();
    let mut expected = vec![
        StoreConversation::Direct {
            peer: peer(P),
            endpoint: human(),
        },
        StoreConversation::Direct {
            peer: peer(Q),
            endpoint: human(),
        },
        StoreConversation::Channel(room),
    ];
    expected.sort();
    assert_eq!(shown, expected);
}
