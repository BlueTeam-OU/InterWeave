// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The open conversation's window (design/2026-10-27-start-window.md
//! sections 4 to 6), driven as a view and the facade would.

use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::Origin;
use interweave_human_core::RowId;
use interweave_transport_api::{EndpointId, TransportIdentity};

use super::{Edge, MarkKind, PageAsk, PageEnd, PageOrder, PageRows, WindowInput};
use crate::model::SESSION_ITEM_CAP;
use crate::model::summaries::{NewestRow, StoredSummary};
use crate::model::{ConversationKey, ListedInbound, Table, UiModel};
use crate::preview::cut;

const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
const Q: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";

fn peer() -> TransportIdentity {
    TransportIdentity::parse(P).expect("a PeerId")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("an endpoint")
}

fn conversation() -> ConversationKey {
    ConversationKey::Direct {
        peer: peer(),
        endpoint: Some(human()),
    }
}

/// A windowed model whose one conversation the store says holds
/// `unread` unread rows, the newest at time 1,000.
fn started(unread: u64) -> UiModel {
    let mut model = UiModel::new();
    model.started(
        Vec::new(),
        vec![StoredSummary {
            conversation: conversation(),
            unread,
            newest: Some(NewestRow {
                table: Table::Unread,
                row: RowId::from_stored(1_000),
                received_at: 1_000,
                preview: cut("newest"),
                card: false,
            }),
        }],
    );
    model
}

fn row(n: i64) -> ListedInbound {
    ListedInbound {
        row: RowId::from_stored(n),
        origin: Origin::Direct {
            peer: peer(),
            endpoint: human(),
        },
        envelope: HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: format!("{n:032x}"),
            text: format!("row {n}"),
            reply_to: None,
            sent_at_ms: None,
            from_endpoint: None,
        },
        received_at: u64::try_from(n).expect("small"),
    }
}

/// The page for `ask`: its unread rows numbered `rows`, and whether more
/// remain.
fn page(ask: &PageAsk, rows: &[i64], end: PageEnd) -> PageRows {
    PageRows {
        ask: ask.clone(),
        unread: rows.iter().copied().map(row).collect(),
        kept: Vec::new(),
        proposals: Vec::new(),
        end,
    }
}

fn reached(model: &mut UiModel) {
    model.window_input(WindowInput::Reached {
        conversation: conversation(),
        edge: Edge::Top,
    });
}

fn left(model: &mut UiModel) {
    model.window_input(WindowInput::Left {
        conversation: conversation(),
        edge: Edge::Top,
    });
}

fn one_ask(model: &mut UiModel) -> PageAsk {
    let mut asks = model.take_page_asks();
    assert_eq!(asks.len(), 1, "{asks:?}");
    asks.remove(0)
}

fn earlier_unread(model: &UiModel) -> Option<u64> {
    model
        .window_marks(&conversation())
        .into_iter()
        .find_map(|m| match m.kind {
            MarkKind::EarlierUnread { count, .. } => Some(count),
            _ => None,
        })
}

/// Opening a conversation asks one page, at its newest row, newest first
/// and not shortened (design section 4: the opening is not covered by the
/// limit row); while it is on its way the top shows the progress row.
/// Mutation: the opening asking below a bound.
#[test]
fn opening_asks_one_page_at_the_newest() {
    let mut model = started(5);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    assert_eq!(ask.conversation, conversation());
    assert_eq!((ask.older_than, ask.newer_than), (None, None));
    assert_eq!(ask.order, PageOrder::NewestFirst);
    assert_eq!(ask.unread_room, None);
    assert!(
        model
            .window_marks(&conversation())
            .iter()
            .any(|m| m.kind == MarkKind::Loading(Edge::Top))
    );
}

/// A page's rows are held, and the line counts the unread rows the store
/// holds that are not loaded -- never one that is (section 6); with every
/// unread row loaded there is no line. Mutation: the line counting the
/// store's whole count.
#[test]
fn the_line_counts_the_unread_rows_not_loaded() {
    let mut model = started(5);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[999, 1_000], PageEnd::More));
    assert_eq!(model.len(), 2, "both rows held");
    assert_eq!(earlier_unread(&model), Some(3));

    let mut model = started(2);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[999, 1_000], PageEnd::More));
    assert_eq!(earlier_unread(&model), None, "nothing left to count");
}

/// The top edge asks its next page below the oldest loaded row, once each
/// time it is reached: reached again before it was left asks nothing,
/// left and reached again asks the next (section 5). Mutation: the edge
/// re-armed by the page's arrival.
#[test]
fn a_reach_asks_once_until_the_edge_is_left() {
    let mut model = started(10);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[990, 1_000], PageEnd::More));

    reached(&mut model);
    let next = one_ask(&mut model);
    assert_eq!(
        next.older_than.map(|m| m.row),
        Some(RowId::from_stored(990))
    );
    assert_eq!(next.order, PageOrder::NewestFirst);
    model.page_arrived(page(&next, &[980, 985], PageEnd::More));

    reached(&mut model);
    assert!(model.take_page_asks().is_empty(), "still at the edge");
    left(&mut model);
    reached(&mut model);
    let after = one_ask(&mut model);
    assert_eq!(
        after.older_than.map(|m| m.row),
        Some(RowId::from_stored(980))
    );
}

/// No page is asked while one is on its way. Mutation: a reach asking
/// whatever the edge's state.
#[test]
fn no_second_ask_while_one_is_on_its_way() {
    let mut model = started(10);
    model.window_input(WindowInput::Opened(conversation()));
    let _opening = one_ask(&mut model);
    reached(&mut model);
    left(&mut model);
    reached(&mut model);
    assert!(model.take_page_asks().is_empty());
}

/// A page that could not be read shows the failure row; reaching the edge
/// again does not retry, Try again does (section 5). Mutation: a reach
/// clearing the failure.
#[test]
fn a_failure_is_retried_by_try_again_alone() {
    let mut model = started(10);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_failed(&ask);
    assert!(
        model
            .window_marks(&conversation())
            .iter()
            .any(|m| m.kind == MarkKind::Failed(Edge::Top))
    );
    left(&mut model);
    reached(&mut model);
    assert!(model.take_page_asks().is_empty(), "no retry by reaching");
    model.window_input(WindowInput::TryAgain {
        conversation: conversation(),
        edge: Edge::Top,
    });
    assert_eq!(
        one_ask(&mut model).older_than,
        None,
        "the opening's page again"
    );
}

/// When the store holds nothing older, nothing shows at the top and
/// nothing is asked: no marker of a beginning (section 5). Mutation: a
/// reach asking past an exhausted edge.
#[test]
fn an_exhausted_top_shows_and_asks_nothing() {
    let mut model = started(2);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[999, 1_000], PageEnd::Exhausted));
    assert!(model.window_marks(&conversation()).is_empty());
    reached(&mut model);
    assert!(model.take_page_asks().is_empty());
}

/// A page that comes back for an opening that is over -- the conversation
/// closed and opened again -- adds nothing; the new opening's does.
/// Mutation: the page's opening not checked.
#[test]
fn a_page_for_an_earlier_opening_adds_nothing() {
    let mut model = started(10);
    model.window_input(WindowInput::Opened(conversation()));
    let first = one_ask(&mut model);
    model.window_input(WindowInput::Closed(conversation()));
    model.window_input(WindowInput::Opened(conversation()));
    let second = one_ask(&mut model);
    assert_ne!(first.seq, second.seq);
    model.page_arrived(page(&first, &[999, 1_000], PageEnd::More));
    assert!(model.is_empty(), "the old opening's rows are not held");
    model.page_arrived(page(&second, &[999, 1_000], PageEnd::More));
    assert_eq!(model.len(), 2);
}

/// The listed mode has no window: every row is held there. Mutation: the
/// window running in the listed mode.
#[test]
fn the_listed_mode_asks_no_page() {
    let mut model = UiModel::new();
    model.window_input(WindowInput::Opened(conversation()));
    assert!(model.take_page_asks().is_empty());
}

/// `count` rows of another conversation, received and read: session-only
/// rows, which the session bound counts. Their rows are numbered from
/// 100,000 on.
fn read_elsewhere(model: &mut UiModel, count: usize) {
    use interweave_human_client_api::Received;
    for n in 0..count {
        let n = 100_000 + i64::try_from(n).expect("small");
        let listed = row(n);
        model.received(Received {
            row: listed.row,
            origin: Origin::Direct {
                peer: TransportIdentity::parse(Q).expect("a PeerId"),
                endpoint: human(),
            },
            envelope: listed.envelope,
            received_at: 10,
        });
        model.read(listed.row);
    }
}

/// An opened conversation whose first page held `held` unread rows of
/// `unread`, with `session` session-only rows elsewhere.
fn opened_near_the_bound(unread: u64, held: &[i64], session: usize) -> UiModel {
    let mut model = started(unread);
    read_elsewhere(&mut model, session);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, held, PageEnd::More));
    model
}

/// Near the session bound a page is shortened to the room left: the
/// session-only rows' room, less the unread rows held, which become
/// session-only when read (section 7.1, U4). Mutation: the ask carrying
/// no room.
#[test]
fn a_page_near_the_bound_is_shortened_to_the_room() {
    let mut model = opened_near_the_bound(50, &[999, 1_000], SESSION_ITEM_CAP - 5);
    reached(&mut model);
    assert_eq!(one_ask(&mut model).unread_room, Some(3));
}

/// At the bound with unread rows beyond the edge, the limit row shows and
/// nothing is asked; Show more messages asks the page, not shortened, and
/// from then until the conversation is closed paging is automatic; opened
/// again, it asks again. Mutations: the limit ignored; Show more not
/// latched for the opening.
#[test]
fn at_the_bound_the_limit_row_asks_first_and_show_more_holds_until_closed() {
    let mut model = opened_near_the_bound(50, &[999, 1_000], SESSION_ITEM_CAP - 2);
    reached(&mut model);
    assert!(
        model.take_page_asks().is_empty(),
        "nothing fetched at the limit"
    );
    assert!(
        model
            .window_marks(&conversation())
            .iter()
            .any(|m| m.kind == MarkKind::AtLimit(Edge::Top))
    );
    model.window_input(WindowInput::ShowAnyway {
        conversation: conversation(),
        edge: Edge::Top,
    });
    let chosen = one_ask(&mut model);
    assert_eq!(chosen.unread_room, None, "the person chose the drop");
    model.page_arrived(page(&chosen, &[980, 985], PageEnd::More));
    left(&mut model);
    reached(&mut model);
    assert_eq!(one_ask(&mut model).unread_room, None, "still chosen");

    model.window_input(WindowInput::Closed(conversation()));
    model.window_input(WindowInput::Opened(conversation()));
    let opening = one_ask(&mut model);
    model.page_arrived(page(&opening, &[999, 1_000], PageEnd::More));
    reached(&mut model);
    assert!(
        model.take_page_asks().is_empty(),
        "asked again after reopening"
    );
}

/// At the bound with nothing unread beyond the edge, a page makes no row
/// session-only, so it is asked as below the bound. Mutation: the limit
/// row whatever the line.
#[test]
fn with_nothing_unread_beyond_a_full_session_still_pages() {
    let mut model = opened_near_the_bound(2, &[999, 1_000], SESSION_ITEM_CAP);
    reached(&mut model);
    assert_eq!(one_ask(&mut model).unread_room, Some(0));
}

/// A page that stopped for its room shows the limit row at the edge.
/// Mutation: `RoomUsed` taken as `More`.
#[test]
fn a_page_stopped_by_its_room_shows_the_limit_row() {
    let mut model = opened_near_the_bound(50, &[999, 1_000], SESSION_ITEM_CAP - 5);
    reached(&mut model);
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[995, 996, 997], PageEnd::RoomUsed));
    assert!(
        model
            .window_marks(&conversation())
            .iter()
            .any(|m| m.kind == MarkKind::AtLimit(Edge::Top))
    );
}

/// The room is the smaller of the session's and the read proposals'
/// (architect-cto, 01a180cc): with read proposals near their cap, a page
/// may bring no more unread rows than they have room for. Mutation: the
/// room ignoring the read proposals.
#[test]
fn the_room_is_the_smaller_of_the_two_rooms() {
    use super::super::cards::READ_PROPOSAL_CAP;
    let mut model = opened_near_the_bound(50, &[999, 1_000], 0);
    for n in 0..READ_PROPOSAL_CAP - 4 {
        let row = RowId::from_stored(200_000 + i64::try_from(n).expect("small"));
        model.card_received(card(row, Q));
        model.read(row);
    }
    reached(&mut model);
    assert_eq!(one_ask(&mut model).unread_room, Some(4));
}

fn card(row: RowId, p: &str) -> super::super::cards::Proposal {
    use super::super::cards::{KeyView, Proposal, ProposalContext, ValueChanges};
    use interweave_human_chat_protocol::Card;
    Proposal {
        row,
        peer: TransportIdentity::parse(p).expect("a PeerId"),
        endpoint: human(),
        card: Card {
            app_message_id: "0".repeat(32),
            name: "Ada".to_owned(),
            surname: None,
            photo: None,
            identity_key: [1; 32],
            sent_at_ms: None,
            from_endpoint: None,
        },
        received_at: 1,
        context: ProposalContext {
            updates: None,
            shares_node: Vec::new(),
            same_name: Vec::new(),
            key_matches: Vec::new(),
            key: KeyView::New,
            fingerprint: "0123456789abcdef".to_owned(),
            changes_nothing: false,
            values: ValueChanges::default(),
        },
    }
}

/// The open conversation's rows, read: rows 1 to `count`, each its own
/// item, session-only once read.
fn read_here(model: &mut UiModel, count: i64) -> Vec<crate::model::ItemKey> {
    let mut asks = model.take_page_asks();
    let ask = asks.pop().expect("an ask");
    model.page_arrived(page(
        &ask,
        &(1..=count).collect::<Vec<_>>(),
        PageEnd::Exhausted,
    ));
    for n in 1..=count {
        model.read(RowId::from_stored(n));
    }
    model
        .messages(&conversation())
        .into_iter()
        .map(|m| m.key)
        .collect()
}

/// The item the person moved focus to is never dropped by the session
/// bound: the next in the order goes instead (section 7.1). Mutation: the
/// bound ignoring focus.
#[test]
fn the_focused_row_is_never_dropped_the_next_one_is() {
    let mut model = started(0);
    model.window_input(WindowInput::Opened(conversation()));
    let keys = read_here(&mut model, 3);
    model.window_input(WindowInput::Place {
        conversation: conversation(),
        focused: Some(keys[0]),
    });
    read_elsewhere(&mut model, SESSION_ITEM_CAP - 2);
    let held: Vec<_> = model
        .messages(&conversation())
        .into_iter()
        .map(|m| m.key)
        .collect();
    assert_eq!(
        held,
        [keys[0], keys[2]],
        "the focused oldest kept, the next dropped"
    );
}

/// Each run of the open conversation's rows the bound dropped stands as
/// one `ReadNotShown` line with the run's count, above the row now below
/// it: two rows dropped together are one line of 2. Another
/// conversation's drops say nothing. Mutation: a line for each row.
#[test]
fn each_run_of_dropped_rows_has_one_line_with_its_count() {
    let mut model = started(0);
    model.window_input(WindowInput::Opened(conversation()));
    let keys = read_here(&mut model, 4);
    read_elsewhere(&mut model, SESSION_ITEM_CAP - 2);
    let runs: Vec<_> = model
        .window_marks(&conversation())
        .into_iter()
        .filter_map(|m| match m.kind {
            MarkKind::ReadNotShown { count } => Some((m.above, count)),
            _ => None,
        })
        .collect();
    assert_eq!(runs, [(Some(keys[2]), 2)]);
}
