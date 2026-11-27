// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Activity's model side, fed by the hub: what the window shows
//! follows whether the network service runs, as well as what it sends.
//!
//! A change of running state starts a fresh model either way. Started:
//! the listing that follows is a new runtime's, and only the drafts carry
//! over, so what is written waits. Stopped: the window holds no rows and
//! no Ready session, so it shows the service as not running and offers
//! nothing that sends -- a kept model would still read Ready, show no
//! notice, and mark a Send in flight that no facade would ever answer
//! (review F3).
//!
//! One exception: a stop the person's lock follows. A restart refused
//! until the person authenticates neither empties the list nor closes a
//! conversation (unlock note §11), so the stopped model is set aside --
//! stopped, which fails what it held in flight and drops its Ready
//! session -- and shown again once the lock is said while nothing runs.
//! A store answer other than the lock's drops it: the start it answers
//! ran or was refused for another reason, and what it showed replaced the
//! stopped model, so the set-aside rows are no longer the list as it was
//! (a refused or discarded store's rows never come back).
//!
//! Every model made here is named with the identities the hub last said
//! are held ([`ToView::Held`]) as it is made, and every update applies to
//! the model of the identity it names, as the desktop's window does: a
//! model side applies an update naming an identity it does not hold to
//! none (`an_update_reaches_the_model_of_the_identity_it_names`).

use interweave_human_app_core::{
    AddAct, AddTold, CardSaved, Command, Crossing, Failure, ModelSide, Opener, Surface, Update,
};
use interweave_human_transport_client::AddOutcome;
use interweave_human_ui_model::{ConnectPhase, HeldIdentity, HeldProblem, StartBusy};
use interweave_transport_api::EndpointId;

use crate::hub::{Held, StoreAnswer, StoreGeneration, ToView, UnlockSaid};

/// The store could not be listed for this view: what it holds stays
/// unshown until the view attaches again. A class, never content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListingFailed;

/// How many add classes wait for [`ViewSide::take_add_classes`] at most;
/// past it the oldest goes (`add_classes_wait_to_a_bound`).
const ADD_CLASS_CAP: usize = 32;

/// What came of an add act or a settle, as the platform log names it: its
/// class, never the identity, its alias or anything its card says. The
/// match has no wildcard, so an [`AddTold`] given a variant does not
/// build until it is classed here, and no class is a format of the
/// variant's fields (`add_classes_name_no_id_alias_or_card`).
#[must_use]
pub const fn add_class(told: &AddTold) -> &'static str {
    match told {
        AddTold::Offered => "create offered",
        AddTold::WaitsSeed => "create waits for the first identity's seed",
        AddTold::NoSeed => "the first identity's seed was not answered",
        AddTold::OutOfBounds => "the card is outside a card's bounds",
        AddTold::NotStarted => "create did not start",
        AddTold::Ended { outcome, card } => match (outcome, card) {
            (AddOutcome::Created { .. }, Some(CardSaved::Saved)) => "created, card saved",
            (AddOutcome::Created { .. }, Some(CardSaved::NotSaved)) => "created, card not saved",
            (AddOutcome::Created { .. }, None) => "created",
            (AddOutcome::NotFinished, _) => "created, not finished",
            (AddOutcome::Failed, _) => "refused, nothing created",
            (AddOutcome::Unknown, _) => "outcome unknown",
            (AddOutcome::Limit, _) => "refused at the limit",
            (AddOutcome::KeyGone, _) => "the form's key is gone",
        },
        AddTold::Held(_) => "an identity is held from now on",
        AddTold::NotHeld(_) => "an identity is not held yet, tried again",
        AddTold::NotHeldUntilRestart(_) => "an identity is not held, not tried again until restart",
    }
}

/// The model side a view attached to the hub drives.
pub struct ViewSide<S: Surface, O: Opener> {
    side: ModelSide<S, O>,
    fresh: Box<dyn FnMut() -> ModelSide<S, O>>,
    /// The identities held, as the hub last said them; `None` until it
    /// has.
    held: Option<Held>,
    /// The held identities' rows, as the hub last said them; `None` until
    /// it has. Told to every model made, as `held` names it.
    identities: Option<Vec<HeldIdentity>>,
    /// Why the last list told was refused by the model, until the root
    /// takes it ([`Self::take_held_refused`]).
    held_refused: Option<HeldProblem>,
    /// Network access as the hub last said it.
    network: bool,
    /// Whether the person must authenticate first, as the hub last said.
    authentication: bool,
    /// The last start's answer and the hub's count with it, as the hub
    /// last said; `None` until a start has answered.
    store: Option<(StoreAnswer, StoreGeneration)>,
    /// Whether the service runs, as the hub last said.
    running: bool,
    /// The count of the start the hub last said runs; `None` until one
    /// has.
    start: Option<u64>,
    /// The unlock facts, as the hub last said them.
    unlock: Option<UnlockSaid>,
    /// The hub's count of answers when a running service last stopped:
    /// the start that answered last before it ran and ended.
    stopped: Option<StoreGeneration>,
    /// The model the last stop set aside, stopped: shown again if the
    /// lock is said while nothing runs, its drafts carried by the next
    /// start.
    kept: Option<ModelSide<S, O>>,
    /// `IdStartBusy`'s notice, as the hub last said it: the process's,
    /// so every model made here shows it.
    busy: Option<StartBusy>,
    /// `Refused`'s, for a start that compared with C4's `PeerId`
    /// ([`ToView::StartRefused`]), kept as `busy` is.
    refused: Option<StartBusy>,
    /// What the add form's acts and the settles came to, as classes for
    /// the platform log, until the root takes them
    /// ([`Self::take_add_classes`]): no screen shows them yet.
    add_classes: Vec<&'static str>,
}

impl<S: Surface, O: Opener> ViewSide<S, O> {
    /// `fresh` makes a model side over the view: called now, and again at
    /// every change of the service's running state.
    pub fn new(mut fresh: impl FnMut() -> ModelSide<S, O> + 'static) -> Self {
        Self {
            side: fresh(),
            fresh: Box::new(fresh),
            held: None,
            identities: None,
            held_refused: None,
            network: true,
            authentication: false,
            store: None,
            running: false,
            start: None,
            unlock: None,
            stopped: None,
            kept: None,
            busy: None,
            refused: None,
            add_classes: Vec::new(),
        }
    }

    /// The last start's answer as the store saw it (ADR-0060 decision 6),
    /// with the hub's count of answers when it was said, which a view
    /// attached again hears unchanged; `None` until a start has answered.
    #[must_use]
    pub const fn store_answer(&self) -> Option<(StoreAnswer, StoreGeneration)> {
        self.store
    }

    /// A model made again, named with the identities held and told their
    /// rows as the hub last said them, none once the service stopped
    /// (`a_model_made_again_is_told_the_identities_again`).
    fn made(&mut self) -> ModelSide<S, O> {
        let mut side = (self.fresh)();
        if let Some(held) = &self.held {
            side.hold(&held.held, &held.front);
        }
        if let Some(identities) = &self.identities
            && let Err(problem) = side.held_changed(identities)
        {
            self.held_refused = Some(problem);
        }
        side
    }

    /// Why the last held list told was refused by the model, if it was,
    /// once: a class for the platform log.
    pub const fn take_held_refused(&mut self) -> Option<HeldProblem> {
        self.held_refused.take()
    }

    /// Apply one message from the hub.
    ///
    /// # Errors
    /// [`ListingFailed`] when the hub says the store could not be listed,
    /// for the root to report.
    pub fn apply(&mut self, message: ToView) -> Result<(), ListingFailed> {
        match message {
            ToView::Running(running) => {
                if !running {
                    // As the hub drops its rows at a stop, and tells a view
                    // attached after one none: the model made now is not
                    // told rows no facade holds.
                    self.identities = None;
                }
                let made = self.made();
                let last = std::mem::replace(&mut self.side, made);
                if running {
                    // The model set aside first, then the one shown since,
                    // whose edits are the later.
                    let mut drafts = self
                        .kept
                        .take()
                        .map(|kept| kept.held_drafts())
                        .unwrap_or_default();
                    drafts.extend(last.held_drafts());
                    self.side.carry_held_drafts(drafts);
                } else if self.running {
                    // Recorded as it is applied, so a stop and the next
                    // start drained in one pump still show it.
                    self.stopped = self.store.map(|(_, generation)| generation);
                    let mut last = last;
                    last.stopped();
                    self.kept = Some(last);
                }
                self.running = running;
                self.side.daemon_seen(Some(running));
                // Access is the platform's, not the service's: a fresh
                // model keeps what the hub last said of it.
                self.side.network_access(self.network);
                self.side.authentication_needed(self.authentication);
                self.side.start_busy(self.busy);
                self.side.start_refused(self.refused);
                self.say_unlock();
                if !running && self.authentication {
                    self.show_kept();
                }
            }
            ToView::Held(held) => {
                // The model shown is named now too: no update can reach it
                // before, and a model already named keeps its names.
                self.side.hold(&held.held, &held.front);
                self.held = Some(held);
            }
            // Before the updates and the held list that name them: the
            // facade sends them in that order. The held list the model
            // side is made with again names them too.
            ToView::HeldMore(more) => {
                if let Some(held) = self.held.as_mut() {
                    for identity in &more {
                        if !held.held.contains(identity) {
                            held.held.push(identity.clone());
                        }
                    }
                }
                self.side.hold_more(&more);
            }
            ToView::Add(told) => {
                if self.add_classes.len() == ADD_CLASS_CAP {
                    self.add_classes.remove(0);
                }
                self.add_classes.push(add_class(&told));
            }
            ToView::Identities(identities) => {
                if let Err(problem) = self.side.held_changed(&identities) {
                    self.held_refused = Some(problem);
                }
                self.identities = Some(identities);
            }
            ToView::Start(start) => self.start = Some(start),
            ToView::NetworkAccess(allowed) => {
                self.network = allowed;
                self.side.network_access(allowed);
            }
            ToView::AuthenticationNeeded(needed) => {
                let lifted = self.authentication && !needed;
                self.authentication = needed;
                self.side.authentication_needed(needed);
                if needed && !self.running {
                    self.show_kept();
                }
                // Lifted while nothing runs: a start is on its way -- the
                // grant's, or any start's, which lifts it first -- so the
                // session reads re-opening rather than "not running" until
                // the start answers (j98 A17 finding 3).
                if lifted && !self.running {
                    self.side.daemon_seen(None);
                }
            }
            // An identity not held has no model to apply to: with the
            // hub saying the held before the start it names, none comes.
            ToView::Update(identity, update) => {
                let _ = self.side.apply_as(&identity, *update);
            }
            ToView::ListingFailed => return Err(ListingFailed),
            ToView::Unlock(said) => {
                self.unlock = Some(said);
                self.say_unlock();
            }
            ToView::StartBusy(busy) => {
                self.busy = busy;
                self.side.start_busy(busy);
            }
            ToView::StartRefused(refused) => {
                self.refused = refused;
                self.side.start_refused(refused);
            }
            ToView::Store(answer, generation) => {
                // Any other answer is a start that ran or was refused for
                // a reason other than the lock: what it showed replaced the
                // stopped model, so a later lock must not bring the stopped
                // rows back (a refused or discarded store's among them).
                if answer != StoreAnswer::Locked {
                    self.kept = None;
                }
                // A refused start leaves nothing running; an opened one's
                // service says so itself, whichever message comes first.
                if !self.running && answer != StoreAnswer::Opened {
                    self.side.daemon_seen(Some(false));
                }
                self.store = Some((answer, generation));
            }
        }
        Ok(())
    }

    /// The classes of what the add form's acts and the settles came to
    /// since the last call, in order, for the platform log.
    pub fn take_add_classes(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.add_classes)
    }

    /// Hand what the person did with the add form or the list of
    /// identities to the facade through `send` -- the hub's
    /// [`add`](crate::Hub::add) -- in the order it was sent among the
    /// commands and switches. False once nothing runs to take it
    /// (`an_add_act_reaches_the_facade_in_the_order_sent_and_none_once_nothing_runs`).
    pub fn add(&mut self, act: AddAct, send: impl FnOnce(AddAct) -> Result<(), AddAct>) -> bool {
        send(act).is_ok()
    }

    /// The model the last stop set aside, shown again: the restart is
    /// refused for the person, and the list and an open conversation stay
    /// as they were (unlock note §11).
    fn show_kept(&mut self) {
        if let Some(mut kept) = self.kept.take() {
            kept.network_access(self.network);
            kept.authentication_needed(self.authentication);
            kept.start_busy(self.busy);
            kept.start_refused(self.refused);
            kept.carry_held_drafts(self.side.held_drafts());
            kept.remade();
            self.side = kept;
            // Set aside before the facts a refused restart learnt: the
            // locked Trust page shows the hub's last word, not the
            // running client's (unlock note A13(b); #610 review F1).
            self.say_unlock();
        }
    }

    /// The unlock facts as the hub last said them, to the model shown:
    /// they outlive a change of the running state, as the record does.
    fn say_unlock(&mut self) {
        if let Some(said) = self.unlock {
            self.side.unlock_facts(said.facts, said.start, said.seen);
        }
    }

    /// Whether the start a held C4 or R4 waits on has decided which screen
    /// opens (add-a-peer §5.1). `asked_at` is the hub's
    /// [`store_generation`](crate::Hub::store_generation) read before
    /// the start was asked, so an earlier start's answer, which a view
    /// attaching hears again, decides nothing. Decided when the start was
    /// refused -- for the person, whose locked notice Chats shows, or by
    /// any later answer other than an opened store -- and, once the service
    /// runs, when nothing is owed (the start's success decides) or the
    /// model's reads settled, opened or deferred Connect.
    #[must_use]
    pub fn start_decided(&self, asked_at: StoreGeneration) -> bool {
        if self.authentication {
            return true;
        }
        if let Some((answer, generation)) = self.store
            && generation > asked_at
            && answer != StoreAnswer::Opened
        {
            return true;
        }
        self.running && self.side.model().connect_phase() != ConnectPhase::Owed
    }

    /// Whether the start a held screen waits on was refused with the
    /// profile in use (identity note §6): an answer counted past
    /// `asked_at`, as [`start_decided`](Self::start_decided) reads one,
    /// that is [`StoreAnswer::ProfileBusy`]. Before the bound the screen
    /// shows `IdStartBusy` rather than handing over to Chats; after it,
    /// Chats shows the notice ([`ToView::StartBusy`]).
    #[must_use]
    pub fn start_busy(&self, asked_at: StoreGeneration) -> bool {
        self.store.is_some_and(|(answer, generation)| {
            generation > asked_at && answer == StoreAnswer::ProfileBusy
        })
    }

    /// Whether the start a held C4 or R4 waits on ran and then ended
    /// without a refusal (add-a-peer §5.1): a running service stopped
    /// after an answer counted past `asked_at`. On a foreground-only
    /// profile that is Home letting go of the binding, and the return is
    /// a new launch, not this start's to decide. True also once a later
    /// start runs, so a turn that comes only at the return, with the stop
    /// and the next start in one pump, still ends the held screen. A
    /// refusal never ran, so it is [`start_decided`](Self::start_decided)'s.
    #[must_use]
    pub fn start_ended(&self, asked_at: StoreGeneration) -> bool {
        self.stopped.is_some_and(|generation| generation > asked_at)
    }

    /// Whether the service runs, as the hub last said.
    #[must_use]
    pub const fn running(&self) -> bool {
        self.running
    }

    /// The count of the start that runs, while one does: the hub says it
    /// before it says the service runs, so it is never an earlier start's
    /// while [`running`](Self::running) is true.
    #[must_use]
    pub const fn start(&self) -> Option<u64> {
        if self.running { self.start } else { None }
    }

    /// Bring `to` in front, as the desktop's window does: the model side
    /// drains the old front's acts and hands them to `send` as that
    /// identity's, then the switch (`ModelSide::switch`), for the root to
    /// cross to the facade ([`crate::Hub::cross`]), each crossing carried
    /// as [`Self::take_identity_acts`] carries them: a switch handed back
    /// names no model made later with `to`, and a command handed back is
    /// failed to its identity's model. False, and nothing sent, when `to`
    /// is in front or not held -- with one identity held, always
    /// (`a_switch_crosses_to_the_hub_and_with_one_held_none_can`).
    pub fn switch(
        &mut self,
        to: &EndpointId,
        cross: impl FnMut(Crossing) -> Result<(), Crossing>,
    ) -> bool {
        let mut switched = false;
        self.carry(cross, |side, send| switched = side.switch(to, send));
        switched
    }

    /// The identity in front is `to`: a model made again is named with it
    /// ([`Self::made`]) and not with the one the start named
    /// (`a_model_made_after_a_switch_is_named_with_the_new_front`).
    fn fronted(&mut self, to: &EndpointId) {
        if let Some(held) = self.held.as_mut() {
            held.front = to.clone();
        }
    }

    /// What the person did to the identities, before the model's turn,
    /// as the desktop's window takes them (`ModelSide::take_identity_acts`):
    /// each crossing handed to `cross` -- the hub's
    /// [`cross`](crate::Hub::cross) -- in its order, a switch's drained
    /// commands of the old front before its [`Crossing::Front`]. Once
    /// `cross` hands one back, no facade takes them: nothing more is
    /// crossed, and each command handed back or not crossed is failed
    /// explicitly to the model of the identity it names, so none waits in
    /// flight for an answer no facade will give (owner's finding P2-2).
    /// True when one was handed back
    /// (`a_switch_intent_reaches_the_facade_after_the_old_fronts_commands`).
    pub fn take_identity_acts(
        &mut self,
        cross: impl FnMut(Crossing) -> Result<(), Crossing>,
    ) -> bool {
        self.carry(cross, |side, send| side.take_identity_acts(send))
    }

    /// Run `act` on the model side, each crossing it sends handed to
    /// `cross` until one is handed back; the front is recorded only from
    /// an accepted [`Crossing::Front`]
    /// (`a_switch_crosses_to_the_hub_and_with_one_held_none_can`,
    /// `a_model_made_after_a_switch_is_named_with_the_new_front`), and every
    /// command handed back or not crossed is failed to its identity's
    /// model. True when one was handed back.
    fn carry(
        &mut self,
        mut cross: impl FnMut(Crossing) -> Result<(), Crossing>,
        act: impl FnOnce(&mut ModelSide<S, O>, &mut dyn FnMut(Crossing)),
    ) -> bool {
        let mut stopped = false;
        let mut failed: Vec<(EndpointId, Command)> = Vec::new();
        let mut fronted = None;
        act(&mut self.side, &mut |crossing| {
            let front = match &crossing {
                Crossing::Front(to) => Some(to.clone()),
                Crossing::Command { .. } => None,
            };
            let back = if stopped {
                Err(crossing)
            } else {
                cross(crossing)
            };
            match back {
                Ok(()) => {
                    if front.is_some() {
                        fronted = front;
                    }
                }
                Err(crossing) => {
                    stopped = true;
                    if let Crossing::Command { identity, command } = crossing {
                        failed.push((identity, command));
                    }
                }
            }
        });
        if let Some(to) = fronted {
            self.fronted(&to);
        }
        for (identity, command) in failed {
            let _ = self.side.apply_as(
                &identity,
                Update::Failed {
                    command,
                    why: Failure::ServiceStopped,
                },
            );
        }
        stopped
    }

    /// The model side, for the root's turn.
    pub const fn side_mut(&mut self) -> &mut ModelSide<S, O> {
        &mut self.side
    }

    /// The model side, to read.
    #[must_use]
    pub const fn side(&self) -> &ModelSide<S, O> {
        &self.side
    }
}

#[cfg(test)]
mod tests {
    use interweave_human_app_core::Update;
    use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
    use interweave_human_client_api::{ClientEvent, Origin, Received, SessionState};
    use interweave_human_core::RowId;
    use interweave_human_ui_model::{IdentityIntent, SessionNotice, StartBusy, UiModel, ViewEvent};
    use interweave_profile_identity::ProfileIdentity;

    use super::*;

    struct Blank;

    impl Surface for Blank {
        fn render(&mut self, _model: &UiModel) {}

        fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            Vec::new()
        }
    }

    impl Opener for Blank {
        fn open(&mut self, _destination: &str) {}
        fn ask_network_access(&mut self) {}
        fn authenticate(&mut self) {}
    }

    /// A view side told the one identity held, as the hub tells it before
    /// a start.
    fn view_side() -> ViewSide<Blank, Blank> {
        let mut side = ViewSide::new(|| ModelSide::new(Blank, Blank));
        side.apply(ToView::Held(held())).expect("applied");
        side
    }

    /// The one identity a facade of these tests holds.
    fn me() -> EndpointId {
        interweave_test_support::endpoint_keys::endpoint_id("human")
    }

    fn held() -> Held {
        Held {
            held: vec![me()],
            front: me(),
        }
    }

    fn arrival() -> Received {
        let peer = ProfileIdentity::generate()
            .transport_identity()
            .expect("a peer id");
        Received {
            row: RowId::from_stored(1),
            origin: Origin::Direct {
                peer,
                endpoint: interweave_test_support::endpoint_keys::endpoint_id("human"),
            },
            envelope: HumanChatV2 {
                v: 2,
                kind: MessageKind::Text,
                app_message_id: format!("{:032x}", 1),
                text: "hello".to_owned(),
                reply_to: None,
                sent_at_ms: None,
                from_endpoint: None,
            },
            received_at: 1,
        }
    }

    fn model(side: &ViewSide<Blank, Blank>) -> &UiModel {
        side.side().model()
    }

    fn other() -> EndpointId {
        interweave_test_support::endpoint_keys::endpoint_id("other")
    }

    /// An update applies to the model of the identity it names: the one
    /// held, in front, shows it; one naming an identity not held applies
    /// to nothing. Mutation: applying every update to the model shown,
    /// whatever it names, fails the first assert.
    #[test]
    fn an_update_reaches_the_model_of_the_identity_it_names() {
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        side.apply(ToView::Update(
            other(),
            Box::new(Update::Received(arrival())),
        ))
        .expect("applied");
        assert!(
            model(&side).conversations().is_empty(),
            "another identity's arrival is not this model's"
        );
        side.apply(ToView::Update(me(), Box::new(Update::Received(arrival()))))
            .expect("applied");
        assert_eq!(model(&side).conversations().len(), 1, "its own is");
    }

    /// A view attached late -- or a window made again -- hears the held
    /// identities before the start, and names the model the start makes
    /// before its first update, which then applies. Mutations: the model a
    /// start makes left unnamed, or `Held` not kept for it, drops the
    /// update and fails the last assert.
    #[test]
    fn a_view_attached_late_names_the_held_before_its_first_update() {
        let mut side = ViewSide::new(|| ModelSide::new(Blank, Blank));
        assert_eq!(side.side().front(), None, "nothing said yet");
        side.apply(ToView::Held(held())).expect("applied");
        side.apply(ToView::Start(1)).expect("applied");
        side.apply(ToView::Running(true)).expect("applied");
        assert_eq!(side.side().front(), Some(&me()));
        side.apply(ToView::Update(me(), Box::new(Update::Received(arrival()))))
            .expect("applied");
        assert_eq!(model(&side).conversations().len(), 1, "applied");
    }

    /// An identity held while the service runs is given a model of its
    /// own by `HeldMore`, before its first update, which then applies to
    /// it and to no other; and a model made again is named with it too.
    /// Mutations: `HeldMore` not reaching `ModelSide::hold_more` -- the
    /// update is dropped and the first assert fails; `held` not grown --
    /// the model made again lacks it and the last assert fails.
    #[test]
    fn an_identity_held_later_gets_a_model_before_its_first_update() {
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        assert!(side.side().model_of(&other()).is_none(), "not held yet");
        side.apply(ToView::HeldMore(vec![other()]))
            .expect("applied");
        side.apply(ToView::Update(
            other(),
            Box::new(Update::Received(arrival())),
        ))
        .expect("applied");
        let made = side.side().model_of(&other()).expect("a model of its own");
        assert_eq!(made.conversations().len(), 1, "its update applied to it");
        assert!(model(&side).conversations().is_empty(), "and to no other");
        assert_eq!(side.side().front(), Some(&me()), "the front unchanged");
        side.apply(ToView::Running(true)).expect("applied");
        assert!(
            side.side().model_of(&other()).is_some(),
            "a model made again names it"
        );
    }

    /// What an add came to reaches the log as a class, in order, naming
    /// no identity, alias or card; and what waits is bounded. Mutation:
    /// no bound -- the length assert fails.
    #[test]
    fn add_classes_name_no_id_alias_or_card() {
        use interweave_human_app_core::CardSaved;
        use interweave_human_transport_client::AddOutcome;
        let made = other();
        let told = [
            AddTold::Offered,
            AddTold::Ended {
                outcome: AddOutcome::Created {
                    endpoint: made.clone(),
                    alias: Some("zelinda".to_owned()),
                },
                card: Some(CardSaved::Saved),
            },
            AddTold::Held(made.clone()),
            AddTold::NotHeld(made.clone()),
            AddTold::NotHeldUntilRestart(made.clone()),
        ];
        let mut side = view_side();
        for told in told.clone() {
            side.apply(ToView::Add(told)).expect("applied");
        }
        let classes = side.take_add_classes();
        assert_eq!(
            classes,
            [
                "create offered",
                "created, card saved",
                "an identity is held from now on",
                "an identity is not held yet, tried again",
                "an identity is not held, not tried again until restart",
            ]
        );
        for class in &classes {
            assert!(!class.contains(made.as_str()), "{class}");
            assert!(!class.contains("zelinda"), "{class}");
        }
        assert!(side.take_add_classes().is_empty(), "taken once");
        for _ in 0..ADD_CLASS_CAP + 5 {
            side.apply(ToView::Add(AddTold::NotStarted))
                .expect("applied");
        }
        assert_eq!(side.take_add_classes().len(), ADD_CLASS_CAP, "bounded");
    }

    /// The add act goes to the hub as it was given, in order with the
    /// commands, and the view side says false once no facade runs.
    /// Mutation: `add` answering true whatever `send` answers -- the last
    /// assert fails.
    #[test]
    fn an_add_act_goes_to_the_hub_and_is_refused_once_nothing_runs() {
        let hub = crate::Hub::new();
        let (commands, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let generation = hub.started(commands, held());
        let mut side = view_side();
        assert!(side.add(AddAct::Opened, |act| hub.add(act)));
        assert_eq!(
            rx.try_recv().ok(),
            Some(crate::ToFacade::Add(AddAct::Opened))
        );
        hub.retire(generation);
        assert!(!side.add(AddAct::Left, |act| hub.add(act)), "nothing runs");
    }

    /// Every model made again at a change of the running state is named
    /// again with what the hub last said is held. Mutation: `made` not
    /// naming its model fails the first assert.
    #[test]
    fn a_model_made_again_is_named_again() {
        let mut side = view_side();
        for running in [true, false, true] {
            side.apply(ToView::Running(running)).expect("applied");
            assert_eq!(side.side().front(), Some(&me()), "running {running}");
        }
        side.apply(ToView::Update(me(), Box::new(Update::Received(arrival()))))
            .expect("applied");
        assert_eq!(model(&side).conversations().len(), 1, "and applies");
    }

    /// The switch crosses to the hub as the desktop's does, the switch
    /// naming the new front; with one identity held it cannot happen and
    /// sends nothing. Mutation: `switch` not handing
    /// the model side's crossings on fails the `ToFacade::Front(other())`
    /// assert.
    #[test]
    fn a_switch_crosses_to_the_hub_and_with_one_held_none_can() {
        let hub = crate::Hub::new();
        let (commands, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = hub.started(commands, held());
        let cross = |crossing| {
            hub.cross(crossing).expect("a facade takes it");
            Ok(())
        };

        let mut one = view_side();
        assert!(!one.switch(&me(), cross), "in front already");
        assert!(!one.switch(&other(), cross), "not held");
        assert!(rx.try_recv().is_err(), "nothing crossed");

        let mut two = ViewSide::new(|| ModelSide::new(Blank, Blank));
        two.apply(ToView::Held(Held {
            held: vec![me(), other()],
            front: me(),
        }))
        .expect("applied");
        assert!(two.switch(&other(), cross), "two held: switched");
        assert_eq!(
            rx.try_recv().ok(),
            Some(crate::ToFacade::Front(other())),
            "the switch crossed"
        );
        assert_eq!(two.side().front(), Some(&other()));
        // `switch` names the new front for the model made next too, as
        // `take_identity_acts` does. Mutation: `switch` not recording it
        // -- the model made at the stop is named with `me`.
        two.apply(ToView::Running(false)).expect("applied");
        assert_eq!(two.side().front(), Some(&other()), "named with the new");

        // A switch no facade takes names no model made later with it
        // (round 2 F7). Mutation: `switch` recording the front whatever
        // `cross` answers -- the model made at the stop is named `other`.
        let stopped = crate::Hub::new();
        let mut three = ViewSide::new(|| ModelSide::new(Blank, Blank));
        three
            .apply(ToView::Held(Held {
                held: vec![me(), other()],
                front: me(),
            }))
            .expect("applied");
        assert!(
            three.switch(&other(), |c| stopped.cross(c)),
            "switched here"
        );
        three.apply(ToView::Running(false)).expect("applied");
        assert_eq!(three.side().front(), Some(&me()), "handed back: as it was");
    }

    #[test]
    fn a_service_that_stops_leaves_no_ready_session_and_no_rows() {
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        side.apply(ToView::Update(
            me(),
            Box::new(Update::Client(ClientEvent::Session(SessionState::Ready {
                endpoint: None,
            }))),
        ))
        .expect("applied");
        side.apply(ToView::Update(me(), Box::new(Update::Received(arrival()))))
            .expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            None,
            "the control: Ready, no notice"
        );
        assert_eq!(
            model(&side).conversations().len(),
            1,
            "the control: a row shown"
        );

        side.apply(ToView::Running(false)).expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::NoDaemon),
            "the window says the service is not running"
        );
        assert!(model(&side).conversations().is_empty(), "and shows no row");
    }

    /// A stop the lock follows keeps the list (unlock note §11): the
    /// stopped model comes back once the lock is said while nothing runs,
    /// reading locked and offline, with nothing Ready; with no lock, the
    /// stop shows none (the control, review F3).
    #[test]
    fn a_stop_the_lock_follows_keeps_the_rows() {
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        side.apply(ToView::Update(
            me(),
            Box::new(Update::Client(ClientEvent::Session(SessionState::Ready {
                endpoint: None,
            }))),
        ))
        .expect("applied");
        side.apply(ToView::Update(me(), Box::new(Update::Received(arrival()))))
            .expect("applied");
        side.apply(ToView::Running(false)).expect("applied");
        assert!(
            model(&side).conversations().is_empty(),
            "control: no lock, no row"
        );

        side.apply(ToView::AuthenticationNeeded(true))
            .expect("applied");
        assert_eq!(model(&side).conversations().len(), 1, "the row is back");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::AuthenticationNeeded)
        );
        side.apply(ToView::AuthenticationNeeded(false))
            .expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::Reconnecting),
            "and no Ready session once the lock is gone"
        );
    }

    /// The lock lifted while nothing runs is a start on its way: the
    /// session reads re-opening, never "not running", until the start
    /// answers -- refused, it reads not running again; opened, its service
    /// says it runs (j98 A17 finding 3: one frame of "not running" between
    /// the grant and the start). Mutations: the lift leaving the stopped
    /// model's "not running"; a refusal leaving it re-opening.
    #[test]
    fn a_lifted_lock_reads_re_opening_until_the_start_answers() {
        let locked = || {
            let mut side = view_side();
            side.apply(ToView::Running(true)).expect("applied");
            side.apply(ToView::Running(false)).expect("applied");
            side.apply(ToView::Store(
                StoreAnswer::Locked,
                StoreGeneration::default(),
            ))
            .expect("applied");
            side.apply(ToView::AuthenticationNeeded(true))
                .expect("applied");
            side.apply(ToView::AuthenticationNeeded(false))
                .expect("applied");
            assert_eq!(
                model(&side).session_notice(),
                Some(SessionNotice::Reconnecting),
                "a start on its way"
            );
            side
        };

        let mut side = locked();
        side.apply(ToView::Store(
            StoreAnswer::OtherRefusal,
            StoreGeneration::default(),
        ))
        .expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::NoDaemon),
            "refused: nothing runs"
        );

        let mut side = locked();
        side.apply(ToView::Store(
            StoreAnswer::Opened,
            StoreGeneration::default(),
        ))
        .expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::Reconnecting),
            "opened: its service comes"
        );

        // The control: a view attached while nothing runs and nothing is
        // locked reads not running, since no start was asked.
        let mut side = view_side();
        side.apply(ToView::Running(false)).expect("applied");
        side.apply(ToView::AuthenticationNeeded(false))
            .expect("applied");
        assert_eq!(model(&side).session_notice(), Some(SessionNotice::NoDaemon));
    }

    /// A start the store answered between the stop and the lock drops the
    /// set-aside rows: after a refusal or a discard they are not the list
    /// as it was. The lock's own answer keeps them.
    #[test]
    fn a_store_answer_between_the_stop_and_the_lock_drops_the_rows() {
        let stopped_with_a_row = || {
            let mut side = view_side();
            side.apply(ToView::Running(true)).expect("applied");
            side.apply(ToView::Update(me(), Box::new(Update::Received(arrival()))))
                .expect("applied");
            side.apply(ToView::Running(false)).expect("applied");
            side
        };

        for answer in [
            StoreAnswer::Refused(crate::hub::StoreRefusal::Recovery),
            StoreAnswer::OtherRefusal,
        ] {
            let mut side = stopped_with_a_row();
            side.apply(ToView::Store(answer, StoreGeneration::default()))
                .expect("applied");
            side.apply(ToView::AuthenticationNeeded(true))
                .expect("applied");
            assert!(
                model(&side).conversations().is_empty(),
                "no row after {answer:?}"
            );
        }

        let mut side = stopped_with_a_row();
        side.apply(ToView::Store(
            StoreAnswer::Locked,
            StoreGeneration::default(),
        ))
        .expect("applied");
        side.apply(ToView::AuthenticationNeeded(true))
            .expect("applied");
        assert_eq!(
            model(&side).conversations().len(),
            1,
            "control: the lock keeps it"
        );
    }

    /// A lock said before the stop shows the stopped model at once.
    #[test]
    fn a_stop_while_locked_keeps_the_rows() {
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        side.apply(ToView::Update(me(), Box::new(Update::Received(arrival()))))
            .expect("applied");
        side.apply(ToView::AuthenticationNeeded(true))
            .expect("applied");
        side.apply(ToView::Running(false)).expect("applied");
        assert_eq!(model(&side).conversations().len(), 1);
    }

    /// What is written waits: a draft carries into the model the next
    /// start makes, from the model set aside or the one shown (A17).
    #[test]
    fn a_draft_carries_into_the_next_start() {
        use interweave_human_ui_model::ConversationKey;
        let key = ConversationKey::Channel(
            interweave_transport_api::ChannelId::parse("general").expect("a channel"),
        );
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        // As the person's edit sets it.
        side.side_mut()
            .carry_held_drafts(vec![(me(), key.clone(), "half written".to_owned())]);
        side.apply(ToView::Running(false)).expect("applied");
        side.apply(ToView::Running(true)).expect("applied");
        assert_eq!(model(&side).composer(&key).draft, "half written");
    }

    type Presses = std::rc::Rc<std::cell::RefCell<Vec<IdentityIntent>>>;
    type TwoHeld = (
        ViewSide<Pressing, Blank>,
        crate::Hub,
        u64,
        Presses,
        tokio::sync::mpsc::UnboundedReceiver<crate::hub::ToFacade>,
    );

    /// A view side over two held identities, a hub whose facade runs
    /// (its generation, to retire), and the identity presses the test
    /// scripts.
    fn two_held() -> TwoHeld {
        let hub = crate::Hub::new();
        let (commands, rx) = tokio::sync::mpsc::unbounded_channel();
        let both = Held {
            held: vec![me(), other()],
            front: me(),
        };
        let generation = hub.started(commands, both.clone());
        let intents = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let pressed = std::rc::Rc::clone(&intents);
        let mut side = ViewSide::new(move || {
            ModelSide::new(
                Pressing {
                    events: Vec::new(),
                    intents: std::rc::Rc::clone(&pressed),
                },
                Blank,
            )
        });
        side.apply(ToView::Held(both)).expect("applied");
        (side, hub, generation, intents, rx)
    }

    fn choose(id: &EndpointId) -> [IdentityIntent; 2] {
        [
            IdentityIntent::OpenSwitcher,
            IdentityIntent::Choose(interweave_human_ui_model::IdentityKey::new(id.as_str())),
        ]
    }

    fn general() -> interweave_human_ui_model::ConversationKey {
        interweave_human_ui_model::ConversationKey::Channel(
            interweave_transport_api::ChannelId::parse("general").expect("a channel"),
        )
    }

    /// What is written waits across a restart for every identity held,
    /// each draft in its own identity's model, whichever is in front.
    /// Mutations run, each failing it: `ModelSide::held_drafts` over the
    /// front's model alone -- the draft of the identity not in front is
    /// gone; `ModelSide::carry_held_drafts` writing every draft into the
    /// front's model, as a carry keyed by conversation only does -- the
    /// first identity's model reads empty.
    #[test]
    fn a_restart_keeps_the_drafts_of_every_held_identity() {
        let (mut side, hub, _generation, intents, _rx) = two_held();
        side.apply(ToView::Running(true)).expect("applied");
        let told = || rows(&[(me(), "mine"), (other(), "work")]);
        side.apply(ToView::Identities(told())).expect("applied");
        side.side_mut()
            .carry_held_drafts(vec![(me(), general(), "of me".to_owned())]);
        intents.borrow_mut().extend(choose(&other()));
        assert!(!side.take_identity_acts(|c| hub.cross(c)), "taken");
        side.side_mut()
            .carry_held_drafts(vec![(other(), general(), "of other".to_owned())]);

        side.apply(ToView::Running(false)).expect("applied");
        side.apply(ToView::Running(true)).expect("applied");
        side.apply(ToView::Identities(told())).expect("applied");
        intents.borrow_mut().extend(choose(&me()));
        assert!(!side.take_identity_acts(|c| hub.cross(c)), "taken back");
        let draft = |e: &EndpointId| {
            side.side()
                .model_of(e)
                .expect("held")
                .composer(&general())
                .draft
        };
        assert_eq!(draft(&me()), "of me");
        assert_eq!(draft(&other()), "of other");
    }

    /// A switch taken locally while the kept model is shown under the
    /// lock, handed back by the hub (no facade runs), leaves the next
    /// start naming the old front; the drafts still land in the identity
    /// that wrote them, never in the one named. Mutations run, each
    /// failing it: `ModelSide::carry_held_drafts` writing every draft
    /// into the front's model (keyed by conversation only) -- the first
    /// identity's model reads the other's draft; `ModelSide::held_drafts`
    /// over the front's model alone -- the first identity's draft is
    /// gone.
    #[test]
    fn a_draft_stays_with_its_identity_across_a_switch_under_the_lock() {
        let (mut side, hub, generation, intents, rx) = two_held();
        side.apply(ToView::Running(true)).expect("applied");
        let told = || rows(&[(me(), "mine"), (other(), "work")]);
        side.side_mut().carry_held_drafts(vec![
            (me(), general(), "of me".to_owned()),
            (other(), general(), "of other".to_owned()),
        ]);
        side.apply(ToView::Running(false)).expect("applied");
        side.apply(ToView::AuthenticationNeeded(true))
            .expect("applied");
        assert_eq!(side.side().front(), Some(&me()), "the kept model shown");

        // The facade is gone: the hub hands the front's crossing back.
        drop(rx);
        hub.retire(generation);
        side.apply(ToView::Identities(told())).expect("applied");
        intents.borrow_mut().extend(choose(&other()));
        assert!(side.take_identity_acts(|c| hub.cross(c)), "handed back");
        assert_eq!(side.side().front(), Some(&other()), "applied locally");

        side.apply(ToView::Running(true)).expect("applied");
        assert_eq!(side.side().front(), Some(&me()), "named as before");
        let draft = |e: &EndpointId| {
            side.side()
                .model_of(e)
                .expect("held")
                .composer(&general())
                .draft
        };
        assert_eq!(draft(&me()), "of me");
        assert_eq!(draft(&other()), "of other");
    }

    /// A view whose events and identity presses a test scripts, any time:
    /// each press taken once, by the model side's turn and its identity
    /// acts.
    struct Scripted {
        events: std::rc::Rc<std::cell::RefCell<Vec<ViewEvent>>>,
        intents: std::rc::Rc<std::cell::RefCell<Vec<IdentityIntent>>>,
    }

    impl Surface for Scripted {
        fn render(&mut self, _model: &UiModel) {}

        fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            std::mem::take(&mut *self.events.borrow_mut())
        }

        fn take_identity_intents(&mut self) -> Vec<IdentityIntent> {
            std::mem::take(&mut *self.intents.borrow_mut())
        }
    }

    /// With two identities held, the model brought in front by a switch
    /// -- running, and under the lock with nothing to take the switch --
    /// is that identity's own: its session and the sends it has in flight
    /// are its own, never the one switched from. The first is Ready with
    /// a send in flight, the other neither; a second press of Send is
    /// swallowed only by the identity whose send is in flight. Under the
    /// lock the first, which was in front at the stop, is brought back
    /// with its send failed and no Ready session.
    /// Mutations run, each failing it: `ModelSide::exchange` not
    /// swapping `in_flight` -- the first identity's send is swallowed
    /// for the other; not swapping the model -- the other reads the
    /// first's Ready session.
    #[test]
    fn a_switch_brings_in_the_identitys_own_session_and_sends_in_flight() {
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let intents = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let hub = crate::Hub::new();
        let (commands, rx) = tokio::sync::mpsc::unbounded_channel();
        let both = Held {
            held: vec![me(), other()],
            front: me(),
        };
        let generation = hub.started(commands, both.clone());
        let (e, i) = (std::rc::Rc::clone(&events), std::rc::Rc::clone(&intents));
        let mut side = ViewSide::new(move || {
            ModelSide::new(
                Scripted {
                    events: std::rc::Rc::clone(&e),
                    intents: std::rc::Rc::clone(&i),
                },
                Blank,
            )
        });
        side.apply(ToView::Held(both)).expect("applied");
        side.apply(ToView::Running(true)).expect("applied");
        let told = || rows(&[(me(), "mine"), (other(), "work")]);
        side.apply(ToView::Identities(told())).expect("applied");
        side.apply(ToView::Update(
            me(),
            Box::new(Update::Client(ClientEvent::Session(SessionState::Ready {
                endpoint: None,
            }))),
        ))
        .expect("applied");
        // One Send pressed as the identity in front; a press of the same
        // while it waits is not sent again.
        let press = |side: &mut ViewSide<Scripted, Blank>| {
            events
                .borrow_mut()
                .push(ViewEvent::Intent(interweave_human_ui_model::Intent::Send {
                    key: general(),
                    draft: "hi".to_owned(),
                }));
            side.side_mut()
                .turn()
                .iter()
                .filter(|c| matches!(c, Command::Send { .. }))
                .count()
        };
        let own = |side: &ViewSide<Scripted, Blank>, ready: bool| {
            assert_eq!(
                side.side().model().session_notice().is_none(),
                ready,
                "the session of the model in front"
            );
        };
        own(&side, true);
        assert_eq!(press(&mut side), 1, "sent");
        assert_eq!(press(&mut side), 0, "its send is in flight");

        // The other identity's own: not Ready, and no send of its own
        // waiting, whatever the first has.
        intents.borrow_mut().extend(choose(&other()));
        assert!(!side.take_identity_acts(|c| hub.cross(c)), "taken");
        assert_eq!(side.side().front(), Some(&other()));
        own(&side, false);
        assert_eq!(press(&mut side), 1, "not swallowed by the first's send");
        assert_eq!(press(&mut side), 0, "its own is in flight now");

        // And back: the first's own, Ready with its send still waiting.
        intents.borrow_mut().extend(choose(&me()));
        assert!(!side.take_identity_acts(|c| hub.cross(c)), "taken");
        own(&side, true);
        assert_eq!(press(&mut side), 0, "its own send is still in flight");

        // Under the lock, nothing to take the switch: the other is
        // brought in front all the same, and back again, each applied
        // locally. The stop failed the first's send and left it reading
        // nothing Ready, which is what the first brings back; the stop's
        // effect on an identity that was not in front is not held here.
        drop(rx);
        hub.retire(generation);
        side.apply(ToView::Running(false)).expect("applied");
        side.apply(ToView::AuthenticationNeeded(true))
            .expect("applied");
        side.apply(ToView::Identities(told())).expect("applied");
        assert_eq!(side.side().front(), Some(&me()), "the kept model shown");
        intents.borrow_mut().extend(choose(&other()));
        assert!(side.take_identity_acts(|c| hub.cross(c)), "handed back");
        assert_eq!(side.side().front(), Some(&other()), "applied locally");
        intents.borrow_mut().extend(choose(&me()));
        assert!(side.take_identity_acts(|c| hub.cross(c)), "handed back");
        assert_eq!(side.side().front(), Some(&me()), "and back");
        side.apply(ToView::AuthenticationNeeded(false))
            .expect("applied");
        own(&side, false);
        assert_eq!(press(&mut side), 1, "the stop failed its send in flight");
    }

    #[test]
    fn a_service_that_starts_again_starts_from_a_fresh_model() {
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        side.apply(ToView::Update(me(), Box::new(Update::Received(arrival()))))
            .expect("applied");
        side.apply(ToView::Running(true)).expect("applied");
        assert!(
            model(&side).conversations().is_empty(),
            "nothing carries over"
        );
    }

    #[test]
    fn withheld_network_access_outlives_a_fresh_model() {
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::Reconnecting),
            "the control: access held"
        );
        side.apply(ToView::NetworkAccess(false)).expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::NetworkDenied)
        );
        side.apply(ToView::Running(true)).expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::NetworkDenied),
            "a fresh model still knows access is withheld"
        );
        side.apply(ToView::NetworkAccess(true)).expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::Reconnecting)
        );
    }

    /// A restart refused for the person after the client ran shows the
    /// model the stop set aside, with the unlock facts that restart
    /// learnt, said before the lock: the locked P-up line, not the
    /// running client's none (unlock note A13(b); #610 review F1).
    #[test]
    fn the_kept_model_shown_at_a_lock_takes_the_last_unlock_facts() {
        use interweave_human_ui_model::{KeyUnlock, UnlockFacts, UnlockLine};
        let facts = |configured| {
            Some(UnlockFacts {
                in_force: KeyUnlock::BackgroundCompatible,
                configured: Some(configured),
                pending: None,
                restart_needs_unlock: Some(false),
            })
        };
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        side.apply(ToView::Unlock(crate::hub::UnlockSaid {
            facts: facts(KeyUnlock::BackgroundCompatible),
            start: 1,
            seen: false,
        }))
        .expect("applied");
        assert!(model(&side).unlock_lines().is_empty(), "the control: N");
        side.apply(ToView::Running(false)).expect("applied");
        side.apply(ToView::Unlock(crate::hub::UnlockSaid {
            facts: facts(KeyUnlock::UserPresence),
            start: 1,
            seen: false,
        }))
        .expect("applied");
        side.apply(ToView::AuthenticationNeeded(true))
            .expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::AuthenticationNeeded),
            "the kept model, locked"
        );
        assert!(
            matches!(
                model(&side).unlock_lines()[..],
                [UnlockLine::Pending { .. }]
            ),
            "{:?}",
            model(&side).unlock_lines()
        );
    }

    /// The person's authentication, while needed, outlives a fresh model
    /// as access does; the notice is the one that says so, never the
    /// "will connect" of no service, and the line reads offline (#309
    /// acceptance 10, conditions 1 and 3).
    #[test]
    fn a_needed_authentication_outlives_a_fresh_model_and_reads_offline() {
        use interweave_human_client_api::Connectivity;
        let mut side = view_side();
        side.apply(ToView::Running(false)).expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::NoDaemon),
            "the control: no service, the notice that promises a connection"
        );
        side.apply(ToView::AuthenticationNeeded(true))
            .expect("applied");
        side.apply(ToView::Running(false)).expect("applied");
        assert_eq!(
            model(&side).session_notice(),
            Some(SessionNotice::AuthenticationNeeded),
            "a fresh model still knows"
        );
        assert_eq!(model(&side).connectivity(), Connectivity::Offline);
        side.apply(ToView::AuthenticationNeeded(false))
            .expect("applied");
        assert_ne!(
            model(&side).session_notice(),
            Some(SessionNotice::AuthenticationNeeded)
        );
    }

    #[test]
    fn a_listing_that_failed_is_said_to_the_root() {
        let mut side = view_side();
        assert_eq!(side.apply(ToView::ListingFailed), Err(ListingFailed));
    }

    /// The held start decides on its own answers only (add-a-peer §5.1):
    /// a refusal said for it, or the service running with nothing owed or
    /// the reads settled. An earlier start's answer heard again at the
    /// attach, and a running service whose owed reads are unanswered,
    /// decide nothing. Mutation: dropping the generation's comparison, or
    /// the phase's, fails here.
    #[test]
    fn a_held_start_decides_on_its_own_answers_only() {
        use interweave_human_client_api::{AddressList, TrustList};

        use crate::hub::{StoreGeneration, StoreRefusal};

        let asked_at = StoreGeneration::default();
        let earlier = (StoreAnswer::Refused(StoreRefusal::Recovery), asked_at);

        // Nothing owed: the start's success decides.
        let mut not_owed = view_side();
        not_owed
            .apply(ToView::Store(earlier.0, earlier.1))
            .expect("applied");
        assert!(!not_owed.start_decided(asked_at), "an earlier answer");
        not_owed.apply(ToView::Running(true)).expect("applied");
        assert!(not_owed.start_decided(asked_at), "running, nothing owed");

        // Owed: running is not enough; the reads decide.
        let mut owed = ViewSide::new(|| {
            let mut side = ModelSide::new(Blank, Blank);
            side.connect_owed();
            side
        });
        owed.apply(ToView::Held(held())).expect("applied");
        owed.apply(ToView::Running(true)).expect("applied");
        assert!(!owed.start_decided(asked_at), "owed, the reads unanswered");
        owed.side_mut().apply(Update::TrustRead(Ok(TrustList {
            local_peer: None,
            allowed: Vec::new(),
        })));
        owed.side_mut().apply(Update::AddressesRead(Ok(AddressList {
            addresses: Vec::new(),
        })));
        let _ = owed.side_mut().turn();
        assert!(owed.start_decided(asked_at), "the reads settled it");

        // Refused, for the store or for the person: decided, not running.
        let mut refused = view_side();
        // A start answered after the ask: the hub counts it past `asked_at`.
        let hub = crate::Hub::new();
        hub.set_store_answer(StoreAnswer::Opened);
        let later = hub.store_generation();
        assert!(later > asked_at, "control");
        refused
            .apply(ToView::Store(StoreAnswer::OtherRefusal, later))
            .expect("applied");
        assert!(refused.start_decided(asked_at));
        let mut opened = view_side();
        opened
            .apply(ToView::Store(StoreAnswer::Opened, later))
            .expect("applied");
        assert!(!opened.start_decided(asked_at), "opened: the service says");
        let mut locked = view_side();
        locked
            .apply(ToView::AuthenticationNeeded(true))
            .expect("applied");
        assert!(locked.start_decided(asked_at), "locked: Chats' notice");
    }

    /// A held start that ran and then stopped has ended (add-a-peer §5.1,
    /// foreground-only Home), still so once the next start runs; one that
    /// runs, one only answered, one that never ran (refused), and a stop
    /// of an earlier start heard before the ask, have not. Mutation:
    /// recording nothing at the stop, or dropping the generation's
    /// comparison, fails here.
    #[test]
    fn a_held_start_that_ran_and_stopped_has_ended() {
        let hub = crate::Hub::new();
        let before = hub.store_generation();
        hub.set_store_answer(StoreAnswer::Opened);
        let earlier = hub.store_generation();

        // An earlier start ran and stopped before the ask.
        let mut side = view_side();
        side.apply(ToView::Store(StoreAnswer::Opened, earlier))
            .expect("applied");
        side.apply(ToView::Running(true)).expect("applied");
        assert!(!side.start_ended(earlier), "running: not ended");
        side.apply(ToView::Running(false)).expect("applied");
        assert!(side.start_ended(before), "control: stopped past `before`");
        let asked_at = hub.store_generation();
        assert!(!side.start_ended(asked_at), "an earlier start's stop");

        // The held start answers: answered and running is not ended.
        hub.set_store_answer(StoreAnswer::Opened);
        let held = hub.store_generation();
        side.apply(ToView::Store(StoreAnswer::Opened, held))
            .expect("applied");
        assert!(!side.start_ended(asked_at), "answered, not yet running");
        side.apply(ToView::Running(true)).expect("applied");
        assert!(!side.start_ended(asked_at), "running");
        // Home on foreground-only: stopped, and the return's start runs
        // in the same pump.
        side.apply(ToView::Running(false)).expect("applied");
        assert!(side.start_ended(asked_at), "ran and stopped");
        side.apply(ToView::Start(2)).expect("applied");
        side.apply(ToView::Running(true)).expect("applied");
        assert!(side.start_ended(asked_at), "still, once the next runs");

        // Refused: never ran, so never ended.
        let mut refused = view_side();
        hub.set_store_answer(StoreAnswer::OtherRefusal);
        refused
            .apply(ToView::Store(
                StoreAnswer::OtherRefusal,
                hub.store_generation(),
            ))
            .expect("applied");
        refused.apply(ToView::Running(false)).expect("applied");
        assert!(!refused.start_ended(asked_at), "refused: not ended");
    }

    /// Drives `side` with what the hub sent since the last take: one turn
    /// of the root's pump. True when the hub detached the link.
    fn pump(side: &mut ViewSide<Blank, Blank>, link: &mut crate::ViewLink) -> bool {
        for message in link.take() {
            let _ = side.apply(message);
        }
        link.lost()
    }

    /// A side whose every model owes Connect, as the root's does under a
    /// held C4 or R4: the marker is written at the seal and the restore
    /// (`onboard.rs`), and each fresh model asks it (`activity.rs`).
    fn owed_side() -> ViewSide<Blank, Blank> {
        ViewSide::new(|| {
            let mut side = ModelSide::new(Blank, Blank);
            side.connect_owed();
            side
        })
    }

    /// The held screen decides the running start: not while the owed
    /// reads are unanswered, and once they settle it.
    fn held_screen_decides(side: &mut ViewSide<Blank, Blank>, asked_at: StoreGeneration) {
        use interweave_human_client_api::{AddressList, TrustList};

        assert!(!side.start_decided(asked_at), "owed, the reads unanswered");
        side.side_mut().apply(Update::TrustRead(Ok(TrustList {
            local_peer: None,
            allowed: Vec::new(),
        })));
        side.side_mut().apply(Update::AddressesRead(Ok(AddressList {
            addresses: Vec::new(),
        })));
        let _ = side.side_mut().turn();
        assert!(side.start_decided(asked_at), "the reads settled it");
    }

    fn attached(hub: &crate::Hub) -> crate::ViewLink {
        hub.attach(std::sync::Arc::new(|| {}))
    }

    /// The service's start, as `service.rs` orders it: the facade runs
    /// (`Start`, `Running(true)`), then the start's answer is counted.
    fn service_starts(hub: &crate::Hub) -> u64 {
        let (commands, _rx) = tokio::sync::mpsc::unbounded_channel();
        let generation = hub.started(commands, held());
        hub.set_store_answer(StoreAnswer::Opened);
        generation
    }

    /// Add-a-peer §5.1, the first case the build of #678 had not run: Home
    /// pressed after a held start was asked for and before the service
    /// exists. No facade runs before the return, so no running service
    /// stops and the view reads no stop: the start made at the return is
    /// not ended (`start_ended`) and is decided by the held screen
    /// (`start_decided`, over a model owing Connect as a held one does), as
    /// the note says. Mutation: `start_ended` read from any answer counted
    /// past `asked_at` rather than from a stop the view saw fails here, and
    /// so does `start_decided` dropping its Connect-phase term.
    #[test]
    fn home_before_the_service_exists_leaves_the_return_to_the_held_screen() {
        let hub = crate::Hub::new();
        let mut link = attached(&hub);
        let mut side = owed_side();
        assert!(!pump(&mut side, &mut link));
        let asked_at = hub.store_generation();

        // Home: the binding goes before the service is made, and nothing
        // reaches the hub. The return binds again; the service's start
        // runs and answers.
        assert!(
            !pump(&mut side, &mut link),
            "nothing said before the return"
        );
        assert!(!side.start_ended(asked_at), "no stop to read");
        let _ = service_starts(&hub);
        assert!(!pump(&mut side, &mut link));
        assert!(side.running(), "control: the return's start runs");
        assert!(
            !side.start_ended(asked_at),
            "the return's start is not ended"
        );
        // And the held screen decides it, as the note says.
        held_screen_decides(&mut side, asked_at);
    }

    /// The second case: the service's messages to the window fill the
    /// queue ([`crate::hub::TO_VIEW`]) while the held start runs, and it
    /// stops. The hub detaches the view, so the stop is never queued for
    /// it; the view that attaches again hears `Running(false)` as its
    /// first word, which records no stop. So the held start that ran and
    /// ended is NOT ended for the view (`start_ended` false), and the
    /// start made at the return is decided by the held screen -- unlike
    /// the same start read through a queue with room (the control).
    /// What the test shows, reported to architect-cto with the note's
    /// case; the rule is unchanged. Mutation: recording nothing at the
    /// stop fails the control; `start_ended` read from any answer counted
    /// past `asked_at` fails after the attach; `start_decided` dropping
    /// its Connect-phase term fails at the return.
    #[test]
    fn a_stop_lost_with_a_full_queue_leaves_the_return_to_the_held_screen() {
        use interweave_human_client_api::Diagnostics;

        // The control: room in the queue, the stop is read and the held
        // start has ended.
        let hub = crate::Hub::new();
        let mut link = attached(&hub);
        let mut side = owed_side();
        let asked_at = hub.store_generation();
        let generation = service_starts(&hub);
        assert!(!pump(&mut side, &mut link));
        hub.retire(generation);
        assert!(!pump(&mut side, &mut link));
        assert!(side.start_ended(asked_at), "control: the stop read");

        // The queue full while the held start runs.
        let hub = crate::Hub::new();
        let mut link = attached(&hub);
        let mut side = owed_side();
        let asked_at = hub.store_generation();
        let generation = service_starts(&hub);
        let relists = hub.wants_listing(generation).expect("a listing owed");
        hub.listed(
            generation,
            relists,
            [ToView::Update(
                me(),
                Box::new(Update::Diagnostics(Diagnostics::default())),
            )],
        );
        for _ in 0..crate::hub::TO_VIEW {
            hub.update(
                generation,
                &me(),
                Update::Diagnostics(Diagnostics::default()),
            );
        }
        hub.retire(generation);
        assert!(pump(&mut side, &mut link), "detached by the full queue");
        assert!(side.running(), "the last word it read: running");
        assert!(!side.start_ended(asked_at), "the stop never reached it");

        // Attached again, as the root does on a lost link: a fresh side.
        let mut link = attached(&hub);
        let mut side = owed_side();
        assert!(!pump(&mut side, &mut link));
        assert!(!side.running(), "control: told nothing runs");
        assert!(!side.start_ended(asked_at), "and still no stop recorded");

        // The return's start: decided by the held screen.
        let _ = service_starts(&hub);
        assert!(!pump(&mut side, &mut link));
        assert!(side.running());
        assert!(
            !side.start_ended(asked_at),
            "the return's start is not ended"
        );
        held_screen_decides(&mut side, asked_at);
    }

    /// The third case: the stop and the next start read in two turns
    /// rather than one. The first turn reads the stop alone and the held
    /// start has ended there, so the root's `held_ended` sends the screen
    /// to Chats at that turn; the second turn's start does not undo it.
    /// The one-pump case is `a_held_start_that_ran_and_stopped_has_ended`.
    /// Mutation: recording nothing at the stop fails here at the first
    /// turn.
    #[test]
    fn a_stop_and_the_next_start_read_in_two_turns_end_the_held_start_at_the_first() {
        let hub = crate::Hub::new();
        let mut link = attached(&hub);
        let mut side = owed_side();
        let asked_at = hub.store_generation();
        let generation = service_starts(&hub);
        assert!(!pump(&mut side, &mut link));
        assert!(!side.start_ended(asked_at), "control: running, not ended");

        // Turn one: Home on foreground-only, the stop alone.
        hub.retire(generation);
        assert!(!pump(&mut side, &mut link));
        assert!(!side.running());
        assert!(side.start_ended(asked_at), "ended at the first turn");

        // Turn two: the return's start.
        let next = service_starts(&hub);
        assert!(!pump(&mut side, &mut link));
        assert_eq!(side.start(), Some(next), "the return's start runs");
        assert!(side.start_ended(asked_at), "still ended");
    }

    /// `IdStartBusy` after the bound (identity note §6, A start refused
    /// with the profile in use after the bound; §11d Q12): a held
    /// screen's start refused with the profile in use is told apart from
    /// any other refused start by its answer -- this start's, counted past
    /// the ask, never an earlier one an attach hears again -- and Chats
    /// shows the notice, never `NoDaemon` and never the locked notice. The
    /// notice is the process's: a reattach (a foreground-only Home and
    /// return) shows it, and a model made again at a stop keeps it. A
    /// return's retry shows it with Try again disabled until that start
    /// answers; `LockHeld` again shows it enabled; any other answer, or a
    /// new launch's start, takes it away.
    ///
    /// Mutations: the hub not saying the notice at an attach, the fresh
    /// model at `Running` not given it, `start_busy` reading an earlier
    /// answer, or `start_asked(false)` keeping it, fails here.
    #[test]
    fn a_start_refused_busy_shows_its_notice_until_another_answer() {
        let busy = |side: &ViewSide<Blank, Blank>| model(side).session_notice();
        let hub = crate::Hub::new();
        hub.set_store_answer(StoreAnswer::ProfileBusy);
        // A held screen asks its start: an earlier busy answer is not its.
        hub.start_asked(false);
        let asked_at = hub.store_generation();
        let mut link = hub.attach(std::sync::Arc::new(|| {}));
        let mut side = view_side();
        assert!(!pump(&mut side, &mut link));
        assert!(!side.start_busy(asked_at), "an earlier start's answer");
        assert!(side.start_busy(StoreGeneration::default()), "control: read");
        assert_eq!(busy(&side), Some(SessionNotice::NoDaemon), "control");

        hub.set_store_answer(StoreAnswer::ProfileBusy);
        assert!(!pump(&mut side, &mut link));
        assert!(side.start_busy(asked_at), "this start's: refused busy");
        assert!(side.start_decided(asked_at), "and decided");
        assert_eq!(
            busy(&side),
            Some(SessionNotice::StartBusy(StartBusy::Shown)),
            "in NoDaemon's place, no locked notice"
        );

        // A foreground-only Home and return: attached again.
        let mut link = hub.attach(std::sync::Arc::new(|| {}));
        let mut side = view_side();
        assert!(!pump(&mut side, &mut link));
        assert_eq!(
            busy(&side),
            Some(SessionNotice::StartBusy(StartBusy::Shown))
        );
        // The return's retry: shown, Try again disabled, through a model
        // made again.
        hub.start_asked(true);
        assert!(!pump(&mut side, &mut link));
        side.apply(ToView::Running(false)).expect("applied");
        assert_eq!(
            busy(&side),
            Some(SessionNotice::StartBusy(StartBusy::Starting))
        );
        hub.set_store_answer(StoreAnswer::ProfileBusy);
        assert!(!pump(&mut side, &mut link));
        assert_eq!(
            busy(&side),
            Some(SessionNotice::StartBusy(StartBusy::Shown)),
            "LockHeld again: Try again enabled"
        );
        hub.set_store_answer(StoreAnswer::OtherRefusal);
        assert!(!pump(&mut side, &mut link));
        assert_eq!(busy(&side), Some(SessionNotice::NoDaemon), "another answer");

        // A new launch's start: its own answer, never this notice.
        hub.set_store_answer(StoreAnswer::ProfileBusy);
        assert!(!pump(&mut side, &mut link));
        hub.start_asked(false);
        assert!(!pump(&mut side, &mut link));
        assert_eq!(busy(&side), Some(SessionNotice::NoDaemon), "a launch");
        hub.start_asked(true);
        assert!(!pump(&mut side, &mut link));
        assert_eq!(
            busy(&side),
            Some(SessionNotice::NoDaemon),
            "a retry with no notice shows none"
        );
    }

    /// `Refused` after the bound (identity note §6, An answer after the
    /// bound): a start that compared with C4's `PeerId` and was refused
    /// shows it on Chats, in `IdStartBusy`'s place and never `NoDaemon`;
    /// the process keeps it, so a reattach and a model made again at a
    /// stop show it; its Try again's start shows it disabled until that
    /// start answers, refused the same way it shows enabled again
    /// (The comparison outlasts the bound), and any other answer or a
    /// new launch's start takes it away. It is still a refusal: the held
    /// screen's start is decided by it.
    ///
    /// Mutations: `set_compared_refusal` saying no notice, the hub not
    /// saying it at an attach, the fresh model at `Running` not given it,
    /// `start_asked(true)` not marking its start, or an ordinary answer
    /// keeping it, fails here.
    #[test]
    fn a_compared_start_refused_shows_refused_until_another_answer() {
        let notice = |side: &ViewSide<Blank, Blank>| model(side).session_notice();
        let hub = crate::Hub::new();
        let asked_at = hub.store_generation();
        let mut link = hub.attach(std::sync::Arc::new(|| {}));
        let mut side = view_side();
        assert!(!pump(&mut side, &mut link));
        hub.set_compared_refusal();
        assert!(!pump(&mut side, &mut link));
        assert!(side.start_decided(asked_at), "a refusal: decided");
        assert!(!side.start_busy(asked_at), "not the profile in use");
        assert_eq!(
            notice(&side),
            Some(SessionNotice::StartRefused(StartBusy::Shown)),
            "in NoDaemon's place"
        );

        // A foreground-only Home and return: attached again.
        let mut link = hub.attach(std::sync::Arc::new(|| {}));
        let mut side = view_side();
        assert!(!pump(&mut side, &mut link));
        assert_eq!(
            notice(&side),
            Some(SessionNotice::StartRefused(StartBusy::Shown))
        );
        // Its Try again: disabled through a model made again.
        hub.start_asked(true);
        assert!(!pump(&mut side, &mut link));
        side.apply(ToView::Running(false)).expect("applied");
        assert_eq!(
            notice(&side),
            Some(SessionNotice::StartRefused(StartBusy::Starting))
        );
        hub.set_compared_refusal();
        assert!(!pump(&mut side, &mut link));
        assert_eq!(
            notice(&side),
            Some(SessionNotice::StartRefused(StartBusy::Shown)),
            "refused the same again: Try again enabled"
        );
        hub.set_store_answer(StoreAnswer::ProfileBusy);
        assert!(!pump(&mut side, &mut link));
        assert_eq!(
            notice(&side),
            Some(SessionNotice::StartBusy(StartBusy::Shown)),
            "LockHeld: IdStartBusy in its place"
        );
        hub.set_compared_refusal();
        hub.set_store_answer(StoreAnswer::OtherRefusal);
        assert!(!pump(&mut side, &mut link));
        assert_eq!(
            notice(&side),
            Some(SessionNotice::NoDaemon),
            "another answer"
        );

        // A new launch's start: its own answer, never this notice.
        hub.set_compared_refusal();
        assert!(!pump(&mut side, &mut link));
        hub.start_asked(false);
        assert!(!pump(&mut side, &mut link));
        assert_eq!(notice(&side), Some(SessionNotice::NoDaemon), "a launch");
    }

    /// Rows for `ids`, the first first, each aliased by `names`.
    fn rows(ids: &[(EndpointId, &str)]) -> Vec<HeldIdentity> {
        ids.iter()
            .enumerate()
            .map(|(i, (id, name))| HeldIdentity {
                key: interweave_human_ui_model::IdentityKey::new(id.as_str()),
                card_name: None,
                alias: (*name).to_owned(),
                state: interweave_human_ui_model::IdentityState::Ready,
                is_first: i == 0,
                is_default: false,
                unread: 0,
                not_sent: false,
            })
            .collect()
    }

    fn held_rows(side: &ViewSide<Blank, Blank>) -> Vec<HeldIdentity> {
        model(side).identities().held().to_vec()
    }

    /// The identities' rows the hub tells reach the model shown, and
    /// every model made again while the service runs is told them again;
    /// a stop drops them, as the hub drops its rows, so a model made at
    /// the stop, and at the next start until the facade tells its own, is
    /// told none; a list the model refuses is kept for the root to log.
    /// Two identities are held. Mutations: `Identities` not told to the
    /// model shown -- the first assert; `made` not telling them -- the
    /// "made again" assert; the rows kept at a stop -- the "stopped"
    /// assert; the refusal dropped -- `take_held_refused` answers `None`.
    #[test]
    fn a_model_made_again_is_told_the_identities_again() {
        let mut side = ViewSide::new(|| ModelSide::new(Blank, Blank));
        side.apply(ToView::Held(Held {
            held: vec![me(), other()],
            front: me(),
        }))
        .expect("applied");
        let told = rows(&[(me(), "mine"), (other(), "work")]);
        side.apply(ToView::Identities(told.clone()))
            .expect("applied");
        assert_eq!(held_rows(&side), told, "told to the model shown");
        side.apply(ToView::Running(true)).expect("applied");
        assert_eq!(held_rows(&side), told, "made again (running)");
        side.apply(ToView::Running(false)).expect("applied");
        assert_eq!(held_rows(&side), [], "stopped: no facade holds rows");
        side.apply(ToView::Running(true)).expect("applied");
        assert_eq!(held_rows(&side), [], "started: the facade tells its own");
        side.apply(ToView::Identities(told.clone()))
            .expect("applied");
        assert_eq!(held_rows(&side), told, "told again");
        assert_eq!(side.take_held_refused(), None);
        let twice = rows(&[(me(), "mine"), (me(), "again")]);
        side.apply(ToView::Identities(twice)).expect("applied");
        assert_eq!(
            side.take_held_refused(),
            Some(HeldProblem::DuplicateKey),
            "kept for the log"
        );
        assert_eq!(side.take_held_refused(), None, "once");
        assert_eq!(held_rows(&side), told, "the model keeps what it held");
    }

    /// A view whose events and identity presses a test scripts, each
    /// taken once.
    struct Pressing {
        events: Vec<ViewEvent>,
        intents: std::rc::Rc<std::cell::RefCell<Vec<IdentityIntent>>>,
    }

    impl Surface for Pressing {
        fn render(&mut self, _model: &UiModel) {}

        fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
            std::mem::take(&mut self.events)
        }

        fn take_identity_intents(&mut self) -> Vec<IdentityIntent> {
            std::mem::take(&mut *self.intents.borrow_mut())
        }
    }

    /// A model made after a switch is named with the identity switched
    /// to, not the one in front at the start: the facade took the switch,
    /// so a stop's model, and the next start's, show it. A switch handed
    /// back -- no facade takes it -- leaves the naming as it was.
    /// Mutation: `take_identity_acts` not recording the front taken --
    /// the first assert names `me`.
    #[test]
    fn a_model_made_after_a_switch_is_named_with_the_new_front() {
        let hub = crate::Hub::new();
        let (commands, rx) = tokio::sync::mpsc::unbounded_channel();
        let both = Held {
            held: vec![me(), other()],
            front: me(),
        };
        let generation = hub.started(commands, both.clone());
        let intents = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let pressed = std::rc::Rc::clone(&intents);
        let mut side = ViewSide::new(move || {
            ModelSide::new(
                Pressing {
                    events: Vec::new(),
                    intents: std::rc::Rc::clone(&pressed),
                },
                Blank,
            )
        });
        side.apply(ToView::Held(both)).expect("applied");
        side.apply(ToView::Identities(rows(&[
            (me(), "mine"),
            (other(), "work"),
        ])))
        .expect("applied");
        let choose = |id: &EndpointId| {
            [
                IdentityIntent::OpenSwitcher,
                IdentityIntent::Choose(interweave_human_ui_model::IdentityKey::new(id.as_str())),
            ]
        };
        intents.borrow_mut().extend(choose(&other()));
        assert!(!side.take_identity_acts(|c| hub.cross(c)), "taken");
        side.apply(ToView::Running(false)).expect("applied");
        assert_eq!(side.side().front(), Some(&other()), "named with the new");

        // A facade that stopped holds no rows: the next start tells its own.
        hub.retire(generation);
        drop(rx);
        side.apply(ToView::Identities(rows(&[
            (me(), "mine"),
            (other(), "work"),
        ])))
        .expect("applied");
        intents.borrow_mut().extend(choose(&me()));
        assert!(side.take_identity_acts(|c| hub.cross(c)), "handed back");
        side.apply(ToView::Running(false)).expect("applied");
        assert_eq!(side.side().front(), Some(&other()), "a handed-back switch");
    }

    /// A choice in the switcher reaches the facade through the hub as the
    /// switch, after the command the old front's view had not yet asked,
    /// which crosses naming the old front; once the hub hands a crossing
    /// back -- no facade runs -- the take says so and crosses nothing
    /// more. Mutations: `take_identity_acts` not handing the model
    /// side's crossings to `cross` -- nothing reaches the facade; a
    /// handed-back crossing not reported -- the take answers false; a
    /// handed-back command not failed to its model -- no problem said.
    #[test]
    fn a_switch_intent_reaches_the_facade_after_the_old_fronts_commands() {
        let hub = crate::Hub::new();
        let (commands, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let generation = hub.started(
            commands,
            Held {
                held: vec![me(), other()],
                front: me(),
            },
        );
        let intents = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let pressed = std::rc::Rc::clone(&intents);
        let mut side = ViewSide::new(move || {
            ModelSide::new(
                Pressing {
                    events: vec![ViewEvent::Intent(interweave_human_ui_model::Intent::Reopen)],
                    intents: std::rc::Rc::clone(&pressed),
                },
                Blank,
            )
        });
        side.apply(ToView::Held(Held {
            held: vec![me(), other()],
            front: me(),
        }))
        .expect("applied");
        side.apply(ToView::Identities(rows(&[
            (me(), "mine"),
            (other(), "work"),
        ])))
        .expect("applied");
        intents.borrow_mut().push(IdentityIntent::OpenSwitcher);
        assert!(
            !side.take_identity_acts(|c| hub.cross(c)),
            "opening crosses nothing"
        );
        assert!(rx.try_recv().is_err(), "nothing crossed yet");
        intents.borrow_mut().push(IdentityIntent::Choose(
            interweave_human_ui_model::IdentityKey::new(other().as_str()),
        ));
        assert!(
            !side.take_identity_acts(|c| hub.cross(c)),
            "a facade took them"
        );
        assert_eq!(
            rx.try_recv().ok(),
            Some(crate::ToFacade::Command {
                identity: Some(me()),
                command: Command::Reopen
            }),
            "the old front's command first, naming it"
        );
        assert_eq!(rx.try_recv().ok(), Some(crate::ToFacade::Front(other())));
        assert!(rx.try_recv().is_err());
        assert_eq!(side.side().front(), Some(&other()));

        hub.retire(generation);
        intents.borrow_mut().extend([
            IdentityIntent::OpenSwitcher,
            IdentityIntent::Choose(interweave_human_ui_model::IdentityKey::new(me().as_str())),
        ]);
        assert!(
            side.take_identity_acts(|c| hub.cross(c)),
            "handed back: said"
        );
        assert!(rx.try_recv().is_err(), "nothing crossed after the stop");

        // A command no facade took is failed to its identity's model, not
        // left waiting for an answer.
        let _ = side.side_mut().take_problems();
        intents.borrow_mut().push(IdentityIntent::TryAgain(
            interweave_human_ui_model::IdentityKey::new(me().as_str()),
        ));
        assert!(side.take_identity_acts(|c| hub.cross(c)), "handed back");
        assert!(
            side.side_mut().take_problems().iter().any(|p| matches!(
                p,
                interweave_human_app_core::Problem::Command {
                    command: Command::Reopen,
                    why: Failure::ServiceStopped
                }
            )),
            "failed explicitly"
        );
    }
}
