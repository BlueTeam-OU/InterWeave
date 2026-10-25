// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Contact cards, drawn (plan section 29 step 4;
//! `design/2026-10-21-contact-cards.md`): a conversation's proposals, the
//! Contacts list, a contact's page, the own card's page, the share sheet
//! and Details' card line, from `ui-model`'s cards. Every word is a
//! label's; every press goes back through the queue as a [`CardsInput`],
//! judged against the model when it is taken.
//!
//! What a proposal shows of its photo is only that there is one: the
//! model holds a received photo's bytes undecoded until Accept, and a
//! `ProposalItem` exposes no more than that it exists, so no image of it
//! is drawn
//! (`a_proposal_names_the_claim_beside_the_full_peer_id_and_draws_no_image_before_accept`). A received name is drawn inside an isolate wherever it
//! sits inside other text ([`isolated`]).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use interweave_human_ui_model::{
    AcceptLine, Avatar, AvatarHue, CardAnswer, ContactView, ConversationKey, FocusTarget, Intent,
    KeyProblem, KeyStep, KeyView, Photo, PhotoChange, PhotoEdit, ProposalItem, ProposalNote, RowId,
    UiModel, UiText, fill, isolated, name_problem, placeholder_en, short_peer,
};

use interweave_transport_api::EndpointId;
use slint::{
    ComponentHandle as _, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel,
};

use crate::generated::{
    AppWindow, CardsPage, ContactListRow, ContactRouteRow, ProposalNoteRow, ProposalRow,
    ShareAgainRow, SwipeKind,
};
use crate::{Input, Shared, View, enqueue};

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

/// What the person did on a card screen, as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CardsInput {
    /// A proposal's Accept or Decline: the model's own intent for the row
    /// as it was drawn, offered again when taken or dropped.
    Answer(Intent),
    /// The same answer by a touch swipe (design section 5.4.4), which
    /// leaves focus where it was unless it was inside the row (swipe spec
    /// section 7).
    Swiped(Intent),
    /// Contacts or the own card opened from the app's options.
    Open(Opened),
    /// The card page closed: Back, Escape, or the back arrow.
    Close,
    /// A contact opened from the list, by its id.
    Contact(String),
    /// A contact's name saved: the contact, and the field as typed.
    SaveName(String, String),
    /// A contact's photo removed.
    RemoveContactPhoto(String),
    /// A contact deleted, once its confirmation was pressed.
    DeleteContact(String),
    /// The own card saved: the two fields as typed.
    SaveOwn(String, String),
    /// The own card's photo removed.
    RemoveOwnPhoto,
    /// The share sheet's Share, for the conversation it was opened on.
    Share(ConversationKey),
    /// The identity's key block: Try again.
    KeyTryAgain,
    /// The identity's key block: Make a new key.
    KeyMakeNew,
    /// The new key's confirmation: Make a new key.
    KeyConfirm,
    /// The new key's confirmation closed: Cancel, Escape, Back, the scrim.
    KeyCancel,
    /// A conversation to share the card in again, by its row's handle.
    ShareAgain(i32),
    /// The share sheet's Open your card, while the key cannot be used.
    OpenCardFromShare,
}

/// Which card page the app's options opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Opened {
    /// Contacts.
    Contacts,
    /// The own card.
    OwnCard,
}

/// What the callbacks resolve against: the rows and the page as drawn.
#[derive(Default)]
pub(crate) struct Drawn {
    /// Each drawn proposal's Accept and Decline, by its handle.
    answers: HashMap<i32, (Intent, Intent)>,
    /// Each drawn contact row's id, by its handle.
    contacts: HashMap<i32, String>,
    /// The contact whose page is drawn.
    contact: Option<String>,
    /// The conversation the share sheet can share for.
    shareable: Option<ConversationKey>,
}

/// The card screens' callbacks: each becomes an input, taken as the others
/// are.
pub(crate) fn wire(window: &AppWindow, shared: &Rc<RefCell<Shared>>) {
    let on = |s: &Rc<RefCell<Shared>>, input: CardsInput| {
        let s = Rc::clone(s);
        move || enqueue(&s, Input::Cards(input.clone()))
    };
    window.on_open_contacts(on(shared, CardsInput::Open(Opened::Contacts)));
    window.on_open_own_card(on(shared, CardsInput::Open(Opened::OwnCard)));
    window.on_close_cards(on(shared, CardsInput::Close));
    window.on_own_card_remove_photo(on(shared, CardsInput::RemoveOwnPhoto));
    let s = Rc::clone(shared);
    window.on_proposal_accept(move |handle| {
        let answer = s.borrow().cards.answers.get(&handle).map(|a| a.0.clone());
        if let Some(intent) = answer {
            enqueue(&s, Input::Cards(CardsInput::Answer(intent)));
        }
    });
    let s = Rc::clone(shared);
    window.on_proposal_decline(move |handle| {
        let answer = s.borrow().cards.answers.get(&handle).map(|a| a.1.clone());
        if let Some(intent) = answer {
            enqueue(&s, Input::Cards(CardsInput::Answer(intent)));
        }
    });
    let s = Rc::clone(shared);
    window.on_contact_chosen(move |handle| {
        let id = s.borrow().cards.contacts.get(&handle).cloned();
        if let Some(id) = id {
            enqueue(&s, Input::Cards(CardsInput::Contact(id)));
        }
    });
    let s = Rc::clone(shared);
    let w = window.as_weak();
    window.on_contact_save(move || {
        let id = s.borrow().cards.contact.clone();
        if let (Some(id), Some(w)) = (id, w.upgrade()) {
            let typed = w.get_contact_name_draft().to_string();
            enqueue(&s, Input::Cards(CardsInput::SaveName(id, typed)));
        }
    });
    let s = Rc::clone(shared);
    window.on_contact_remove_photo(move || {
        let id = s.borrow().cards.contact.clone();
        if let Some(id) = id {
            enqueue(&s, Input::Cards(CardsInput::RemoveContactPhoto(id)));
        }
    });
    let s = Rc::clone(shared);
    window.on_contact_delete(move || {
        let id = s.borrow().cards.contact.clone();
        if let Some(id) = id {
            enqueue(&s, Input::Cards(CardsInput::DeleteContact(id)));
        }
    });
    let s = Rc::clone(shared);
    let w = window.as_weak();
    window.on_own_card_save(move || {
        if let Some(w) = w.upgrade() {
            let input = CardsInput::SaveOwn(
                w.get_own_given().to_string(),
                w.get_own_surname().to_string(),
            );
            enqueue(&s, Input::Cards(input));
        }
    });
    window.on_own_key_try_again(on(shared, CardsInput::KeyTryAgain));
    window.on_own_key_make_new(on(shared, CardsInput::KeyMakeNew));
    window.on_new_key_confirm(on(shared, CardsInput::KeyConfirm));
    window.on_new_key_cancel(on(shared, CardsInput::KeyCancel));
    window.on_share_open_card(on(shared, CardsInput::OpenCardFromShare));
    let s = Rc::clone(shared);
    window.on_own_key_share_again_chosen(move |handle| {
        enqueue(&s, Input::Cards(CardsInput::ShareAgain(handle)));
    });
    let s = Rc::clone(shared);
    window.on_share_confirmed(move || {
        let key = s.borrow().cards.shareable.clone();
        if let Some(key) = key {
            enqueue(&s, Input::Cards(CardsInput::Share(key)));
        }
    });
}

/// A key problem's reason line (design/2026-10-24-identity-key.md section
/// 3's table).
fn key_reason(problem: &KeyProblem) -> &'static str {
    text(match problem {
        KeyProblem::LostOnDevice => UiText::KeyLostDevice,
        KeyProblem::BelowDevice => UiText::KeyBelowDevice,
        KeyProblem::OtherDevice => UiText::KeyOtherDevice,
        KeyProblem::NewerVersion => UiText::KeyNewerVersion,
        KeyProblem::Damaged => UiText::KeyDamaged,
        KeyProblem::Exposed => UiText::KeyExposed,
        KeyProblem::DirUnsafe { .. } => UiText::KeyDirUnsafe,
        KeyProblem::NotNow => UiText::KeyNotNow,
        KeyProblem::Missing => UiText::KeyMissing,
    })
}

/// A name's initials, as the look spec draws an avatar without a photo:
/// the first character of its first word and of its last, uppercased.
fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let first = words.first().and_then(|w| w.chars().next());
    let last = (words.len() > 1)
        .then(|| words.last().and_then(|w| w.chars().next()))
        .flatten();
    first
        .into_iter()
        .chain(last)
        .flat_map(char::to_uppercase)
        .collect()
}

const fn hue(hue: AvatarHue) -> i32 {
    match hue {
        AvatarHue::Teal => 0,
        AvatarHue::Amber => 1,
        AvatarHue::Navy => 2,
    }
}

/// A contact's hue: the one its first route's `PeerId` already picks, so a
/// contact keeps its colour when it gains a name.
fn contact_hue(contact: &ContactView) -> i32 {
    contact.routes.first().map_or(0, |r| {
        hue(Avatar::of(&ConversationKey::Direct {
            peer: r.peer.clone(),
            endpoint: Some(r.endpoint.clone()),
        })
        .hue)
    })
}

fn image(photo: &Photo) -> Image {
    Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        &photo.rgba,
        photo.width,
        photo.height,
    ))
}

/// A proposal as its row draws it; `received` is the line saying when it
/// came.
fn proposal_row(handle: i32, p: &ProposalItem, received: &str) -> ProposalRow {
    let name = isolated(&p.name);
    let swipe = p.swipe();
    let swipe_key = proposal_swipe_key(p.row);
    let context = match &p.accept {
        AcceptLine::Creates => text(UiText::CardWillCreate).to_owned(),
        AcceptLine::Updates(contact) => {
            fill(text(UiText::CardUpdates), &[("name", &isolated(contact))])
        }
        AcceptLine::Matches(contact) => {
            fill(text(UiText::CardSame), &[("contact", &isolated(contact))])
        }
    };
    // What Accept changes and which of the person's edits it keeps, right
    // after the context line (design section 5.4.1, cases U and E).
    let contact = match &p.accept {
        AcceptLine::Updates(c) | AcceptLine::Matches(c) => c.as_str(),
        AcceptLine::Creates => "",
    };
    let values = &p.values;
    let value_lines = [
        values
            .new_name
            .as_ref()
            .map(|name| fill(text(UiText::CardUpdatesName), &[("name", &isolated(name))])),
        values
            .kept_name
            .then(|| fill(text(UiText::CardKeepsEdit), &[("name", &isolated(contact))])),
        match values.photo {
            PhotoChange::Unchanged => None,
            PhotoChange::New => Some(text(UiText::CardUpdatesPhoto).to_owned()),
            PhotoChange::Removed => Some(text(UiText::CardUpdatesPhotoRemoved).to_owned()),
            PhotoChange::Kept => Some(text(UiText::CardKeepsPhoto).to_owned()),
        },
    ]
    .into_iter()
    .flatten()
    .map(|line| ProposalNoteRow {
        text: line.into(),
        warning: false,
    });
    let notes: Vec<ProposalNoteRow> = value_lines
        .chain(p.notes.iter().map(|n| {
            let (template, contact, warning) = match n {
                ProposalNote::SameName(c) => (UiText::CardSameName, c, true),
                ProposalNote::SharesNode(c) => (UiText::CardSharesNode, c, false),
                ProposalNote::KeyMatches(c) => (UiText::CardKeyMatches, c, false),
            };
            ProposalNoteRow {
                text: fill(text(template), &[("contact", &isolated(contact))]).into(),
                warning,
            }
        }))
        .collect();
    let key_change = match &p.key {
        KeyView::Changed { recorded } => fill(
            text(UiText::CardKeyChange),
            &[("old", recorded), ("new", &p.fingerprint)],
        ),
        KeyView::New | KeyView::Same => String::new(),
    };
    ProposalRow {
        handle,
        heading: fill(text(UiText::CardProposal), &[("name", &name)]).into(),
        full_name: name.into(),
        photo_line: if p.has_photo {
            text(UiText::CardIncludesPhoto).into()
        } else {
            SharedString::new()
        },
        key_change: key_change.into(),
        peer_label: text(UiText::DetailsPeerId).into(),
        peer: p.peer.as_str().into(),
        fingerprint_label: text(UiText::Fingerprint).into(),
        fingerprint: p.fingerprint.as_str().into(),
        context: context.into(),
        notes: ModelRc::new(VecModel::from(notes)),
        // Nothing is lost by closing a card that changes nothing.
        goes: if matches!(p.accept, AcceptLine::Matches(_)) {
            SharedString::new()
        } else {
            text(UiText::CardGoesAtClose).into()
        },
        received: received.into(),
        closes: matches!(p.accept, AcceptLine::Matches(_)),
        accept: text(p.accept_text()).into(),
        decline: text(UiText::CardDecline).into(),
        swipe_key: swipe_key.into(),
        end_kind: if swipe.accept {
            SwipeKind::Accept
        } else {
            SwipeKind::None
        },
        end_label: text(p.accept_text()).into(),
        start_kind: if swipe.decline {
            SwipeKind::Decline
        } else {
            SwipeKind::None
        },
        start_label: text(UiText::CardDecline).into(),
        copy: text(UiText::CopyPeerId).into(),
        answered: false,
        answer: SharedString::new(),
        photo_note: SharedString::new(),
        hint: SharedString::new(),
    }
}

/// A proposal row's swipe key: by its card's row, which a re-render
/// keeps, where its handle is its place in the list.
fn proposal_swipe_key(row: RowId) -> String {
    format!("p:{}", row.get())
}

/// The confirmation row an Accept left in its proposal's place (design
/// section 5.4.2): `name` is the contact's name as it stands.
fn answer_row(handle: i32, a: &CardAnswer, name: &str, hinted: bool) -> ProposalRow {
    let template = if a.created {
        UiText::CardAccepted
    } else {
        UiText::CardUpdated
    };
    ProposalRow {
        handle,
        answered: true,
        answer: fill(text(template), &[("name", &isolated(name))]).into(),
        photo_note: match (a.photo_dropped, a.photo_removed) {
            (true, true) => text(UiText::CardPhotoNotUsedRemoved).into(),
            (true, false) => text(UiText::CardPhotoNotUsed).into(),
            (false, _) => SharedString::new(),
        },
        hint: if hinted {
            text(UiText::CardUndoHint).into()
        } else {
            SharedString::new()
        },
        ..ProposalRow::default()
    }
}

/// What the proposals list draws, in the conversation's time order: a
/// proposal, or the confirmation row of one accepted, with the contact's
/// name as it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    Proposal(ProposalItem),
    /// The answer, its contact's name, and whether its Accept's snackbar
    /// is open (its Undo hint read with it).
    Answer(CardAnswer, String, bool),
}

impl Entry {
    fn place(&self) -> (u64, RowId) {
        match self {
            Self::Proposal(p) => (p.received_at, p.row),
            Self::Answer(a, _, _) => (a.received_at, a.row),
        }
    }
}

/// The view's card state: the page open and what was last drawn, so a
/// list is replaced only when what it shows changed (a fresh model would
/// rebuild a focused button and drop its focus).
#[derive(Default)]
pub(crate) struct CardScreens {
    pub(crate) page: Option<CardsPage>,
    contact: Option<String>,
    proposals_shown: Vec<Entry>,
    proposals: Option<Rc<VecModel<ProposalRow>>>,
    /// Each drawn row's handle and whether it is a confirmation row, by
    /// the card's row: where [`FocusTarget::Proposal`] moves focus.
    ///
    /// [`FocusTarget::Proposal`]: interweave_human_ui_model::FocusTarget::Proposal
    pub(crate) drawn_rows: HashMap<RowId, (i32, bool)>,
    /// The proposal whose own Accept or Decline was pressed last: its
    /// answer moves focus, as a message row's own button does.
    pub(crate) pressed: Option<RowId>,
    contacts_shown: Vec<ContactView>,
    contact_rows: Option<Rc<VecModel<ContactListRow>>>,
    contact_drawn: Option<ContactView>,
    own_drawn: Option<(String, Option<String>, Option<Photo>)>,
    contact_error: String,
    own_errors: (String, String),
    /// The key step as last drawn: what a change of it moves focus to.
    key_step_drawn: Option<KeyStep>,
    /// Try again pressed for this problem, until its answer is drawn.
    retrying: Option<KeyProblem>,
    /// The share-again rows as drawn, by handle.
    share_again: Vec<ConversationKey>,
    /// The conversation whose share sheet opened the card page: Back
    /// returns to its sheet (design/2026-10-24-identity-key.md section
    /// 4.3).
    return_to_share: Option<ConversationKey>,
    /// A share-again row chosen: its conversation's sheet opens once it is
    /// on screen.
    share_on_open: Option<ConversationKey>,
}

impl View {
    /// Take one card input: an intent for the model, or a page change.
    pub(crate) fn take_cards(&mut self, model: &UiModel, input: CardsInput) -> Vec<Intent> {
        let window = self.window.clone_strong();
        let by_button = matches!(input, CardsInput::Answer(_));
        match input {
            CardsInput::Answer(intent) | CardsInput::Swiped(intent) => {
                let offered = match &intent {
                    Intent::AcceptCard(row) | Intent::DeclineCard(row) => {
                        // A swipe is judged again here, on the row as the
                        // model holds it now: a card judged again into the
                        // same-name warning since the finger lifted accepts
                        // nothing (design section 5.4.4, #433 round 3 F1;
                        // `a_swiped_accept_is_refused_once_the_row_is_warned`).
                        let swipe_allows = by_button
                            || self
                                .shown
                                .as_ref()
                                .and_then(|key| {
                                    model.proposals(key).into_iter().find(|p| p.row == *row)
                                })
                                .is_some_and(|p| {
                                    let swipe = p.swipe();
                                    if matches!(intent, Intent::AcceptCard(_)) {
                                        swipe.accept
                                    } else {
                                        swipe.decline
                                    }
                                });
                        let offered = swipe_allows && model.card_actions(*row).contains(&intent);
                        // The row's own button: its answer moves focus
                        // whatever the focus id last said (swipe spec 7).
                        // A swipe claims nothing: focus moves only if it
                        // was inside the row.
                        if offered && by_button {
                            self.cards.pressed = Some(*row);
                        }
                        offered
                    }
                    _ => false,
                };
                return offered.then_some(intent).into_iter().collect();
            }
            CardsInput::KeyTryAgain => {
                self.cards.retrying = model.key_problem().cloned();
                return model.retry_key().into_iter().collect();
            }
            CardsInput::KeyMakeNew => return vec![Intent::AskNewKey],
            CardsInput::KeyConfirm => return vec![Intent::MakeNewKey],
            CardsInput::KeyCancel => return vec![Intent::CloseKeyStep],
            CardsInput::ShareAgain(handle) => {
                let key = usize::try_from(handle)
                    .ok()
                    .and_then(|i| self.cards.share_again.get(i))
                    .cloned();
                if let Some(key) = key {
                    self.close_cards(&window);
                    self.cards.share_on_open = Some(key.clone());
                    enqueue(&self.shared, Input::Select(key));
                    return vec![Intent::CloseKeyStep];
                }
            }
            CardsInput::OpenCardFromShare => {
                self.cards.return_to_share = self.shown.clone();
                return self.take_cards(model, CardsInput::Open(Opened::OwnCard));
            }
            CardsInput::Open(opened) => {
                let page = match opened {
                    Opened::Contacts => CardsPage::Contacts,
                    Opened::OwnCard => CardsPage::OwnCard,
                };
                self.page = crate::Page::Cards;
                self.cards.page = Some(page);
                self.cards.contact = None;
                self.cards.contact_drawn = None;
                self.cards.own_drawn = None;
                self.cards.own_errors = (String::new(), String::new());
                window.set_cards_page(page);
                self.focus_key_after_draw("close-cards".to_owned());
            }
            CardsInput::Close => {
                let own = self.cards.page == Some(CardsPage::OwnCard);
                self.close_cards(&window);
                // Back from the card page to the share sheet that opened it.
                if let Some(key) = self.cards.return_to_share.take()
                    && self.shown.as_ref() == Some(&key)
                {
                    window.set_share_open(true);
                }
                if own {
                    return vec![Intent::CloseKeyStep];
                }
            }
            CardsInput::Contact(id) => {
                if model.contact(&id).is_some() {
                    self.cards.page = Some(CardsPage::Contact);
                    self.cards.contact = Some(id);
                    self.cards.contact_drawn = None;
                    self.cards.contact_error.clear();
                    window.set_cards_page(CardsPage::Contact);
                    self.focus_key_after_draw("close-cards".to_owned());
                }
            }
            CardsInput::SaveName(id, typed) => {
                if model.contact(&id).is_none() {
                    return Vec::new();
                }
                let name = typed.trim().to_owned();
                if let Some(problem) = name_problem(&name) {
                    text(problem.text()).clone_into(&mut self.cards.contact_error);
                    self.focus_key("contact-name".to_owned());
                    return Vec::new();
                }
                self.cards.contact_error.clear();
                return vec![Intent::RenameContact {
                    contact_id: id,
                    name,
                }];
            }
            CardsInput::RemoveContactPhoto(id) => {
                if model.contact(&id).is_some_and(|c| c.photo.is_some()) {
                    return vec![Intent::SetContactPhoto {
                        contact_id: id,
                        photo: PhotoEdit::Remove,
                    }];
                }
            }
            CardsInput::DeleteContact(id) => {
                if let Some(intent) = model.delete_contact(&id) {
                    self.cards.page = Some(CardsPage::Contacts);
                    self.cards.contact = None;
                    window.set_cards_page(CardsPage::Contacts);
                    self.focus_key_after_draw("close-cards".to_owned());
                    return vec![intent];
                }
            }
            CardsInput::SaveOwn(given, surname) => {
                let given = given.trim().to_owned();
                let surname = surname.trim().to_owned();
                let given_problem = name_problem(&given);
                let surname_problem = (!surname.is_empty())
                    .then(|| name_problem(&surname))
                    .flatten();
                self.cards.own_errors = (
                    given_problem.map_or_else(String::new, |p| text(p.text()).to_owned()),
                    surname_problem.map_or_else(String::new, |p| text(p.text()).to_owned()),
                );
                if given_problem.is_some() || surname_problem.is_some() {
                    let first = if given_problem.is_some() {
                        "own-given"
                    } else {
                        "own-surname"
                    };
                    self.focus_key(first.to_owned());
                    return Vec::new();
                }
                if model.identity().is_none() {
                    return Vec::new();
                }
                self.close_cards(&window);
                return vec![Intent::SetOwnCard {
                    name: given,
                    surname: (!surname.is_empty()).then_some(surname),
                    photo: PhotoEdit::Keep,
                }];
            }
            CardsInput::RemoveOwnPhoto => {
                if let Some(own) = model.identity().filter(|i| i.has_photo) {
                    return vec![Intent::SetOwnCard {
                        name: own.given.clone(),
                        surname: own.surname.clone(),
                        photo: PhotoEdit::Remove,
                    }];
                }
            }
            CardsInput::Share(key) => {
                // Shared only for the conversation still shown, by the
                // model's own offer: never a channel, never on its own.
                if self.shown.as_ref() == Some(&key) {
                    return model.share_card(&key).into_iter().collect();
                }
            }
        }
        Vec::new()
    }

    fn close_cards(&mut self, window: &AppWindow) {
        self.page = crate::Page::Conversation;
        self.cards.page = None;
        self.cards.contact = None;
        window.set_cards_page(CardsPage::None);
        window.set_contact_confirming_delete(false);
    }

    /// The focus key a contact's focus target names on the card page
    /// drawn, if it is drawn: the Contacts list's first row, else the
    /// page's back arrow, its empty text and its title being no focus
    /// stop; a contact's row; a contact page's back arrow; its Remove
    /// photo.
    pub(crate) fn contact_focus(&self, focus: &FocusTarget) -> Option<String> {
        let shared = self.shared.borrow();
        let drawn = &shared.cards;
        let page = self.cards.page;
        match focus {
            FocusTarget::Contacts => (page == Some(CardsPage::Contacts)).then(|| {
                drawn
                    .contacts
                    .keys()
                    .min()
                    .map_or_else(|| "close-cards".to_owned(), |h| format!("contact:{h}"))
            }),
            FocusTarget::Contact(id) => (page == Some(CardsPage::Contacts))
                .then(|| drawn.contacts.iter().find(|(_, c)| *c == id))
                .flatten()
                .map(|(h, _)| format!("contact:{h}")),
            FocusTarget::ContactPage(id) => (page == Some(CardsPage::Contact)
                && self.cards.contact.as_ref() == Some(id))
            .then(|| "close-cards".to_owned()),
            FocusTarget::ContactRemovePhoto(id) => (page == Some(CardsPage::Contact)
                && self.cards.contact.as_ref() == Some(id))
            .then(|| "contact-remove-photo".to_owned()),
            _ => None,
        }
    }

    /// Draw the card screens from `model`.
    pub(crate) fn render_cards(&mut self, model: &UiModel) -> Option<String> {
        let window = self.window.clone_strong();
        // Another page took the pane -- the trust settings, Peers, a
        // conversation picked from the list -- so no card page is open.
        if self.page != crate::Page::Cards && self.cards.page.is_some() {
            self.cards.page = None;
            self.cards.contact = None;
            window.set_cards_page(CardsPage::None);
            window.set_contact_confirming_delete(false);
        }
        self.render_proposals(model, &window);
        // A contact's page whose contact is gone shows the list.
        if self.cards.page == Some(CardsPage::Contact)
            && self
                .cards
                .contact
                .as_ref()
                .is_none_or(|id| model.contact(id).is_none())
        {
            self.cards.page = Some(CardsPage::Contacts);
            self.cards.contact = None;
            window.set_cards_page(CardsPage::Contacts);
        }
        self.render_contacts(model, &window);
        self.render_contact(model, &window);
        self.render_own(model, &window);
        let said = self.render_key(model, &window);
        self.render_share(model, &window);
        if let Some(key) = self.cards.share_on_open.take() {
            if self.shown.as_ref() == Some(&key) && self.page == crate::Page::Conversation {
                window.set_share_open(true);
            } else {
                self.cards.share_on_open = Some(key);
            }
        }
        said
    }

    /// The identity's key on its card page (design/2026-10-24-identity-
    /// key.md sections 4.1, 5 and 6): the block in the fingerprint's place
    /// while it cannot be used, the confirmation, the step under way, and
    /// what it came to; focus moves as section 8's table says, and the
    /// step and its result are announced once.
    fn render_key(&mut self, model: &UiModel, window: &AppWindow) -> Option<String> {
        let problem = model.key_problem().cloned();
        let step = model.key_step().cloned();
        window.set_own_key_problem(problem.is_some());
        window.set_own_key_title(text(UiText::KeyStateTitle).into());
        window.set_own_key_cannot_share(text(UiText::KeyCannotShare).into());
        window.set_own_key_try_label(text(UiText::TryAgain).into());
        window.set_own_key_new_label(text(UiText::KeyMakeNew).into());
        window.set_own_key_making_label(text(UiText::KeyMaking).into());
        let (label, _) = text(UiText::KeyDirUnsafePath)
            .split_once("{path}")
            .unwrap_or((text(UiText::KeyDirUnsafePath), ""));
        window.set_own_key_path_label(label.trim_end().into());
        window.set_own_key_reason(problem.as_ref().map_or("", key_reason).into());
        window.set_own_key_path(
            match &problem {
                Some(KeyProblem::DirUnsafe { folder }) => folder.as_str(),
                _ => "",
            }
            .into(),
        );
        let making = matches!(step, Some(KeyStep::Making | KeyStep::Asking));
        window.set_own_key_making(matches!(step, Some(KeyStep::Making)));
        window.set_own_key_try_shown(
            !making && problem.as_ref().is_some_and(KeyProblem::offers_try_again),
        );
        window.set_own_key_new_shown(
            !making && problem.as_ref().is_some_and(KeyProblem::offers_new_key),
        );
        window.set_own_key_failure(
            match &step {
                Some(KeyStep::Failed { gone: true }) => text(UiText::KeyNewFailedGone),
                Some(KeyStep::Failed { gone: false }) => text(UiText::KeyNewFailed),
                Some(KeyStep::StillNotNow) => text(UiText::KeyNotNowAgain),
                _ => "",
            }
            .into(),
        );
        // Made: the line, and where to share the card again.
        let again = match &step {
            Some(KeyStep::Made { share_again }) => share_again.clone(),
            _ => Vec::new(),
        };
        window.set_own_key_made(
            if matches!(step, Some(KeyStep::Made { .. })) {
                text(UiText::KeyMade)
            } else {
                ""
            }
            .into(),
        );
        window.set_own_key_share_again_title(text(UiText::KeyShareAgainTitle).into());
        if again != self.cards.share_again {
            let rows: Vec<ShareAgainRow> = again
                .iter()
                .enumerate()
                .map(|(i, k)| ShareAgainRow {
                    handle: i32::try_from(i).unwrap_or(i32::MAX),
                    title: isolated(&model.drawn_title(k)).into(),
                })
                .collect();
            window.set_own_key_share_again(ModelRc::new(VecModel::from(rows)));
            self.cards.share_again = again;
        }
        // The confirmation.
        let identity = model.identity().map_or_else(String::new, |own| {
            if own.name.is_empty() {
                own.endpoint.as_str().to_owned()
            } else {
                own.name.clone()
            }
        });
        let pending = match &step {
            Some(KeyStep::Confirming { pending }) => Some(*pending),
            _ => None,
        };
        window.set_new_key_open(pending.is_some());
        window.set_new_key_title(
            fill(
                text(UiText::KeyNewTitle),
                &[("identity", &isolated(&identity))],
            )
            .into(),
        );
        window.set_new_key_body(text(UiText::KeyNewBody).into());
        let newer = matches!(problem, Some(KeyProblem::NewerVersion));
        window.set_new_key_old(
            match &problem {
                Some(KeyProblem::NewerVersion) => text(UiText::KeyNewOldNewer),
                Some(p) if p.deletes_old_file() => text(UiText::KeyNewOld),
                _ => "",
            }
            .into(),
        );
        window.set_new_key_pending(
            if pending == Some(true) {
                text(UiText::KeyNewPending)
            } else {
                ""
            }
            .into(),
        );
        window.set_new_key_confirm_label(text(UiText::KeyNewConfirm).into());
        window.set_new_key_danger(newer);
        // Focus and announcements, once per change of step (section 8).
        let before = std::mem::replace(&mut self.cards.key_step_drawn, step.clone());
        let mut said = None;
        if before != step {
            match (&before, &step) {
                (Some(KeyStep::Confirming { .. }), None) => {
                    self.focus_key_after_draw("own-key-new".to_owned());
                }
                (_, Some(KeyStep::Making)) => {
                    self.focus_key_after_draw("own-key-making".to_owned());
                    said = Some(text(UiText::KeyMaking).to_owned());
                }
                (_, Some(KeyStep::Made { .. })) => {
                    self.focus_key_after_draw("own-key-fingerprint".to_owned());
                    said = Some(text(UiText::KeyMade).to_owned());
                }
                (_, Some(KeyStep::Failed { .. } | KeyStep::StillNotNow)) => {
                    self.focus_key_after_draw("own-key-failure".to_owned());
                }
                _ => {}
            }
        }
        // Try again's answer: opened, focus on the fingerprint row.
        if let Some(tried) = self.cards.retrying.clone()
            && problem.as_ref() != Some(&tried)
        {
            self.cards.retrying = None;
            if problem.is_none() {
                self.focus_key_after_draw("own-key-fingerprint".to_owned());
            }
        } else if matches!(step, Some(KeyStep::StillNotNow)) {
            self.cards.retrying = None;
        }
        said
    }

    /// When a card came, as its row says it: the date and the time.
    fn received(&self, at_ms: u64) -> String {
        fill(
            text(UiText::CardReceivedAt),
            &[
                ("date", &self.clock.date_text(at_ms)),
                ("time", &self.clock.time_text(at_ms)),
            ],
        )
    }

    fn render_proposals(&mut self, model: &UiModel, window: &AppWindow) {
        // The card whose Accept's snackbar is open, with its Undo.
        let undo_open_for = model
            .snackbar()
            .filter(|s| s.undo && crate::swipe::is_accept(s.confirms))
            .and_then(|s| s.card);
        let mut shown: Vec<Entry> = match (&self.page, &self.shown) {
            (crate::Page::Conversation, Some(key)) => model
                .proposals(key)
                .into_iter()
                .map(Entry::Proposal)
                .chain(model.card_answers(key).into_iter().map(|a| {
                    let name = model
                        .contact(&a.contact_id)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    let hinted = undo_open_for == Some(a.row);
                    Entry::Answer(a, name, hinted)
                }))
                .collect(),
            _ => Vec::new(),
        };
        shown.sort_by_key(Entry::place);
        let rows = Rc::clone(
            self.cards
                .proposals
                .get_or_insert_with(|| Rc::new(VecModel::default())),
        );
        if shown != self.cards.proposals_shown {
            let mut answers = HashMap::new();
            let mut swipes = HashMap::new();
            let mut handles = HashMap::new();
            let drawn: Vec<ProposalRow> = shown
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let handle = i32::try_from(i).unwrap_or(i32::MAX);
                    let p = match entry {
                        Entry::Answer(a, name, hinted) => {
                            handles.insert(a.row, (handle, true));
                            return answer_row(handle, a, name, *hinted);
                        }
                        Entry::Proposal(p) => p,
                    };
                    handles.insert(p.row, (handle, false));
                    // Case S offers Decline alone, which the row's one
                    // button, Close, raises whichever callback it takes.
                    match model.card_actions(p.row).as_slice() {
                        [accept, decline] => {
                            answers.insert(handle, (accept.clone(), decline.clone()));
                            // Its swipes raise the same intents, each way
                            // its case offers (design section 5.4.4).
                            let swipe = p.swipe();
                            if swipe.accept || swipe.decline {
                                swipes.insert(
                                    proposal_swipe_key(p.row),
                                    crate::swipe::Swipeable::Proposal {
                                        accept: swipe.accept.then(|| accept.clone()),
                                        decline: swipe.decline.then(|| decline.clone()),
                                    },
                                );
                            }
                        }
                        [only] => {
                            answers.insert(handle, (only.clone(), only.clone()));
                        }
                        _ => {}
                    }
                    proposal_row(handle, p, &self.received(p.received_at))
                })
                .collect();
            // A proposal's open menu shows the answers its row had: a row
            // changed, moved or gone closes it (message-swipe section 7),
            // so no entry acts on another card at its place (#433 F2).
            let menu = window.get_card_menu();
            if menu >= 0 {
                let was = usize::try_from(menu)
                    .ok()
                    .and_then(|i| self.cards.proposals_shown.get(i));
                let now = usize::try_from(menu).ok().and_then(|i| shown.get(i));
                if was != now {
                    window.set_card_menu(-1);
                    // Still drawn elsewhere or changed: focus on its first
                    // control. Gone: the removal's rule decides.
                    let row = was.map(Entry::place).map(|(_, row)| row);
                    let found = row
                        .and_then(|row| shown.iter().enumerate().find(|(_, e)| e.place().1 == row));
                    if let Some((at, entry)) = found {
                        self.focus_key_after_draw(match entry {
                            Entry::Proposal(_) => format!("proposal-copy:{at}"),
                            Entry::Answer(..) => format!("answer:{at}"),
                        });
                    }
                }
            }
            rows.set_vec(drawn);
            let mut shared = self.shared.borrow_mut();
            shared.cards.answers = answers;
            shared.swipes.retain(|k, _| !k.starts_with("p:"));
            shared.swipes.extend(swipes);
            drop(shared);
            self.cards.drawn_rows = handles;
            self.cards.proposals_shown = shown;
        }
        window.set_proposals(ModelRc::from(rows));
    }

    fn render_contacts(&mut self, model: &UiModel, window: &AppWindow) {
        window.set_contacts_title(text(UiText::ContactsTitle).into());
        window.set_own_card_title(text(UiText::OwnCardTitle).into());
        window.set_contacts_empty(text(UiText::ContactsEmpty).into());
        let mut contacts: Vec<ContactView> = model.contacts().to_vec();
        contacts.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.contact_id.cmp(&b.contact_id))
        });
        let rows = Rc::clone(
            self.cards
                .contact_rows
                .get_or_insert_with(|| Rc::new(VecModel::default())),
        );
        if contacts != self.cards.contacts_shown {
            let mut ids = HashMap::new();
            let drawn: Vec<ContactListRow> = contacts
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let handle = i32::try_from(i).unwrap_or(i32::MAX);
                    ids.insert(handle, c.contact_id.clone());
                    ContactListRow {
                        handle,
                        name: isolated(&c.name).into(),
                        letters: initials(&c.name).into(),
                        hue: contact_hue(c),
                        has_photo: c.photo.is_some(),
                        photo: c.photo.as_ref().map(image).unwrap_or_default(),
                    }
                })
                .collect();
            rows.set_vec(drawn);
            self.shared.borrow_mut().cards.contacts = ids;
            self.cards.contacts_shown = contacts;
        }
        window.set_contact_rows(ModelRc::from(rows));
    }

    fn render_contact(&mut self, model: &UiModel, window: &AppWindow) {
        window.set_name_label(text(UiText::CardName).into());
        window.set_save_label(text(UiText::Save).into());
        window.set_remove_photo_label(text(UiText::RemovePhoto).into());
        window.set_delete_contact_label(text(UiText::DeleteContact).into());
        window.set_cards_cancel_label(text(UiText::Cancel).into());
        window.set_routes_heading(text(UiText::Conversations).into());
        window.set_peer_id_label(text(UiText::DetailsPeerId).into());
        window.set_fingerprint_label(text(UiText::Fingerprint).into());
        window.set_contact_name_error(self.cards.contact_error.as_str().into());
        let contact = self
            .cards
            .contact
            .as_ref()
            .and_then(|id| model.contact(id))
            .cloned();
        self.shared.borrow_mut().cards.contact = contact.as_ref().map(|c| c.contact_id.clone());
        let Some(contact) = contact else {
            self.cards.contact_drawn = None;
            return;
        };
        if self.cards.contact_drawn.as_ref() == Some(&contact) {
            return;
        }
        // The field takes the name once per change of the contact, never
        // while the person types.
        if self
            .cards
            .contact_drawn
            .as_ref()
            .is_none_or(|d| d.name != contact.name)
        {
            window.set_contact_name_draft(contact.name.as_str().into());
        }
        let name = isolated(&contact.name);
        window.set_contact_title(name.as_str().into());
        window.set_delete_contact_confirm(
            fill(text(UiText::ContactDeleteConfirm), &[("name", &name)]).into(),
        );
        window.set_contact_letters(initials(&contact.name).into());
        window.set_contact_hue(contact_hue(&contact));
        window.set_contact_has_photo(contact.photo.is_some());
        window.set_contact_photo(contact.photo.as_ref().map(image).unwrap_or_default());
        let when = |at: Option<u64>| at.map_or_else(String::new, |at| self.clock.date_text(at));
        let name_source = if contact.name_from_card {
            UiText::ContactNameFromCard
        } else {
            UiText::ContactNameTyped
        };
        window.set_contact_name_source(
            fill(text(name_source), &[("date", &when(contact.name_at))]).into(),
        );
        let photo_source = match (contact.photo.is_some(), contact.photo_from_card) {
            (true, Some(true)) => Some(UiText::ContactPhotoFromCard),
            (true, _) => Some(UiText::ContactPhotoChosen),
            (false, Some(false)) => Some(UiText::ContactPhotoRemoved),
            (false, _) => None,
        };
        window.set_contact_photo_source(
            photo_source
                .map(|t| fill(text(t), &[("date", &when(contact.photo_at))]))
                .unwrap_or_default()
                .into(),
        );
        let mut shares = false;
        let routes: Vec<ContactRouteRow> = contact
            .routes
            .iter()
            .map(|r| {
                let sharing: Vec<SharedString> = model
                    .contacts_sharing(&contact.contact_id, &r.peer)
                    .iter()
                    .map(|c| {
                        fill(
                            text(UiText::ContactSharesDevice),
                            &[("name", &isolated(&c.name))],
                        )
                        .into()
                    })
                    .collect();
                shares |= !sharing.is_empty();
                ContactRouteRow {
                    title: route_title(&r.peer, &r.endpoint).into(),
                    peer: r.peer.as_str().into(),
                    fingerprint: r.fingerprint.clone().unwrap_or_default().into(),
                    sharing: ModelRc::new(VecModel::from(sharing)),
                }
            })
            .collect();
        window.set_contact_routes(ModelRc::new(VecModel::from(routes)));
        window.set_contact_shares_note(if shares {
            text(UiText::ContactSharesDeviceNote).into()
        } else {
            SharedString::new()
        });
        self.cards.contact_drawn = Some(contact);
    }

    fn render_own(&mut self, model: &UiModel, window: &AppWindow) {
        window.set_surname_label(text(UiText::CardSurname).into());
        window.set_own_photo_note(text(UiText::CardPhotoElsewhere).into());
        window.set_own_shared_note(text(UiText::CardSharedStays).into());
        window.set_own_given_error(self.cards.own_errors.0.as_str().into());
        window.set_own_surname_error(self.cards.own_errors.1.as_str().into());
        let Some(own) = model.identity() else {
            return;
        };
        // The key changes without the card: a new key, or one opened again.
        window.set_own_fingerprint(own.key.fingerprint().unwrap_or_default().into());
        let drawn = (own.given.clone(), own.surname.clone(), own.photo.clone());
        if self.cards.own_drawn.as_ref() == Some(&drawn) {
            return;
        }
        // The fields take the card's values when it changes, or when the
        // page opens; never while the person types.
        window.set_own_given(own.given.as_str().into());
        window.set_own_surname(own.surname.clone().unwrap_or_default().into());
        window.set_own_letters(initials(&own.name).into());
        window.set_own_hue(own.peer.as_ref().map_or(0, |p| {
            hue(Avatar::of(&ConversationKey::Direct {
                peer: p.clone(),
                endpoint: Some(own.endpoint.clone()),
            })
            .hue)
        }));
        window.set_own_has_photo(own.photo.is_some());
        window.set_own_photo(own.photo.as_ref().map(image).unwrap_or_default());
        window.set_own_peer(
            own.peer
                .as_ref()
                .map(interweave_transport_api::TransportIdentity::as_str)
                .unwrap_or_default()
                .into(),
        );
        self.cards.own_drawn = Some(drawn);
    }

    fn render_share(&mut self, model: &UiModel, window: &AppWindow) {
        let key = match self.page {
            crate::Page::Conversation => self.shown.clone(),
            _ => None,
        };
        let offered = key.as_ref().and_then(|k| model.share_card(k)).is_some();
        window.set_share_offered(offered);
        window.set_share_label(text(UiText::ShareCard).into());
        if !offered {
            window.set_share_open(false);
        }
        self.shared.borrow_mut().cards.shareable = key.clone().filter(|_| offered);
        let title = key
            .as_ref()
            .map(|k| model.drawn_title(k))
            .unwrap_or_default();
        window.set_share_title(fill(text(UiText::ShareCardTitle), &[("title", &title)]).into());
        window.set_share_preview_heading(
            fill(text(UiText::ShareCardPreview), &[("title", &title)]).into(),
        );
        window.set_share_note(text(UiText::ShareCardNote).into());
        window.set_share_action(text(UiText::ShareCardAction).into());
        // The key cannot be used: the sheet says why and leads to the card
        // page, over "no name yet" (design/2026-10-24-identity-key.md 4.3).
        let problem = model.key_problem();
        window.set_share_key_reason(problem.map_or("", key_reason).into());
        window.set_share_key_path(
            match problem {
                Some(KeyProblem::DirUnsafe { folder }) => folder.as_str(),
                _ => "",
            }
            .into(),
        );
        window.set_share_key_blocked(text(UiText::KeyCannotShareHere).into());
        window.set_share_open_card_label(text(UiText::KeyOpenCard).into());
        match model.identity() {
            Some(own) => {
                window.set_share_no_name(SharedString::new());
                window.set_share_preview_name(isolated(&own.name).into());
                window.set_share_preview_photo(if own.has_photo {
                    text(UiText::ShareCardWithPhoto).into()
                } else {
                    SharedString::new()
                });
            }
            None => window.set_share_no_name(text(UiText::ShareCardNoName).into()),
        }
        // Details: whether this identity's card was shared here, and when.
        let details = match &key {
            Some(
                k @ ConversationKey::Direct {
                    endpoint: Some(_), ..
                },
            ) => model.card_shared_at(k).map_or_else(
                || text(UiText::DetailsNoCard).to_owned(),
                |at| {
                    fill(
                        text(UiText::DetailsCardShared),
                        &[("date", &self.clock.date_text(at))],
                    )
                },
            ),
            _ => String::new(),
        };
        window.set_details_card(details.into());
    }
}

/// A route as the contact page names it: the short `PeerId` and the route
/// label, as a direct conversation's title builds it.
fn route_title(
    peer: &interweave_transport_api::TransportIdentity,
    endpoint: &EndpointId,
) -> String {
    fill(
        text(UiText::DirectTitle),
        &[
            ("peer", &short_peer(peer.as_str())),
            ("route", endpoint.as_str()),
        ],
    )
}
