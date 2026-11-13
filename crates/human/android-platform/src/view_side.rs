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

use interweave_human_app_core::{ModelSide, Opener, Surface};
use interweave_human_ui_model::ConnectPhase;

use crate::hub::{StoreAnswer, StoreGeneration, ToView, UnlockSaid};

/// The store could not be listed for this view: what it holds stays
/// unshown until the view attaches again. A class, never content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListingFailed;

/// The model side a view attached to the hub drives.
pub struct ViewSide<S: Surface, O: Opener> {
    side: ModelSide<S, O>,
    fresh: Box<dyn FnMut() -> ModelSide<S, O>>,
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
    /// The model the last stop set aside, stopped: shown again if the
    /// lock is said while nothing runs, its drafts carried by the next
    /// start.
    kept: Option<ModelSide<S, O>>,
}

impl<S: Surface, O: Opener> ViewSide<S, O> {
    /// `fresh` makes a model side over the view: called now, and again at
    /// every change of the service's running state.
    pub fn new(mut fresh: impl FnMut() -> ModelSide<S, O> + 'static) -> Self {
        Self {
            side: fresh(),
            fresh: Box::new(fresh),
            network: true,
            authentication: false,
            store: None,
            running: false,
            start: None,
            unlock: None,
            kept: None,
        }
    }

    /// The last start's answer as the store saw it (ADR-0060 decision 6),
    /// with the hub's count of answers when it was said, which a view
    /// attached again hears unchanged; `None` until a start has answered.
    #[must_use]
    pub const fn store_answer(&self) -> Option<(StoreAnswer, StoreGeneration)> {
        self.store
    }

    /// Apply one message from the hub.
    ///
    /// # Errors
    /// [`ListingFailed`] when the hub says the store could not be listed,
    /// for the root to report.
    pub fn apply(&mut self, message: ToView) -> Result<(), ListingFailed> {
        match message {
            ToView::Running(running) => {
                let last = std::mem::replace(&mut self.side, (self.fresh)());
                if running {
                    // The model set aside first, then the one shown since,
                    // whose edits are the later.
                    let mut drafts = self
                        .kept
                        .take()
                        .map(|kept| kept.drafts())
                        .unwrap_or_default();
                    drafts.extend(last.drafts());
                    self.side.carry_drafts(drafts);
                } else if self.running {
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
                self.say_unlock();
                if !running && self.authentication {
                    self.show_kept();
                }
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
            ToView::Update(update) => self.side.apply(*update),
            ToView::ListingFailed => return Err(ListingFailed),
            ToView::Unlock(said) => {
                self.unlock = Some(said);
                self.say_unlock();
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

    /// The model the last stop set aside, shown again: the restart is
    /// refused for the person, and the list and an open conversation stay
    /// as they were (unlock note §11).
    fn show_kept(&mut self) {
        if let Some(mut kept) = self.kept.take() {
            kept.network_access(self.network);
            kept.authentication_needed(self.authentication);
            kept.carry_drafts(self.side.drafts());
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
    use interweave_human_ui_model::{SessionNotice, UiModel, ViewEvent};
    use interweave_profile_identity::ProfileIdentity;
    use interweave_transport_api::EndpointId;

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

    fn view_side() -> ViewSide<Blank, Blank> {
        ViewSide::new(|| ModelSide::new(Blank, Blank))
    }

    fn arrival() -> Received {
        let peer = ProfileIdentity::generate()
            .transport_identity()
            .expect("a peer id");
        Received {
            row: RowId::from_stored(1),
            origin: Origin::Direct {
                peer,
                endpoint: EndpointId::parse("human").expect("an endpoint"),
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

    #[test]
    fn a_service_that_stops_leaves_no_ready_session_and_no_rows() {
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        side.apply(ToView::Update(Box::new(Update::Client(
            ClientEvent::Session(SessionState::Ready { endpoint: None }),
        ))))
        .expect("applied");
        side.apply(ToView::Update(Box::new(Update::Received(arrival()))))
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
        side.apply(ToView::Update(Box::new(Update::Client(
            ClientEvent::Session(SessionState::Ready { endpoint: None }),
        ))))
        .expect("applied");
        side.apply(ToView::Update(Box::new(Update::Received(arrival()))))
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
            side.apply(ToView::Update(Box::new(Update::Received(arrival()))))
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
        side.apply(ToView::Update(Box::new(Update::Received(arrival()))))
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
            .carry_drafts(vec![(key.clone(), "half written".to_owned())]);
        side.apply(ToView::Running(false)).expect("applied");
        side.apply(ToView::Running(true)).expect("applied");
        assert_eq!(model(&side).composer(&key).draft, "half written");
    }

    #[test]
    fn a_service_that_starts_again_starts_from_a_fresh_model() {
        let mut side = view_side();
        side.apply(ToView::Running(true)).expect("applied");
        side.apply(ToView::Update(Box::new(Update::Received(arrival()))))
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
}
