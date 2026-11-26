// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The model side: owns the [`UiModel`], applies the facade side's
//! [`Update`]s to it, drives a view through [`Surface`], and turns the
//! intents the view resolves into [`Command`]s.

use std::collections::{HashSet, VecDeque};

use interweave_human_chat_protocol::is_allowed_link_scheme;
use interweave_human_client_api::{ClientEvent, SessionState};
use interweave_human_ui_model::{
    ConnectGate, ConversationKey, IdentityIntent, Intent, StartBusy, TrustInput, UiModel,
    UnlockFacts, ViewEvent, WindowEvent, WindowInput,
};
use interweave_transport_api::EndpointId;

use crate::facade::CANCELLED_CAP;
use crate::protocol::{Command, Crossing, Failure, Update, WindowCommand};

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
/// Not here: `start_busy` and `start_refused`, which the model itself
/// changes ([`UiModel::retry_start_asked`]), and `unlock_facts`, whose
/// `seen` flag the model changes as the Trust page shows its line; a copy
/// held here would go stale the moment the model changed its own. They
/// are the front's alone, so a root that holds more than one identity
/// must not call them: each asserts that no model is behind
/// (`a_front_only_fact_with_a_model_behind_is_refused`).
#[derive(Debug, Clone, Copy)]
struct RootFacts {
    daemon: Option<bool>,
    network_allowed: bool,
    authentication_needed: bool,
}

impl Default for RootFacts {
    fn default() -> Self {
        // What a fresh model reads before it is told anything.
        Self {
            daemon: None,
            network_allowed: true,
            authentication_needed: false,
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
        } = self;
        model.daemon_seen(daemon);
        model.network_access(network_allowed);
        model.authentication_needed(authentication_needed);
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
                self.behind.push((identity.clone(), made));
            }
        }
        true
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

    /// The service stopped under this model, which a root keeps showing
    /// -- its rows, its open conversation, its drafts -- while the start
    /// that follows is refused for the person (unlock note §11). What a
    /// kept model must not do is what review F3 found in one: read Ready,
    /// and hold a command in flight no facade will answer. So every
    /// command in flight fails as [`Failure::ServiceStopped`], and the
    /// session reads as a fresh model's with no service: not running.
    pub fn stopped(&mut self) {
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
        self.daemon_seen(Some(false));
    }

    /// Every draft holding text ([`UiModel::drafts`]), for the model a new
    /// runtime's start makes.
    #[must_use]
    pub fn drafts(&self) -> Vec<(ConversationKey, String)> {
        self.model.drafts()
    }

    /// Carry `drafts` over from the model before this one: what is written
    /// waits across a restart (unlock note §11, A17).
    pub fn carry_drafts(&mut self, drafts: Vec<(ConversationKey, String)>) {
        for (key, draft) in drafts {
            self.model.draft_changed(key, draft);
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
        debug_assert!(
            self.behind.is_empty(),
            "a front-only fact told with a model behind (RootFacts)"
        );
        self.model.start_busy(busy);
    }

    /// Whether the last start compared with C4's `PeerId` and was refused
    /// after the bound, or before it from a held R2, R3 or R4 that opened
    /// Chats in its place, and whether a start for its notice runs, as the
    /// root learns it from the platform ([`UiModel::start_refused`]).
    pub fn start_refused(&mut self, refused: Option<StartBusy>) {
        debug_assert!(
            self.behind.is_empty(),
            "a front-only fact told with a model behind (RootFacts)"
        );
        self.model.start_refused(refused);
    }

    /// The unlock note's facts as the platform says them, the count of
    /// the start they are from, and whether its P-up line was seen.
    pub fn unlock_facts(&mut self, facts: Option<UnlockFacts>, start: u64, seen: bool) {
        debug_assert!(
            self.behind.is_empty(),
            "a front-only fact told with a model behind (RootFacts)"
        );
        self.model.unlock_facts(facts, start, seen);
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
                self.model.retry_start_asked();
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

    /// A draft carried over into a new model reads as written there.
    #[test]
    fn drafts_carry_over_into_a_new_model() {
        let mut old = ModelSide::new(Nothing, Nothing);
        let key = ConversationKey::Channel(
            interweave_transport_api::ChannelId::parse("general").expect("a channel"),
        );
        old.model
            .draft_changed(key.clone(), "still here".to_owned());
        let mut new = ModelSide::new(Nothing, Nothing);
        new.carry_drafts(old.drafts());
        assert_eq!(new.model().composer(&key).draft, "still here");
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

    /// The three facts `RootFacts` leaves to the front alone are refused
    /// once a model is behind, so the doc's "must not call them" holds.
    /// Mutation: dropping the assertion from any of the three -- its test
    /// does not panic.
    #[test]
    #[should_panic(expected = "a front-only fact told with a model behind")]
    fn a_front_only_fact_with_a_model_behind_is_refused() {
        let mut side = ModelSide::new(Nothing, Nothing);
        assert!(side.hold(&[first(), second()], &first()));
        side.start_busy(None);
    }

    #[test]
    #[should_panic(expected = "a front-only fact told with a model behind")]
    fn start_refused_with_a_model_behind_is_refused() {
        let mut side = ModelSide::new(Nothing, Nothing);
        assert!(side.hold(&[first(), second()], &first()));
        side.start_refused(None);
    }

    #[test]
    #[should_panic(expected = "a front-only fact told with a model behind")]
    fn unlock_facts_with_a_model_behind_are_refused() {
        let mut side = ModelSide::new(Nothing, Nothing);
        assert!(side.hold(&[first(), second()], &first()));
        side.unlock_facts(None, 0, false);
    }

    /// Named, the model shown so far is the front's, drafts and all, and
    /// every other identity held has a model of its own, empty; a second
    /// naming changes nothing. Mutations: `hold` giving the front a fresh
    /// model -- the draft is gone; `hold` naming again -- the front moves.
    #[test]
    fn naming_the_held_identities_keeps_the_model_shown_as_the_fronts() {
        let mut side = ModelSide::new(Nothing, Nothing);
        assert_eq!(side.front(), None);
        side.carry_drafts(vec![(general(), "written".to_owned())]);
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
        side.carry_drafts(vec![(general(), "written".to_owned())]);
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
}
