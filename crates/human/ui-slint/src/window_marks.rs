// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The open conversation's window marks as drawn
//! (design/2026-10-27-start-window.md sections 5 to 8): the progress row,
//! shown once a page has taken 250 ms; the failure row and its Try again,
//! announced once as the edge fails; the limit row and its Show more
//! messages; the `EarlierUnread` and `ReadNotShown` lines. Each stands
//! where the model places it -- above everything, above a message or a
//! card row, or below everything -- and a press of a row's button leaves
//! as the window report it is. A pressed row stays, inert, until the
//! progress row or the page replaces it (section 5, "A pressed row").
//!
//! The view also reports what the person reaches: the conversation opened
//! and closed as it is shown and no longer shown (section 7.1's U5 rule),
//! the person's place -- the focused item and the items in view, which
//! the held-row ceiling never lets go -- and each edge reached or left by
//! scrolling it into view or by moving focus onto it. Focus the app places
//! reaches nothing. In the listed mode nothing is reported.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

use interweave_human_ui_model::{
    Anchor, ConversationKey, Edge, MarkKind, MarkPlace, UiModel, UiText, WindowEvent, WindowInput,
    WindowMark, by_count, fill, placeholder_en,
};
use slint::{Model as _, ModelRc, VecModel};

use crate::generated::{MarkKindRow, MarkRow, MessageRow};
use crate::{AppWindow, INPUT_CAP, ItemKey, Page, Shared, View};

/// How long a page is on its way before its progress row shows (section
/// 5): a quick page shows none, so nothing flashes.
pub(crate) const LOADING_SHOWN_AFTER: Duration = Duration::from_millis(250);

/// The marks' state between renders.
#[derive(Default)]
pub(crate) struct Marks {
    /// The marks this render draws: the model's, less a progress row whose
    /// page has not yet taken [`LOADING_SHOWN_AFTER`].
    pub(crate) now: Vec<WindowMark>,
    /// When each edge's page was first seen on its way.
    loading_since: HashMap<Edge, Instant>,
    /// The edges the model shows failed: a failure is announced as its
    /// edge enters this set, so a repeat after Try again is too.
    failed: HashSet<Edge>,
    /// Each edge whose failure or limit row was pressed, with that row,
    /// drawn inert in place of a progress row not shown yet.
    pressed: HashMap<Edge, MarkKind>,
    /// A failure to announce this render.
    announce: bool,
    /// The rows drawn, and the model the window reads them from.
    rows: Rc<VecModel<MarkRow>>,
    bound: bool,
    /// The model reads its rows a window at a time.
    windowed: bool,
    /// Where each edge a page can still be asked for stands.
    edges: Vec<(Edge, MarkPlace)>,
    /// The conversation reported opened.
    opened: Option<ConversationKey>,
    /// The edges reported reached and not yet left.
    reached: HashSet<Edge>,
    /// The person's place as last reported.
    place: Option<(Option<ItemKey>, Vec<ItemKey>)>,
    /// The focus the app last placed: focus there is not the person's.
    app_focus: RefCell<Option<String>>,
    /// For each mark a person can focus, the focus target of what stands
    /// just below it as last drawn: the next mark at its place, else the
    /// row the marks stand above (section 7's focus table).
    below: HashMap<String, String>,
}

impl Marks {
    /// The app placed focus on `target`.
    pub(crate) fn app_placed(&self, target: &str) {
        *self.app_focus.borrow_mut() = Some(target.to_owned());
    }

    /// Whether focus at `id` is where the app placed it. A message's
    /// target is `m:` and its handle, its focus id `item:` and the same.
    fn by_app(&self, id: &str) -> bool {
        self.app_focus.borrow().as_deref().is_some_and(|target| {
            target == id
                || target
                    .strip_prefix("m:")
                    .is_some_and(|h| id.strip_prefix("item:") == Some(h))
        })
    }
}

/// What the message list says of itself, between takes.
#[derive(Default)]
pub(crate) struct ListView {
    in_view: HashSet<i32>,
    at_top: bool,
    /// Which way it last moved: true toward older rows.
    moved_up: Option<bool>,
    /// The rows at an edge: one coming into view or leaving it wakes the
    /// root, so the edge is reported at once; any other waits for the
    /// next turn.
    edge_handles: HashSet<i32>,
}

fn wake(shared: &Rc<RefCell<Shared>>) {
    let wake = shared.borrow().wake.clone();
    if let Some(wake) = wake {
        wake();
    }
}

/// The marks' buttons, each press queued by edge, and the list's reports;
/// what can change an edge wakes the root.
pub(crate) fn wire(window: &AppWindow, shared: &Rc<RefCell<Shared>>) {
    let s = Rc::clone(shared);
    window.on_row_in_view(move |handle, shown| {
        let at_edge = {
            let mut shared = s.borrow_mut();
            let view = &mut shared.window_view;
            if shown {
                view.in_view.insert(handle);
            } else {
                view.in_view.remove(&handle);
            }
            view.edge_handles.contains(&handle)
        };
        if at_edge {
            wake(&s);
        }
    });
    let s = Rc::clone(shared);
    window.on_list_at_top(move |top| {
        s.borrow_mut().window_view.at_top = top;
        wake(&s);
    });
    let s = Rc::clone(shared);
    window.on_list_moved(move |up| {
        s.borrow_mut().window_view.moved_up = Some(up);
    });
    let s = Rc::clone(shared);
    window.on_focus_moved(move || wake(&s));
    let s = Rc::clone(shared);
    window.on_window_mark_pressed(move |edge| {
        let mut shared = s.borrow_mut();
        // As many as the inputs' cap: a held key repeats.
        if shared.window_presses.len() < INPUT_CAP {
            shared.window_presses.push(edge.to_string());
        }
        let wake = shared.wake.clone();
        drop(shared);
        if let Some(wake) = wake {
            wake();
        }
    });
}

/// An edge as the view hands it to the window and back.
pub(crate) fn edge_id(edge: Edge) -> String {
    match edge {
        Edge::Top => "t".to_owned(),
        Edge::GapUpper(id) => format!("u{id}"),
        Edge::GapLower(id) => format!("l{id}"),
    }
}

fn edge_of(id: &str) -> Option<Edge> {
    match id.split_at_checked(1)? {
        ("t", "") => Some(Edge::Top),
        ("u", n) => n.parse().ok().map(Edge::GapUpper),
        ("l", n) => n.parse().ok().map(Edge::GapLower),
        _ => None,
    }
}

const fn edge_of_kind(kind: MarkKind) -> Option<Edge> {
    match kind {
        MarkKind::Loading(edge)
        | MarkKind::Failed(edge)
        | MarkKind::AtLimit(edge)
        | MarkKind::EarlierUnread { edge, .. } => Some(edge),
        MarkKind::ReadNotShown { .. } => None,
    }
}

impl View {
    /// The marks the shown conversation draws this render, the progress
    /// row held back until its page has taken [`LOADING_SHOWN_AFTER`] --
    /// the root woken then -- and each message's count of the marks above
    /// it. Before the rows are set, so each row says it has marks.
    fn visible_marks(&mut self, model: &UiModel) -> Vec<WindowMark> {
        let marks = self
            .shown
            .as_ref()
            .map_or_else(Vec::new, |key| model.window_marks(key));
        let now = (self.monotonic)();
        let loading: HashSet<Edge> = marks
            .iter()
            .filter_map(|m| match m.kind {
                MarkKind::Loading(edge) => Some(edge),
                _ => None,
            })
            .collect();
        self.marks.loading_since.retain(|e, _| loading.contains(e));
        let mut due: Option<Duration> = None;
        let shown: Vec<WindowMark> = marks
            .into_iter()
            .filter(|m| {
                let MarkKind::Loading(edge) = m.kind else {
                    return true;
                };
                let since = *self.marks.loading_since.entry(edge).or_insert(now);
                let waited = now.saturating_duration_since(since);
                if waited >= LOADING_SHOWN_AFTER {
                    return true;
                }
                let left = LOADING_SHOWN_AFTER.saturating_sub(waited);
                due = Some(due.map_or(left, |d| d.min(left)));
                false
            })
            .collect();
        if let Some(due) = due {
            self.wake_after(due);
        }
        // A pressed row stays, inert, while its read is on its way and its
        // progress row not shown; the progress row, the page or another
        // answer -- the limit, a failure -- ends it.
        let mut shown = shown;
        self.marks.pressed.retain(|edge, kind| {
            if !loading.contains(edge) || shown.iter().any(|m| m.kind == MarkKind::Loading(*edge)) {
                return false;
            }
            let place = self
                .shown
                .as_ref()
                .and_then(|key| {
                    model
                        .window_marks(key)
                        .into_iter()
                        .find(|m| m.kind == MarkKind::Loading(*edge))
                })
                .map(|m| m.place);
            if let Some(place) = place {
                shown.push(WindowMark { place, kind: *kind });
            }
            true
        });
        let failed: HashSet<Edge> = shown
            .iter()
            .filter_map(|m| match m.kind {
                MarkKind::Failed(edge) if !self.marks.pressed.contains_key(&edge) => Some(edge),
                _ => None,
            })
            .collect();
        self.marks.announce |= failed.iter().any(|e| !self.marks.failed.contains(e));
        self.marks.failed = failed;
        self.marks.now.clone_from(&shown);
        self.marks.windowed = model.windowed();
        if self.window.get_windowed() != self.marks.windowed {
            self.window.set_windowed(self.marks.windowed);
        }
        self.marks.edges = self
            .shown
            .as_ref()
            .map_or_else(Vec::new, |key| model.window_edges(key));
        shown
    }

    /// The marks as the window draws them, once the message and card rows
    /// have their handles; the announcement of an edge that failed since
    /// the last render, said once while it stays failed.
    pub(crate) fn render_window(&mut self) -> Option<String> {
        let marks = self.marks.now.clone();
        let rows: Vec<MarkRow> = marks.iter().map(|mark| self.mark_row(mark)).collect();
        self.focus_below_a_mark_gone(&rows);
        let model = &self.marks.rows;
        if model.row_count() == rows.len() {
            // Row by row: a reset would drop a touch held in the list.
            for (i, row) in rows.iter().enumerate() {
                if model.row_data(i).as_ref() != Some(row) {
                    model.set_row_data(i, row.clone());
                }
            }
        } else {
            model.set_vec(rows);
        }
        let edge_handles: HashSet<i32> = self
            .marks
            .edges
            .clone()
            .into_iter()
            .filter_map(|(_, place)| self.row_of(place))
            .collect();
        self.shared.borrow_mut().window_view.edge_handles = edge_handles;
        if !self.marks.bound {
            self.marks.bound = true;
            self.window
                .set_window_marks(ModelRc::from(Rc::clone(&self.marks.rows)));
        }
        std::mem::take(&mut self.marks.announce)
            .then(|| placeholder_en::text(UiText::EarlierLoadFailed).to_owned())
    }

    /// Section 7: focus on a mark that goes -- the line whose count reached
    /// zero, a pressed row the progress row or its page replaced -- moves
    /// to what stood just below it, as app-placed focus, which reaches no
    /// edge; past that too, if it went in the same render. A limit row
    /// becoming the failure row keeps its button's focus key, and so its
    /// focus.
    fn focus_below_a_mark_gone(&mut self, rows: &[MarkRow]) {
        let drawn: HashSet<String> = rows
            .iter()
            .map(|r| r.focus_key.to_string())
            .filter(|k| !k.is_empty())
            .collect();
        let focus = self.window.get_focus_id().to_string();
        if !drawn.contains(&focus)
            && let Some(mut target) = self.marks.below.get(&focus).cloned()
        {
            for _ in 0..self.marks.below.len() {
                match self.marks.below.get(&target) {
                    Some(next) if !drawn.contains(&target) => target.clone_from(next),
                    _ => break,
                }
            }
            self.focus_key_after_draw(target);
        }
        let mut below = HashMap::new();
        for (i, row) in rows.iter().enumerate() {
            if row.focus_key.is_empty() {
                continue;
            }
            let next = rows[i + 1..]
                .iter()
                .take_while(|r| r.place == row.place && r.handle == row.handle)
                .find(|r| !r.focus_key.is_empty())
                .map(|r| r.focus_key.to_string())
                .or_else(|| self.anchor_target(row.place, row.handle));
            if let Some(next) = next {
                below.insert(row.focus_key.to_string(), next);
            }
        }
        self.marks.below = below;
    }

    /// The focus target of what marks at `place` stand above: the message
    /// `handle`, the card row `handle`'s first control, or at the top the
    /// first thing drawn; nothing at the end.
    fn anchor_target(&self, place: i32, handle: i32) -> Option<String> {
        let card = |handle: i32| {
            self.cards
                .drawn_rows
                .values()
                .find(|(h, _)| *h == handle)
                .map(|(h, answered)| {
                    if *answered {
                        format!("answer:{h}")
                    } else {
                        format!("proposal-copy:{h}")
                    }
                })
        };
        match place {
            1 => Some(format!("m:{handle}")),
            2 => card(handle),
            0 => {
                let leading = self
                    .shared
                    .borrow()
                    .order
                    .card_after
                    .iter()
                    .position(|after| *after < 0)
                    .and_then(|i| i32::try_from(i).ok());
                leading.and_then(card).or_else(|| {
                    self.message_keys
                        .first()
                        .and_then(|k| self.item_handles.handle_of(k))
                        .map(|h| format!("m:{h}"))
                })
            }
            _ => None,
        }
    }

    fn mark_row(&mut self, mark: &WindowMark) -> MarkRow {
        let (place, handle) = match mark.place {
            MarkPlace::Top => (0, -1),
            MarkPlace::Above(Anchor::Item(key)) => self
                .message_handle(key)
                .map_or((3, -1), |handle| (1, handle)),
            MarkPlace::Above(Anchor::Card(row)) => self
                .cards
                .drawn_rows
                .get(&row)
                .map_or((3, -1), |(handle, _)| (2, *handle)),
            MarkPlace::End => (3, -1),
        };
        let edge = edge_of_kind(mark.kind).map(edge_id).unwrap_or_default();
        let text = |t| placeholder_en::text(t).to_owned();
        let (kind, said, action, focus_key) = match mark.kind {
            MarkKind::Loading(_) => (
                MarkKindRow::Loading,
                text(UiText::EarlierLoading),
                String::new(),
                String::new(),
            ),
            MarkKind::Failed(_) => (
                MarkKindRow::Failed,
                text(UiText::EarlierLoadFailed),
                text(UiText::TryAgain),
                format!("mark-action:{edge}"),
            ),
            MarkKind::AtLimit(_) => (
                MarkKindRow::AtLimit,
                text(UiText::EarlierAtLimit),
                text(UiText::EarlierShowAnyway),
                format!("mark-action:{edge}"),
            ),
            MarkKind::EarlierUnread { count, .. } => (
                MarkKindRow::Line,
                fill(
                    placeholder_en::text(UiText::EarlierUnread),
                    &[("count", &count.to_string())],
                ),
                String::new(),
                format!("earlier-unread:{edge}"),
            ),
            MarkKind::ReadNotShown { count } => (
                MarkKindRow::Line,
                fill(
                    placeholder_en::text(by_count(
                        count,
                        UiText::ReadNotShownOne,
                        UiText::ReadNotShown,
                    )),
                    &[("count", &count.to_string())],
                ),
                String::new(),
                // Where it stands, so focus stays on it while its count
                // changes (section 7).
                format!("read-not-shown:{place}:{handle}"),
            ),
        };
        MarkRow {
            place,
            handle,
            kind,
            text: said.into(),
            action: action.into(),
            edge: edge.into(),
            focus_key: focus_key.into(),
        }
    }

    /// The message row whose coming into view reaches `place`: the message
    /// it stands above, or for a card row the message the card follows;
    /// `None` at the top, at the end, or before every message.
    fn row_of(&mut self, place: MarkPlace) -> Option<i32> {
        match place {
            MarkPlace::Above(Anchor::Item(key)) => self.message_handle(key),
            MarkPlace::Above(Anchor::Card(row)) => {
                let (handle, _) = *self.cards.drawn_rows.get(&row)?;
                let after = self
                    .shared
                    .borrow()
                    .order
                    .card_after
                    .get(usize::try_from(handle).ok()?)
                    .copied()?;
                (after >= 0).then_some(after)
            }
            MarkPlace::Top | MarkPlace::End => None,
        }
    }

    /// Whether `place` is in the list's view: the top while it shows its
    /// top, a row's place while the row is in view; a card before every
    /// message at the top.
    fn place_in_view(&mut self, place: MarkPlace) -> bool {
        let (at_top, in_view) = {
            let shared = self.shared.borrow();
            (
                shared.window_view.at_top,
                shared.window_view.in_view.clone(),
            )
        };
        match (place, self.row_of(place)) {
            (MarkPlace::Top, _) | (MarkPlace::Above(Anchor::Card(_)), None) => at_top,
            (_, Some(handle)) => in_view.contains(&handle),
            _ => false,
        }
    }

    /// The conversation whose window is open: the one shown while it is
    /// shown -- its Details and a contact page one of its proposals
    /// opened included -- and none in Chats on a narrow window, or with
    /// Trust, Peers or a card page in its place (section 7.1's U5).
    fn window_conversation(&self) -> Option<ConversationKey> {
        if !self.marks.windowed {
            return None;
        }
        let shown = self.shown.clone()?;
        if self.window.get_narrow() && self.window.get_list_shown() {
            return None;
        }
        match self.page {
            Page::Conversation | Page::Connect => Some(shown),
            Page::Cards
                if self
                    .cards
                    .return_to_link
                    .as_ref()
                    .is_some_and(|(key, ..)| *key == shown) =>
            {
                Some(shown)
            }
            Page::Cards | Page::Trust | Page::Peers => None,
        }
    }

    /// The opening and closing since the last call.
    fn open_events(&mut self) -> Vec<WindowEvent> {
        let want = self.window_conversation();
        if want == self.marks.opened {
            return Vec::new();
        }
        let mut events = Vec::new();
        if let Some(before) = self.marks.opened.take() {
            events.push(WindowEvent::Input(WindowInput::Closed(before)));
        }
        if let Some(now) = want.clone() {
            events.push(WindowEvent::Input(WindowInput::Opened(now)));
        }
        self.marks.opened = want;
        self.marks.reached.clear();
        self.marks.place = None;
        events
    }

    /// The person's place, when it changed, and each edge reached or left
    /// since the last call: by the list's view, or by the person moving
    /// focus onto the edge's row or one of its marks. Of a gap's two sides
    /// reached together, the one the person came from first: moving down
    /// the upper, otherwise the lower (ux-designer's U3), since the model
    /// asks the first and holds the second until the first page is in.
    fn reach_events(&mut self, conversation: &ConversationKey) -> Vec<WindowEvent> {
        let mut events = Vec::new();
        let focus_id = self.window.get_focus_id().to_string();
        let focused = focus_id
            .strip_prefix("item:")
            .and_then(|h| h.parse::<i32>().ok())
            .and_then(|h| self.item_handles.key_of(h).copied());
        let (in_view, moved_up) = {
            let shared = self.shared.borrow();
            (
                shared.window_view.in_view.clone(),
                shared.window_view.moved_up,
            )
        };
        let mut items: Vec<ItemKey> = in_view
            .iter()
            .filter_map(|h| self.item_handles.key_of(*h).copied())
            .collect();
        items.sort_unstable();
        let place = (focused, items);
        if self.marks.place.as_ref() != Some(&place) {
            events.push(WindowEvent::Input(WindowInput::Place {
                conversation: conversation.clone(),
                focused: place.0,
                in_view: place.1.clone(),
            }));
            self.marks.place = Some(place);
        }
        let by_person = !self.marks.by_app(&focus_id);
        let mut now = HashSet::new();
        for (edge, at) in self.marks.edges.clone() {
            let id = edge_id(edge);
            let focus_on = by_person
                && (focus_id == format!("earlier-unread:{id}")
                    || focus_id == format!("mark-action:{id}")
                    || matches!(at, MarkPlace::Above(Anchor::Item(k)) if focused == Some(k)));
            if focus_on || self.place_in_view(at) {
                now.insert(edge);
            }
        }
        let mut left: Vec<Edge> = self.marks.reached.difference(&now).copied().collect();
        let mut reached: Vec<Edge> = now.difference(&self.marks.reached).copied().collect();
        let upper_first = moved_up == Some(false);
        let order = |edge: &Edge| match edge {
            Edge::Top => (0, 0),
            Edge::GapUpper(id) => (*id + 1, u64::from(!upper_first)),
            Edge::GapLower(id) => (*id + 1, u64::from(upper_first)),
        };
        left.sort_by_key(order);
        reached.sort_by_key(order);
        events.extend(left.into_iter().map(|edge| {
            WindowEvent::Input(WindowInput::Left {
                conversation: conversation.clone(),
                edge,
            })
        }));
        events.extend(reached.into_iter().map(|edge| {
            WindowEvent::Input(WindowInput::Reached {
                conversation: conversation.clone(),
                edge,
            })
        }));
        self.marks.reached = now;
        events
    }

    /// Each message row's count of the window marks standing above it, so
    /// it draws them; before the rows are set.
    pub(crate) fn mark_rows(&mut self, model: &UiModel, keys: &[ItemKey], rows: &mut [MessageRow]) {
        let marks = self.visible_marks(model);
        for (row, key) in rows.iter_mut().zip(keys) {
            let above = marks
                .iter()
                .filter(|m| m.place == MarkPlace::Above(Anchor::Item(*key)))
                .count();
            row.marks_above = i32::try_from(above).unwrap_or(i32::MAX);
        }
    }

    /// The message rows set, the person's place kept: the anchor row named,
    /// and a render that adds rows above it told to the list, which keeps
    /// that row where it stands on screen while they are laid out.
    pub(crate) fn set_messages_keeping_place(&mut self, keys: &[ItemKey], rows: &[MessageRow]) {
        let anchor = self.place_anchor();
        let follow = self.rows_added_above(anchor, keys);
        crate::update_by_key(&*self.messages, &mut self.message_keys, keys, rows);
        if self.window.get_anchor_handle() != anchor {
            self.window.set_anchor_handle(anchor);
        }
        let seq = self.window.get_render_seq().wrapping_add(1);
        self.window.set_render_seq(seq);
        if follow {
            self.window.set_follow_seq(seq);
        }
    }

    /// The handle of the message row that holds the person's place, of
    /// those drawn before this render: the focused one, else the first in
    /// view; -1 for none, and in the listed mode. The list keeps that row
    /// where it is on screen as rows are added above it.
    fn place_anchor(&self) -> i32 {
        if !self.marks.windowed {
            return -1;
        }
        let before = &self.message_keys;
        let focus_id = self.window.get_focus_id();
        let in_view = self.shared.borrow().window_view.in_view.clone();
        let anchor = focus_id
            .strip_prefix("item:")
            .and_then(|h| h.parse::<i32>().ok())
            .and_then(|h| self.item_handles.key_of(h).copied())
            .filter(|k| before.contains(k))
            .or_else(|| {
                before.iter().copied().find(|k| {
                    self.item_handles
                        .handle_of(k)
                        .is_some_and(|h| in_view.contains(&h))
                })
            });
        anchor
            .and_then(|k| self.item_handles.handle_of(&k))
            .unwrap_or(-1)
    }

    /// Whether the rows about to be drawn, `keys`, add rows above the
    /// anchor row and none after the last row drawn: a page of older rows,
    /// or a gap's rows above the person's place. A message arriving at the
    /// end is not, and behaves as today (section 5).
    fn rows_added_above(&self, anchor: i32, keys: &[ItemKey]) -> bool {
        let Some(anchor) = self.item_handles.key_of(anchor) else {
            return false;
        };
        let before = &self.message_keys;
        match (
            before.iter().position(|k| k == anchor),
            keys.iter().position(|k| k == anchor),
        ) {
            (Some(was), Some(now)) => now > was && before.last() == keys.last(),
            _ => false,
        }
    }

    /// The handle of a message drawn now, `None` for one that is not.
    fn message_handle(&mut self, key: ItemKey) -> Option<i32> {
        self.message_keys
            .contains(&key)
            .then(|| self.item_handles.of(&key))
    }

    /// The window reports since the last call, for the root to apply: the
    /// conversation opened or closed, the person's place and the edges
    /// reached or left, and the presses of a failure row's Try again and a
    /// limit row's Show more messages. A press on a row no longer drawn,
    /// or on one already pressed whose read is on its way, reports nothing.
    pub fn take_window_events(&mut self) -> Vec<WindowEvent> {
        let mut events = self.open_events();
        let presses = std::mem::take(&mut self.shared.borrow_mut().window_presses);
        let Some(conversation) = self.marks.opened.clone() else {
            return events;
        };
        events.extend(self.reach_events(&conversation));
        for id in presses {
            let Some(edge) = edge_of(&id) else { continue };
            if self.marks.pressed.contains_key(&edge) {
                continue;
            }
            let Some(kind) =
                self.marks.now.iter().map(|m| m.kind).find(
                    |k| matches!(k, MarkKind::Failed(e) | MarkKind::AtLimit(e) if *e == edge),
                )
            else {
                continue;
            };
            let conversation = conversation.clone();
            events.push(WindowEvent::Input(match kind {
                MarkKind::Failed(_) => WindowInput::TryAgain { conversation, edge },
                _ => WindowInput::ShowAnyway { conversation, edge },
            }));
            self.marks.pressed.insert(edge, kind);
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::{Edge, edge_id, edge_of};

    #[test]
    fn an_edge_comes_back_as_it_was_handed_over() {
        for edge in [Edge::Top, Edge::GapUpper(7), Edge::GapLower(0)] {
            assert_eq!(edge_of(&edge_id(edge)), Some(edge));
        }
        assert_eq!(edge_of("x1"), None);
        assert_eq!(edge_of("t1"), None);
    }
}
