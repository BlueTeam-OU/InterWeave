// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The open conversation's window (design/2026-10-27-start-window.md
//! sections 4 to 6): its rows read from the store a page at a time, newest
//! first, older pages fetched when the person reaches the top edge, and the
//! line that counts the unread rows not loaded yet. The windowed mode only:
//! a model given a whole listing holds every row, as it always has.
//!
//! The model decides what to ask and when; the facade reads the store.
//! Asks leave through [`UiModel::take_page_asks`], answers come back
//! through [`UiModel::page_arrived`] and [`UiModel::page_failed`]. What a
//! view draws at the edge -- the progress row, the failure row, the line --
//! is [`UiModel::window_marks`].

use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};

use interweave_human_core::RowId;

use super::cards::Proposal;
use super::cards::READ_PROPOSAL_CAP;
use super::{ConversationKey, ItemKey, ListedInbound, SESSION_ITEM_CAP, Table, UiModel};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// A page the model asks the facade for: the rows of `conversation`
/// strictly between the bounds, in `order`. `unread_room`, when set, is
/// how many unread rows the page may hold (the session bound, design
/// section 7.1); `seq` names the opening it is for, so a page that comes
/// back after the conversation was closed or opened again adds nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
    /// The item the person moved focus to, if any: the session bound never
    /// drops it.
    Place {
        /// The conversation.
        conversation: ConversationKey,
        /// The focused item.
        focused: Option<ItemKey>,
    },
}

/// Something a view draws at an edge, anchored above an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowMark {
    /// The item it stands directly above; `None` when the conversation
    /// holds no item.
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
    /// `EarlierUnread`: `count` unread rows the store holds above it that
    /// are not loaded.
    EarlierUnread {
        /// The edge it stands at.
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

/// An edge's state.
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

/// The open conversation's window.
#[derive(Debug)]
struct Window {
    conversation: ConversationKey,
    seq: u64,
    /// The oldest loaded row's place, once a page brought one.
    top: Option<PageMark>,
    top_state: EdgeState,
    /// The person is at the top edge.
    at_top: bool,
    /// The person chose Show more messages for this opening.
    show_anyway: bool,
    /// The item the person moved focus to.
    focused: Option<ItemKey>,
    /// Each run of the conversation's rows the session bound dropped, by
    /// the item it stands directly above (`None`: below every item).
    runs: BTreeMap<Option<ItemKey>, u64>,
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
    fn ask_top(&mut self, unread_room: Option<usize>) {
        let Some(window) = self.open.as_mut() else {
            return;
        };
        window.top_state = EdgeState::InFlight;
        self.asks.push_back(PageAsk {
            conversation: window.conversation.clone(),
            edge: Edge::Top,
            older_than: window.top,
            newer_than: None,
            order: PageOrder::NewestFirst,
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
        let windows = &mut self.windows;
        match input {
            WindowInput::Opened(conversation) => {
                // One conversation is open at a time: the one before is
                // closed, its stored rows let go.
                if let Some(before) = windows.open.take() {
                    self.let_go(&before.conversation);
                    self.cards.let_go_unread(&before.conversation);
                }
                let windows = &mut self.windows;
                windows.next_seq += 1;
                windows.open = Some(Window {
                    conversation,
                    seq: windows.next_seq,
                    top: None,
                    top_state: EdgeState::Armed,
                    at_top: false,
                    show_anyway: false,
                    focused: None,
                    runs: BTreeMap::new(),
                });
                // The opening reads its window at once, not covered by the
                // limit row (sections 4 and 7.1).
                windows.ask_top(None);
            }
            WindowInput::Closed(conversation) => {
                if windows.open_for(&conversation).is_some() {
                    windows.open = None;
                    // A closed conversation holds none of its stored rows
                    // (section 3): they stay in the store for its next
                    // opening.
                    self.let_go(&conversation);
                    self.cards.let_go_unread(&conversation);
                }
            }
            WindowInput::Reached {
                conversation,
                edge: Edge::Top,
            } => {
                let Some(window) = windows.open_for(&conversation) else {
                    return;
                };
                window.at_top = true;
                // Once each time the edge is reached, never while a page is
                // on its way, never after a failure: Try again retries. At
                // the limit it is judged again: room may have come.
                if matches!(window.top_state, EdgeState::Armed | EdgeState::AtLimit) {
                    self.reach_top();
                }
            }
            WindowInput::Left {
                conversation,
                edge: Edge::Top,
            } => {
                let Some(window) = windows.open_for(&conversation) else {
                    return;
                };
                window.at_top = false;
                if window.top_state == EdgeState::Spent {
                    window.top_state = EdgeState::Armed;
                }
            }
            WindowInput::ShowAnyway {
                conversation,
                edge: Edge::Top,
            } => {
                if let Some(window) = windows.open_for(&conversation)
                    && window.top_state == EdgeState::AtLimit
                {
                    window.show_anyway = true;
                    windows.ask_top(None);
                }
            }
            WindowInput::Place {
                conversation,
                focused,
            } => {
                if let Some(window) = windows.open_for(&conversation) {
                    window.focused = focused;
                }
            }
            WindowInput::TryAgain {
                conversation,
                edge: Edge::Top,
            } => {
                if windows
                    .open_for(&conversation)
                    .is_some_and(|w| w.top_state == EdgeState::Failed)
                {
                    self.reach_top();
                }
            }
        }
    }

    /// The pages asked since the last call, for the facade to read.
    pub fn take_page_asks(&mut self) -> Vec<PageAsk> {
        self.windows.asks.drain(..).collect()
    }

    /// A page the facade read: its rows are held, the top edge moves to
    /// the oldest of them, and the edge waits to be reached again. A page
    /// for an opening that is over -- the conversation closed or opened
    /// again -- adds nothing: its rows stay in the store
    /// (`a_page_for_an_earlier_opening_adds_nothing`).
    pub fn page_arrived(&mut self, page: PageRows) {
        let current = self
            .windows
            .open
            .as_ref()
            .is_some_and(|w| w.conversation == page.ask.conversation && w.seq == page.ask.seq);
        if !current {
            return;
        }
        let mut oldest: Option<PageMark> = None;
        let mut mark = |m: PageMark| {
            if oldest.is_none_or(|o| m < o) {
                oldest = Some(m);
            }
        };
        for row in &page.unread {
            mark(PageMark {
                received_at: row.received_at,
                kept: false,
                row: row.row,
            });
        }
        for row in &page.kept {
            mark(PageMark {
                received_at: row.received_at,
                kept: true,
                row: row.row,
            });
        }
        for proposal in &page.proposals {
            mark(PageMark {
                received_at: proposal.received_at,
                kept: false,
                row: proposal.row,
            });
        }
        for row in page.unread {
            self.unread_listed(vec![row]);
        }
        self.kept_listed(page.kept);
        for proposal in page.proposals {
            self.card_received(proposal);
        }
        let Some(window) = self.windows.open.as_mut() else {
            return;
        };
        if let Some(oldest) = oldest
            && window.top.is_none_or(|t| oldest < t)
        {
            window.top = Some(oldest);
        }
        window.top_state = match page.end {
            PageEnd::Exhausted => EdgeState::Exhausted,
            PageEnd::RoomUsed if !window.show_anyway => EdgeState::AtLimit,
            PageEnd::More | PageEnd::RoomUsed if window.at_top => EdgeState::Spent,
            PageEnd::More | PageEnd::RoomUsed => EdgeState::Armed,
        };
    }

    /// The person reached the top edge with it armed, at the limit, or
    /// after Try again: ask the next page, within the room the session has
    /// left -- or, at the session bound with unread rows beyond the edge,
    /// show the limit row and ask nothing, until the person chooses Show
    /// more messages (design section 7.1). With nothing unread beyond the
    /// edge, a page makes no row session-only, so it is asked whatever the
    /// room.
    fn reach_top(&mut self) {
        let Some(window) = self.windows.open.as_ref() else {
            return;
        };
        if window.show_anyway {
            self.windows.ask_top(None);
            return;
        }
        let conversation = window.conversation.clone();
        let room = self.page_room(&conversation);
        if room == 0 && self.unread_not_loaded(&conversation) > 0 {
            if let Some(window) = self.windows.open.as_mut() {
                window.top_state = EdgeState::AtLimit;
            }
            return;
        }
        self.windows.ask_top(Some(room));
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

    /// The page asked for `ask` could not be read: the edge shows the
    /// failure row, and only Try again asks again.
    pub fn page_failed(&mut self, ask: &PageAsk) {
        if let Some(window) = self.windows.open.as_mut()
            && window.conversation == ask.conversation
            && window.seq == ask.seq
        {
            window.top_state = EdgeState::Failed;
        }
    }

    /// What a view draws at the open conversation's edges, each above the
    /// item it stands over: the progress or failure row at the top, then
    /// `EarlierUnread` with the count of the unread rows the store holds
    /// that are not loaded (design section 6). Nothing for a conversation
    /// that is not open.
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
        let above = self.oldest_item(conversation);
        let mut marks = Vec::new();
        match window.top_state {
            EdgeState::InFlight => marks.push(WindowMark {
                above,
                kind: MarkKind::Loading(Edge::Top),
            }),
            EdgeState::Failed => marks.push(WindowMark {
                above,
                kind: MarkKind::Failed(Edge::Top),
            }),
            EdgeState::AtLimit => marks.push(WindowMark {
                above,
                kind: MarkKind::AtLimit(Edge::Top),
            }),
            EdgeState::Armed | EdgeState::Spent | EdgeState::Exhausted => {}
        }
        let count = self.unread_not_loaded(conversation);
        if count > 0 && window.top_state != EdgeState::Exhausted {
            marks.push(WindowMark {
                above,
                kind: MarkKind::EarlierUnread {
                    edge: Edge::Top,
                    count,
                },
            });
        }
        for (below, count) in &window.runs {
            marks.push(WindowMark {
                above: *below,
                kind: MarkKind::ReadNotShown { count: *count },
            });
        }
        marks
    }

    /// The unread rows of `conversation` the store holds and the model
    /// does not: the store's count less the unread rows held, messages and
    /// cards alike, one per row as the store counts them.
    fn unread_not_loaded(&self, conversation: &ConversationKey) -> u64 {
        let Some(stored) = self.windowed.as_ref().map(|w| w.unread_of(conversation)) else {
            return 0;
        };
        let (messages, cards) = self.held_unread(conversation);
        let held = u64::try_from(messages + cards).unwrap_or(u64::MAX);
        stored.saturating_sub(held)
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
