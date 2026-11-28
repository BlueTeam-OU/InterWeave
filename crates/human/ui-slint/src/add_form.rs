// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The add form, drawn (`design/2026-11-04-identities.md` §6.2, §11's
//! add form row and F5, F7, F8, F24-F27; plan §30): ui-model's
//! [`AddForm`] as a card page, and what the person does on it queued as
//! [`AddIntent`]s, which the root takes with [`View::take_add_intents`]
//! and carries out on the one form open (app-core's model side). Nothing
//! here decides: whether Create runs, whether leaving asks, what shows in
//! Create's place are the model's, and this draws what it says.
//!
//! The fields' text is the window's, which the person edits: it is set
//! only when a form opens, never while the person types, and each change
//! goes to the model as `Edited`.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use interweave_human_ui_model::{
    AddFocus, AddForm, AddFormView, AddIntent, PROGRESS_AFTER_MS, Said, UiModel, UiText,
    placeholder_en,
};
use slint::{ComponentHandle as _, SharedString};

use crate::generated::{AddFormShown, AppWindow};
use crate::{Shared, View};

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

/// The most acts held between two takes: as many as the view's own input
/// queue; one past it is refused (`acts_past_the_cap_are_refused`).
const ACT_CAP: usize = crate::INPUT_CAP;

/// The acts waiting for the root.
#[derive(Debug, Default)]
pub(crate) struct Acts {
    queue: VecDeque<AddIntent>,
    refused: u64,
}

impl Acts {
    fn push(&mut self, intent: AddIntent) -> bool {
        if self.queue.len() >= ACT_CAP {
            self.refused = self.refused.saturating_add(1);
            return false;
        }
        self.queue.push_back(intent);
        true
    }
}

/// The focus key of each control of the form a focus request can name.
const fn focus_key(at: AddFocus) -> Option<&'static str> {
    match at {
        AddFocus::Name => Some("add-name"),
        AddFocus::Surname => Some("add-surname"),
        AddFocus::Create => Some("add-create"),
        AddFocus::LineAbove => Some("add-line-above"),
        AddFocus::InPlace => Some("add-in-place"),
        AddFocus::TryAgain => Some("add-try-again"),
        AddFocus::Other => None,
    }
}

/// The control a focus key names, as the model is told it.
fn focus_of(key: &str) -> AddFocus {
    match key {
        "add-name" => AddFocus::Name,
        "add-surname" => AddFocus::Surname,
        "add-create" => AddFocus::Create,
        "add-line-above" => AddFocus::LineAbove,
        "add-in-place" => AddFocus::InPlace,
        "add-try-again" => AddFocus::TryAgain,
        _ => AddFocus::Other,
    }
}

/// The form's acts and what it last drew.
#[derive(Default)]
pub(crate) struct AddForms {
    acts: Rc<RefCell<Acts>>,
    /// The form as last drawn, so the window is set only on a change.
    drawn: Option<AddFormView>,
    /// A form is shown: its fields were emptied when it opened.
    open: bool,
    /// The form's focus request last carried out.
    focus_seen: Option<u64>,
    /// `CardDiscardChanges` as last drawn.
    asking: bool,
}

impl AddForms {
    /// Queue `intent` and wake the root.
    pub(crate) fn queue(&self, shared: &RefCell<Shared>, intent: AddIntent) {
        queue(&self.acts, shared, intent);
    }
}

fn queue(acts: &RefCell<Acts>, shared: &RefCell<Shared>, intent: AddIntent) {
    let queued = acts.borrow_mut().push(intent);
    let wake = shared.borrow().wake.clone();
    if queued && let Some(wake) = wake {
        wake();
    }
}

/// Wire the form's callbacks on `window`.
pub(crate) fn wire(window: &AppWindow, shared: &Rc<RefCell<Shared>>, forms: &AddForms) {
    let on = |intent: AddIntent| {
        let (acts, shared) = (Rc::clone(&forms.acts), Rc::clone(shared));
        move || queue(&acts, &shared, intent.clone())
    };
    window.on_add_create(on(AddIntent::Create));
    window.on_add_try_again(on(AddIntent::TryAgain));
    window.on_add_remove_photo(on(AddIntent::RemovePhoto));
    window.on_add_leave(on(AddIntent::Leave));
    let (acts, s) = (Rc::clone(&forms.acts), Rc::clone(shared));
    window.on_add_discard(move |close| queue(&acts, &s, AddIntent::Discard(close)));
    let (acts, s) = (Rc::clone(&forms.acts), Rc::clone(shared));
    window.on_add_focused(move |key| queue(&acts, &s, AddIntent::Focused(focus_of(&key))));
    let (acts, s) = (Rc::clone(&forms.acts), Rc::clone(shared));
    let weak = window.as_weak();
    window.on_add_edited(move || {
        if let Some(w) = weak.upgrade() {
            let intent = AddIntent::Edited {
                name: w.get_add_name().to_string(),
                surname: w.get_add_surname().to_string(),
            };
            queue(&acts, &s, intent);
        }
    });
    window.set_add_progress_after(i32::try_from(PROGRESS_AFTER_MS).unwrap_or(i32::MAX));
}

/// What the form draws in place of `view`.
fn shown(view: &AddFormView) -> AddFormShown {
    let said = |s: Option<&Said>| s.map(Said::english).unwrap_or_default();
    let create = view.create.as_ref();
    let in_place = view.in_place.as_ref();
    let try_again = in_place.and_then(|p| p.try_again);
    AddFormShown {
        title: text(view.title).into(),
        back: text(view.back).into(),
        note: text(view.note).into(),
        inert: view.fields_inert,
        has_photo: view.photo.is_some(),
        photo: view
            .photo
            .as_ref()
            .map_or_else(crate::cards::no_photo, crate::cards::image),
        circle: text(match (view.photo_line, &view.photo) {
            (Some(UiText::CardPhotoPreparing), _) => UiText::CardPhotoPreparing,
            (_, Some(_)) => UiText::CardPhotoLabel,
            (_, None) => UiText::CardPhotoNone,
        })
        .into(),
        photo_preparing: view.photo_line == Some(UiText::CardPhotoPreparing),
        photo_unusable: if view.photo_line == Some(UiText::PhotoUnusable) {
            text(UiText::PhotoUnusable)
        } else {
            ""
        }
        .into(),
        name_error: view.name_problem.map_or("", text).into(),
        surname_error: view.surname_problem.map_or("", text).into(),
        line_above: said(view.line_above.as_ref()).into(),
        line_error: view
            .line_above
            .as_ref()
            .is_some_and(|l| l.text == UiText::IdentityAddFailed),
        create_shown: create.is_some(),
        create_label: create.map_or("", |c| text(c.text)).into(),
        create_name: said(create.and_then(|c| c.name.as_ref())).into(),
        // Inert with no progress yet: the seed is asked, and the window
        // shows the progress once its threshold passes.
        create_checking: create
            .is_some_and(|c| c.name.as_ref().map(|n| n.text) == Some(UiText::IdentityAddChecking)),
        create_running: create
            .is_some_and(|c| c.name.as_ref().map(|n| n.text) == Some(UiText::IdentityCreating)),
        in_place: said(in_place.map(|p| &p.said)).into(),
        try_again: try_again.map_or("", |t| text(t.text)).into(),
        try_enabled: try_again.is_some_and(|t| t.enabled),
        try_progress: try_again.is_some_and(|t| t.progress),
    }
}

impl View {
    /// What the person did on the add form since the last take, in order,
    /// for the root to carry out on the form open.
    pub fn take_add_intents(&mut self) -> Vec<AddIntent> {
        self.add_forms.acts.borrow_mut().queue.drain(..).collect()
    }

    /// How many add form acts were refused at the cap since the view was
    /// made.
    #[must_use]
    pub fn refused_add_intents(&self) -> u64 {
        self.add_forms.acts.borrow().refused
    }

    /// Draw the form the model holds, if it is the card page shown; the
    /// form's focus request is carried out once, after the draw.
    pub(crate) fn render_add_form(&mut self, model: &UiModel) {
        let window = self.window.clone_strong();
        let shown_page = self.page == crate::Page::Cards
            && self.cards.page == Some(crate::generated::CardsPage::AddIdentity);
        let form: Option<&AddForm> = model.identities().add_form().filter(|_| shown_page);
        let Some(form) = form else {
            if std::mem::take(&mut self.add_forms.open) {
                self.add_forms.drawn = None;
                self.add_forms.asking = false;
                window.set_own_discard_open(false);
            }
            return;
        };
        if !std::mem::replace(&mut self.add_forms.open, true) {
            // A form opens empty (§6.2): its fields are set once, here.
            window.set_add_name(SharedString::new());
            window.set_add_surname(SharedString::new());
            // Each form counts its focus requests from one.
            self.add_forms.focus_seen = None;
        }
        // The time only decides an inert Create's progress, which the
        // window times itself from what is drawn here.
        let view = form.view(0);
        if self.add_forms.drawn.as_ref() != Some(&view) {
            window.set_add_form(shown(&view));
            window.set_discard_title(text(UiText::CardDiscardChanges).into());
            window.set_keep_editing_label(text(UiText::KeepEditing).into());
            window.set_discard_label(text(UiText::DiscardChanges).into());
            window.set_add_photo_label(text(UiText::AddPhoto).into());
            window.set_change_photo_label(text(UiText::ChangePhoto).into());
            window.set_remove_photo_label(text(UiText::RemovePhoto).into());
            window.set_name_label(text(UiText::CardName).into());
            window.set_surname_label(text(UiText::CardSurname).into());
            window.set_own_photo_choosable(self.photo_choosable());
            let asking = view.discard.is_some();
            if asking != self.add_forms.asking {
                self.add_forms.asking = asking;
                window.set_own_discard_open(asking);
            }
            self.add_forms.drawn = Some(view);
        }
        if let Some((seq, at)) = form.focus_request()
            && self.add_forms.focus_seen != Some(seq)
        {
            self.add_forms.focus_seen = Some(seq);
            if let Some(key) = focus_key(at) {
                self.focus_key_after_draw(key.to_owned());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acts_past_the_cap_are_refused() {
        let mut acts = Acts::default();
        for _ in 0..ACT_CAP {
            assert!(acts.push(AddIntent::Create));
        }
        assert!(!acts.push(AddIntent::Leave));
        assert_eq!(acts.queue.len(), ACT_CAP);
        assert_eq!(acts.refused, 1);
        assert_eq!(acts.queue.back(), Some(&AddIntent::Create));
    }

    #[test]
    fn every_focus_key_names_its_control_back() {
        for at in [
            AddFocus::Name,
            AddFocus::Surname,
            AddFocus::Create,
            AddFocus::LineAbove,
            AddFocus::InPlace,
            AddFocus::TryAgain,
        ] {
            let key = focus_key(at).expect("a key");
            assert_eq!(focus_of(key), at);
        }
        assert_eq!(focus_of("close-cards"), AddFocus::Other);
    }
}
