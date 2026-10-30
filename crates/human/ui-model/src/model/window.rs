// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The open conversation's window (design/2026-10-27-start-window.md
//! sections 4 to 7): its rows read from the store a page at a time, newest
//! first, older pages fetched when the person reaches the top edge, the
//! line that counts the unread rows not loaded yet, the session bound's
//! limit row and the runs it dropped, and the held-row ceiling's gaps. The
//! windowed mode only: a model given a whole listing holds every row, as it
//! always has.
//!
//! The model decides what to ask and when; the facade reads the store.
//! Asks leave through [`UiModel::take_page_asks`], answers come back
//! through [`UiModel::page_arrived`] and [`UiModel::page_failed`]. What a
//! view draws at an edge -- the progress row, the failure row, the limit
//! row, the lines -- is [`UiModel::window_marks`].

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, VecDeque};

use interweave_human_core::RowId;

use super::cards::{Proposal, READ_PROPOSAL_CAP};
use super::summaries::NewestRow;
use super::{ConversationKey, ItemKey, ListedInbound, SESSION_ITEM_CAP, Table, UiModel};

/// The most stored rows -- unread and kept, which keep their stored copy --
/// the model holds at once (design section 5.1). Larger than the rows a
/// view shows at the smallest text on the largest window, so what is never
/// let go always fits. Pending rows are not counted and never let go.
pub const HELD_ROW_CAP: usize = 1_024;

/// A row's place in its conversation, as the store orders rows: by when
/// it arrived, an unread row before a kept one at the same instant, then
/// by row. What a page's bound is made of, so the facade can hand it back
/// to the store whether or not the row is still held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageMark {
    /// When it reached this app.
    pub received_at: u64,
    /// Whether the row is kept rather than unread.
    pub kept: bool,
    /// Its row in that table.
    pub row: RowId,
}

impl PageMark {
    fn key(&self) -> (u64, bool, i64) {
        (self.received_at, self.kept, self.row.get())
    }
}

impl Ord for PageMark {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

impl PartialOrd for PageMark {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The order a page is read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageOrder {
    /// Newest first, downward from a bound.
    NewestFirst,
    /// Oldest first, upward from a bound.
    OldestFirst,
}

/// Where a page is fetched for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    /// The top edge: the rows older than the oldest loaded one.
    Top,
    /// A gap's upper side (design section 5.1): its rows nearest the held
    /// row above it, fetched oldest first.
    GapUpper(u64),
    /// A gap's lower side: its rows nearest the held row below it, fetched
    /// newest first.
    GapLower(u64),
}

/// A page the model asks the facade for: the rows of `conversation`
/// strictly between the bounds, in `order`. `unread_room`, when set, is
/// how many unread rows the page may hold (the session bound, design
/// section 7.1); `seq` names the opening it is for, so a page that comes
/// back after the conversation was closed or opened again adds nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PageAsk {
    /// The conversation.
    pub conversation: ConversationKey,
    /// The edge it is for.
    pub edge: Edge,
    /// Rows older than this, when set.
    pub older_than: Option<PageMark>,
    /// Rows newer than this, when set.
    pub newer_than: Option<PageMark>,
    /// The order the page is read in.
    pub order: PageOrder,
    /// How many unread rows the page may hold, when bounded.
    pub unread_room: Option<usize>,
    /// The opening it is for.
    pub seq: u64,
}

/// Whether the store holds more past a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageEnd {
    /// Nothing more on that side.
    Exhausted,
    /// More rows remain.
    More,
    /// More rows remain, and the page stopped because its unread room was
    /// used: the edge shows the limit row (design section 7.1).
    RoomUsed,
}

/// A page the facade read for a [`PageAsk`]: its rows, each table apart,
/// the card proposals among them, and whether more remain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRows {
    /// The ask it answers.
    pub ask: PageAsk,
    /// Its unread message rows.
    pub unread: Vec<ListedInbound>,
    /// Its kept rows.
    pub kept: Vec<ListedInbound>,
    /// Its unread card rows, as proposals.
    pub proposals: Vec<Proposal>,
    /// Whether more remain.
    pub end: PageEnd,
}

/// What a view reports of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowInput {
    /// The conversation was opened: its first page is asked.
    Opened(ConversationKey),
    /// The conversation was closed.
    Closed(ConversationKey),
    /// The person reached an edge: it scrolled into view, or the person
    /// moved focus onto it. Focus the app places is never this.
    Reached {
        /// The conversation.
        conversation: ConversationKey,
        /// The edge.
        edge: Edge,
    },
    /// The edge left the view or the person's focus (section 5: either
    /// leaving arms it again).
    Left {
        /// The conversation.
        conversation: ConversationKey,
        /// The edge.
        edge: Edge,
    },
    /// The person pressed Try again on an edge's failure row.
    TryAgain {
        /// The conversation.
        conversation: ConversationKey,
        /// The edge.
        edge: Edge,
    },
    /// The person pressed Show more messages on an edge's limit row: from
    /// now until the conversation is closed, paging drops read rows as it
    /// needs (section 7.1).
    ShowAnyway {
        /// The conversation.
        conversation: ConversationKey,
        /// The edge.
        edge: Edge,
    },
    /// The person's place: the item they moved focus to, if any, and the
    /// items any part of which is in view. Neither the session bound nor
    /// the held-row ceiling takes one of them.
    Place {
        /// The conversation.
        conversation: ConversationKey,
        /// The focused item.
        focused: Option<ItemKey>,
        /// The items in view.
        in_view: Vec<ItemKey>,
    },
}

/// Something the open conversation draws, in the order a view draws it:
/// a message item, or a card proposal's row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
    /// A message item.
    Item(ItemKey),
    /// A card proposal's row.
    Card(RowId),
}

/// Where a [`WindowMark`] stands among what the conversation draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkPlace {
    /// Above everything the conversation draws, leading proposals
    /// included: the top edge's marks.
    Top,
    /// Directly above this.
    Above(Anchor),
    /// Below everything.
    End,
}

/// Something a view draws at an edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowMark {
    /// Where it stands.
    pub place: MarkPlace,
    /// What it is.
    pub kind: MarkKind,
}

/// What a [`WindowMark`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    /// A page is on its way: the progress row (a view shows it only once
    /// the page has taken 250 ms).
    Loading(Edge),
    /// The page could not be read: the failure row and its Try again.
    Failed(Edge),
    /// `EarlierUnread`: `count` unread rows the store holds above it, at
    /// the top or in a gap, that are not loaded.
    EarlierUnread {
        /// The edge it stands at: the top, or a gap's lower side.
        edge: Edge,
        /// How many.
        count: u64,
    },
    /// The limit row: `EarlierAtLimit` and its Show more messages.
    AtLimit(Edge),
    /// `ReadNotShown`: `count` read or sent rows the session bound dropped
    /// here, which cannot come back.
    ReadNotShown {
        /// How many.
        count: u64,
    },
}

/// One side's state: the top edge, or a side of a gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeState {
    /// Waiting to be reached.
    Armed,
    /// A page is on its way.
    InFlight,
    /// A page arrived while the person was at the edge: it is asked again
    /// only once the edge was left and reached again.
    Spent,
    /// The page could not be read: only Try again asks again.
    Failed,
    /// The store holds nothing more on that side.
    Exhausted,
    /// The next page would drop a read row: the limit row shows until the
    /// person chooses Show more messages.
    AtLimit,
}

#[derive(Debug, Clone, Copy)]
struct Side {
    state: EdgeState,
    /// The person is at it.
    at: bool,
}

impl Side {
    const fn armed() -> Self {
        Self {
            state: EdgeState::Armed,
            at: false,
        }
    }
}

/// Stored rows between two held ones that the model let go (section 5.1).
#[derive(Debug, Clone)]
struct Gap {
    /// The held row just above it: older.
    above: PageMark,
    /// The held row just below it: newer.
    below: PageMark,
    /// The unread rows let go into it and not loaded again.
    unread: u64,
    upper: Side,
    lower: Side,
}

/// A place in a conversation's drawn order: when, a message before a card
/// at the same instant (as a view places a card after every message that
/// arrived no later), then the table and row. Positions are compared, never
/// looked up, so a mark keeps its place when the row it was taken from is
/// read, hidden or let go.
type Pos = (u64, u8, Table, i64);

/// The open conversation's window.
#[derive(Debug)]
struct Window {
    conversation: ConversationKey,
    seq: u64,
    /// The oldest loaded row's place, once a page brought one.
    top: Option<PageMark>,
    top_side: Side,
    gaps: BTreeMap<u64, Gap>,
    next_gap: u64,
    /// Each held stored row's place.
    marks: HashMap<(Table, RowId), PageMark>,
    /// Each held card proposal's place: a held row like a message's, so it
    /// bounds a gap (ux-designer's U2), but no item the ceiling counts.
    card_marks: HashMap<RowId, PageMark>,
    /// The person chose Show more messages for this opening.
    show_anyway: bool,
    /// The item the person moved focus to.
    focused: Option<ItemKey>,
    /// The items in view.
    in_view: Vec<ItemKey>,
    /// Each run of the conversation's rows the session bound dropped, by
    /// the place of the newest row it dropped: the line stands above
    /// whatever is drawn next, whichever row that is by then.
    runs: BTreeMap<Pos, u64>,
    /// The newest place of any stored row this opening held, a card's
    /// included: a stored row newer than it was never handed over.
    newest_seen: Option<PageMark>,
}

impl Window {
    fn side(&mut self, edge: Edge) -> Option<&mut Side> {
        match edge {
            Edge::Top => Some(&mut self.top_side),
            Edge::GapUpper(id) => self.gaps.get_mut(&id).map(|g| &mut g.upper),
            Edge::GapLower(id) => self.gaps.get_mut(&id).map(|g| &mut g.lower),
        }
    }

    /// The bounds and order a page for `edge` is read with.
    fn bounds(&self, edge: Edge) -> Option<(Option<PageMark>, Option<PageMark>, PageOrder)> {
        match edge {
            Edge::Top => Some((self.top, None, PageOrder::NewestFirst)),
            Edge::GapUpper(id) => self
                .gaps
                .get(&id)
                .map(|g| (Some(g.below), Some(g.above), PageOrder::OldestFirst)),
            Edge::GapLower(id) => self
                .gaps
                .get(&id)
                .map(|g| (Some(g.below), Some(g.above), PageOrder::NewestFirst)),
        }
    }

    fn gaps_unread(&self) -> u64 {
        self.gaps.values().map(|g| g.unread).sum()
    }
}

/// The windowed mode's windows: the open conversation's, and the asks not
/// yet taken.
#[derive(Debug, Default)]
pub(super) struct Windows {
    open: Option<Window>,
    next_seq: u64,
    asks: VecDeque<PageAsk>,
}

impl Windows {
    fn ask(&mut self, edge: Edge, unread_room: Option<usize>) {
        let Some(window) = self.open.as_mut() else {
            return;
        };
        let Some((older_than, newer_than, order)) = window.bounds(edge) else {
            return;
        };
        // One page of a gap at a time (section 5.1): the rows on its two
        // sides stand next to each other on screen, so both are reached
        // together, and two pages would read the same rows and count them
        // twice out of the gap's line
        // (`reaching_both_sides_of_a_gap_asks_one_page`).
        if let Edge::GapUpper(id) | Edge::GapLower(id) = edge
            && window.gaps.get(&id).is_some_and(|g| {
                g.upper.state == EdgeState::InFlight || g.lower.state == EdgeState::InFlight
            })
        {
            return;
        }
        if let Some(side) = window.side(edge) {
            side.state = EdgeState::InFlight;
        }
        self.asks.push_back(PageAsk {
            conversation: window.conversation.clone(),
            edge,
            older_than,
            newer_than,
            order,
            unread_room,
            seq: window.seq,
        });
    }

    /// The item the person moved focus to in the open conversation.
    pub(super) fn focused(&self) -> Option<ItemKey> {
        self.open.as_ref().and_then(|w| w.focused)
    }

    /// The open conversation, if any.
    pub(super) fn open_conversation(&self) -> Option<&ConversationKey> {
        self.open.as_ref().map(|w| &w.conversation)
    }

    /// A held stored row of the open conversation, at `mark`.
    pub(super) fn holds(&mut self, table: Table, row: RowId, mark: PageMark) {
        if let Some(window) = self.open.as_mut() {
            window.marks.insert((table, row), mark);
        }
        self.seen(mark);
    }

    /// The open conversation held a stored row at `mark`.
    pub(super) fn seen(&mut self, mark: PageMark) {
        if let Some(window) = self.open.as_mut() {
            window.newest_seen = window.newest_seen.max(Some(mark));
        }
    }

    /// Whether the open conversation is `conversation` and the store holds
    /// a row of it at `newest` that it never held: newer than every one it
    /// did, so one the facade could not hand over.
    pub(super) fn missed(&self, conversation: &ConversationKey, newest: PageMark) -> bool {
        self.open
            .as_ref()
            .is_some_and(|w| &w.conversation == conversation && w.newest_seen < Some(newest))
    }

    /// A row of the open conversation is no longer a held stored row: read,
    /// or let go.
    pub(super) fn forget(&mut self, table: Table, row: RowId) {
        if let Some(window) = self.open.as_mut() {
            window.marks.remove(&(table, row));
        }
    }

    /// A held card proposal of the open conversation, at `mark`.
    pub(super) fn holds_card(
        &mut self,
        conversation: &ConversationKey,
        row: RowId,
        mark: PageMark,
    ) {
        if let Some(window) = self.open_for(conversation) {
            window.card_marks.insert(row, mark);
        }
        self.seen(mark);
    }

    /// A row of the open conversation the session bound dropped, from
    /// `at`: a run of its own, joined to its neighbours by
    /// [`UiModel::join_runs`] once it is no longer drawn.
    pub(super) fn dropped(&mut self, at: Pos) {
        if let Some(window) = self.open.as_mut() {
            *window.runs.entry(at).or_insert(0) += 1;
        }
    }

    fn open_for(&mut self, conversation: &ConversationKey) -> Option<&mut Window> {
        self.open
            .as_mut()
            .filter(|w| &w.conversation == conversation)
    }
}

impl UiModel {
    /// What a view reports of the open conversation's window. Nothing in
    /// the listed mode, where every row is held.
    pub fn window_input(&mut self, input: WindowInput) {
        if self.windowed.is_none() {
            return;
        }
        match input {
            WindowInput::Opened(conversation) => {
                // One conversation is open at a time: the one before is
                // closed, its stored rows let go.
                if let Some(before) = self.windows.open.take() {
                    self.let_go(&before.conversation);
                    self.cards.let_go_unread(&before.conversation);
                }
                let windows = &mut self.windows;
                windows.next_seq += 1;
                windows.open = Some(Window {
                    conversation,
                    seq: windows.next_seq,
                    top: None,
                    top_side: Side::armed(),
                    gaps: BTreeMap::new(),
                    next_gap: 0,
                    marks: HashMap::new(),
                    card_marks: HashMap::new(),
                    show_anyway: false,
                    focused: None,
                    in_view: Vec::new(),
                    runs: BTreeMap::new(),
                    newest_seen: None,
                });
                // The opening reads its window at once, not covered by the
                // limit row (sections 4 and 7.1).
                windows.ask(Edge::Top, None);
            }
            WindowInput::Closed(conversation) => {
                if self.windows.open_for(&conversation).is_some() {
                    self.windows.open = None;
                    // A closed conversation holds none of its stored rows
                    // (section 3): they stay in the store for its next
                    // opening.
                    self.let_go(&conversation);
                    self.cards.let_go_unread(&conversation);
                }
            }
            WindowInput::Reached { conversation, edge } => {
                let Some(side) = self
                    .windows
                    .open_for(&conversation)
                    .and_then(|w| w.side(edge))
                else {
                    return;
                };
                side.at = true;
                // Once each time the edge is reached, never while a page is
                // on its way, never after a failure: Try again retries. At
                // the limit it is judged again: room may have come.
                if matches!(side.state, EdgeState::Armed | EdgeState::AtLimit) {
                    self.reach(edge);
                }
            }
            WindowInput::Left { conversation, edge } => {
                if let Some(side) = self
                    .windows
                    .open_for(&conversation)
                    .and_then(|w| w.side(edge))
                {
                    side.at = false;
                    if side.state == EdgeState::Spent {
                        side.state = EdgeState::Armed;
                    }
                }
            }
            WindowInput::TryAgain { conversation, edge } => {
                if self
                    .windows
                    .open_for(&conversation)
                    .and_then(|w| w.side(edge))
                    .is_some_and(|s| s.state == EdgeState::Failed)
                {
                    self.reach(edge);
                }
            }
            WindowInput::ShowAnyway { conversation, edge } => {
                if let Some(window) = self.windows.open_for(&conversation)
                    && window
                        .side(edge)
                        .is_some_and(|s| s.state == EdgeState::AtLimit)
                {
                    window.show_anyway = true;
                    self.windows.ask(edge, None);
                }
            }
            WindowInput::Place {
                conversation,
                focused,
                in_view,
            } => {
                if let Some(window) = self.windows.open_for(&conversation) {
                    window.focused = focused;
                    window.in_view = in_view;
                }
            }
        }
    }

    /// The pages asked since the last call, for the facade to read.
    pub fn take_page_asks(&mut self) -> Vec<PageAsk> {
        self.windows.asks.drain(..).collect()
    }

    /// A page the facade read: its rows are held, the edge that asked moves
    /// past them, and waits to be reached again; then the held-row ceiling
    /// lets go of what it must (section 5.1). A page for an opening that is
    /// over -- the conversation closed or opened again -- or for a gap no
    /// longer there adds nothing: its rows stay in the store
    /// (`a_page_for_an_earlier_opening_adds_nothing`).
    pub fn page_arrived(&mut self, page: PageRows) {
        let current = self.windows.open.as_ref().is_some_and(|w| {
            w.conversation == page.ask.conversation
                && w.seq == page.ask.seq
                && w.bounds(page.ask.edge).is_some()
        });
        if !current {
            return;
        }
        // A top page asked from a top the held-row ceiling has since moved
        // down reads only rows older than the old top, and would leave the
        // rows let go between the two tops in no gap and above no edge,
        // never read back. It adds nothing, and the top is asked again from
        // where it stands now
        // (`a_top_page_in_flight_when_the_ceiling_moves_the_top_adds_nothing`).
        if page.ask.edge == Edge::Top
            && let Some(window) = self.windows.open.as_mut()
            && window.top != page.ask.older_than
        {
            window.top_side.state = EdgeState::Armed;
            if window.top_side.at {
                self.reach(Edge::Top);
            }
            return;
        }
        let mut marks: Vec<((Table, RowId), PageMark, bool)> = Vec::new();
        for row in &page.unread {
            let mark = PageMark {
                received_at: row.received_at,
                kept: false,
                row: row.row,
            };
            marks.push(((Table::Unread, row.row), mark, true));
        }
        for row in &page.kept {
            let mark = PageMark {
                received_at: row.received_at,
                kept: true,
                row: row.row,
            };
            marks.push(((Table::Kept, row.row), mark, false));
        }
        for proposal in &page.proposals {
            let mark = PageMark {
                received_at: proposal.received_at,
                kept: false,
                row: proposal.row,
            };
            marks.push(((Table::Unread, proposal.row), mark, true));
        }
        let unread_brought =
            u64::try_from(marks.iter().filter(|(_, _, u)| *u).count()).unwrap_or(u64::MAX);
        let oldest = marks.iter().map(|(_, m, _)| *m).min();
        let newest = marks.iter().map(|(_, m, _)| *m).max();
        // A row committed after the start's read that the drain has not
        // handed over yet is in no count: the page brings it first, and
        // counts it (once).
        if let Some(windowed) = self.windowed.as_mut() {
            let conversation = &page.ask.conversation;
            for row in &page.unread {
                if windowed.paged(row.row) {
                    windowed.arrived(
                        conversation,
                        NewestRow::message(
                            Table::Unread,
                            row.row,
                            row.received_at,
                            &row.envelope.text,
                        ),
                    );
                }
            }
            for proposal in &page.proposals {
                if windowed.paged(proposal.row) {
                    windowed.arrived(
                        conversation,
                        NewestRow::card(proposal.row, proposal.received_at),
                    );
                }
            }
        }
        for row in page.unread {
            self.unread_listed(vec![row]);
        }
        self.kept_listed(page.kept);
        for proposal in page.proposals {
            self.card_received(proposal);
        }
        if let Some(newest) = newest {
            self.windows.seen(newest);
        }
        // Each message row the model now holds has its place; a proposal is
        // not an item, and the ceiling counts items' rows.
        for ((table, row), mark, _) in &marks {
            if self.by_row.contains_key(&(*table, *row)) {
                self.windows.holds(*table, *row, *mark);
            }
        }
        let Some(window) = self.windows.open.as_mut() else {
            return;
        };
        let edge = page.ask.edge;
        let mut gap_done = false;
        match edge {
            Edge::Top => {
                if let Some(oldest) = oldest
                    && window.top.is_none_or(|t| oldest < t)
                {
                    window.top = Some(oldest);
                }
            }
            Edge::GapUpper(id) | Edge::GapLower(id) => {
                if let Some(gap) = window.gaps.get_mut(&id) {
                    gap.unread = gap.unread.saturating_sub(unread_brought);
                    match edge {
                        Edge::GapUpper(_) => {
                            if let Some(newest) = newest {
                                gap.above = gap.above.max(newest);
                            }
                        }
                        _ => {
                            if let Some(oldest) = oldest {
                                gap.below = gap.below.min(oldest);
                            }
                        }
                    }
                    gap_done = page.end == PageEnd::Exhausted;
                }
            }
        }
        if gap_done && let Edge::GapUpper(id) | Edge::GapLower(id) = edge {
            // A gap whose rows have all loaded is gone, and nothing marks
            // where it was.
            window.gaps.remove(&id);
        } else {
            let show_anyway = window.show_anyway;
            if let Some(side) = window.side(edge) {
                side.state = match page.end {
                    PageEnd::Exhausted => EdgeState::Exhausted,
                    PageEnd::RoomUsed if !show_anyway => EdgeState::AtLimit,
                    PageEnd::More | PageEnd::RoomUsed if side.at => EdgeState::Spent,
                    PageEnd::More | PageEnd::RoomUsed => EdgeState::Armed,
                };
            }
        }
        // The gap's other side, reached while this page was on its way and
        // so not asked then, is asked now that none is.
        let other = match edge {
            Edge::GapUpper(id) if !gap_done => Some(Edge::GapLower(id)),
            Edge::GapLower(id) if !gap_done => Some(Edge::GapUpper(id)),
            _ => None,
        };
        if let Some(other) = other
            && window
                .side(other)
                .is_some_and(|s| s.at && matches!(s.state, EdgeState::Armed | EdgeState::AtLimit))
        {
            self.reach(other);
        }
        self.hold_ceiling();
    }

    /// The page asked for `ask` could not be read: the edge shows the
    /// failure row, and only Try again asks again.
    pub fn page_failed(&mut self, ask: &PageAsk) {
        if let Some(window) = self.windows.open.as_mut()
            && window.conversation == ask.conversation
            && window.seq == ask.seq
            && let Some(side) = window.side(ask.edge)
        {
            side.state = EdgeState::Failed;
        }
    }

    /// The person reached `edge` with it armed, at the limit, or after Try
    /// again: ask its next page, within the room the session has left --
    /// or, at the session bound with unread rows beyond the edge, show the
    /// limit row and ask nothing, until the person chooses Show more
    /// messages (design section 7.1). With nothing unread beyond the edge,
    /// a page makes no row session-only, so it is asked whatever the room.
    fn reach(&mut self, edge: Edge) {
        let Some(window) = self.windows.open.as_ref() else {
            return;
        };
        if window.show_anyway {
            self.windows.ask(edge, None);
            return;
        }
        let conversation = window.conversation.clone();
        let beyond = match edge {
            Edge::Top => self.unread_not_loaded(&conversation),
            Edge::GapUpper(id) | Edge::GapLower(id) => window.gaps.get(&id).map_or(0, |g| g.unread),
        };
        let room = self.page_room(&conversation);
        if room == 0 && beyond > 0 {
            if let Some(side) = self.windows.open.as_mut().and_then(|w| w.side(edge)) {
                side.state = EdgeState::AtLimit;
            }
            return;
        }
        self.windows.ask(edge, Some(room));
    }

    /// How many unread rows a page may bring without the session bound
    /// dropping a read row (design section 7.1): the smaller of the room
    /// session-only rows have and the room read card proposals have
    /// (architect-cto's ruling, 01a180cc), each less the open
    /// conversation's unread rows already held, which become session-only
    /// when read.
    fn page_room(&self, conversation: &ConversationKey) -> usize {
        let (messages, cards) = self.held_unread(conversation);
        let session = SESSION_ITEM_CAP.saturating_sub(self.ephemeral.len() + messages);
        let proposals = READ_PROPOSAL_CAP.saturating_sub(self.cards.read_count() + cards);
        session.min(proposals)
    }

    /// Past [`HELD_ROW_CAP`] held stored rows, let go of the open
    /// conversation's rows farthest from the person's place -- the focused
    /// item, else the middle of the view, else the newest row -- in items
    /// along the conversation; never a pending row, the newest row, the
    /// focused item, an item in view, or one an open snackbar's Undo needs
    /// (design section 5.1). A run let go at the top moves the top edge
    /// down; anywhere else it leaves a gap, its unread rows counted there.
    pub(super) fn hold_ceiling(&mut self) {
        let Some(window) = self.windows.open.as_ref() else {
            return;
        };
        let conversation = window.conversation.clone();
        let focused = window.focused;
        let in_view = window.in_view.clone();
        loop {
            let held: Vec<(ItemKey, Vec<(Table, RowId)>)> = self.stored_items(&conversation);
            let count: usize = held
                .iter()
                .map(|(_, rows)| rows.iter().filter(|(t, _)| *t != Table::Pending).count())
                .sum();
            if count <= HELD_ROW_CAP {
                return;
            }
            let order = self.conversation_order(&conversation);
            // The newest item holding a stored row, not the newest item: a
            // message sent here is newer and holds none, and below the
            // newest stored row no edge reads rows back.
            let Some(newest) = order
                .iter()
                .rev()
                .find(|k| held.iter().any(|(h, _)| h == *k))
                .copied()
            else {
                return;
            };
            let place = focused
                .and_then(|f| order.iter().position(|k| *k == f))
                .or_else(|| {
                    let at: Vec<usize> = in_view
                        .iter()
                        .filter_map(|v| order.iter().position(|k| k == v))
                        .collect();
                    (!at.is_empty()).then(|| at[at.len() / 2])
                })
                .unwrap_or(order.len() - 1);
            let mut candidates: Vec<_> = held
                .iter()
                .filter(|(key, rows)| {
                    *key != newest
                        && Some(*key) != focused
                        && !in_view.contains(key)
                        && !self.swipe.protects(key)
                        && rows.iter().all(|(t, _)| *t != Table::Pending)
                })
                .filter_map(|(key, rows)| {
                    order.iter().position(|k| k == key).map(|at| {
                        (
                            at.abs_diff(place),
                            std::cmp::Reverse(at),
                            *key,
                            rows.clone(),
                        )
                    })
                })
                .collect();
            candidates.sort_by_key(|(distance, older, key, _)| (*distance, *older, *key));
            // The farthest first; one that cannot go -- an item holding a
            // stored row newer than every other held row, a later copy of
            // an older message -- is passed over for the next, so the
            // ceiling holds while anything can go
            // (`the_ceiling_holds_past_an_item_holding_the_newest_stored_row`).
            if !candidates
                .into_iter()
                .rev()
                .any(|(_, _, key, rows)| self.let_go_item(key, &rows))
            {
                return;
            }
        }
    }

    /// Let go of one held item's stored rows, keeping the window's top and
    /// gaps true: below every other held row, the top moves down to the
    /// oldest one left; between held rows, the rows join a gap, merged with
    /// a neighbour's. False, and the item kept, when no held row is newer:
    /// rows let go there would have no side to be read back from.
    fn let_go_item(&mut self, key: ItemKey, rows: &[(Table, RowId)]) -> bool {
        let Some(window) = self.windows.open.as_mut() else {
            return false;
        };
        let mut unread = 0;
        let gone: Vec<PageMark> = rows
            .iter()
            .filter_map(|r| window.marks.get(r).copied())
            .collect();
        let (Some(low), Some(high)) = (gone.iter().min().copied(), gone.iter().max().copied())
        else {
            self.drop_items(&[key]);
            return true;
        };
        // A held proposal bounds a gap as a held message does: released rows
        // on its two sides are two gaps (ux-designer's U2).
        let held = || window.marks.values().chain(window.card_marks.values());
        let above = held().filter(|m| **m < low).max().copied();
        let Some(below) = held().filter(|m| **m > high).min().copied() else {
            return false;
        };
        for (table, row) in rows {
            window.marks.remove(&(*table, *row));
            if *table == Table::Unread {
                unread += 1;
            }
        }
        match above {
            None => {
                // At the top: the edge moves down to the oldest row left,
                // and the rows let go are older than it, loaded again from
                // there.
                window.top = Some(below);
                if window.top_side.state == EdgeState::Exhausted {
                    window.top_side.state = EdgeState::Armed;
                }
                // A gap whose upper side was this row now opens onto the
                // top: its rows are older than the top edge.
                let joined: Vec<u64> = window
                    .gaps
                    .iter()
                    .filter(|(_, g)| g.above >= low && g.above <= high)
                    .map(|(id, _)| *id)
                    .collect();
                for id in joined {
                    window.gaps.remove(&id);
                }
            }
            Some(above) => {
                let touching: Vec<u64> = window
                    .gaps
                    .iter()
                    .filter(|(_, g)| {
                        (g.below >= low && g.below <= high) || (g.above >= low && g.above <= high)
                    })
                    .map(|(id, _)| *id)
                    .collect();
                let mut merged = Gap {
                    above,
                    below,
                    unread,
                    upper: Side::armed(),
                    lower: Side::armed(),
                };
                for id in touching {
                    if let Some(g) = window.gaps.remove(&id) {
                        merged.above = merged.above.min(g.above);
                        merged.below = merged.below.max(g.below);
                        merged.unread += g.unread;
                    }
                }
                window.next_gap += 1;
                window.gaps.insert(window.next_gap, merged);
            }
        }
        self.drop_items(&[key]);
        true
    }

    /// The open conversation's items that hold a stored row, with those
    /// rows.
    fn stored_items(&self, conversation: &ConversationKey) -> Vec<(ItemKey, Vec<(Table, RowId)>)> {
        self.items
            .iter()
            .filter(|(_, i)| &i.conversation == conversation)
            .map(|(k, i)| {
                (
                    *k,
                    i.rows
                        .iter()
                        .copied()
                        .filter(|(t, _)| matches!(t, Table::Unread | Table::Kept | Table::Pending))
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, rows)| rows.iter().any(|(t, _)| *t != Table::Pending))
            .collect()
    }

    /// A conversation's items, in its own order.
    fn conversation_order(&self, conversation: &ConversationKey) -> Vec<ItemKey> {
        let mut keys: Vec<(u64, ItemKey)> = self
            .items
            .iter()
            .filter(|(_, i)| &i.conversation == conversation && !i.hidden)
            .map(|(k, i)| (i.local_at, *k))
            .collect();
        keys.sort();
        keys.into_iter().map(|(_, k)| k).collect()
    }

    /// What a view draws at the open conversation's edges: the progress,
    /// failure or limit row at the top, then `EarlierUnread` with the count
    /// of the unread rows the store holds that are not loaded and not in a
    /// gap (design section 6), all above everything drawn; each gap's rows
    /// on its sides and its own `EarlierUnread`, where its released rows
    /// stood; and each `ReadNotShown` run, runs with nothing drawn between
    /// them read as one. Each is placed by position, so it keeps its place
    /// when the row it was taken from is read, hidden or let go
    /// (`a_gap_keeps_its_place_when_the_row_above_it_is_read`,
    /// `a_read_not_shown_line_stays_when_the_row_below_it_is_hidden`).
    /// Nothing for a conversation that is not open.
    #[must_use]
    pub fn window_marks(&self, conversation: &ConversationKey) -> Vec<WindowMark> {
        let Some(window) = self
            .windows
            .open
            .as_ref()
            .filter(|w| &w.conversation == conversation)
        else {
            return Vec::new();
        };
        let drawn = self.drawn(conversation);
        let mut marks = Vec::new();
        if let Some(kind) = side_mark(window.top_side.state, Edge::Top) {
            marks.push(WindowMark {
                place: MarkPlace::Top,
                kind,
            });
        }
        let count = self.unread_not_loaded(conversation);
        if count > 0 && window.top_side.state != EdgeState::Exhausted {
            marks.push(WindowMark {
                place: MarkPlace::Top,
                kind: MarkKind::EarlierUnread {
                    edge: Edge::Top,
                    count,
                },
            });
        }
        for (id, gap) in &window.gaps {
            let upper = place_after(&drawn, self.mark_pos(window, gap.above));
            let lower = place_at(&drawn, self.mark_pos(window, gap.below));
            if let Some(kind) = side_mark(gap.upper.state, Edge::GapUpper(*id)) {
                marks.push(WindowMark { place: upper, kind });
            }
            if gap.unread > 0 {
                marks.push(WindowMark {
                    place: lower,
                    kind: MarkKind::EarlierUnread {
                        edge: Edge::GapLower(*id),
                        count: gap.unread,
                    },
                });
            }
            if let Some(kind) = side_mark(gap.lower.state, Edge::GapLower(*id)) {
                marks.push(WindowMark { place: lower, kind });
            }
        }
        let mut runs: Vec<(MarkPlace, u64)> = Vec::new();
        for (at, count) in &window.runs {
            let place = place_after(&drawn, *at);
            match runs.last_mut() {
                Some((last, sum)) if *last == place => *sum += count,
                _ => runs.push((place, *count)),
            }
        }
        marks.extend(runs.into_iter().map(|(place, count)| WindowMark {
            place,
            kind: MarkKind::ReadNotShown { count },
        }));
        marks
    }

    /// Where the person reaches each edge of the open conversation that a
    /// page can still be asked for: the top, and both sides of every gap,
    /// each at the place its marks stand. A view reports
    /// [`WindowInput::Reached`] when that place comes into view or the
    /// person moves focus onto what stands there, and
    /// [`WindowInput::Left`] when it leaves either.
    #[must_use]
    pub fn window_edges(&self, conversation: &ConversationKey) -> Vec<(Edge, MarkPlace)> {
        let Some(window) = self
            .windows
            .open
            .as_ref()
            .filter(|w| &w.conversation == conversation)
        else {
            return Vec::new();
        };
        let drawn = self.drawn(conversation);
        let mut edges = Vec::new();
        if window.top_side.state != EdgeState::Exhausted {
            edges.push((Edge::Top, MarkPlace::Top));
        }
        for (id, gap) in &window.gaps {
            edges.push((
                Edge::GapUpper(*id),
                place_after(&drawn, self.mark_pos(window, gap.above)),
            ));
            edges.push((
                Edge::GapLower(*id),
                place_at(&drawn, self.mark_pos(window, gap.below)),
            ));
        }
        edges
    }

    /// Runs of dropped rows with nothing drawn between them are one run,
    /// kept at the newest one's place, so the runs are never more than
    /// what is drawn, plus one.
    pub(super) fn join_runs(&mut self) {
        let Some(conversation) = self.windows.open_conversation().cloned() else {
            return;
        };
        let drawn = self.drawn(&conversation);
        let Some(window) = self.windows.open.as_mut() else {
            return;
        };
        let mut joined: BTreeMap<Pos, u64> = BTreeMap::new();
        let mut last: Option<(MarkPlace, Pos)> = None;
        for (at, count) in std::mem::take(&mut window.runs) {
            let place = place_after(&drawn, at);
            match last {
                Some((before, kept)) if before == place => {
                    let sum = joined.remove(&kept).unwrap_or(0) + count;
                    joined.insert(at, sum);
                }
                _ => {
                    joined.insert(at, count);
                }
            }
            last = Some((place, at));
        }
        window.runs = joined;
    }

    /// What `conversation` draws, in the order a view draws it: its shown
    /// items, and its card proposals each after every message that arrived
    /// no later.
    fn drawn(&self, conversation: &ConversationKey) -> Vec<(Pos, Anchor)> {
        let mut drawn: Vec<(Pos, Anchor)> = self
            .items
            .iter()
            .filter(|(_, i)| &i.conversation == conversation && !i.hidden)
            .map(|(k, i)| (item_pos(i.local_at, *k), Anchor::Item(*k)))
            .collect();
        drawn.extend(
            self.proposals(conversation)
                .into_iter()
                .map(|p| (card_pos(p.received_at, p.row), Anchor::Card(p.row))),
        );
        drawn.sort_unstable_by_key(|(at, _)| *at);
        drawn
    }

    /// The drawn position of a held row's place.
    fn mark_pos(&self, window: &Window, mark: PageMark) -> Pos {
        if window.card_marks.get(&mark.row) == Some(&mark) {
            return card_pos(mark.received_at, mark.row);
        }
        let table = if mark.kept {
            Table::Kept
        } else {
            Table::Unread
        };
        self.by_row
            .get(&(table, mark.row))
            .and_then(|k| self.items.get(k).map(|i| item_pos(i.local_at, *k)))
            .unwrap_or_else(|| item_pos(mark.received_at, ItemKey(table, mark.row)))
    }

    /// The drawn position of a held item, for the session bound's runs.
    pub(super) fn drop_pos(&self, key: ItemKey) -> Option<Pos> {
        self.items.get(&key).map(|i| item_pos(i.local_at, key))
    }

    /// The unread rows of `conversation` the store holds above the top edge
    /// and not loaded: the store's count less the unread rows held,
    /// messages and cards alike, one per row as the store counts them, and
    /// less those let go into gaps, which each gap's line counts.
    fn unread_not_loaded(&self, conversation: &ConversationKey) -> u64 {
        let Some(stored) = self.windowed.as_ref().map(|w| w.unread_of(conversation)) else {
            return 0;
        };
        let (messages, cards) = self.held_unread(conversation);
        let held = u64::try_from(messages + cards).unwrap_or(u64::MAX);
        let gaps = self
            .windows
            .open
            .as_ref()
            .filter(|w| &w.conversation == conversation)
            .map_or(0, Window::gaps_unread);
        stored.saturating_sub(held).saturating_sub(gaps)
    }

    /// The unread rows of `conversation` the model holds: messages, one
    /// per row, and card proposals.
    fn held_unread(&self, conversation: &ConversationKey) -> (usize, usize) {
        let messages = self
            .by_row
            .iter()
            .filter(|((table, _), key)| {
                *table == Table::Unread
                    && self
                        .items
                        .get(key)
                        .is_some_and(|i| &i.conversation == conversation)
            })
            .count();
        let cards = match conversation {
            ConversationKey::Direct {
                peer,
                endpoint: Some(at),
            } => self.cards.unread_in(peer, at).len(),
            _ => 0,
        };
        (messages, cards)
    }
}

fn item_pos(at: u64, key: ItemKey) -> Pos {
    (at, 0, key.0, key.1.get())
}

fn card_pos(at: u64, row: RowId) -> Pos {
    (at, 1, Table::Unread, row.get())
}

/// Directly above the first thing drawn after `at`, else at the end.
fn place_after(drawn: &[(Pos, Anchor)], at: Pos) -> MarkPlace {
    let i = drawn.partition_point(|(p, _)| *p <= at);
    drawn
        .get(i)
        .map_or(MarkPlace::End, |(_, a)| MarkPlace::Above(*a))
}

/// Directly above the first thing drawn at or after `at`, else at the end.
fn place_at(drawn: &[(Pos, Anchor)], at: Pos) -> MarkPlace {
    let i = drawn.partition_point(|(p, _)| *p < at);
    drawn
        .get(i)
        .map_or(MarkPlace::End, |(_, a)| MarkPlace::Above(*a))
}

const fn side_mark(state: EdgeState, edge: Edge) -> Option<MarkKind> {
    match state {
        EdgeState::InFlight => Some(MarkKind::Loading(edge)),
        EdgeState::Failed => Some(MarkKind::Failed(edge)),
        EdgeState::AtLimit => Some(MarkKind::AtLimit(edge)),
        EdgeState::Armed | EdgeState::Spent | EdgeState::Exhausted => None,
    }
}

#[cfg(test)]
mod tests;
