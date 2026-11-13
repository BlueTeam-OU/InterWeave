// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the Activity's root decides about a start with Connect owed
//! (add-a-peer section 5.1), kept here so the host tests reach it: the
//! root itself builds for Android only.
//!
//! Two rules. A held C4's or R4's bound that passes before its service
//! answered defers the model that service makes when it does answer, so
//! Connect does not open over Chats later that start -- unless that start
//! is refused, which leaves Connect owed at the next start. And a start
//! with Connect owed and no held screen -- a launch, or a start after a
//! refused one -- opens Connect over Chats only within [`BOUND_MS`] of the
//! service running and only if nothing was opened from Chats since the
//! count began: when Chats showed at this launch, or when a later start
//! in the same process was asked for. An opening made before the service
//! ran counts, and so does one closed again.

use interweave_human_app_core::{Command, Opener, Surface};
use interweave_human_ui_model::{BOUND_MS, ConnectPhase};

use crate::{StoreGeneration, ViewSide};

/// The root's state for those two rules, carried across a reattach.
#[derive(Debug, Default)]
pub struct StartRules {
    running: bool,
    /// The held start whose bound passed before its service answered:
    /// that service's model is deferred when it runs.
    defer_next: Option<StoreGeneration>,
    /// The view's count of what was opened from Chats when the count
    /// began ([`count_from`](Self::count_from)).
    counted_from: u64,
    /// A start with Connect owed: when its service ran, in the root's
    /// milliseconds.
    owed_since: Option<u64>,
}

/// What one pump's messages did to the service's running state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pumped {
    /// The service ran now and did not at the last pump.
    pub started: bool,
    /// That start's model is to be deferred at once.
    pub defer: bool,
}

/// What a launch's owed Connect asks of the root at this pump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launch {
    /// Nothing is owed, or a held screen decides instead.
    Idle,
    /// The clock started: wake the root once [`BOUND_MS`] has passed.
    Armed,
    /// Still within the bound with nothing opened.
    Waiting,
    /// The bound passed or something was opened: defer Connect now.
    GiveUp,
}

/// What [`StartRules::turn`] did beside the model's commands.
#[derive(Debug)]
pub struct Turned {
    /// The service ran now and did not at the last turn.
    pub started: bool,
    /// What the owed Connect asked: on [`Launch::Armed`] the root wakes
    /// itself once [`BOUND_MS`] has passed.
    pub launch: Launch,
    /// The model side's commands, for the facade.
    pub commands: Vec<Command>,
}

impl StartRules {
    /// No start seen yet, and nothing opened from Chats.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            running: false,
            defer_next: None,
            counted_from: 0,
            owed_since: None,
        }
    }

    /// What was opened from Chats counts from here: Chats showed at this
    /// launch, or a later start was asked for. `opened` is the view's
    /// count now.
    pub const fn count_from(&mut self, opened: u64) {
        self.counted_from = opened;
    }

    /// One turn of the root, after the hub's messages were applied: these
    /// rules first, then the model side's turn. The order is the rule:
    /// the turn is where the reads that came in open Connect, so a
    /// deferral asked after it would come once Connect already covered
    /// what the person opened. `held` is whether a held C4 or R4 decides
    /// this start instead; `opened`, the view's count of what was opened
    /// from Chats, queued openings included; `now_ms`, the root's clock.
    pub fn turn<S: Surface, O: Opener>(
        &mut self,
        side: &mut ViewSide<S, O>,
        held: bool,
        opened: u64,
        now_ms: u64,
    ) -> Turned {
        let pumped = self.pumped(side.running(), |asked_at| side.start_decided(asked_at));
        if pumped.defer {
            side.side_mut().connect_defer();
        }
        let owed =
            side.running() && side.side().model().connect_phase() == ConnectPhase::Owed && !held;
        let launch = self.launch(owed, opened, now_ms);
        if launch == Launch::GiveUp {
            side.side_mut().connect_defer();
        }
        Turned {
            started: pumped.started,
            launch,
            commands: side.side_mut().turn(),
        }
    }

    /// After the hub's messages: whether the service started, and whether
    /// its model is the one a passed bound deferred. `refused` says
    /// whether the start asked at a generation was refused, which is what
    /// `ViewSide::start_decided` answers while the service does not run;
    /// a refused start drops the deferral, so the next start owes Connect
    /// again (#651 F1).
    pub fn pumped(
        &mut self,
        running: bool,
        refused: impl FnOnce(StoreGeneration) -> bool,
    ) -> Pumped {
        let started = running && !self.running;
        self.running = running;
        let defer = started && self.defer_next.take().is_some();
        if !running && self.defer_next.is_some_and(refused) {
            self.defer_next = None;
        }
        Pumped { started, defer }
    }

    /// A held start's bound passed. The model of a service that has not
    /// answered is deferred when it does; one already refused is not, as
    /// the next start owes Connect.
    pub const fn bound_passed(&mut self, asked_at: StoreGeneration, refused: bool) {
        if !self.running && !refused {
            self.defer_next = Some(asked_at);
        }
    }

    /// A launch's owed Connect at this pump. `owed` is whether the service
    /// runs with Connect owed and no held screen; `opened`, the view's
    /// count of what was opened from Chats, queued openings included;
    /// `now_ms`, the root's clock. The bound counts from the first pump
    /// that sees it owed, the openings from [`count_from`](Self::count_from).
    pub const fn launch(&mut self, owed: bool, opened: u64, now_ms: u64) -> Launch {
        if !owed {
            self.owed_since = None;
            return Launch::Idle;
        }
        if opened != self.counted_from {
            self.owed_since = None;
            return Launch::GiveUp;
        }
        match self.owed_since {
            None => {
                self.owed_since = Some(now_ms);
                Launch::Armed
            }
            Some(since) => {
                if now_ms.saturating_sub(since) >= BOUND_MS {
                    self.owed_since = None;
                    Launch::GiveUp
                } else {
                    Launch::Waiting
                }
            }
        }
    }

    /// The hub detached the view and it attached again: the service's
    /// state is read afresh; a deferral owed and where the count of
    /// openings began are kept, as it is the same launch.
    pub const fn reattached(&mut self) {
        self.running = false;
        self.owed_since = None;
    }
}

#[cfg(test)]
mod tests {
    use interweave_human_app_core::{ModelSide, Update};
    use interweave_human_client_api::{AddressList, TrustList};
    use interweave_human_ui_model::{UiModel, ViewEvent};
    use interweave_profile_identity::ProfileIdentity;

    use super::*;
    use crate::{Hub, StoreAnswer, ToView};

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

    fn later() -> StoreGeneration {
        let hub = Hub::new();
        hub.set_store_answer(StoreAnswer::Opened);
        hub.store_generation()
    }

    #[test]
    fn a_bound_passed_before_the_service_answered_defers_its_model_once() {
        let mut rules = StartRules::new();
        let asked = StoreGeneration::default();
        assert_eq!(rules.pumped(false, |_| false), Pumped::default());
        rules.bound_passed(asked, false);
        let ran = rules.pumped(true, |_| false);
        assert_eq!(
            ran,
            Pumped {
                started: true,
                defer: true
            }
        );
        assert_eq!(rules.pumped(true, |_| false), Pumped::default(), "once");
        assert_eq!(rules.pumped(false, |_| false), Pumped::default());
        assert_eq!(
            rules.pumped(true, |_| false),
            Pumped {
                started: true,
                defer: false
            },
            "the next start owes Connect again"
        );
    }

    #[test]
    fn a_start_refused_after_its_bound_leaves_connect_owed_at_the_next() {
        let mut rules = StartRules::new();
        let asked = StoreGeneration::default();
        rules.bound_passed(asked, false);
        let mut seen = None;
        let _ = rules.pumped(false, |at| {
            seen = Some(at);
            true
        });
        assert_eq!(seen, Some(asked), "asked about the held start");
        assert_eq!(
            rules.pumped(true, |_| false),
            Pumped {
                started: true,
                defer: false
            },
            "the start after the refused one is not deferred"
        );
    }

    #[test]
    fn a_start_refused_in_the_turn_its_bound_passed_defers_nothing() {
        let mut rules = StartRules::new();
        rules.bound_passed(StoreGeneration::default(), true);
        assert!(!rules.pumped(true, |_| false).defer);
    }

    #[test]
    fn a_bound_passed_with_the_service_running_defers_no_later_start() {
        let mut rules = StartRules::new();
        let _ = rules.pumped(true, |_| false);
        rules.bound_passed(StoreGeneration::default(), false);
        let _ = rules.pumped(false, |_| false);
        assert!(!rules.pumped(true, |_| false).defer);
    }

    #[test]
    fn a_deferral_survives_a_reattach() {
        let mut rules = StartRules::new();
        rules.bound_passed(later(), false);
        rules.reattached();
        assert!(rules.pumped(true, |_| false).defer);
    }

    #[test]
    fn a_launch_opens_connect_only_within_the_bound() {
        let mut rules = StartRules::new();
        rules.count_from(4);
        assert_eq!(rules.launch(true, 4, 100), Launch::Armed);
        assert_eq!(rules.launch(true, 4, 100 + BOUND_MS - 1), Launch::Waiting);
        assert_eq!(rules.launch(true, 4, 100 + BOUND_MS), Launch::GiveUp);
    }

    #[test]
    fn a_launch_gives_connect_up_once_something_was_opened() {
        let mut rules = StartRules::new();
        rules.count_from(4);
        assert_eq!(rules.launch(true, 4, 0), Launch::Armed);
        assert_eq!(rules.launch(true, 4, 10), Launch::Waiting, "control");
        assert_eq!(rules.launch(true, 6, 20), Launch::GiveUp);
    }

    /// Since Chats showed, not since the service ran (§5.1): an opening
    /// made while the start was still on its way gives Connect up at the
    /// first pump that sees it owed. Mutation: taking the count at that
    /// pump, as #651 did, arms the clock here instead.
    #[test]
    fn an_opening_before_the_service_ran_gives_connect_up() {
        let mut rules = StartRules::new();
        rules.count_from(4);
        assert_eq!(rules.launch(false, 5, 0), Launch::Idle);
        assert_eq!(rules.launch(true, 5, 10), Launch::GiveUp);
    }

    #[test]
    fn a_later_start_counts_from_its_ask() {
        let mut rules = StartRules::new();
        rules.count_from(0);
        // Opened while the first start was refused; then the person's
        // Unlock asks the next.
        rules.count_from(3);
        assert_eq!(rules.launch(true, 3, 0), Launch::Armed);
        assert_eq!(rules.launch(true, 4, 10), Launch::GiveUp, "control");
    }

    #[test]
    fn nothing_owed_resets_the_launch_clock() {
        let mut rules = StartRules::new();
        assert_eq!(rules.launch(true, 0, 0), Launch::Armed);
        assert_eq!(rules.launch(false, 0, 5), Launch::Idle);
        assert_eq!(rules.launch(true, 0, BOUND_MS + 5), Launch::Armed);
        rules.reattached();
        assert_eq!(rules.launch(true, 0, BOUND_MS + 6), Launch::Armed);
    }

    /// A reattach is the same launch: what was opened before it still
    /// counts.
    #[test]
    fn a_reattach_keeps_where_the_count_began() {
        let mut rules = StartRules::new();
        rules.count_from(3);
        rules.reattached();
        assert_eq!(rules.launch(true, 3, 0), Launch::Armed, "control");
        rules.reattached();
        assert_eq!(rules.launch(true, 4, 1), Launch::GiveUp);
    }

    fn owed_side() -> ViewSide<Blank, Blank> {
        let mut side = ViewSide::new(|| {
            let mut side = ModelSide::new(Blank, Blank);
            side.connect_owed();
            side
        });
        side.apply(ToView::Running(true)).expect("applied");
        let me = ProfileIdentity::generate()
            .transport_identity()
            .expect("a peer id");
        let reads = [
            Update::TrustRead(Ok(TrustList {
                local_peer: Some(me),
                allowed: Vec::new(),
            })),
            Update::AddressesRead(Ok(AddressList::default())),
        ];
        for read in reads {
            side.apply(ToView::Update(Box::new(read))).expect("applied");
        }
        side
    }

    /// The queued case (add-a-peer §8 item 12): the reads came in with
    /// two empty lists, so the model's turn would open Connect, while an
    /// opening waits in the view's queue for that same turn. The rules
    /// run first and defer Connect, so it never covers what was opened.
    /// Mutation: running the side's turn before the launch rule opens
    /// Connect here.
    #[test]
    fn the_rules_decide_before_the_turn_the_reads_open_connect_in() {
        let mut control = StartRules::new();
        let mut side = owed_side();
        let turned = control.turn(&mut side, false, 0, 0);
        assert_eq!(turned.launch, Launch::Armed);
        assert!(turned.started);
        assert_eq!(
            side.side().model().connect_phase(),
            ConnectPhase::Open,
            "control: nothing opened, the reads open Connect"
        );

        let mut rules = StartRules::new();
        let mut side = owed_side();
        let turned = rules.turn(&mut side, false, 1, 0);
        assert_eq!(turned.launch, Launch::GiveUp);
        assert_eq!(side.side().model().connect_phase(), ConnectPhase::Deferred);
    }

    #[test]
    fn a_held_screen_decides_instead_of_the_launch_rule() {
        let mut rules = StartRules::new();
        let mut side = owed_side();
        let turned = rules.turn(&mut side, true, 1, 0);
        assert_eq!(turned.launch, Launch::Idle);
    }
}
