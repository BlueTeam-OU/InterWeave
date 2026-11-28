// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The add form (design/2026-11-04-identities.md §6.2, §11 F5, F7, F8,
//! F24-F27; cards §5.2's editor for a card that does not exist yet):
//! where it was opened from, whether the first identity's seed was
//! answered, a Create running, the line above Create, what stands in
//! Create's place, and the focus it asks. Presentation state, no I/O:
//! the root asks the seed and runs the add, and tells the form what came
//! of each.

use crate::identities::{IdentityKey, Said, TryAgainControl};
use crate::labels::{UiText, isolated};
use crate::model::cards::{NameProblem, Photo, name_problem};

/// How long Create, inert while the form asks the seed, waits before it
/// shows its progress indicator (cards §7.2's threshold, which identities
/// §6.2 takes).
pub const PROGRESS_AFTER_MS: u64 = 300;

/// Where the add form was opened from, which its back arrow names
/// (identities §6.2, "A back arrow names where it leads").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddFrom {
    /// Your identities' Add an identity: `BackToIdentities`.
    Identities,
    /// The card editor's Add another identity, one identity held:
    /// `BackToCard`.
    Card,
}

/// How a Create ended, as the root tells it (identities §6.2's outcomes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddEnding {
    /// Done: `key` is an identity from now on; `card_saved` whether its
    /// card was written.
    Done {
        /// The new identity.
        key: IdentityKey,
        /// Whether its card was written.
        card_saved: bool,
    },
    /// Refused, with nothing created: the client knows that nothing was.
    Failed,
    /// Whether the identity was created is not known.
    Unknown,
    /// Created, not finished: the endpoint exists, the record stands.
    NotFinished,
    /// The add was refused at the profile's bound.
    Limit,
    /// The form's key is gone.
    KeyGone,
}

/// A control of the form that focus can be on, or that the form asks
/// focus to go to (identities §11's add form row and F5-F8, F24-F27).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddFocus {
    /// The Name field.
    Name,
    /// The Family name field.
    Surname,
    /// Create.
    Create,
    /// The line above Create: `IdentityAddFailed`, `IdentityAddUnknown`
    /// or `IdentityAddNotFinished`.
    LineAbove,
    /// The line in Create's place: `IdentityLimit`, `IdentityAddKeyGone`
    /// or `IdentityAddWaitsSeed`.
    InPlace,
    /// Try again under `IdentityAddWaitsSeed`.
    TryAgain,
    /// Any other control of the form.
    Other,
}

/// What a way of leaving the form does (identities §6.2, "Leaving asks
/// first").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leave {
    /// Nothing: Create runs, and Back and Escape are inert.
    Inert,
    /// `CardDiscardChanges` opens first.
    Ask,
    /// The form is left.
    Go,
}

/// A person's act on the add form, as a view returns it; the root
/// carries it out on the one form open (app-core's model side).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddIntent {
    /// The Name or the Family name field changed: what both hold now.
    Edited {
        /// The Name field.
        name: String,
        /// The Family name field.
        surname: String,
    },
    /// The view's focus moved to a control of the form.
    Focused(AddFocus),
    /// A photo was chosen: the file, as read.
    ChoosePhoto(Vec<u8>),
    /// The photo was removed.
    RemovePhoto,
    /// Create, or Enter in a field.
    Create,
    /// Try again under `IdentityAddWaitsSeed`.
    TryAgain,
    /// The back arrow, Escape or Back.
    Leave,
    /// `CardDiscardChanges` answered: Close without saving when `true`,
    /// Continue editing when `false`.
    Discard(bool),
}

/// The form's photo.
#[derive(Debug, Clone, PartialEq, Eq)]
enum FormPhoto {
    None,
    /// Chosen, its preparation numbered `seq` running.
    Preparing(u64),
    /// Prepared: the bytes a Create carries, and what is shown.
    Prepared {
        bytes: Vec<u8>,
        shown: Photo,
    },
    /// The file chosen cannot go on a card.
    Unusable,
}

/// Create as the form draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateControl {
    /// `CreateIdentity`.
    pub text: UiText,
    /// Its accessible name in place of its text: `IdentityAddChecking`
    /// while the seed is asked, `IdentityCreating` while it runs.
    pub name: Option<Said>,
    /// Choosing it does nothing.
    pub inert: bool,
    /// Its progress indicator shows.
    pub progress: bool,
}

/// What stands in Create's place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InPlace {
    /// `IdentityLimit`, `IdentityAddKeyGone` or `IdentityAddWaitsSeed`.
    pub said: Said,
    /// Try again under `IdentityAddWaitsSeed` alone.
    pub try_again: Option<TryAgainControl>,
}

/// The form as a view draws it, top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddFormView {
    /// `AddIdentityTitle`.
    pub title: UiText,
    /// The back arrow's accessible name.
    pub back: UiText,
    /// `IdentityNewNote`.
    pub note: UiText,
    /// The fields take no input: Create runs.
    pub fields_inert: bool,
    /// The photo shown, prepared.
    pub photo: Option<Photo>,
    /// The photo's line: `CardPhotoPreparing` while it is prepared,
    /// `PhotoUnusable` for a file a card cannot carry.
    pub photo_line: Option<UiText>,
    /// What the Name field says is wrong, after a Create on it.
    pub name_problem: Option<UiText>,
    /// What the Family name field says is wrong, after a Create on it.
    pub surname_problem: Option<UiText>,
    /// `CardDiscardChanges`, while a leaving asks it.
    pub discard: Option<UiText>,
    /// The line above Create.
    pub line_above: Option<Said>,
    /// Create, or `None` when something stands in its place.
    pub create: Option<CreateControl>,
    /// What stands in Create's place.
    pub in_place: Option<InPlace>,
}

/// What the form's fields hold.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddValues {
    /// The Name field.
    pub name: String,
    /// The Family name field.
    pub surname: String,
    /// Whether the form shows a photo.
    pub photo: bool,
}

impl AddValues {
    /// Unsaved changes: the form's saved card is an empty one (§6.2,
    /// "Leaving asks first").
    #[must_use]
    pub fn changed(&self) -> bool {
        !self.name.is_empty() || !self.surname.is_empty() || self.photo
    }

    /// The name as the row will show it, name and family name (Q-A6).
    fn shown(&self) -> String {
        if self.surname.is_empty() {
            self.name.clone()
        } else {
            format!("{} {}", self.name, self.surname)
        }
    }
}

/// Done: the form has closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddDone {
    /// The new identity, whose row Your identities focuses (F6, F9).
    pub key: IdentityKey,
    /// `ConfirmIdentityAdded`, with the card saved. With the card not
    /// saved the row and its `IdentityCardNotSaved` are the status, and
    /// nothing is shown (the plan's answer to Q-U4).
    pub snackbar: Option<Said>,
    /// The form's values, with the card not saved: the identity's page
    /// opens with them for this session.
    pub kept: Option<AddValues>,
}

/// The seed's state for this form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seed {
    /// Asked; `again` when Try again asked it.
    Asking { since: u64, again: bool },
    /// The ask ended unanswered.
    NotAnswered,
    /// Answered in this run.
    Answered,
}

/// The line the last Create left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shown {
    Failed,
    Unknown,
    NotFinished,
    Limit,
    KeyGone,
}

/// One add form, from its opening to Done or its leaving.
#[derive(Debug, Clone)]
pub struct AddForm {
    from: AddFrom,
    seed: Seed,
    running: bool,
    shown: Option<Shown>,
    values: AddValues,
    closed: bool,
    /// Where the view's focus is, as it last said.
    focus_at: AddFocus,
    focus: Option<(u64, AddFocus)>,
    photo: FormPhoto,
    /// The last photo choice's number.
    photo_seq: u64,
    /// The fields' problems a Create found, until the field changes.
    problems: (Option<UiText>, Option<UiText>),
    /// `CardDiscardChanges` is asked.
    asking: bool,
}

impl AddForm {
    /// The form opens at `now` (ms), focus on the Name field (F5). When
    /// no seed was answered in this run the form asks one, and the root
    /// asks it ([`Self::asks_seed`]).
    #[must_use]
    pub fn open(from: AddFrom, seed_answered: bool, now: u64) -> Self {
        let mut form = Self {
            from,
            seed: if seed_answered {
                Seed::Answered
            } else {
                Seed::Asking {
                    since: now,
                    again: false,
                }
            },
            running: false,
            shown: None,
            values: AddValues::default(),
            closed: false,
            focus_at: AddFocus::Name,
            focus: None,
            photo: FormPhoto::None,
            photo_seq: 0,
            problems: (None, None),
            asking: false,
        };
        form.ask_focus(AddFocus::Name);
        form
    }

    /// Whether the form is asking the seed now.
    #[must_use]
    pub const fn asks_seed(&self) -> bool {
        matches!(self.seed, Seed::Asking { .. })
    }

    /// Whether a Create runs.
    #[must_use]
    pub const fn running(&self) -> bool {
        self.running
    }

    /// Whether the form closed at Done.
    #[must_use]
    pub const fn closed(&self) -> bool {
        self.closed
    }

    /// What the fields hold.
    #[must_use]
    pub const fn values(&self) -> &AddValues {
        &self.values
    }

    fn ask_focus(&mut self, to: AddFocus) {
        let seq = self.focus.map_or(1, |(seq, _)| seq.wrapping_add(1));
        self.focus = Some((seq, to));
    }

    /// The newest focus request, by its count: a view moves focus once
    /// per count.
    #[must_use]
    pub const fn focus_request(&self) -> Option<(u64, AddFocus)> {
        self.focus
    }

    /// The view's focus moved to `at`: a seed's answer moves it only from
    /// where F25 says.
    pub const fn focused(&mut self, at: AddFocus) {
        self.focus_at = at;
    }

    /// The person changed the fields. Refused while Create runs, when
    /// the fields take no input (§6.2 step 1).
    pub fn edit(&mut self, values: AddValues) -> bool {
        if self.running || self.closed {
            return false;
        }
        // A field's problem stands until that field changes.
        if values.name != self.values.name {
            self.problems.0 = None;
        }
        if values.surname != self.values.surname {
            self.problems.1 = None;
        }
        self.values = values;
        true
    }

    /// Where the form was opened from.
    #[must_use]
    pub const fn from(&self) -> AddFrom {
        self.from
    }

    /// A photo was chosen: its preparation's number, for the root to
    /// prepare it by, or `None` while Create runs or once closed. A later
    /// choice replaces an earlier one still being prepared.
    pub fn photo_chosen(&mut self) -> Option<u64> {
        if self.running || self.closed {
            return None;
        }
        self.photo_seq += 1;
        self.photo = FormPhoto::Preparing(self.photo_seq);
        self.values.photo = true;
        Some(self.photo_seq)
    }

    /// The preparation numbered `seq` ended: the bytes and what is shown,
    /// or `None` for a file a card cannot carry. One a later choice
    /// replaced, or a removal, changes nothing
    /// (`a_photo_answer_for_a_replaced_choice_changes_nothing`).
    pub fn photo_prepared(&mut self, seq: u64, prepared: Option<(Vec<u8>, Photo)>) {
        if self.photo != FormPhoto::Preparing(seq) {
            return;
        }
        self.photo = match prepared {
            Some((bytes, shown)) => FormPhoto::Prepared { bytes, shown },
            None => FormPhoto::Unusable,
        };
        self.values.photo = matches!(self.photo, FormPhoto::Prepared { .. });
    }

    /// The photo was removed, or a choice still prepared let go.
    pub fn photo_removed(&mut self) -> bool {
        if self.running || self.closed {
            return false;
        }
        self.photo = FormPhoto::None;
        self.values.photo = false;
        true
    }

    /// Create was chosen: cards §5.2's input rules are checked first, as
    /// on any card, and a field they refuse says why and takes focus
    /// with nothing started; otherwise as [`Self::create`]. Inert too
    /// while the photo is still prepared, so a Create never leaves a
    /// chosen photo behind
    /// (`a_create_refused_by_the_input_rules_names_the_field_and_starts_nothing`).
    pub fn create_checked(&mut self) -> bool {
        if self.closed || self.running || matches!(self.photo, FormPhoto::Preparing(_)) {
            return false;
        }
        let name = name_problem(&self.values.name).map(NameProblem::text);
        let surname = if self.values.surname.trim().is_empty() {
            None
        } else {
            name_problem(&self.values.surname).map(NameProblem::text)
        };
        if name.is_some() || surname.is_some() {
            self.problems = (name, surname);
            self.ask_focus(if name.is_some() {
                AddFocus::Name
            } else {
                AddFocus::Surname
            });
            return false;
        }
        self.create()
    }

    /// The card a Create carries: the name, the family name when one was
    /// given, and the prepared photo's bytes.
    #[must_use]
    pub fn card(&self) -> (String, Option<String>, Option<Vec<u8>>) {
        let surname = (!self.values.surname.trim().is_empty()).then(|| self.values.surname.clone());
        let photo = match &self.photo {
            FormPhoto::Prepared { bytes, .. } => Some(bytes.clone()),
            FormPhoto::None | FormPhoto::Preparing(_) | FormPhoto::Unusable => None,
        };
        (self.values.name.clone(), surname, photo)
    }

    /// A way of leaving was chosen, as [`Self::leave`] decides it; when
    /// that asks, `CardDiscardChanges` shows until it is answered.
    pub fn ask_leave(&mut self) -> Leave {
        let leave = self.leave();
        self.asking = leave == Leave::Ask;
        leave
    }

    /// `CardDiscardChanges` answered: whether the form is left -- Close
    /// without saving -- or stays as it was. Nothing when it was not
    /// asked.
    pub fn discard_answered(&mut self, close: bool) -> bool {
        if !std::mem::take(&mut self.asking) {
            return false;
        }
        close
    }

    /// The seed's ask ended: `answered` when it was answered `seeded` or
    /// `present`, here or on any other connection of the run.
    pub fn seed_told(&mut self, answered: bool) {
        let was = self.seed;
        if answered {
            self.seed = Seed::Answered;
            // F25's second case and F27's last: from the line or its Try
            // again, focus goes to Create, which takes their place. The
            // first ask's answer moves nothing: the line never showed.
            let from_line = matches!(self.focus_at, AddFocus::InPlace | AddFocus::TryAgain);
            let line_showed = matches!(was, Seed::NotAnswered | Seed::Asking { again: true, .. });
            if line_showed && from_line {
                self.ask_focus(AddFocus::Create);
            }
            return;
        }
        if let Seed::Asking { again, .. } = was {
            self.seed = Seed::NotAnswered;
            // F25's first case: from the inert Create to the line. After
            // Try again, focus stays on Try again (F27).
            if !again && self.focus_at == AddFocus::Create {
                self.ask_focus(AddFocus::InPlace);
            }
        }
    }

    /// Try again under `IdentityAddWaitsSeed`, at `now` (ms): the root
    /// asks the seed again when this answers `true`. Inert while it asks,
    /// focus staying on it (F27).
    pub fn try_again(&mut self, now: u64) -> bool {
        if self.seed != Seed::NotAnswered || self.closed {
            return false;
        }
        self.seed = Seed::Asking {
            since: now,
            again: true,
        };
        true
    }

    /// Create was chosen on a valid form -- [`Self::create_checked`]
    /// checks cards §5.2's input rules first, as on any card: the root
    /// runs the add when this answers
    /// `true`, once (`create_runs_once_and_back_and_escape_do_nothing_while_it_runs`).
    /// Inert while the seed is asked or unanswered, while one runs, and
    /// when something stands in its place.
    pub fn create(&mut self) -> bool {
        if self.closed
            || self.running
            || self.seed != Seed::Answered
            || matches!(self.shown, Some(Shown::Limit | Shown::KeyGone))
        {
            return false;
        }
        self.running = true;
        self.shown = None;
        true
    }

    /// The running Create ended. Done closes the form and answers what
    /// the root shows next; every other ending keeps the form, its line
    /// shown and focused (F7, F8, F24).
    pub fn ended(&mut self, ending: AddEnding) -> Option<AddDone> {
        if !self.running || self.closed {
            return None;
        }
        self.running = false;
        let (shown, focus) = match ending {
            AddEnding::Done { key, card_saved } => {
                self.closed = true;
                return Some(AddDone {
                    key,
                    snackbar: card_saved.then(|| self.named(UiText::ConfirmIdentityAdded)),
                    kept: (!card_saved).then(|| self.values.clone()),
                });
            }
            AddEnding::Failed => (Shown::Failed, AddFocus::LineAbove),
            AddEnding::Unknown => (Shown::Unknown, AddFocus::LineAbove),
            AddEnding::NotFinished => (Shown::NotFinished, AddFocus::LineAbove),
            AddEnding::Limit => (Shown::Limit, AddFocus::InPlace),
            AddEnding::KeyGone => (Shown::KeyGone, AddFocus::InPlace),
        };
        self.shown = Some(shown);
        self.ask_focus(focus);
        None
    }

    /// A way of leaving was chosen: Back, Escape, or any other of cards
    /// §5.2's. Only unsaved changes decide whether `CardDiscardChanges`
    /// opens, whatever line shows, except at the limit and with the key
    /// gone, where no Create can save the fields any more; while Create
    /// runs, leaving does nothing
    /// (`leaving_asks_only_with_unsaved_changes_and_never_at_the_limit_key_gone_or_while_running`).
    #[must_use]
    pub fn leave(&self) -> Leave {
        if self.running {
            Leave::Inert
        } else if matches!(self.shown, Some(Shown::Limit | Shown::KeyGone)) {
            Leave::Go
        } else if self.values.changed() {
            Leave::Ask
        } else {
            Leave::Go
        }
    }

    fn named(&self, text: UiText) -> Said {
        Said::of(text).with("name", isolated(&self.values.shown()))
    }

    /// The form at `now` (ms).
    #[must_use]
    pub fn view(&self, now: u64) -> AddFormView {
        let line_above = match self.shown {
            Some(Shown::Failed) => Some(Said::of(UiText::IdentityAddFailed)),
            Some(Shown::Unknown) => Some(self.named(UiText::IdentityAddUnknown)),
            Some(Shown::NotFinished) => Some(self.named(UiText::IdentityAddNotFinished)),
            Some(Shown::Limit | Shown::KeyGone) | None => None,
        };
        let in_place = match (self.shown, self.seed) {
            (Some(Shown::Limit), _) => Some(InPlace {
                said: Said::of(UiText::IdentityLimit),
                try_again: None,
            }),
            (Some(Shown::KeyGone), _) => Some(InPlace {
                said: self.named(UiText::IdentityAddKeyGone),
                try_again: None,
            }),
            (_, Seed::NotAnswered) => Some(InPlace {
                said: Said::of(UiText::IdentityAddWaitsSeed),
                try_again: Some(TryAgainControl {
                    text: UiText::TryAgain,
                    enabled: true,
                    progress: false,
                }),
            }),
            (_, Seed::Asking { again: true, .. }) => Some(InPlace {
                said: Said::of(UiText::IdentityAddWaitsSeed),
                try_again: Some(TryAgainControl {
                    text: UiText::TryAgain,
                    enabled: false,
                    progress: true,
                }),
            }),
            _ => None,
        };
        let create = in_place.is_none().then(|| match self.seed {
            Seed::Asking { since, .. } => CreateControl {
                text: UiText::CreateIdentity,
                name: Some(Said::of(UiText::IdentityAddChecking)),
                inert: true,
                progress: now.saturating_sub(since) >= PROGRESS_AFTER_MS,
            },
            _ if self.running => CreateControl {
                text: UiText::CreateIdentity,
                name: Some(self.named(UiText::IdentityCreating)),
                inert: true,
                progress: true,
            },
            _ => CreateControl {
                text: UiText::CreateIdentity,
                name: None,
                inert: false,
                progress: false,
            },
        });
        AddFormView {
            title: UiText::AddIdentityTitle,
            back: match self.from {
                AddFrom::Identities => UiText::BackToIdentities,
                AddFrom::Card => UiText::BackToCard,
            },
            note: UiText::IdentityNewNote,
            fields_inert: self.running,
            photo: match &self.photo {
                FormPhoto::Prepared { shown, .. } => Some(shown.clone()),
                FormPhoto::None | FormPhoto::Preparing(_) | FormPhoto::Unusable => None,
            },
            photo_line: match self.photo {
                FormPhoto::Preparing(_) => Some(UiText::CardPhotoPreparing),
                FormPhoto::Unusable => Some(UiText::PhotoUnusable),
                FormPhoto::None | FormPhoto::Prepared { .. } => None,
            },
            name_problem: self.problems.0,
            surname_problem: self.problems.1,
            discard: self.asking.then_some(UiText::CardDiscardChanges),
            line_above,
            create,
            in_place,
        }
    }

    /// When the inert Create's progress indicator next shows, for the
    /// root's timer.
    #[must_use]
    pub fn next_change(&self, now: u64) -> Option<u64> {
        match self.seed {
            Seed::Asking {
                since,
                again: false,
            } => {
                let at = since.saturating_add(PROGRESS_AFTER_MS);
                (at > now).then_some(at)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identities::{HeldIdentity, Identities, IdentityFocus, IdentityState};

    fn key(tag: &str) -> IdentityKey {
        IdentityKey::new(format!("12D3KooWtest{tag}"))
    }

    fn focus(form: &AddForm) -> Option<AddFocus> {
        form.focus_request().map(|(_, f)| f)
    }

    fn seq(form: &AddForm) -> Option<u64> {
        form.focus_request().map(|(s, _)| s)
    }

    fn ada() -> AddValues {
        AddValues {
            name: "Ada".to_owned(),
            surname: "Lovelace".to_owned(),
            photo: false,
        }
    }

    /// A form that offers Create, holding Ada Lovelace.
    fn ready() -> AddForm {
        let mut form = AddForm::open(AddFrom::Identities, true, 0);
        assert!(form.edit(ada()));
        form
    }

    /// A form whose seed's first ask ended unanswered.
    fn unanswered() -> AddForm {
        let mut form = AddForm::open(AddFrom::Identities, false, 0);
        form.seed_told(false);
        form
    }

    fn ended(form: &mut AddForm, ending: AddEnding) {
        assert!(form.create(), "Create runs");
        assert_eq!(form.ended(ending), None, "the form stays");
    }

    /// F5: the add form opens on its Name field, under its title and
    /// above its note, with Create offered once a seed was answered in
    /// this run (identities §6.2, "The form").
    #[test]
    fn the_form_opens_on_the_name_field() {
        let form = AddForm::open(AddFrom::Identities, true, 0);
        assert_eq!(focus(&form), Some(AddFocus::Name));
        let view = form.view(0);
        assert_eq!(view.title, UiText::AddIdentityTitle);
        assert_eq!(view.note, UiText::IdentityNewNote);
        assert!(!view.fields_inert);
        assert_eq!(view.line_above, None);
        assert_eq!(view.in_place, None);
        assert_eq!(
            view.create,
            Some(CreateControl {
                text: UiText::CreateIdentity,
                name: None,
                inert: false,
                progress: false,
            })
        );
        assert!(!form.asks_seed());
        let asking = AddForm::open(AddFrom::Identities, false, 0);
        assert_eq!(focus(&asking), Some(AddFocus::Name), "whether or not");
    }

    /// Identities §6.2, "The form asks": with no seed answered in this
    /// run the form asks one; Create shows, inert, read
    /// `IdentityAddChecking`, its progress from 300 ms (cards §7.2).
    /// Mutation: Create let through while the seed is asked.
    #[test]
    fn create_is_inert_and_reads_checking_while_the_seed_is_asked() {
        let mut form = AddForm::open(AddFrom::Identities, false, 1_000);
        assert!(form.edit(ada()));
        assert!(form.asks_seed());
        let create = |form: &AddForm, now| form.view(now).create.expect("Create shows");
        let at_once = create(&form, 1_000);
        assert!(at_once.inert);
        assert_eq!(
            at_once.name.map(|s| s.text),
            Some(UiText::IdentityAddChecking)
        );
        assert!(!at_once.progress);
        assert!(!create(&form, 1_000 + PROGRESS_AFTER_MS - 1).progress);
        assert!(create(&form, 1_000 + PROGRESS_AFTER_MS).progress);
        assert_eq!(form.next_change(1_000), Some(1_000 + PROGRESS_AFTER_MS));
        assert!(!form.create(), "choosing it does nothing");
        assert!(!form.running());
    }

    /// F25's first case: the seed not answered, `IdentityAddWaitsSeed`
    /// and Try again stand in Create's place, and focus moves to the line
    /// only when it was on the inert Create. Mutation: focus always
    /// moved.
    #[test]
    fn an_unanswered_seed_puts_the_line_and_try_again_in_creates_place() {
        let mut on_create = AddForm::open(AddFrom::Identities, false, 0);
        on_create.focused(AddFocus::Create);
        on_create.seed_told(false);
        let view = on_create.view(0);
        assert_eq!(view.create, None);
        let in_place = view.in_place.expect("in Create's place");
        assert_eq!(in_place.said.text, UiText::IdentityAddWaitsSeed);
        assert_eq!(
            in_place.try_again,
            Some(TryAgainControl {
                text: UiText::TryAgain,
                enabled: true,
                progress: false,
            })
        );
        assert_eq!(focus(&on_create), Some(AddFocus::InPlace));
        assert!(!on_create.create());
        assert!(on_create.edit(ada()), "the fields take input throughout");

        let mut on_name = AddForm::open(AddFrom::Identities, false, 0);
        let before = seq(&on_name);
        on_name.focused(AddFocus::Name);
        on_name.seed_told(false);
        assert_eq!(seq(&on_name), before, "focus stays where it was");
    }

    /// F27: Try again asks the seed again, inert with its progress while
    /// it asks, focus staying on it; unanswered again, it is enabled
    /// again and focus still stays. Mutation: Try again taken while it
    /// asks.
    #[test]
    fn try_again_keeps_focus_and_is_inert_while_it_asks() {
        let mut form = unanswered();
        form.focused(AddFocus::TryAgain);
        let before = seq(&form);
        assert!(form.try_again(10));
        assert!(form.asks_seed());
        let again = form.view(10).in_place.expect("the line stays");
        assert_eq!(again.said.text, UiText::IdentityAddWaitsSeed);
        assert_eq!(
            again.try_again,
            Some(TryAgainControl {
                text: UiText::TryAgain,
                enabled: false,
                progress: true,
            })
        );
        assert!(!form.try_again(20), "inert while it asks");
        assert_eq!(seq(&form), before);

        form.seed_told(false);
        assert_eq!(
            form.view(30)
                .in_place
                .and_then(|p| p.try_again)
                .map(|t| t.enabled),
            Some(true)
        );
        assert_eq!(seq(&form), before, "still on Try again");
    }

    /// F25's second case: a seed answered while the line shows -- on
    /// this form's Try again or on another connection -- puts Create in
    /// its place, and focus moves to Create only from the line or Try
    /// again; the first ask's answer moves nothing. The model announces
    /// nothing: it has no announcement to give. Mutation: focus moved to
    /// Create whatever it was on.
    #[test]
    fn a_seed_answered_while_the_line_shows_brings_create_back() {
        let mut elsewhere = unanswered();
        elsewhere.focused(AddFocus::InPlace);
        elsewhere.seed_told(true);
        let view = elsewhere.view(0);
        assert_eq!(view.in_place, None);
        assert!(view.create.is_some_and(|c| !c.inert));
        assert_eq!(focus(&elsewhere), Some(AddFocus::Create));

        let mut retried = unanswered();
        retried.focused(AddFocus::TryAgain);
        assert!(retried.try_again(5));
        retried.seed_told(true);
        assert_eq!(focus(&retried), Some(AddFocus::Create));

        let mut on_name = unanswered();
        on_name.focused(AddFocus::Name);
        let before = seq(&on_name);
        on_name.seed_told(true);
        assert_eq!(seq(&on_name), before, "focus stays where it was");
        assert!(on_name.create(), "Create offered");

        let mut first = AddForm::open(AddFrom::Identities, false, 0);
        first.focused(AddFocus::Create);
        let before = seq(&first);
        first.seed_told(true);
        assert_eq!(seq(&first), before, "no line showed");
    }

    /// Identities §6.2 step 1: Create runs once, its progress in itself
    /// and read `IdentityCreating`, the fields inert, and Back and Escape
    /// do nothing while it runs. Mutation: leaving while it runs.
    #[test]
    fn create_runs_once_and_back_and_escape_do_nothing_while_it_runs() {
        let mut form = ready();
        assert!(form.create());
        assert!(!form.create(), "once");
        assert!(form.running());
        assert_eq!(form.leave(), Leave::Inert);
        assert!(!form.edit(AddValues::default()), "no input");
        let view = form.view(0);
        assert!(view.fields_inert);
        let create = view.create.expect("Create shows");
        assert!(create.inert && create.progress);
        let name = create.name.expect("its accessible name");
        assert_eq!(name.text, UiText::IdentityCreating);
        assert_eq!(name.values, [("name", isolated("Ada Lovelace"))]);
    }

    /// F7, F8 and F24: each ending other than Done keeps the form, its
    /// values kept, and shows its own line -- above Create for refused,
    /// unknown and not finished, in Create's place at the limit and with
    /// the key gone -- which takes focus.
    #[test]
    fn each_ending_shows_its_own_line_and_takes_focus_there() {
        for (ending, above, place) in [
            (AddEnding::Failed, Some(UiText::IdentityAddFailed), None),
            (AddEnding::Unknown, Some(UiText::IdentityAddUnknown), None),
            (
                AddEnding::NotFinished,
                Some(UiText::IdentityAddNotFinished),
                None,
            ),
            (AddEnding::Limit, None, Some(UiText::IdentityLimit)),
            (AddEnding::KeyGone, None, Some(UiText::IdentityAddKeyGone)),
        ] {
            let mut form = ready();
            ended(&mut form, ending.clone());
            let view = form.view(0);
            assert_eq!(view.line_above.map(|s| s.text), above, "{ending:?}");
            assert_eq!(view.in_place.map(|p| p.said.text), place, "{ending:?}");
            assert_eq!(
                focus(&form),
                Some(if above.is_some() {
                    AddFocus::LineAbove
                } else {
                    AddFocus::InPlace
                }),
                "{ending:?}"
            );
            assert_eq!(form.values(), &ada(), "{ending:?}: kept");
            assert!(!form.closed());
        }
    }

    /// Identities §6.2, "The outcome is unknown": `IdentityAddUnknown`
    /// naming the identity, the form open and Create offered again --
    /// never `IdentityAddFailed`, never Done. Mutation: Unknown shown as
    /// Failed.
    #[test]
    fn an_unknown_outcome_is_never_shown_as_failed_or_as_done() {
        let mut form = ready();
        ended(&mut form, AddEnding::Unknown);
        let line = form.view(0).line_above.expect("a line");
        assert_eq!(line.text, UiText::IdentityAddUnknown);
        assert_ne!(line.text, UiText::IdentityAddFailed);
        assert_eq!(line.values, [("name", isolated("Ada Lovelace"))]);
        assert!(!form.closed(), "not Done");
        assert!(form.create(), "Create tries again");
    }

    /// Identities §6.2: at the limit and with the key gone no Create on
    /// the form can succeed, so none is offered and choosing where it
    /// stood does nothing; `IdentityAddKeyGone` names the identity.
    #[test]
    fn key_gone_and_the_limit_take_creates_place_and_offer_no_create() {
        for ending in [AddEnding::Limit, AddEnding::KeyGone] {
            let mut form = ready();
            ended(&mut form, ending.clone());
            let view = form.view(0);
            assert_eq!(view.create, None, "{ending:?}");
            let place = view.in_place.expect("in Create's place");
            assert_eq!(place.try_again, None, "{ending:?}");
            if ending == AddEnding::KeyGone {
                assert_eq!(place.said.values, [("name", isolated("Ada Lovelace"))]);
            }
            assert!(!form.create(), "{ending:?}");
        }
    }

    /// Identities §6.2, "Leaving asks first", and F26: only unsaved
    /// changes decide whether `CardDiscardChanges` opens, whatever line
    /// shows; it never opens at the limit, with the key gone or while
    /// Create runs. Mutation: asked whenever a line shows.
    #[test]
    fn leaving_asks_only_with_unsaved_changes_and_never_at_the_limit_key_gone_or_while_running() {
        let blank = AddForm::open(AddFrom::Identities, true, 0);
        assert_eq!(blank.leave(), Leave::Go);
        for values in [
            ada(),
            AddValues {
                surname: "L".to_owned(),
                ..AddValues::default()
            },
            AddValues {
                photo: true,
                ..AddValues::default()
            },
        ] {
            let mut form = AddForm::open(AddFrom::Identities, true, 0);
            assert!(form.edit(values));
            assert_eq!(form.leave(), Leave::Ask);
        }
        for ending in [
            AddEnding::Failed,
            AddEnding::Unknown,
            AddEnding::NotFinished,
        ] {
            let mut changed = ready();
            ended(&mut changed, ending.clone());
            assert_eq!(changed.leave(), Leave::Ask, "{ending:?}");
            let mut unchanged = AddForm::open(AddFrom::Identities, true, 0);
            ended(&mut unchanged, ending.clone());
            assert_eq!(unchanged.leave(), Leave::Go, "{ending:?}: a line alone");
        }
        for ending in [AddEnding::Limit, AddEnding::KeyGone] {
            let mut form = ready();
            ended(&mut form, ending.clone());
            assert_eq!(form.leave(), Leave::Go, "{ending:?}");
        }
        let mut waits = unanswered();
        assert!(waits.edit(ada()));
        assert_eq!(
            waits.leave(),
            Leave::Ask,
            "the seed's line alone decides nothing"
        );
        let mut running = ready();
        assert!(running.create());
        assert_eq!(running.leave(), Leave::Inert);
    }

    /// F6, identities §6.2 step 4: Done closes the form, names the new
    /// row -- which Your identities opens on -- and gives
    /// `ConfirmIdentityAdded` once; with the card not saved there is no
    /// snackbar, the row is marked (F9) and the form's values are kept
    /// for its page. Mutation: a second ending answered again.
    #[test]
    fn done_names_the_new_row_and_announces_once() {
        let held = |tag: &str| HeldIdentity {
            key: key(tag),
            card_name: None,
            alias: format!("alias-{tag}"),
            state: IdentityState::Ready,
            is_first: tag == "a",
            is_default: tag == "a",
            unread: 0,
            not_sent: false,
        };
        let mut ids = Identities::new();
        ids.held_changed(vec![held("a"), held("n")]).expect("held");
        ids.start(None);

        let mut form = ready();
        assert!(form.create());
        let done = form
            .ended(AddEnding::Done {
                key: key("n"),
                card_saved: true,
            })
            .expect("Done");
        assert_eq!(done.key, key("n"));
        let snackbar = done.snackbar.expect("announced");
        assert_eq!(snackbar.text, UiText::ConfirmIdentityAdded);
        assert_eq!(snackbar.values, [("name", isolated("Ada Lovelace"))]);
        assert_eq!(done.kept, None);
        assert!(form.closed());
        assert_eq!(
            form.ended(AddEnding::Done {
                key: key("n"),
                card_saved: true,
            }),
            None,
            "once"
        );
        assert!(!form.create(), "closed");
        ids.added(&done.key, done.kept.clone());
        assert_eq!(
            ids.focus_request().map(|(_, f)| f),
            Some(IdentityFocus::Row)
        );
        assert_eq!(ids.focus_row(), Some(&key("n")));

        let mut unsaved = ready();
        assert!(unsaved.create());
        let done = unsaved
            .ended(AddEnding::Done {
                key: key("n"),
                card_saved: false,
            })
            .expect("Done");
        assert_eq!(done.snackbar, None);
        assert_eq!(done.kept, Some(ada()));
        ids.added(&done.key, done.kept.clone());
        let page = ids.your_identities().expect("the list");
        let row = page.rows.iter().find(|r| r.key == key("n")).expect("row");
        assert_eq!(row.card_not_saved, Some(UiText::IdentityCardNotSaved));
        assert_eq!(row.name, "alias-n");
    }

    /// Identities §6.2, "A back arrow names where it leads": the list's
    /// form returns to the list, the card editor's to the card.
    #[test]
    fn the_back_arrow_names_where_it_leads() {
        assert_eq!(
            AddForm::open(AddFrom::Identities, true, 0).view(0).back,
            UiText::BackToIdentities
        );
        assert_eq!(
            AddForm::open(AddFrom::Card, true, 0).view(0).back,
            UiText::BackToCard
        );
    }

    fn photo(side: u32) -> Photo {
        Photo {
            width: side,
            height: side,
            rgba: vec![0; 4],
        }
    }

    /// Cards §5.2's input rules, on the form as on any card: an empty
    /// name, or a family name the card refuses, says why under its field,
    /// takes focus there and starts nothing; the field's change clears
    /// it, and a valid form's Create runs. While the photo is still being
    /// prepared Create starts nothing either, so no chosen photo is left
    /// behind. Mutation: the check skipped -- an empty name runs.
    #[test]
    fn a_create_refused_by_the_input_rules_names_the_field_and_starts_nothing() {
        let mut form = AddForm::open(AddFrom::Identities, true, 0);
        assert!(!form.create_checked(), "an empty name");
        assert!(!form.running());
        assert_eq!(form.view(0).name_problem, Some(UiText::CardNameNeeded));
        assert_eq!(focus(&form), Some(AddFocus::Name));

        assert!(form.edit(AddValues {
            name: "Ada".to_owned(),
            surname: "Love\u{2028}lace".to_owned(),
            photo: false,
        }));
        assert_eq!(form.view(0).name_problem, None, "its field changed");
        assert!(!form.create_checked(), "a family name refused");
        let view = form.view(0);
        assert_eq!(view.name_problem, None);
        assert_eq!(view.surname_problem, Some(UiText::CardNameBadCharacters));
        assert_eq!(focus(&form), Some(AddFocus::Surname));

        assert!(form.edit(ada()));
        let seq = form.photo_chosen().expect("a choice");
        assert!(!form.create_checked(), "the photo is being prepared");
        assert!(!form.running());
        form.photo_prepared(seq, None);
        assert!(form.create_checked(), "a valid form");
        assert!(form.running());
        assert_eq!(
            form.card(),
            ("Ada".to_owned(), Some("Lovelace".to_owned()), None)
        );
    }

    /// The form's photo (cards §5.2's editor): a choice shows
    /// `CardPhotoPreparing` until its answer; prepared, the photo shows
    /// and a Create carries its bytes; a file a card cannot carry shows
    /// `PhotoUnusable` and carries none; an answer for a choice a later
    /// one replaced, or after a removal, changes nothing. A photo is an
    /// unsaved change. Mutation: the answer taken whatever its number.
    #[test]
    fn a_photo_answer_for_a_replaced_choice_changes_nothing() {
        let mut form = AddForm::open(AddFrom::Identities, true, 0);
        let first = form.photo_chosen().expect("a choice");
        assert_eq!(form.view(0).photo_line, Some(UiText::CardPhotoPreparing));
        assert!(form.values().changed(), "a photo is a change");
        let second = form.photo_chosen().expect("another");
        form.photo_prepared(first, Some((vec![1], photo(1))));
        assert_eq!(form.view(0).photo, None, "the replaced choice's answer");
        form.photo_prepared(second, Some((vec![2], photo(2))));
        let view = form.view(0);
        assert_eq!(view.photo, Some(photo(2)));
        assert_eq!(view.photo_line, None);
        assert_eq!(form.card().2, Some(vec![2]));

        let third = form.photo_chosen().expect("a third");
        form.photo_prepared(third, None);
        assert_eq!(form.view(0).photo_line, Some(UiText::PhotoUnusable));
        assert_eq!(form.card().2, None);
        assert!(!form.values().changed(), "no photo held");

        let fourth = form.photo_chosen().expect("a fourth");
        assert!(form.photo_removed());
        form.photo_prepared(fourth, Some((vec![4], photo(4))));
        assert_eq!(form.view(0).photo, None, "removed before its answer");
    }

    /// Identities §6.2, "Leaving asks first", as the form holds it: a
    /// leaving that asks shows `CardDiscardChanges` until it is answered;
    /// Continue editing keeps the form, Close without saving leaves it,
    /// and an answer with nothing asked does nothing. Mutation: the
    /// question not held.
    #[test]
    fn a_leaving_that_asks_holds_the_question_until_it_is_answered() {
        let mut form = ready();
        assert_eq!(form.ask_leave(), Leave::Ask);
        assert_eq!(form.view(0).discard, Some(UiText::CardDiscardChanges));
        assert!(!form.discard_answered(false), "Continue editing");
        assert_eq!(form.view(0).discard, None);
        assert!(!form.discard_answered(true), "nothing asked now");
        assert_eq!(form.ask_leave(), Leave::Ask);
        assert!(form.discard_answered(true), "Close without saving");

        let mut empty = AddForm::open(AddFrom::Card, true, 0);
        assert_eq!(empty.ask_leave(), Leave::Go);
        assert_eq!(empty.view(0).discard, None);
        assert_eq!(empty.from(), AddFrom::Card);
    }
}
