// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The loss ladder: after the loss of an established connection the
//! first retries are short (ADR-0011 A 2026-11-17, the owner's word;
//! `transport/libp2p/CONNECTIVITY.md` §11).
//!
//! When the last connection to a data-plane peer closes after holding
//! [`LOSS_LADDER_FLOOR_MS`] or more, the delays between the settlement
//! of one failed attempt and the start of the next are
//! [`LOSS_LADDER_MS`] -- 1, 2, 4, 8 and 16 s -- and the gate's 30 s
//! step from the sixth failure on. A connection that held less does not
//! start the ladder again: the retries go on from the step reached.
//!
//! ONE PLACE, read by one accessor: `ConnectionManager::retry_delay_ms`
//! is the only reader, and the delay it returns is the one the peer
//! backoff, the address's failure record and the scheduled retry all
//! take (`record_failure`'s "ONE delay, used for both"), so the two
//! cannot disagree about which step a peer stands on.
//!
//! A peer that reaches the 30 s step LEAVES the map: on the ladder at
//! that step and off it are the same answer, so keeping it would cost
//! an entry for nothing. That is also why a short connection's loss
//! needs no entry of its own -- off the ladder is already "the step
//! reached" once that step is 30 s.

use std::collections::BTreeMap;

use interweave_transport_api::TransportIdentity;

/// The ladder's steps after a loss, in milliseconds: the delay after
/// the first failure, the second, and so on to the fifth. From the
/// sixth failure on the step is `RETRY_CEILING_MS`, the 30 s bound of
/// ADR-0011 A 2026-11-14, which every step here is under.
pub const LOSS_LADDER_MS: [u64; 5] = [1_000, 2_000, 4_000, 8_000, 16_000];

/// How long the lost connection must have held for its loss to start
/// the ladder: 30 seconds. What keeps a peer that accepts and drops from
/// being dialled every second (ADR-0011 A 2026-11-17).
pub const LOSS_LADDER_FLOOR_MS: u64 = 30_000;

/// The peers on the ladder and how many failures each has settled since
/// the loss that put it there.
///
/// BOUNDED twice: an entry is made only for a peer classified
/// `DataPlaneTrusted` at the loss, and only while fewer than the
/// caller's cap (the peer-backoff table's `max_peers`) are held -- a
/// peer refused a place keeps the 30 s step, the direction that dials
/// less. `ConnectionManager::set_trust` drops every peer the new trust
/// does not classify `DataPlaneTrusted`, so the keys stay within the
/// data-plane allowlist.
#[derive(Debug, Default)]
pub(crate) struct LossLadder {
    steps: BTreeMap<TransportIdentity, u32>,
}

impl LossLadder {
    /// The delay the next failure of `peer` takes, or `None` when the
    /// peer is not on the ladder and the caller's own step applies.
    pub(crate) fn delay_ms(&self, peer: &TransportIdentity) -> Option<u64> {
        self.steps
            .get(peer)
            .and_then(|failed| usize::try_from(*failed).ok())
            .and_then(|failed| LOSS_LADDER_MS.get(failed).copied())
    }

    /// A failure of `peer` settled and took the step `delay_ms` gave:
    /// the next one takes the step after. A peer past the last short
    /// step leaves the map (the module note).
    pub(crate) fn failed(&mut self, peer: &TransportIdentity) {
        if let Some(failed) = self.steps.get_mut(peer) {
            *failed = failed.saturating_add(1);
            if usize::try_from(*failed).map_or(true, |f| f >= LOSS_LADDER_MS.len()) {
                self.steps.remove(peer);
            }
        }
    }

    /// The last connection to `peer` closed after holding `held_ms`.
    /// Starts the ladder at its first step when the connection held for
    /// [`LOSS_LADDER_FLOOR_MS`] or more and the peer is `data_plane`;
    /// otherwise changes nothing, so the retries go on from the step
    /// reached. Returns whether the peer is now at the first step.
    pub(crate) fn lost(
        &mut self,
        peer: &TransportIdentity,
        held_ms: u64,
        data_plane: bool,
        cap: usize,
    ) -> bool {
        if !data_plane || held_ms < LOSS_LADDER_FLOOR_MS {
            return false;
        }
        if !self.steps.contains_key(peer) && self.steps.len() >= cap {
            return false;
        }
        self.steps.insert(peer.clone(), 0);
        true
    }

    /// Keep only the peers `keep` answers true for.
    pub(crate) fn retain(&mut self, keep: impl Fn(&TransportIdentity) -> bool) {
        self.steps.retain(|peer, _| keep(peer));
    }

    /// Peers on the ladder, for the bound's tests.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.steps.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P1: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
    const P2: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";

    fn peer(s: &str) -> TransportIdentity {
        TransportIdentity::parse(s).expect("valid identity")
    }

    /// The production numbers are the amendment's, pinned here rather
    /// than only through the manager, so a retuned constant fails by name.
    #[test]
    fn the_ladder_is_the_amendments_numbers() {
        assert_eq!(LOSS_LADDER_MS, [1_000, 2_000, 4_000, 8_000, 16_000]);
        assert_eq!(LOSS_LADDER_FLOOR_MS, 30_000);
    }

    /// The cap: a peer past it is refused a place and keeps the caller's
    /// step; a peer already on the ladder restarts whatever the count.
    #[test]
    fn the_ladder_holds_no_more_peers_than_its_cap() {
        let mut l = LossLadder::default();
        assert!(l.lost(&peer(P1), LOSS_LADDER_FLOOR_MS, true, 1));
        assert!(!l.lost(&peer(P2), LOSS_LADDER_FLOOR_MS, true, 1));
        assert_eq!(l.len(), 1);
        assert_eq!(l.delay_ms(&peer(P2)), None);
        l.failed(&peer(P1));
        assert!(l.lost(&peer(P1), LOSS_LADDER_FLOOR_MS, true, 1));
        assert_eq!(l.delay_ms(&peer(P1)), Some(1_000));
    }

    /// Only a data-plane peer is put on it.
    #[test]
    fn a_peer_not_on_the_data_plane_is_not_put_on_the_ladder() {
        let mut l = LossLadder::default();
        assert!(!l.lost(&peer(P1), u64::MAX, false, 8));
        assert_eq!(l.len(), 0);
    }
}
