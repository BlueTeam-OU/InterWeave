// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identity switch, the switcher and the "As" line above Send (plan
//! §30 step 1; `ui/identities.slint`): the model's
//! [`interweave_human_ui_model::Identities`] drawn, and the person's acts
//! on them queued as [`IdentityIntent`]s, which the root takes with
//! [`View::take_identity_intents`] and applies with `Identities::act`.
//!
//! With fewer than two identities held the model answers `None` for
//! every part, so nothing here is drawn and nothing can be pressed
//! (ADR-0061 d. 3; `tests/identities.rs`,
//! `with_one_identity_nothing_of_the_switch_is_shown`).
//!
//! A press names a row by its place in the switcher as last drawn, and
//! is turned back into the identity drawn there: the rows a render drew
//! are the ones a press can name.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use interweave_human_ui_model::{
    IdentityFocus, IdentityIntent, IdentityKey, Said, SendControl, TryAgainControl, UiModel,
    UiText, cluster, placeholder_en,
};
use slint::{Model as _, ModelRc, SharedString, VecModel};

use crate::generated::{AppWindow, IdComposer, IdRow, IdSwitch, IdSwitcherView};
use crate::{Input, Shared, View, enqueue};

/// The most presses held between two takes: as many as the view's own
/// input queue ([`crate::INPUT_CAP`]); a press past it is refused
/// (`presses_past_the_cap_are_refused`).
pub(crate) const PRESS_CAP: usize = crate::INPUT_CAP;

/// The focus key of the switch, which a switch and a closed switcher
/// send focus to (identities §11, F2 and F3).
pub(crate) const SWITCH_FOCUS: &str = "identity-switch";

/// The send control's focus key and the composer field's (app.slint).
const SEND_FOCUS: &str = "send";
const COMPOSER_FOCUS: &str = "composer";

/// The presses waiting for the root.
#[derive(Debug, Default)]
pub(crate) struct Presses {
    queue: VecDeque<IdentityIntent>,
    refused: u64,
}

impl Presses {
    /// Queue `intent`, or refuse it at [`PRESS_CAP`].
    fn push(&mut self, intent: IdentityIntent) -> bool {
        if self.queue.len() >= PRESS_CAP {
            self.refused = self.refused.saturating_add(1);
            return false;
        }
        self.queue.push_back(intent);
        true
    }
}

/// What the identity views drew, and what their presses name.
pub(crate) struct IdentityViews {
    presses: Rc<RefCell<Presses>>,
    /// The switcher's rows as last drawn, by place.
    rows: Rc<RefCell<Vec<IdentityKey>>>,
    /// The identity whose composer line was last drawn, for its Try again.
    composer: Rc<RefCell<Option<IdentityKey>>>,
    /// The model's focus request last carried out.
    focus_seen: Option<u64>,
    /// The switch's snackbar whose time runs, by id.
    snack_timed: Option<u64>,
    /// The switch's snackbar last announced, by id.
    snack_announced: Option<u64>,
    /// The model's own snackbar as last seen, by id.
    model_snack_seen: Option<u64>,
    /// Send was held back by the identity's state at the last render.
    send_held: bool,
    /// The switcher's rows, one model for the view's life.
    switcher_rows: Rc<VecModel<IdRow>>,
    /// The switcher's view was set once, its rows model with it.
    switcher_set: bool,
}

/// Queue `intent` from a window callback and wake the root, after the
/// borrows are released, as the view's own presses do.
fn press(presses: &RefCell<Presses>, shared: &RefCell<Shared>, intent: IdentityIntent) {
    let queued = presses.borrow_mut().push(intent);
    let wake = shared.borrow().wake.clone();
    if queued && let Some(wake) = wake {
        wake();
    }
}

/// A text as a screen reader is given it (app.slint's `Drawn.spoken`).
fn spoken(text: &str) -> String {
    cluster::spoken(text, placeholder_en::text(UiText::ReplacedCharacter)).into_owned()
}

/// Sentences read one after another as one name: each ends in a full
/// stop unless it has its own.
fn sentences(parts: impl IntoIterator<Item = String>) -> String {
    let mut out = String::new();
    for part in parts {
        if !out.is_empty() {
            if !out.ends_with(['.', '!', '?']) {
                out.push('.');
            }
            out.push(' ');
        }
        out.push_str(&part);
    }
    out
}

fn try_again_parts(control: Option<TryAgainControl>) -> (SharedString, bool, bool) {
    control.map_or_else(
        || (SharedString::new(), false, false),
        |t| (placeholder_en::text(t.text).into(), t.enabled, t.progress),
    )
}

impl IdentityViews {
    /// Wire the identity views' callbacks on `window`.
    pub(crate) fn wire(window: &AppWindow, shared: &Rc<RefCell<Shared>>) -> Self {
        let presses = Rc::new(RefCell::new(Presses::default()));
        let rows: Rc<RefCell<Vec<IdentityKey>>> = Rc::new(RefCell::new(Vec::new()));
        let composer: Rc<RefCell<Option<IdentityKey>>> = Rc::new(RefCell::new(None));
        let on = |intent: IdentityIntent| {
            let (p, s) = (Rc::clone(&presses), Rc::clone(shared));
            move || press(&p, &s, intent.clone())
        };
        window.on_id_switch_pressed(on(IdentityIntent::OpenSwitcher));
        window.on_id_switcher_closed(on(IdentityIntent::CloseSwitcher));
        window.on_id_your_identities(on(IdentityIntent::OpenIdentities));
        let (p, s, r) = (Rc::clone(&presses), Rc::clone(shared), Rc::clone(&rows));
        window.on_id_row_chosen(move |at| {
            let key = usize::try_from(at)
                .ok()
                .and_then(|at| r.borrow().get(at).cloned());
            if let Some(key) = key {
                press(&p, &s, IdentityIntent::Choose(key));
            }
        });
        // A row's Try again by its place; the composer's line's at -1.
        let (p, s, r, c) = (
            Rc::clone(&presses),
            Rc::clone(shared),
            Rc::clone(&rows),
            Rc::clone(&composer),
        );
        window.on_id_try_again(move |at| {
            let key = if at < 0 {
                c.borrow().clone()
            } else {
                usize::try_from(at)
                    .ok()
                    .and_then(|at| r.borrow().get(at).cloned())
            };
            if let Some(key) = key {
                press(&p, &s, IdentityIntent::TryAgain(key));
            }
        });
        let (p, s) = (Rc::clone(&presses), Rc::clone(shared));
        window.on_id_snack_pressed(move |id| {
            if let Ok(id) = u64::try_from(id) {
                press(&p, &s, IdentityIntent::SwitchBack(id));
            }
        });
        let (p, s) = (Rc::clone(&presses), Rc::clone(shared));
        window.on_id_snack_closed(move |id| {
            if let Ok(id) = u64::try_from(id) {
                press(&p, &s, IdentityIntent::SnackbarClosed(id));
            }
        });
        Self {
            presses,
            rows,
            composer,
            focus_seen: None,
            snack_timed: None,
            snack_announced: None,
            model_snack_seen: None,
            send_held: false,
            switcher_rows: Rc::new(VecModel::default()),
            switcher_set: false,
        }
    }

    /// Draw the model's identities: the switch, the switcher, the switch's
    /// snackbar and the composer's line. Returns what to announce -- the
    /// switch's snackbar as it opens -- and where to move focus.
    pub(crate) fn render(
        &mut self,
        window: &AppWindow,
        model: &UiModel,
        shared: &Rc<RefCell<Shared>>,
    ) -> (Option<String>, Option<&'static str>) {
        let ids = model.identities();
        let mut focus = None;

        let switch = ids.switch_view();
        window.set_id_switch(switch.as_ref().map_or_else(IdSwitch::default, |v| {
            IdSwitch {
                shown: true,
                name: v.name.as_str().into(),
                least: v.least.as_str().into(),
                accessible: spoken(&sentences(v.accessible.iter().map(Said::english))).into(),
                badge: v.badge.is_some(),
                count: v
                    .badge
                    .filter(|b| b.unread > 0)
                    .map(|b| b.unread.to_string())
                    .unwrap_or_default()
                    .into(),
            }
        }));

        let switcher = ids.switcher().filter(|_| ids.switcher_open());
        *self.rows.borrow_mut() = switcher
            .as_ref()
            .map(|s| s.rows.iter().map(|r| r.key.clone()).collect())
            .unwrap_or_default();
        let rows: Vec<IdRow> = switcher.as_ref().map_or_else(Vec::new, |s| {
            s.rows
                .iter()
                .map(|r| {
                    let (try_again, try_enabled, try_progress) = try_again_parts(r.try_again);
                    let not_sent = r.not_sent.map(placeholder_en::text).unwrap_or_default();
                    let state = r.state.as_ref().map(Said::english).unwrap_or_default();
                    let description = sentences(
                        [
                            Some(r.second_line_label.english()),
                            r.unread_said.as_ref().map(Said::english),
                            (!not_sent.is_empty()).then(|| not_sent.to_owned()),
                            (!state.is_empty()).then(|| state.clone()),
                        ]
                        .into_iter()
                        .flatten(),
                    );
                    IdRow {
                        name: r.name.as_str().into(),
                        name_said: spoken(&r.name).into(),
                        second_line: r.second_line.as_str().into(),
                        description: spoken(&description).into(),
                        unread: if r.unread == 0 {
                            SharedString::new()
                        } else {
                            r.unread.to_string().into()
                        },
                        not_sent: not_sent.into(),
                        state: state.into(),
                        try_again,
                        try_enabled,
                        try_progress,
                        chosen: r.chosen,
                    }
                })
                .collect()
        });
        // In place, row by row: a row made again loses the focus it holds,
        // and a render while the switcher is open must not take it
        // (`tab_reaches_the_switch_and_every_stop_of_the_switcher`, whose
        // arrows render between their presses).
        let model_rows = &self.switcher_rows;
        for (at, row) in rows.iter().enumerate() {
            if at < model_rows.row_count() {
                if model_rows.row_data(at).as_ref() != Some(row) {
                    model_rows.set_row_data(at, row.clone());
                }
            } else {
                model_rows.push(row.clone());
            }
        }
        while model_rows.row_count() > rows.len() {
            model_rows.remove(model_rows.row_count() - 1);
        }
        let (open, title, identities) = switcher.as_ref().map_or_else(
            || (false, SharedString::new(), SharedString::new()),
            |s| {
                (
                    true,
                    placeholder_en::text(s.title).into(),
                    placeholder_en::text(s.identities).into(),
                )
            },
        );
        let drawn = window.get_id_switcher();
        if !self.switcher_set
            || drawn.open != open
            || drawn.title != title
            || drawn.identities != identities
        {
            self.switcher_set = true;
            window.set_id_switcher(IdSwitcherView {
                open,
                title,
                rows: ModelRc::from(Rc::clone(&self.switcher_rows)),
                identities,
            });
        }

        // The line above Send names the conversation's identity, which is
        // the one in front: only its conversations are shown (cards
        // §5.7.2).
        let line = ids.front().and_then(|k| ids.composer(k));
        *self.composer.borrow_mut() = line.as_ref().and(ids.front().cloned());
        let shown_line = line.as_ref().filter(|l| l.line.is_some());
        window.set_id_composer(shown_line.map_or_else(IdComposer::default, |l| {
            let text = l.line.as_ref().map(Said::english).unwrap_or_default();
            let (try_again, try_enabled, try_progress) = try_again_parts(l.try_again);
            IdComposer {
                shown: true,
                line_said: spoken(&text).into(),
                line: text.into(),
                try_again,
                try_enabled,
                try_progress,
            }
        }));
        let (send_name, held) = match line.as_ref().map(|l| &l.send) {
            Some(SendControl::Enabled { name }) => (name.as_ref().map(Said::english), false),
            Some(SendControl::Disabled { .. }) => (None, true),
            None => (None, false),
        };
        window.set_send_label(
            send_name
                .unwrap_or_else(|| placeholder_en::text(UiText::Send).to_owned())
                .into(),
        );
        window.set_id_send_held(held);
        // Send held back while it has focus: focus to the composer's field,
        // which stays (identities §5.2, F21).
        if held && !self.send_held && window.get_focus_id() == SEND_FOCUS {
            focus = Some(COMPOSER_FOCUS);
        }
        self.send_held = held;

        // The switch's snackbar: one at a time with the model's own. A
        // switch closes the one up before it (cards §5.7.3), and a later
        // one of the model's takes its place.
        let model_snack = model.snackbar().map(|s| s.id);
        let model_snack_new = model_snack.is_some() && model_snack != self.model_snack_seen;
        self.model_snack_seen = model_snack;
        let mut snack = ids.switch_snackbar();
        let snack_new = snack.is_some_and(|s| self.snack_announced != Some(s.id));
        if let Some(s) = snack
            && model_snack_new
            && !snack_new
        {
            press(&self.presses, shared, IdentityIntent::SnackbarClosed(s.id));
            snack = None;
        }
        let mut said = None;
        if let Some(s) = snack {
            if snack_new {
                self.snack_announced = Some(s.id);
                said = Some(s.snackbar.said.english());
                if let Some(id) = model_snack {
                    enqueue(shared, Input::SnackClosed(id));
                }
            }
            if self.snack_timed != Some(s.id) {
                self.snack_timed = Some(s.id);
                let (presses, shared, id) = (Rc::clone(&self.presses), Rc::clone(shared), s.id);
                let ms = shared.borrow().touch.snackbar_ms;
                slint::Timer::single_shot(Duration::from_millis(ms), move || {
                    press(&presses, &shared, IdentityIntent::SnackbarClosed(id));
                });
            }
        }
        window.set_id_snack_shown(snack.is_some());
        if let Some(s) = snack {
            window.set_id_snack_id(i32::try_from(s.id).unwrap_or(i32::MAX));
            window.set_id_snack_text(s.snackbar.said.english().into());
            window.set_id_snack_action(placeholder_en::text(s.snackbar.action).into());
        }

        // The model's focus request, once: the switcher's title takes focus
        // as it opens on its own (F1), so only the switch is asked here.
        if let Some((seq, to)) = ids.focus_request()
            && self.focus_seen != Some(seq)
        {
            self.focus_seen = Some(seq);
            if to == IdentityFocus::Switch && switch.is_some() {
                focus = Some(SWITCH_FOCUS);
            }
        }
        (said, focus)
    }
}

impl View {
    /// The person's acts on the identity switch, the switcher, its
    /// snackbar and the composer's Try again since the last take, in
    /// order, for the root to apply (`Identities::act`) before the next
    /// render.
    pub fn take_identity_intents(&mut self) -> Vec<IdentityIntent> {
        self.identities
            .presses
            .borrow_mut()
            .queue
            .drain(..)
            .collect()
    }

    /// How many identity presses were refused at the cap since the view
    /// was made.
    #[must_use]
    pub fn refused_identity_intents(&self) -> u64 {
        self.identities.presses.borrow().refused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presses_past_the_cap_are_refused() {
        let mut presses = Presses::default();
        for _ in 0..PRESS_CAP {
            assert!(presses.push(IdentityIntent::OpenSwitcher));
        }
        assert!(!presses.push(IdentityIntent::CloseSwitcher));
        assert_eq!(presses.queue.len(), PRESS_CAP);
        assert_eq!(presses.refused, 1);
        assert_eq!(
            presses.queue.back(),
            Some(&IdentityIntent::OpenSwitcher),
            "the refused press is not queued"
        );
    }

    #[test]
    fn sentences_are_joined_with_one_full_stop() {
        assert_eq!(
            sentences([
                "Identity: A. Switch identity".to_owned(),
                "2 unread".to_owned()
            ]),
            "Identity: A. Switch identity. 2 unread"
        );
        assert_eq!(sentences(["One.".to_owned(), "Two".to_owned()]), "One. Two");
        assert_eq!(sentences(["Alone".to_owned()]), "Alone");
    }
}
