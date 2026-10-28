// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

//! When each peer last had a retained connection, for `peer-list`'s
//! `last_seen_ms` (1.2.0): the reading's own time while one is open,
//! else the moment the last one closed, else nothing -- a peer with no
//! retained connection since the runtime started, which is what an
//! operator needs to tell an unreachable host from a silent process
//! on a reachable one.
//!
//! ONE THING, the connection's lifetime: not the last frame, which
//! the muxer sees and nothing above it does, and not the last
//! establishment, which would read a connection up for a week as a
//! peer last seen a week ago. Held in memory: a restart forgets it, and
//! the row says `null` until the peer connects again
//! (`peer_list_says_when_each_peer_was_last_seen_and_a_2_11_client_reads_the_1_1_row`
//! in `tests/desktop-e2e/tests/daemon.rs` restarts the daemon and reads it).
//!
//! STAMPED ON THE WALL CLOCK AT THE CLOSE, not converted at each
//! reading: a monotonic time re-derived as `wall - (now - at)` moves by
//! a millisecond between two readings (the two clocks truncate
//! independently) and by any step of the wall clock, so "the moment it
//! closed" would not be one moment (#503 review F1).

use std::collections::HashMap;

use interweave_transport_api::TransportIdentity;
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};

/// How many peers [`LastSeen`] remembers: every peer the trust sources
/// can classify at once, the allowlist and the infrastructure set.
///
/// Two full, disjoint sets reach it exactly. At the bound a close first
/// forgets every peer the sources no longer classify, and a newcomer
/// they no longer classify is not recorded at all -- it has left the
/// list that reads it. So a classified newcomer always finds room (the
/// classified peers held, itself not among them, number fewer than the
/// bound), and a classified peer is never evicted for anyone (#503
/// review F6). The oldest-first eviction behind it is a backstop
/// (`the_table_is_bounded_and_drops_the_unclassified_first`).
pub(super) const MAX_LAST_SEEN: usize =
    PeerTrustPolicy::MAX_ALLOWED_PEERS + InfrastructureSet::MAX_ALLOWED_PEERS;

/// The wall-clock time, in milliseconds since the Unix epoch, each
/// peer's last retained connection closed, bounded by [`MAX_LAST_SEEN`]
/// (`the_table_is_bounded_and_drops_the_unclassified_first`).
#[derive(Debug, Default)]
pub(super) struct LastSeen {
    closed_at_ms: HashMap<TransportIdentity, u64>,
}

impl LastSeen {
    /// A retained connection to `peer` closed at `wall_ms`. Recorded for
    /// every close, not the last of the peer's connections alone: while
    /// another is open the reading says so and takes its own time.
    /// `classified` says whether the trust sources classify a peer now.
    pub(super) fn closed(
        &mut self,
        peer: &TransportIdentity,
        wall_ms: u64,
        classified: impl Fn(&TransportIdentity) -> bool,
    ) {
        if !self.closed_at_ms.contains_key(peer) && self.closed_at_ms.len() >= MAX_LAST_SEEN {
            // Reached by trust churn up to the bound and nothing else, so
            // neither scan is the common close's cost.
            if !classified(peer) {
                return;
            }
            self.closed_at_ms.retain(|held, _| classified(held));
            if self.closed_at_ms.len() >= MAX_LAST_SEEN
                && let Some(oldest) = self
                    .closed_at_ms
                    .iter()
                    .min_by_key(|(_, at)| **at)
                    .map(|(peer, _)| peer.clone())
            {
                self.closed_at_ms.remove(&oldest);
            }
        }
        self.closed_at_ms.insert(peer.clone(), wall_ms);
    }

    /// When `peer` was last seen, read at `wall_ms`: then while
    /// `connected`, else its last close, else `None`.
    pub(super) fn at(
        &self,
        peer: &TransportIdentity,
        connected: bool,
        wall_ms: u64,
    ) -> Option<u64> {
        if connected {
            Some(wall_ms)
        } else {
            self.closed_at_ms.get(peer).copied()
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.closed_at_ms.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(n: u32) -> TransportIdentity {
        use libp2p::identity::Keypair;
        let mut seed = [0u8; 32];
        seed[..4].copy_from_slice(&n.to_be_bytes());
        let key = Keypair::ed25519_from_bytes(seed).expect("a 32-byte seed");
        TransportIdentity::parse(key.public().to_peer_id().to_base58()).expect("a PeerId")
    }

    fn all(_: &TransportIdentity) -> bool {
        true
    }

    #[test]
    fn a_peer_reads_now_while_connected_its_close_after_and_none_before() {
        let mut seen = LastSeen::default();
        let p = peer(1);
        assert_eq!(seen.at(&p, false, 50), None, "never connected");
        assert_eq!(
            seen.at(&p, true, 60),
            Some(60),
            "connected: the reading's time"
        );
        seen.closed(&p, 70, all);
        assert_eq!(seen.at(&p, false, 90), Some(70), "closed: when it closed");
        assert_eq!(seen.at(&p, false, 5_000), Some(70), "and it stays put");
        assert_eq!(seen.at(&p, true, 95), Some(95), "connected again");
        seen.closed(&p, 99, all);
        assert_eq!(seen.at(&p, false, 120), Some(99), "the latest close");
        assert_eq!(
            seen.at(&peer(2), false, 120),
            None,
            "another peer stays unseen"
        );
    }

    /// The bound holds under churn past it. What goes first is every
    /// peer no longer classified, so a classified peer closed long ago
    /// keeps its time (#503 review F2) -- THE CONTROL: with every peer
    /// classified, the one closed longest ago goes instead. Never the
    /// one just recorded, nor a held peer being re-recorded.
    #[test]
    fn the_table_is_bounded_and_drops_the_unclassified_first() {
        let fill = |seen: &mut LastSeen| {
            for n in 0..u32::try_from(MAX_LAST_SEEN).expect("fits") {
                seen.closed(&peer(n), u64::from(n), all);
            }
        };
        let (first, late) = (peer(0), peer(u32::MAX));

        // Every held peer but `first` was revoked, and `late` -- trusted
        // -- closes: the revoked go, `first` stays, `late` is recorded.
        let mut seen = LastSeen::default();
        fill(&mut seen);
        assert_eq!(seen.len(), MAX_LAST_SEEN);
        seen.closed(&peer(5), 1_000_000, |_| false);
        assert_eq!(
            seen.len(),
            MAX_LAST_SEEN,
            "re-recording a held peer evicts nobody"
        );
        seen.closed(&late, 2_000_000, |p| *p == first || *p == late);
        assert_eq!(seen.len(), 2, "the unclassified went");
        assert_eq!(
            seen.at(&first, false, 0),
            Some(0),
            "the classified one kept its time"
        );
        assert_eq!(
            seen.at(&late, false, 0),
            Some(2_000_000),
            "the new one kept"
        );

        // Full of classified peers -- two full, disjoint sets -- and a
        // revoked newcomer closes: it is not recorded, nobody is evicted.
        let mut seen = LastSeen::default();
        fill(&mut seen);
        seen.closed(&late, 2_000_000, |p| *p != late);
        assert_eq!(seen.len(), MAX_LAST_SEEN, "bounded");
        assert_eq!(
            seen.at(&first, false, 0),
            Some(0),
            "a classified peer kept its time"
        );
        assert_eq!(
            seen.at(&late, false, 0),
            None,
            "the revoked newcomer was not recorded"
        );

        // The backstop: a classified newcomer at a bound of classified
        // peers -- more than the sets can hold, so only a predicate
        // builds it -- evicts the one closed longest ago.
        let mut seen = LastSeen::default();
        fill(&mut seen);
        seen.closed(&late, 2_000_000, all);
        assert_eq!(seen.len(), MAX_LAST_SEEN, "bounded");
        assert_eq!(seen.at(&first, false, 0), None, "the longest closed went");
        assert_eq!(
            seen.at(&late, false, 0),
            Some(2_000_000),
            "the new one kept"
        );
    }
}
