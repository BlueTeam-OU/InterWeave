// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The open conversation's window (design/2026-10-27-start-window.md
//! sections 4 to 6), driven as a view and the facade would.

use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::Origin;
use interweave_human_core::RowId;
use interweave_transport_api::{EndpointId, TransportIdentity};

use super::{
    Anchor, Edge, MarkKind, MarkPlace, PageAsk, PageEnd, PageOrder, PageRows, WindowEvent,
    WindowInput,
};
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
/// `unread` unread rows, the newest at time 1,000, and whose counts hold
/// every row these tests page: the start's mark is above all of them.
fn started(unread: u64) -> UiModel {
    started_through(unread, i64::MAX)
}

/// [`started`], the start's mark at row `mark`.
fn started_through(unread: u64, mark: i64) -> UiModel {
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
        Some(RowId::from_stored(mark)),
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
            MarkKind::EarlierUnread {
                edge: Edge::Top,
                count,
            } => Some(count),
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

/// `count` rows of another conversation, read while it was open, then
/// closed: session-only rows, which the session bound counts. Their rows
/// are numbered from 100,000 on.
fn read_elsewhere(model: &mut UiModel, count: usize) {
    let elsewhere = ConversationKey::Direct {
        peer: TransportIdentity::parse(Q).expect("a PeerId"),
        endpoint: Some(human()),
    };
    model.window_input(WindowInput::Opened(elsewhere.clone()));
    let ask = one_ask(model);
    let rows: Vec<ListedInbound> = (0..count)
        .map(|n| {
            let mut listed = row(100_000 + i64::try_from(n).expect("small"));
            listed.origin = Origin::Direct {
                peer: TransportIdentity::parse(Q).expect("a PeerId"),
                endpoint: human(),
            };
            listed
        })
        .collect();
    let numbers: Vec<RowId> = rows.iter().map(|r| r.row).collect();
    model.page_arrived(PageRows {
        ask,
        unread: rows,
        kept: Vec::new(),
        proposals: Vec::new(),
        end: PageEnd::Exhausted,
    });
    for row in numbers {
        model.read(row);
    }
    model.window_input(WindowInput::Closed(elsewhere));
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

/// A page of the open conversation's rows `rows`, read: each its own
/// item, session-only once read. The keys of the conversation's items.
fn read_here(
    model: &mut UiModel,
    rows: std::ops::RangeInclusive<i64>,
) -> Vec<crate::model::ItemKey> {
    let ask = model.take_page_asks().pop().unwrap_or_else(|| PageAsk {
        conversation: conversation(),
        edge: Edge::Top,
        // A top page's ask carries the top it was read from; one that does
        // not is stale.
        older_than: model.windows.open.as_ref().and_then(|w| w.top),
        newer_than: None,
        order: PageOrder::NewestFirst,
        unread_room: None,
        seq: model.windows.open.as_ref().map_or(0, |w| w.seq),
    });
    let numbers: Vec<i64> = rows.collect();
    model.page_arrived(page(&ask, &numbers, PageEnd::More));
    for n in numbers {
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
    let keys = read_here(&mut model, 1..=3);
    model.window_input(WindowInput::Place {
        conversation: conversation(),
        focused: Some(keys[0]),
        in_view: Vec::new(),
    });
    let cap = i64::try_from(SESSION_ITEM_CAP).expect("small");
    let held = read_here(&mut model, 10..=10 + cap - 3);
    assert_eq!(
        held[..2],
        [keys[0], keys[2]],
        "the focused oldest kept, the next dropped"
    );
}

/// Each run of the open conversation's rows the bound dropped stands as
/// one `ReadNotShown` line with the run's count, above the row now below
/// it: two rows dropped together are one line of 2. Mutation: a line for
/// each row.
#[test]
fn each_run_of_dropped_rows_has_one_line_with_its_count() {
    let mut model = started(0);
    model.window_input(WindowInput::Opened(conversation()));
    let keys = read_here(&mut model, 1..=4);
    let cap = i64::try_from(SESSION_ITEM_CAP).expect("small");
    read_here(&mut model, 10..=10 + cap - 3);
    let runs: Vec<_> = model
        .window_marks(&conversation())
        .into_iter()
        .filter_map(|m| match m.kind {
            MarkKind::ReadNotShown { count } => Some((m.place, count)),
            _ => None,
        })
        .collect();
    assert_eq!(runs, [(MarkPlace::Above(Anchor::Item(keys[2])), 2)]);
}

/// Closed, a conversation holds none of its stored rows in the windowed
/// mode -- they stay in the store -- but keeps one's own pending row; its
/// next opening loads the stored rows again (section 3). Mutation: the
/// rows released rather than let go, which refuses them on the reload.
#[test]
fn a_closed_conversation_holds_none_of_its_stored_rows_and_reloads_them() {
    use interweave_human_client_api::Destination;
    let mut model = started(2);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[999, 1_000], PageEnd::Exhausted));
    model.sent(
        RowId::from_stored(7),
        &Destination::Direct {
            peer: peer(),
            endpoint: Some(human()),
        },
        row(7).envelope,
        2_000,
    );
    assert_eq!(model.len(), 3);
    model.window_input(WindowInput::Closed(conversation()));
    assert_eq!(model.len(), 1, "only the pending row");

    model.window_input(WindowInput::Opened(conversation()));
    let again = one_ask(&mut model);
    model.page_arrived(page(&again, &[999, 1_000], PageEnd::Exhausted));
    assert_eq!(model.len(), 3, "loaded again");
}

/// A message arriving in a closed conversation is counted in its summary
/// and not held. Mutation: the arrival held whatever the window.
#[test]
fn an_arrival_in_a_closed_conversation_is_counted_not_held() {
    use interweave_human_client_api::Received;
    let mut model = started_through(1, 1_000);
    let listed = row(2_000);
    model.received(Received {
        row: listed.row,
        origin: listed.origin,
        envelope: listed.envelope,
        received_at: 2_000,
    });
    assert!(model.is_empty());
    assert_eq!(model.conversations()[0].unread, 2);
}

/// Opening another conversation closes the one open before: its stored
/// rows are let go. Mutation: the earlier window replaced without being
/// closed.
#[test]
fn opening_another_conversation_closes_the_first() {
    let mut model = started(2);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[999, 1_000], PageEnd::Exhausted));
    model.window_input(WindowInput::Opened(ConversationKey::Direct {
        peer: TransportIdentity::parse(Q).expect("a PeerId"),
        endpoint: Some(human()),
    }));
    assert!(model.is_empty());
}

/// An opened conversation whose first page brought rows `1..=n`, all
/// unread or all kept, with focus on row `focused`.
fn opened_with(n: i64, focused: i64, table: Table, end: PageEnd) -> UiModel {
    let unread = if table == Table::Unread { n } else { 0 };
    let mut model = started(u64::try_from(unread).expect("small"));
    model.window_input(WindowInput::Opened(conversation()));
    place(&mut model, focused, table);
    let ask = one_ask(&mut model);
    model.page_arrived(rows_of(&ask, &(1..=n).collect::<Vec<_>>(), table, end));
    model
}

fn place(model: &mut UiModel, focused: i64, table: Table) {
    model.window_input(WindowInput::Place {
        conversation: conversation(),
        focused: Some(crate::model::ItemKey(table, RowId::from_stored(focused))),
        in_view: Vec::new(),
    });
}

/// The page for `ask` of rows `rows`, in `table`.
fn rows_of(ask: &PageAsk, rows: &[i64], table: Table, end: PageEnd) -> PageRows {
    let mut page = page(ask, rows, end);
    if table == Table::Kept {
        page.kept = std::mem::take(&mut page.unread);
    }
    page
}

fn holds(model: &UiModel, n: i64, table: Table) -> bool {
    model.by_row.contains_key(&(table, RowId::from_stored(n)))
}

fn gap_lines(model: &UiModel) -> Vec<u64> {
    model
        .window_marks(&conversation())
        .into_iter()
        .filter_map(|m| match m.kind {
            MarkKind::EarlierUnread {
                edge: Edge::GapLower(_),
                count,
            } => Some(count),
            _ => None,
        })
        .collect()
}

/// Past the held-row ceiling the rows farthest from the person's focus
/// are let go, never the newest row nor the focused one, and rows let go
/// between held ones leave a gap whose own line counts its unread rows --
/// which the top line then does not, so the lines and the held rows agree
/// with the badge (section 5.1). Mutations: the newest row let go; a gap's
/// unread counted at the top too.
#[test]
fn past_the_ceiling_far_rows_leave_a_gap_never_the_newest_or_the_focused() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let model = opened_with(n, 10, Table::Unread, PageEnd::More);
    assert!(holds(&model, n, Table::Unread), "the newest row stays");
    assert!(holds(&model, 10, Table::Unread), "the focused row stays");
    for gone in n - 4..n {
        assert!(!holds(&model, gone, Table::Unread), "row {gone} let go");
    }
    assert_eq!(gap_lines(&model), [4]);
    assert_eq!(
        earlier_unread(&model),
        None,
        "the top line counts none of them"
    );
}

/// A gap reached from above fetches its rows oldest first, from below
/// newest first, both between the held rows on either side; its rows come
/// back where they were, none refused, and a gap loaded whole is gone.
/// Mutations: the rows let go released, which refuses them; the two
/// sides' orders swapped.
#[test]
fn a_gap_loads_its_rows_again_from_either_side() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let mut model = opened_with(n, 10, Table::Kept, PageEnd::Exhausted);
    let id = model
        .windows
        .open
        .as_ref()
        .and_then(|w| w.gaps.keys().next().copied())
        .expect("a gap");
    model.window_input(WindowInput::Reached {
        conversation: conversation(),
        edge: Edge::GapUpper(id),
    });
    let upper = one_ask(&mut model);
    assert_eq!(upper.order, PageOrder::OldestFirst);
    assert_eq!(
        upper.newer_than.map(|m| m.row),
        Some(RowId::from_stored(n - 5))
    );
    assert_eq!(upper.older_than.map(|m| m.row), Some(RowId::from_stored(n)));
    model.page_failed(&upper);

    place(&mut model, n - 5, Table::Kept);
    model.window_input(WindowInput::Reached {
        conversation: conversation(),
        edge: Edge::GapLower(id),
    });
    let lower = one_ask(&mut model);
    assert_eq!(lower.order, PageOrder::NewestFirst);
    model.page_arrived(rows_of(
        &lower,
        &[n - 1, n - 2, n - 3, n - 4],
        Table::Kept,
        PageEnd::Exhausted,
    ));
    for back in n - 4..n {
        assert!(
            holds(&model, back, Table::Kept),
            "row {back} back where it was"
        );
    }
    assert!(
        model
            .windows
            .open
            .as_ref()
            .is_some_and(|w| w.gaps.is_empty()),
        "the gap is gone"
    );
}

/// What is never let go stays even when nothing else is left to let go
/// and the ceiling is passed: the focused item, every item in view and
/// the newest row (section 5.1). Mutations: the focused item, or the items
/// in view, let go.
#[test]
fn the_focused_the_in_view_and_the_newest_are_never_let_go() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 2;
    let mut model = started(0);
    model.window_input(WindowInput::Opened(conversation()));
    model.window_input(WindowInput::Place {
        conversation: conversation(),
        focused: Some(crate::model::ItemKey(Table::Kept, RowId::from_stored(1))),
        in_view: (2..n)
            .map(|r| crate::model::ItemKey(Table::Kept, RowId::from_stored(r)))
            .collect(),
    });
    let ask = one_ask(&mut model);
    model.page_arrived(rows_of(
        &ask,
        &(1..=n).collect::<Vec<_>>(),
        Table::Kept,
        PageEnd::Exhausted,
    ));
    for r in 1..=n {
        assert!(holds(&model, r, Table::Kept), "row {r} stays");
    }
}

/// The summary with its newest row at `row`.
fn newest_at(unread: u64, row: i64) -> Vec<StoredSummary> {
    vec![StoredSummary {
        conversation: conversation(),
        unread,
        newest: Some(NewestRow {
            table: Table::Unread,
            row: RowId::from_stored(row),
            received_at: u64::try_from(row).expect("small"),
            preview: cut("newest"),
            card: false,
        }),
    }]
}

/// Asked again after the facade could not hand rows over, the store's
/// newest row of the open conversation is one its window never held --
/// newer than all it did, and no edge reads newer rows -- so the window
/// opens again at the newest row; a newest row it held -- a card that
/// arrived while it was open among them -- or one older (the newer ones
/// read and gone) leaves the window as it is. Mutations: the re-base
/// leaving the open window alone whatever the store holds; every re-base
/// opening it again; an arrived card not counted as held.
#[test]
fn rows_never_handed_over_open_the_window_again_at_the_newest() {
    let mut model = started(2);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[999, 1_000], PageEnd::Exhausted));

    model.summaries_listed(newest_at(2, 1_000), None);
    assert!(model.take_page_asks().is_empty(), "the newest is held");
    model.summaries_listed(newest_at(1, 999), None);
    assert!(model.take_page_asks().is_empty(), "an older newest");

    // A card that arrived while open is held too.
    let mut arrived = card(RowId::from_stored(1_001), P);
    arrived.received_at = 1_001;
    model.card_arrived(arrived);
    model.summaries_listed(newest_at(3, 1_001), None);
    assert!(model.take_page_asks().is_empty(), "the card is held");

    model.summaries_listed(newest_at(5, 1_003), None);
    let again = one_ask(&mut model);
    assert_eq!((again.older_than, again.newer_than), (None, None));
    assert!(again.seq > ask.seq, "a new opening");
    model.page_arrived(page(&again, &[1_001, 1_002, 1_003], PageEnd::More));
    assert!(model.len() >= 3, "the rows never handed over are held");
}

/// Row `n` as the drain hands it over.
fn handed(model: &mut UiModel, n: i64) {
    let listed = row(n);
    model.received(interweave_human_client_api::Received {
        row: listed.row,
        origin: listed.origin,
        envelope: listed.envelope,
        received_at: listed.received_at,
    });
}

/// Each unread row committed after the start's read counts once, whichever
/// comes first: the drain handing it over, or a page bringing it from the
/// store while the facade still holds it back. Mutations: a page's rows
/// counted by no one; the drain counting a row a page counted first.
#[test]
fn a_row_counts_once_whether_the_drain_or_a_page_brings_it_first() {
    let mut model = started_through(2, 1_000);
    let unread = |model: &UiModel| model.conversations()[0].unread;
    handed(&mut model, 1_002);
    assert_eq!(unread(&model), 3, "handed over while closed");

    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[999, 1_000, 1_002, 1_003], PageEnd::Exhausted));
    assert_eq!(
        unread(&model),
        4,
        "1,003 is new; 1,002 and the start's rows are not"
    );

    // Closed, the conversation holds the row no more: what decides is that
    // the page counted it, not that it is held.
    model.window_input(WindowInput::Closed(conversation()));
    handed(&mut model, 1_003);
    assert_eq!(unread(&model), 4, "the page counted it first");
    handed(&mut model, 1_004);
    assert_eq!(unread(&model), 5);
}

/// With a message sent in the conversation as its newest item, the ceiling
/// still never lets the newest stored row go, and the rows it does let go
/// leave a gap: no edge reads rows newer than the newest one held, so
/// rows let go below it would be out of reach until the conversation is
/// opened again. Mutation: the newest item, not the newest stored item,
/// kept by the ceiling.
#[test]
fn a_sent_message_newest_leaves_the_newest_stored_row_held() {
    use super::HELD_ROW_CAP;
    use interweave_human_client_api::Destination;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let mut model = started(u64::try_from(n).expect("small"));
    model.window_input(WindowInput::Opened(conversation()));
    place(&mut model, 10, Table::Unread);
    model.sent(
        RowId::from_stored(1),
        &Destination::Direct {
            peer: peer(),
            endpoint: Some(human()),
        },
        row(1).envelope,
        u64::MAX,
    );
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &(1..=n).collect::<Vec<_>>(), PageEnd::More));
    assert!(
        holds(&model, n, Table::Unread),
        "the newest stored row stays"
    );
    for gone in n - 4..n {
        assert!(!holds(&model, gone, Table::Unread), "row {gone} let go");
    }
    assert_eq!(
        gap_lines(&model),
        [4],
        "into a gap, read back from either side"
    );
}

/// The rows on a gap's two sides stand next to each other on screen, so
/// both are reached together: one page is asked, the other side's after
/// it arrives, and no row is read twice (section 5.1: no page of a gap
/// while another of the same gap is in flight). Mutation: the guard in
/// `Windows::ask` removed -- two asks over the same rows.
#[test]
fn reaching_both_sides_of_a_gap_asks_one_page() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let mut model = opened_with(n, 10, Table::Kept, PageEnd::Exhausted);
    let id = gap_id(&model);
    place(&mut model, n - 5, Table::Kept);
    for edge in [Edge::GapUpper(id), Edge::GapLower(id)] {
        model.window_input(WindowInput::Reached {
            conversation: conversation(),
            edge,
        });
    }
    let upper = one_ask(&mut model);
    assert_eq!(upper.edge, Edge::GapUpper(id));
    model.page_arrived(rows_of(&upper, &[n - 4, n - 3], Table::Kept, PageEnd::More));
    let lower = one_ask(&mut model);
    assert_eq!(lower.edge, Edge::GapLower(id), "the other side after it");
    assert_eq!(
        lower.newer_than.map(|m| m.row),
        Some(RowId::from_stored(n - 3)),
        "past the rows the first page read"
    );
    model.page_arrived(rows_of(
        &lower,
        &[n - 1, n - 2],
        Table::Kept,
        PageEnd::Exhausted,
    ));
    assert!(model.take_page_asks().is_empty());
    for back in n - 4..n {
        assert!(holds(&model, back, Table::Kept), "row {back} back");
    }
}

/// The same with unread rows at the session bound, where Show more
/// messages asks: the second side's Show more waits for the first page,
/// and the gap's line counts each row out once. Mutation: the guard
/// removed -- two asks, and the line short by the rows read twice.
#[test]
fn show_more_on_both_sides_of_a_gap_counts_each_row_once() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let mut model = opened_with(n, 10, Table::Unread, PageEnd::More);
    assert_eq!(gap_lines(&model), [4]);
    let id = gap_id(&model);
    place(&mut model, n - 5, Table::Unread);
    for edge in [Edge::GapUpper(id), Edge::GapLower(id)] {
        model.window_input(WindowInput::Reached {
            conversation: conversation(),
            edge,
        });
    }
    assert!(model.take_page_asks().is_empty(), "both at the limit");
    for edge in [Edge::GapUpper(id), Edge::GapLower(id)] {
        model.window_input(WindowInput::ShowAnyway {
            conversation: conversation(),
            edge,
        });
    }
    let upper = one_ask(&mut model);
    model.page_arrived(page(&upper, &[n - 4, n - 3], PageEnd::More));
    assert_eq!(gap_lines(&model), [2], "each row counted out once");
    let lower = one_ask(&mut model);
    model.page_arrived(page(&lower, &[n - 1, n - 2], PageEnd::Exhausted));
    assert!(gap_lines(&model).is_empty(), "the gap is gone");
}

fn gap_id(model: &UiModel) -> u64 {
    model
        .windows
        .open
        .as_ref()
        .and_then(|w| w.gaps.keys().next().copied())
        .expect("a gap")
}

/// A top page on its way while arrivals push the ceiling to let go of the
/// oldest rows, moving the top down past them: the page, read from the old
/// top, adds nothing -- its rows would sit above rows let go into no gap,
/// which nothing would read back -- and the top is asked again from where
/// it stands (section 5.1: rows released, never lost). Mutation: the
/// staleness check removed -- the page's rows are held and nothing asks
/// past the rows let go.
#[test]
fn a_top_page_in_flight_when_the_ceiling_moves_the_top_adds_nothing() {
    use super::HELD_ROW_CAP;
    let cap = i64::try_from(HELD_ROW_CAP).expect("small");
    let mut model = started(0);
    model.window_input(WindowInput::Opened(conversation()));
    place(&mut model, 100 + cap - 1, Table::Kept);
    let opening = one_ask(&mut model);
    model.page_arrived(rows_of(
        &opening,
        &(100..100 + cap).collect::<Vec<_>>(),
        Table::Kept,
        PageEnd::More,
    ));
    reached(&mut model);
    let top = one_ask(&mut model);
    assert_eq!(top.older_than.map(|m| m.row), Some(RowId::from_stored(100)));
    for n in 100 + cap..100 + cap + 4 {
        handed(&mut model, n);
    }
    for gone in 100..104 {
        assert!(!holds(&model, gone, Table::Kept), "row {gone} let go");
    }
    model.page_arrived(rows_of(
        &top,
        &(90..100).collect::<Vec<_>>(),
        Table::Kept,
        PageEnd::More,
    ));
    for n in 90..100 {
        assert!(!holds(&model, n, Table::Kept), "row {n} not added");
    }
    let again = one_ask(&mut model);
    assert_eq!(again.edge, Edge::Top);
    assert_eq!(
        again.older_than.map(|m| m.row),
        Some(RowId::from_stored(104)),
        "asked again from the top as it stands, above the rows let go"
    );
}

/// An item can hold a stored row newer than every other held row while
/// it stands far up the conversation: a later copy of an older message is
/// attached to the older message's item. Such an item cannot be let go --
/// its rows would have no side to be read back from -- and the ceiling
/// passes it over for the next farthest, so the held rows stay at the
/// ceiling (section 5.1). Mutation: stopping at the first item that cannot
/// go -- the held rows stay past the ceiling.
#[test]
fn the_ceiling_holds_past_an_item_holding_the_newest_stored_row() {
    use super::HELD_ROW_CAP;
    let cap = i64::try_from(HELD_ROW_CAP).expect("small");
    let mut model = started(0);
    model.window_input(WindowInput::Opened(conversation()));
    place(&mut model, cap + 1, Table::Kept);
    let ask = one_ask(&mut model);
    let mut rows = rows_of(
        &ask,
        &(1..=cap + 1).collect::<Vec<_>>(),
        Table::Kept,
        PageEnd::Exhausted,
    );
    let mut copy = row(1);
    copy.row = RowId::from_stored(cap + 10);
    copy.received_at = u64::try_from(cap + 10).expect("small");
    rows.kept.push(copy);
    model.page_arrived(rows);
    assert!(holds(&model, 1, Table::Kept), "the copy's item cannot go");
    assert!(holds(&model, cap + 10, Table::Kept));
    let held = (1..=cap + 1)
        .chain([cap + 10])
        .filter(|n| holds(&model, *n, Table::Kept))
        .count();
    assert_eq!(held, HELD_ROW_CAP, "the next farthest went instead");
    assert!(!holds(&model, 2, Table::Kept));
}

fn kept(n: i64) -> crate::model::ItemKey {
    crate::model::ItemKey(Table::Kept, RowId::from_stored(n))
}

/// The top edge's marks stand above everything the conversation draws --
/// with nothing drawn yet too, where "above the oldest item" named no
/// item and read as the end (ux-designer's U2). Mutation: the top's
/// marks placed at the oldest item -- `End` here.
#[test]
fn top_marks_stand_at_the_top_with_nothing_drawn() {
    let mut model = started(5);
    model.window_input(WindowInput::Opened(conversation()));
    let marks = model.window_marks(&conversation());
    assert!(!marks.is_empty());
    assert!(marks.iter().all(|m| m.place == MarkPlace::Top), "{marks:?}");
}

/// A gap's line keeps its place when the row below it is read: a read
/// row is no held stored row, but it is still drawn, and the line stands
/// above it as before. Mutation: a place found only through the held
/// rows -- the line falls to the end.
#[test]
fn a_gap_keeps_its_place_when_the_row_below_it_is_read() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let mut model = opened_with(n, 10, Table::Unread, PageEnd::More);
    let line = |model: &UiModel| {
        model
            .window_marks(&conversation())
            .into_iter()
            .find(|m| {
                matches!(
                    m.kind,
                    MarkKind::EarlierUnread {
                        edge: Edge::GapLower(_),
                        ..
                    }
                )
            })
            .map(|m| m.place)
    };
    let newest = MarkPlace::Above(Anchor::Item(crate::model::ItemKey(
        Table::Unread,
        RowId::from_stored(n),
    )));
    assert_eq!(line(&model), Some(newest));
    model.read(RowId::from_stored(n));
    assert_eq!(line(&model), Some(newest), "still above the row read");
}

/// A `ReadNotShown` line stands where its rows stood: hiding the row
/// below it moves it above the next row drawn, never off the screen.
/// Mutation: hidden rows counted as drawn -- the line stays on the
/// hidden row, which no view draws.
#[test]
fn a_read_not_shown_line_stays_when_the_row_below_it_is_hidden() {
    let mut model = started(0);
    model.window_input(WindowInput::Opened(conversation()));
    let keys = read_here(&mut model, 1..=4);
    let cap = i64::try_from(SESSION_ITEM_CAP).expect("small");
    read_here(&mut model, 10..=10 + cap - 3);
    model.hide(keys[2]);
    let runs: Vec<_> = model
        .window_marks(&conversation())
        .into_iter()
        .filter(|m| matches!(m.kind, MarkKind::ReadNotShown { .. }))
        .map(|m| (m.place, m.kind))
        .collect();
    assert_eq!(
        runs,
        [(
            MarkPlace::Above(Anchor::Item(keys[3])),
            MarkKind::ReadNotShown { count: 2 }
        )]
    );
}

/// A held card proposal is a held row like a message: rows released on
/// its two sides are two gaps, each with its own sides, the proposal
/// standing between them where it arrived (ux-designer's U2). Mutation:
/// proposals left out of the held rows that bound a gap -- one gap, and
/// the proposal shown next to rows it is not next to in time.
#[test]
fn a_held_proposal_between_released_rows_splits_them_into_two_gaps() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let mut model = started(0);
    model.window_input(WindowInput::Opened(conversation()));
    place(&mut model, 10, Table::Kept);
    let ask = one_ask(&mut model);
    let mut rows = rows_of(
        &ask,
        &(1..=n).collect::<Vec<_>>(),
        Table::Kept,
        PageEnd::Exhausted,
    );
    // Times spaced apart, so the proposal stands strictly between two rows.
    for r in &mut rows.kept {
        r.received_at *= 2;
    }
    let card_row = RowId::from_stored(50_000);
    let mut proposal = card(card_row, P);
    proposal.received_at = u64::try_from(2 * (n - 3) + 1).expect("small");
    rows.proposals.push(proposal);
    model.page_arrived(rows);
    let edges = model.window_edges(&conversation());
    let at = |edge: fn(u64) -> Edge| -> Vec<MarkPlace> {
        let gaps: Vec<u64> = model
            .windows
            .open
            .as_ref()
            .map(|w| w.gaps.keys().copied().collect())
            .unwrap_or_default();
        gaps.iter()
            .filter_map(|id| edges.iter().find(|(e, _)| *e == edge(*id)).map(|(_, p)| *p))
            .collect()
    };
    let mut upper = at(Edge::GapUpper);
    upper.sort_by_key(|p| format!("{p:?}"));
    assert_eq!(
        upper,
        [
            MarkPlace::Above(Anchor::Card(card_row)),
            MarkPlace::Above(Anchor::Item(kept(n))),
        ],
        "one gap above the proposal, one below it"
    );
}

/// A report that changes what a view draws says so, so the root draws
/// again with nothing asked of the facade -- reaching the top at the
/// session bound shows the limit row and asks nothing -- and the person's
/// place, which a scroll reports on every move, says nothing changed.
/// Mutations: always false (the limit row never drawn); always true (a
/// scroll redrawing on every report).
#[test]
fn a_window_event_says_whether_anything_drawn_changed() {
    let cap = i64::try_from(SESSION_ITEM_CAP).expect("small");
    let mut model = opened_near_the_bound(
        SESSION_ITEM_CAP as u64 + 10,
        &(1..=cap).collect::<Vec<_>>(),
        0,
    );
    assert!(!model.window_event(WindowEvent::Input(WindowInput::Place {
        conversation: conversation(),
        focused: None,
        in_view: Vec::new(),
    })));
    assert!(!model.window_event(WindowEvent::PhotoShown("a".to_owned())));
    assert!(model.window_event(WindowEvent::Input(WindowInput::Reached {
        conversation: conversation(),
        edge: Edge::Top,
    })));
    assert!(
        model.take_page_asks().is_empty(),
        "the limit row asks nothing"
    );
    assert!(
        model
            .window_marks(&conversation())
            .iter()
            .any(|m| m.kind == MarkKind::AtLimit(Edge::Top))
    );
}

/// A gap's marks come in section 8's reading order: the upper side's row,
/// the lower side's row, then the gap's line, all where the gap stands.
/// Mutation: the line before the lower side's row.
#[test]
fn a_gaps_marks_follow_section_8s_reading_order() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let mut model = opened_with(n, 10, Table::Unread, PageEnd::More);
    let id = gap_id(&model);
    place(&mut model, n - 5, Table::Unread);
    for edge in [Edge::GapUpper(id), Edge::GapLower(id)] {
        model.window_input(WindowInput::Reached {
            conversation: conversation(),
            edge,
        });
    }
    let kinds: Vec<MarkKind> = model
        .window_marks(&conversation())
        .into_iter()
        .filter(|m| m.place != MarkPlace::Top)
        .map(|m| m.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            MarkKind::AtLimit(Edge::GapUpper(id)),
            MarkKind::EarlierUnread {
                edge: Edge::GapLower(id),
                count: 4
            },
        ]
    );
}

/// At the session bound a gap shows one limit row, on the side that would
/// have fetched -- the one reached first -- never one on each side; once
/// its Show more messages brings that side's page, the other side, still
/// where the person is, is asked in turn (design section 5.1, section 10
/// item 14 (c)). Mutation: the other side marked at the limit too.
#[test]
fn a_gap_at_the_bound_shows_one_limit_row() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let mut model = opened_with(n, 10, Table::Unread, PageEnd::More);
    let id = gap_id(&model);
    place(&mut model, n - 5, Table::Unread);
    reach_both(&mut model, id);
    let limits: Vec<Edge> = model
        .window_marks(&conversation())
        .into_iter()
        .filter_map(|m| match m.kind {
            MarkKind::AtLimit(edge) => Some(edge),
            _ => None,
        })
        .collect();
    assert_eq!(
        limits,
        [Edge::GapUpper(id)],
        "one, on the side reached first"
    );
    model.window_input(WindowInput::ShowAnyway {
        conversation: conversation(),
        edge: Edge::GapUpper(id),
    });
    let upper = one_ask(&mut model);
    model.page_arrived(page(&upper, &[n - 4, n - 3], PageEnd::More));
    let lower = one_ask(&mut model);
    assert_eq!(lower.edge, Edge::GapLower(id), "the other side in turn");
}

fn reach_both(model: &mut UiModel, id: u64) {
    for edge in [Edge::GapUpper(id), Edge::GapLower(id)] {
        model.window_input(WindowInput::Reached {
            conversation: conversation(),
            edge,
        });
    }
}

/// A gap wholly in view fills page by page without the person moving: a
/// side still in view when the other side's page arrives counts as
/// reached again (section 5.1, U3), until the gap is gone. Mutation: a
/// side the person stays at, its page in, never asked again -- the gap
/// stalls after one page each side.
#[test]
fn a_gap_in_view_fills_page_by_page() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let mut model = opened_with(n, 10, Table::Kept, PageEnd::Exhausted);
    let id = gap_id(&model);
    place(&mut model, n - 5, Table::Kept);
    reach_both(&mut model, id);
    for (edge, row, end) in [
        (Edge::GapUpper(id), n - 4, PageEnd::More),
        (Edge::GapLower(id), n - 1, PageEnd::More),
        (Edge::GapUpper(id), n - 3, PageEnd::More),
        (Edge::GapLower(id), n - 2, PageEnd::Exhausted),
    ] {
        let ask = one_ask(&mut model);
        assert_eq!(ask.edge, edge);
        model.page_arrived(rows_of(&ask, &[row], Table::Kept, end));
    }
    assert!(model.take_page_asks().is_empty(), "the gap is full");
    for back in n - 4..n {
        assert!(holds(&model, back, Table::Kept), "row {back} back");
    }
}

/// A side asked while the other side's page is on its way is never
/// dropped: when that page fails, the waiting side is asked; a Try again
/// pressed while the other side's page is on its way is asked once that
/// page arrives. Mutations: the failure handing nothing over; the
/// deferred Try again forgotten.
#[test]
fn a_deferred_side_is_asked_when_the_other_page_settles() {
    use super::HELD_ROW_CAP;
    let n = i64::try_from(HELD_ROW_CAP).expect("small") + 4;
    let mut model = opened_with(n, 10, Table::Kept, PageEnd::Exhausted);
    let id = gap_id(&model);
    place(&mut model, n - 5, Table::Kept);
    reach_both(&mut model, id);
    let upper = one_ask(&mut model);
    assert_eq!(upper.edge, Edge::GapUpper(id));
    model.page_failed(&upper);
    let lower = one_ask(&mut model);
    assert_eq!(lower.edge, Edge::GapLower(id), "asked as the other failed");
    model.window_input(WindowInput::TryAgain {
        conversation: conversation(),
        edge: Edge::GapUpper(id),
    });
    assert!(
        model.take_page_asks().is_empty(),
        "one page of the gap at a time"
    );
    model.page_arrived(rows_of(&lower, &[n - 1], Table::Kept, PageEnd::More));
    let again = one_ask(&mut model);
    assert_eq!(again.edge, Edge::GapUpper(id), "the Try again, not dropped");
}

fn card_marks(model: &UiModel) -> usize {
    model
        .windows
        .open
        .as_ref()
        .map_or(0, |w| w.card_marks.len())
}

/// A card proposal's mark goes with it, at once, whichever way the
/// proposal goes -- its copy gone, declined, read past the read-proposal
/// bound -- and comes back with it on an Undo. Mutations: any of
/// `card_gone`, `card_resolved`, `read` and `card_restored` not following
/// the proposals held.
#[test]
fn card_marks_are_the_held_proposals_alone() {
    use super::super::cards::CardOutcome;
    let mut model = started(0);
    model.set_bounds_for_test(SESSION_ITEM_CAP, super::HELD_ROW_CAP, 1);
    model.window_input(WindowInput::Opened(conversation()));
    let _ = one_ask(&mut model);
    let row = |n: i64| RowId::from_stored(2_000 + n);

    model.card_received(card(row(1), P));
    assert_eq!(card_marks(&model), 1, "held");
    model.card_gone(row(1));
    assert_eq!(card_marks(&model), 0, "its copy gone");

    model.card_received(card(row(2), P));
    model.card_resolved(row(2), CardOutcome::Declined);
    assert_eq!(card_marks(&model), 0, "declined");

    model.card_received(card(row(3), P));
    model.card_received(card(row(4), P));
    model.read(row(3));
    model.read(row(4));
    assert_eq!(
        card_marks(&model),
        1,
        "the older read past the bound of one"
    );

    model.card_restored(card(row(3), P));
    let marks: Vec<RowId> = model
        .windows
        .open
        .as_ref()
        .map(|w| w.card_marks.keys().copied().collect())
        .unwrap_or_default();
    assert_eq!(marks, [row(3)], "back with its Undo, the bound holding");
}

/// A declined card older than every held row is no held row for the
/// held-row ceiling: letting go of the oldest message moves the top down
/// to the oldest row left, with no gap above it (#515 review F1: a mark
/// left behind made a gap there, bounded by a card no longer drawn).
/// Mutation: `card_resolved` not following the proposals held.
#[test]
fn a_declined_oldest_card_bounds_no_gap() {
    use super::super::cards::CardOutcome;
    let mut model = started(0);
    model.set_bounds_for_test(SESSION_ITEM_CAP, 8, 256);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(rows_of(
        &ask,
        &(2..=9).collect::<Vec<_>>(),
        Table::Kept,
        PageEnd::More,
    ));
    let mut oldest = card(RowId::from_stored(100), P);
    oldest.received_at = 0;
    model.card_received(oldest);
    model.card_resolved(RowId::from_stored(100), CardOutcome::Declined);
    place(&mut model, 9, Table::Kept);
    reached(&mut model);
    let ask = one_ask(&mut model);
    model.page_arrived(rows_of(&ask, &[1], Table::Kept, PageEnd::More));
    assert!(!holds(&model, 1, Table::Kept), "the farthest let go");
    let gaps = model.windows.open.as_ref().map_or(0, |w| w.gaps.len());
    assert_eq!(gaps, 0, "the top moved down, no gap made");
}

/// Built as it ships, the model holds to the published caps; only a test
/// build sets them lower. Mutation: a default other than the caps.
#[test]
fn the_default_bounds_are_the_published_caps() {
    let model = UiModel::new();
    assert_eq!(
        model.bounds,
        crate::model::Bounds {
            session_items: SESSION_ITEM_CAP,
            held_rows: super::HELD_ROW_CAP,
            read_proposals: crate::model::cards::READ_PROPOSAL_CAP,
        }
    );
}

/// A test build's held-row ceiling is the one the ceiling lets go at:
/// twelve stored rows past a ceiling of eight leave eight held, the
/// focused and the newest among them. Mutation: the ceiling read from
/// the published cap -- all twelve held.
#[test]
fn a_small_held_ceiling_lets_go_at_its_figure() {
    let mut model = started(0);
    model.set_bounds_for_test(SESSION_ITEM_CAP, 8, 256);
    model.window_input(WindowInput::Opened(conversation()));
    place(&mut model, 12, Table::Kept);
    let ask = one_ask(&mut model);
    model.page_arrived(rows_of(
        &ask,
        &(1..=12).collect::<Vec<_>>(),
        Table::Kept,
        PageEnd::More,
    ));
    let held = (1..=12).filter(|n| holds(&model, *n, Table::Kept)).count();
    assert_eq!(held, 8);
    assert!(holds(&model, 12, Table::Kept), "the focused, newest row");
}

/// A test build's session bound is the one a page's room is taken from:
/// with a bound of eight and eight unread rows held, reaching the top
/// with more unread beyond shows the limit row and asks nothing.
/// Mutation: the room taken from the published cap -- a page asked.
#[test]
fn a_small_session_bound_reaches_the_limit_row() {
    let mut model = started(20);
    model.set_bounds_for_test(8, super::HELD_ROW_CAP, 256);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &(13..=20).collect::<Vec<_>>(), PageEnd::More));
    reached(&mut model);
    assert!(
        model.take_page_asks().is_empty(),
        "the limit row asks nothing"
    );
    assert!(
        model
            .window_marks(&conversation())
            .iter()
            .any(|m| m.kind == MarkKind::AtLimit(Edge::Top))
    );
}

/// A card of another conversation -- listed while this one is open, or
/// brought back by an Undo that finished after the person moved here --
/// is none of this window's rows: a row the store holds here, newer than
/// every one the window held, still opens it again at the newest.
/// Mutations: `seen` or `holds_card` moving the open window's newest seen
/// for a card of any conversation.
#[test]
fn another_conversations_card_moves_no_newest_seen() {
    let mut model = started(2);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[999, 1_000], PageEnd::Exhausted));

    let mut listed = card(RowId::from_stored(1_005), Q);
    listed.received_at = 1_005;
    model.card_received(listed);
    let mut undone = card(RowId::from_stored(1_006), Q);
    undone.received_at = 1_006;
    model.card_restored(undone);

    model.summaries_listed(newest_at(3, 1_003), None);
    let again = one_ask(&mut model);
    assert_eq!((again.older_than, again.newer_than), (None, None));
    assert!(again.seq > ask.seq, "a new opening");
}

/// A second start on a live model -- Android's view listed again from a
/// newer facade -- re-bases its counts as a relist does: the open window
/// opens again at the newest row when the store's newest is one it never
/// held, and stays as it is when it held it. Mutation: the second start
/// replacing the counts and leaving the window alone.
#[test]
fn a_second_start_opens_the_window_again_on_a_row_never_held() {
    let mut model = started(2);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[999, 1_000], PageEnd::Exhausted));

    model.started(Vec::new(), newest_at(2, 1_000), None);
    assert!(model.take_page_asks().is_empty(), "the newest is held");
    assert_eq!(model.len(), 2, "the window as it was");

    model.started(Vec::new(), newest_at(3, 1_001), None);
    let again = one_ask(&mut model);
    assert_eq!((again.older_than, again.newer_than), (None, None));
    assert!(again.seq > ask.seq, "a new opening");
}

/// A page's rows are rows the person asked for, never arrivals: only one
/// the drain hands over while the window is open is, for a view to
/// announce (design section 10, item 3: the opening announces nothing).
/// Mutations: a page's rows marked arrived; the drain's not.
#[test]
fn only_the_drains_rows_are_arrivals_never_a_pages() {
    let mut model = started(2);
    model.window_input(WindowInput::Opened(conversation()));
    let ask = one_ask(&mut model);
    model.page_arrived(page(&ask, &[999, 1_000], PageEnd::Exhausted));
    let arrived = |model: &UiModel| -> Vec<String> {
        model
            .messages(&conversation())
            .into_iter()
            .filter(|m| m.arrived)
            .map(|m| m.source)
            .collect()
    };
    assert_eq!(model.messages(&conversation()).len(), 2);
    assert!(arrived(&model).is_empty(), "the page's rows");

    handed(&mut model, 1_001);
    assert_eq!(arrived(&model), ["row 1001"], "the drain's row");
}
