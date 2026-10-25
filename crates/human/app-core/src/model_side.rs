// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The model side: owns the [`UiModel`], applies the facade side's
//! [`Update`]s to it, drives a view through [`Surface`], and turns the
//! intents the view resolves into [`Command`]s.

use std::collections::{HashSet, VecDeque};

use interweave_human_chat_protocol::is_allowed_link_scheme;
use interweave_human_client_api::{ClientEvent, SessionState};
use interweave_human_ui_model::{ConversationKey, Intent, UiModel, ViewEvent};

use crate::facade::CANCELLED_CAP;
use crate::protocol::{Command, Failure, Update};

/// A view as the root drives it: show the model, then hand back what the
/// person did. `ui-slint`'s `View` is one; a test can script one.
pub trait Surface {
    /// Show `model`.
    fn render(&mut self, model: &UiModel);
    /// What the person did since the last call, resolved against `model`.
    /// A call returns at most up to the first draft edit, which the root
    /// applies before calling again.
    fn take_events(&mut self, model: &UiModel) -> Vec<ViewEvent>;
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
    /// The store's unread rows could not be listed again.
    UnreadNotListed(Failure),
    /// A card's answer or its Undo was carried out, and the cards or
    /// contacts could not be read back afterwards.
    CardsNotRead(Failure),
}

/// How many problems wait for [`ModelSide::take_problems`], at most; past
/// it the oldest goes and is counted in [`ModelSide::problems_dropped`].
/// A `MarkRead` the store keeps refusing is raised again at every focused
/// render, so a root that never takes them must not grow this.
pub const PROBLEM_CAP: usize = 32;

/// The model side of the root.
pub struct ModelSide<S: Surface, O: Opener> {
    model: UiModel,
    surface: S,
    opener: O,
    in_flight: HashSet<InFlight>,
    problems: VecDeque<Problem>,
    problems_dropped: u64,
    hidden_unread: usize,
    hidden_other: usize,
    /// The trust and address lists are to be read: a session became
    /// ready with either unread, which is the start for a runtime that is
    /// up, and the first moment both can be answered for one that was not.
    reads_owed: bool,
}

impl<S: Surface, O: Opener> ModelSide<S, O> {
    /// A model side over `surface`, opening activated links with `opener`.
    pub fn new(surface: S, opener: O) -> Self {
        Self {
            model: UiModel::new(),
            surface,
            opener,
            in_flight: HashSet::new(),
            problems: VecDeque::new(),
            problems_dropped: 0,
            hidden_unread: 0,
            hidden_other: 0,
            reads_owed: false,
        }
    }

    /// Whether a transport daemon serves the profile, as the root sees
    /// it: the facade cannot tell a missing daemon from any other failed
    /// open, the root can.
    pub fn daemon_seen(&mut self, present: Option<bool>) {
        self.model.daemon_seen(present);
    }

    /// Whether the platform gives this app network access, as the root
    /// sees it ([`UiModel::network_access`]).
    pub fn network_access(&mut self, allowed: bool) {
        self.model.network_access(allowed);
    }

    /// Whether nothing can run until the person authenticates, as the
    /// root learns it from the platform.
    pub fn authentication_needed(&mut self, needed: bool) {
        self.model.authentication_needed(needed);
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
        match update {
            Update::Listed(listing) => {
                // Pending first: it is authoritative over held updates.
                self.model.pending_listed(listing.pending);
                self.model.unread_listed(listing.unread);
                self.model.kept_listed(listing.kept);
                self.hidden_unread = listing.undecodable_unread;
                self.hidden_other = listing.undecodable_other;
                for proposal in listing.unread_cards {
                    self.model.card_received(proposal);
                }
                self.model.contacts_listed(listing.contacts);
                if let Some(identity) = listing.identity {
                    self.model.identity_shown(identity);
                }
            }
            Update::UnreadListed { rows, undecodable } => {
                self.model.unread_listed(rows);
                // A relist reads the unread rows only: their count is
                // replaced, the start's count of the others stands.
                self.hidden_unread = undecodable;
            }
            Update::UnreadNotListed(why) => self.problem(Problem::UnreadNotListed(why)),
            Update::Received(received) => self.model.received(received),
            Update::ReceivedCard(proposal) => self.model.card_received(*proposal),
            Update::ProposalJudged(proposal) => self.model.card_judged(*proposal),
            Update::CardResolved { row, outcome } => self.model.card_resolved(row, outcome),
            Update::ProposalRestored(proposal) => self.model.card_restored(*proposal),
            Update::ProposalNotRestored(row) => self.model.card_not_restored(row),
            Update::CardsNotRead(why) => self.problem(Problem::CardsNotRead(why)),
            Update::ContactsListed(contacts) => self.model.contacts_listed(contacts),
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
            Update::OwnPhotoPrepared(photo) => self.model.own_photo_prepared(photo),
            Update::NewKeyAsked { pending } => self.model.new_key_asked(pending),
            Update::KeyMade {
                identity,
                fingerprint,
                share_again,
            } => self.model.key_made(identity, fingerprint, share_again),
            Update::KeyNotMade {
                identity,
                problem,
                gone,
            } => self.model.key_not_made(identity, problem, gone),
            Update::KeyRetried(identity) => self.model.key_retried(identity),
            Update::CardShared { key, at, .. } => self.model.card_shared(key, at),
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
                // A key step the facade refused ends here: never one the
                // page waits on for good (#423 F1).
                if matches!(command, Command::AskNewKey | Command::MakeNewKey) {
                    self.model.key_command_refused();
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
                if std::mem::take(&mut unrendered) {
                    self.surface.render(&self.model);
                    continue;
                }
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
                    ViewEvent::Trust(input) => {
                        unrendered = true;
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
                }
            }
        }
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
            Intent::AskNewKey => {
                self.model.ask_new_key()?;
                Command::AskNewKey
            }
            Intent::MakeNewKey => {
                self.model.confirm_new_key()?;
                Command::MakeNewKey
            }
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
                self.model.own_photo_chosen();
                Command::PrepareOwnPhoto(chosen)
            }
            Intent::ForgetOwnPhoto => {
                self.model.own_photo_forgotten();
                Command::ForgetOwnPhoto
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

    #[test]
    fn a_relist_replaces_the_unread_count_and_keeps_the_others() {
        let mut side = ModelSide::new(Nothing, Nothing);
        side.apply(Update::Listed(crate::protocol::Listing {
            undecodable_unread: 2,
            undecodable_other: 1,
            ..crate::protocol::Listing::default()
        }));
        assert_eq!(side.hidden_rows(), 3);
        side.apply(Update::UnreadListed {
            rows: Vec::new(),
            undecodable: 0,
        });
        assert_eq!(side.hidden_rows(), 1, "the pending and kept count stands");
        side.apply(Update::UnreadListed {
            rows: Vec::new(),
            undecodable: 4,
        });
        assert_eq!(side.hidden_rows(), 5, "replaced, not added");
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
        use interweave_transport_api::{EndpointId, TransportIdentity};
        let peer = TransportIdentity::parse("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN")
            .expect("a PeerId");
        let endpoint = EndpointId::parse("human").expect("an endpoint");
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
        side.apply(Update::ContactsListed(vec![ContactView {
            contact_id: "c1".to_owned(),
            name: "Ada".to_owned(),
            name_from_card: true,
            name_at: None,
            photo_from_card: None,
            photo_at: None,
            photo: None,
            routes: Vec::new(),
        }]));
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
}
