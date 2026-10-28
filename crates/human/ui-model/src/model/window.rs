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
    /// The edge left the view and the person's focus.
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

/// Something a view draws at an edge, anchored above an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowMark {
    /// The item it stands directly above; `None` when it stands below
    /// every item.
    pub above: Option<ItemKey>,
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
    /// The person chose Show more messages for this opening.
    show_anyway: bool,
    /// The item the person moved focus to.
    focused: Option<ItemKey>,
    /// The items in view.
    in_view: Vec<ItemKey>,
    /// Each run of the conversation's rows the session bound dropped, by
    /// the item it stands directly above (`None`: below every item).
    runs: BTreeMap<Option<ItemKey>, u64>,
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
    }

    /// A row of the open conversation is no longer a held stored row: read,
    /// or let go.
    pub(super) fn forget(&mut self, table: Table, row: RowId) {
        if let Some(window) = self.open.as_mut() {
            window.marks.remove(&(table, row));
        }
    }

    /// A row of the open conversation the session bound dropped, `below`
    /// the item now directly below where it stood: its run grows, and a
    /// run that stood directly above the dropped row joins it, the two
    /// being one run now.
    pub(super) fn dropped(&mut self, dropped: ItemKey, below: Option<ItemKey>) {
        if let Some(window) = self.open.as_mut() {
            let joined = window.runs.remove(&Some(dropped)).unwrap_or(0);
            *window.runs.entry(below).or_insert(0) += joined + 1;
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
                    show_anyway: false,
                    focused: None,
                    in_view: Vec::new(),
                    runs: BTreeMap::new(),
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
        for row in page.unread {
            self.unread_listed(vec![row]);
        }
        self.kept_listed(page.kept);
        for proposal in page.proposals {
            self.card_received(proposal);
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
            let Some(newest) = order.last().copied() else {
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
            let victim = held
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
                .max_by_key(|(distance, older, key, _)| (*distance, *older, *key));
            let Some((_, _, key, rows)) = victim else {
                return;
            };
            self.let_go_item(key, &rows);
        }
    }

    /// Let go of one held item's stored rows, keeping the window's top and
    /// gaps true: below every other held row, the top moves down to the
    /// oldest one left; between held rows, the rows join a gap, merged with
    /// a neighbour's.
    fn let_go_item(&mut self, key: ItemKey, rows: &[(Table, RowId)]) {
        let Some(window) = self.windows.open.as_mut() else {
            return;
        };
        let mut unread = 0;
        let mut gone: Vec<PageMark> = Vec::new();
        for (table, row) in rows {
            if let Some(mark) = window.marks.remove(&(*table, *row)) {
                gone.push(mark);
            }
            if *table == Table::Unread {
                unread += 1;
            }
        }
        let (Some(low), Some(high)) = (gone.iter().min().copied(), gone.iter().max().copied())
        else {
            self.drop_items(&[key]);
            return;
        };
        let above = window.marks.values().filter(|m| **m < low).max().copied();
        let below = window.marks.values().filter(|m| **m > high).min().copied();
        match (above, below) {
            (None, Some(below)) => {
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
            (Some(above), Some(below)) => {
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
            // The newest row is never let go, so there is always a row
            // below: these cannot happen, and nothing is done.
            (_, None) => {}
        }
        self.drop_items(&[key]);
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

    /// What a view draws at the open conversation's edges, each above the
    /// item it stands over: the progress, failure or limit row at the top,
    /// then `EarlierUnread` with the count of the unread rows the store
    /// holds that are not loaded and not in a gap (design section 6); each
    /// gap's rows on its sides and its own `EarlierUnread`; and each
    /// `ReadNotShown` run. Nothing for a conversation that is not open.
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
        let mut marks = Vec::new();
        let side_mark = |state: EdgeState, edge: Edge| match state {
            EdgeState::InFlight => Some(MarkKind::Loading(edge)),
            EdgeState::Failed => Some(MarkKind::Failed(edge)),
            EdgeState::AtLimit => Some(MarkKind::AtLimit(edge)),
            EdgeState::Armed | EdgeState::Spent | EdgeState::Exhausted => None,
        };
        let above = self.oldest_item(conversation);
        if let Some(kind) = side_mark(window.top_side.state, Edge::Top) {
            marks.push(WindowMark { above, kind });
        }
        let count = self.unread_not_loaded(conversation);
        if count > 0 && window.top_side.state != EdgeState::Exhausted {
            marks.push(WindowMark {
                above,
                kind: MarkKind::EarlierUnread {
                    edge: Edge::Top,
                    count,
                },
            });
        }
        for (id, gap) in &window.gaps {
            let row_above = self.item_of(window, gap.above);
            let row_below = self.item_of(window, gap.below);
            if let Some(kind) = side_mark(gap.upper.state, Edge::GapUpper(*id)) {
                marks.push(WindowMark {
                    above: row_above.and_then(|k| self.item_below(k)),
                    kind,
                });
            }
            if let Some(kind) = side_mark(gap.lower.state, Edge::GapLower(*id)) {
                marks.push(WindowMark {
                    above: row_below,
                    kind,
                });
            }
            if gap.unread > 0 {
                marks.push(WindowMark {
                    above: row_below,
                    kind: MarkKind::EarlierUnread {
                        edge: Edge::GapLower(*id),
                        count: gap.unread,
                    },
                });
            }
        }
        for (below, count) in &window.runs {
            marks.push(WindowMark {
                above: *below,
                kind: MarkKind::ReadNotShown { count: *count },
            });
        }
        marks
    }

    /// The held item at `mark`.
    fn item_of(&self, window: &Window, mark: PageMark) -> Option<ItemKey> {
        window
            .marks
            .iter()
            .find(|(_, m)| **m == mark)
            .and_then(|(at, _)| self.by_row.get(at).copied())
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

    /// The item directly below `key` in its conversation's order, `key`
    /// itself not counted.
    pub(super) fn item_below(&self, key: ItemKey) -> Option<ItemKey> {
        let item = self.items.get(&key)?;
        let at = (item.local_at, key);
        self.items
            .iter()
            .filter(|(k, i)| {
                i.conversation == item.conversation && !i.hidden && (i.local_at, **k) > at
            })
            .min_by_key(|(k, i)| (i.local_at, **k))
            .map(|(k, _)| *k)
    }

    /// The oldest item a conversation holds, in its own order.
    fn oldest_item(&self, conversation: &ConversationKey) -> Option<ItemKey> {
        self.items
            .iter()
            .filter(|(_, i)| &i.conversation == conversation && !i.hidden)
            .min_by_key(|(key, i)| (i.local_at, **key))
            .map(|(key, _)| *key)
    }
}

#[cfg(test)]
mod tests;
