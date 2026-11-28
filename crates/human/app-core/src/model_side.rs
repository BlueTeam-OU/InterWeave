// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The model side: owns the [`UiModel`], applies the facade side's
//! [`Update`]s to it, drives a view through [`Surface`], and turns the
//! intents the view resolves into [`Command`]s.

use std::collections::{HashSet, VecDeque};

use interweave_human_chat_protocol::is_allowed_link_scheme;
use interweave_human_client_api::{ClientEvent, SessionState};
use interweave_human_transport_client::AddOutcome;
use interweave_human_ui_model::{
    AddDone, AddEnding, AddForm, AddIntent, AddValues, ConnectGate, ConversationKey, FrontOutcome,
    HeldChange, HeldIdentity, HeldProblem, IdentityIntent, IdentityKey, Intent, Leave, StartBusy,
    TrustInput, UiModel, UnlockFacts, ViewEvent, WindowEvent, WindowInput,
};
use interweave_transport_api::EndpointId;

use crate::facade::CANCELLED_CAP;
use crate::protocol::{
    AddAct, AddTold, CardSaved, Command, Crossing, Failure, Update, WindowCommand,
};

/// A view as the root drives it: show the model, then hand back what the
/// person did. `ui-slint`'s `View` is one; a test can script one.
pub trait Surface {
    /// Show `model`.
    fn render(&mut self, model: &UiModel);
    /// What the person did since the last call, resolved against `model`.
    /// A call returns at most up to the first draft edit, which the root
    /// applies before calling again.
    fn take_events(&mut self, model: &UiModel) -> Vec<ViewEvent>;
    /// What the person did to the windowed mode since the last call: the
    /// open conversation's window, the avatars in view, the Contacts
    /// list's Try again. None by default: a surface that draws no window
    /// reports nothing.
    fn take_window(&mut self) -> Vec<WindowEvent> {
        Vec::new()
    }
    /// A fresh model now reads this surface: what it reported to an
    /// earlier one is reported again (Android makes a model per running
    /// state and per attach, under one view; #517 review F1). Nothing by
    /// default: a surface that reports nothing forgets nothing.
    fn model_made(&mut self) {}
    /// The identity in front changes from `old` to `new` (identities
    /// §5.3): `left` is `old`'s model, its switch snackbar closed, and the
    /// next render is of `new`'s. Nothing by default: a surface holding
    /// nothing per identity has nothing to change.
    fn front_changed(&mut self, _old: &EndpointId, _new: &EndpointId, _left: &UiModel) {}
    /// What the person did to the identity switch, the switcher and their
    /// rows since the last call ([`ModelSide::take_identity_acts`]). None
    /// by default: a surface that draws no switch reports nothing.
    fn take_identity_intents(&mut self) -> Vec<IdentityIntent> {
        Vec::new()
    }
    /// What the person did on the add form since the last call
    /// ([`ModelSide::take_add_acts`]). None by default: a surface that
    /// draws no form reports nothing.
    fn take_add_intents(&mut self) -> Vec<AddIntent> {
        Vec::new()
    }
    /// Show the page of an identity behind the front, opened from its row
    /// of Your identities: `model` is THAT identity's, whose card the page
    /// edits ([`ModelSide::page_turn`]). Nothing by default: a surface
    /// that draws no such page has nothing to show.
    fn render_page(&mut self, _model: &UiModel) {}
    /// What the person did on that page since the last call, resolved
    /// against `model`, that identity's. None by default.
    fn take_page_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
        Vec::new()
    }
}

/// Whether `intent` is one a page of an identity behind the front
/// carries out as that identity: its card's Save, its photo, its key.
/// Anything else on that page is dropped, so no other act can be made as
/// an identity the person is not using
/// (`only_the_cards_acts_leave_a_page_of_an_identity_behind`).
const fn page_act(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::SetOwnCard { .. }
            | Intent::ChooseOwnPhoto(_)
            | Intent::ForgetOwnPhoto
            | Intent::RemoveOwnPhoto
            | Intent::RetryKey
            | Intent::CloseKeyStep
    )
}

/// The add form and the list's acts: the profile's, not one identity's,
/// so what they show is told to every model held
/// (`the_form_reaches_every_model_and_a_switch_keeps_it`).
#[derive(Default)]
struct AddSide {
    /// The root can add an identity: false until it says so.
    adds: bool,
    /// A first-identity seed was answered in this run, as told: a form
    /// opened later does not ask again.
    seeded: bool,
    /// The one form open.
    form: Option<AddForm>,
    /// What goes to the facade side, in the person's order.
    out: Vec<AddAct>,
    /// Done, until the held list names its identity.
    done: Option<AddDone>,
    /// The clock as the root last gave it, for a form opened by an
    /// identity act, which carries none.
    now: u64,
}

impl AddSide {
    /// Tell `model` whether the root can add and the form as it is.
    fn tell(&self, model: &mut UiModel) {
        let identities = model.identities_mut();
        identities.set_adds(self.adds);
        identities.set_form(self.form.clone());
    }
}

/// How a Create ended, as the form shows it.
fn ending(outcome: AddOutcome, card: Option<CardSaved>) -> AddEnding {
    match outcome {
        AddOutcome::Created { endpoint, .. } => AddEnding::Done {
            key: IdentityKey::new(endpoint.as_str()),
            card_saved: card == Some(CardSaved::Saved),
        },
        AddOutcome::NotFinished => AddEnding::NotFinished,
        AddOutcome::Failed => AddEnding::Failed,
        AddOutcome::Unknown => AddEnding::Unknown,
        AddOutcome::Limit => AddEnding::Limit,
        AddOutcome::KeyGone => AddEnding::KeyGone,
    }
}

/// Opens a link a person activated, outside the client: a browser, a mail
/// program. It never reaches the facade.
pub trait Opener {
    /// Open `destination`, which is an allowlisted link.
    fn open(&mut self, destination: &str);

    /// Ask the person for the network access the platform withholds: its
    /// prompt while it still offers one, else the app's settings page
    /// (human-client-android.md, "Runtime permissions and the
    /// network-denied state"). Raised only after the root said access is
    /// withheld ([`ModelSide::network_access`]).
    fn ask_network_access(&mut self);

    /// Raise the platform's authentication prompt for a profile whose key
    /// needs the person. Raised only while the root says it is needed
    /// ([`ModelSide::authentication_needed`]).
    fn authenticate(&mut self);

    /// Onboarding's Connect is no longer owed: forget that it was, so no
    /// later start opens it (add-a-peer §5.1). Raised once, only after the
    /// root said it was owed ([`ModelSide::connect_owed`]); a platform
    /// with no onboarding never owes it
    /// (`connect_settled_reaches_the_platform_once_and_only_when_owed`).
    fn connect_settled(&mut self) {}

    /// The Trust page's P-up line was seen in the start counted `start`
    /// (unlock note section 4): kept by the platform, in memory and never
    /// stored, so a model made again for that start does not show the
    /// Chats notice again. Raised once per start, only after the root said
    /// that start's facts ([`ModelSide::unlock_facts`]); a platform with
    /// no unlock policy never says any.
    fn unlock_line_seen(&mut self, _start: u64) {}

    /// Make one service start for the notice of a start refused with the
    /// profile in use (`SessionNotice::StartBusy`), or of a start that
    /// compared with C4's `PeerId` and was refused after the bound, or
    /// before it from a held R2, R3 or R4 that opened Chats in its place
    /// (`SessionNotice::StartRefused`), by the path a granted Unlock uses
    /// (design/2026-10-19-android-identity.md §11d, Q12; §6, An answer
    /// after the bound; §8, A compared start answered before the bound).
    /// Raised only while the model offers it -- either
    /// notice shows and no start for it runs -- so at most once per
    /// answer; a platform that never says either notice never raises it
    /// (`a_busy_starts_try_again_reaches_the_platform_once`,
    /// `a_refused_starts_try_again_reaches_the_platform_once`).
    fn retry_start(&mut self) {}
}

/// An action still being carried out, so a second press of the same one
/// is not sent twice: a render's "viewed" can raise `MarkRead` again
/// before the first one is answered, and a person can press Send twice.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum InFlight {
    /// One send per conversation at a time.
    Send(ConversationKey),
    /// Any other command, by its value.
    Other(Command),
}

impl InFlight {
    fn of(command: &Command) -> Self {
        match command {
            Command::Send { key, .. } => Self::Send(key.clone()),
            other => Self::Other(other.clone()),
        }
    }
}

/// Something that did not happen and that a person or a log should hear
/// about: the root shows or records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// A command could not be carried out; nothing changed.
    Command {
        /// The command.
        command: Command,
        /// Why.
        why: Failure,
    },
    /// The store's unread content could not be read again: the summaries,
    /// or a card it committed.
    UnreadNotListed(Failure),
    /// The cards could not be read: back after an answer or its Undo, or
    /// the own cards on their way ([`Update::CardsNotRead`]).
    CardsNotRead(Failure),
    /// An own card was accepted and its share could not be recorded yet;
    /// the facade writes it again at each turn.
    ShareNotRecorded(Failure),
}

/// How many problems wait for [`ModelSide::take_problems`], at most; past
/// it the oldest goes and is counted in [`ModelSide::problems_dropped`].
/// A `MarkRead` the store keeps refusing is raised again at every focused
/// render, so a root that never takes them must not grow this.
pub const PROBLEM_CAP: usize = 32;

/// An identity's model while another identity is in front: what the
/// model side holds per identity (ux ruling U2), the same fields the
/// front's model is held in on [`ModelSide`] itself.
struct Behind {
    model: UiModel,
    in_flight: HashSet<InFlight>,
    hidden_unread: usize,
    hidden_other: usize,
    kept: usize,
    reads_owed: bool,
}

impl Behind {
    fn new() -> Self {
        Self {
            model: UiModel::new(),
            in_flight: HashSet::new(),
            hidden_unread: 0,
            hidden_other: 0,
            kept: 0,
            reads_owed: false,
        }
    }
}

/// What the root tells the model side once, about the whole profile
/// rather than one identity: every model held, in front or behind, reads
/// it, so a switch never brings in a model that missed a fact told while
/// it was behind.
///
/// The start's facts are here too: they are the Service's start, which
/// every held identity starts in or not together. Two of them a model
/// also changes itself -- Try again turns a start notice to Starting
/// ([`UiModel::retry_start_asked`]), the Trust page marks the P-up line
/// seen ([`UiModel::unlock_line_seen`]) -- so each such change is made
/// here first and told to every model; a copy left behind would undo it
/// at the next tell
/// (`a_retry_and_a_seen_line_outlive_the_next_fact_and_a_switch`).
#[derive(Debug, Clone, Copy)]
struct RootFacts {
    daemon: Option<bool>,
    network_allowed: bool,
    authentication_needed: bool,
    start_busy: Option<StartBusy>,
    start_refused: Option<StartBusy>,
    unlock: Option<UnlockFacts>,
    unlock_start: u64,
    unlock_seen: bool,
}

impl Default for RootFacts {
    fn default() -> Self {
        // What a fresh model reads before it is told anything.
        Self {
            daemon: None,
            network_allowed: true,
            authentication_needed: false,
            start_busy: None,
            start_refused: None,
            unlock: None,
            unlock_start: 0,
            unlock_seen: false,
        }
    }
}

impl RootFacts {
    /// Tell `model` every fact. The destructuring names every field, so a
    /// fact added to [`RootFacts`] does not build until it is carried.
    fn tell(self, model: &mut UiModel) {
        let Self {
            daemon,
            network_allowed,
            authentication_needed,
            start_busy,
            start_refused,
            unlock,
            unlock_start,
            unlock_seen,
        } = self;
        model.daemon_seen(daemon);
        model.network_access(network_allowed);
        model.authentication_needed(authentication_needed);
        model.start_busy(start_busy);
        model.start_refused(start_refused);
        model.unlock_facts(unlock, unlock_start, unlock_seen);
    }

    /// [`UiModel::retry_start_asked`]'s change, made to the facts every
    /// model is told.
    fn retry_start_asked(&mut self) {
        for notice in [&mut self.start_busy, &mut self.start_refused] {
            if notice.is_some() {
                *notice = Some(StartBusy::Starting);
            }
        }
    }
}

/// The model side of the root.
///
/// One [`UiModel`] per held identity behind one [`Surface`] (ux ruling U2
/// of 2026-11-25): the front identity's model is held in the fields
/// below and is the one rendered; every other identity's is in `behind`,
/// where an update for it applies and draws nothing
/// (`an_update_for_an_identity_behind_applies_to_its_model_and_draws_nothing`).
pub struct ModelSide<S: Surface, O: Opener> {
    model: UiModel,
    surface: S,
    opener: O,
    in_flight: HashSet<InFlight>,
    problems: VecDeque<Problem>,
    problems_dropped: u64,
    hidden_unread: usize,
    hidden_other: usize,
    kept: usize,
    /// The trust and address lists are to be read: a session became
    /// ready with either unread, which is the start for a runtime that is
    /// up, and the first moment both can be answered for one that was not.
    reads_owed: bool,
    /// The identity in front, once the root named the identities held
    /// ([`Self::hold`]); `None` for a model side of one that was never
    /// named, which is every root before Stage 22.
    identity: Option<EndpointId>,
    /// The other identities held, each with its model, in held order.
    behind: Vec<(EndpointId, Behind)>,
    /// What the root said about the profile, told to every model held.
    facts: RootFacts,
    /// The add form and the list's acts.
    add: AddSide,
}

impl<S: Surface, O: Opener> ModelSide<S, O> {
    /// A model side over `surface`, opening activated links with `opener`.
    pub fn new(mut surface: S, opener: O) -> Self {
        surface.model_made();
        Self {
            model: UiModel::new(),
            surface,
            opener,
            in_flight: HashSet::new(),
            problems: VecDeque::new(),
            problems_dropped: 0,
            hidden_unread: 0,
            hidden_other: 0,
            kept: 0,
            reads_owed: false,
            identity: None,
            behind: Vec::new(),
            facts: RootFacts::default(),
            add: AddSide::default(),
        }
    }

    /// Tell the front's model and every model behind it the facts.
    fn tell_all(&mut self) {
        self.facts.tell(&mut self.model);
        for (_, behind) in &mut self.behind {
            self.facts.tell(&mut behind.model);
        }
    }

    /// Name the identities held, `front` among them: the model shown so
    /// far becomes `front`'s, and every other identity in `held` is given
    /// a model of its own. The root calls it once, as the facade side says
    /// what it holds, before any update for a named identity; a model side
    /// already named is left as it is, and the call answers false
    /// (`naming_the_held_identities_keeps_the_model_shown_as_the_fronts`).
    pub fn hold(&mut self, held: &[EndpointId], front: &EndpointId) -> bool {
        if self.identity.is_some() {
            return false;
        }
        self.identity = Some(front.clone());
        for identity in held {
            if identity != front && !self.behind.iter().any(|(e, _)| e == identity) {
                let mut made = Behind::new();
                self.facts.tell(&mut made.model);
                self.add.tell(&mut made.model);
                self.behind.push((identity.clone(), made));
            }
        }
        true
    }

    /// Name identities held since [`Self::hold`]: one made while the
    /// client runs ([`crate::Roster::take_held_more`]). Each not yet held
    /// is given a model of its own behind the front, told the facts every
    /// model is told; the front does not change. Called before the held
    /// list that names them ([`Self::held_changed`]) and before any update
    /// for them. False, and nothing held, on a model side never named.
    /// (`an_identity_held_later_gets_a_model_of_its_own`)
    pub fn hold_more(&mut self, more: &[EndpointId]) -> bool {
        if self.identity.is_none() {
            return false;
        }
        for identity in more {
            if self.identity.as_ref() != Some(identity)
                && !self.behind.iter().any(|(e, _)| e == identity)
            {
                let mut made = Behind::new();
                self.facts.tell(&mut made.model);
                self.add.tell(&mut made.model);
                self.behind.push((identity.clone(), made));
            }
        }
        true
    }

    /// The held identities as the facade side read them, told to every
    /// model held, in front or behind
    /// ([`interweave_human_ui_model::Identities::held_changed`]): each
    /// model is its own identity's, so each is told it is in front of its
    /// own ([`interweave_human_ui_model::Identities::start`]) the first
    /// time it is not. The front's [`HeldChange`] is returned
    /// (`the_held_identities_reach_every_model_each_in_front_of_its_own`).
    ///
    /// # Errors
    /// A list the model refuses ([`HeldProblem`]): every model keeps what
    /// it held.
    pub fn held_changed(&mut self, held: &[HeldIdentity]) -> Result<HeldChange, HeldProblem> {
        let own = |identity: Option<&EndpointId>| identity.map(|e| IdentityKey::new(e.as_str()));
        let tell = |model: &mut UiModel, own: Option<IdentityKey>| {
            let identities = model.identities_mut();
            let change = identities.held_changed(held.to_vec());
            if change.is_ok()
                && let Some(own) = own
                && identities.front() != Some(&own)
            {
                identities.start(Some(own));
            }
            change
        };
        // The front's first: a refused list is refused alike by every
        // model, so none is told when the front's is not.
        let change = tell(&mut self.model, own(self.identity.as_ref()))?;
        for (identity, behind) in &mut self.behind {
            let _ = tell(&mut behind.model, own(Some(identity)));
        }
        // Done waits for this list to name its identity, so the row it
        // focuses exists (F6).
        self.done_if_held();
        Ok(change)
    }

    /// Carry out what the person did to the identities, sending what the
    /// facade side must do as crossings.
    ///
    /// Try again on an identity that could not start becomes a
    /// [`Command::Reopen`] run as THAT identity, whichever is in front
    /// (identities §5.2;
    /// `try_again_reopens_the_identity_it_names_and_no_other`); one
    /// naming an identity not held sends nothing.
    ///
    /// Every other act is the front's model's to decide
    /// ([`interweave_human_ui_model::Identities::act`]): opening and
    /// closing the switcher, a snackbar closing and Your identities change
    /// what is drawn and send no crossing -- the list's opening and its
    /// Add go to the facade side as add acts, taken by
    /// [`Self::take_add_acts`] -- and a choice or Switch back that the
    /// model decides is a switch brings that identity in front through
    /// [`Self::switch`], in its order, and the shown switch state -- the
    /// switcher closed, the switch's snackbar naming the new identity, the
    /// focus asked for the switch -- goes in front with it
    /// (`a_choice_in_the_switcher_brings_the_identity_in_front_in_the_switchs_order`).
    pub fn take_identity_acts(&mut self, mut send: impl FnMut(Crossing)) {
        for intent in self.surface.take_identity_intents() {
            if let IdentityIntent::TryAgain(key) = intent {
                if let Some(identity) = self.held_endpoint(&key) {
                    send(Crossing::Command {
                        identity,
                        command: Command::Reopen,
                    });
                }
                continue;
            }
            self.identity_act(intent, &mut send);
        }
    }

    /// The held identity `key` names, in front or behind.
    fn held_endpoint(&self, key: &IdentityKey) -> Option<EndpointId> {
        self.identity
            .iter()
            .chain(self.behind.iter().map(|(e, _)| e))
            .find(|e| e.as_str() == key.as_str())
            .cloned()
    }

    /// One act on the switch, the switcher or the switch's snackbar.
    ///
    /// The act is decided on a copy of the front's identities, so that
    /// the old front's model, which the switch's drain draws once more,
    /// still names its own identity: no frame shows the new identity's
    /// name over the old identity's rows (identities §5.4). The decided
    /// copy is then the new front's: its snackbar and focus counts carry
    /// on from the ones the view last drew, so the view announces the new
    /// snackbar and moves focus once.
    fn identity_act(&mut self, intent: IdentityIntent, send: &mut impl FnMut(Crossing)) {
        let mut decided = self.model.identities().clone();
        let was_listed = decided.identities_open();
        // No identity's page with unsaved changes reaches the model on
        // this root: the desktop's own card editor asks its discard
        // question in the view, the add form's leaving asks in the form
        // ([`Self::take_add_acts`]), and no act answers
        // `Identities::discard_confirmed` yet.
        let outcome = decided.act(intent, false);
        // The list's opening is a settle moment, once per opening, and
        // only on a root that can add
        // (`opening_the_list_sends_one_settle_moment_per_opening`).
        if self.add.adds && !was_listed && decided.identities_open() {
            self.add.out.push(AddAct::ListOpened);
        }
        let switched = match outcome {
            FrontOutcome::Switched(switched) => switched,
            other => {
                *self.model.identities_mut() = decided;
                if let FrontOutcome::OpenAdd(from) = other
                    && self.add.adds
                    && self.add.form.is_none()
                {
                    self.add.form = Some(AddForm::open(from, self.add.seeded, self.add.now));
                    self.add.out.push(AddAct::Opened);
                    self.tell_add();
                }
                return;
            }
        };
        let old = self.identity.clone();
        let switched_in = self
            .held_endpoint(&switched.to)
            .is_some_and(|to| self.switch(&to, &mut *send));
        if !switched_in {
            // Held by the list and with no model here (added after the
            // identities were named, and not named since by
            // `hold_more`): the switcher closes, nothing
            // switches, focus back on the switch, as a choice of the row
            // in front.
            self.model
                .identities_mut()
                .act(IdentityIntent::CloseSwitcher, false);
            return;
        }
        *self.model.identities_mut() = decided;
        // The model left behind names its own identity again, its switcher
        // closed: a later switch back replaces what it shows anyway, and
        // a held list told meanwhile finds it in front of its own.
        if let Some(old) = old
            && let Some((_, behind)) = self.behind.iter_mut().find(|(e, _)| *e == old)
        {
            behind
                .model
                .identities_mut()
                .start(Some(IdentityKey::new(old.as_str())));
        }
    }

    /// The page of an identity behind the front, while the front's Your
    /// identities shows it (identities §6.2 step 4): drawn from that
    /// identity's own model, and its card's acts -- Save, the photo, the
    /// key -- carried out as THAT identity, each sent as a crossing naming
    /// it, whichever identity is in front
    /// (`save_on_that_page_writes_that_identitys_card_and_no_other`). The
    /// page of the identity in front is its own model's editor, which
    /// [`Self::turn`] draws and carries; nothing is done here for it.
    pub fn page_turn(&mut self, mut send: impl FnMut(Crossing)) {
        let Some(page) = self.model.identities().identity_page() else {
            return;
        };
        let Some(i) = self
            .behind
            .iter()
            .position(|(e, _)| e.as_str() == page.key.as_str())
        else {
            return;
        };
        let (identity, mut behind) = self.behind.remove(i);
        // That identity's model in the front's fields for the page, as
        // `apply_as` does, so its in-flight marks are its own.
        self.exchange(&mut behind);
        self.surface.render_page(&self.model);
        let mut commands = Vec::new();
        for event in self.surface.take_page_events(&self.model) {
            if let ViewEvent::Intent(intent) = event
                && page_act(&intent)
                && let Some(command) = self.command_for(intent)
            {
                commands.push(command);
            }
        }
        self.exchange(&mut behind);
        self.behind.insert(i, (identity.clone(), behind));
        for command in commands {
            send(Crossing::Command {
                identity: identity.clone(),
                command,
            });
        }
    }

    /// Tell every model held, in front or behind, the add form and whether
    /// the root can add.
    fn tell_add(&mut self) {
        self.add.tell(&mut self.model);
        for (_, behind) in &mut self.behind {
            self.add.tell(&mut behind.model);
        }
    }

    /// Whether this root can add an identity: a root composed with an add
    /// factory can (`crate::Roster::adding`). Until it says so no Add is
    /// drawn and no act is sent (`a_root_that_cannot_add_sends_no_act`).
    pub fn can_add(&mut self, adds: bool) {
        self.add.adds = adds;
        if !adds {
            self.add.form = None;
            self.add.out.clear();
        }
        self.tell_add();
    }

    /// What the person did on the add form since the last call, at `now`
    /// (ms), and what the list's acts asked: the acts for the facade side
    /// ([`crate::Roster::add`]), in the person's order
    /// (`the_forms_acts_reach_the_facade_in_the_persons_order`). Create
    /// goes once the form's own rules let it, carrying the form's card;
    /// Try again is the form opened again, which asks the seed once more;
    /// a leaving goes once it no longer asks. A root that cannot add takes
    /// nothing.
    pub fn take_add_acts(&mut self, now: u64) -> Vec<AddAct> {
        self.add.now = now;
        let intents = self.surface.take_add_intents();
        if !self.add.adds {
            return Vec::new();
        }
        let mut changed = false;
        for intent in intents {
            let Some(form) = self.add.form.as_mut() else {
                break;
            };
            changed = true;
            let mut left = false;
            match intent {
                AddIntent::Edited { name, surname } => {
                    let photo = form.values().photo;
                    form.edit(AddValues {
                        name,
                        surname,
                        photo,
                    });
                }
                AddIntent::Focused(at) => form.focused(at),
                AddIntent::ChoosePhoto(chosen) => {
                    if let Some(seq) = form.photo_chosen() {
                        self.add.out.push(AddAct::PreparePhoto { chosen, seq });
                    }
                }
                AddIntent::RemovePhoto => {
                    form.photo_removed();
                }
                AddIntent::Create => {
                    if form.create_checked() {
                        let (name, surname, photo) = form.card();
                        self.add.out.push(AddAct::Create {
                            name,
                            surname,
                            photo,
                        });
                    }
                }
                AddIntent::TryAgain => {
                    if form.try_again(now) {
                        self.add.out.push(AddAct::Opened);
                    }
                }
                AddIntent::Leave => left = form.ask_leave() == Leave::Go,
                AddIntent::Discard(close) => left = form.discard_answered(close),
            }
            if left {
                self.add.form = None;
                self.add.out.push(AddAct::Left);
            }
        }
        if changed {
            self.tell_add();
        }
        std::mem::take(&mut self.add.out)
    }

    /// What became of an add act or a settle, at `now` (ms), for the form
    /// and the list to show. A told for a form already left, or closed at
    /// Done, changes nothing (`a_told_for_a_form_already_left_changes_nothing`);
    /// Done closes the form, and Your identities opens on the new row
    /// once the held list names it
    /// (`done_opens_the_list_and_focuses_the_new_row_once_the_held_list_names_it`),
    /// or at once when its identity could not be held yet (Q-U5).
    pub fn add_told(&mut self, told: AddTold, now: u64) {
        self.add.now = now;
        if told == AddTold::Offered {
            self.add.seeded = true;
        }
        // Done's identity could not be held yet: the list opens now, its
        // title focused, and the row comes when it is held (Q-U5).
        if let AddTold::NotHeld(id) | AddTold::NotHeldUntilRestart(id) = &told
            && let Some(done) = self
                .add
                .done
                .take_if(|done| done.key.as_str() == id.as_str())
        {
            self.apply_done(&done);
            return;
        }
        let Some(form) = self.add.form.as_mut() else {
            return;
        };
        let mut done = None;
        match told {
            AddTold::Offered => form.seed_told(true),
            AddTold::NoSeed => form.seed_told(false),
            // Nothing was made, written or sent.
            AddTold::OutOfBounds | AddTold::NotStarted => {
                form.ended(AddEnding::Failed);
            }
            AddTold::Ended { outcome, card } => done = form.ended(ending(outcome, card)),
            AddTold::PhotoPrepared { seq, photo } => {
                form.photo_prepared(seq, photo.map(|p| (p.bytes, p.shown)));
            }
            AddTold::WaitsSeed
            | AddTold::Held(_)
            | AddTold::NotHeld(_)
            | AddTold::NotHeldUntilRestart(_) => {}
        }
        if let Some(done) = done {
            self.add.form = None;
            self.add.done = Some(done);
            self.done_if_held();
        }
        self.tell_add();
    }

    /// Done's identity is named by the held list: Your identities opens
    /// on its row in every model.
    fn done_if_held(&mut self) {
        let held = self.add.done.as_ref().is_some_and(|done| {
            self.model
                .identities()
                .held()
                .iter()
                .any(|h| h.key == done.key)
        });
        if held && let Some(done) = self.add.done.take() {
            self.apply_done(&done);
        }
    }

    /// Done, shown: in every model, so a switch shows the list's state as
    /// it was.
    fn apply_done(&mut self, done: &AddDone) {
        let models = std::iter::once(&mut self.model)
            .chain(self.behind.iter_mut().map(|(_, b)| &mut b.model));
        for model in models {
            let identities = model.identities_mut();
            identities.added(&done.key, done.kept.clone());
            if let Some(said) = &done.snackbar {
                identities.confirm_added(said.clone());
            }
        }
    }

    /// The identity in front, once named ([`Self::hold`]).
    #[must_use]
    pub const fn front(&self) -> Option<&EndpointId> {
        self.identity.as_ref()
    }

    /// An identity's model: the one in front, or one behind it.
    #[must_use]
    pub fn model_of(&self, identity: &EndpointId) -> Option<&UiModel> {
        if self.identity.as_ref() == Some(identity) {
            return Some(&self.model);
        }
        self.behind
            .iter()
            .find(|(e, _)| e == identity)
            .map(|(_, b)| &b.model)
    }

    /// Exchange the front's model with `behind`'s, every per-identity
    /// field of it. The destructuring is whole, so a field added to
    /// [`Behind`] does not build until it is exchanged here.
    fn exchange(&mut self, behind: &mut Behind) {
        let Behind {
            model,
            in_flight,
            hidden_unread,
            hidden_other,
            kept,
            reads_owed,
        } = behind;
        std::mem::swap(&mut self.model, model);
        std::mem::swap(&mut self.in_flight, in_flight);
        std::mem::swap(&mut self.hidden_unread, hidden_unread);
        std::mem::swap(&mut self.hidden_other, hidden_other);
        std::mem::swap(&mut self.kept, kept);
        std::mem::swap(&mut self.reads_owed, reads_owed);
    }

    /// Apply what the facade side did as `identity`: to the front's model
    /// as [`Self::apply`] does, or to the model of an identity behind it,
    /// which draws nothing until that identity is in front. False when
    /// `identity` is not held, or the model side was never named: nothing
    /// was applied.
    pub fn apply_as(&mut self, identity: &EndpointId, update: Update) -> bool {
        if self.identity.as_ref() == Some(identity) {
            self.apply(update);
            return true;
        }
        let Some(i) = self.behind.iter().position(|(e, _)| e == identity) else {
            return false;
        };
        let (named, mut behind) = self.behind.remove(i);
        // The update is applied as the front's would be, then the models
        // go back: applying touches neither the surface nor the opener.
        self.exchange(&mut behind);
        self.apply(update);
        self.exchange(&mut behind);
        self.behind.insert(i, (named, behind));
        true
    }

    /// Bring `to` in front, the model side's half of a switch, in its
    /// order (ruling of 2026-11-25): what the person did under the old
    /// front that its model has not yet asked is drained into commands
    /// and sent as the old front's; the old front's switch snackbar
    /// closes; the surface is told; the models change places; and the
    /// switch is sent, naming `to`. Everything goes through `send`, in
    /// that order, so a facade that carries out what it is sent in order
    /// runs the old front's commands before its own swap
    /// (`a_switch_drains_sends_closes_tells_then_names_the_new_front`).
    /// False, and nothing done, when `to` is in front, is not held, or
    /// the model side was never named.
    pub fn switch(&mut self, to: &EndpointId, mut send: impl FnMut(Crossing)) -> bool {
        let Some(old) = self.identity.clone() else {
            return false;
        };
        let Some(i) = self.behind.iter().position(|(e, _)| e == to) else {
            return false;
        };
        for command in self.turn() {
            send(Crossing::Command {
                identity: old.clone(),
                command,
            });
        }
        if let Some(id) = self.model.identities().switch_snackbar().map(|s| s.id) {
            self.model
                .identities_mut()
                .act(IdentityIntent::SnackbarClosed(id), false);
        }
        self.surface.front_changed(&old, to, &self.model);
        let (_, mut behind) = self.behind.remove(i);
        self.exchange(&mut behind);
        self.behind.insert(i, (old, behind));
        self.identity = Some(to.clone());
        send(Crossing::Front(to.clone()));
        true
    }

    /// Onboarding's Connect is owed (add-a-peer §5.1): a new or restored
    /// identity's first start, or a later one whose earlier start left it
    /// undecided. Said before the session's first reads answer; the root
    /// opens it, or settles it through [`Opener::connect_settled`], or
    /// leaves it owed for the next start when a read failed.
    pub fn connect_owed(&mut self) {
        self.model.connect_owed();
    }

    /// This start gives an owed Connect up -- the 10 s bound passed, or
    /// at a launch the person opened something from Chats first
    /// (add-a-peer §5.1) -- and the surface draws it at once: the trace
    /// says `connect=deferred` and a driver waits on it, and no turn may
    /// follow to draw it. True when Connect was still owed.
    pub fn connect_defer(&mut self) -> bool {
        let deferred = self.model.connect_defer();
        if deferred {
            self.surface.render(&self.model);
        }
        deferred
    }

    /// Whether a transport daemon serves the profile, as the root sees
    /// it: the facade cannot tell a missing daemon from any other failed
    /// open, the root can.
    pub fn daemon_seen(&mut self, present: Option<bool>) {
        self.facts.daemon = present;
        self.tell_all();
    }

    /// The service stopped under the models, which a root keeps showing
    /// -- their rows, their open conversation, their drafts -- while the
    /// start that follows is refused for the person (unlock note §11).
    /// What a kept model must not do is what review F3 found in one: read
    /// Ready, and hold a command in flight no facade will answer. So in
    /// every identity held, in front or behind (the service is the
    /// profile's, not one identity's), every command in flight fails as
    /// [`Failure::ServiceStopped`], and the session reads as a fresh
    /// model's with no service: not running. Counts and kept state are
    /// left as they are, as the front's always were
    /// (`a_stop_reaches_the_identity_behind_as_it_does_the_front`).
    pub fn stopped(&mut self) {
        self.stop_front();
        // Each identity behind is brought in front of its own fields for
        // the stop, as `apply_as` does, so one routine serves both.
        let mut behind = std::mem::take(&mut self.behind);
        for (_, held) in &mut behind {
            self.exchange(held);
            self.stop_front();
            self.exchange(held);
        }
        self.behind = behind;
        self.daemon_seen(Some(false));
    }

    /// The stop of [`Self::stopped`] for the model held in the front's
    /// fields: its commands in flight failed and its session reading
    /// "not running".
    fn stop_front(&mut self) {
        let in_flight: Vec<InFlight> = self.in_flight.drain().collect();
        for waiting in in_flight {
            match waiting {
                InFlight::Other(command) => self.apply(Update::Failed {
                    command,
                    why: Failure::ServiceStopped,
                }),
                // Its text is the store's or the composer's already: only
                // the claim that one send waits goes.
                InFlight::Send(key) => self.problem(Problem::Command {
                    command: Command::Send {
                        key,
                        draft: String::new(),
                    },
                    why: Failure::ServiceStopped,
                }),
            }
        }
        // The session a model made with no service reads: re-opening, said
        // as "not running" while nothing serves the profile.
        self.model
            .client_event(ClientEvent::Session(SessionState::Reconnecting {
                attempt: 0,
                next_at: 0,
            }));
    }

    /// Every draft holding text ([`UiModel::drafts`]) of every identity
    /// held, in front or behind, each named by its identity, for the side
    /// a new runtime's start makes. The front's come first. Before the
    /// held identities are named ([`Self::hold`]) the front has no
    /// identity to name its drafts by, and there are none to give.
    #[must_use]
    pub fn held_drafts(&self) -> Vec<(EndpointId, ConversationKey, String)> {
        let front = self.identity.iter().map(|e| (e, &self.model));
        let behind = self.behind.iter().map(|(e, b)| (e, &b.model));
        front
            .chain(behind)
            .flat_map(|(identity, model)| {
                model
                    .drafts()
                    .into_iter()
                    .map(|(key, draft)| (identity.clone(), key, draft))
            })
            .collect()
    }

    /// Carry `drafts` over from the side before this one, each into the
    /// model of the identity it was written as: what is written waits
    /// across a restart (unlock note §11, A17). A draft of an identity
    /// this side does not hold is dropped, there being no model to hold
    /// it; later drafts of one conversation win.
    pub fn carry_held_drafts(&mut self, drafts: Vec<(EndpointId, ConversationKey, String)>) {
        for (identity, key, draft) in drafts {
            if self.identity.as_ref() == Some(&identity) {
                self.model.draft_changed(key, draft);
            } else if let Some((_, behind)) = self.behind.iter_mut().find(|(e, _)| *e == identity) {
                behind.model.draft_changed(key, draft);
            }
        }
    }

    /// This model is shown again over its surface, after another was:
    /// what the surface reported to that one is reported again, as to a
    /// model made new ([`Surface::model_made`]).
    pub fn remade(&mut self) {
        self.surface.model_made();
    }

    /// Replies name what was sent to a peer with no endpoint named, for a
    /// root that prints ids rather than drawing the line
    /// ([`UiModel::name_replies_to_default_sends`]).
    pub fn name_replies_to_default_sends(&mut self) {
        self.model.name_replies_to_default_sends();
    }

    /// Whether the platform gives this app network access, as the root
    /// sees it ([`UiModel::network_access`]).
    pub fn network_access(&mut self, allowed: bool) {
        self.facts.network_allowed = allowed;
        self.tell_all();
    }

    /// Whether nothing can run until the person authenticates, as the
    /// root learns it from the platform.
    pub fn authentication_needed(&mut self, needed: bool) {
        self.facts.authentication_needed = needed;
        self.tell_all();
    }

    /// Whether the last start was refused with the profile in use after
    /// the bound, and whether a start for its notice runs, as the root
    /// learns it from the platform.
    pub fn start_busy(&mut self, busy: Option<StartBusy>) {
        self.facts.start_busy = busy;
        self.tell_all();
    }

    /// Whether the last start compared with C4's `PeerId` and was refused
    /// after the bound, or before it from a held R2, R3 or R4 that opened
    /// Chats in its place, and whether a start for its notice runs, as the
    /// root learns it from the platform ([`UiModel::start_refused`]).
    pub fn start_refused(&mut self, refused: Option<StartBusy>) {
        self.facts.start_refused = refused;
        self.tell_all();
    }

    /// The unlock note's facts as the platform says them, the count of
    /// the start they are from, and whether its P-up line was seen.
    pub fn unlock_facts(&mut self, facts: Option<UnlockFacts>, start: u64, seen: bool) {
        self.facts.unlock = facts;
        self.facts.unlock_start = start;
        self.facts.unlock_seen = seen;
        self.tell_all();
    }

    /// What did not happen since the last call, oldest first.
    pub fn take_problems(&mut self) -> Vec<Problem> {
        self.problems.drain(..).collect()
    }

    /// How many problems were dropped past [`PROBLEM_CAP`].
    #[must_use]
    pub const fn problems_dropped(&self) -> u64 {
        self.problems_dropped
    }

    /// How many stored rows the last listings could not decode for
    /// display: held in the store, never shown. The root says so.
    #[must_use]
    pub const fn hidden_rows(&self) -> usize {
        self.hidden_unread + self.hidden_other
    }

    /// How many kept rows of a kind this build shows the store held at
    /// the windowed start or the last relist, whichever came last: a Keep
    /// or an Unkeep since is not in it; 0 before a windowed start.
    #[must_use]
    pub const fn kept_rows(&self) -> usize {
        self.kept
    }

    /// The store at a windowed start, applied.
    fn started(&mut self, start: crate::protocol::Start) {
        let crate::protocol::Start {
            pending,
            summaries,
            counted_through,
            undecodable_unread,
            undecodable_other,
            kept,
            contacts,
            identity,
        } = start;
        self.model.started(pending, summaries, counted_through);
        self.hidden_unread = undecodable_unread;
        self.hidden_other = undecodable_other;
        self.kept = kept;
        self.model.contacts_directory(contacts.map_err(|_| ()));
        if let Some(identity) = identity {
            self.model.identity_shown(identity);
        }
    }

    fn problem(&mut self, problem: Problem) {
        self.problems.push_back(problem);
        while self.problems.len() > PROBLEM_CAP {
            self.problems.pop_front();
            self.problems_dropped += 1;
        }
    }

    /// The model.
    #[must_use]
    pub const fn model(&self) -> &UiModel {
        &self.model
    }

    /// The surface.
    #[must_use]
    pub const fn surface(&self) -> &S {
        &self.surface
    }

    /// The surface, to drive the platform's half of it (window focus).
    pub const fn surface_mut(&mut self) -> &mut S {
        &mut self.surface
    }

    /// Apply what the facade side did.
    pub fn apply(&mut self, update: Update) {
        self.apply_update(update);
        // Chats ends with the count of rows the app cannot show (design
        // section 6.1), in either mode.
        self.model.set_rows_not_shown(self.hidden_rows());
    }

    fn apply_update(&mut self, update: Update) {
        match update {
            Update::Started(start) => self.started(*start),
            Update::Page(rows) => self.model.page_arrived(*rows),
            Update::ReplyTarget { ask, stored } => self.model.reply_target_answered(&ask, stored),
            Update::ContactDirectory(answer) => {
                self.model.contacts_directory(answer.map_err(|_| ()));
            }
            Update::ContactPhoto {
                contact_id,
                photo_at,
                photo,
            } => self.model.photo_arrived(&contact_id, photo_at, photo),
            Update::SummariesListed {
                summaries,
                counted_through,
                undecodable,
                kept,
            } => {
                self.model.summaries_listed(summaries, counted_through);
                // A relist counts the unread rows it cannot show: their
                // count is replaced, the start's count of the others stands.
                self.hidden_unread = undecodable;
                self.kept = kept;
            }
            Update::UnreadNotListed(why) => self.problem(Problem::UnreadNotListed(why)),
            Update::Received(received) => self.model.received(received),
            // Committed by the drain: new to the counts unless they hold
            // it already.
            Update::ReceivedCard(proposal) => self.model.card_arrived(*proposal),
            Update::ProposalJudged(proposal) => self.model.card_judged(*proposal),
            Update::CardResolved { row, outcome } => self.model.card_resolved(row, outcome),
            Update::ProposalRestored(proposal) => self.model.card_restored(*proposal),
            Update::ProposalNotRestored(row) => self.model.card_not_restored(row),
            Update::CardsNotRead(why) => self.problem(Problem::CardsNotRead(why)),
            Update::ShareNotRecorded(why) => self.problem(Problem::ShareNotRecorded(why)),
            Update::ContactChanged {
                contact_id,
                change,
                hold,
            } => self.model.contact_changed(contact_id, change, hold),
            Update::ContactRestored { contact_id, change } => {
                self.model.contact_restored(&contact_id, change);
            }
            Update::ContactUndoWithdrawn(hold) => self.model.contact_undo_withdrawn(hold),
            Update::IdentityShown(identity) => self.model.identity_shown(identity),
            Update::OwnPhotoPrepared { seq, photo } => self.model.own_photo_prepared(seq, photo),
            Update::KeyRetried(identity) => self.model.key_retried(identity),
            Update::CardShared { key, at } => self.model.card_shared(key, at),
            // Its own card row (contact-cards section 5.5); a queued card
            // is no share until its Accepted.
            Update::CardQueued {
                key,
                row,
                app_message_id,
                card,
                at,
            } => self.model.card_queued(row, key, app_message_id, card, at),
            Update::Client(event) => {
                // Every session that becomes ready reads both lists: the
                // no-reach notice needs both, a runtime that was not up at
                // start answered neither, and one restarted may hold others
                // (its configuration reloaded). The facade raises Ready
                // only on a change, and a read already on its way asks no
                // second, so a flapping session stacks no reads. The lists
                // as last read stand until the answers come.
                if matches!(event, ClientEvent::Session(SessionState::Ready { .. })) {
                    self.reads_owed = true;
                }
                self.model.client_event(event);
            }
            Update::Diagnostics(diagnostics) => self.model.diagnostics_updated(diagnostics),
            Update::ThisDevice(peer) => self.model.this_device(peer),
            Update::Sent {
                row,
                destination,
                envelope,
                at,
                ..
            } => self.model.sent(row, &destination, envelope, at),
            Update::SendRefused { key, draft, error } => {
                self.model.send_refused(key, draft, &error);
            }
            Update::Read(row) => self.model.read(row),
            Update::Kept { item, row } => self.model.kept(item, row),
            Update::Unkept(row) => self.model.unkept(row),
            Update::Cancelled { row, restorable } => self.model.cancelled(row, restorable),
            Update::Restored { cancelled, row } => self.model.restored(cancelled, row),
            // An admin command is freed by its answer, not only by its
            // `Done`: the model holds its own mark of what is on its way
            // and offers the retry once it is answered, and each platform
            // may deliver the answer and the `Done` in separate turns. A
            // retry taken between them would be dropped as still on its
            // way while the model waits for its answer, and Peers would
            // stay busy for good (`an_admin_retry_answered_before_its_done_is_sent`).
            // The late `Done` frees the retry's entry in its turn, which
            // the model's own mark still guards.
            Update::TrustRead(answer) => {
                self.in_flight.remove(&InFlight::Other(Command::ReadTrust));
                self.model.trust_settings_mut().read(answer);
            }
            Update::TrustSet { change, answer } => {
                self.in_flight
                    .remove(&InFlight::Other(Command::SetTrust(change.clone())));
                self.model.trust_settings_mut().set(change, answer);
            }
            Update::AddressesRead(answer) => {
                self.in_flight
                    .remove(&InFlight::Other(Command::ReadAddresses));
                self.model.addresses_read(answer);
            }
            Update::OwnAddressesRead(answer) => {
                self.in_flight
                    .remove(&InFlight::Other(Command::ReadOwnAddresses));
                self.model.own_addresses_read(answer);
            }
            Update::AddressesAdded { lines, answer } => {
                self.in_flight
                    .remove(&InFlight::Other(Command::AddAddresses(lines.clone())));
                self.model.addresses_added(&lines, answer);
            }
            Update::AddressRemoved { address, answer } => {
                self.in_flight
                    .remove(&InFlight::Other(Command::RemoveAddress(address.clone())));
                self.model.address_removed(&address, answer);
            }
            Update::Done(command) => {
                self.in_flight.remove(&InFlight::of(&command));
            }
            Update::Failed { command, why } => {
                self.in_flight.remove(&InFlight::of(&command));
                if let (Command::Keep { item, .. }, Failure::CopyGone) = (&command, why) {
                    // The content is gone: stop offering what cannot work.
                    self.model.copy_gone(*item);
                }
                if let (
                    Command::AcceptCard { row, .. } | Command::DeclineCard(row),
                    Failure::CopyGone,
                ) = (&command, why)
                {
                    // The card's copy is gone: so is its proposal.
                    self.model.card_gone(*row);
                }
                // A refused Undo closes nothing more, but focus goes where
                // the action itself sent it; a refused Cancel's row, slid
                // out by its swipe, springs back.
                let undone = match &command {
                    Command::Keep { item, from } => Some(Intent::Keep {
                        item: *item,
                        from: *from,
                    }),
                    Command::Unkeep(row) => Some(Intent::Unkeep(*row)),
                    Command::Restore(row) => Some(Intent::Restore(*row)),
                    Command::Cancel(row) => Some(Intent::Cancel(*row)),
                    Command::UndoCard(row) => Some(Intent::UndoCard(*row)),
                    Command::UndoContact {
                        contact_id,
                        change,
                        hold,
                    } => Some(Intent::UndoContact {
                        contact_id: contact_id.clone(),
                        change: *change,
                        hold: *hold,
                    }),
                    _ => None,
                };
                if let Some(intent) = undone {
                    self.model.intent_refused(&intent);
                }
                // A read the window waits on ends here too: the edge shows
                // its failure row, a photo its initials -- but a page no
                // service took was never read, and a Try again could not
                // succeed while none runs (unlock note §11).
                match &command {
                    Command::Window(WindowCommand::FetchPage(ask))
                        if why == Failure::ServiceStopped =>
                    {
                        self.model.page_not_read(ask);
                    }
                    Command::Window(WindowCommand::FetchPage(ask)) => self.model.page_failed(ask),
                    Command::Window(WindowCommand::ReadReplyTarget(ask)) => {
                        self.model.reply_target_failed(ask);
                    }
                    Command::Window(WindowCommand::ReadPhoto(ask)) => {
                        self.model
                            .photo_arrived(&ask.contact_id, ask.photo_at, None);
                    }
                    _ => {}
                }
                self.problem(Problem::Command { command, why });
            }
        }
    }

    /// Render the model, take what the person did, and return the commands
    /// it asks of the facade side. The view's queue is drained before this
    /// returns -- `ui-slint`'s queue bound holds only for a root that does.
    pub fn turn(&mut self) -> Vec<Command> {
        // One render, then takes until one comes back empty. The render
        // comes first because it is what lets the view resolve a
        // conversation shown with unread items in it; a draft edit taken
        // here reaches the screen at the next turn's render, and the view
        // never writes over text it is still holding.
        self.surface.render(&self.model);
        // What closed snackbars no longer need goes first, ahead of any
        // cancel this turn issues.
        let mut commands = Vec::new();
        if std::mem::take(&mut self.reads_owed) {
            let reads = [
                self.model.trust_settings_mut().opened(),
                self.model.read_addresses(),
            ];
            for intent in reads.into_iter().flatten() {
                commands.extend(self.command_for(intent));
            }
        }
        let released = self.model.take_released();
        commands.extend(released.copies.into_iter().map(Command::ForgetCopy));
        commands.extend(released.cancelled.into_iter().map(Command::ForgetCancelled));
        commands.extend(released.cards.into_iter().map(Command::ForgetCard));
        commands.extend(released.contacts.into_iter().map(Command::ForgetContact));
        // A trust input changes what the settings show -- a proposal to
        // confirm, why a typed value was not proposed -- and may ask the
        // facade nothing, so no update would bring another turn: the
        // view would sit on its press. One more render once the takes
        // drain, then takes until empty again (the shipped client over
        // AT-SPI showed a removal never reaching its confirmation). The
        // same holds for what the model does alone: a Hide, an Undo, a
        // snackbar closing (seen on the phone: a hidden row held slid out,
        // no snackbar, until something else turned the root).
        let mut unrendered = false;
        loop {
            let events = self.surface.take_events(&self.model);
            if events.is_empty() {
                // The view's window reports. One that changes what is drawn
                // -- a reach at the session bound shows the limit row, and
                // asks nothing -- is drawn again; the person's place, which
                // a scroll reports on every move, is not
                // (`a_report_that_changes_what_is_drawn_draws_and_a_place_never_does`).
                for event in self.surface.take_window() {
                    if self.model.window_event(event) {
                        unrendered = true;
                    }
                }
                // What a closed or replaced snackbar's Undo no longer needs.
                let released = self.model.take_released();
                commands.extend(released.copies.into_iter().map(Command::ForgetCopy));
                commands.extend(released.cancelled.into_iter().map(Command::ForgetCancelled));
                commands.extend(released.cards.into_iter().map(Command::ForgetCard));
                commands.extend(released.contacts.into_iter().map(Command::ForgetContact));
                // A change's answer may have left the list to read again.
                if let Some(intent) = self.model.trust_settings_mut().take_reread()
                    && let Some(command) = self.command_for(intent)
                {
                    commands.push(command);
                }
                // The same for the addresses, and step two's blocks once
                // trust answered them. A trust answer reaches Peers only
                // here, after this turn's render, and may ask the facade
                // nothing -- a block's problem, a decided page closing,
                // focus moving -- so a Peers that changed is drawn again.
                let before = self.model.peers().clone();
                if let Some(intent) = self.model.peers_turn()
                    && let Some(command) = self.command_for(intent)
                {
                    commands.push(command);
                }
                if *self.model.peers() != before {
                    unrendered = true;
                }
                // Once both start reads answered: Connect opens, is
                // settled, or waits for the next start. While it is open
                // its line follows a trust answer this turn took after
                // its render, which may ask the facade nothing -- its
                // `Done` came in the same batch -- so a Connect that
                // changed is drawn again, as Peers is
                // (`a_late_trust_answer_is_drawn_on_connect_by_the_turn_that_takes_it`).
                // A settle or a defer changes nothing Connect draws, but
                // the phase is drawn too -- the debug trace says it and a
                // driver waits on it -- so a phase that moved is drawn by
                // this turn
                // (`a_connect_settled_or_deferred_is_drawn_by_the_turn_that_decides_it`).
                let before = self.model.connect().cloned();
                let phase = self.model.connect_phase();
                if self.model.connect_turn() == Some(ConnectGate::Settled) {
                    self.opener.connect_settled();
                }
                if self.model.connect() != before.as_ref() || self.model.connect_phase() != phase {
                    unrendered = true;
                }
                if std::mem::take(&mut unrendered) {
                    self.surface.render(&self.model);
                    continue;
                }
                self.window_reads(&mut commands);
                return commands;
            }
            for event in events {
                match event {
                    ViewEvent::DraftChanged { key, draft } => {
                        self.model.draft_changed(key, draft);
                    }
                    ViewEvent::Peers(input) => {
                        unrendered = true;
                        for intent in self.model.peers_input(input) {
                            if let Some(command) = self.command_for(intent) {
                                commands.push(command);
                            }
                        }
                    }
                    ViewEvent::Connect(input) => {
                        unrendered = true;
                        for intent in self.model.connect_input(input) {
                            if let Some(command) = self.command_for(intent) {
                                commands.push(command);
                            }
                        }
                    }
                    ViewEvent::Trust(input) => {
                        unrendered = true;
                        if input == TrustInput::Opened
                            && let Some(start) = self.model.unlock_line_seen()
                        {
                            // Seen for every identity held: a switch does
                            // not offer the line again in this start.
                            self.facts.unlock_seen = true;
                            self.tell_all();
                            self.opener.unlock_line_seen(start);
                        }
                        if let Some(intent) = self.model.trust_settings_mut().input(input)
                            && let Some(command) = self.command_for(intent)
                        {
                            commands.push(command);
                        }
                    }
                    // Trust reaches the daemon only through the settings'
                    // inputs, which confirm the change shown: a trust
                    // intent handed over bare, by any surface, is dropped
                    // (`a_trust_intent_handed_over_bare_reaches_nothing`).
                    // The addresses likewise reach the runtime only through
                    // Peers' inputs, after the client's own rules
                    // (`a_bare_address_intent_reaches_nothing`).
                    ViewEvent::Intent(
                        Intent::ReadTrust
                        | Intent::SetTrust(_)
                        | Intent::ReadAddresses
                        | Intent::AddAddresses(_)
                        | Intent::RemoveAddress(_)
                        | Intent::ReadOwnAddresses,
                    ) => {}
                    ViewEvent::Intent(intent) => {
                        unrendered |=
                            matches!(intent, Intent::Hide(_) | Intent::HideConversation(_));
                        if let Some(command) = self.command_for(intent) {
                            commands.push(command);
                        }
                    }
                    ViewEvent::Undo(id) => {
                        unrendered = true;
                        if let Some(intent) = self.model.undo(id)
                            && let Some(command) = self.command_for(intent)
                        {
                            commands.push(command);
                        }
                    }
                    ViewEvent::SnackbarClosed(id) => {
                        unrendered = true;
                        self.model.snackbar_closed(id);
                    }
                    ViewEvent::IdentifierCopied(key) => {
                        unrendered = true;
                        self.model.identifier_copied(key);
                    }
                    ViewEvent::MessageCopied(key) => {
                        unrendered = true;
                        self.model.message_copied(key);
                    }
                }
            }
        }
    }

    /// What the windowed mode asks to read, since the last turn: its
    /// pages, the contacts, the photos a view shows, and replies' targets
    /// not held (start-window §5.2).
    fn window_reads(&mut self, commands: &mut Vec<Command>) {
        for ask in self.model.take_page_asks() {
            commands.push(Command::Window(WindowCommand::FetchPage(Box::new(ask))));
        }
        if self.model.take_contacts_ask() {
            commands.push(Command::Window(WindowCommand::ReadContacts));
        }
        for ask in self.model.take_photo_asks() {
            commands.push(Command::Window(WindowCommand::ReadPhoto(ask)));
        }
        for ask in self.model.take_reply_asks() {
            commands.push(Command::Window(WindowCommand::ReadReplyTarget(ask)));
        }
    }

    /// TEST BUILDS ONLY: hold the model to bounds below the published caps
    /// (ui-model's `set_bounds_for_test`).
    #[cfg(feature = "test-hooks")]
    pub fn set_bounds_for_test(
        &mut self,
        session_items: usize,
        held_rows: usize,
        read_proposals: usize,
    ) {
        self.model
            .set_bounds_for_test(session_items, held_rows, read_proposals);
    }

    /// The person moved in the open conversation's window, or opened or
    /// closed one: what it asks of the store goes at the next turn.
    pub fn window(&mut self, input: WindowInput) {
        self.model.window_input(input);
    }

    /// A view shows `contact_id`'s avatar: its photo is read, once.
    pub fn photo_shown(&mut self, contact_id: &str) {
        self.model.photo_shown(contact_id);
    }

    /// The avatar left the view.
    pub fn photo_hidden(&mut self, contact_id: &str) {
        self.model.photo_hidden(contact_id);
    }

    /// The Contacts list's Try again: the contacts are read once more.
    pub fn contacts_try_again(&mut self) {
        self.model.contacts_try_again();
    }

    /// The command for `intent`, or `None` when it needs none -- a link is
    /// opened here -- or the same action is still in flight.
    fn command_for(&mut self, intent: Intent) -> Option<Command> {
        // Only what the model offers now: an earlier input of the same take
        // may have changed it (a Dismiss before a Keep of the same message).
        if !self.model.offers(&intent) {
            // What an earlier input of the same take makes no longer
            // offered: a Keep after a Dismiss of the same
            // message, reported, since the person asked for it and nothing
            // else will say so (`a_keep_taken_after_its_dismiss_is_not_issued`);
            // and a Dismiss or a conversation hide after a Keep of a message
            // in it, which the Keep's own confirmation answers
            // (`hides_after_a_keep_in_one_take_leave_the_keep_and_report_nothing`).
            if let Intent::Keep { item, from } = intent {
                self.problem(Problem::Command {
                    command: Command::Keep { item, from },
                    why: Failure::NoSuchRow,
                });
            }
            return None;
        }
        let command = match intent {
            Intent::OpenLink(destination) => {
                // The model raises it only for an allowlisted link a person
                // activated; checked again here, at the edge where the
                // link leaves the client.
                if is_allowed_link_scheme(&destination) {
                    self.opener.open(&destination);
                }
                return None;
            }
            Intent::AllowNetwork => {
                // The platform's to carry out, as a link is: the facade
                // has no part in it.
                self.opener.ask_network_access();
                return None;
            }
            Intent::Authenticate => {
                self.opener.authenticate();
                return None;
            }
            // Disabled from here until the platform says the start
            // answered: the model offers no second one meanwhile.
            Intent::RetryStart => {
                self.facts.retry_start_asked();
                self.tell_all();
                self.opener.retry_start();
                return None;
            }
            Intent::Send { key, draft } => Command::Send { key, draft },
            Intent::MarkRead(row) => Command::MarkRead(row),
            Intent::Keep { item, from } => {
                // A Keep on its way: Hide is not offered until it is
                // answered.
                self.model.keep_requested(&Intent::Keep { item, from });
                Command::Keep { item, from }
            }
            Intent::Unkeep(row) => Command::Unkeep(row),
            Intent::Retry(row) => Command::Retry(row),
            Intent::Cancel(row) => {
                // Each cancel's record is held for its Undo until its
                // snackbar goes, and the facade side evicts the oldest past
                // CANCELLED_CAP. The open snackbar's record is the newest
                // answer applied, so only cancels still in flight come
                // after it; keeping at most CANCELLED_CAP - 2 of them in
                // flight keeps it, however late the answers come. A cancel
                // past that is refused and reported, and its row springs back at
                // the view's commit bound (review F2 on #289, rounds 3 to 5).
                let in_flight = self
                    .in_flight
                    .iter()
                    .filter(|f| matches!(f, InFlight::Other(Command::Cancel(_))))
                    .count();
                if in_flight >= CANCELLED_CAP - 2 {
                    self.problem(Problem::Command {
                        command: Command::Cancel(row),
                        why: Failure::TooMany,
                    });
                    return None;
                }
                Command::Cancel(row)
            }
            Intent::Restore(row) => Command::Restore(row),
            // Hiding writes nothing: the model's alone.
            Intent::Hide(item) => {
                self.model.hide(item);
                return None;
            }
            Intent::HideConversation(key) => {
                self.model.hide_conversation(&key);
                return None;
            }
            Intent::Reopen => Command::Reopen,
            Intent::RecheckStorage => Command::RecheckStorage,
            Intent::ReadTrust => Command::ReadTrust,
            Intent::SetTrust(change) => Command::SetTrust(change),
            Intent::ReadAddresses => Command::ReadAddresses,
            Intent::ReadOwnAddresses => Command::ReadOwnAddresses,
            Intent::AddAddresses(lines) => Command::AddAddresses(lines),
            Intent::RemoveAddress(address) => Command::RemoveAddress(address),
            // Accept carries the context the person was shown; a proposal
            // already gone has nothing to accept.
            Intent::AcceptCard(row) => Command::AcceptCard {
                row,
                shown: Box::new(self.model.proposal_context(row)?.clone()),
            },
            Intent::DeclineCard(row) => Command::DeclineCard(row),
            Intent::UndoCard(row) => Command::UndoCard(row),
            // Each is the model's step before it is the facade's: a press
            // the model does not offer now asks nothing.
            Intent::RetryKey => {
                self.model.retry_key()?;
                Command::RetryKey
            }
            Intent::CloseKeyStep => {
                self.model.key_step_closed();
                return None;
            }
            Intent::ShareCard(key) => Command::ShareCard(key),
            Intent::ChooseOwnPhoto(chosen) => {
                let seq = self.model.own_photo_chosen();
                let basis = self.model.own_photo_basis();
                Command::PrepareOwnPhoto { chosen, seq, basis }
            }
            Intent::ForgetOwnPhoto => {
                self.model.own_photo_forgotten();
                Command::ForgetOwnPhoto
            }
            // The facade side lets go of a photo it holds or prepares, and
            // holds the removal for a Save to write.
            Intent::RemoveOwnPhoto => {
                self.model.own_photo_removed();
                Command::RemoveOwnPhoto
            }
            Intent::SetOwnCard {
                name,
                surname,
                photo,
            } => {
                // Saved, or saved without it: the editor's choice is done.
                self.model.own_photo_forgotten();
                Command::SetOwnCard {
                    name,
                    surname,
                    photo,
                }
            }
            Intent::RenameContact { contact_id, name } => {
                Command::RenameContact { contact_id, name }
            }
            Intent::SetContactPhoto { contact_id, photo } => {
                Command::SetContactPhoto { contact_id, photo }
            }
            Intent::DeleteContact { contact_id } => Command::DeleteContact(contact_id),
            Intent::UndoContact {
                contact_id,
                change,
                hold,
            } => Command::UndoContact {
                contact_id,
                change,
                hold,
            },
            Intent::DeleteRoute {
                contact_id,
                peer,
                endpoint,
            } => Command::DeleteRoute {
                contact_id,
                peer,
                endpoint,
            },
        };
        if !self.in_flight.insert(InFlight::of(&command)) {
            return None;
        }
        if let Command::Send { key, .. } = &command {
            // The answer may touch the composer only if it is not edited
            // after this press.
            self.model.send_pressed(key);
        }
        if let Command::Cancel(row) = &command {
            // Until it is answered, the snackbar up now stays; one opened
            // after it is never taken away by its answer (§6).
            self.model.cancel_requested(*row);
        }
        Some(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interweave_test_support::endpoint_keys::endpoint_id;

    use interweave_human_store::RowId;

    struct Nothing;

    impl Surface for Nothing {
        fn render(&mut self, _model: &UiModel) {}
        fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            Vec::new()
        }
    }

    impl Opener for Nothing {
        fn open(&mut self, _destination: &str) {}
        fn ask_network_access(&mut self) {}
        fn authenticate(&mut self) {}
    }

    /// Each model side made tells its surface, once, that a fresh model
    /// reads it (#517 review F1). Mutation: `new` not telling it.
    #[test]
    fn a_model_side_made_tells_its_surface_once() {
        struct Counting(std::rc::Rc<std::cell::Cell<u32>>);
        impl Surface for Counting {
            fn render(&mut self, _model: &UiModel) {}
            fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
                Vec::new()
            }
            fn model_made(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let made = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut side = ModelSide::new(Counting(std::rc::Rc::clone(&made)), Nothing);
        let _ = side.turn();
        assert_eq!(made.get(), 1);
        let _second = ModelSide::new(Counting(std::rc::Rc::clone(&made)), Nothing);
        assert_eq!(made.get(), 2, "one per model");
        side.remade();
        assert_eq!(made.get(), 3, "and once more when one is shown again");
    }

    /// A model kept after its service stopped holds nothing in flight no
    /// facade will answer, and reads no Ready session (review F3, kept
    /// for unlock note §11). Mutations: `stopped` failing nothing; leaving
    /// the session Ready.
    #[test]
    fn a_stopped_model_fails_what_is_in_flight_and_reads_no_ready_session() {
        use interweave_human_ui_model::SessionNotice;
        let mut side = ModelSide::new(Nothing, Nothing);
        side.apply(Update::Client(ClientEvent::Session(SessionState::Ready {
            endpoint: None,
        })));
        assert_eq!(side.model().session_notice(), None, "control: Ready");
        let row = RowId::from_stored(7);
        let read = side.command_for(Intent::MarkRead(row)).expect("issued");
        assert!(
            side.command_for(Intent::MarkRead(row)).is_none(),
            "control: held in flight"
        );

        side.stopped();
        assert_eq!(
            side.take_problems(),
            vec![Problem::Command {
                command: read,
                why: Failure::ServiceStopped
            }]
        );
        assert_eq!(
            side.model().session_notice(),
            Some(SessionNotice::NoDaemon),
            "the service reads as not running"
        );
        assert!(
            side.command_for(Intent::MarkRead(row)).is_some(),
            "nothing held in flight"
        );
    }

    /// The service is the profile's: a stop reaches the identity behind
    /// as it does the front -- its commands and its send in flight fail,
    /// its session reads not running -- and a switch brings that, not the
    /// state before the stop, to the front. Mutation: `stopped` stopping
    /// only the front.
    #[test]
    fn a_stop_reaches_the_identity_behind_as_it_does_the_front() {
        use interweave_human_ui_model::SessionNotice;
        let mut side = ModelSide::new(Nothing, Nothing);
        side.hold(&[first(), second()], &first());
        let ready = || Update::Client(ClientEvent::Session(SessionState::Ready { endpoint: None }));
        side.apply_as(&first(), ready());
        side.apply_as(&second(), ready());
        let row = RowId::from_stored(7);

        // The second's in flight, put there while it is in front.
        assert!(side.switch(&second(), |_| {}));
        let read = side.command_for(Intent::MarkRead(row)).expect("issued");
        let sent = Intent::Send {
            key: general(),
            draft: "hi".to_owned(),
        };
        side.command_for(sent.clone()).expect("send issued");
        assert!(side.command_for(sent.clone()).is_none(), "control: waits");
        // The first's, with it behind.
        assert!(side.switch(&first(), |_| {}));
        let first_row = RowId::from_stored(8);
        let first_read = side
            .command_for(Intent::MarkRead(first_row))
            .expect("issued");
        assert_eq!(side.model().session_notice(), None, "control: Ready");

        side.stopped();
        let problems = side.take_problems();
        let failed = |command| Problem::Command {
            command,
            why: Failure::ServiceStopped,
        };
        // The reads a switch asks of a model are in flight too, so the
        // three named are among the failures, not all of them.
        assert!(problems.contains(&failed(first_read)), "the front's");
        assert!(problems.contains(&failed(read)), "the one behind's command");
        assert!(
            problems.contains(&failed(Command::Send {
                key: general(),
                draft: String::new()
            })),
            "the one behind's send"
        );
        assert_eq!(
            side.model().session_notice(),
            Some(SessionNotice::NoDaemon),
            "control: the front"
        );
        assert!(
            side.command_for(Intent::MarkRead(first_row)).is_some(),
            "control: the front's freed"
        );

        assert!(side.switch(&second(), |_| {}));
        assert_eq!(
            side.model().session_notice(),
            Some(SessionNotice::NoDaemon),
            "the one behind reads not running"
        );
        assert!(side.command_for(Intent::MarkRead(row)).is_some(), "freed");
        assert!(side.command_for(sent).is_some(), "its send no longer waits");
    }

    /// The drafts of every held identity, the front's and the one behind
    /// it, carry into a new side made with the same held list, each into
    /// its own identity's model; a draft of an identity the new side does
    /// not hold is dropped. Mutations run: `held_drafts` listing only the
    /// front's model -- the second identity's draft is gone; the carry
    /// writing every draft into the front's model -- the front reads the
    /// last draft carried, the unheld identity's.
    #[test]
    fn drafts_carry_over_into_the_model_of_each_identity() {
        let mut old = ModelSide::new(Nothing, Nothing);
        old.hold(&[first(), second()], &first());
        old.model.draft_changed(general(), "of first".to_owned());
        let (_, behind) = old.behind.first_mut().expect("second behind");
        behind
            .model
            .draft_changed(general(), "of second".to_owned());
        let mut drafts = old.held_drafts();
        drafts.push((endpoint_id("gone"), general(), "of nobody".to_owned()));

        let mut new = ModelSide::new(Nothing, Nothing);
        new.hold(&[first(), second()], &first());
        new.carry_held_drafts(drafts);
        let draft = |e: &EndpointId| new.model_of(e).expect("held").composer(&general()).draft;
        assert_eq!(draft(&first()), "of first");
        assert_eq!(draft(&second()), "of second");
    }

    fn started(undecodable_unread: usize, undecodable_other: usize) -> Update {
        Update::Started(Box::new(crate::protocol::Start {
            pending: Vec::new(),
            summaries: Vec::new(),
            counted_through: None,
            undecodable_unread,
            undecodable_other,
            kept: 0,
            contacts: Ok(Vec::new()),
            identity: None,
        }))
    }

    fn relisted(undecodable: usize) -> Update {
        Update::SummariesListed {
            summaries: Vec::new(),
            counted_through: None,
            undecodable,
            kept: 0,
        }
    }

    /// A relist counts the unread rows it cannot show again; the start's
    /// count of the pending and kept ones stands. Mutations: the relist's
    /// count added; the others' count replaced too.
    #[test]
    fn a_relist_replaces_the_unread_count_and_keeps_the_others() {
        let mut side = ModelSide::new(Nothing, Nothing);
        side.apply(started(2, 1));
        assert_eq!(side.hidden_rows(), 3);
        side.apply(relisted(0));
        assert_eq!(side.hidden_rows(), 1, "the pending and kept count stands");
        side.apply(relisted(4));
        assert_eq!(side.hidden_rows(), 5, "replaced, not added");
    }

    /// The kept count is the start's, then each relist's, replaced.
    /// Mutation: the relist's count not applied.
    #[test]
    fn the_kept_count_is_the_starts_then_each_relists() {
        let mut side = ModelSide::new(Nothing, Nothing);
        side.apply(Update::Started(Box::new(crate::protocol::Start {
            pending: Vec::new(),
            summaries: Vec::new(),
            counted_through: None,
            undecodable_unread: 0,
            undecodable_other: 0,
            kept: 3,
            contacts: Ok(Vec::new()),
            identity: None,
        })));
        assert_eq!(side.kept_rows(), 3);
        side.apply(Update::SummariesListed {
            summaries: Vec::new(),
            counted_through: None,
            undecodable: 0,
            kept: 2,
        });
        assert_eq!(side.kept_rows(), 2, "replaced, not added");
    }

    #[test]
    fn a_relist_that_could_not_be_read_is_a_problem() {
        let mut side = ModelSide::new(Nothing, Nothing);
        side.apply(Update::UnreadNotListed(Failure::StorageUnavailable));
        assert_eq!(
            side.take_problems(),
            [Problem::UnreadNotListed(Failure::StorageUnavailable)]
        );
    }

    /// #414 F2, F9, F10: an Undo whose card could not be judged again
    /// drops its confirmation row through `apply`, and a failed read after
    /// a card's answer is its own problem, not an unread listing.
    #[test]
    fn an_undo_not_restored_drops_its_row_and_a_failed_read_back_is_its_own_problem() {
        use interweave_human_chat_protocol::Card;
        use interweave_human_store::RowId;
        use interweave_human_ui_model::{
            CardOutcome, ContactView, KeyView, Proposal, ProposalContext,
        };
        use interweave_transport_api::TransportIdentity;
        let peer = TransportIdentity::parse("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN")
            .expect("a PeerId");
        let endpoint = endpoint_id("human");
        let row = RowId::from_stored(3);
        let key = ConversationKey::Direct {
            peer: peer.clone(),
            endpoint: Some(endpoint.clone()),
        };
        let mut side = ModelSide::new(Nothing, Nothing);
        side.apply(Update::ReceivedCard(Box::new(Proposal {
            row,
            peer: peer.clone(),
            endpoint,
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
                values: interweave_human_ui_model::ValueChanges::default(),
            },
        })));
        side.apply(Update::CardResolved {
            row,
            outcome: CardOutcome::Accepted {
                contact_id: "c1".to_owned(),
                created: false,
                photo_dropped: false,
                photo_removed: false,
            },
        });
        side.apply(Update::ContactDirectory(Ok(vec![
            interweave_human_ui_model::DirectoryContact {
                view: ContactView {
                    contact_id: "c1".to_owned(),
                    name: "Ada".to_owned(),
                    name_from_card: true,
                    name_at: None,
                    photo_from_card: None,
                    photo_at: None,
                    photo: None,
                    routes: Vec::new(),
                },
                has_photo: false,
            },
        ])));
        assert_eq!(side.model().card_answers(&key).len(), 1);
        side.apply(Update::CardsNotRead(Failure::Corrupt));
        side.apply(Update::ProposalNotRestored(row));
        assert!(side.model().card_answers(&key).is_empty(), "the row goes");
        assert_eq!(
            side.take_problems(),
            [Problem::CardsNotRead(Failure::Corrupt)]
        );
    }

    #[test]
    fn the_daemon_seen_reaches_the_model() {
        use interweave_human_client_api::{ClientEvent, SessionState};
        use interweave_human_ui_model::SessionNotice;
        let mut side = ModelSide::new(Nothing, Nothing);
        side.apply(Update::Client(ClientEvent::Session(
            SessionState::Reconnecting {
                attempt: 1,
                next_at: 0,
            },
        )));
        side.daemon_seen(Some(false));
        assert_eq!(side.model().session_notice(), Some(SessionNotice::NoDaemon));
        side.daemon_seen(Some(true));
        assert_eq!(
            side.model().session_notice(),
            Some(SessionNotice::Reconnecting)
        );
    }

    /// The daemon's presence is told to every identity held, not only the
    /// one in front (#769 r1 F1): reported absent while the first is in
    /// front, the second reads no daemon when it comes forward; returned
    /// while the second is behind, the first reads no stale no-daemon
    /// when it comes back. Mutation: `tell_all` telling the front alone.
    #[test]
    fn the_daemon_seen_reaches_every_identity_held() {
        use interweave_human_client_api::{ClientEvent, SessionState};
        use interweave_human_ui_model::SessionNotice;
        let reconnecting = || {
            Update::Client(ClientEvent::Session(SessionState::Reconnecting {
                attempt: 1,
                next_at: 0,
            }))
        };
        let mut side = ModelSide::new(Nothing, Nothing);
        side.hold(&[first(), second()], &first());
        side.apply(reconnecting());
        assert!(side.apply_as(&second(), reconnecting()));
        side.daemon_seen(Some(false));
        assert!(side.switch(&second(), |_| {}));
        assert_eq!(side.model().session_notice(), Some(SessionNotice::NoDaemon));

        side.daemon_seen(Some(true));
        assert!(side.switch(&first(), |_| {}));
        assert_eq!(
            side.model().session_notice(),
            Some(SessionNotice::Reconnecting),
            "no stale no-daemon"
        );
        assert_eq!(
            side.model_of(&second()).unwrap().session_notice(),
            Some(SessionNotice::Reconnecting)
        );
    }

    /// A model made in `hold` after the daemon was reported reads it.
    /// Mutation: `hold` not telling the model it makes.
    #[test]
    fn a_model_made_in_hold_reads_the_daemon_already_seen() {
        use interweave_human_client_api::{ClientEvent, SessionState};
        use interweave_human_ui_model::SessionNotice;
        let mut side = ModelSide::new(Nothing, Nothing);
        side.daemon_seen(Some(false));
        side.hold(&[first(), second()], &first());
        side.apply_as(
            &second(),
            Update::Client(ClientEvent::Session(SessionState::Reconnecting {
                attempt: 1,
                next_at: 0,
            })),
        );
        assert_eq!(
            side.model_of(&second()).unwrap().session_notice(),
            Some(SessionNotice::NoDaemon)
        );
    }

    #[test]
    fn problems_never_pass_the_cap_and_the_oldest_are_counted() {
        let mut side = ModelSide::new(Nothing, Nothing);
        let failed = |n: usize| Update::Failed {
            command: Command::MarkRead(RowId::from_stored(i64::try_from(n).expect("small"))),
            why: Failure::StorageUnavailable,
        };
        for n in 0..PROBLEM_CAP + 3 {
            side.apply(failed(n));
        }
        assert_eq!(side.problems_dropped(), 3);
        let problems = side.take_problems();
        assert_eq!(problems.len(), PROBLEM_CAP);
        assert_eq!(
            problems[0],
            Problem::Command {
                command: Command::MarkRead(RowId::from_stored(3)),
                why: Failure::StorageUnavailable
            },
            "the oldest three went"
        );
        assert!(side.take_problems().is_empty(), "taken means gone");
    }

    fn first() -> EndpointId {
        endpoint_id("human")
    }

    fn second() -> EndpointId {
        endpoint_id("second")
    }

    fn general() -> ConversationKey {
        ConversationKey::Channel(
            interweave_transport_api::ChannelId::parse("general").expect("a channel"),
        )
    }

    /// A surface that hands over `events` at the next take.
    struct Events(Vec<ViewEvent>);

    impl Surface for Events {
        fn render(&mut self, _model: &UiModel) {}
        fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            std::mem::take(&mut self.0)
        }
    }

    /// The start's facts are the Service's, told to every identity held
    /// (rust-ui-dev-02's REPLY 01a2190d): a busy start told while the
    /// first is in front shows when the second comes forward; Try again
    /// in front leaves every model Starting, and a fact told after it
    /// (any `tell_all`) does not put Shown back; the P-up line seen on the
    /// first's Trust page is seen on the second's too, past the next
    /// tell. Mutations: `start_busy` telling the front alone -- the
    /// second reads no notice; `RetryStart` changing the front's model
    /// alone -- the next fact puts Shown back; the seen line not written
    /// to the facts -- the second offers the P-up notice again.
    #[test]
    fn a_retry_and_a_seen_line_outlive_the_next_fact_and_a_switch() {
        use interweave_human_ui_model::{KeyUnlock, SessionNotice};
        let mut side = ModelSide::new(Events(Vec::new()), Nothing);
        side.hold(&[first(), second()], &first());
        side.start_busy(Some(StartBusy::Shown));
        assert_eq!(
            side.model_of(&second()).unwrap().session_notice(),
            Some(SessionNotice::StartBusy(StartBusy::Shown)),
            "told to the identity behind"
        );
        side.unlock_facts(
            Some(UnlockFacts {
                in_force: KeyUnlock::BackgroundCompatible,
                configured: Some(KeyUnlock::UserPresence),
                pending: None,
                restart_needs_unlock: Some(false),
            }),
            7,
            false,
        );
        assert!(side.model_of(&second()).unwrap().unlock_notice(), "control");

        side.surface.0 = vec![
            ViewEvent::Intent(Intent::RetryStart),
            ViewEvent::Trust(TrustInput::Opened),
        ];
        let _ = side.turn();
        side.network_access(true);
        assert_eq!(
            side.model().session_notice(),
            Some(SessionNotice::StartBusy(StartBusy::Starting)),
            "the next tell keeps the retry"
        );
        assert!(!side.model().unlock_notice(), "the next tell keeps it seen");

        assert!(side.switch(&second(), |_| {}));
        assert_eq!(
            side.model().session_notice(),
            Some(SessionNotice::StartBusy(StartBusy::Starting))
        );
        assert!(!side.model().unlock_notice(), "seen for this start");
    }

    /// Named, the model shown so far is the front's, drafts and all, and
    /// every other identity held has a model of its own, empty; a second
    /// naming changes nothing. Mutations: `hold` giving the front a fresh
    /// model -- the draft is gone; `hold` naming again -- the front moves.
    #[test]
    fn naming_the_held_identities_keeps_the_model_shown_as_the_fronts() {
        let mut side = ModelSide::new(Nothing, Nothing);
        assert_eq!(side.front(), None);
        side.model.draft_changed(general(), "written".to_owned());
        assert!(side.hold(&[first(), second()], &first()));
        assert_eq!(side.front(), Some(&first()));
        let shown = side.model_of(&first()).expect("the front's");
        assert_eq!(shown.composer(&general()).draft, "written");
        let behind = side.model_of(&second()).expect("its own");
        assert!(behind.drafts().is_empty(), "a model of its own");
        assert!(side.model_of(&endpoint_id("gone")).is_none());

        assert!(!side.hold(&[first(), second()], &second()));
        assert_eq!(side.front(), Some(&first()), "named once");
    }

    /// A surface counting its renders.
    #[derive(Default)]
    struct Counting {
        renders: usize,
    }

    impl Surface for Counting {
        fn render(&mut self, _model: &UiModel) {
            self.renders += 1;
        }
        fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            Vec::new()
        }
    }

    /// An update for an identity behind the front applies to that
    /// identity's model, and draws nothing: the front's model is as it
    /// was, the surface is not drawn, and the identity's model holds it
    /// when it comes in front; a session of it that became ready owes its
    /// lists' reads to that identity, asked once it is in front. An
    /// update for an identity not held, or to a model side never named,
    /// applies to nothing. Mutations: `apply_as` applying to the front's
    /// model -- the front counts the rows; `exchange` leaving
    /// `reads_owed` out -- the reads are asked as the front.
    #[test]
    fn an_update_for_an_identity_behind_applies_to_its_model_and_draws_nothing() {
        let mut unnamed = ModelSide::new(Counting::default(), Nothing);
        assert!(!unnamed.apply_as(&first(), relisted(3)), "never named");
        assert_eq!(unnamed.model().rows_not_shown(), 0);

        let mut side = ModelSide::new(Counting::default(), Nothing);
        side.hold(&[first(), second()], &first());
        assert!(side.apply_as(&second(), relisted(3)));
        assert!(side.apply_as(
            &second(),
            Update::Client(ClientEvent::Session(SessionState::Ready { endpoint: None }))
        ));
        assert!(!side.apply_as(&endpoint_id("gone"), relisted(5)));
        assert_eq!(side.surface().renders, 0, "nothing drawn");
        assert_eq!(side.model().rows_not_shown(), 0, "the front's is as it was");
        assert_eq!(side.hidden_rows(), 0);
        assert_eq!(
            side.model_of(&second()).expect("held").rows_not_shown(),
            3,
            "its own model holds it"
        );
        assert!(side.turn().is_empty(), "the front owes no reads");

        let mut sent = Vec::new();
        assert!(side.switch(&second(), |c| sent.push(c)));
        assert_eq!(side.hidden_rows(), 3, "shown at once in front");
        assert_eq!(side.model().rows_not_shown(), 3);
        let reads = side.turn();
        assert!(
            reads.contains(&Command::ReadTrust) && reads.contains(&Command::ReadAddresses),
            "its reads, asked in front: {reads:?}"
        );
    }

    /// What a recording surface and the crossing saw, in one order.
    type Log = std::rc::Rc<std::cell::RefCell<Vec<String>>>;

    struct Recording {
        log: Log,
        events: Vec<ViewEvent>,
    }

    impl Surface for Recording {
        fn render(&mut self, _model: &UiModel) {}
        fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            std::mem::take(&mut self.events)
        }
        fn front_changed(&mut self, old: &EndpointId, new: &EndpointId, left: &UiModel) {
            self.log.borrow_mut().push(format!(
                "told {} {} snackbar={} drafts={}",
                name(old),
                name(new),
                left.identities().switch_snackbar().is_some(),
                left.drafts().len()
            ));
        }
    }

    fn name(e: &EndpointId) -> &'static str {
        [("human", first()), ("second", second())]
            .into_iter()
            .find(|(_, id)| id == e)
            .map_or("?", |(n, _)| n)
    }

    /// A surface whose person pressed `intents`, handed over at the
    /// first take.
    struct Pressed(Vec<IdentityIntent>);

    impl Surface for Pressed {
        fn render(&mut self, _model: &UiModel) {}
        fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            Vec::new()
        }
        fn take_identity_intents(&mut self) -> Vec<IdentityIntent> {
            std::mem::take(&mut self.0)
        }
    }

    fn key_of(endpoint: &EndpointId) -> IdentityKey {
        IdentityKey::new(endpoint.as_str())
    }

    /// Try again names its identity, and that identity's re-open is sent
    /// as its own whichever is in front: one behind the front is
    /// reopened as itself; one not held sends nothing; the other acts
    /// send nothing. Mutations: the crossing naming the front -- the
    /// identity behind is reopened as the first; `take_identity_acts`
    /// not draining the surface -- nothing is sent.
    #[test]
    fn try_again_reopens_the_identity_it_names_and_no_other() {
        let mut side = ModelSide::new(
            Pressed(vec![
                IdentityIntent::OpenSwitcher,
                IdentityIntent::TryAgain(key_of(&second())),
                IdentityIntent::TryAgain(key_of(&endpoint_id("gone"))),
            ]),
            Nothing,
        );
        side.hold(&[first(), second()], &first());
        let mut sent = Vec::new();
        side.take_identity_acts(|c| sent.push(c));
        assert_eq!(
            sent,
            [Crossing::Command {
                identity: second(),
                command: Command::Reopen,
            }]
        );
        side.take_identity_acts(|c| sent.push(c));
        assert_eq!(sent.len(), 1, "taken once");
    }

    /// The held identities reach every model, the front's and each
    /// behind, and each model is in front of its own identity; a refused
    /// list changes no model. Mutations: `held_changed` telling the
    /// front's model alone -- the model behind holds none; not starting
    /// each model on its own -- the model behind has the first in front.
    #[test]
    fn the_held_identities_reach_every_model_each_in_front_of_its_own() {
        let mut side = ModelSide::new(Nothing, Nothing);
        side.hold(&[first(), second()], &first());
        let list = vec![held(first().as_str(), true), held(second().as_str(), false)];
        side.held_changed(&list).expect("taken");
        for identity in [first(), second()] {
            let identities = side.model_of(&identity).expect("held").identities();
            assert_eq!(identities.held(), list.as_slice(), "{identity:?}");
            assert_eq!(identities.front(), Some(&key_of(&identity)));
        }
        let twice = vec![list[0].clone(), list[0].clone()];
        assert!(side.held_changed(&twice).is_err());
        assert_eq!(
            side.model_of(&second()).expect("held").identities().held(),
            list.as_slice(),
            "kept"
        );
    }

    fn held(key: &str, is_first: bool) -> interweave_human_ui_model::HeldIdentity {
        interweave_human_ui_model::HeldIdentity {
            key: interweave_human_ui_model::IdentityKey::new(key),
            card_name: None,
            alias: key.to_owned(),
            state: interweave_human_ui_model::IdentityState::Ready,
            is_first,
            is_default: is_first,
            unread: 0,
            not_sent: false,
        }
    }

    /// A switch, on the model side: the old front's pending acts are
    /// drained into commands and sent as the old front, its switch
    /// snackbar closes, the surface is told with the old front's model
    /// still shown, and only then do the models change places and the
    /// switch go, naming the new front -- the last thing sent, so a facade
    /// carrying out what it is sent in order runs the old front's
    /// commands before its swap. A switch to the identity in front, to
    /// one not held, or on a side never named, does nothing at all.
    /// Mutations: the switch sent before the drained commands; the
    /// commands sent as the new front; the drain skipped; the snackbar
    /// closed after the surface is told (`snackbar=true`); the surface
    /// told after the models change places (`drafts=0`); the snackbar
    /// closed after the exchange (the new front's closes, the old's
    /// stays).
    #[test]
    fn a_switch_drains_sends_closes_tells_then_names_the_new_front() {
        let log: Log = std::rc::Rc::default();
        let mut never = ModelSide::new(
            Recording {
                log: log.clone(),
                events: vec![ViewEvent::Intent(Intent::Reopen)],
            },
            Nothing,
        );
        assert!(!never.switch(&second(), |_| log.borrow_mut().push("sent".into())));
        let mut side = ModelSide::new(
            Recording {
                log: log.clone(),
                events: vec![ViewEvent::Intent(Intent::Reopen)],
            },
            Nothing,
        );
        side.hold(&[first(), second()], &first());
        let send = |c: Crossing| {
            let line = match c {
                Crossing::Command { identity, command } => {
                    format!("command {} {command:?}", name(&identity))
                }
                Crossing::Front(to) => format!("front {}", name(&to)),
            };
            log.borrow_mut().push(line);
        };
        assert!(!side.switch(&first(), send));
        assert!(!side.switch(&endpoint_id("gone"), send));
        assert!(log.borrow().is_empty(), "nothing done: {:?}", log.borrow());

        // The old front shows a switch's snackbar and holds a draft.
        side.model.draft_changed(general(), "written".to_owned());
        let identities = side.model.identities_mut();
        identities
            .held_changed(vec![held("a", true), held("b", false)])
            .expect("two held");
        identities.start(None);
        identities.switch(&interweave_human_ui_model::IdentityKey::new("b"), false);
        assert!(side.model().identities().switch_snackbar().is_some());

        assert!(side.switch(&second(), send));
        assert_eq!(
            *log.borrow(),
            [
                "command human Reopen",
                "told human second snackbar=false drafts=1",
                "front second",
            ]
        );
        assert_eq!(side.front(), Some(&second()));
        let left = side.model_of(&first()).expect("still held");
        assert!(left.identities().switch_snackbar().is_none(), "closed");
        assert_eq!(left.drafts().len(), 1, "kept behind");
        assert!(side.model().drafts().is_empty(), "the new front's shown");
    }

    /// What one identity has in flight is its own: a command its model
    /// sent and that is not answered holds back the same command of that
    /// identity, never another's, across switches. Mutation: `exchange`
    /// leaving `in_flight` out -- the second identity's Reopen is held
    /// back as the first's.
    #[test]
    fn a_command_in_flight_is_its_identitys_own() {
        let log: Log = std::rc::Rc::default();
        let mut side = ModelSide::new(
            Recording {
                log,
                events: vec![ViewEvent::Intent(Intent::Reopen)],
            },
            Nothing,
        );
        side.hold(&[first(), second()], &first());
        assert_eq!(side.turn(), [Command::Reopen], "the first's, in flight");
        assert!(side.switch(&second(), |_| {}));
        side.surface_mut().events = vec![ViewEvent::Intent(Intent::Reopen)];
        assert_eq!(side.turn(), [Command::Reopen], "the second's own");
        assert!(side.switch(&first(), |_| {}));
        side.surface_mut().events = vec![ViewEvent::Intent(Intent::Reopen)];
        assert!(side.turn().is_empty(), "the first's still in flight");
        side.apply_as(&first(), Update::Done(Command::Reopen));
        side.surface_mut().events = vec![ViewEvent::Intent(Intent::Reopen)];
        assert_eq!(side.turn(), [Command::Reopen], "answered: free again");
    }

    /// A person's surface: the identity acts and the view events it hands
    /// over at the next take, and a log of what it drew and was told,
    /// in the order the crossings were sent.
    struct Person {
        log: Log,
        events: Vec<ViewEvent>,
        intents: Vec<IdentityIntent>,
        add_intents: Vec<AddIntent>,
    }

    impl Surface for Person {
        fn render(&mut self, model: &UiModel) {
            let ids = model.identities();
            self.log.borrow_mut().push(format!(
                "drawn {} switcher={}",
                ids.front().map_or("none", key_name),
                ids.switcher_open()
            ));
        }
        fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            std::mem::take(&mut self.events)
        }
        fn front_changed(&mut self, old: &EndpointId, new: &EndpointId, _left: &UiModel) {
            self.log
                .borrow_mut()
                .push(format!("told {} {}", name(old), name(new)));
        }
        fn take_identity_intents(&mut self) -> Vec<IdentityIntent> {
            std::mem::take(&mut self.intents)
        }
        fn take_add_intents(&mut self) -> Vec<AddIntent> {
            std::mem::take(&mut self.add_intents)
        }
    }

    fn key_name(k: &IdentityKey) -> &'static str {
        [("human", first()), ("second", second()), ("third", third())]
            .into_iter()
            .find(|(_, id)| id.as_str() == k.as_str())
            .map_or("?", |(n, _)| n)
    }

    fn third() -> EndpointId {
        endpoint_id("third")
    }

    /// A side holding the first and the second, the first in front, each
    /// model told both, and the person's acts logged with every crossing.
    fn person_side(log: &Log) -> ModelSide<Person, Nothing> {
        let mut side = ModelSide::new(
            Person {
                log: log.clone(),
                events: Vec::new(),
                intents: Vec::new(),
                add_intents: Vec::new(),
            },
            Nothing,
        );
        side.hold(&[first(), second()], &first());
        side.held_changed(&[held(first().as_str(), true), held(second().as_str(), false)])
            .expect("two held");
        side
    }

    /// Act `intents` as the person, every crossing into the log.
    fn act(side: &mut ModelSide<Person, Nothing>, intents: Vec<IdentityIntent>) {
        side.surface_mut().intents = intents;
        let log = side.surface().log.clone();
        side.take_identity_acts(|c| {
            let line = match c {
                Crossing::Command { identity, command } => {
                    format!("command {} {command:?}", name(&identity))
                }
                Crossing::Front(to) => format!("front {}", name(&to)),
            };
            log.borrow_mut().push(line);
        });
    }

    /// A row chosen in the open switcher brings its identity in front in
    /// the switch's order: the old front's pending acts drained and sent
    /// as its own, with the old front's model drawn naming the old
    /// identity, the surface told, then the switch sent. The new front's
    /// model shows the switcher closed, the switch's snackbar and the
    /// switch asked for focus (F3), with Switch back to the old front;
    /// the model left behind is in front of its own with its switcher
    /// closed. Mutations: the act decided on the front's model itself --
    /// the drain draws `second` over the first's rows; the decided state
    /// not given to the new front -- no snackbar; the model left not
    /// started on its own -- its switcher stays open; `take_identity_acts`
    /// routing nothing but Try again -- nothing is sent.
    #[test]
    fn a_choice_in_the_switcher_brings_the_identity_in_front_in_the_switchs_order() {
        let log: Log = std::rc::Rc::default();
        let mut side = person_side(&log);
        act(&mut side, vec![IdentityIntent::OpenSwitcher]);
        assert!(side.model().identities().switcher_open(), "opened");
        assert!(log.borrow().is_empty(), "opening sends nothing");
        let opened = side.model().identities().focus_request();
        assert_eq!(
            opened.map(|(_, f)| f),
            Some(interweave_human_ui_model::IdentityFocus::SwitcherTitle)
        );

        side.surface_mut().events = vec![ViewEvent::Intent(Intent::Reopen)];
        act(&mut side, vec![IdentityIntent::Choose(key_of(&second()))]);
        assert_eq!(
            *log.borrow(),
            [
                "drawn human switcher=true",
                "command human Reopen",
                "told human second",
                "front second",
            ]
        );
        assert_eq!(side.front(), Some(&second()));
        let ids = side.model().identities();
        assert_eq!(ids.front(), Some(&key_of(&second())));
        assert_eq!(ids.previous(), Some(&key_of(&first())));
        assert!(!ids.switcher_open(), "closed by the choice");
        assert!(ids.switch_snackbar().is_some(), "the switch's snackbar");
        let (seq, focus) = ids.focus_request().expect("asked");
        assert_eq!(focus, interweave_human_ui_model::IdentityFocus::Switch);
        assert!(Some(seq) > opened.map(|(s, _)| s), "a new request");
        let left = side.model_of(&first()).expect("kept").identities();
        assert_eq!(left.front(), Some(&key_of(&first())), "its own");
        assert!(!left.switcher_open(), "closed behind");
        assert!(left.switch_snackbar().is_none());
    }

    /// Switch back on the switch's snackbar brings the previous identity
    /// in front again, through the same switch, its snackbar a new one;
    /// a Switch back naming a snackbar no longer up does nothing.
    /// Mutation: Switch back left unrouted -- the second stays in front.
    #[test]
    fn switch_back_on_the_snackbar_brings_the_previous_identity_in_front() {
        let log: Log = std::rc::Rc::default();
        let mut side = person_side(&log);
        act(&mut side, vec![IdentityIntent::OpenSwitcher]);
        act(&mut side, vec![IdentityIntent::Choose(key_of(&second()))]);
        let shown = side.model().identities().switch_snackbar().expect("up").id;
        log.borrow_mut().clear();

        act(&mut side, vec![IdentityIntent::SwitchBack(shown + 7)]);
        assert!(log.borrow().is_empty(), "not the one up");
        assert_eq!(side.front(), Some(&second()));

        act(&mut side, vec![IdentityIntent::SwitchBack(shown)]);
        assert_eq!(
            *log.borrow(),
            [
                "drawn second switcher=false",
                "told second human",
                "front human"
            ]
        );
        assert_eq!(side.front(), Some(&first()));
        let ids = side.model().identities();
        assert_eq!(ids.front(), Some(&key_of(&first())));
        assert_eq!(ids.previous(), Some(&key_of(&second())));
        let back = ids.switch_snackbar().expect("its own snackbar").id;
        assert!(back > shown, "a new snackbar, announced");
    }

    /// The acts that switch nothing change what the front's model shows
    /// and send nothing: the switcher closing, Your identities, the row
    /// in front chosen, a row of an identity held by the list with no
    /// model on this side (which closes the switcher as the row in front
    /// does), and a snackbar closing. Mutations: the non-switching acts
    /// not given to the front's model -- the switcher stays open; the
    /// no-model fallback left out -- the switcher stays open.
    #[test]
    fn acts_that_switch_nothing_change_the_front_and_send_nothing() {
        let log: Log = std::rc::Rc::default();
        let mut side = person_side(&log);
        let open = |side: &mut ModelSide<Person, Nothing>| {
            act(side, vec![IdentityIntent::OpenSwitcher]);
            assert!(side.model().identities().switcher_open());
        };
        open(&mut side);
        act(&mut side, vec![IdentityIntent::CloseSwitcher]);
        assert!(!side.model().identities().switcher_open(), "closed");

        open(&mut side);
        act(&mut side, vec![IdentityIntent::OpenIdentities]);
        assert!(
            !side.model().identities().switcher_open(),
            "Your identities"
        );

        open(&mut side);
        act(&mut side, vec![IdentityIntent::Choose(key_of(&first()))]);
        assert!(
            !side.model().identities().switcher_open(),
            "the row in front"
        );

        // A third identity the list holds and this side has no model of.
        side.held_changed(&[
            held(first().as_str(), true),
            held(second().as_str(), false),
            held(third().as_str(), false),
        ])
        .expect("three held");
        open(&mut side);
        act(&mut side, vec![IdentityIntent::Choose(key_of(&third()))]);
        let ids = side.model().identities();
        assert!(!ids.switcher_open(), "closed");
        assert_eq!(ids.front(), Some(&key_of(&first())), "nothing switched");
        assert_eq!(
            ids.focus_request().map(|(_, f)| f),
            Some(interweave_human_ui_model::IdentityFocus::Switch)
        );
        assert!(log.borrow().is_empty(), "nothing sent: {:?}", log.borrow());
        assert_eq!(side.front(), Some(&first()));

        // A switch, then its snackbar closed by its time.
        open(&mut side);
        act(&mut side, vec![IdentityIntent::Choose(key_of(&second()))]);
        let id = side.model().identities().switch_snackbar().expect("up").id;
        log.borrow_mut().clear();
        act(&mut side, vec![IdentityIntent::SnackbarClosed(id)]);
        assert!(side.model().identities().switch_snackbar().is_none());
        assert!(log.borrow().is_empty(), "nothing sent");
    }

    /// The person's acts on the add form, then the add acts they asked.
    fn form_acts(side: &mut ModelSide<Person, Nothing>, intents: Vec<AddIntent>) -> Vec<AddAct> {
        side.surface_mut().add_intents = intents;
        side.take_add_acts(0)
    }

    /// A side of two identities that can add, with Your identities open
    /// and its Add chosen: the form open, its seed unanswered.
    fn adding_side(log: &Log) -> (ModelSide<Person, Nothing>, Vec<AddAct>) {
        let mut side = person_side(log);
        side.can_add(true);
        act(
            &mut side,
            vec![IdentityIntent::OpenIdentities, IdentityIntent::Add],
        );
        let acts = side.take_add_acts(0);
        (side, acts)
    }

    fn created(endpoint: &EndpointId, card: CardSaved) -> AddTold {
        AddTold::Ended {
            outcome: AddOutcome::Created {
                endpoint: endpoint.clone(),
                alias: Some("anna".to_owned()),
            },
            card: Some(card),
        }
    }

    fn anna() -> AddIntent {
        AddIntent::Edited {
            name: "Anna".to_owned(),
            surname: String::new(),
        }
    }

    /// Your identities' opening is a settle moment (identities §6.2, the
    /// settle): one `ListOpened` per opening, none for an act on a list
    /// already open, another after it closed and opened again.
    /// Mutation: every act on the open list sent as an opening.
    #[test]
    fn opening_the_list_sends_one_settle_moment_per_opening() {
        let log: Log = std::rc::Rc::default();
        let mut side = person_side(&log);
        side.can_add(true);
        act(&mut side, vec![IdentityIntent::OpenIdentities]);
        assert_eq!(side.take_add_acts(0), [AddAct::ListOpened]);
        act(&mut side, vec![IdentityIntent::OpenIdentities]);
        act(&mut side, vec![IdentityIntent::OpenRow(key_of(&second()))]);
        assert!(side.take_add_acts(0).is_empty(), "already open");
        act(
            &mut side,
            vec![IdentityIntent::ClosePage, IdentityIntent::CloseIdentities],
        );
        act(&mut side, vec![IdentityIntent::OpenIdentities]);
        assert_eq!(side.take_add_acts(0), [AddAct::ListOpened], "opened again");
        assert!(log.borrow().is_empty(), "no crossing: {:?}", log.borrow());
    }

    /// The form's acts reach the facade side in the person's order (the
    /// roster's `add`): Add opens the form, `Opened`; Try again under
    /// `IdentityAddWaitsSeed` is `Opened` again; Create, once offered,
    /// carries the form's card; a leaving with changes asks first and goes
    /// as `Left` at Close without saving. Mutations: Try again sent as
    /// nothing; Create sent with no card's name.
    #[test]
    fn the_forms_acts_reach_the_facade_in_the_persons_order() {
        let log: Log = std::rc::Rc::default();
        let (mut side, mut acts) = adding_side(&log);
        side.add_told(AddTold::WaitsSeed, 0);
        side.add_told(AddTold::NoSeed, 0);
        acts.extend(form_acts(&mut side, vec![AddIntent::TryAgain]));
        side.add_told(AddTold::Offered, 0);
        acts.extend(form_acts(&mut side, vec![anna(), AddIntent::Create]));
        side.add_told(
            AddTold::Ended {
                outcome: AddOutcome::Failed,
                card: None,
            },
            0,
        );
        acts.extend(form_acts(&mut side, vec![AddIntent::Leave]));
        assert!(
            side.model()
                .identities()
                .add_form()
                .is_some_and(|f| f.view(0).discard.is_some()),
            "asked first"
        );
        acts.extend(form_acts(&mut side, vec![AddIntent::Discard(true)]));
        assert_eq!(
            acts,
            [
                AddAct::ListOpened,
                AddAct::Opened,
                AddAct::Opened,
                AddAct::Create {
                    name: "Anna".to_owned(),
                    surname: None,
                    photo: None,
                },
                AddAct::Left,
            ]
        );
        assert!(side.model().identities().add_form().is_none(), "left");
    }

    /// F6: Done closes the form and opens Your identities on the new row
    /// -- once the held list names it, the facade side sending the
    /// identity held, then the list (`HeldMore`, then `Held`) -- with
    /// `ConfirmIdentityAdded`; a Done whose identity could not be held yet
    /// opens the list on its title at once (Q-U5). Mutation: Done applied
    /// before the held list names the row -- focus stays on the title.
    #[test]
    fn done_opens_the_list_and_focuses_the_new_row_once_the_held_list_names_it() {
        use interweave_human_ui_model::IdentityFocus;
        let log: Log = std::rc::Rc::default();
        let (mut side, _) = adding_side(&log);
        side.add_told(AddTold::Offered, 0);
        form_acts(&mut side, vec![anna(), AddIntent::Create]);
        side.add_told(created(&third(), CardSaved::Saved), 0);
        side.add_told(AddTold::Held(third()), 0);
        assert!(side.model().identities().add_form().is_none(), "closed");
        assert_eq!(side.model().identities().added_snackbar(), None, "not yet");

        assert!(side.hold_more(&[third()]));
        side.held_changed(&[
            held(first().as_str(), true),
            held(second().as_str(), false),
            held(third().as_str(), false),
        ])
        .expect("three held");
        let ids = side.model().identities();
        assert!(ids.identities_open());
        assert_eq!(
            ids.focus_request().map(|(_, f)| f),
            Some(IdentityFocus::Row)
        );
        assert_eq!(ids.focus_row(), Some(&key_of(&third())));
        let (_, said) = ids.added_snackbar().expect("announced");
        assert_eq!(
            said.text,
            interweave_human_ui_model::UiText::ConfirmIdentityAdded
        );

        // Done with its identity not held yet: the list's title.
        act(&mut side, vec![IdentityIntent::Add]);
        side.take_add_acts(0);
        form_acts(&mut side, vec![anna(), AddIntent::Create]);
        let gone = endpoint_id("not-held");
        side.add_told(created(&gone, CardSaved::NotSaved), 0);
        side.add_told(AddTold::NotHeld(gone), 0);
        assert_eq!(
            side.model().identities().focus_request().map(|(_, f)| f),
            Some(IdentityFocus::IdentitiesTitle)
        );
    }

    /// A told that comes after its form was left changes nothing: no
    /// form comes back, no list opens, no row is focused when a held list
    /// later names the identity. Mutation: Done applied with no form open.
    #[test]
    fn a_told_for_a_form_already_left_changes_nothing() {
        let log: Log = std::rc::Rc::default();
        let (mut side, _) = adding_side(&log);
        act(&mut side, vec![IdentityIntent::CloseIdentities]);
        assert_eq!(form_acts(&mut side, vec![AddIntent::Leave]), [AddAct::Left]);
        let before = side.model().identities().focus_request();
        side.add_told(AddTold::Offered, 0);
        side.add_told(created(&third(), CardSaved::NotSaved), 0);
        side.add_told(
            AddTold::PhotoPrepared {
                seq: 1,
                photo: None,
            },
            0,
        );
        assert!(side.hold_more(&[third()]));
        side.held_changed(&[
            held(first().as_str(), true),
            held(second().as_str(), false),
            held(third().as_str(), false),
        ])
        .expect("three held");
        let ids = side.model().identities();
        assert!(ids.add_form().is_none());
        assert!(!ids.identities_open(), "no list opened");
        assert_eq!(ids.focus_request(), before);
        assert_eq!(ids.added_snackbar(), None);
    }

    /// Android composes its roster with no add (plan §1.1): a root that
    /// never said it can add draws no Add, opens no form and sends no
    /// act, the list's opening included; one that stops being able to
    /// lets its form go. Mutation: a root that can add by default.
    #[test]
    fn a_root_that_cannot_add_sends_no_act() {
        let log: Log = std::rc::Rc::default();
        let mut side = person_side(&log);
        act(
            &mut side,
            vec![IdentityIntent::OpenIdentities, IdentityIntent::Add],
        );
        assert!(form_acts(&mut side, vec![anna(), AddIntent::Create]).is_empty());
        let ids = side.model().identities();
        assert!(ids.add_form().is_none());
        assert_eq!(ids.your_identities().and_then(|p| p.add), None);

        let (mut side, _) = adding_side(&log);
        assert!(side.model().identities().add_form().is_some());
        side.can_add(false);
        assert!(side.model().identities().add_form().is_none());
        assert!(form_acts(&mut side, vec![AddIntent::Leave]).is_empty());
    }

    /// F28 (identities §6.2, the settle): with one identity held the card
    /// editor's overflow offers Add another identity; a second identity
    /// found while it shows takes the item away -- in this build the
    /// first identity's overflow then holds nothing, so the menu closes
    /// with its button gone -- in every model held. Mutation: the root's
    /// `can_add` not told to the models -- no item to begin with. The
    /// item's going is the identities' own
    /// (`the_editors_overflow_offers_add_another_with_one_held_alone`).
    #[test]
    fn a_second_identity_found_while_the_editor_shows_closes_its_overflow_menu() {
        let mut side = ModelSide::new(Nothing, Nothing);
        side.hold(&[first()], &first());
        side.can_add(true);
        side.held_changed(&[held(first().as_str(), true)])
            .expect("one held");
        assert_eq!(
            side.model().identities().editor_overflow(),
            Some(interweave_human_ui_model::UiText::AddAnotherIdentity)
        );
        assert!(side.hold_more(&[second()]));
        side.held_changed(&[held(first().as_str(), true), held(second().as_str(), false)])
            .expect("two held");
        assert_eq!(side.model().identities().editor_overflow(), None);
        assert_eq!(
            side.model_of(&second())
                .expect("held")
                .identities()
                .editor_overflow(),
            None
        );
    }

    /// The form is the profile's, not one identity's: every model held is
    /// told it, so a switch shows it still, and a model made later is
    /// told it too. Mutation: the form told to the front's model alone.
    #[test]
    fn the_form_reaches_every_model_and_a_switch_keeps_it() {
        let log: Log = std::rc::Rc::default();
        let (mut side, _) = adding_side(&log);
        form_acts(&mut side, vec![anna()]);
        let held_form = |side: &ModelSide<Person, Nothing>, identity: &EndpointId| {
            side.model_of(identity)
                .and_then(|m| m.identities().add_form())
                .map(|f| f.values().name.clone())
        };
        assert_eq!(held_form(&side, &second()).as_deref(), Some("Anna"));
        assert!(side.switch(&second(), |_| {}));
        assert_eq!(
            side.model()
                .identities()
                .add_form()
                .map(|f| f.values().name.clone()),
            Some("Anna".to_owned())
        );
        assert!(side.hold_more(&[third()]));
        assert_eq!(held_form(&side, &third()).as_deref(), Some("Anna"));
    }

    /// A surface with a page of an identity behind the front: what it
    /// drew the page from, and the person's acts on the page and the
    /// identities.
    #[derive(Default)]
    struct Paged {
        drawn: Vec<Option<EndpointId>>,
        page_events: Vec<ViewEvent>,
        intents: Vec<IdentityIntent>,
    }

    impl Surface for Paged {
        fn render(&mut self, _model: &UiModel) {}
        fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            Vec::new()
        }
        fn take_identity_intents(&mut self) -> Vec<IdentityIntent> {
            std::mem::take(&mut self.intents)
        }
        fn render_page(&mut self, model: &UiModel) {
            self.drawn
                .push(model.identity().map(|view| view.endpoint.clone()));
        }
        fn take_page_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            std::mem::take(&mut self.page_events)
        }
    }

    fn view_of(endpoint: &EndpointId, name: &str) -> interweave_human_ui_model::IdentityView {
        interweave_human_ui_model::IdentityView {
            endpoint: endpoint.clone(),
            name: name.to_owned(),
            given: name.to_owned(),
            surname: None,
            has_photo: false,
            photo: None,
            key: interweave_human_ui_model::OwnKey::Ready {
                fingerprint: "0123456789abcdef".to_owned(),
            },
            peer: None,
        }
    }

    /// Three held, the first in front, each model shown its own card, and
    /// the page of `page` opened from Your identities.
    fn paged_side(page: &EndpointId) -> ModelSide<Paged, Nothing> {
        let mut side = ModelSide::new(Paged::default(), Nothing);
        side.hold(&[first(), second(), third()], &first());
        side.held_changed(&[
            held(first().as_str(), true),
            held(second().as_str(), false),
            held(third().as_str(), false),
        ])
        .expect("three held");
        for (identity, name) in [(first(), "First"), (second(), "Second"), (third(), "Third")] {
            assert!(side.apply_as(&identity, Update::IdentityShown(view_of(&identity, name))));
        }
        side.surface_mut().intents = vec![
            IdentityIntent::OpenIdentities,
            IdentityIntent::OpenRow(key_of(page)),
        ];
        side.take_identity_acts(|_| {});
        side
    }

    fn save(name: &str) -> ViewEvent {
        ViewEvent::Intent(Intent::SetOwnCard {
            name: name.to_owned(),
            surname: None,
            photo: interweave_human_ui_model::PhotoEdit::Keep,
        })
    }

    /// Identities §6.2 step 4 and cards §5.2: the page of an identity
    /// behind the front is drawn from THAT identity's model, its card the
    /// one shown; the front's own page is its editor and draws nothing
    /// here; with no page open nothing is drawn. Mutation: the page drawn
    /// from the front's model.
    #[test]
    fn a_page_of_an_identity_behind_draws_that_identitys_card() {
        let mut side = paged_side(&third());
        side.page_turn(|_| {});
        assert_eq!(side.surface().drawn, [Some(third())]);
        assert_eq!(side.front(), Some(&first()), "nothing switched");
        assert_eq!(
            side.model().identity().map(|v| v.name.as_str()),
            Some("First"),
            "the front's model is back in front"
        );

        let mut own = paged_side(&first());
        own.page_turn(|_| {});
        assert!(own.surface().drawn.is_empty(), "the front's own editor");
    }

    /// Save on the page of an identity behind the front is sent as THAT
    /// identity's command, a crossing naming it, while the identity in
    /// front sends nothing; a second Save before the first is answered is
    /// held back as that identity's own, never the front's. Mutation:
    /// the Save sent as the front.
    #[test]
    fn save_on_that_page_writes_that_identitys_card_and_no_other() {
        let mut side = paged_side(&second());
        side.surface_mut().page_events = vec![save("Anna")];
        let mut sent = Vec::new();
        side.page_turn(|c| sent.push(c));
        assert_eq!(
            sent,
            [Crossing::Command {
                identity: second(),
                command: Command::SetOwnCard {
                    name: "Anna".to_owned(),
                    surname: None,
                    photo: interweave_human_ui_model::PhotoEdit::Keep,
                },
            }]
        );
        assert!(side.turn().is_empty(), "the front sends nothing");
        side.surface_mut().page_events = vec![save("Anna")];
        side.page_turn(|c| sent.push(c));
        assert_eq!(sent.len(), 1, "in flight as the second's");
    }

    /// A photo chosen on that page is that identity's own photo edit:
    /// prepared as its command, its number that model's. Mutation: the
    /// choice sent as the front.
    #[test]
    fn a_photo_chosen_on_that_page_is_that_identitys_edit() {
        let mut side = paged_side(&third());
        side.surface_mut().page_events =
            vec![ViewEvent::Intent(Intent::ChooseOwnPhoto(vec![1, 2, 3]))];
        let mut sent = Vec::new();
        side.page_turn(|c| sent.push(c));
        match sent.as_slice() {
            [
                Crossing::Command {
                    identity,
                    command: Command::PrepareOwnPhoto { chosen, .. },
                },
            ] => {
                assert_eq!(identity, &third());
                assert_eq!(chosen, &[1, 2, 3]);
            }
            other => panic!("{other:?}"),
        }
    }

    /// Only the card's acts leave a page of an identity behind: a send,
    /// a trust act or a reopen made there is dropped, so nothing is done
    /// as an identity the person is not using. Mutation: every intent on
    /// the page carried out.
    #[test]
    fn only_the_cards_acts_leave_a_page_of_an_identity_behind() {
        let mut side = paged_side(&second());
        side.surface_mut().page_events = vec![
            ViewEvent::Intent(Intent::Reopen),
            ViewEvent::Intent(Intent::RecheckStorage),
            ViewEvent::Intent(Intent::RetryKey),
        ];
        let mut sent = Vec::new();
        side.page_turn(|c| sent.push(c));
        assert!(
            sent.iter().all(|c| matches!(
                c,
                Crossing::Command {
                    command: Command::RetryKey,
                    ..
                }
            )),
            "{sent:?}"
        );
    }

    /// F9: a row whose card Done did not write says so until a Save on
    /// its page lands -- the held list then gives it a card's name -- and
    /// its page then holds the form's values no longer. Mutation: a held
    /// list after Done's not told to the front's model -- the line stays.
    #[test]
    fn a_save_that_lands_clears_the_rows_not_saved_line() {
        let log: Log = std::rc::Rc::default();
        let (mut side, _) = adding_side(&log);
        side.add_told(AddTold::Offered, 0);
        form_acts(&mut side, vec![anna(), AddIntent::Create]);
        side.add_told(created(&third(), CardSaved::NotSaved), 0);
        assert!(side.hold_more(&[third()]));
        let mut list = vec![
            held(first().as_str(), true),
            held(second().as_str(), false),
            held(third().as_str(), false),
        ];
        side.held_changed(&list).expect("three held");
        let row = |side: &ModelSide<Person, Nothing>| {
            side.model()
                .identities()
                .your_identities()
                .and_then(|p| p.rows.into_iter().find(|r| r.key == key_of(&third())))
                .expect("its row")
        };
        assert!(row(&side).card_not_saved.is_some());
        act(&mut side, vec![IdentityIntent::OpenRow(key_of(&third()))]);
        assert!(
            side.model()
                .identities()
                .identity_page()
                .is_some_and(|p| p.unsaved.is_some())
        );

        // Its Save landed: the facade side reads the card's name.
        list[2].card_name = Some("Anna".to_owned());
        side.held_changed(&list).expect("three held");
        assert_eq!(row(&side).card_not_saved, None);
        assert!(
            side.model()
                .identities()
                .identity_page()
                .is_some_and(|p| p.unsaved.is_none())
        );
    }
}
