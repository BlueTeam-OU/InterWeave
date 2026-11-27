// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Trust as one identity holds it (ADR-0067 decision 6, `LOCAL-IPC.md`'s
//! endpoint dimension): every read and write names the endpoint of the
//! identity it is made as, and none reaches the operator's list.
//!
//! [`IdentityTrustPort`] is the only way the facade reaches a trust
//! port: it keeps the port private and the endpoint fixed, so a call
//! without an endpoint has no door here
//! (`no_trust_call_reads_or_writes_the_operators_list`, `tests/facade.rs`).

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use interweave_local_client_api::{
    AdminBinding, AdminCapability, AdminPort, TrustChange, TrustDirection, TrustedPeer,
};
use interweave_transport_api::{EndpointId, TransportError, TransportIdentity};

/// The authored first identity's seed (`admin.trust.seed`, ADR-0067
/// decision 6): asked on EVERY trust connection the client opens, before
/// that connection's first read or write, whichever identity the
/// connection is for -- the seed is safe to repeat, and the first
/// identity's set must exist before any trust act shows or changes trust
/// -- and for the first identity alone: an added identity is given its
/// own empty set at its add, so it is never seeded
/// (`an_added_identity_is_never_seeded`). Clones share one record of
/// whether a seed was answered in this process run.
#[derive(Debug, Clone)]
pub struct FirstSeed {
    first: EndpointId,
    answered: Arc<AtomicBool>,
}

impl FirstSeed {
    /// The seed of `first`, the authored first identity's endpoint.
    #[must_use]
    pub fn new(first: EndpointId) -> Self {
        Self {
            first,
            answered: Arc::default(),
        }
    }

    /// The endpoint it seeds.
    #[must_use]
    pub const fn first(&self) -> &EndpointId {
        &self.first
    }

    /// Whether a seed was answered -- `seeded` or `present`, either is the
    /// set existing -- on any connection of any clone in this process
    /// run. Kept in memory only: a new run asks again, and a failed seed
    /// leaves it as it was (`a_failed_seed_changes_nothing_the_read_or_the_write_says`).
    #[must_use]
    pub fn answered(&self) -> bool {
        self.answered.load(Ordering::Acquire)
    }

    /// Record that a seed was answered, for every clone.
    pub(crate) fn answer(&self) {
        self.answered.store(true, Ordering::Release);
    }
}

/// This node's `PeerId` as any client of the process learned it. Every
/// identity a profile holds lives on the one node, so they share one
/// `PeerId`, and a client that never opened a session of its own (a
/// refused open, or one that reads no status) still names it as the local
/// peer of its trust list: without it, typing the node's own `PeerId`
/// would not be told from any other refusal
/// (`a_client_without_a_session_names_the_peer_another_learned`). Clones
/// share one cell; a client made without one keeps a private cell, which
/// is the peer it learned itself.
///
/// A sibling of [`FirstSeed`] and not a field of it: the seed is fixed at
/// construction and answers a question about the first identity, this
/// one is written by whichever client learns the peer first.
#[derive(Debug, Clone, Default)]
pub struct NodePeer {
    peer: Arc<Mutex<Option<TransportIdentity>>>,
}

impl NodePeer {
    /// An empty cell, to give to every client of one node.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The peer, once any client sharing the cell learned it.
    #[must_use]
    pub fn get(&self) -> Option<TransportIdentity> {
        self.peer
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Record the peer a session or a status read named.
    pub(crate) fn learn(&self, peer: TransportIdentity) {
        *self.peer.lock().unwrap_or_else(PoisonError::into_inner) = Some(peer);
    }
}

/// One administrative connection holding `admin.trust` alone, bound to
/// one identity's endpoint: the port is never handed out, and its only
/// calls are the identity's read and write.
pub(crate) struct IdentityTrustPort<P> {
    port: P,
    endpoint: EndpointId,
}

impl<P: AdminPort> IdentityTrustPort<P> {
    /// Open the connection and, given a seed, ask it first. A failed seed
    /// is dropped and draws no state of its own (architect-cto's ruling,
    /// 2026-11-26): the read or write that follows still names the
    /// endpoint, and says what failed if the set is not there.
    pub(crate) async fn open<A: AdminBinding<Admin = P>>(
        binding: &A,
        endpoint: EndpointId,
        seed: Option<&FirstSeed>,
    ) -> Result<Self, TransportError> {
        let port = binding
            .admin(BTreeSet::from([AdminCapability::Trust]))
            .await?;
        if let Some(seed) = seed
            && port.seed_endpoint_trust(seed.first.clone()).await.is_ok()
        {
            seed.answer();
        }
        Ok(Self { port, endpoint })
    }

    /// The peers BOTH members of the identity's set hold, in the inbound
    /// member's order: the person's view of the set (`LOCAL-IPC.md`, the
    /// endpoint dimension). A peer one member alone holds is not trusted
    /// to the person, and the trust act on it writes both.
    ///
    /// The two directions are two reads, not one atomic answer: a write
    /// between them can show a peer in one and not the other, which reads
    /// as not trusted until the next read, which sees both again.
    pub(crate) async fn read(&self) -> Result<Vec<TrustedPeer>, TransportError> {
        let inbound = self
            .port
            .endpoint_trust(self.endpoint.clone(), TrustDirection::Inbound)
            .await?;
        let outbound = self
            .port
            .endpoint_trust(self.endpoint.clone(), TrustDirection::Outbound)
            .await?;
        let held: BTreeSet<&TransportIdentity> =
            outbound.allowed.iter().map(|row| &row.peer).collect();
        Ok(inbound
            .allowed
            .into_iter()
            .filter(|row| held.contains(&row.peer))
            .collect())
    }

    /// Write `peer` into, or out of, both members of the identity's set.
    pub(crate) async fn set(
        &self,
        peer: TransportIdentity,
        allowed: bool,
    ) -> Result<TrustChange, TransportError> {
        self.port
            .set_endpoint_trust(self.endpoint.clone(), peer, allowed)
            .await
    }
}
