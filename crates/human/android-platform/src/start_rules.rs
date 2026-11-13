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
//!
//! A view attached again, or a window made again while the service runs,
//! is not a launch: it shows what this start decided -- Connect if it was
//! open, Chats if the start deferred or settled it -- and while the start
//! has not decided, the bound goes on from the service's start and what
//! the earlier window opened still counts. The rules are the process's,
//! the Activity's root keeping one for every window it makes.

use interweave_human_app_core::{Command, Opener, Surface};
use interweave_human_ui_model::{BOUND_MS, ConnectPhase};

use crate::{StoreGeneration, ViewSide};

/// The root's state for those two rules, carried across a reattach and
/// into a window made again.
#[derive(Debug, Default)]
pub struct StartRules {
    running: bool,
    /// The held start whose bound passed before its service answered:
    /// that service's model is deferred when it runs.
    defer_next: Option<StoreGeneration>,
    /// The view's count of what was opened from Chats when the count
    /// began ([`count_from`](Self::count_from)), or when this window's
    /// view was made ([`new_window`](Self::new_window)).
    counted_from: u64,
    /// The view's count at the last turn, so a window made again knows
    /// whether the one before it opened anything.
    last_opened: u64,
    /// Something was opened from Chats since the count began, in a window
    /// since replaced.
    opened_before: bool,
    /// A start with Connect owed: when its service ran, in the root's
    /// milliseconds.
    owed_since: Option<u64>,
    /// The bound's clock is running and the window that armed its wake
    /// may be gone: the next turn within the bound arms one again.
    rearm: bool,
    /// The hub's count of the start these rules last saw run.
    start: Option<u64>,
    /// What that start decided about an owed Connect, once it did.
    decided: Option<Decided>,
}

/// What a start decided about an owed Connect, which a model made again
/// in that start -- a reattach, a window made again -- shows too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decided {
    /// Connect opened: a model made again opens it again from the reads,
    /// and the launch rule leaves it be.
    Open,
    /// Deferred or settled: Chats, a model made again deferred at once.
    Chats,
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
    /// Wake the root after this many milliseconds: the bound's end, at
    /// [`Launch::Armed`] or in a window made again while the bound runs.
    pub wake_in_ms: Option<u64>,
}

impl StartRules {
    /// No start seen yet, and nothing opened from Chats.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            running: false,
            defer_next: None,
            counted_from: 0,
            last_opened: 0,
            opened_before: false,
            owed_since: None,
            rearm: false,
            start: None,
            decided: None,
        }
    }

    /// What was opened from Chats counts from here: Chats showed at this
    /// launch, or a later start was asked for. `opened` is the view's
    /// count now.
    pub const fn count_from(&mut self, opened: u64) {
        self.counted_from = opened;
        self.last_opened = opened;
        self.opened_before = false;
    }

    /// A window was made, its view's count `opened`: the first of the
    /// process, or one made again after the system ended the last. Its
    /// count starts here, what the last window opened since the count
    /// began still counting; the service's state is read afresh, the
    /// start's clock and decision kept until a later start runs.
    pub const fn new_window(&mut self, opened: u64) {
        if self.last_opened != self.counted_from {
            self.opened_before = true;
        }
        self.counted_from = opened;
        self.last_opened = opened;
        self.reattached();
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
        let pumped = self.pumped(side.running(), side.start(), |asked_at| {
            side.start_decided(asked_at)
        });
        if pumped.defer {
            side.side_mut().connect_defer();
        }
        // A start that opened Connect opens it again in a model made
        // again, from the reads, however long they take.
        let owed = side.running()
            && side.side().model().connect_phase() == ConnectPhase::Owed
            && !held
            && self.decided.is_none();
        let launch = self.launch(owed, opened, now_ms);
        if launch == Launch::GiveUp {
            side.side_mut().connect_defer();
        }
        let wake_in_ms = self.wake(launch, now_ms);
        let commands = side.side_mut().turn();
        self.remember(side);
        self.last_opened = opened;
        Turned {
            started: pumped.started,
            launch,
            commands,
            wake_in_ms,
        }
    }

    /// What the running start's model decided about Connect, kept for a
    /// model made again in that start. Called after each turn, and after
    /// anything else that defers it.
    pub fn remember<S: Surface, O: Opener>(&mut self, side: &ViewSide<S, O>) {
        if !side.running() {
            return;
        }
        match side.side().model().connect_phase() {
            ConnectPhase::Open => self.decided = Some(Decided::Open),
            ConnectPhase::Deferred | ConnectPhase::Settled => self.decided = Some(Decided::Chats),
            ConnectPhase::Owed | ConnectPhase::NotOwed => {}
        }
    }

    /// When the root is to wake for the bound: at its start, or at the
    /// first turn within it in a window made again, for what is left.
    const fn wake(&mut self, launch: Launch, now_ms: u64) -> Option<u64> {
        match (launch, self.owed_since) {
            (Launch::Armed, _) => {
                self.rearm = false;
                Some(BOUND_MS)
            }
            (Launch::Waiting, Some(since)) if self.rearm => {
                self.rearm = false;
                Some(BOUND_MS.saturating_sub(now_ms.saturating_sub(since)))
            }
            _ => None,
        }
    }

    /// After the hub's messages: whether the service started, and whether
    /// its model is the one a passed bound deferred, or a model made again
    /// in a start that deferred or settled Connect. `start` is the hub's
    /// count of the start that runs, `None` while none does. `refused`
    /// says whether the start asked at a generation was refused, which is
    /// what `ViewSide::start_decided` answers while the service does not
    /// run; a refused start drops the deferral, so the next start owes
    /// Connect again (#651 F1).
    pub fn pumped(
        &mut self,
        running: bool,
        start: Option<u64>,
        refused: impl FnOnce(StoreGeneration) -> bool,
    ) -> Pumped {
        // A start is told by its count as well as by the running edge: a
        // stop and the next start drained in one pump show no edge.
        let started = running && (!self.running || (start.is_some() && start != self.start));
        self.running = running;
        let mut defer = started && self.defer_next.take().is_some();
        if started {
            if start.is_some() && start == self.start {
                // The same start, seen again: what it decided stands.
                defer |= self.decided == Some(Decided::Chats);
            } else {
                // A later start decides afresh, its bound from its own run.
                self.start = start;
                self.decided = None;
                self.owed_since = None;
            }
        }
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
        if opened != self.counted_from || self.opened_before {
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
    /// state is read afresh. It is the same launch: a deferral owed, where
    /// the count of openings began, the bound's clock and what the start
    /// decided are kept (add-a-peer §5.1), the clock until a later start
    /// runs.
    pub const fn reattached(&mut self) {
        self.running = false;
        self.rearm = true;
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
        assert_eq!(rules.pumped(false, None, |_| false), Pumped::default());
        rules.bound_passed(asked, false);
        let ran = rules.pumped(true, Some(1), |_| false);
        assert_eq!(
            ran,
            Pumped {
                started: true,
                defer: true
            }
        );
        assert_eq!(
            rules.pumped(true, Some(1), |_| false),
            Pumped::default(),
            "once"
        );
        assert_eq!(rules.pumped(false, None, |_| false), Pumped::default());
        assert_eq!(
            rules.pumped(true, Some(2), |_| false),
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
        let _ = rules.pumped(false, None, |at| {
            seen = Some(at);
            true
        });
        assert_eq!(seen, Some(asked), "asked about the held start");
        assert_eq!(
            rules.pumped(true, Some(1), |_| false),
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
        assert!(!rules.pumped(true, Some(1), |_| false).defer);
    }

    #[test]
    fn a_bound_passed_with_the_service_running_defers_no_later_start() {
        let mut rules = StartRules::new();
        let _ = rules.pumped(true, Some(1), |_| false);
        rules.bound_passed(StoreGeneration::default(), false);
        let _ = rules.pumped(false, None, |_| false);
        assert!(!rules.pumped(true, Some(2), |_| false).defer);
    }

    #[test]
    fn a_deferral_survives_a_reattach() {
        let mut rules = StartRules::new();
        rules.bound_passed(later(), false);
        rules.reattached();
        assert!(rules.pumped(true, Some(1), |_| false).defer);
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
        assert_eq!(
            rules.launch(true, 0, BOUND_MS + 6),
            Launch::Waiting,
            "a reattach is not nothing owed"
        );
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
        let mut side = unread_side(1);
        read_empty(&mut side);
        side
    }

    /// A model of start `start` with Connect owed and its reads not yet
    /// answered: a fresh attach's, or a window's made again.
    fn unread_side(start: u64) -> ViewSide<Blank, Blank> {
        let mut side = ViewSide::new(|| {
            let mut side = ModelSide::new(Blank, Blank);
            side.connect_owed();
            side
        });
        side.apply(ToView::Start(start)).expect("applied");
        side.apply(ToView::Running(true)).expect("applied");
        side
    }

    /// Both reads answer with empty lists: the model's turn opens Connect.
    fn read_empty(side: &mut ViewSide<Blank, Blank>) {
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

    /// add-a-peer §8 item 12: a view attached again is the same launch,
    /// so the bound goes on from the start's success and passing it gives
    /// Connect up, the wake armed again for what is left. Mutation:
    /// `reattached` clearing the clock, as #658 did, arms it again here.
    #[test]
    fn a_reattach_past_the_bound_gives_connect_up() {
        let mut rules = StartRules::new();
        rules.count_from(0);
        let mut side = unread_side(1);
        let turned = rules.turn(&mut side, false, 0, 0);
        assert_eq!(turned.launch, Launch::Armed);
        assert_eq!(turned.wake_in_ms, Some(BOUND_MS));

        rules.reattached();
        let mut again = unread_side(1);
        let turned = rules.turn(&mut again, false, 0, 4_000);
        assert_eq!(turned.launch, Launch::Waiting, "the same clock");
        assert_eq!(
            turned.wake_in_ms,
            Some(BOUND_MS - 4_000),
            "woken at its end"
        );
        assert_eq!(
            rules.turn(&mut again, false, 0, 5_000).wake_in_ms,
            None,
            "once"
        );
        let turned = rules.turn(&mut again, false, 0, BOUND_MS);
        assert_eq!(turned.launch, Launch::GiveUp);
        assert_eq!(again.side().model().connect_phase(), ConnectPhase::Deferred);
    }

    /// A later start, seen after the view attached again, runs a bound of
    /// its own: the clock goes on only within one start.
    #[test]
    fn a_later_start_seen_at_a_reattach_starts_its_own_bound() {
        let mut rules = StartRules::new();
        let mut side = unread_side(1);
        assert_eq!(rules.turn(&mut side, false, 0, 0).launch, Launch::Armed);
        rules.reattached();
        let mut later = unread_side(2);
        assert_eq!(
            rules.turn(&mut later, false, 0, BOUND_MS).launch,
            Launch::Armed
        );
        let mut same = unread_side(2);
        rules.reattached();
        assert_eq!(
            rules.turn(&mut same, false, 0, BOUND_MS + 1).launch,
            Launch::Waiting,
            "control: the same start goes on"
        );
    }

    /// A window made again while the service runs shows what the start
    /// decided (add-a-peer §5.1): a deferred Connect stays deferred though
    /// the fresh model's reads would open it. Control: a later start's
    /// reads open it. Mutation: forgetting the decision at `new_window`
    /// opens Connect over Chats.
    #[test]
    fn a_window_made_again_keeps_a_deferred_connect_deferred() {
        let mut rules = StartRules::new();
        rules.new_window(0);
        let mut side = unread_side(1);
        let _ = rules.turn(&mut side, false, 0, 0);
        assert_eq!(
            rules.turn(&mut side, false, 0, BOUND_MS).launch,
            Launch::GiveUp
        );

        rules.new_window(0);
        let mut again = unread_side(1);
        read_empty(&mut again);
        let _ = rules.turn(&mut again, false, 0, BOUND_MS + 1);
        assert_eq!(again.side().model().connect_phase(), ConnectPhase::Deferred);

        rules.new_window(0);
        let mut later = unread_side(2);
        read_empty(&mut later);
        let _ = rules.turn(&mut later, false, 0, BOUND_MS + 2);
        assert_eq!(
            later.side().model().connect_phase(),
            ConnectPhase::Open,
            "control: a later start decides afresh"
        );
    }

    /// The other half: a start that opened Connect opens it again in a
    /// window made again, past the bound and whatever the new window's
    /// count. Mutation: letting the launch rule judge that model defers it.
    #[test]
    fn a_window_made_again_opens_connect_its_start_opened() {
        let mut rules = StartRules::new();
        rules.new_window(0);
        let mut side = owed_side();
        let _ = rules.turn(&mut side, false, 0, 0);
        assert_eq!(side.side().model().connect_phase(), ConnectPhase::Open);

        rules.new_window(7);
        let mut again = unread_side(1);
        let turned = rules.turn(&mut again, false, 9, BOUND_MS * 3);
        assert_eq!(turned.launch, Launch::Idle);
        read_empty(&mut again);
        let _ = rules.turn(&mut again, false, 9, BOUND_MS * 3 + 1);
        assert_eq!(again.side().model().connect_phase(), ConnectPhase::Open);
    }

    /// What the window before opened from Chats, before the service ran,
    /// still counts in the window made after it. Control: nothing opened,
    /// the clock is armed. Mutation: dropping the earlier window's count
    /// at `new_window` arms the clock in both.
    #[test]
    fn what_an_earlier_window_opened_still_counts() {
        for (opened, expect) in [(0, Launch::Armed), (2, Launch::GiveUp)] {
            let mut rules = StartRules::new();
            rules.new_window(0);
            let mut starting = ViewSide::new(|| ModelSide::new(Blank, Blank));
            let _ = rules.turn(&mut starting, false, opened, 0);
            rules.new_window(0);
            let mut side = unread_side(1);
            assert_eq!(
                rules.turn(&mut side, false, 0, 10).launch,
                expect,
                "{opened}"
            );
        }
    }

    /// A restart whose stop and next start reach the view in one pump
    /// (the hub's `take` drains both) is a later start all the same: its
    /// count, not a running edge, tells it from the start before, so the
    /// earlier start's deferral does not stand for it and its bound runs.
    /// Control: the same messages over two pumps. Mutation: telling a
    /// start only by the running edge leaves it `Idle`, and its reads
    /// open Connect over Chats unbounded (#662 F1).
    #[test]
    fn a_restart_in_one_pump_is_a_later_start() {
        for split in [true, false] {
            let mut rules = StartRules::new();
            rules.count_from(0);
            let mut side = unread_side(1);
            let _ = rules.turn(&mut side, false, 0, 0);
            assert_eq!(
                rules.turn(&mut side, false, 0, BOUND_MS).launch,
                Launch::GiveUp,
                "start 1 deferred Connect"
            );
            side.apply(ToView::Running(false)).expect("applied");
            if split {
                let _ = rules.turn(&mut side, false, 0, BOUND_MS + 1);
            }
            side.apply(ToView::Start(2)).expect("applied");
            side.apply(ToView::Running(true)).expect("applied");
            let turned = rules.turn(&mut side, false, 0, BOUND_MS + 2);
            assert!(turned.started, "split {split}");
            assert_eq!(turned.launch, Launch::Armed, "split {split}");
            assert_eq!(turned.wake_in_ms, Some(BOUND_MS), "split {split}");
            assert_eq!(
                rules.turn(&mut side, false, 1, BOUND_MS + 3).launch,
                Launch::GiveUp,
                "split {split}: an opening gives the later start's Connect up"
            );
        }
    }

    /// A later start the person asked for counts from its ask, whatever an
    /// earlier window opened.
    #[test]
    fn a_later_ask_counts_from_itself_after_a_window_made_again() {
        let mut rules = StartRules::new();
        rules.new_window(0);
        let mut starting = ViewSide::new(|| ModelSide::new(Blank, Blank));
        let _ = rules.turn(&mut starting, false, 2, 0);
        rules.new_window(0);
        rules.count_from(1);
        let mut side = unread_side(1);
        assert_eq!(rules.turn(&mut side, false, 1, 10).launch, Launch::Armed);
    }
}
