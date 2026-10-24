// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

//! The order admin writes stand in across admin ports (LOCAL-IPC.md
//! §Cancellation mapping and request concurrency; architect-cto's
//! decision of 2026-10-23, option 4): a write from a port admitted
//! BEFORE the one whose write for the same key the driver last applied is
//! refused `superseded`, never applied over it.
//!
//! The case is one person on one host who gave up on a connection A
//! before opening B: A's write may still be in the server's hands or in
//! the driver's queue when B's write lands, and nothing orders the two
//! but this. Admission order is the order the person meant. A READ
//! orders nothing: a lost write may still take effect after a later read
//! on another port, which the contract states as its limit.
//!
//! Bounded. A key's stamp is needed only while a writer older than it
//! may still deliver a write, so an entry is dropped once no older write
//! admission is alive -- a port that can write, or a write request still
//! queued after its port has gone, each holding its [`Admission`]. Past
//! [`MAX_ORDERED_KEYS`] the oldest entry is evicted and its stamp becomes
//! a FLOOR every older writer is refused below, for every key: the rule
//! fails closed, refusing a write it can no longer order rather than
//! applying it late.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use interweave_local_client_api::AdminCapability;
use interweave_transport_api::TransportIdentity;

/// The most keys whose last writer is remembered: the allowlist's
/// ceiling, so a full trust rewrite while an older port lingers fits
/// without the floor rising.
pub(crate) const MAX_ORDERED_KEYS: usize = 4096;

/// Where admission order is counted and who may still write: shared by
/// the binding, which admits ports, and the driver, which orders writes.
#[derive(Debug, Default)]
pub(crate) struct AdmissionOrder {
    next: AtomicU64,
    /// The stamps of every write admission still alive.
    alive: Mutex<BTreeSet<u64>>,
}

impl AdmissionOrder {
    /// A port's write admission: `Some` exactly when `capabilities` let
    /// it write (`admin.trust` or `admin.peers`). A port that cannot
    /// write is not admitted: the daemon's status port is held for its
    /// whole life, and as the oldest admission alive it would keep every
    /// entry [`LastWrites`] remembers, so the map would never be pruned
    /// and the ceiling's floor would refuse older writers for every key
    /// (`a_port_that_cannot_write_is_not_admitted`).
    pub(crate) fn admit_writer(
        self: &Arc<Self>,
        capabilities: &BTreeSet<AdminCapability>,
    ) -> Option<Admission> {
        (capabilities.contains(&AdminCapability::Trust)
            || capabilities.contains(&AdminCapability::Peers))
        .then(|| self.admit())
    }

    /// Admit a port that may write: the next stamp, alive until the port
    /// and every request carrying a clone of it have gone.
    pub(crate) fn admit(self: &Arc<Self>) -> Admission {
        // The stamp is taken UNDER the lock that publishes it: taken
        // before, a newer admission could be inserted, write and prune
        // while the older one is not yet alive, and `oldest_alive` would
        // skip it. From 1, so no stamp is below a floor of 0.
        let mut alive = self.alive.lock().unwrap_or_else(PoisonError::into_inner);
        let stamp = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        alive.insert(stamp);
        drop(alive);
        Admission(Arc::new(Alive {
            stamp,
            order: Arc::clone(self),
        }))
    }

    /// The oldest write admission still alive, if any.
    fn oldest_alive(&self) -> Option<u64> {
        self.alive
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .first()
            .copied()
    }
}

/// One write admission: a port's place in admission order, carried by
/// each of its write requests so a request queued after its port closed
/// still counts as a writer that may deliver.
#[derive(Debug, Clone)]
pub(crate) struct Admission(Arc<Alive>);

impl Admission {
    pub(crate) fn stamp(&self) -> u64 {
        self.0.stamp
    }
}

#[derive(Debug)]
struct Alive {
    stamp: u64,
    order: Arc<AdmissionOrder>,
}

impl Drop for Alive {
    fn drop(&mut self) {
        self.order
            .alive
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.stamp);
    }
}

/// What a write is ordered by: the peer an `admin.trust.set` names, or
/// the address an `admin.peers` write names, as the bootstrap overlay
/// compares it (the exact string), so the key and the overlay never
/// disagree on whether two writes touch the same entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum WriteKey {
    Peer(TransportIdentity),
    Address(String),
}

/// Per key, the stamp of the admission whose write was last applied.
#[derive(Debug)]
pub(crate) struct LastWrites {
    order: Arc<AdmissionOrder>,
    last: BTreeMap<WriteKey, u64>,
    /// Every writer below it is refused for every key: the stamp of the
    /// newest entry the ceiling evicted.
    floor: u64,
}

impl LastWrites {
    pub(crate) const fn new(order: Arc<AdmissionOrder>) -> Self {
        Self {
            order,
            last: BTreeMap::new(),
            floor: 0,
        }
    }

    /// Whether a write from `by` to `key` is superseded: a later
    /// admission's write to it was applied, or `by` is below the floor.
    pub(crate) fn superseded(&self, by: &Admission, key: &WriteKey) -> bool {
        let stamp = by.stamp();
        stamp < self.floor || self.last.get(key).is_some_and(|&last| stamp < last)
    }

    /// Record that `by`'s write to each of `keys` was applied.
    pub(crate) fn applied(&mut self, by: &Admission, keys: impl IntoIterator<Item = WriteKey>) {
        let stamp = by.stamp();
        for key in keys {
            let last = self.last.entry(key).or_insert(stamp);
            *last = (*last).max(stamp);
        }
        // An entry refuses only a writer older than it, and every live
        // writer -- `by` among them -- is at least the oldest alive.
        if let Some(oldest) = self.order.oldest_alive() {
            self.last.retain(|_, &mut last| last > oldest);
        } else {
            self.last.clear();
        }
        while self.last.len() > MAX_ORDERED_KEYS {
            let Some((key, last)) = self
                .last
                .iter()
                .min_by_key(|&(_, &last)| last)
                .map(|(key, &last)| (key.clone(), last))
            else {
                break;
            };
            self.last.remove(&key);
            self.floor = self.floor.max(last);
        }
    }

    /// How many keys are remembered: what the bound tests read.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.last.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only a port that can write is admitted, so the status port the
    /// daemon holds for its life never holds the map unpruned. The
    /// control is the two writing capabilities, each admitted.
    #[test]
    fn a_port_that_cannot_write_is_not_admitted() {
        let order = Arc::new(AdmissionOrder::default());
        let status = order.admit_writer(&BTreeSet::from([AdminCapability::Status]));
        assert!(status.is_none());
        let others = order.admit_writer(&BTreeSet::from([
            AdminCapability::Status,
            AdminCapability::Endpoints,
            AdminCapability::Shutdown,
        ]));
        assert!(others.is_none(), "no ordered write among them");
        assert_eq!(
            order.oldest_alive(),
            None,
            "nothing alive for the status port"
        );
        let mut writes = LastWrites::new(Arc::clone(&order));
        let b = order
            .admit_writer(&BTreeSet::from([AdminCapability::Trust]))
            .expect("a trust port writes");
        writes.applied(&b, [address(1)]);
        drop(b);
        let c = order
            .admit_writer(&BTreeSet::from([AdminCapability::Peers]))
            .expect("a peers port writes");
        writes.applied(&c, [address(2)]);
        assert_eq!(writes.len(), 0, "pruned: no writer older than either entry");
    }

    fn address(n: usize) -> WriteKey {
        WriteKey::Address(format!("/ip4/192.0.2.1/tcp/{n}"))
    }

    /// B admitted after A: B's write stands and A's is superseded; A's
    /// write to another key is not; a later write on B itself is not.
    #[test]
    fn an_older_admissions_write_is_superseded_for_that_key_only() {
        let order = Arc::new(AdmissionOrder::default());
        let mut writes = LastWrites::new(Arc::clone(&order));
        let a = order.admit();
        let b = order.admit();
        assert!(!writes.superseded(&a, &address(1)), "nothing applied yet");
        writes.applied(&b, [address(1)]);
        assert!(writes.superseded(&a, &address(1)));
        assert!(!writes.superseded(&a, &address(2)), "another key");
        assert!(!writes.superseded(&b, &address(1)), "the same admission");
        // The control: in the other order nothing is superseded.
        writes.applied(&a, [address(2)]);
        assert!(!writes.superseded(&b, &address(2)));
    }

    /// A WRITE QUEUED AFTER ITS PORT CLOSED is still refused: the request
    /// holds the admission alive, so B's entry is not pruned under it.
    /// Without the clone A's write would be the oldest writer no longer
    /// alive and the entry dropped, which is the defect the rule exists
    /// for.
    #[test]
    fn a_request_that_outlives_its_port_is_still_ordered() {
        let order = Arc::new(AdmissionOrder::default());
        let mut writes = LastWrites::new(Arc::clone(&order));
        let port_a = order.admit();
        let queued = port_a.clone();
        drop(port_a);
        let b = order.admit();
        writes.applied(&b, [address(1)]);
        assert!(writes.superseded(&queued, &address(1)));
    }

    /// Once no older writer is alive, nothing is remembered.
    #[test]
    fn an_entry_goes_once_no_older_writer_is_alive() {
        let order = Arc::new(AdmissionOrder::default());
        let mut writes = LastWrites::new(Arc::clone(&order));
        let a = order.admit();
        let b = order.admit();
        writes.applied(&b, [address(1)]);
        assert_eq!(writes.len(), 1, "A may still deliver");
        drop(a);
        writes.applied(&b, [address(2)]);
        assert_eq!(writes.len(), 0, "no writer older than B is alive");
        drop(b);
        let c = order.admit();
        writes.applied(&c, [address(3)]);
        assert_eq!(writes.len(), 0);
    }

    /// Past the ceiling the oldest entry goes and the floor rises: an
    /// older writer is then refused for every key, never applied late.
    #[test]
    fn past_the_ceiling_the_floor_refuses_every_older_writer() {
        let order = Arc::new(AdmissionOrder::default());
        let mut writes = LastWrites::new(Arc::clone(&order));
        let a = order.admit();
        let mut held = Vec::new();
        for n in 0..=MAX_ORDERED_KEYS {
            let b = order.admit();
            writes.applied(&b, [address(n)]);
            held.push(b);
        }
        assert_eq!(writes.len(), MAX_ORDERED_KEYS, "bounded");
        // address(0)'s entry was evicted; A is refused for it by the floor.
        assert!(writes.superseded(&a, &address(0)));
        assert!(writes.superseded(&a, &address(MAX_ORDERED_KEYS + 7)));
        // The control: a writer newer than the floor is refused only where
        // a later write stands.
        let newest = order.admit();
        assert!(!writes.superseded(&newest, &address(0)));
        assert!(!writes.superseded(&held[0], &address(0)));
    }
}
