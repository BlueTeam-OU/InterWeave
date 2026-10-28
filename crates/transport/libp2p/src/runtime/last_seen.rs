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
//! peer last seen a week ago. Held in memory only: a restart forgets
//! it, and the row says `null` until the peer connects again.

use std::collections::HashMap;

use interweave_transport_api::TransportIdentity;
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};

/// How many peers [`LastSeen`] remembers. A close is recorded only for
/// a RETAINED connection, and retention admits only a peer the trust
/// sources classify -- the allowlist or the infrastructure set -- so
/// this many is every peer the runtime can hold at once; it is reached
/// only when the sets churn, and then the peer closed longest ago goes.
pub(super) const MAX_LAST_SEEN: usize =
    PeerTrustPolicy::MAX_ALLOWED_PEERS + InfrastructureSet::MAX_ALLOWED_PEERS;

/// The runtime clock's time each peer's last retained connection
/// closed, bounded by [`MAX_LAST_SEEN`].
#[derive(Debug, Default)]
pub(super) struct LastSeen {
    closed_at_ms: HashMap<TransportIdentity, u64>,
}

impl LastSeen {
    /// A retained connection to `peer` closed at `now_ms`. Recorded for
    /// every close, not only the last of the peer's connections: while
    /// another is open the reading says so and takes its own time.
    pub(super) fn closed(&mut self, peer: &TransportIdentity, now_ms: u64) {
        if !self.closed_at_ms.contains_key(peer) && self.closed_at_ms.len() >= MAX_LAST_SEEN {
            // A scan: reached only by churn past every peer the sets
            // can hold, so its cost is not the common close's.
            if let Some(oldest) = self
                .closed_at_ms
                .iter()
                .min_by_key(|(_, at)| **at)
                .map(|(peer, _)| peer.clone())
            {
                self.closed_at_ms.remove(&oldest);
            }
        }
        self.closed_at_ms.insert(peer.clone(), now_ms);
    }

    /// When `peer` was last seen on the runtime clock, read at `now_ms`:
    /// now while `connected`, else its last close, else `None`.
    pub(super) fn at(&self, peer: &TransportIdentity, connected: bool, now_ms: u64) -> Option<u64> {
        if connected {
            Some(now_ms)
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
        seen.closed(&p, 70);
        assert_eq!(seen.at(&p, false, 90), Some(70), "closed: when it closed");
        assert_eq!(seen.at(&p, true, 95), Some(95), "connected again");
        seen.closed(&p, 99);
        assert_eq!(seen.at(&p, false, 120), Some(99), "the latest close");
        assert_eq!(
            seen.at(&peer(2), false, 120),
            None,
            "another peer stays unseen"
        );
    }

    /// The bound holds under churn past it, and what goes is the peer
    /// closed longest ago -- never the one just recorded, nor a peer
    /// already held being re-recorded.
    #[test]
    fn the_table_is_bounded_and_drops_the_longest_closed() {
        let mut seen = LastSeen::default();
        let first = peer(0);
        seen.closed(&first, 0);
        for n in 1..u32::try_from(MAX_LAST_SEEN).expect("fits") {
            seen.closed(&peer(n), u64::from(n));
        }
        assert_eq!(seen.len(), MAX_LAST_SEEN);
        // Re-recording a held peer at the bound evicts nobody.
        seen.closed(&peer(5), 1_000_000);
        assert_eq!(seen.len(), MAX_LAST_SEEN);
        assert_eq!(seen.at(&first, false, 0), Some(0), "nothing evicted yet");
        let late = peer(u32::MAX);
        seen.closed(&late, 2_000_000);
        assert_eq!(seen.len(), MAX_LAST_SEEN, "bounded");
        assert_eq!(seen.at(&first, false, 0), None, "the longest closed went");
        assert_eq!(
            seen.at(&late, false, 0),
            Some(2_000_000),
            "the new one kept"
        );
        assert_eq!(seen.at(&peer(5), false, 0), Some(1_000_000));
    }
}
