// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

//! Removed bootstrap addresses the book could not let go yet: a live
//! quarantine the policy table had no room for stays in the book,
//! quarantined, and the removal is asked again each discovery round
//! (`ConnectionManager::forget_address`, ADR-0062 decision 2).
//!
//! A retry is judged against the operator's inputs AS THEY ARE AT THE
//! RETRY, not as they were at the removal: an address added back since
//! is the operator's again, and a retried removal would take it out of
//! the book after the add put it there. The rule is held here, in the
//! one way out of the set, rather than at each site that can name an
//! address again (`a_removal_named_again_is_dropped_not_retried`).

use interweave_transport_api::TransportIdentity;

#[derive(Debug, Default)]
pub(crate) struct PendingForgets(Vec<(TransportIdentity, String)>);

impl PendingForgets {
    /// Hold `address`'s removal for the next round; a removal already
    /// held is held once (`a_removal_held_twice_is_asked_once`).
    pub(crate) fn hold(&mut self, peer: TransportIdentity, address: String) {
        if !self.0.iter().any(|(p, a)| *p == peer && *a == address) {
            self.0.push((peer, address));
        }
    }

    /// Every held removal still to ask, taken out of the set: one that
    /// `named` -- every operator input as it is now -- names again is
    /// dropped. A removal the book still refuses is handed back to
    /// [`Self::hold`].
    pub(crate) fn due(&mut self, named: &[String]) -> Vec<(TransportIdentity, String)> {
        std::mem::take(&mut self.0)
            .into_iter()
            .filter(|(_, address)| !named.contains(address))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use interweave_profile_identity::ProfileIdentity;

    use super::*;

    fn peer() -> TransportIdentity {
        ProfileIdentity::generate()
            .transport_identity()
            .expect("peer id")
    }

    fn at(peer: &TransportIdentity) -> String {
        format!("/ip4/192.168.77.1/tcp/1/p2p/{}", peer.as_str())
    }

    /// The removal of an address added back before its retry is dropped:
    /// not asked now, and not held for a later round either. The control:
    /// the same removal with the address not named again is asked.
    #[test]
    fn a_removal_named_again_is_dropped_not_retried() {
        let p = peer();
        let address = at(&p);
        let mut pending = PendingForgets::default();
        pending.hold(p.clone(), address.clone());
        assert!(
            pending.due(std::slice::from_ref(&address)).is_empty(),
            "named again, it is not asked"
        );
        assert!(pending.due(&[]).is_empty(), "and it is not held either");

        let mut control = PendingForgets::default();
        control.hold(p.clone(), address.clone());
        let other = at(&peer());
        assert_eq!(control.due(&[other]), vec![(p, address)]);
        assert!(control.due(&[]).is_empty(), "taken out once asked");
    }

    /// A removal held twice -- removed, added back and removed again
    /// within one round -- is asked once; the control, two addresses
    /// held, asks both.
    #[test]
    fn a_removal_held_twice_is_asked_once() {
        let p = peer();
        let mut pending = PendingForgets::default();
        pending.hold(p.clone(), at(&p));
        pending.hold(p.clone(), at(&p));
        assert_eq!(pending.due(&[]).len(), 1);

        let q = peer();
        pending.hold(p.clone(), at(&p));
        pending.hold(q.clone(), at(&q));
        assert_eq!(pending.due(&[]).len(), 2);
    }
}
