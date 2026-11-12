// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! C4's or R4's Continue, held until the start decides which screen opens
//! (`design/2026-10-22-add-a-peer.md` §5.1, "Between Continue and the
//! decision"): when the line under Continue shows, and when the start's
//! 10 s bound has passed. A clock only -- what decides the screen is the
//! root's -- over the platform's monotonic milliseconds, so a host test
//! drives every timing the device run reads from its trace.

/// How long after the press, or after a granted prompt, the screen holds
/// before `ConnectStarting` shows under Continue (§5.1: Nielsen's 1 s).
pub const STARTING_AFTER_MS: u64 = 1_000;

/// How long the start may take before Chats opens and the start counts as
/// deferred (§5.1, the bound: Nielsen's 10 s, and the identity note's
/// bound for a wait of this kind).
pub const BOUND_MS: u64 = 10_000;

/// The held screen's clock: the 1 s from the press, and the 10 s from the
/// commit, each stopped while a system prompt is up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartHold {
    /// Since when the 1 s runs; `None` while a prompt is up.
    starting_from: Option<u64>,
    /// The bound's time spent before its last stop.
    spent: u64,
    /// Since when the bound runs; `None` before the commit and while a
    /// prompt is up.
    bound_from: Option<u64>,
    /// The identity was committed: the bound counts.
    committed: bool,
}

impl StartHold {
    /// Continue pressed at `now`. On R4 the identity was committed at R3
    /// and the bound counts from the press; on C4 it counts from the
    /// seal's success, [`Self::committed`].
    #[must_use]
    pub const fn pressed(now: u64, committed: bool) -> Self {
        Self {
            starting_from: Some(now),
            spent: 0,
            bound_from: if committed { Some(now) } else { None },
            committed,
        }
    }

    /// C4's seal succeeded at `now`: the bound starts. A second call
    /// changes nothing.
    pub const fn committed(&mut self, now: u64) {
        if !self.committed {
            self.committed = true;
            self.bound_from = Some(now);
        }
    }

    /// A system prompt came up at `now` (the seal's, or the service
    /// start's): nothing shows under Continue, and neither the 1 s nor the
    /// bound runs while it is up -- it is the person's time, not the app's.
    pub const fn prompt_shown(&mut self, now: u64) {
        self.starting_from = None;
        if let Some(from) = self.bound_from.take() {
            self.spent += now.saturating_sub(from);
        }
    }

    /// The prompt was granted at `now`: the 1 s starts again from zero,
    /// and the bound goes on from where it stopped, not from zero. (A
    /// cancelled prompt ends the hold, which is the root's.)
    pub const fn prompt_granted(&mut self, now: u64) {
        self.starting_from = Some(now);
        if self.committed && self.bound_from.is_none() {
            self.bound_from = Some(now);
        }
    }

    /// `ConnectStarting` shows under Continue at `now`.
    #[must_use]
    pub const fn starting(&self, now: u64) -> bool {
        match self.starting_from {
            Some(from) => now.saturating_sub(from) >= STARTING_AFTER_MS,
            None => false,
        }
    }

    /// The bound has passed at `now`: Chats opens and the start is
    /// deferred.
    #[must_use]
    pub const fn bound_passed(&self, now: u64) -> bool {
        self.bound_spent(now) >= BOUND_MS
    }

    /// When the clock next changes what [`Self::starting`] or
    /// [`Self::bound_passed`] answers, after `now`: the time to wake the
    /// root at. `None` while nothing is due (a prompt is up, or both have
    /// passed).
    #[must_use]
    pub fn next_change(&self, now: u64) -> Option<u64> {
        let starting = self
            .starting_from
            .map(|from| from + STARTING_AFTER_MS)
            .filter(|&at| at > now);
        let bound = self
            .bound_from
            .map(|_| now + (BOUND_MS - self.bound_spent(now).min(BOUND_MS)))
            .filter(|&at| at > now);
        match (starting, bound) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    const fn bound_spent(&self, now: u64) -> u64 {
        match self.bound_from {
            Some(from) => self.spent + now.saturating_sub(from),
            None => self.spent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BOUND_MS, STARTING_AFTER_MS, StartHold};

    /// The line shows 1 s after the press and not before; a decision
    /// under 1 s shows nothing (§5.1). Mutation: `>=` to `>` in
    /// `starting`, or counting from anything but the press, fails here.
    #[test]
    fn the_line_shows_one_second_after_the_press() {
        let hold = StartHold::pressed(500, true);
        assert!(!hold.starting(500));
        assert!(!hold.starting(500 + STARTING_AFTER_MS - 1));
        assert!(hold.starting(500 + STARTING_AFTER_MS));
        assert_eq!(hold.next_change(500), Some(1_500));
    }

    /// On C4 the bound counts from the seal's success, on R4 from the
    /// press (§5.1, the bound).
    #[test]
    fn the_bound_counts_from_the_commit() {
        let r4 = StartHold::pressed(0, true);
        assert!(!r4.bound_passed(BOUND_MS - 1));
        assert!(r4.bound_passed(BOUND_MS));

        let mut c4 = StartHold::pressed(0, false);
        assert!(!c4.bound_passed(60_000), "no bound before the seal");
        c4.committed(2_000);
        assert!(!c4.bound_passed(2_000 + BOUND_MS - 1));
        assert!(c4.bound_passed(2_000 + BOUND_MS));
        c4.committed(5_000);
        assert!(c4.bound_passed(2_000 + BOUND_MS), "a second commit moves nothing");
    }

    /// While a prompt is up nothing shows; a grant starts the 1 s again
    /// from zero (§8 item 12: the seal prompt up 3 s, the line 1 s after
    /// the grant).
    #[test]
    fn a_prompt_hides_the_line_and_its_grant_starts_the_second_again() {
        let mut c4 = StartHold::pressed(0, false);
        c4.prompt_shown(100);
        assert!(!c4.starting(3_000), "the prompt is up");
        assert_eq!(c4.next_change(3_000), None, "nothing due while it is up");
        c4.prompt_granted(3_100);
        assert!(!c4.starting(3_100 + STARTING_AFTER_MS - 1));
        assert!(c4.starting(3_100 + STARTING_AFTER_MS));
    }

    /// The bound stops while the service start's prompt is up and goes on
    /// from where it stopped (§8 item 12: R4, a 5 s prompt, Chats 15 s
    /// after the press). Mutation: restarting the bound from zero at the
    /// grant, or counting the prompt's time, fails here.
    #[test]
    fn the_bound_stops_while_the_start_prompt_is_up() {
        let mut r4 = StartHold::pressed(0, true);
        r4.prompt_shown(1_000);
        assert!(!r4.bound_passed(20_000), "stopped");
        r4.prompt_granted(6_000);
        assert!(!r4.bound_passed(BOUND_MS + 5_000 - 1));
        assert!(r4.bound_passed(BOUND_MS + 5_000));
        assert_eq!(r4.next_change(14_000), Some(15_000));
    }

    /// The next change is whichever comes first, and none once both
    /// passed.
    #[test]
    fn the_next_change_is_the_earlier_one_due() {
        let hold = StartHold::pressed(0, true);
        assert_eq!(hold.next_change(0), Some(STARTING_AFTER_MS));
        assert_eq!(hold.next_change(STARTING_AFTER_MS), Some(BOUND_MS));
        assert_eq!(hold.next_change(BOUND_MS), None);
    }
}
