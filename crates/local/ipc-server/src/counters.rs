// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The counters only the server can keep, reported by `admin.status`
//! beside the port's own view (`ipc/admin-status`'s `ipc` block).

use std::sync::atomic::{AtomicU64, Ordering};

use interweave_ipc_protocol::{AuthorityDomain, ServerCounters};

/// Shared by every connection task; each field moves on its own.
#[derive(Debug, Default)]
pub struct Counters {
    data_connections: AtomicU64,
    admin_connections: AtomicU64,
    cross_domain_capability_denied: AtomicU64,
    peer_credential_refused: AtomicU64,
    events_dropped_at_stop: AtomicU64,
}

impl Counters {
    /// What `admin.status` reports.
    #[must_use]
    pub fn snapshot(&self) -> ServerCounters {
        ServerCounters {
            data_connections: self.data_connections.load(Ordering::Relaxed),
            admin_connections: self.admin_connections.load(Ordering::Relaxed),
            cross_domain_capability_denied_total: self
                .cross_domain_capability_denied
                .load(Ordering::Relaxed),
            peer_credential_refused_total: self.peer_credential_refused.load(Ordering::Relaxed),
        }
    }

    /// The events the stop's hand-over could not give a client before its
    /// grace ended: what was still in the connection's lane and what its
    /// session still held. Not part of `admin.status`, which is not read
    /// once the server stops; the daemon adds it to the stop's
    /// dropped-event count (`LOCAL-IPC.md`'s shutdown order).
    #[must_use]
    pub fn events_dropped_at_stop(&self) -> u64 {
        self.events_dropped_at_stop.load(Ordering::Relaxed)
    }

    pub(crate) fn dropped_at_stop(&self, events: u64) {
        self.events_dropped_at_stop
            .fetch_add(events, Ordering::Relaxed);
    }

    pub(crate) fn peer_credential_refused(&self) {
        self.peer_credential_refused.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn cross_domain_denied(&self) {
        self.cross_domain_capability_denied
            .fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn opened(&self, domain: AuthorityDomain) {
        self.open(domain).fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn closed(&self, domain: AuthorityDomain) {
        self.open(domain).fetch_sub(1, Ordering::Relaxed);
    }

    const fn open(&self, domain: AuthorityDomain) -> &AtomicU64 {
        match domain {
            AuthorityDomain::Data => &self.data_connections,
            AuthorityDomain::Admin => &self.admin_connections,
        }
    }
}
