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
//! start and only while nothing was opened from Chats since.

use interweave_human_ui_model::BOUND_MS;

use crate::StoreGeneration;

/// The root's state for those two rules, carried across a reattach.
#[derive(Debug, Default)]
pub struct StartRules {
    running: bool,
    /// The held start whose bound passed before its service answered:
    /// that service's model is deferred when it runs.
    defer_next: Option<StoreGeneration>,
    /// A launch with Connect owed: when its service ran, in the root's
    /// milliseconds, and what had been opened from Chats then.
    owed_launch: Option<(u64, u64)>,
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

impl StartRules {
    /// No start seen yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            running: false,
            defer_next: None,
            owed_launch: None,
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
    /// `now_ms`, the root's clock.
    pub const fn launch(&mut self, owed: bool, opened: u64, now_ms: u64) -> Launch {
        if !owed {
            self.owed_launch = None;
            return Launch::Idle;
        }
        match self.owed_launch {
            None => {
                self.owed_launch = Some((now_ms, opened));
                Launch::Armed
            }
            Some((since, at)) => {
                if now_ms.saturating_sub(since) >= BOUND_MS || opened != at {
                    self.owed_launch = None;
                    Launch::GiveUp
                } else {
                    Launch::Waiting
                }
            }
        }
    }

    /// The hub detached the view and it attached again: the service's
    /// state is read afresh, a deferral owed is kept.
    pub const fn reattached(&mut self) {
        self.running = false;
        self.owed_launch = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hub, StoreAnswer};

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
        assert_eq!(rules.launch(true, 4, 100), Launch::Armed);
        assert_eq!(rules.launch(true, 4, 100 + BOUND_MS - 1), Launch::Waiting);
        assert_eq!(rules.launch(true, 4, 100 + BOUND_MS), Launch::GiveUp);
    }

    #[test]
    fn a_launch_gives_connect_up_once_something_was_opened() {
        let mut rules = StartRules::new();
        assert_eq!(rules.launch(true, 4, 0), Launch::Armed);
        assert_eq!(rules.launch(true, 4, 10), Launch::Waiting, "control");
        assert_eq!(rules.launch(true, 6, 20), Launch::GiveUp);
    }

    #[test]
    fn nothing_owed_resets_the_launch_clock() {
        let mut rules = StartRules::new();
        assert_eq!(rules.launch(true, 0, 0), Launch::Armed);
        assert_eq!(rules.launch(false, 0, 5), Launch::Idle);
        assert_eq!(rules.launch(true, 2, BOUND_MS + 5), Launch::Armed);
        rules.reattached();
        assert_eq!(rules.launch(true, 2, BOUND_MS + 6), Launch::Armed);
    }
}
