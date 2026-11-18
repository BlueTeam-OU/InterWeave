// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The queue of what the window asks of the Activity's Kotlin side
//! (`Native.waitIdentityAsk`), and what a drop of it reports: on every
//! target, so the host tests that a dropped Unlock ask names the presence
//! gate's hold it was for (onboard.rs, `unlock_unasked`).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

/// The asks, in order, for one waiting Kotlin thread: each with its
/// argument, 0 where it takes none. An ask is onboard.rs's `ask` code,
/// a JNI `jint`.
pub(crate) struct Asks {
    queue: Mutex<VecDeque<(i32, u64)>>,
    changed: Condvar,
    /// The last number [`Self::push_numbered`] gave, counted up from 1.
    numbered: AtomicU64,
}

impl Asks {
    pub(crate) const fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            changed: Condvar::new(),
            numbered: AtomicU64::new(0),
        }
    }

    /// `ask` with the next number above every one given before, as its
    /// argument, and that number: a hold asked after a drop has a number
    /// above any the drop reports, so the drop never releases it
    /// (`a_hold_asked_after_a_drop_survives_it`).
    pub(crate) fn push_numbered(&self, ask: i32) -> u64 {
        let number = self.numbered.fetch_add(1, Ordering::SeqCst) + 1;
        self.push_with(ask, number);
        number
    }

    pub(crate) fn push(&self, ask: i32) {
        self.push_with(ask, 0);
    }

    /// `ask` with an argument, which `wait` hands over above its
    /// [`ASK_BITS`](crate::ask_word::ASK_BITS).
    pub(crate) fn push_with(&self, ask: i32, arg: u64) {
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back((ask, arg));
        self.changed.notify_all();
    }

    /// Drop every waiting `ask`: an Unlock prompt asked while a window
    /// showed is not raised over the next one (unlock note §11: leaving
    /// while the prompt shows counts as a cancel). `None` when none
    /// waited, else the largest argument among those dropped -- for the
    /// Unlock ask, the presence gate's ask number, 0 for every other
    /// asker's.
    pub(crate) fn drop_all(&self, ask: i32) -> Option<u64> {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        let dropped = largest_arg(&queue, ask);
        queue.retain(|&(waiting, _)| waiting != ask);
        dropped
    }

    /// Drop what waits: asks an earlier Activity instance left unread
    /// belong to its window, and a late `SECURE_OFF` would clear the new
    /// window's flag while its flow runs (#309 review, risk). Whether a
    /// `reported` ask was among them, as [`Self::drop_all`] answers it.
    pub(crate) fn clear(&self, reported: i32) -> Option<u64> {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        let dropped = largest_arg(&queue, reported);
        queue.clear();
        dropped
    }

    /// The next ask within `timeout`, its argument above its
    /// [`ASK_BITS`](crate::ask_word::ASK_BITS), or 0.
    pub(crate) fn wait(&self, timeout: Duration) -> i64 {
        let queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut queue, _) = self
            .changed
            .wait_timeout_while(queue, timeout, |queue| queue.is_empty())
            .unwrap_or_else(PoisonError::into_inner);
        queue
            .pop_front()
            .map_or(0, |(ask, arg)| crate::ask_word::pack(ask, arg))
    }
}

fn largest_arg(queue: &VecDeque<(i32, u64)>, ask: i32) -> Option<u64> {
    queue
        .iter()
        .filter(|&&(waiting, _)| waiting == ask)
        .map(|&(_, arg)| arg)
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNLOCK: i32 = 8;
    const OTHER: i32 = 3;

    /// A dropped Unlock ask reports the largest gate number among those
    /// dropped, so the gate whose own ask went releases its hold, and
    /// one dropped only of the route's or the notice's asks reports 0;
    /// an ask raised already -- taken by `wait` -- is not among them, and
    /// the other asks stay queued. Mutation: `drop_all` answering
    /// `Some(0)` for any drop fails the first assertion (the gate's hold
    /// is never released); answering the first dropped arg rather than the
    /// largest fails it too.
    #[test]
    fn a_drop_reports_the_gate_number_it_dropped() {
        let asks = Asks::new();
        asks.push_with(UNLOCK, 0);
        asks.push_with(UNLOCK, 4);
        asks.push(OTHER);
        asks.push_with(UNLOCK, 2);
        assert_eq!(asks.drop_all(UNLOCK), Some(4));
        assert_eq!(asks.drop_all(UNLOCK), None, "nothing left to drop");
        assert_eq!(
            asks.wait(Duration::ZERO),
            crate::ask_word::pack(OTHER, 0),
            "another ask stays"
        );

        asks.push(UNLOCK);
        assert_eq!(asks.drop_all(UNLOCK), Some(0), "no gate number dropped");

        asks.push_with(UNLOCK, 9);
        assert_eq!(
            asks.wait(Duration::ZERO),
            crate::ask_word::pack(UNLOCK, 9),
            "raised: Kotlin took it"
        );
        assert_eq!(asks.drop_all(UNLOCK), None, "a raised ask is not dropped");
    }

    /// A gate holding for an ask numbered after a drop is not released by
    /// it, while the hold whose ask the drop took is. Mutation:
    /// `push_numbered` giving any fixed number fails the second
    /// assertion.
    #[test]
    fn a_hold_asked_after_a_drop_survives_it() {
        use interweave_human_ui_model::{Admit, Effect, PresenceGate};

        let asks = Asks::new();
        let mut gate = PresenceGate::new(true);
        assert_eq!(gate.admit(Effect::Seal, 0), Admit::Prompt);
        gate.asked(asks.push_numbered(UNLOCK));
        let through = asks.drop_all(UNLOCK).expect("one dropped");
        assert_eq!(gate.unasked(through), Some(Effect::Seal), "control");

        assert_eq!(gate.admit(Effect::StartService, 1), Admit::Prompt);
        gate.asked(asks.push_numbered(UNLOCK));
        assert_eq!(gate.unasked(through), None, "the later hold stays");
        assert!(gate.holding());
    }

    /// `clear` drops every ask and reports the Unlock asks as `drop_all`
    /// does. Mutation: `clear` answering `None` whatever it dropped fails
    /// the first assertion.
    #[test]
    fn a_clear_reports_the_unlock_asks_it_dropped() {
        let asks = Asks::new();
        asks.push(OTHER);
        asks.push_with(UNLOCK, 6);
        assert_eq!(asks.clear(UNLOCK), Some(6));
        assert_eq!(asks.wait(Duration::ZERO), 0, "nothing left");
        asks.push(OTHER);
        assert_eq!(asks.clear(UNLOCK), None, "no Unlock ask waited");
    }
}
