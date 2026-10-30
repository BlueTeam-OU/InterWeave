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

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

use interweave_human_ui_model::{
    Anchor, Edge, MarkKind, MarkPlace, UiModel, UiText, WindowEvent, WindowInput, WindowMark,
    by_count, fill, placeholder_en,
};
use slint::{Model as _, ModelRc, VecModel};

use crate::generated::{MarkKindRow, MarkRow};
use crate::{AppWindow, INPUT_CAP, ItemKey, Shared, View};

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
}

/// The marks' buttons: each press is queued by edge and the root woken.
pub(crate) fn wire(window: &AppWindow, shared: &Rc<RefCell<Shared>>) {
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
    pub(crate) fn visible_marks(&mut self, model: &UiModel) -> Vec<WindowMark> {
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
        shown
    }

    /// The marks as the window draws them, once the message and card rows
    /// have their handles; the announcement of an edge that failed since
    /// the last render, said once while it stays failed.
    pub(crate) fn render_window(&mut self) -> Option<String> {
        let marks = self.marks.now.clone();
        let rows: Vec<MarkRow> = marks.iter().map(|mark| self.mark_row(mark)).collect();
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
        if !self.marks.bound {
            self.marks.bound = true;
            self.window
                .set_window_marks(ModelRc::from(Rc::clone(&self.marks.rows)));
        }
        std::mem::take(&mut self.marks.announce)
            .then(|| placeholder_en::text(UiText::EarlierLoadFailed).to_owned())
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

    /// The handle of a message drawn now, `None` for one that is not.
    fn message_handle(&mut self, key: ItemKey) -> Option<i32> {
        self.message_keys
            .contains(&key)
            .then(|| self.item_handles.of(&key))
    }

    /// The window reports since the last call: the presses of a failure
    /// row's Try again and a limit row's Show more messages, for the
    /// conversation shown. A press on a row no longer drawn, or on one
    /// already pressed whose read is on its way, reports nothing.
    pub fn take_window_events(&mut self) -> Vec<WindowEvent> {
        let presses = std::mem::take(&mut self.shared.borrow_mut().window_presses);
        let Some(conversation) = self.shown.clone() else {
            return Vec::new();
        };
        let mut events = Vec::new();
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
