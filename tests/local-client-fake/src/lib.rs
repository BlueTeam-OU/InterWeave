// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An in-memory binding of the four neutral traits (plan §17 (2)): two
//! nodes wired to each other by [`FakeNetwork::pair`], each with its own
//! endpoint table, lease table and per-session queues, and scripted
//! faults.
//!
//! WHAT IT PROVES AND WHAT IT CANNOT. It passes the same generic
//! functions `tests/local-client-conformance` runs against the real
//! bindings (`tests/fake.rs` there), so every local-semantics claim a
//! client relies on -- leases, epochs, queues, notices, the admin overlay
//! -- holds here as it does there. What a fake cannot honour is ASSERTED,
//! not proved: the peer identity a message carries is the configured one
//! ("Noise proved the peer" is configuration here), the two nodes trust
//! each other from pairing until an administrator's `set_trust` revokes
//! it, and the fake does not produce `Timeout` itself: a client sees it
//! via [`FakeNode::inject_send`]. A revocation that takes the peer out of
//! the last set holding it -- the operator's list or an endpoint's own
//! set (ADR-0067 decision 6) -- cuts the pair in both directions, as the
//! runtime's closing of the connections does: the revoking node's sends
//! and queries to the peer are `UnauthorizedPeer`, the peer's to it
//! `PeerUnreachable`, and no broadcast crosses either way; the
//! revocation's `PeerDisconnected` with the `policy` reason is owed to
//! every session of the revoking node -- the peer's own sessions are told
//! nothing, where the runtime would report the closed connection as
//! `closed`. A peer another set still holds stays connected, and the
//! endpoint's own set decides its route: a send from an endpoint whose
//! outbound set lacks the peer is `UnauthorizedPeer`, a direct to an
//! endpoint whose inbound set lacks the sender `RemoteEndpointUnavailable`
//! (the wire's `no_route`), as the runtime answers them
//! (`endpoint_trust_is_the_endpoints_own` in the conformance suite). Two
//! more the fake produces from its own state: `PeerUnreachable` when the
//! other node is dropped or stopped, and `RemoteEndpointUnavailable` when
//! the destination is unknown, disabled or unleased (or no default is
//! configured). So a client tested against it is proved to handle every
//! outcome, not that the network produces them -- reachability stays with
//! `tests/direct-v2` and the end-to-end suites.
//!
//! Every lock is a `std::sync::Mutex` held for one synchronous step and
//! never across an await, and one node's lock is never held while taking
//! the other's.

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the traits are async for the bindings that cross a connection; this one is in memory"
)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::task::{Poll, Waker};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use interweave_local_client_api::{
    AddressChange, AddressRefusal, AddressSource, AddressesChange, AdminBinding, AdminCapability,
    AdminPort, AdminStatus, BootstrapAddress, DataCapability, DataSessionBinding, DataSessionPort,
    EndpointAdminView, EndpointLease, EndpointSetKind, EndpointTrustView, Generation, LeaseRecord,
    LineChange, LocalAdminPort, LocalDataSession, LocalSessionEvent, MAX_ADDRESSES_PER_ADD,
    MAX_EVENT_QUEUE, OwnAddress, OwnAddressKind, OwnAddresses, PeerGateView, PeerOutcome,
    RECONNECTED, ROUTE_ESTABLISHED, ReceivedBroadcast, ReceivedDirect, SessionEvent, SessionLosses,
    SessionRequest, TrustAdminView, TrustChange, TrustDirection, TrustRefusal, TrustSeed,
    TrustSource, TrustedPeer,
};
use interweave_transport_api::{
    BroadcastMessageV1, ChannelId, ConnectivitySummary, DirectDestination, DirectInboundState,
    EndpointDirectoryV1, EndpointId, Health, MessageId, PathReadiness, Payload, PeerPath,
    PreferredPathPolicy, TransportError, TransportIdentity,
};

/// The freshness a directory answer carries, as the real runtime's
/// default clamp gives it: long enough that a test reads it as fresh.
pub const DIRECTORY_TTL_MS: u32 = 60_000;

/// One configured endpoint, as a profile would declare it.
#[derive(Debug, Clone)]
pub struct FakeEndpoint {
    /// The endpoint.
    pub id: EndpointId,
    /// Whether it may be claimed and receives traffic.
    pub enabled: bool,
    /// Whether a directory query lists it while it is leased.
    pub advertise: bool,
    /// The client kinds that may claim it; `None` admits every kind.
    pub allowed_client_kinds: Option<BTreeSet<String>>,
}

impl FakeEndpoint {
    /// An enabled endpoint any kind may claim.
    #[must_use]
    pub const fn open(id: EndpointId, advertise: bool) -> Self {
        Self {
            id,
            enabled: true,
            advertise,
            allowed_client_kinds: None,
        }
    }
}

/// One node's configuration: what a profile would give the runtime.
#[derive(Debug, Clone)]
pub struct FakeConfig {
    /// This node's identity, carried on everything it sends.
    pub peer: TransportIdentity,
    /// Its configured endpoints.
    pub endpoints: Vec<FakeEndpoint>,
    /// The endpoint that receives a directed send naming none.
    pub default_endpoint: Option<EndpointId>,
    /// Every session queue's bound: direct, broadcast and notices alike.
    pub queue_bound: usize,
}

/// The pair constructor.
pub struct FakeNetwork;

impl FakeNetwork {
    /// Two nodes, each the other's only peer, trusting each other.
    ///
    /// # Panics
    /// If a configuration's `queue_bound` is zero or over
    /// [`MAX_EVENT_QUEUE`], which no session may carry -- refused here so
    /// no `open` fails after taking its lease.
    #[must_use]
    pub fn pair(a: FakeConfig, b: FakeConfig) -> (FakeNode, FakeNode) {
        for bound in [a.queue_bound, b.queue_bound] {
            assert!(
                (1..=MAX_EVENT_QUEUE).contains(&bound),
                "a session queue holds 1 to {MAX_EVENT_QUEUE} events, not {bound}"
            );
        }
        let a = Arc::new(Node::new(a, "a"));
        let b = Arc::new(Node::new(b, "b"));
        *lock(&a.remote) = Arc::downgrade(&b);
        *lock(&b.remote) = Arc::downgrade(&a);
        // Each node's configuration names the other: what a row reports
        // as `configured`, and what a restart starts from.
        for (node, other) in [(&a, &b), (&b, &a)] {
            let mut state = lock(&node.state);
            state.trusted.insert(other.peer.clone());
            state.configured.insert(other.peer.clone());
        }
        (FakeNode(a), FakeNode(b))
    }
}

/// One node of a pair: a [`DataSessionBinding`] and an [`AdminBinding`].
#[derive(Clone)]
pub struct FakeNode(Arc<Node>);

impl FakeNode {
    /// This node's identity.
    #[must_use]
    pub fn peer(&self) -> &TransportIdentity {
        &self.0.peer
    }

    /// The next `send_direct` from any session of this node answers
    /// `error` instead of reaching the other node: one per call, in the
    /// order injected. How a client is shown the network outcomes a fake
    /// cannot produce.
    pub fn inject_send(&self, error: TransportError) {
        lock(&self.0.injected).push_back(error);
    }

    /// The runtime stopped and started again over the same state: every
    /// session and lease is gone, and the allowlist is what the admin port
    /// last made of it -- the fake keeps its trust overlay, as every
    /// production binding does (ADR-0028 A 2026-10-07) -- and so are its
    /// endpoints and default: it keeps its endpoint overlay too (ADR-0028
    /// A 2026-10-11).
    pub fn restart(&self) {
        let mut state = lock(&self.0.state);
        for queues in state.sessions.values_mut() {
            queues.wake();
        }
        let ended: Vec<Generation> = state.sessions.keys().cloned().collect();
        state.ended.extend(ended);
        state.sessions.clear();
        state.leases.clear();
        state.stopped = false;
    }

    /// The static-bootstrap entries this node's profile authors, as a
    /// `config.yaml` would: listed first, never removable through the
    /// admin port (ADR-0062).
    pub fn author_bootstrap(&self, entries: Vec<String>) {
        lock(&self.0.state).authored = entries;
    }

    /// Refuse the other node's broadcasts past the runtime's per-peer
    /// ingress default from now, counting each refusal as
    /// `ingress_refused` for the sessions holding its channel, as the
    /// runtime's gate does. Off by default.
    pub fn limit_ingress(&self) {
        lock(&self.0.state).ingress = Some(Bucket::new());
    }

    /// The runtime has stopped: every call from now answers
    /// `BackendUnavailable`.
    pub fn stop(&self) {
        let mut state = lock(&self.0.state);
        state.stopped = true;
        // No session waits on a stopped runtime.
        for queues in state.sessions.values_mut() {
            queues.wake();
        }
    }

    /// `peer`'s path changed: owed to each session with a route to it,
    /// merged into its pending notice ([`Queues::owe_path`]). How a test
    /// drives what a real runtime reports.
    pub fn path_changed(
        &self,
        peer: &TransportIdentity,
        previous: PeerPath,
        current: PeerPath,
        reason_class: &str,
        observed_at: u64,
    ) {
        let mut state = lock(&self.0.state);
        state.path = Some(current);
        for queues in state.sessions.values_mut() {
            if queues.routes.contains(peer) {
                queues.owe_path(peer, Some(previous), current, reason_class, observed_at);
            }
        }
    }

    /// The pair's connection went, `closed`: each session is owed the
    /// `PeerDisconnected`, and a path notice pending for `peer` is
    /// withdrawn while the route is kept, as the runtime does. How a test
    /// drives a reconnect ([`FakeNode::connected`]).
    pub fn disconnected(&self, peer: &TransportIdentity) {
        let mut state = lock(&self.0.state);
        state.path = None;
        let bound = state.queue_bound;
        for queues in state.sessions.values_mut() {
            queues.paths.remove(peer);
            if queues.notices.len() >= bound {
                queues.notices.pop_front();
            }
            queues
                .notices
                .push_back(LocalSessionEvent::PeerDisconnected {
                    peer: peer.clone(),
                    reason_class: "closed".into(),
                });
            queues.wake();
        }
    }

    /// The pair connected again on `path`: each session holding a route
    /// to `peer` is owed that path with no `previous` (`reconnected`, A
    /// 2026-10-09), as the runtime owes it.
    pub fn connected(&self, peer: &TransportIdentity, path: PeerPath) {
        let mut state = lock(&self.0.state);
        state.path = Some(path);
        let now = wall_ms();
        for queues in state.sessions.values_mut() {
            if queues.routes.contains(peer) {
                let class = if queues.unannounced.remove(peer) {
                    ROUTE_ESTABLISHED
                } else {
                    RECONNECTED
                };
                queues.owe_path(peer, None, path, class, now);
            }
        }
    }

    /// The node's health is now `health`: each session is owed it as its
    /// one pending `ServerState`, replacing what it held, when it
    /// changed. How a test drives the state a real runtime computes.
    pub fn set_health(&self, health: Health) {
        let mut state = lock(&self.0.state);
        if state.health == health {
            return;
        }
        state.health = health;
        for queues in state.sessions.values_mut() {
            queues.state = Some(state_event(health));
            queues.wake();
        }
    }

    /// What the owner was asked by `admin.shutdown`, oldest first.
    #[must_use]
    pub fn shutdown_requests(&self) -> Vec<Duration> {
        lock(&self.0.state).shutdown_requests.clone()
    }

    /// What `admin.endpoints.set_advertise` was asked and answered, oldest
    /// first: how a test sees the human client's assertion before a
    /// lease.
    #[must_use]
    pub fn advertise_sets(&self) -> Vec<(EndpointId, bool)> {
        lock(&self.0.state).advertise_sets.clone()
    }

    /// How many calls reached this node's OPERATOR's list -- `trust`
    /// and `set_trust`, the endpoint-less forms -- answered or refused:
    /// how a test proves a client never reads or writes it.
    #[must_use]
    pub fn operator_trust_calls(&self) -> usize {
        lock(&self.0.state).operator_calls
    }

    /// The operator's list as it stands, the peers in order: what an
    /// endpoint-less `trust` would answer, read without counting as a
    /// call.
    #[must_use]
    pub fn operator_list(&self) -> Vec<TransportIdentity> {
        lock(&self.0.state).trusted.iter().cloned().collect()
    }

    /// Every `admin.trust.seed` this node answered, oldest first, with
    /// its answer: a refused seed is not listed.
    #[must_use]
    pub fn seeds(&self) -> Vec<(EndpointId, TrustSeed)> {
        lock(&self.0.state).seeds.clone()
    }

    /// Every trust call any admin port of this node made, oldest first,
    /// each with the port it came on (its admission number): how a test
    /// sees what a connection did first.
    #[must_use]
    pub fn trust_log(&self) -> Vec<(u64, FakeTrustCall)> {
        lock(&self.0.state).trust_log.clone()
    }

    /// Give `endpoint` its own set as written, replacing any it had --
    /// the two members apart, as an overlay written by hand may hold
    /// them (LOCAL-IPC.md, the endpoint dimension). Not a port call:
    /// how a test sets up the state a client meets. It checks no bound:
    /// a test must keep the state inside the runtime's start-time bounds
    /// (an admission set of at most 4096 peers, own rows at most 4 x
    /// 4096), the pair's other node counting as admitted, or the fake
    /// answers from a state no runtime holds.
    pub fn give_own_set(
        &self,
        endpoint: EndpointId,
        inbound: impl IntoIterator<Item = TransportIdentity>,
        outbound: impl IntoIterator<Item = TransportIdentity>,
    ) {
        lock(&self.0.state).own.insert(
            endpoint,
            OwnSet {
                inbound: inbound.into_iter().collect(),
                outbound: outbound.into_iter().collect(),
            },
        );
    }

    /// Sessions open on this node: how a test watches a dropped session
    /// go.
    #[must_use]
    pub fn open_sessions(&self) -> usize {
        lock(&self.0.state).sessions.len()
    }

    /// Sessions on this node holding a route to `peer`: what a test waits
    /// on before driving a change it means to be owed as one. A send's
    /// route is held only once the far end accepted it, as the runtime's,
    /// so the far end can hold the message first, and a change in that
    /// window is the path the route begins on, not a change from it.
    #[must_use]
    pub fn routed_sessions(&self, peer: &TransportIdentity) -> usize {
        lock(&self.0.state)
            .sessions
            .values()
            .filter(|queues| queues.routes.contains(peer))
            .count()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// One trust call an admin port made, as [`FakeNode::trust_log`] lists
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeTrustCall {
    /// `trust`: the operator's list read.
    OperatorRead,
    /// `set_trust`: the operator's list written.
    OperatorSet(TransportIdentity, bool),
    /// `endpoint_trust`: one direction of an endpoint's set read.
    EndpointRead(EndpointId, TrustDirection),
    /// `set_endpoint_trust`: an endpoint's own set written.
    EndpointSet(EndpointId, TransportIdentity, bool),
    /// `seed_endpoint_trust`.
    Seed(EndpointId),
}

/// An endpoint's own set, its two members (ADR-0067 decision 6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct OwnSet {
    inbound: BTreeSet<TransportIdentity>,
    outbound: BTreeSet<TransportIdentity>,
}

impl OwnSet {
    const fn member(&self, direction: TrustDirection) -> &BTreeSet<TransportIdentity> {
        match direction {
            TrustDirection::Inbound => &self.inbound,
            TrustDirection::Outbound => &self.outbound,
        }
    }

    fn holds(&self, peer: &TransportIdentity) -> bool {
        self.inbound.contains(peer) || self.outbound.contains(peer)
    }
}

struct Node {
    peer: TransportIdentity,
    /// The other node of the pair. Weak, so a pair dropped is freed.
    remote: Mutex<Weak<Node>>,
    state: Mutex<State>,
    injected: Mutex<VecDeque<TransportError>>,
    /// Fresh generations: a node tag and a counter, so an epoch or a
    /// session id is never issued twice by either node.
    tag: &'static str,
    next: AtomicU64,
}

#[derive(Debug, Clone)]
struct Lease {
    epoch: Generation,
    session: Generation,
    client_kind: String,
}

#[derive(Default)]
struct Queues {
    /// The node's state, the newest only.
    state: Option<LocalSessionEvent>,
    notices: VecDeque<LocalSessionEvent>,
    direct: VecDeque<ReceivedDirect>,
    broadcast: VecDeque<ReceivedBroadcast>,
    joins: BTreeSet<ChannelId>,
    /// Its broadcast losses since it opened: a dropped oldest, and a
    /// broadcast the ingress bucket refused on a channel it held then.
    losses: SessionLosses,
    /// The peers this session has a route to: a direct message it took or
    /// sent and had accepted, a broadcast it took -- taken, not merely
    /// queued, as the runtime records them. Unbounded and
    /// uncounted, unlike the runtime's (`MAX_ROUTED_PEERS`): a fake node
    /// has exactly one other peer, its pair, so neither set grows past
    /// one entry.
    routes: BTreeSet<TransportIdentity>,
    /// One pending path notice per routed peer, merged as the runtime
    /// merges them; the merges are not counted here.
    paths: BTreeMap<TransportIdentity, (Option<PeerPath>, PeerPath, String, u64)>,
    /// Routes begun while the pair was not connected, whose begin is
    /// announced at the pair's next connection (`route_established`), as
    /// the runtime's `Owed::unannounced`.
    unannounced: BTreeSet<TransportIdentity>,
    /// Every `ready` waiting on this session -- it takes `&self`, so
    /// there may be several -- woken by what is queued, and all of them.
    wakers: Vec<Waker>,
}

impl Queues {
    /// Take at most `max` of what waits here, in the port's order: the
    /// state, the notices, the direct messages, the broadcasts, then the
    /// path notices. `events` and `close` take through it.
    fn take(&mut self, max: usize, path: Option<PeerPath>) -> Vec<SessionEvent> {
        let queues = self;
        let mut taken = Vec::new();
        if max > 0
            && let Some(state) = queues.state.take()
        {
            taken.push(SessionEvent::Local(state));
        }
        while taken.len() < max {
            if let Some(notice) = queues.notices.pop_front() {
                taken.push(SessionEvent::Local(notice));
            } else if let Some(direct) = queues.direct.pop_front() {
                // A message TAKEN is a route, as the runtime records it
                // (`SessionNotices::drained_from`): not one merely queued.
                queues.route(direct.source_peer.clone(), path);
                taken.push(SessionEvent::Direct(direct));
            } else if let Some(broadcast) = queues.broadcast.pop_front() {
                queues.route(broadcast.source_peer.clone(), path);
                taken.push(SessionEvent::Broadcast(broadcast));
            } else if let Some((peer, (previous, current, reason_class, observed_at))) =
                queues.paths.pop_first()
            {
                // The ordinary lane, after every message.
                taken.push(SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
                    peer,
                    previous,
                    current,
                    reason_class,
                    observed_at,
                }));
            } else {
                break;
            }
        }
        taken
    }

    fn wake(&mut self) {
        for waker in self.wakers.drain(..) {
            waker.wake();
        }
    }

    /// Merge one path notice into the pending one for `peer`, as the
    /// runtime merges them: the pending `previous` kept -- an absent one
    /// stays absent -- the newer `current`, class and time taken, and one
    /// that comes back to its `previous` withdrawn.
    fn owe_path(
        &mut self,
        peer: &TransportIdentity,
        previous: Option<PeerPath>,
        current: PeerPath,
        reason_class: &str,
        observed_at: u64,
    ) {
        let merged = self
            .paths
            .remove(peer)
            .map_or(previous, |(pending, ..)| pending);
        if merged != Some(current) {
            self.paths.insert(
                peer.clone(),
                (merged, current, reason_class.to_owned(), observed_at),
            );
        }
        self.wake();
    }

    /// Hold a route to `peer`: one that BEGINS here, toward the pair
    /// while it is connected, is owed its path with no `previous`
    /// (`route_established`, A 2026-10-09).
    fn route(&mut self, peer: TransportIdentity, path: Option<PeerPath>) {
        if self.routes.contains(&peer) {
            return;
        }
        match path {
            Some(path) => self.owe_path(&peer, None, path, ROUTE_ESTABLISHED, wall_ms()),
            None => {
                self.unannounced.insert(peer.clone());
            }
        }
        self.routes.insert(peer);
    }

    fn holds_anything(&self) -> bool {
        self.state.is_some()
            || !self.notices.is_empty()
            || !self.direct.is_empty()
            || !self.broadcast.is_empty()
            || !self.paths.is_empty()
    }
}

/// The summary the fake reports: nothing measured, as a node with no
/// connectivity behaviour configured reads.
fn summary() -> ConnectivitySummary {
    ConnectivitySummary {
        direct_inbound: DirectInboundState::Unknown,
        relay_inbound: PathReadiness::Unavailable,
        active_relay_reservations: 0,
        target_relay_reservations: 0,
        active_relayed_peer_paths: 0,
        hole_punch_inflight: 0,
        preferred_path_policy: PreferredPathPolicy::DirectFirst,
        updated_at: wall_ms(),
    }
}

fn state_event(health: Health) -> LocalSessionEvent {
    LocalSessionEvent::ServerState {
        health,
        connectivity: Some(summary()),
    }
}

struct State {
    endpoints: BTreeMap<EndpointId, FakeEndpoint>,
    default: Option<EndpointId>,
    leases: BTreeMap<EndpointId, Lease>,
    sessions: BTreeMap<Generation, Queues>,
    queue_bound: usize,
    stopped: bool,
    shutdown_requests: Vec<Duration>,
    /// What `admin.endpoints.set_advertise` was asked and applied, oldest
    /// first.
    advertise_sets: Vec<(EndpointId, bool)>,
    /// What [`FakeNode::set_health`] last set.
    health: Health,
    /// The pair's path while it is connected: direct from pairing, then
    /// what [`FakeNode::path_changed`], [`FakeNode::disconnected`] and
    /// [`FakeNode::connected`] make of it. A route that begins is owed it.
    path: Option<PeerPath>,
    /// The data-plane allowlist: the pair's other node from pairing, then
    /// what the admin port's `set_trust` makes of it.
    trusted: BTreeSet<TransportIdentity>,
    /// The allowlist this node was configured with: a row's source.
    configured: BTreeSet<TransportIdentity>,
    /// Each endpoint's own set, once a seed or an endpoint write gave it
    /// one (the endpoint overlay's `inbound` and `outbound`), kept across
    /// a restart as the overlay is. An endpoint without one inherits the
    /// operator's list; the fake authors no `static_subset`.
    own: BTreeMap<EndpointId, OwnSet>,
    /// Calls to the operator's list ([`FakeNode::operator_trust_calls`]).
    operator_calls: usize,
    /// Answered seeds ([`FakeNode::seeds`]), unbounded here since a test
    /// makes few seeds.
    seeds: Vec<(EndpointId, TrustSeed)>,
    /// Every trust call ([`FakeNode::trust_log`]), unbounded here since a
    /// test makes few calls.
    trust_log: Vec<(u64, FakeTrustCall)>,
    /// The sessions a restart ended: theirs was the runtime before it, so
    /// each answers `BackendUnavailable`, as a stopped runtime's does.
    ended: BTreeSet<Generation>,
    /// The static-bootstrap entries the profile authors
    /// ([`FakeNode::author_bootstrap`]), which the overlay cannot remove.
    authored: Vec<String>,
    /// The bootstrap-peers overlay: added entries in the order added,
    /// kept across a restart as every production binding keeps it.
    added: Vec<String>,
    /// Per key -- `peer:` and the peer, `address:` and the address -- the
    /// admission of the port whose write was last applied: the runtime's
    /// superseded rule (LOCAL-IPC.md §Cancellation mapping and request
    /// concurrency), unbounded here since a test makes few writes.
    last_writes: BTreeMap<String, u64>,
    /// The ingress gate's bucket for the pair's other node, the only
    /// publisher a fake node has: the runtime's per-peer default, a burst
    /// of 32 refilled at 120 a minute. `None` unless
    /// [`FakeNode::limit_ingress`] asked for it: a consumer's test that
    /// publishes past the burst is not testing the gate.
    ingress: Option<Bucket>,
}

impl State {
    /// Whether any set admits `peer` at the connection: the operator's
    /// list or any endpoint's own set, either member (ADR-0067 decision
    /// 6's admission set).
    fn admits(&self, peer: &TransportIdentity) -> bool {
        self.trusted.contains(peer) || self.own.values().any(|own| own.holds(peer))
    }

    /// One direction of `endpoint`'s effective set: its own member, else
    /// the operator's list.
    fn effective(
        &self,
        endpoint: &EndpointId,
        direction: TrustDirection,
    ) -> &BTreeSet<TransportIdentity> {
        self.own
            .get(endpoint)
            .map_or(&self.trusted, |own| own.member(direction))
    }

    /// The rows the endpoints' own sets hold, both members of every
    /// endpoint counted: what the bound of 4 x 4096 counts (ADR-0028 A
    /// 2026-11-22).
    fn own_rows(&self) -> usize {
        self.own
            .values()
            .map(|own| own.inbound.len() + own.outbound.len())
            .sum()
    }

    /// The admission set's size.
    fn admitted(&self) -> usize {
        self.admitted_peers().len()
    }

    /// The admission set itself: the operator's list and every own set's
    /// two members, in order.
    fn admitted_peers(&self) -> BTreeSet<&TransportIdentity> {
        let mut all: BTreeSet<&TransportIdentity> = self.trusted.iter().collect();
        for own in self.own.values() {
            all.extend(own.inbound.iter().chain(&own.outbound));
        }
        all
    }

    /// `peer` left the last set that held it: its connection closes, which
    /// every session is told as `PeerDisconnected` with the `policy`
    /// reason, bounded as any notice. The revocation ENDS the route and
    /// withdraws its pending notice, as the runtime's
    /// `SessionNotices::revoked` does: a new exchange after a re-allow is
    /// a new route, owed a new `route_established`.
    fn disconnect(&mut self, peer: &TransportIdentity) {
        let bound = self.queue_bound;
        for queues in self.sessions.values_mut() {
            queues.routes.remove(peer);
            queues.unannounced.remove(peer);
            queues.paths.remove(peer);
            if queues.notices.len() >= bound {
                queues.notices.pop_front();
            }
            queues
                .notices
                .push_back(LocalSessionEvent::PeerDisconnected {
                    peer: peer.clone(),
                    reason_class: "policy".into(),
                });
            queues.wake();
        }
    }
}

/// A token bucket at the runtime's per-peer broadcast defaults.
struct Bucket {
    tokens: u32,
    refilled: std::time::Instant,
}

impl Bucket {
    const BURST: u32 = 32;
    const REFILL: Duration = Duration::from_millis(500);

    fn new() -> Self {
        Self {
            tokens: Self::BURST,
            refilled: std::time::Instant::now(),
        }
    }

    /// Whether one more broadcast is admitted now.
    fn take(&mut self) -> bool {
        let now = std::time::Instant::now();
        let earned = now.duration_since(self.refilled).as_millis() / Self::REFILL.as_millis();
        if earned > 0 {
            let earned = u32::try_from(earned).unwrap_or(u32::MAX);
            self.tokens = self.tokens.saturating_add(earned).min(Self::BURST);
            self.refilled = now;
        }
        if self.tokens == 0 {
            return false;
        }
        self.tokens -= 1;
        true
    }
}

impl Node {
    fn new(config: FakeConfig, tag: &'static str) -> Self {
        let endpoints: BTreeMap<EndpointId, FakeEndpoint> = config
            .endpoints
            .into_iter()
            .map(|e| (e.id.clone(), e))
            .collect();
        Self {
            peer: config.peer,
            remote: Mutex::new(Weak::new()),
            state: Mutex::new(State {
                endpoints,
                default: config.default_endpoint,
                leases: BTreeMap::new(),
                sessions: BTreeMap::new(),
                queue_bound: config.queue_bound,
                stopped: false,
                shutdown_requests: Vec::new(),
                health: Health::Healthy,
                path: Some(PeerPath::Direct),
                trusted: BTreeSet::new(),
                configured: BTreeSet::new(),
                own: BTreeMap::new(),
                operator_calls: 0,
                seeds: Vec::new(),
                trust_log: Vec::new(),
                ended: BTreeSet::new(),
                authored: Vec::new(),
                added: Vec::new(),
                ingress: None,
                last_writes: BTreeMap::new(),
                advertise_sets: Vec::new(),
            }),
            injected: Mutex::new(VecDeque::new()),
            tag,
            next: AtomicU64::new(1),
        }
    }

    #[expect(
        clippy::expect_used,
        reason = "`fake-<tag>-<what>-<16 digits>` is inside the generation grammar by construction"
    )]
    fn fresh(&self, what: &str) -> Generation {
        let n = self.next.fetch_add(1, Ordering::Relaxed);
        Generation::parse(format!("fake-{}-{what}-{n:016}", self.tag))
            .expect("inside the generation grammar")
    }

    fn running(&self) -> Result<MutexGuard<'_, State>, TransportError> {
        let state = lock(&self.state);
        if state.stopped {
            Err(TransportError::BackendUnavailable)
        } else {
            Ok(state)
        }
    }

    /// This node as the FAR end sees it: stopped is the peer gone, never
    /// the sender's own `BackendUnavailable`, which says its runtime
    /// stopped.
    fn reachable(&self) -> Result<MutexGuard<'_, State>, TransportError> {
        self.running().map_err(|_| TransportError::PeerUnreachable)
    }

    fn remote(&self) -> Result<Arc<Self>, TransportError> {
        lock(&self.remote)
            .upgrade()
            .ok_or(TransportError::PeerUnreachable)
    }

    /// The pair's other node, refused as the runtime refuses a peer no
    /// set admits (a revocation from the last set that held it).
    fn trusted_remote(&self) -> Result<Arc<Self>, TransportError> {
        let remote = self.remote()?;
        if self.running()?.admits(&remote.peer) {
            Ok(remote)
        } else {
            Err(TransportError::UnauthorizedPeer)
        }
    }

    /// End `endpoint`'s lease as an administrative act: its holder is
    /// owed the epoch that ended, and what waited on its queue goes.
    fn revoke(state: &mut State, endpoint: &EndpointId) -> Option<Generation> {
        let lease = state.leases.remove(endpoint)?;
        let bound = state.queue_bound;
        if let Some(queues) = state.sessions.get_mut(&lease.session) {
            queues.direct.clear();
            if queues.notices.len() >= bound {
                queues.notices.pop_front();
            }
            queues
                .notices
                .push_back(LocalSessionEvent::EndpointLeaseChanged {
                    endpoint: endpoint.clone(),
                    revoked_epoch: lease.epoch.clone(),
                });
            queues.wake();
        }
        Some(lease.epoch)
    }

    /// Admit a direct message to the lease holder of its destination, as
    /// the receiving runtime does: resolve the endpoint (explicit or the
    /// default), require it enabled, its inbound set holding the sender,
    /// and leased, then room in the holder's queue. A refusal is the
    /// coarse remote class the wire carries.
    fn admit_direct(
        &self,
        source_peer: &TransportIdentity,
        source_endpoint: &EndpointId,
        destination: Option<EndpointId>,
        message_id: MessageId,
        payload: Payload,
    ) -> Result<EndpointId, TransportError> {
        let mut state = self.reachable()?;
        // A peer this node revoked is one whose connections it closed:
        // the sender reaches nothing, as it would not over the network.
        if !state.admits(source_peer) {
            return Err(TransportError::PeerUnreachable);
        }
        let endpoint = destination
            .or_else(|| state.default.clone())
            .ok_or(TransportError::RemoteEndpointUnavailable)?;
        // The endpoint's inbound set decides its route, `no_route` on the
        // wire like every other refusal here.
        let enabled = state.endpoints.get(&endpoint).is_some_and(|e| e.enabled)
            && state
                .effective(&endpoint, TrustDirection::Inbound)
                .contains(source_peer);
        let holder = state.leases.get(&endpoint).map(|l| l.session.clone());
        let (true, Some(holder)) = (enabled, holder) else {
            return Err(TransportError::RemoteEndpointUnavailable);
        };
        let bound = state.queue_bound;
        let queues = state
            .sessions
            .get_mut(&holder)
            .ok_or(TransportError::RemoteEndpointUnavailable)?;
        if queues.direct.len() >= bound {
            return Err(TransportError::Overloaded);
        }
        queues.direct.push_back(ReceivedDirect {
            source_peer: source_peer.clone(),
            source_endpoint: source_endpoint.clone(),
            destination_endpoint: endpoint.clone(),
            message_id,
            payload,
            received_at_ms: wall_ms(),
        });
        queues.wake();
        Ok(endpoint)
    }

    /// Deliver a broadcast to every session on this node that joined its
    /// channel. A full queue gives up its oldest broadcast, as the real
    /// session queue does: a publisher is never refused for a slow reader.
    /// Past the ingress bucket it reaches nobody, and each session holding
    /// the channel counts it refused; a dropped oldest is counted for the
    /// session that dropped it (`session.losses`).
    fn deliver_broadcast(
        &self,
        source_peer: &TransportIdentity,
        channel: &ChannelId,
        message: &BroadcastMessageV1,
    ) {
        let Ok(mut state) = self.running() else {
            return;
        };
        // Nor does a revoked peer's broadcast arrive.
        if !state.admits(source_peer) {
            return;
        }
        if state.ingress.as_mut().is_some_and(|bucket| !bucket.take()) {
            for queues in state.sessions.values_mut() {
                if queues.joins.contains(channel) {
                    queues.losses.ingress_refused += 1;
                }
            }
            return;
        }
        let bound = state.queue_bound;
        for queues in state.sessions.values_mut() {
            if !queues.joins.contains(channel) {
                continue;
            }
            if queues.broadcast.len() >= bound {
                queues.broadcast.pop_front();
                queues.losses.queue_overflow += 1;
            }
            queues.broadcast.push_back(ReceivedBroadcast {
                source_peer: source_peer.clone(),
                channel: channel.clone(),
                message_id: message.message_id,
                payload: message.payload.clone(),
                received_at_ms: wall_ms(),
            });
            queues.wake();
        }
    }
}

impl DataSessionBinding for FakeNode {
    type Session = FakeSession;

    async fn open(&self, request: SessionRequest) -> Result<FakeSession, TransportError> {
        let node = &self.0;
        let session_id = node.fresh("session");
        let mut state = node.running()?;
        let lease = match request.endpoint() {
            None => None,
            Some(endpoint) => {
                let configured = state
                    .endpoints
                    .get(endpoint)
                    .ok_or(TransportError::EndpointUnknown)?;
                if !configured.enabled {
                    return Err(TransportError::EndpointDisabled);
                }
                if configured
                    .allowed_client_kinds
                    .as_ref()
                    .is_some_and(|kinds| !kinds.contains(request.client_kind()))
                {
                    return Err(TransportError::EndpointClientKindDenied);
                }
                if state.leases.contains_key(endpoint) {
                    return Err(TransportError::EndpointInUse);
                }
                Some(EndpointLease {
                    endpoint: endpoint.clone(),
                    epoch: node.fresh("epoch"),
                })
            }
        };
        // The session is built BEFORE the lease is recorded, so a refusal
        // here leaves no lease held by a session that never opened.
        let record = lease
            .as_ref()
            .map(|l| (l.endpoint.clone(), l.epoch.clone()));
        let session = LocalDataSession::new(
            session_id.clone(),
            self.0.peer.clone(),
            request.client_kind(),
            lease,
            request.capabilities().iter().copied(),
            state.queue_bound,
        )
        .map_err(|_| TransportError::InvalidArgument)?;
        if let Some((endpoint, epoch)) = record {
            state.leases.insert(
                endpoint,
                Lease {
                    epoch,
                    session: session_id.clone(),
                    client_kind: request.client_kind().to_owned(),
                },
            );
        }
        // Owed the node's state from the start, as a connection is sent
        // it on connect.
        let owed = Queues {
            state: Some(state_event(state.health)),
            ..Queues::default()
        };
        state.sessions.insert(session_id, owed);
        Ok(FakeSession {
            node: Arc::clone(node),
            session,
            closed: false,
        })
    }
}

/// One open session of a [`FakeNode`]. Dropping it ends it, as `close`
/// does: its lease released, its joins left.
pub struct FakeSession {
    node: Arc<Node>,
    session: LocalDataSession,
    closed: bool,
}

impl FakeSession {
    fn require(&self, capability: DataCapability) -> Result<(), TransportError> {
        if self.session.holds(capability) {
            Ok(())
        } else {
            Err(TransportError::CapabilityDenied)
        }
    }

    /// The node's state while this session's runtime runs: a stopped
    /// runtime, or a session a restart ended, is `BackendUnavailable`.
    fn running(&self) -> Result<MutexGuard<'_, State>, TransportError> {
        let state = self.node.running()?;
        if state.ended.contains(self.session.session_id()) {
            return Err(TransportError::BackendUnavailable);
        }
        Ok(state)
    }

    /// Whether this session's lease is still the live one: a revoked or
    /// replaced lease sends nothing.
    fn lease_is_live(state: &State, lease: &EndpointLease) -> bool {
        state
            .leases
            .get(&lease.endpoint)
            .is_some_and(|live| live.epoch == lease.epoch)
    }

    fn end(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        let mut state = lock(&self.node.state);
        Self::end_in(&mut state, self.session.session_id());
    }

    /// The end itself, under a lock the caller holds.
    fn end_in(state: &mut State, id: &Generation) {
        state.leases.retain(|_, lease| &lease.session != id);
        state.sessions.remove(id);
        // Its last use: an ended session leaves the set with it.
        state.ended.remove(id);
    }
}

impl Drop for FakeSession {
    fn drop(&mut self) {
        self.end();
    }
}

impl DataSessionPort for FakeSession {
    fn session(&self) -> &LocalDataSession {
        &self.session
    }

    async fn join(&self, channel: ChannelId) -> Result<(), TransportError> {
        self.require(DataCapability::Commands)?;
        let mut state = self.running()?;
        let queues = state
            .sessions
            .get_mut(self.session.session_id())
            .ok_or(TransportError::BackendUnavailable)?;
        queues.joins.insert(channel);
        Ok(())
    }

    async fn leave(&self, channel: ChannelId) -> Result<(), TransportError> {
        self.require(DataCapability::Commands)?;
        let mut state = self.running()?;
        if let Some(queues) = state.sessions.get_mut(self.session.session_id()) {
            queues.joins.remove(&channel);
        }
        Ok(())
    }

    async fn losses(&self) -> Result<SessionLosses, TransportError> {
        self.require(DataCapability::Commands)?;
        let state = self.running()?;
        state
            .sessions
            .get(self.session.session_id())
            .map(|queues| queues.losses)
            .ok_or(TransportError::BackendUnavailable)
    }

    async fn broadcast(
        &self,
        channel: ChannelId,
        message: BroadcastMessageV1,
    ) -> Result<(), TransportError> {
        self.require(DataCapability::Commands)?;
        {
            let state = self.running()?;
            let joined = state
                .sessions
                .get(self.session.session_id())
                .is_some_and(|q| q.joins.contains(&channel));
            if !joined {
                return Err(TransportError::ChannelNotJoined);
            }
        }
        // Accepted locally; the other node delivers it to its joined
        // sessions, and the publisher's own node does not echo it.
        if let Ok(remote) = self.node.trusted_remote() {
            remote.deliver_broadcast(&self.node.peer, &channel, &message);
        }
        Ok(())
    }

    async fn send_direct(
        &self,
        destination: DirectDestination,
        message_id: MessageId,
        payload: Payload,
    ) -> Result<EndpointId, TransportError> {
        let source = self.session.authorize_direct_send()?.clone();
        let Some(lease) = self.session.endpoint_lease() else {
            return Err(TransportError::EndpointNotRegistered);
        };
        let live = {
            let state = self.running()?;
            Self::lease_is_live(&state, lease)
        };
        if !live {
            return Err(TransportError::EndpointNotRegistered);
        }
        if let Some(injected) = lock(&self.node.injected).pop_front() {
            return Err(injected);
        }
        let remote = self.node.remote()?;
        if destination.peer != remote.peer {
            return Err(TransportError::PeerUnknown);
        }
        let remote = self.node.trusted_remote()?;
        // The source endpoint's outbound set narrows the send, as the
        // runtime's `authorize_outbound` does, with its answer.
        if !self
            .running()?
            .effective(&source, TrustDirection::Outbound)
            .contains(&remote.peer)
        {
            return Err(TransportError::UnauthorizedPeer);
        }
        let accepted = remote.admit_direct(
            &self.node.peer,
            &source,
            destination.endpoint,
            message_id,
            payload,
        );
        if accepted.is_ok() {
            let mut state = lock(&self.node.state);
            let path = state.path;
            if let Some(queues) = state.sessions.get_mut(self.session.session_id()) {
                queues.route(destination.peer, path);
            }
        }
        accepted
    }

    async fn events(&self, max: usize) -> Result<Vec<SessionEvent>, TransportError> {
        self.require(DataCapability::Events)?;
        let mut state = self.running()?;
        let path = state.path;
        let Some(queues) = state.sessions.get_mut(self.session.session_id()) else {
            return Ok(Vec::new());
        };
        Ok(queues.take(max, path))
    }

    async fn ready(&self) -> Result<(), TransportError> {
        self.require(DataCapability::Events)?;
        std::future::poll_fn(|cx| {
            let mut state = lock(&self.node.state);
            if state.stopped {
                return Poll::Ready(Ok(()));
            }
            match state.sessions.get_mut(self.session.session_id()) {
                Some(queues) if !queues.holds_anything() => {
                    // One entry per waiting task: a task polled again
                    // replaces its own rather than adding one.
                    queues.wakers.retain(|w| !w.will_wake(cx.waker()));
                    queues.wakers.push(cx.waker().clone());
                    Poll::Pending
                }
                // Something waits, or the session has gone.
                _ => Poll::Ready(Ok(())),
            }
        })
        .await
    }

    async fn query_endpoints(
        &self,
        peer: TransportIdentity,
    ) -> Result<EndpointDirectoryV1, TransportError> {
        self.require(DataCapability::EndpointsQuery)?;
        drop(self.running()?);
        let remote = self.node.remote()?;
        if peer != remote.peer {
            return Err(TransportError::PeerUnknown);
        }
        let remote = self.node.trusted_remote()?;
        let state = remote.reachable()?;
        if !state.admits(&self.node.peer) {
            return Err(TransportError::PeerUnreachable);
        }
        let endpoints = state
            .endpoints
            .values()
            .filter(|e| e.enabled && e.advertise && state.leases.contains_key(&e.id))
            // The runtime's `advertised_for` drops an endpoint whose own
            // inbound set refuses the querier, as it drops one that is
            // not leased: the directory shows what the querier may send to.
            .filter(|e| {
                state
                    .effective(&e.id, TrustDirection::Inbound)
                    .contains(&self.node.peer)
            })
            .map(|e| e.id.clone())
            .collect();
        Ok(EndpointDirectoryV1 {
            generated_at_ms: wall_ms(),
            ttl_ms: DIRECTORY_TTL_MS,
            endpoints,
        })
    }

    /// What the session's queues hold is taken in the step that ends its
    /// lease, under the one lock a send admits under, so nothing is
    /// admitted between the take and the end (`LOCAL-CLIENT.md`, A
    /// 2026-11-28).
    async fn close(mut self) -> Result<Vec<SessionEvent>, TransportError> {
        let (stopped, remainder) = {
            let mut state = lock(&self.node.state);
            let stopped = state.stopped || state.ended.contains(self.session.session_id());
            let path = state.path;
            let remainder = match state.sessions.get_mut(self.session.session_id()) {
                Some(queues) if !stopped && self.session.holds(DataCapability::Events) => {
                    queues.take(usize::MAX, path)
                }
                _ => Vec::new(),
            };
            self.closed = true;
            Self::end_in(&mut state, self.session.session_id());
            (stopped, remainder)
        };
        if stopped {
            Err(TransportError::BackendUnavailable)
        } else {
            Ok(remainder)
        }
    }
}

impl AdminBinding for FakeNode {
    type Admin = FakeAdmin;

    async fn admin(
        &self,
        capabilities: BTreeSet<AdminCapability>,
    ) -> Result<FakeAdmin, TransportError> {
        drop(self.0.running()?);
        Ok(FakeAdmin {
            node: Arc::clone(&self.0),
            port: LocalAdminPort::new(self.0.fresh("admin"), capabilities),
            admitted: self.0.next.fetch_add(1, Ordering::Relaxed),
        })
    }
}

/// One open administrative port of a [`FakeNode`]. It never holds a lease.
pub struct FakeAdmin {
    node: Arc<Node>,
    port: LocalAdminPort,
    /// The port's place in admission order, which orders its writes.
    admitted: u64,
}

impl FakeAdmin {
    /// Whether this port's write to `key` is superseded: a port admitted
    /// after it wrote `key` last.
    fn superseded(&self, state: &State, key: &str) -> bool {
        state
            .last_writes
            .get(key)
            .is_some_and(|&last| self.admitted < last)
    }

    fn applied(&self, state: &mut State, key: String) {
        let last = state.last_writes.entry(key).or_insert(self.admitted);
        *last = (*last).max(self.admitted);
    }

    /// Record a trust call on the node's log, and an operator's-list call
    /// on its count, whatever it is answered.
    fn log(&self, call: FakeTrustCall) {
        let mut state = lock(&self.node.state);
        if matches!(
            call,
            FakeTrustCall::OperatorRead | FakeTrustCall::OperatorSet(..)
        ) {
            state.operator_calls += 1;
        }
        state.trust_log.push((self.admitted, call));
    }

    fn require(&self, capability: AdminCapability) -> Result<(), TransportError> {
        if self.port.holds(capability) {
            Ok(())
        } else {
            Err(TransportError::CapabilityDenied)
        }
    }
}

impl AdminPort for FakeAdmin {
    fn port(&self) -> &LocalAdminPort {
        &self.port
    }

    async fn status(&self) -> Result<AdminStatus, TransportError> {
        self.require(AdminCapability::Status)?;
        let state = self.node.running()?;
        Ok(AdminStatus {
            health: state.health,
            peer: self.node.peer.clone(),
            connectivity: summary(),
            active_leases: state.leases.len(),
            pre_auth: None,
            ingress: None,
            // The fake keeps no count of its queues' drops, so it names none.
            events_dropped: None,
            events_refused: None,
        })
    }

    async fn leases(&self) -> Result<Vec<EndpointAdminView>, TransportError> {
        self.require(AdminCapability::Endpoints)?;
        let state = self.node.running()?;
        Ok(state
            .endpoints
            .values()
            .map(|e| EndpointAdminView {
                endpoint: e.id.clone(),
                enabled: e.enabled,
                default: state.default.as_ref() == Some(&e.id),
                // Kept across `restart`, as the endpoint overlay is.
                persisted: true,
                alias: None,
                source: interweave_local_client_api::EndpointSource::Authored,
                advertise: e.advertise,
                lease: state.leases.get(&e.id).map(|lease| LeaseRecord {
                    endpoint: e.id.clone(),
                    epoch: lease.epoch.clone(),
                    client_kind: lease.client_kind.clone(),
                    session_id: Some(lease.session.as_str().to_owned()),
                }),
            })
            .collect())
    }

    async fn revoke_endpoint(&self, endpoint: EndpointId) -> Result<(), TransportError> {
        self.require(AdminCapability::Endpoints)?;
        let mut state = self.node.running()?;
        Node::revoke(&mut state, &endpoint);
        Ok(())
    }

    async fn set_endpoint_enabled(
        &self,
        endpoint: EndpointId,
        enabled: bool,
    ) -> Result<Option<Generation>, TransportError> {
        self.require(AdminCapability::Endpoints)?;
        let mut state = self.node.running()?;
        let configured = state
            .endpoints
            .get_mut(&endpoint)
            .ok_or(TransportError::EndpointUnknown)?;
        configured.enabled = enabled;
        if enabled {
            return Ok(None);
        }
        // Disabling the default clears it, and enabling restores nothing.
        if state.default.as_ref() == Some(&endpoint) {
            state.default = None;
        }
        Ok(Node::revoke(&mut state, &endpoint))
    }

    async fn set_default_endpoint(
        &self,
        endpoint: Option<EndpointId>,
    ) -> Result<(), TransportError> {
        self.require(AdminCapability::Endpoints)?;
        let mut state = self.node.running()?;
        if let Some(endpoint) = &endpoint {
            let configured = state
                .endpoints
                .get(endpoint)
                .ok_or(TransportError::EndpointUnknown)?;
            if !configured.enabled {
                return Err(TransportError::EndpointDisabled);
            }
        }
        state.default = endpoint;
        Ok(())
    }

    async fn set_endpoint_advertise(
        &self,
        endpoint: EndpointId,
        advertise: bool,
    ) -> Result<(), TransportError> {
        self.require(AdminCapability::Endpoints)?;
        let mut state = self.node.running()?;
        let configured = state
            .endpoints
            .get_mut(&endpoint)
            .ok_or(TransportError::EndpointUnknown)?;
        // In force in the directory's answers before the answer, as the
        // runtime's is; kept across a restart, as its overlay is.
        configured.advertise = advertise;
        state.advertise_sets.push((endpoint, advertise));
        Ok(())
    }

    async fn shutdown(&self, grace: Duration) -> Result<(), TransportError> {
        self.require(AdminCapability::Shutdown)?;
        let mut state = self.node.running()?;
        state.shutdown_requests.push(grace);
        Ok(())
    }

    async fn trust(&self) -> Result<TrustAdminView, TransportError> {
        self.log(FakeTrustCall::OperatorRead);
        self.require(AdminCapability::Trust)?;
        let state = self.node.running()?;
        Ok(TrustAdminView {
            local_peer: Some(self.node.peer.clone()),
            allowed: operator_rows(&state),
        })
    }

    /// One row per admitted peer, in order: the admission set, as the
    /// runtime's `peer_rows` reads it. The fake has no dial gate, so
    /// nothing is ever in backoff or quarantined; the paired node, while
    /// trusted, reads connected.
    async fn peers(&self) -> Result<Vec<PeerGateView>, TransportError> {
        self.require(AdminCapability::Status)?;
        let paired = self.node.remote().ok().map(|remote| remote.peer.clone());
        let state = self.node.running()?;
        Ok(state
            .admitted_peers()
            .into_iter()
            .map(|peer| {
                let connected = paired.as_ref() == Some(peer);
                PeerGateView {
                    peer: peer.clone(),
                    connected,
                    // Seen now while paired; a fake keeps no closes.
                    last_seen: if connected {
                        interweave_local_client_api::PeerLastSeen::At(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX)),
                        )
                    } else {
                        interweave_local_client_api::PeerLastSeen::Never
                    },
                    backoff_until: None,
                    quarantined_until: None,
                    route_held_until: None,
                    last_outcome: connected.then_some(PeerOutcome::Connected),
                }
            })
            .collect())
    }

    /// The refusals the runtime's policy makes (the local peer, a new
    /// peer at the ceiling) and its no-ops; revoking the paired node from
    /// the last set that held it is owed to every session as
    /// `PeerDisconnected` with the `policy` reason, bounded as any
    /// notice. A set the policy admits from a port admitted before the
    /// last applied set's for the peer is refused `superseded`, nothing
    /// changed; an applied set, a no-op included, orders.
    async fn set_trust(
        &self,
        peer: TransportIdentity,
        allowed: bool,
    ) -> Result<TrustChange, TransportError> {
        self.log(FakeTrustCall::OperatorSet(peer.clone(), allowed));
        self.require(AdminCapability::Trust)?;
        // The only peer that can hold a connection here is the pair's
        // other node; a revoked peer that never could is told to nobody.
        let paired = self.node.remote().is_ok_and(|remote| remote.peer == peer);
        let mut state = self.node.running()?;
        // The bound is the admission set's, which the operator's list
        // shares with every endpoint's own set (runtime `set_trust`).
        if allowed
            && (peer == self.node.peer
                || (!state.admits(&peer) && state.admitted() >= MAX_ALLOWED_PEERS))
        {
            return Err(TransportError::InvalidArgument);
        }
        let key = format!("peer:{}", peer.as_str());
        if self.superseded(&state, &key) {
            return Ok(TrustChange::Refused(TrustRefusal::Superseded));
        }
        self.applied(&mut state, key);
        if allowed {
            state.trusted.insert(peer);
            return Ok(TrustChange::Set);
        }
        if !state.trusted.remove(&peer) || !paired || state.admits(&peer) {
            return Ok(TrustChange::Set);
        }
        state.disconnect(&peer);
        Ok(TrustChange::Set)
    }

    /// The endpoint's own member when it has a set, every row
    /// administered; else the operator's list, its rows with their
    /// sources, `inherited`.
    async fn endpoint_trust(
        &self,
        endpoint: EndpointId,
        direction: TrustDirection,
    ) -> Result<EndpointTrustView, TransportError> {
        self.log(FakeTrustCall::EndpointRead(endpoint.clone(), direction));
        self.require(AdminCapability::Trust)?;
        let state = self.node.running()?;
        if !state.endpoints.contains_key(&endpoint) {
            return Err(TransportError::EndpointUnknown);
        }
        Ok(match state.own.get(&endpoint) {
            Some(own) => EndpointTrustView {
                set: EndpointSetKind::Own,
                allowed: own
                    .member(direction)
                    .iter()
                    .map(|peer| TrustedPeer {
                        peer: peer.clone(),
                        persisted: true,
                        source: TrustSource::Administered,
                    })
                    .collect(),
            },
            None => EndpointTrustView {
                set: EndpointSetKind::Inherited,
                allowed: operator_rows(&state),
            },
        })
    }

    /// The runtime's order: the endpoint, the local peer, the bounds,
    /// then `superseded` keyed by the endpoint and the peer together. The
    /// first write copies what the endpoint admitted into both members,
    /// then changes both, a no-op included. A peer the write takes out of
    /// the last set that held it is disconnected as an operator revoke's
    /// is; one another set still holds is not.
    async fn set_endpoint_trust(
        &self,
        endpoint: EndpointId,
        peer: TransportIdentity,
        allowed: bool,
    ) -> Result<TrustChange, TransportError> {
        self.log(FakeTrustCall::EndpointSet(
            endpoint.clone(),
            peer.clone(),
            allowed,
        ));
        self.require(AdminCapability::Trust)?;
        let paired = self.node.remote().is_ok_and(|remote| remote.peer == peer);
        let mut state = self.node.running()?;
        if !state.endpoints.contains_key(&endpoint) {
            return Err(TransportError::EndpointUnknown);
        }
        if peer == self.node.peer {
            return Err(TransportError::InvalidArgument);
        }
        let mut own = state.own.get(&endpoint).cloned().unwrap_or_else(|| OwnSet {
            inbound: state.trusted.clone(),
            outbound: state.trusted.clone(),
        });
        for member in [&mut own.inbound, &mut own.outbound] {
            if allowed {
                member.insert(peer.clone());
            } else {
                member.remove(&peer);
            }
        }
        let before = state.admits(&peer);
        let previous = state.own.insert(endpoint.clone(), own);
        let within =
            state.own_rows() <= 4 * MAX_ALLOWED_PEERS && state.admitted() <= MAX_ALLOWED_PEERS;
        let key = format!("endpoint:{}:peer:{}", endpoint.as_str(), peer.as_str());
        let superseded = within && self.superseded(&state, &key);
        if !within || superseded {
            // Nothing written: the set as it was.
            match previous {
                Some(previous) => state.own.insert(endpoint, previous),
                None => state.own.remove(&endpoint),
            };
            return if superseded {
                Ok(TrustChange::Refused(TrustRefusal::Superseded))
            } else {
                Err(TransportError::InvalidArgument)
            };
        }
        self.applied(&mut state, key);
        if before && paired && !state.admits(&peer) {
            state.disconnect(&peer);
        }
        Ok(TrustChange::Set)
    }

    /// The first write's copy alone: `present`, nothing written, when the
    /// endpoint has a set, an empty one included.
    async fn seed_endpoint_trust(&self, endpoint: EndpointId) -> Result<TrustSeed, TransportError> {
        self.log(FakeTrustCall::Seed(endpoint.clone()));
        self.require(AdminCapability::Trust)?;
        let mut state = self.node.running()?;
        if !state.endpoints.contains_key(&endpoint) {
            return Err(TransportError::EndpointUnknown);
        }
        let seed = if state.own.contains_key(&endpoint) {
            TrustSeed::Present
        } else {
            if state.own_rows() + 2 * state.trusted.len() > 4 * MAX_ALLOWED_PEERS {
                return Err(TransportError::InvalidArgument);
            }
            let copy = OwnSet {
                inbound: state.trusted.clone(),
                outbound: state.trusted.clone(),
            };
            state.own.insert(endpoint.clone(), copy);
            TrustSeed::Seeded
        };
        state.seeds.push((endpoint, seed));
        Ok(seed)
    }

    /// The single add is the all-or-none add of one line, as in the
    /// runtime, so the two cannot answer a line apart.
    async fn add_bootstrap_address(
        &self,
        address: String,
    ) -> Result<AddressChange, TransportError> {
        Ok(match self.add_bootstrap_addresses(vec![address]).await? {
            AddressesChange::Added(lines) => match lines.as_slice() {
                [LineChange::Added] => AddressChange::Added,
                _ => AddressChange::Unchanged,
            },
            AddressesChange::Refused { reason, detail, .. } => {
                AddressChange::Refused { reason, detail }
            }
        })
    }

    /// The bootstrap add's rules by name, in the runtime's order, the
    /// whole list checked before anything is kept: every line against the
    /// configuration's entry rule (the same function the runtime's add
    /// reads); then every line's `PeerId` against this node's own; then
    /// every line against the special-use floor on every IP
    /// component; then the bound of 64 authored and added together,
    /// counting only new lines; then every line against the last applied
    /// write's admission (`superseded`). A line present already, or
    /// repeated, changes nothing, and still orders. The fake holds no
    /// `enabled: false` provider.
    async fn add_bootstrap_addresses(
        &self,
        addresses: Vec<String>,
    ) -> Result<AddressesChange, TransportError> {
        self.require(AdminCapability::Peers)?;
        if addresses.is_empty() || addresses.len() > MAX_ADDRESSES_PER_ADD {
            return Err(TransportError::InvalidArgument);
        }
        let mut state = self.node.running()?;
        let refused = |reason, line, detail: &str| {
            Ok(AddressesChange::Refused {
                reason,
                line: Some(line),
                detail: detail.to_owned(),
            })
        };
        for (line, address) in addresses.iter().enumerate() {
            if let Err(why) = interweave_profile_config::bootstrap_overlay::entry_grammar(address) {
                return refused(AddressRefusal::Grammar, line, why);
            }
        }
        for (line, address) in addresses.iter().enumerate() {
            if interweave_profile_config::split_peer_multiaddr(address)
                .is_ok_and(|(_, peer)| peer == self.node.peer)
            {
                return refused(AddressRefusal::OwnPeer, line, "this node's own PeerId");
            }
        }
        for (line, address) in addresses.iter().enumerate() {
            if special_use(address) {
                return refused(AddressRefusal::SpecialUse, line, "a special-use component");
            }
        }
        let mut added = state.added.clone();
        let mut lines = Vec::with_capacity(addresses.len());
        for (line, address) in addresses.iter().enumerate() {
            if state.authored.iter().chain(&added).any(|a| a == address) {
                lines.push(LineChange::Unchanged);
                continue;
            }
            if state.authored.len() + added.len() >= 64 {
                return refused(
                    AddressRefusal::Bound,
                    line,
                    "authored and added would pass 64",
                );
            }
            added.push(address.clone());
            lines.push(LineChange::Added);
        }
        let keys: Vec<String> = addresses.iter().map(|a| format!("address:{a}")).collect();
        if let Some(line) = keys.iter().position(|key| self.superseded(&state, key)) {
            return refused(
                AddressRefusal::Superseded,
                line,
                "a later port's write stands",
            );
        }
        for key in keys {
            self.applied(&mut state, key);
        }
        state.added = added;
        Ok(AddressesChange::Added(lines))
    }

    async fn remove_bootstrap_address(
        &self,
        address: String,
    ) -> Result<AddressChange, TransportError> {
        self.require(AdminCapability::Peers)?;
        let mut state = self.node.running()?;
        if state.authored.contains(&address) {
            return Ok(AddressChange::Refused {
                reason: AddressRefusal::Authored,
                detail: "config.yaml's".to_owned(),
            });
        }
        let key = format!("address:{address}");
        if self.superseded(&state, &key) {
            return Ok(AddressChange::Refused {
                reason: AddressRefusal::Superseded,
                detail: "a later port's write stands".to_owned(),
            });
        }
        self.applied(&mut state, key);
        let before = state.added.len();
        state.added.retain(|a| *a != address);
        Ok(if state.added.len() == before {
            AddressChange::Unchanged
        } else {
            AddressChange::Removed
        })
    }

    async fn bootstrap_addresses(&self) -> Result<Vec<BootstrapAddress>, TransportError> {
        self.require(AdminCapability::Peers)?;
        let state = self.node.running()?;
        let row = |address: &String, source| BootstrapAddress {
            address: address.clone(),
            source,
            dialled: true,
        };
        Ok(state
            .authored
            .iter()
            .map(|a| row(a, AddressSource::Authored))
            .chain(state.added.iter().map(|a| row(a, AddressSource::Added)))
            .collect())
    }

    /// One listener on a private address, as a node on a LAN shows: the
    /// fake binds no socket, so the address is this node's by
    /// configuration, and nothing is confirmed external.
    async fn own_addresses(&self) -> Result<OwnAddresses, TransportError> {
        self.require(AdminCapability::Peers)?;
        drop(self.node.running()?);
        let host = if self.node.tag == "a" { 10 } else { 11 };
        Ok(OwnAddresses {
            peer: self.node.peer.clone(),
            addresses: vec![OwnAddress {
                address: format!(
                    "/ip4/192.168.77.{host}/tcp/4001/p2p/{}",
                    self.node.peer.as_str()
                ),
                kind: OwnAddressKind::Listen,
            }],
            truncated: false,
        })
    }
}

/// The operator's list as `admin.trust.list` answers it: each peer with
/// its source.
fn operator_rows(state: &State) -> Vec<TrustedPeer> {
    state
        .trusted
        .iter()
        .map(|peer| TrustedPeer {
            peer: peer.clone(),
            persisted: true,
            source: if state.configured.contains(peer) {
                TrustSource::Configured
            } else {
                TrustSource::Administered
            },
        })
        .collect()
}

/// Whether an address carries an IP component in a class the add's floor
/// refuses (ADR-0052 rule 2): loopback, unspecified, link-local and
/// multicast, of either family. Private ranges are admitted. The real
/// floor names more classes; the suite's cases need these.
fn special_use(address: &str) -> bool {
    let parts: Vec<&str> = address.split('/').collect();
    parts.windows(2).any(|pair| match pair[0] {
        "ip4" => pair[1].parse::<std::net::Ipv4Addr>().is_ok_and(|ip| {
            ip.is_loopback() || ip.is_unspecified() || ip.is_link_local() || ip.is_multicast()
        }),
        "ip6" => pair[1].parse::<std::net::Ipv6Addr>().is_ok_and(|ip| {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || ip.segments()[0] & 0xffc0 == 0xfe80
        }),
        _ => false,
    })
}

/// `PeerTrustPolicy::MAX_ALLOWED_PEERS`, which this crate does not depend
/// on: the same ceiling, so a client sees the same refusal.
const MAX_ALLOWED_PEERS: usize = 4096;

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::{
        AdminBinding as _, AdminCapability, AdminPort as _, DataCapability,
        DataSessionBinding as _, DataSessionPort as _, EndpointSetKind, FakeConfig, FakeEndpoint,
        FakeNetwork, FakeNode, FakeTrustCall, LocalSessionEvent, SessionEvent, SessionRequest,
        TrustChange, TrustDirection, TrustRefusal, TrustSeed,
    };
    use interweave_test_support::endpoint_keys::endpoint_id;
    use interweave_transport_api::{
        DirectDestination, EndpointId, MessageId, Payload, TransportError, TransportIdentity,
    };

    /// Every fake call here is ready at its first poll: no executor.
    fn now<F: std::future::Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
        {
            std::task::Poll::Ready(out) => out,
            std::task::Poll::Pending => unreachable!("the fake never waits here"),
        }
    }

    fn config(peer: &str) -> FakeConfig {
        FakeConfig {
            peer: TransportIdentity::parse(peer).expect("a peer id"),
            endpoints: vec![
                FakeEndpoint::open(endpoint_id("human"), false),
                FakeEndpoint::open(endpoint_id("agent"), true),
            ],
            default_endpoint: Some(endpoint_id("human")),
            queue_bound: 8,
        }
    }

    fn pair() -> (FakeNode, FakeNode) {
        FakeNetwork::pair(
            config("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN"),
            config("QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N"),
        )
    }

    /// A peer neither node is.
    fn other() -> TransportIdentity {
        TransportIdentity::parse("12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr")
            .expect("a peer id")
    }

    fn payload() -> Payload {
        Payload::at_ceiling(None, b"x".to_vec()).expect("within the ceiling")
    }

    fn peers(node: &FakeNode, endpoint: &EndpointId, direction: TrustDirection) -> Vec<String> {
        let admin = now(node.admin([AdminCapability::Trust].into())).expect("a port");
        now(admin.endpoint_trust(endpoint.clone(), direction))
            .expect("read")
            .allowed
            .into_iter()
            .map(|row| row.peer.as_str().to_owned())
            .collect()
    }

    /// The seed copies the operator's list into both members once; the
    /// next is `present` and copies nothing, though the operator's list
    /// has changed since; an unknown endpoint is `EndpointUnknown`.
    #[test]
    fn a_seed_copies_once_and_then_answers_present() {
        let (a, b) = pair();
        let human = endpoint_id("human");
        let admin = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        let before =
            now(admin.endpoint_trust(human.clone(), TrustDirection::Inbound)).expect("read");
        assert_eq!(before.set, EndpointSetKind::Inherited);
        assert_eq!(
            now(admin.seed_endpoint_trust(human.clone())),
            Ok(TrustSeed::Seeded)
        );
        assert_eq!(
            now(admin.set_trust(other(), true)),
            Ok(TrustChange::Set),
            "the operator's list grows after the seed"
        );
        assert_eq!(
            now(admin.seed_endpoint_trust(human.clone())),
            Ok(TrustSeed::Present)
        );
        for direction in TrustDirection::ALL {
            let view = now(admin.endpoint_trust(human.clone(), direction)).expect("read");
            assert_eq!(view.set, EndpointSetKind::Own);
            assert_eq!(
                peers(&a, &human, direction),
                vec![b.peer().as_str().to_owned()],
                "the copy, and not the later operator's peer"
            );
        }
        assert_eq!(
            a.seeds(),
            vec![
                (human.clone(), TrustSeed::Seeded),
                (human, TrustSeed::Present)
            ]
        );
        assert_eq!(
            now(admin.seed_endpoint_trust(endpoint_id("nobody"))),
            Err(TransportError::EndpointUnknown)
        );
        assert_eq!(a.seeds().len(), 2, "a refused seed is not listed");
    }

    /// An endpoint write changes that endpoint's set alone: the
    /// operator's list and the other endpoint keep the peer, so a revoke
    /// closes nothing, and the peer's route to the endpoint is refused
    /// both ways while the other endpoint still receives it. The
    /// operator's revoke after it lets go of the last set, and closes.
    #[test]
    fn an_endpoint_write_leaves_the_operators_list() {
        let (a, b) = pair();
        let (human, agent) = (endpoint_id("human"), endpoint_id("agent"));
        let admin = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        let calls = a.operator_trust_calls();
        let request = |endpoint: &EndpointId| {
            SessionRequest::new(
                "test",
                Some(endpoint.clone()),
                [DataCapability::Commands, DataCapability::Events],
            )
            .expect("in bounds")
        };
        let at_human = now(a.open(request(&human))).expect("leases");
        let at_agent = now(a.open(request(&agent))).expect("leases");
        let from_b = now(b.open(request(&human))).expect("leases");
        assert_eq!(
            now(admin.set_endpoint_trust(human.clone(), b.peer().clone(), false)),
            Ok(TrustChange::Set)
        );
        assert_eq!(
            a.operator_list(),
            vec![b.peer().clone()],
            "the operator's list"
        );
        assert_eq!(a.operator_trust_calls(), calls, "no operator's call");
        for direction in TrustDirection::ALL {
            assert!(peers(&a, &human, direction).is_empty(), "{direction:?}");
            assert_eq!(
                peers(&a, &agent, direction),
                vec![b.peer().as_str().to_owned()]
            );
        }
        let send = |to: &EndpointId, n: u8| {
            now(from_b.send_direct(
                DirectDestination {
                    peer: a.peer().clone(),
                    endpoint: Some(to.clone()),
                },
                MessageId::from_bytes([n; 16]),
                payload(),
            ))
        };
        assert_eq!(
            send(&human, 1),
            Err(TransportError::RemoteEndpointUnavailable)
        );
        assert_eq!(send(&agent, 2), Ok(agent.clone()));
        let reply = now(at_human.send_direct(
            DirectDestination {
                peer: b.peer().clone(),
                endpoint: Some(human.clone()),
            },
            MessageId::from_bytes([3; 16]),
            payload(),
        ));
        assert_eq!(reply, Err(TransportError::UnauthorizedPeer), "outbound");
        let disconnected = |events: Vec<SessionEvent>| {
            events.iter().any(|e| {
                matches!(
                    e,
                    SessionEvent::Local(LocalSessionEvent::PeerDisconnected { .. })
                )
            })
        };
        assert!(
            !disconnected(now(at_agent.events(16)).expect("reads")),
            "another set holds the peer: nothing closes"
        );
        assert_eq!(
            now(admin.set_trust(b.peer().clone(), false)),
            Ok(TrustChange::Set)
        );
        assert!(disconnected(now(at_agent.events(16)).expect("reads")));
    }

    /// The operator's revoke of a peer an endpoint's own set holds closes
    /// nothing: the endpoint still reaches it, and it the endpoint.
    #[test]
    fn an_operator_revoke_closes_nothing_an_endpoint_set_holds() {
        let (a, b) = pair();
        let human = endpoint_id("human");
        let admin = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        assert_eq!(
            now(admin.seed_endpoint_trust(human.clone())),
            Ok(TrustSeed::Seeded)
        );
        let session = now(a.open(
            SessionRequest::new(
                "test",
                Some(human.clone()),
                [DataCapability::Commands, DataCapability::Events],
            )
            .expect("in bounds"),
        ))
        .expect("leases");
        let _ = now(session.events(16)).expect("reads");
        assert_eq!(
            now(admin.set_trust(b.peer().clone(), false)),
            Ok(TrustChange::Set)
        );
        assert!(a.operator_list().is_empty());
        let told = now(session.events(16)).expect("reads");
        assert!(
            !told.iter().any(|e| matches!(
                e,
                SessionEvent::Local(LocalSessionEvent::PeerDisconnected { .. })
            )),
            "{told:?}"
        );
        let _held = now(b.open(
            SessionRequest::new("test", Some(human.clone()), [DataCapability::Events])
                .expect("in bounds"),
        ))
        .expect("leases");
        let sent = now(session.send_direct(
            DirectDestination {
                peer: b.peer().clone(),
                endpoint: Some(human),
            },
            MessageId::from_bytes([4; 16]),
            payload(),
        ));
        assert!(sent.is_ok(), "{sent:?}");
    }

    /// The `n`th of as many distinct well-formed peers as a bound needs.
    fn peer_n(n: usize) -> TransportIdentity {
        let mut bytes = vec![0x00, 0x24, 0x08, 0x01, 0x12, 0x20];
        bytes.extend_from_slice(&[7; 24]);
        bytes.extend_from_slice(&u64::try_from(n).expect("small").to_be_bytes());
        TransportIdentity::parse(bs58::encode(bytes).into_string()).expect("a peer id")
    }

    fn lease(node: &FakeNode, endpoint: &EndpointId) -> super::FakeSession {
        now(node.open(
            SessionRequest::new(
                "test",
                Some(endpoint.clone()),
                [DataCapability::Commands, DataCapability::Events],
            )
            .expect("in bounds"),
        ))
        .expect("leases")
    }

    fn disconnected(events: &[SessionEvent]) -> bool {
        events.iter().any(|e| {
            matches!(
                e,
                SessionEvent::Local(LocalSessionEvent::PeerDisconnected { .. })
            )
        })
    }

    /// The directory is what the querier may send to: an endpoint whose
    /// own inbound set refuses it is left out, as the runtime's
    /// `advertised_for` leaves it out.
    #[test]
    fn the_directory_leaves_out_an_endpoint_refusing_the_querier() {
        let (a, b) = pair();
        let agent = endpoint_id("agent");
        let admin = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        let _leased = lease(&a, &agent);
        let from_b = now(b.open(
            SessionRequest::new(
                "test",
                Some(endpoint_id("human")),
                [DataCapability::EndpointsQuery],
            )
            .expect("in bounds"),
        ))
        .expect("leases");
        let listed = |session: &super::FakeSession| {
            now(session.query_endpoints(a.peer().clone()))
                .expect("directory")
                .endpoints
        };
        assert_eq!(listed(&from_b), vec![agent.clone()]);
        assert_eq!(
            now(admin.set_endpoint_trust(agent, b.peer().clone(), false)),
            Ok(TrustChange::Set)
        );
        assert!(listed(&from_b).is_empty());
    }

    /// The peers rows are the admission set's: a peer only an endpoint's
    /// own set holds is still listed after the operator's revoke.
    #[test]
    fn the_peer_rows_are_the_admission_set() {
        let (a, b) = pair();
        let admin =
            now(a.admin([AdminCapability::Trust, AdminCapability::Status].into())).expect("a port");
        let _ = now(admin.seed_endpoint_trust(endpoint_id("human"))).expect("seeds");
        assert_eq!(
            now(admin.set_trust(b.peer().clone(), false)),
            Ok(TrustChange::Set)
        );
        assert!(a.operator_list().is_empty());
        let rows = now(admin.peers()).expect("rows");
        assert_eq!(
            rows.iter().map(|r| r.peer.clone()).collect::<Vec<_>>(),
            vec![b.peer().clone()]
        );
    }

    /// The operator's allow is bounded by the admission set, which every
    /// endpoint's own set shares: a peer outside it is refused at the
    /// bound, one already inside is not, and nothing is written.
    #[test]
    fn an_operator_allow_is_bounded_by_the_admission_set() {
        let (a, _b) = pair();
        let admin = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        a.give_own_set(
            endpoint_id("human"),
            // One short of the bound: the pair's other node is the 4096th.
            (0..super::MAX_ALLOWED_PEERS - 1).map(peer_n),
            [],
        );
        assert_eq!(
            now(admin.set_trust(other(), true)),
            Err(TransportError::InvalidArgument)
        );
        assert!(!a.operator_list().contains(&other()));
        assert_eq!(now(admin.set_trust(peer_n(5), true)), Ok(TrustChange::Set));
        assert!(a.operator_list().contains(&peer_n(5)));
    }

    /// An endpoint write's refusals in the runtime's order: the endpoint,
    /// the local peer, the bounds, then superseded, nothing written.
    #[test]
    fn an_endpoint_write_refuses_in_the_runtimes_order() {
        let (a, b) = pair();
        let (human, agent) = (endpoint_id("human"), endpoint_id("agent"));
        let early = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        let late = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        let local = a.peer().clone();
        assert_eq!(
            now(late.set_endpoint_trust(endpoint_id("nobody"), local.clone(), true)),
            Err(TransportError::EndpointUnknown),
            "the endpoint before the local peer"
        );
        assert_eq!(
            now(late.set_endpoint_trust(human.clone(), local.clone(), true)),
            Err(TransportError::InvalidArgument)
        );
        assert_eq!(
            now(late.set_trust(local, true)),
            Err(TransportError::InvalidArgument),
            "the operator's list refuses the local peer too"
        );
        assert_eq!(
            peers(&a, &human, TrustDirection::Inbound),
            vec![b.peer().as_str().to_owned()],
            "nothing written"
        );
        // Superseded is keyed by the endpoint and the peer together.
        assert_eq!(
            now(late.set_endpoint_trust(human.clone(), b.peer().clone(), false)),
            Ok(TrustChange::Set)
        );
        assert_eq!(
            now(early.set_endpoint_trust(human.clone(), b.peer().clone(), true)),
            Ok(TrustChange::Refused(TrustRefusal::Superseded))
        );
        assert!(peers(&a, &human, TrustDirection::Inbound).is_empty());
        assert_eq!(
            now(early.set_endpoint_trust(agent.clone(), b.peer().clone(), false)),
            Ok(TrustChange::Set),
            "another endpoint is another key"
        );
        // The bound before superseded: both hold for this write. The
        // admission set is exactly full (b is the 4096th), so the
        // setup stays inside the bound and only the write crosses it.
        assert_eq!(
            now(late.set_endpoint_trust(human.clone(), other(), false)),
            Ok(TrustChange::Set)
        );
        a.give_own_set(agent, (0..super::MAX_ALLOWED_PEERS - 1).map(peer_n), []);
        assert_eq!(
            now(early.set_endpoint_trust(human.clone(), other(), true)),
            Err(TransportError::InvalidArgument)
        );
        assert!(!peers(&a, &human, TrustDirection::Inbound).contains(&other().as_str().to_owned()));
    }

    /// Three endpoints, so own rows can reach their bound with the
    /// admission set inside its own: two endpoints hold 4 x 4096 rows
    /// only by holding every admitted peer in every member.
    fn three_endpoints() -> (FakeNode, FakeNode) {
        let with_third = |peer: &str| {
            let mut c = config(peer);
            c.endpoints
                .push(FakeEndpoint::open(endpoint_id("third"), false));
            c
        };
        FakeNetwork::pair(
            with_third("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN"),
            with_third("QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N"),
        )
    }

    /// `agent` and `third` each hold peers 0..4095 in both members:
    /// 16380 own rows, and with the pair's other node an admission set of
    /// exactly 4096. `human` has no set.
    fn fill_two_sets(a: &FakeNode) {
        for name in ["agent", "third"] {
            a.give_own_set(
                endpoint_id(name),
                (0..super::MAX_ALLOWED_PEERS - 1).map(peer_n),
                (0..super::MAX_ALLOWED_PEERS - 1).map(peer_n),
            );
        }
    }

    /// The own rows are bounded at 4 x 4096, both members counted: the
    /// write that lands on exactly the bound is answered, the next, which
    /// admits no new peer, is refused and writes nothing.
    #[test]
    fn an_endpoint_write_is_bounded_by_the_own_rows() {
        let (a, _b) = three_endpoints();
        let human = endpoint_id("human");
        fill_two_sets(&a);
        let admin = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        // human's copy of the operator's list (1 peer) plus peer 5, in
        // both members: 16380 + 4 rows, the bound exactly.
        assert_eq!(
            now(admin.set_endpoint_trust(human.clone(), peer_n(5), true)),
            Ok(TrustChange::Set)
        );
        assert_eq!(peers(&a, &human, TrustDirection::Inbound).len(), 2);
        assert_eq!(
            now(admin.set_endpoint_trust(human.clone(), peer_n(6), true)),
            Err(TransportError::InvalidArgument)
        );
        assert_eq!(peers(&a, &human, TrustDirection::Inbound).len(), 2);
    }

    /// The seed's copy counts against the same own-row bound: the
    /// operator's list copied twice must fit, to the row, and a refused
    /// seed leaves the endpoint without a set.
    #[test]
    fn a_seed_is_bounded_by_the_own_rows() {
        let (a, _b) = three_endpoints();
        let human = endpoint_id("human");
        fill_two_sets(&a);
        let admin = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        // Peers the other sets already admit: the operator's list grows
        // to three, 16380 + 2 x 3 rows, over the bound.
        for n in [0, 1] {
            assert_eq!(now(admin.set_trust(peer_n(n), true)), Ok(TrustChange::Set));
        }
        assert_eq!(
            now(admin.seed_endpoint_trust(human.clone())),
            Err(TransportError::InvalidArgument)
        );
        assert!(a.seeds().is_empty());
        let view = now(admin.endpoint_trust(human.clone(), TrustDirection::Inbound)).expect("read");
        assert_eq!(view.set, EndpointSetKind::Inherited);
        // Two peers: 16380 + 4, the bound exactly, is seeded.
        assert_eq!(now(admin.set_trust(peer_n(1), false)), Ok(TrustChange::Set));
        assert_eq!(now(admin.seed_endpoint_trust(human)), Ok(TrustSeed::Seeded));
    }

    /// The local peer is refused whatever the direction, before any
    /// bound is looked at and with nothing written: with the sets full, a
    /// removal of it is refused, not answered as the no-op it would be
    /// inside the bounds.
    #[test]
    fn the_local_peer_is_refused_before_the_bounds() {
        let (a, _b) = three_endpoints();
        let human = endpoint_id("human");
        fill_two_sets(&a);
        let admin = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        for allowed in [false, true] {
            assert_eq!(
                now(admin.set_endpoint_trust(human.clone(), a.peer().clone(), allowed)),
                Err(TransportError::InvalidArgument),
                "allowed {allowed}"
            );
        }
        let view = now(admin.endpoint_trust(human, TrustDirection::Inbound)).expect("read");
        assert_eq!(view.set, EndpointSetKind::Inherited);
    }

    /// An endpoint write that takes the paired node out of the last set
    /// holding it closes the connection, as an operator's revoke does.
    #[test]
    fn an_endpoint_write_that_empties_the_last_set_disconnects() {
        let (a, b) = pair();
        let human = endpoint_id("human");
        let admin = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        let _ = now(admin.seed_endpoint_trust(human.clone())).expect("seeds");
        let session = lease(&a, &human);
        let _ = now(session.events(16)).expect("reads");
        assert_eq!(
            now(admin.set_trust(b.peer().clone(), false)),
            Ok(TrustChange::Set)
        );
        assert!(!disconnected(&now(session.events(16)).expect("reads")));
        assert_eq!(
            now(admin.set_endpoint_trust(human, b.peer().clone(), false)),
            Ok(TrustChange::Set)
        );
        assert!(disconnected(&now(session.events(16)).expect("reads")));
    }

    /// Every trust call is counted and logged, answered or refused; the
    /// operator's count is the operator's calls alone.
    #[test]
    fn trust_calls_are_counted_and_logged() {
        let (a, b) = pair();
        let human = endpoint_id("human");
        let admin = now(a.admin([AdminCapability::Trust].into())).expect("a port");
        let _ = now(admin.trust());
        let _ = now(admin.set_trust(b.peer().clone(), true));
        let _ = now(admin.endpoint_trust(human.clone(), TrustDirection::Inbound));
        let _ = now(admin.seed_endpoint_trust(human.clone()));
        let _ = now(admin.set_endpoint_trust(human.clone(), b.peer().clone(), true));
        assert_eq!(a.operator_trust_calls(), 2);
        let log: Vec<FakeTrustCall> = a.trust_log().into_iter().map(|(_, c)| c).collect();
        assert_eq!(
            log,
            vec![
                FakeTrustCall::OperatorRead,
                FakeTrustCall::OperatorSet(b.peer().clone(), true),
                FakeTrustCall::EndpointRead(human.clone(), TrustDirection::Inbound),
                FakeTrustCall::Seed(human.clone()),
                FakeTrustCall::EndpointSet(human, b.peer().clone(), true),
            ]
        );
    }
}
