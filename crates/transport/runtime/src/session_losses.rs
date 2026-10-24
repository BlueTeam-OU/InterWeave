// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The broadcasts a data session lost, counted (IPC 2.11 `session.losses`).
//!
//! Two classes, each monotonic for one session, promising different
//! things (ADR-0026 A 2026-10-24):
//!
//! - `queue_overflow`: copies the session's own queue lost, its oldest
//!   dropped to take a newer one or a copy refused for want of a queue.
//!   Counted by [`crate::session_queue::SessionQueues::push`], the one
//!   place every inbound and local copy is queued, so no delivery path
//!   can lose a copy without counting it. The broadcast was admitted and
//!   its dedup entry recorded, so within the dedup window a republish is
//!   a duplicate: a lower bound on broadcasts lost.
//! - `ingress_refused`: copies the ingress gate refused (over the
//!   publisher's rate, or while draining) on a channel the session held
//!   when the refusal happened. A refusal records no dedup entry, so a
//!   republish may still be delivered and each refused copy counts: a
//!   signal, never a count of distinct losses.
//!
//! `broadcast_inbound`'s `a_republished_*` tests pin both.
//!
//! # A refusal costs one increment, whoever had joined
//!
//! The gate refuses BEFORE the fan-out, and the rate bucket exists to
//! bound the per-message work done for a publisher that is over it. So a
//! refusal bumps one counter per channel and touches no session. A
//! session records the channel's count when it joins and folds the
//! difference into its own total when it leaves; a read adds what has
//! accrued on the channels it still holds. That is "refused on a channel
//! this session had joined, since the join", with no per-session work on
//! the refused path.
//!
//! # A session's counts live as long as the session, not its queue
//!
//! The queue closes when the session leaves its last channel and opens
//! again on its next join, so a count kept in the queue would fall on a
//! rejoin. Here an entry is made by a session's first join and removed
//! only by [`LossLedger::released`], which a session's end sends; a
//! session that leaves everything and joins again keeps counting from
//! where it was.
//!
//! # What is not counted
//!
//! What GossipSub drops before a message reaches admission (validation,
//! mesh, the backend's own queue), a conflicting body, and a duplicate.
//! So zero means none counted, never none lost.

use std::collections::BTreeMap;

use interweave_transport_api::ChannelId;

/// One session's two counts, as `session.losses` answers them: the
/// neutral API's type, so every binding answers the same shape.
pub use interweave_local_client_api::SessionLosses;

/// One channel's refusal count and how many sessions hold a mark on it.
#[derive(Debug, Default)]
struct ChannelCount {
    refused: u64,
    holders: usize,
}

/// One session's counts and its marks on the channels it holds.
#[derive(Debug, Default)]
struct SessionLedger {
    queue_overflow: u64,
    /// Refusals accrued on channels this session has since left.
    refused_folded: u64,
    /// For each channel held, the channel's count when this session
    /// joined it.
    marks: BTreeMap<ChannelId, u64>,
}

/// Every live session's losses on this profile.
///
/// Bounded by local state: an entry is made by a local join and a
/// channel's count exists only while some session holds a mark on it,
/// so nothing a remote peer names grows either map.
#[derive(Debug, Default)]
pub struct LossLedger {
    channels: BTreeMap<ChannelId, ChannelCount>,
    sessions: BTreeMap<String, SessionLedger>,
}

impl LossLedger {
    /// An empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `session` joined `channel`. A repeated join of a channel already
    /// held keeps the first mark, as the substrate keeps the first join.
    pub fn joined(&mut self, session: &str, channel: &ChannelId) {
        let ledger = self.sessions.entry(session.to_owned()).or_default();
        if ledger.marks.contains_key(channel) {
            return;
        }
        let count = self.channels.entry(channel.clone()).or_default();
        count.holders += 1;
        ledger.marks.insert(channel.clone(), count.refused);
    }

    /// `session` left `channel`: what accrued there since its join is
    /// folded into its own total. A leave of a channel not held is a
    /// no-op.
    pub fn left(&mut self, session: &str, channel: &ChannelId) {
        let Some(ledger) = self.sessions.get_mut(session) else {
            return;
        };
        let Some(mark) = ledger.marks.remove(channel) else {
            return;
        };
        if let Some(count) = self.channels.get_mut(channel) {
            ledger.refused_folded = ledger
                .refused_folded
                .saturating_add(count.refused.saturating_sub(mark));
            count.holders = count.holders.saturating_sub(1);
            if count.holders == 0 {
                self.channels.remove(channel);
            }
        }
    }

    /// `session` ended: its counts and its marks go with it.
    pub fn released(&mut self, session: &str) {
        let Some(ledger) = self.sessions.remove(session) else {
            return;
        };
        for channel in ledger.marks.keys() {
            if let Some(count) = self.channels.get_mut(channel) {
                count.holders = count.holders.saturating_sub(1);
                if count.holders == 0 {
                    self.channels.remove(channel);
                }
            }
        }
    }

    /// The ingress gate refused a broadcast on `channel`. One increment,
    /// and none at all on a channel no session holds.
    pub fn refused(&mut self, channel: &ChannelId) {
        if let Some(count) = self.channels.get_mut(channel) {
            count.refused = count.refused.saturating_add(1);
        }
    }

    /// `session`'s queue lost a copy. A session with no entry has never
    /// joined, or has ended, and nobody can read its count.
    pub fn overflowed(&mut self, session: &str) {
        if let Some(ledger) = self.sessions.get_mut(session) {
            ledger.queue_overflow = ledger.queue_overflow.saturating_add(1);
        }
    }

    /// `session`'s counts now. Zero for a session with no entry.
    #[must_use]
    pub fn read(&self, session: &str) -> SessionLosses {
        let Some(ledger) = self.sessions.get(session) else {
            return SessionLosses::default();
        };
        let held = ledger.marks.iter().fold(0_u64, |sum, (channel, mark)| {
            let now = self.channels.get(channel).map_or(*mark, |c| c.refused);
            sum.saturating_add(now.saturating_sub(*mark))
        });
        SessionLosses {
            queue_overflow: ledger.queue_overflow,
            ingress_refused: ledger.refused_folded.saturating_add(held),
        }
    }

    /// How many sessions and channels the ledger holds, for the tests
    /// that pin it bounded by local state.
    #[cfg(test)]
    fn sizes(&self) -> (usize, usize) {
        (self.sessions.len(), self.channels.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(name: &str) -> ChannelId {
        ChannelId::parse(name).expect("valid channel")
    }

    #[test]
    fn a_refusal_counts_for_the_sessions_holding_its_channel_since_their_join() {
        let mut l = LossLedger::new();
        l.joined("early", &ch("general"));
        l.refused(&ch("general"));
        l.joined("late", &ch("general"));
        l.joined("elsewhere", &ch("other"));
        l.refused(&ch("general"));
        assert_eq!(l.read("early").ingress_refused, 2);
        assert_eq!(l.read("late").ingress_refused, 1, "only since its join");
        assert_eq!(l.read("elsewhere").ingress_refused, 0, "another channel");
    }

    #[test]
    fn a_leave_folds_the_count_in_and_a_rejoin_never_lowers_it() {
        let mut l = LossLedger::new();
        l.joined("s", &ch("general"));
        l.refused(&ch("general"));
        l.overflowed("s");
        l.left("s", &ch("general"));
        let after_leave = l.read("s");
        assert_eq!(
            after_leave,
            SessionLosses {
                queue_overflow: 1,
                ingress_refused: 1
            }
        );
        // Refusals while the session holds nothing are not its losses.
        l.joined("other", &ch("general"));
        l.refused(&ch("general"));
        assert_eq!(l.read("s"), after_leave, "held nothing, lost nothing");
        // Joined again: counting resumes from where it was, never from 0.
        l.joined("s", &ch("general"));
        l.refused(&ch("general"));
        assert_eq!(l.read("s").ingress_refused, 2);
        assert_eq!(l.read("s").queue_overflow, 1);
    }

    #[test]
    fn a_repeated_join_keeps_the_first_mark() {
        let mut l = LossLedger::new();
        l.joined("s", &ch("general"));
        l.refused(&ch("general"));
        l.joined("s", &ch("general"));
        assert_eq!(l.read("s").ingress_refused, 1);
    }

    #[test]
    fn a_release_forgets_the_session_and_a_channel_nobody_holds() {
        let mut l = LossLedger::new();
        l.joined("s", &ch("general"));
        l.overflowed("s");
        l.refused(&ch("general"));
        assert_eq!(l.sizes(), (1, 1), "the control: both held");
        l.released("s");
        assert_eq!(l.sizes(), (0, 0));
        assert_eq!(l.read("s"), SessionLosses::default());
        // A refusal on a channel nobody holds grows nothing.
        l.refused(&ch("general"));
        assert_eq!(l.sizes(), (0, 0));
        // And a session of the same id that joins later starts at zero.
        l.joined("s", &ch("general"));
        assert_eq!(l.read("s"), SessionLosses::default());
    }

    #[test]
    fn a_channel_count_goes_when_its_last_holder_leaves_and_stays_for_the_others() {
        let mut l = LossLedger::new();
        l.joined("a", &ch("general"));
        l.joined("b", &ch("general"));
        l.left("a", &ch("general"));
        assert_eq!(l.sizes().1, 1, "b still holds it");
        l.refused(&ch("general"));
        assert_eq!(l.read("b").ingress_refused, 1);
        assert_eq!(l.read("a").ingress_refused, 0, "a had left");
        l.left("b", &ch("general"));
        assert_eq!(l.sizes().1, 0);
        assert_eq!(l.read("b").ingress_refused, 1, "folded, not lost");
    }

    #[test]
    fn an_overflow_of_a_session_that_never_joined_is_not_kept() {
        let mut l = LossLedger::new();
        l.overflowed("ghost");
        assert_eq!(l.sizes(), (0, 0));
        assert_eq!(l.read("ghost"), SessionLosses::default());
    }
}
