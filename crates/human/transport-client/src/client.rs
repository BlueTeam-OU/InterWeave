// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade: the client's half of retention over any
//! [`DataSessionBinding`], with the admin `Status` connection beside it
//! for connectivity (plan §17 (1), (5)).
//!
//! Poll-driven, on purpose: [`TransportClient::tick`] runs what is due,
//! and nothing is spawned -- so the same code runs in-process on Android,
//! which has no shell loop to drive one, and a test replays a schedule
//! exactly.
//!
//! TWO CLOCKS (agreed amendment A3). Every method's `now` is the
//! caller's MONOTONIC clock, and drives schedules only: when a retry, a
//! re-open or a re-check is due. What is persisted or sent -- a row's
//! creation and receipt times, an attempt's time, a broadcast's
//! `sent_at_ms` -- comes from the wall clock the constructor is given,
//! in Unix ms, because the store orders rows by those fields and a
//! monotonic clock restarts at boot.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::FutureExt as _;
use futures::stream::{FuturesUnordered, StreamExt as _};

use interweave_human_chat_protocol::{
    CardRoute, Classified, HumanChatV2, classify, decode_envelope_bytes, encode_outbound,
    encode_raw_envelope, is_card, parse_media_type,
};
use interweave_human_store::{
    AppMessageId, Cursor, HumanStore, InboundOrigin, NewInbound, NewOutbound, OutboundDestination,
    PageLimits, PendingOutbound, RowId, StorageHealth, StoreError, TerminalCause,
};
use interweave_local_client_api::{
    AddressChange, AddressRefusal, AddressSource, AddressesChange, AdminBinding, AdminCapability,
    AdminPort, DataCapability, DataSessionBinding, DataSessionPort, LineChange, LocalSessionEvent,
    SessionEvent, SessionRequest, TrustChange, TrustRefusal,
};
use interweave_transport_api::{
    BroadcastMessageV1, ChannelId, ConnectivitySummary, DirectDestination, DirectInboundState,
    EndpointId, Health, MediaType, MessageId, PathReadiness, Payload, TransportError,
    TransportIdentity,
};

use crate::backoff::{RECHECK, REOPEN, SEND};
use crate::problem::{
    AttemptFailure, OpenFailure, classify_address, classify_open, classify_send, classify_trust,
    ends_session, may_have_reached,
};
use crate::queue::{Capped, EventQueue};
use interweave_human_client_api::{
    AddressAnswer, AddressChangeFailure, AddressList, AddressOrigin, AddressOutcome,
    AddressProblem, AddressRefused, ClientEvent, Connectivity, Destination, Diagnostics, Inbound,
    LineAdded, Origin, OutboundStatus, OutboundUpdate, PeerAddress, Received, ReceivedCard,
    RowError, SendError, SendProblem, SessionProblem, SessionState, ShowAddresses, TrustList,
    TrustOrigin, TrustProblem, TrustRow, TrustSetFailure,
};

/// How many committed messages wait for hand-over at most: one session
/// queue's ceiling. Past it a message stays unread in the store, is
/// counted (`Diagnostics::held_overflow`), and the caller is told to
/// re-list unread (`ClientEvent::UnreadInStore`); the facade test
/// `a_snapshot_past_the_cap_is_kept_in_the_store_and_announced` pins it.
const HELD_CAP: usize = interweave_local_client_api::MAX_EVENT_QUEUE;

/// How many send attempts may be on the wire at once, across every
/// destination: one per wire slot (`Flight`), and this many in all,
/// under the 64 commands an IPC client may have outstanding
/// (`TRANSPORT.md` §Backpressure) so that its event reads and admin
/// calls keep room (architect-cto's rulings 01a1e901 and 01a1e96d, j288).
/// `a_silent_peer_holds_no_send_to_another_peer` pins the per-slot half,
/// `at_most_the_flight_cap_of_sends_is_in_flight` this one.
pub(crate) const FLIGHT_CAP: usize = 32;

/// The wire slot a send attempt holds: a peer (whichever of its
/// endpoints), or a channel. One attempt on the wire per slot; slots go
/// beside each other, so a peer that never answers holds no line to
/// another (architect-cto's ruling 01a1e901).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Flight {
    Peer(TransportIdentity),
    Channel(ChannelId),
}

/// The order unit: a conversation, the peer and the endpoint as the
/// destination names it (no endpoint, the peer's default, is its own
/// unit, never merged with the endpoint it may turn out to be), or a
/// channel. A conversation whose earliest pending row is on the wire or
/// waiting out its backoff takes no later row of its own; another
/// conversation with the same peer goes on (architect-cto's ruling
/// 01a1e96d, replacing 01a1e901's per-peer unit).
/// `a_row_waiting_its_backoff_holds_the_later_row_to_its_peer` and
/// `an_endpoint_that_never_takes_holds_only_its_own_conversation` pin it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Order {
    Conversation(TransportIdentity, Option<EndpointId>),
    Channel(ChannelId),
}

impl Order {
    fn of(destination: &OutboundDestination) -> Self {
        match destination {
            OutboundDestination::Direct(d) => {
                Self::Conversation(d.peer.clone(), d.endpoint.clone())
            }
            OutboundDestination::Broadcast(c) => Self::Channel(c.clone()),
        }
    }

    fn flight(&self) -> Flight {
        match self {
            Self::Conversation(peer, _) => Flight::Peer(peer.clone()),
            Self::Channel(c) => Flight::Channel(c.clone()),
        }
    }
}

/// Whether a tick's pass could attempt anything: the flights are under
/// `FLIGHT_CAP`, and some conversation's head -- its earliest pending row
/// not held for attention -- is due, not on the wire, and its slot is
/// free. A row behind a head waiting
/// out its backoff stays due and attempts nothing, so a guard that asked
/// only "is any row due" would read the whole pending table every tick
/// while a peer is unreachable (`a_pass_is_due_only_for_a_head`).
fn attemptable(rows: &BTreeMap<RowId, Row>, flying: &BTreeMap<RowId, Flight>, now: u64) -> bool {
    if flying.len() >= FLIGHT_CAP {
        return false;
    }
    let mut heads: BTreeMap<&Order, (u64, RowId)> = BTreeMap::new();
    for (row_id, row) in rows {
        if row.attention.is_some() {
            continue;
        }
        let key = (row.created_at, *row_id);
        heads
            .entry(&row.order)
            .and_modify(|head| *head = (*head).min(key))
            .or_insert(key);
    }
    heads.into_iter().any(|(order, (_, row_id))| {
        let slot = order.flight();
        rows.get(&row_id).is_some_and(|r| r.is_due(now))
            && !flying.contains_key(&row_id)
            && !flying.values().any(|f| *f == slot)
    })
}

/// What a send attempt answers: how the row ends, and the endpoint that
/// accepted a direct one.
type Landing = Result<(TerminalCause, Option<EndpointId>), TransportError>;

/// One attempt on the wire, owning what it needs: the session shared, the
/// row it is for.
type InFlight = Pin<Box<dyn Future<Output = (RowId, Landing)> + Send>>;

/// How often a healthy admin connection is asked for status.
const STATUS_INTERVAL_MS: u64 = 5_000;

/// The wall clock in Unix milliseconds.
pub type WallClock = Box<dyn Fn() -> u64 + Send + Sync>;

/// What the facade is configured with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientConfig {
    /// The client kind the session declares.
    pub client_kind: String,
    /// The endpoints this client may lease, in the order it prefers them;
    /// empty for a session that sends only broadcasts and receives none
    /// directly. Each open leases the first the runtime does not refuse
    /// as disabled: whether an endpoint is enabled is the runtime's
    /// effective state, which an administrator's change kept across
    /// restarts makes differ from the profile's file (ADR-0028 A
    /// 2026-10-11), so the client never decides it from configuration.
    pub endpoints: Vec<EndpointId>,
    /// The channels to join while the session is ready: the only ones a
    /// broadcast may go to.
    pub channels: Vec<ChannelId>,
    /// The transport's effective payload limit for an outbound message.
    pub max_payload_bytes: usize,
}

impl ClientConfig {
    /// Whether a session opened from this configuration can make a send
    /// to `destination` at all (agreed amendment A2).
    fn allows(&self, destination: &OutboundDestination) -> bool {
        match destination {
            OutboundDestination::Direct(_) => !self.endpoints.is_empty(),
            OutboundDestination::Broadcast(channel) => self.channels.contains(channel),
        }
    }
}

#[derive(Debug, Clone)]
struct Row {
    app_message_id: AppMessageId,
    /// The conversation it is in order within, and its place there: the
    /// store pages pending rows by `(created_at, row_id)`.
    order: Order,
    created_at: u64,
    attempts: u32,
    /// `Some` while the facade will not retry on its own.
    attention: Option<SendProblem>,
    next_at: Option<u64>,
    last_problem: Option<SendProblem>,
    last_code: Option<TransportError>,
    /// One-way: once true, true until terminal (agreed amendment A1a).
    may_have_reached: bool,
}

impl Row {
    fn is_due(&self, now: u64) -> bool {
        self.attention.is_none() && self.next_at.is_some_and(|at| at <= now)
    }

    fn status(&self) -> OutboundStatus {
        match (self.attention, self.may_have_reached) {
            (Some(problem), may_have_reached) => OutboundStatus::NeedsAttention {
                problem,
                may_have_reached,
            },
            (None, true) => OutboundStatus::Unconfirmed {
                next_retry_at: self.next_at,
                last_problem: self.last_problem,
            },
            (None, false) => OutboundStatus::Sending {
                attempts: self.attempts,
                next_retry_at: self.next_at,
                last_problem: self.last_problem,
            },
        }
    }
}

/// A pending record an explicit cancel deleted, held so the person can
/// take the cancel back ([`TransportClient::restore`]). Opaque: it carries
/// the stored bytes, and only the facade that cancelled it reads it.
pub struct CancelledSend {
    record: PendingOutbound,
    may_have_reached: bool,
}

impl CancelledSend {
    /// The row the cancel ended.
    #[must_use]
    pub const fn row(&self) -> RowId {
        self.record.row_id
    }
}

impl std::fmt::Debug for CancelledSend {
    // Never the payload: a log line is not a place for content.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancelledSend")
            .field("row", &self.record.row_id)
            .field("attempts", &self.record.attempts)
            .field("may_have_reached", &self.may_have_reached)
            .finish_non_exhaustive()
    }
}

/// A binding whose sessions the facade's send attempts may share while
/// they are on the wire (each holds the session until it lands): every
/// [`DataSessionBinding`] whose session is `Send + Sync + 'static`, which
/// the platform bindings' are.
pub trait SessionBinding: DataSessionBinding<Session: Send + Sync + 'static> {}

impl<T> SessionBinding for T where T: DataSessionBinding<Session: Send + Sync + 'static> {}

/// The human client's transport facade.
pub struct TransportClient<B: DataSessionBinding, A: AdminBinding> {
    binding: B,
    admin_binding: A,
    store: HumanStore,
    config: ClientConfig,
    wall: WallClock,
    /// Shared with the attempts on the wire, which each hold it until they
    /// land; a close drops them first.
    session: Option<Arc<B::Session>>,
    /// The send attempts on the wire, each landed by a later tick.
    in_flight: FuturesUnordered<InFlight>,
    /// Each row on the wire, and the destination it holds.
    flying: BTreeMap<RowId, Flight>,
    /// The endpoint the last open leased: what a sent message names as
    /// its own until another open leases a different one.
    leased: Option<EndpointId>,
    admin: Option<A::Admin>,
    state: SessionState,
    /// Where a degrade came from, if it was `Refused`: a recovered store
    /// returns there rather than re-opening on a timer.
    refused_before_degrade: Option<SessionProblem>,
    reopen_attempt: u32,
    recheck_attempt: u32,
    next_recheck_at: u64,
    connectivity: Connectivity,
    admin_attempt: u32,
    next_status_at: u64,
    rows: BTreeMap<RowId, Row>,
    /// Inbound already committed but not yet handed over, because the
    /// facade took what a session held before closing it. Durable
    /// already; capped at [`HELD_CAP`].
    held: Capped<Inbound>,
    queue: EventQueue,
    diagnostics: Diagnostics,
}

impl<B: SessionBinding, A: AdminBinding> TransportClient<B, A> {
    /// A facade over `binding` and `admin_binding`, holding `store`, with
    /// `wall` the wall clock in Unix ms for what is persisted or sent.
    ///
    /// Opens nothing yet: the first [`tick`](Self::tick) does. Every row
    /// the store holds pending is reported once, due now (agreed item
    /// 1e), so a row that needed attention before a restart is attempted
    /// once more -- its peer's trust may have changed meanwhile.
    ///
    /// # Errors
    /// The store's, if its pending rows cannot be read.
    pub fn new(
        binding: B,
        admin_binding: A,
        store: HumanStore,
        config: ClientConfig,
        wall: WallClock,
        now: u64,
    ) -> Result<Self, StoreError> {
        let mut client = Self {
            binding,
            admin_binding,
            store,
            config,
            wall,
            session: None,
            in_flight: FuturesUnordered::new(),
            flying: BTreeMap::new(),
            leased: None,
            admin: None,
            state: SessionState::Reconnecting {
                attempt: 0,
                next_at: now,
            },
            refused_before_degrade: None,
            reopen_attempt: 0,
            recheck_attempt: 0,
            next_recheck_at: now,
            connectivity: Connectivity::Unknown,
            admin_attempt: 0,
            next_status_at: now,
            rows: BTreeMap::new(),
            held: Capped::new(HELD_CAP),
            queue: EventQueue::default(),
            diagnostics: Diagnostics::default(),
        };
        let mut after = None;
        loop {
            let page = client
                .store
                .pending_outbound_page(after, PageLimits::default())?;
            for pending in page.items {
                client.rows.insert(
                    pending.row_id,
                    Row {
                        app_message_id: pending.app_message_id.clone(),
                        order: Order::of(&pending.destination),
                        created_at: pending.created_at,
                        attempts: pending.attempts,
                        attention: None,
                        next_at: Some(now),
                        last_problem: None,
                        last_code: None,
                        // Attempted before this process: its outcomes are
                        // not known here (agreed amendment A1b).
                        may_have_reached: pending.attempts > 0,
                    },
                );
                client.report(pending.row_id);
            }
            match page.next {
                Some(next) => after = Some(next),
                None => break,
            }
        }
        let state = client.state.clone();
        client.queue.push(ClientEvent::Session(state));
        client
            .queue
            .push(ClientEvent::Connectivity(Connectivity::Unknown));
        Ok(client)
    }

    /// The session's state.
    #[must_use]
    pub const fn session_state(&self) -> &SessionState {
        &self.state
    }

    /// Connectivity as last read.
    #[must_use]
    pub const fn connectivity(&self) -> Connectivity {
        self.connectivity
    }

    /// The endpoint a message sent now names as its own: the one the last
    /// open leased, or before any lease the first this client prefers.
    /// A message queued while no session holds may leave on a different
    /// endpoint if the next open's walk lands elsewhere; the field is the
    /// sender's assertion, never the route (`HUMAN-CHAT.md`).
    #[must_use]
    pub fn endpoint(&self) -> Option<&EndpointId> {
        self.leased
            .as_ref()
            .or_else(|| self.config.endpoints.first())
    }

    /// The diagnostic counts.
    #[must_use]
    pub const fn diagnostics(&self) -> Diagnostics {
        self.diagnostics
    }

    /// The store, for the operations the facade does not own: read,
    /// keep and unkeep (agreed item 3).
    pub const fn store_mut(&mut self) -> &mut HumanStore {
        &mut self.store
    }

    /// The oldest pending event, coalesced per key.
    pub fn next_event(&mut self) -> Option<ClientEvent> {
        self.queue.pop()
    }

    /// Run what is due at `now`: re-check a degraded store, re-open a
    /// session that ended, read connectivity, and retry rows whose time
    /// has come.
    pub async fn tick(&mut self, now: u64) {
        // A store that went degraded outside a commit -- opened over its
        // quota, or failed a write the facade did not make -- is noticed
        // here, before a session is opened or kept on it.
        if self.store.health() == StorageHealth::Degraded {
            self.enter_degraded(now).await;
        }
        self.tick_storage(now);
        self.tick_session(now).await;
        self.tick_connectivity(now).await;
        self.land_ready(now).await;
        if matches!(self.state, SessionState::Ready { .. }) {
            self.retry_due(now).await;
            // An attempt the binding answers at once lands in the tick
            // that made it.
            self.land_ready(now).await;
        }
    }

    /// Compose: commit the pending copy FIRST (`RETENTION.md` §2's
    /// order); the next [`Self::tick`] attempts it if the session is
    /// ready and no earlier row of its conversation is pending (`Order`),
    /// else it follows them in order. Not attempted here: the caller hands the new row to the
    /// person before an attempt to an unreachable peer has taken its time
    /// to fail (`the_first_attempt_waits_for_the_next_tick`).
    ///
    /// # Errors
    /// See [`SendError`]: no row exists after any of them.
    pub async fn send(
        &mut self,
        destination: Destination,
        envelope: &HumanChatV2,
        now: u64,
    ) -> Result<RowId, SendError> {
        // What every receiver will run on it: an envelope it would discard
        // as malformed is refused here, not reported accepted.
        let json = serde_json::to_string(envelope).map_err(|_| SendError::InvalidEnvelope)?;
        HumanChatV2::parse(&json).map_err(|_| SendError::InvalidEnvelope)?;
        let app_message_id = AppMessageId::parse(envelope.app_message_id.clone())
            .map_err(|_| SendError::InvalidEnvelope)?;
        let encoded = encode_outbound(envelope, self.config.max_payload_bytes)
            .map_err(|_| SendError::TooLarge)?;
        self.commit_send(destination, app_message_id, encoded, now)
            .await
    }

    /// Send a contact card, the JSON `build_card` made, to one direct
    /// route: committed pending and attempted as a message is.
    ///
    /// What the receiver will run on it is run here first: a card that
    /// does not verify as [`Classified::Card`] on this node's own route
    /// (its `PeerId` and the endpoint it is sent from) would arrive
    /// malformed, and is refused before any row exists.
    ///
    /// # Errors
    /// [`SendError::InvalidEnvelope`] for such a card or with no session
    /// to say this node's `PeerId`; otherwise as [`Self::send`].
    pub async fn send_card(
        &mut self,
        peer: TransportIdentity,
        endpoint: EndpointId,
        card: &str,
        now: u64,
    ) -> Result<RowId, SendError> {
        let (Some(own), Some(from)) = (self.local_peer().cloned(), self.endpoint().cloned()) else {
            return Err(SendError::InvalidEnvelope);
        };
        let Classified::Card(parsed) = classify(
            card,
            CardRoute {
                source_peer: &own,
                source_endpoint: &from,
            },
        ) else {
            return Err(SendError::InvalidEnvelope);
        };
        let app_message_id =
            AppMessageId::parse(parsed.app_message_id).map_err(|_| SendError::InvalidEnvelope)?;
        let encoded = encode_raw_envelope(card.as_bytes().to_vec(), self.config.max_payload_bytes)
            .map_err(|_| SendError::TooLarge)?;
        self.commit_send(
            Destination::Direct {
                peer,
                endpoint: Some(endpoint),
            },
            app_message_id,
            encoded,
            now,
        )
        .await
    }

    /// This node's `PeerId`, as the open session learned it; `None` while
    /// no session holds.
    #[must_use]
    pub fn local_peer(&self) -> Option<&TransportIdentity> {
        self.session.as_ref().map(|s| s.session().local_peer())
    }

    /// Whether a direct conversation with `peer`'s `endpoint` may be held:
    /// it is held already, or the peer has fewer than a profile's most
    /// endpoints (`MAX_ENDPOINTS`) in the store's conversations and its
    /// contacts' routes together.
    fn admits_endpoint(
        &self,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
    ) -> Result<bool, interweave_human_store::StoreError> {
        if self.store.holds_endpoint(peer, endpoint)? {
            return Ok(true);
        }
        let held = self.store.remote_endpoints(peer)?;
        Ok(held < u64::try_from(interweave_profile_config::MAX_ENDPOINTS).unwrap_or(u64::MAX))
    }

    async fn commit_send(
        &mut self,
        destination: Destination,
        app_message_id: AppMessageId,
        encoded: interweave_human_chat_protocol::Encoded,
        now: u64,
    ) -> Result<RowId, SendError> {
        let media_type =
            MediaType::parse(encoded.media_type).map_err(|_| SendError::InvalidEnvelope)?;
        let destination = match destination {
            Destination::Direct { peer, endpoint } => {
                OutboundDestination::Direct(DirectDestination { peer, endpoint })
            }
            Destination::Broadcast(channel) => OutboundDestination::Broadcast(channel),
        };
        if !self.config.allows(&destination) {
            return Err(SendError::NotConfigured);
        }
        // The person's own send counts toward the peer's endpoints as an
        // inbound does: one that would open a 65th is refused before its
        // pending row is written (HUMAN-CHAT.md, the bound's send side).
        if let OutboundDestination::Direct(DirectDestination {
            peer,
            endpoint: Some(endpoint),
        }) = &destination
        {
            match self.admits_endpoint(peer, endpoint) {
                Ok(true) => {}
                Ok(false) => return Err(SendError::EndpointBound),
                Err(_) => return Err(SendError::StorageUnavailable),
            }
        }
        let new = NewOutbound {
            app_message_id: app_message_id.clone(),
            // Minted once, here, and stored with the row: every retry --
            // after a restart too -- sends under it (HUMAN-CHAT.md
            // §Compression, schema v6). Random, never derived from the
            // application id (line 38's separation).
            transport_message_id: MessageId::from_bytes(rand::random()),
            destination,
            media_type: Some(media_type),
            payload: encoded.bytes,
            created_at: (self.wall)(),
        };
        let (order, created_at) = (Order::of(&new.destination), new.created_at);
        let row_id = match self.store.commit_pending_outbound(&new) {
            Ok(row_id) => row_id,
            Err(e) if e.is_duplicate() => return Err(SendError::AlreadyPending),
            Err(_) => {
                if self.store.health() == StorageHealth::Degraded {
                    self.enter_degraded(now).await;
                }
                return Err(SendError::StorageUnavailable);
            }
        };
        self.rows.insert(
            row_id,
            Row {
                app_message_id,
                order,
                created_at,
                attempts: 0,
                attention: None,
                next_at: Some(now),
                last_problem: None,
                last_code: None,
                may_have_reached: false,
            },
        );
        self.report(row_id);
        Ok(row_id)
    }

    /// Retry a row now, whatever its state: the person's "try again". It
    /// is due at once, and so is every earlier row of its conversation
    /// waiting out a backoff, since the row goes only behind them: the
    /// next [`Self::tick`] attempts the conversation's head, and the row
    /// follows in order. The caller reports its `Sending` before the
    /// attempt's answer
    /// (`a_retry_behind_a_backoff_brings_its_conversation_due`).
    ///
    /// # Errors
    /// [`RowError::NoSuchRow`] for a row that is not pending.
    pub fn retry(&mut self, row_id: RowId, now: u64) -> Result<(), RowError> {
        let row = self.rows.get_mut(&row_id).ok_or(RowError::NoSuchRow)?;
        row.attention = None;
        row.next_at = Some(now);
        let (order, place) = (row.order.clone(), (row.created_at, row_id));
        let ahead: Vec<RowId> = self
            .rows
            .iter()
            .filter(|(id, r)| {
                r.order == order
                    && r.attention.is_none()
                    && (r.created_at, **id) < place
                    && r.next_at.is_some_and(|at| at > now)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in ahead {
            if let Some(r) = self.rows.get_mut(&id) {
                r.next_at = Some(now);
            }
            self.report(id);
        }
        self.report(row_id);
        Ok(())
    }

    /// Cancel a row: it is transport-terminal and its pending copy goes.
    ///
    /// Returns the record the cancel deleted, for the person to take the
    /// cancel back with [`Self::restore`] (`RETENTION.md` §2), or `None`
    /// when no restore could be written: the record could not be read
    /// first, or storage is degraded. Without it the cancel still holds,
    /// and nothing offers an undo that would fail.
    ///
    /// # Errors
    /// [`RowError::NoSuchRow`] for a row that is not pending, or
    /// [`RowError::StorageUnavailable`] if the store could not remove it
    /// (the row stays pending).
    pub fn cancel(&mut self, row_id: RowId) -> Result<Option<CancelledSend>, RowError> {
        let row = self.rows.get(&row_id).ok_or(RowError::NoSuchRow)?;
        // A row on the wire may have reached: its attempt lands after the
        // cancel and is then ignored (`land`).
        let may_have_reached = row.may_have_reached || self.flying.contains_key(&row_id);
        // Read before the delete: after it nothing holds the bytes.
        let record = self.store.pending_outbound_row(row_id).ok().flatten();
        self.store
            .transport_terminal(row_id, TerminalCause::Cancelled)
            .map_err(|_| RowError::StorageUnavailable)?;
        self.finish(row_id, OutboundStatus::Cancelled { may_have_reached });
        Ok(record
            .filter(|_| self.store.health() != StorageHealth::Degraded)
            .map(|record| CancelledSend {
                record,
                may_have_reached,
            }))
    }

    /// Take back a cancel: commit the cancelled record again, unchanged
    /// but for a new row id and its place in its conversation (its
    /// `created_at`), due at once: the next [`Self::tick`] attempts it as
    /// a retry if the session is ready and no earlier row of its
    /// conversation is pending (`Order`) (`RETENTION.md` §2, case 16). The same message, never a new
    /// one: a receiver that admitted the cancelled attempt answers this
    /// one from its dedup entry while it holds it.
    ///
    /// # Errors
    /// [`SendError::NotConfigured`] if this session cannot send there,
    /// [`SendError::AlreadyPending`] if a pending row holds the same
    /// application id, or [`SendError::StorageUnavailable`] if the store
    /// cannot commit it: nothing is sent and no row exists.
    pub async fn restore(
        &mut self,
        cancelled: CancelledSend,
        now: u64,
    ) -> Result<RowId, SendError> {
        let CancelledSend {
            record,
            may_have_reached,
        } = cancelled;
        if !self.config.allows(&record.destination) {
            return Err(SendError::NotConfigured);
        }
        let (order, created_at) = (Order::of(&record.destination), record.created_at);
        let row_id = match self.store.restore_pending_outbound(&record) {
            Ok(row_id) => row_id,
            Err(e) if e.is_duplicate() => return Err(SendError::AlreadyPending),
            Err(_) => {
                if self.store.health() == StorageHealth::Degraded {
                    self.enter_degraded(now).await;
                }
                return Err(SendError::StorageUnavailable);
            }
        };
        self.rows.insert(
            row_id,
            Row {
                app_message_id: record.app_message_id,
                order,
                created_at,
                attempts: record.attempts,
                attention: None,
                next_at: Some(now),
                last_problem: None,
                last_code: None,
                // What the cancelled row said: an attempt that may have
                // reached the remote still may have.
                may_have_reached,
            },
        );
        self.report(row_id);
        Ok(row_id)
    }

    /// Take up to `max` inbound messages: each is committed as unread
    /// BEFORE it is returned (`STATE.md`), so a message the caller shows
    /// is one the store holds. The session is asked for at most `max`
    /// events, and a notice, a duplicate or an undecodable payload takes
    /// a slot, so fewer than `max` -- none, even -- can come back while
    /// more wait: an empty return is not "nothing left" while notices
    /// can fill `max`.
    ///
    /// A message whose commit meets one already held is not returned (a
    /// duplicate is one message); one that cannot be decoded is
    /// discarded and counted, never stored (`HUMAN-CHAT.md` §Consumers);
    /// and if the store cannot commit, nothing more is returned and the
    /// session goes [`SessionState::StorageDegraded`]. Whenever the facade
    /// closes a session, what it holds at that moment is committed first
    /// in one read; up to [`HELD_CAP`] of it is handed over on this and
    /// later calls, and the rest stays unread in the store
    /// ([`ClientEvent::UnreadInStore`]).
    pub async fn drain_inbound(&mut self, max: usize, now: u64) -> Vec<Inbound> {
        let mut received: Vec<Inbound> = Vec::new();
        while received.len() < max {
            let Some(r) = self.held.pop() else {
                break;
            };
            received.push(r);
        }
        let room = max - received.len();
        if room == 0 {
            return received;
        }
        let Some(session) = self.session.as_ref() else {
            return received;
        };
        let events = match session.events(room).await {
            Ok(events) => events,
            Err(e) => {
                if ends_session(e) {
                    self.lose_session(now).await;
                }
                return received;
            }
        };
        let (got, lease_lost, degraded) = self.take(events);
        received.extend(got);
        if degraded {
            self.enter_degraded(now).await;
        } else if lease_lost {
            // What the session holds is committed before it is closed, in
            // one read (`lose_session`): closing releases its queues, and
            // what is in them was already accepted (agreed item 4b
            // re-claims after).
            self.lose_session(now).await;
        }
        received
    }

    /// [`Self::drain_inbound`], messages only: for a caller that shows no
    /// card proposals. A card handed over here is committed unread in the
    /// store all the same, so it is not lost: a card-aware root lists it.
    pub async fn drain(&mut self, max: usize, now: u64) -> Vec<Received> {
        Box::pin(self.drain_inbound(max, now))
            .await
            .into_iter()
            .filter_map(|inbound| match inbound {
                Inbound::Message(received) => Some(received),
                Inbound::Card(_) => None,
            })
            .collect()
    }

    /// Leave [`SessionState::Refused`]: the person's "try again". Re-open
    /// at the next tick.
    pub fn reopen(&mut self, now: u64) {
        if matches!(self.state, SessionState::Refused { .. }) {
            self.reopen_attempt = 0;
            self.set_state(SessionState::Reconnecting {
                attempt: 0,
                next_at: now,
            });
        }
    }

    /// Re-check a degraded store now, as well as on the tick's schedule.
    pub fn recheck(&mut self, now: u64) {
        if self.state == SessionState::StorageDegraded {
            self.next_recheck_at = now;
            self.tick_storage(now);
        }
    }

    /// The profile's trust allowlist (`human-client-ui.md` §8), read over
    /// an administrative connection of its own that holds `admin.trust`
    /// alone and closes with the call: the standing status connection
    /// never holds trust authority, and nothing holds it between a
    /// person's settings actions.
    ///
    /// # Errors
    /// The failure's class.
    pub async fn trust(&self) -> Result<TrustList, TrustProblem> {
        let admin = self.trust_port().await?;
        let view = admin.trust().await.map_err(classify_trust)?;
        Ok(TrustList {
            local_peer: view.local_peer,
            allowed: view.allowed.into_iter().map(trust_row).collect(),
        })
    }

    /// Allow `peer`, or revoke it, then read the allowlist back -- what the
    /// daemon holds afterwards, not what was asked -- on one such
    /// connection. Revoking closes the peer's connections and each
    /// session is told `PeerDisconnected` (`LOCAL-CLIENT.md` §7 item 11).
    ///
    /// # Errors
    /// Whether the change was made, and why the answer is not the list:
    /// not made when the connection or the set failed before anything
    /// left; unconfirmed when the set failed after it may have reached the
    /// daemon (`TRANSPORT.md`'s outcome-unknown class, `may_have_reached`);
    /// made when only the read-back failed.
    pub async fn set_trust(
        &self,
        peer: TransportIdentity,
        allowed: bool,
    ) -> Result<TrustList, TrustSetFailure> {
        let admin = self.trust_port().await.map_err(TrustSetFailure::NotMade)?;
        let change = admin.set_trust(peer, allowed).await.map_err(|error| {
            if may_have_reached(error) {
                TrustSetFailure::Unconfirmed(classify_trust(error))
            } else {
                TrustSetFailure::NotMade(classify_trust(error))
            }
        })?;
        // A refusal wrote nothing: no list is read back as this change's
        // (`a_trust_failure_says_whether_the_change_was_made`).
        match change {
            TrustChange::Set => {}
            TrustChange::Refused(TrustRefusal::Superseded) => {
                return Err(TrustSetFailure::Superseded);
            }
        }
        let view = admin
            .trust()
            .await
            .map_err(|error| TrustSetFailure::MadeNotReadBack(classify_trust(error)))?;
        Ok(TrustList {
            local_peer: view.local_peer,
            allowed: view.allowed.into_iter().map(trust_row).collect(),
        })
    }

    async fn trust_port(&self) -> Result<A::Admin, TrustProblem> {
        self.admin_binding
            .admin(BTreeSet::from([AdminCapability::Trust]))
            .await
            .map_err(classify_trust)
    }

    /// The addresses this device holds for reaching peers (ADR-0062,
    /// `admin.peers.list_addresses`), over an administrative connection of
    /// its own holding `admin.peers` alone and closing with the call, as
    /// trust's does.
    ///
    /// # Errors
    /// The failure's class.
    pub async fn addresses(&self) -> Result<AddressList, AddressProblem> {
        let admin = self.peers_port().await?;
        read_addresses(&admin).await
    }

    /// Add `address` as typed, then read the list back. The runtime
    /// judges it: the grammar, the special-use floor and the bound are
    /// the method's, and a refusal is an answer. Adding grants no trust.
    ///
    /// # Errors
    /// Whether the change was made, as [`Self::set_trust`] says it.
    pub async fn add_address(
        &self,
        address: String,
    ) -> Result<AddressAnswer, AddressChangeFailure> {
        let peer = PeerAddress::peer_of(&address);
        self.change_address(address, true).await.map(|mut answer| {
            if answer.outcome == (AddressOutcome::Added { peer: None }) {
                answer.outcome = AddressOutcome::Added { peer };
            }
            answer
        })
    }

    /// Add `lines` as one add, all or none (ADR-0062 decision 4), then
    /// read the list back: one line as [`Self::add_address`] adds it,
    /// which any runtime with the methods serves; several through IPC
    /// 2.9's `admin.peers.add_addresses`, which checks every line before
    /// it writes any. Several lines are never one call per line, since a
    /// split add can stop half done: below 2.9 the port sends nothing and
    /// the add is not made, its problem `Incompatible`.
    ///
    /// # Errors
    /// As [`Self::add_address`].
    pub async fn add_addresses(
        &self,
        lines: Vec<String>,
    ) -> Result<AddressAnswer, AddressChangeFailure> {
        let mut lines = lines;
        if lines.len() == 1 {
            let line = lines.pop().unwrap_or_default();
            return self.add_address(line).await;
        }
        let admin = self
            .peers_port()
            .await
            .map_err(AddressChangeFailure::NotMade)?;
        let change = admin
            .add_bootstrap_addresses(lines.clone())
            .await
            .map_err(change_failure)?;
        let outcome = match change {
            AddressesChange::Added(rows) if rows.iter().all(|r| *r == LineChange::Unchanged) => {
                AddressOutcome::Unchanged
            }
            AddressesChange::Added(rows) => AddressOutcome::AddedLines(
                rows.into_iter()
                    .zip(&lines)
                    .map(|(row, line)| LineAdded {
                        added: row == LineChange::Added,
                        peer: PeerAddress::peer_of(line),
                    })
                    .collect(),
            ),
            AddressesChange::Refused {
                reason,
                line: Some(line),
                ..
            } => AddressOutcome::RefusedLine {
                why: refused(reason),
                line,
            },
            AddressesChange::Refused { reason, .. } => AddressOutcome::Refused(refused(reason)),
        };
        let list = read_addresses(&admin).await;
        Ok(AddressAnswer { outcome, list })
    }

    /// This device's own addresses for the show screen (ADR-0062 decision
    /// 4, `admin.peers.own_addresses`, IPC 2.8), as the runtime chose,
    /// ordered and floored them; reading them causes nothing.
    ///
    /// # Errors
    /// The failure's class: `Incompatible` below 2.8, nothing sent.
    pub async fn own_addresses(&self) -> Result<ShowAddresses, AddressProblem> {
        let admin = self.peers_port().await?;
        let own = admin.own_addresses().await.map_err(classify_address)?;
        Ok(ShowAddresses {
            peer: own.peer,
            addresses: own.addresses.into_iter().map(|a| a.address).collect(),
            truncated: own.truncated,
        })
    }

    /// Remove `address`, then read the list back. A configuration entry
    /// is refused by name.
    ///
    /// # Errors
    /// As [`Self::add_address`].
    pub async fn remove_address(
        &self,
        address: String,
    ) -> Result<AddressAnswer, AddressChangeFailure> {
        self.change_address(address, false).await
    }

    async fn change_address(
        &self,
        address: String,
        add: bool,
    ) -> Result<AddressAnswer, AddressChangeFailure> {
        let admin = self
            .peers_port()
            .await
            .map_err(AddressChangeFailure::NotMade)?;
        let change = if add {
            admin.add_bootstrap_address(address).await
        } else {
            admin.remove_bootstrap_address(address).await
        };
        let change = change.map_err(change_failure)?;
        let outcome = match change {
            AddressChange::Added => AddressOutcome::Added { peer: None },
            AddressChange::Removed => AddressOutcome::Removed,
            AddressChange::Unchanged => AddressOutcome::Unchanged,
            AddressChange::Refused { reason, .. } => AddressOutcome::Refused(refused(reason)),
        };
        let list = read_addresses(&admin).await;
        Ok(AddressAnswer { outcome, list })
    }

    async fn peers_port(&self) -> Result<A::Admin, AddressProblem> {
        self.admin_binding
            .admin(BTreeSet::from([AdminCapability::Peers]))
            .await
            .map_err(classify_address)
    }

    /// End the session; the facade does nothing more until dropped. What
    /// the session's queue still holds is committed unread first, in one
    /// read, as for a session lost: it was accepted, and the close
    /// releases the queue (`DIRECT.md` §Graceful shutdown, A 2026-11-17;
    /// `a_close_commits_what_the_session_still_held_unread`). A caller
    /// that stops the runtime puts it into its drain before this, so the
    /// read is the queue's last.
    pub async fn close(&mut self) {
        self.take_the_rest().await;
        self.close_session(None).await;
        self.admin = None;
        self.set_state(SessionState::Closed);
    }

    // --- the tick's parts ------------------------------------------------

    fn tick_storage(&mut self, now: u64) {
        if self.state != SessionState::StorageDegraded || now < self.next_recheck_at {
            return;
        }
        match self.store.recheck_health() {
            Ok(StorageHealth::Healthy) => {
                self.recheck_attempt = 0;
                self.reopen_attempt = 0;
                match self.refused_before_degrade.take() {
                    Some(problem) => self.set_state(SessionState::Refused { problem }),
                    None => self.set_state(SessionState::Reconnecting {
                        attempt: 0,
                        next_at: now,
                    }),
                }
            }
            Ok(StorageHealth::Degraded) | Err(_) => {
                self.recheck_attempt = self.recheck_attempt.saturating_add(1);
                self.next_recheck_at = now + RECHECK.delay(self.recheck_attempt, 0);
            }
        }
    }

    async fn tick_session(&mut self, now: u64) {
        let SessionState::Reconnecting { next_at, .. } = self.state else {
            return;
        };
        if now < next_at {
            return;
        }
        match self.open().await {
            Ok((session, endpoint)) => {
                self.session = Some(Arc::new(session));
                self.reopen_attempt = 0;
                self.leased.clone_from(&endpoint);
                self.set_state(SessionState::Ready { endpoint });
            }
            Err(e) => match classify_open(e) {
                OpenFailure::Refused(problem) => {
                    self.set_state(SessionState::Refused { problem });
                }
                OpenFailure::Retry => {
                    self.reopen_attempt = self.reopen_attempt.saturating_add(1);
                    let attempt = self.reopen_attempt;
                    self.set_state(SessionState::Reconnecting {
                        attempt,
                        next_at: now + REOPEN.delay(attempt, 1),
                    });
                }
            },
        }
    }

    /// Open a session on the first configured endpoint the runtime does
    /// not refuse as disabled, and make its joins; a join that fails
    /// closes it. Answers the session and the endpoint it leased.
    ///
    /// Only `EndpointDisabled` moves on to the next endpoint: it is the
    /// one refusal an administrator's kept change makes, and the next
    /// endpoint is as much this client's. Any other refusal -- one in
    /// use, say, held by another window -- stops the walk, so a client
    /// never takes a second endpoint because its own is busy. All of them
    /// disabled answers `EndpointDisabled`.
    async fn open(&self) -> Result<(B::Session, Option<EndpointId>), TransportError> {
        let candidates: Vec<Option<EndpointId>> = if self.config.endpoints.is_empty() {
            vec![None]
        } else {
            self.config.endpoints.iter().cloned().map(Some).collect()
        };
        for endpoint in candidates {
            let request = SessionRequest::new(
                self.config.client_kind.clone(),
                endpoint.clone(),
                [DataCapability::Commands, DataCapability::Events],
            )
            .map_err(|_| TransportError::InvalidArgument)?;
            let session = match self.binding.open(request).await {
                Ok(session) => session,
                Err(TransportError::EndpointDisabled) => continue,
                Err(e) => return Err(e),
            };
            for channel in &self.config.channels {
                if let Err(e) = session.join(channel.clone()).await {
                    let _ = session.close().await;
                    return Err(e);
                }
            }
            return Ok((session, endpoint));
        }
        Err(TransportError::EndpointDisabled)
    }

    async fn tick_connectivity(&mut self, now: u64) {
        if now < self.next_status_at || self.state == SessionState::Closed {
            return;
        }
        if self.admin.is_none() {
            let Ok(admin) = self
                .admin_binding
                .admin(BTreeSet::from([AdminCapability::Status]))
                .await
            else {
                self.admin_failed(now);
                return;
            };
            self.admin = Some(admin);
        }
        let Some(admin) = self.admin.as_ref() else {
            return;
        };
        let Ok(status) = admin.status().await else {
            self.admin = None;
            self.admin_failed(now);
            return;
        };
        self.admin_attempt = 0;
        self.next_status_at = now + STATUS_INTERVAL_MS;
        if let Some(normalized) = connectivity_of(status.health, Some(&status.connectivity)) {
            self.set_connectivity(normalized);
        }
    }

    fn admin_failed(&mut self, now: u64) {
        self.admin_attempt = self.admin_attempt.saturating_add(1);
        self.next_status_at = now + REOPEN.delay(self.admin_attempt, 2);
        self.set_connectivity(Connectivity::Unknown);
    }

    /// Attempt every due row, a page of the store at a time: the store
    /// refuses an unbounded read, and so does this.
    async fn retry_due(&mut self, now: u64) {
        if !attemptable(&self.rows, &self.flying, now) {
            return;
        }
        let mut after: Option<Cursor> = None;
        // A conversation whose earliest pending row this pass did not
        // attempt -- on the wire, waiting out its backoff, or its peer's
        // slot taken -- takes no later row of its own (`Order`). A row
        // held for the person's attention waits for the person, not the
        // transport, and blocks nothing.
        let mut blocked: BTreeSet<Order> = BTreeSet::new();
        loop {
            let Ok(page) = self
                .store
                .pending_outbound_page(after, PageLimits::default())
            else {
                self.diagnostics.pending_unreadable += 1;
                return;
            };
            for pending in page.items {
                if !matches!(self.state, SessionState::Ready { .. }) {
                    return;
                }
                if self.flying.len() >= FLIGHT_CAP {
                    return;
                }
                let Some(row) = self.rows.get(&pending.row_id) else {
                    continue;
                };
                if row.attention.is_some() {
                    continue;
                }
                let order = Order::of(&pending.destination);
                let flight = order.flight();
                if !blocked.insert(order) {
                    continue;
                }
                // One attempt on the wire per slot; a cancelled row's
                // attempt may still be there.
                if row.is_due(now)
                    && !self.flying.contains_key(&pending.row_id)
                    && !self.flying.values().any(|f| *f == flight)
                {
                    self.attempt(&pending, flight, now).await;
                }
            }
            match page.next {
                Some(next) => after = Some(next),
                None => return,
            }
        }
    }

    // --- one attempt -------------------------------------------------------

    /// Put one row's attempt on the wire: it lands in a later tick
    /// ([`Self::land_ready`]), so nothing waits on its exchange.
    async fn attempt(&mut self, pending: &PendingOutbound, flight: Flight, now: u64) {
        let row_id = pending.row_id;
        // A row that survived a restart into a configuration that cannot
        // send it: the person's to decide, and no session is touched.
        if !self.config.allows(&pending.destination) {
            if let Some(row) = self.rows.get_mut(&row_id) {
                row.attention = Some(SendProblem::NotConfigured);
                row.next_at = None;
            }
            self.report(row_id);
            return;
        }
        if self.store.record_attempt(row_id, (self.wall)()).is_err()
            && self.store.health() == StorageHealth::Degraded
        {
            self.enter_degraded(now).await;
            return;
        }
        if let Some(row) = self.rows.get_mut(&row_id) {
            row.attempts = row.attempts.saturating_add(1);
            row.next_at = None;
        }
        let Some(session) = self.session.clone() else {
            return;
        };
        let Ok(payload) = Payload::new(
            pending.media_type.clone(),
            pending.payload.clone(),
            self.config.max_payload_bytes,
        ) else {
            self.failed(row_id, TransportError::PayloadTooLarge, now);
            return;
        };
        let message_id = pending.transport_message_id;
        let flying: InFlight = match pending.destination.clone() {
            OutboundDestination::Direct(destination) => Box::pin(async move {
                let landing = session
                    .send_direct(destination, message_id, payload)
                    .await
                    .map(|endpoint| (TerminalCause::Accepted, Some(endpoint)));
                (row_id, landing)
            }),
            OutboundDestination::Broadcast(channel) => {
                let message = BroadcastMessageV1 {
                    message_id,
                    sent_at_ms: (self.wall)(),
                    payload,
                };
                Box::pin(async move {
                    let landing = session
                        .broadcast(channel, message)
                        .await
                        .map(|()| (TerminalCause::Published, None));
                    (row_id, landing)
                })
            }
        };
        self.flying.insert(row_id, flight);
        self.in_flight.push(flying);
    }

    /// Land every attempt that has answered, without waiting for one that
    /// has not.
    async fn land_ready(&mut self, now: u64) {
        while let Some(Some((row_id, landing))) = self.in_flight.next().now_or_never() {
            self.flying.remove(&row_id);
            self.land(row_id, landing, now).await;
        }
    }

    /// Take the closing session's attempts off the wire: each row's
    /// outcome is unknown, as for a call pending when a session ends, and
    /// it is retried on the next session. `None` on the final close, which
    /// reports nothing more.
    async fn close_session(&mut self, now: Option<u64>) {
        // What has answered is settled first, an acceptance included: only
        // what is still on the wire is taken off it. Settled, not landed --
        // the session is closing already.
        if let Some(now) = now {
            while let Some(Some((row_id, landing))) = self.in_flight.next().now_or_never() {
                self.flying.remove(&row_id);
                // Boxed: a settle that degrades the store closes the session
                // again, which drains this same set, so it ends.
                let _ = Box::pin(self.settle(row_id, landing, now)).await;
            }
        }
        self.in_flight = FuturesUnordered::new();
        let rows: Vec<RowId> = std::mem::take(&mut self.flying).into_keys().collect();
        if let Some(now) = now {
            for row_id in rows {
                self.failed(row_id, TransportError::CancellationRaced, now);
            }
        }
        if let Some(session) = self.session.take() {
            // The attempts that held it are gone, so it is the only owner;
            // a debug build says so loudly if that stops holding, since the
            // lease would then be released only when the last holder drops.
            // A close that fails leaves nothing to undo: the binding ends
            // the session either way.
            match Arc::try_unwrap(session) {
                Ok(session) => {
                    let _ = session.close().await;
                }
                Err(shared) => {
                    debug_assert_eq!(
                        Arc::strong_count(&shared),
                        1,
                        "a session shared at its close"
                    );
                }
            }
        }
    }

    /// What an attempt answered, applied to its row. A row cancelled while
    /// its attempt was on the wire is gone, and the answer with it.
    async fn land(&mut self, row_id: RowId, landing: Landing, now: u64) {
        if let Err(e) = self.settle(row_id, landing, now).await
            && self.send_ended_session(e).await
        {
            self.lose_session(now).await;
        }
    }

    /// Apply an attempt's answer to its row, and return the error it
    /// answered, if any, for the caller to ask whether the session ended.
    /// A row cancelled while its attempt was on the wire is gone, and the
    /// answer with it
    /// (`a_row_cancelled_on_the_wire_is_cancelled_as_may_have_reached`).
    async fn settle(
        &mut self,
        row_id: RowId,
        landing: Landing,
        now: u64,
    ) -> Result<(), TransportError> {
        if !self.rows.contains_key(&row_id) {
            return Ok(());
        }
        match landing {
            Ok((cause, endpoint)) => {
                // Terminal whether or not the store can delete the copy
                // now: if it cannot, the row is sent again after a
                // restart, under the same transport id, and the
                // receiver's dedup absorbs it.
                if self.store.transport_terminal(row_id, cause).is_err()
                    && self.store.health() == StorageHealth::Degraded
                {
                    self.enter_degraded(now).await;
                }
                let status = endpoint.map_or(OutboundStatus::Published, |endpoint| {
                    OutboundStatus::Accepted { endpoint }
                });
                self.finish(row_id, status);
                Ok(())
            }
            Err(e) => {
                self.failed(row_id, e, now);
                Err(e)
            }
        }
    }

    /// Whether a send's `error` says the SESSION ended, not the exchange.
    /// A `BackendUnavailable` is also a remote `ShuttingDown` refusal
    /// mapped back (`TRANSPORT.md` §Error model) over a live connection:
    /// the peer is going, and closing this session for it would drop the
    /// lease every other peer sends to. So the session is asked:
    /// `events(0)` takes nothing, answers an ended session's end and a
    /// live one's empty list (`DataSessionPort::events`). The other codes
    /// `ends_session` counts are this session's own -- a lost lease or
    /// join, a local shutdown -- and end it as they are.
    /// `a_remote_shutdown_keeps_the_session_and_retries_the_send` and
    /// `a_send_on_an_ended_session_re_opens_it` pin both sides.
    async fn send_ended_session(&self, error: TransportError) -> bool {
        if !ends_session(error) {
            return false;
        }
        if error != TransportError::BackendUnavailable {
            return true;
        }
        match self.session.as_ref() {
            Some(session) => session.events(0).await.is_err(),
            None => true,
        }
    }

    fn failed(&mut self, row_id: RowId, error: TransportError, now: u64) {
        let Some(row) = self.rows.get_mut(&row_id) else {
            return;
        };
        row.last_code = Some(error);
        if may_have_reached(error) {
            row.may_have_reached = true;
        }
        let key = u64::from_ne_bytes(row_id.get().to_ne_bytes());
        match classify_send(error) {
            AttemptFailure::Retry(problem) => {
                if problem.is_some() {
                    row.last_problem = problem;
                }
                row.next_at = Some(now + SEND.delay(row.attempts, key));
            }
            AttemptFailure::NeedsAttention(problem) => {
                row.attention = Some(problem);
                row.next_at = None;
            }
        }
        self.report(row_id);
    }

    fn finish(&mut self, row_id: RowId, status: OutboundStatus) {
        if let Some(row) = self.rows.remove(&row_id) {
            self.queue.push(ClientEvent::Outbound(OutboundUpdate {
                row: row_id,
                app_message_id: row.app_message_id,
                status,
                last_code: row.last_code,
            }));
        }
    }

    fn report(&mut self, row_id: RowId) {
        let Some(row) = self.rows.get(&row_id) else {
            return;
        };
        self.queue.push(ClientEvent::Outbound(OutboundUpdate {
            row: row_id,
            app_message_id: row.app_message_id.clone(),
            status: row.status(),
            last_code: row.last_code,
        }));
    }

    // --- inbound -----------------------------------------------------------

    /// Commit what `events` carries, in order. Returns what was committed,
    /// whether the lease was lost, and whether the store failed (after
    /// which the rest is dropped and counted: the handoff window).
    fn take(&mut self, events: Vec<SessionEvent>) -> (Vec<Inbound>, bool, bool) {
        let mut received = Vec::new();
        let mut lease_lost = false;
        let mut degraded = false;
        for event in events {
            if degraded {
                if !matches!(event, SessionEvent::Local(_)) {
                    self.diagnostics.dropped_unstored += 1;
                }
                continue;
            }
            match event {
                SessionEvent::Local(LocalSessionEvent::EndpointLeaseChanged { .. }) => {
                    lease_lost = true;
                }
                SessionEvent::Local(LocalSessionEvent::PeerDisconnected { peer, .. }) => {
                    self.queue.push(ClientEvent::PeerDisconnected { peer });
                }
                // The runtime's state, pushed as it changes: the same
                // connectivity the admin port's status gives, sooner. One
                // with no summary says only whether the runtime is there.
                // Past a store failure it is skipped with the other
                // session notices, uncounted.
                SessionEvent::Local(LocalSessionEvent::ServerState {
                    health,
                    connectivity,
                }) => {
                    if let Some(normalized) = connectivity_of(health, connectivity.as_ref()) {
                        self.set_connectivity(normalized);
                    }
                }
                // The route indicator's (`human-client-ui.md` §7): the
                // path now, never a message. A notice with no `previous`
                // -- the route's begin or the routed peer's return -- sets
                // it just as a change does.
                SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
                    peer, current, ..
                }) => {
                    self.queue.push(ClientEvent::PeerPath {
                        peer,
                        path: current,
                    });
                }
                SessionEvent::Direct(direct) => {
                    let origin = Origin::Direct {
                        peer: direct.source_peer.clone(),
                        endpoint: direct.source_endpoint.clone(),
                    };
                    let stored = InboundOrigin {
                        peer: direct.source_peer,
                        endpoint: Some(direct.source_endpoint),
                        channel: None,
                    };
                    match self.commit(&direct.payload, origin, stored) {
                        Committed::Yes(r) => received.push(*r),
                        Committed::Skipped => {}
                        Committed::StoreFailed => degraded = true,
                    }
                }
                SessionEvent::Broadcast(broadcast) => {
                    let origin = Origin::Channel {
                        channel: broadcast.channel.clone(),
                        publisher: broadcast.source_peer.clone(),
                    };
                    let stored = InboundOrigin {
                        peer: broadcast.source_peer,
                        endpoint: None,
                        channel: Some(broadcast.channel),
                    };
                    match self.commit(&broadcast.payload, origin, stored) {
                        Committed::Yes(r) => received.push(*r),
                        Committed::Skipped => {}
                        Committed::StoreFailed => degraded = true,
                    }
                }
            }
        }
        (received, lease_lost, degraded)
    }

    /// Before the facade closes a session: commit what it holds into
    /// `held`, so nothing already accepted is released with its queues.
    /// ONE read, a snapshot of what is queued now -- not a loop until
    /// empty, which inbound still arriving could keep going.
    async fn take_the_rest(&mut self) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let Ok(events) = session.events(usize::MAX).await else {
            return;
        };
        let (got, _, _) = self.take(events);
        let mut overflowed = false;
        for received in got {
            if !self.held.push(received) {
                self.diagnostics.held_overflow += 1;
                overflowed = true;
            }
        }
        if overflowed {
            self.queue.push(ClientEvent::UnreadInStore {
                not_handed_over: self.diagnostics.held_overflow,
            });
        }
    }

    fn commit(&mut self, payload: &Payload, origin: Origin, stored: InboundOrigin) -> Committed {
        let Some(decoded) = self.decode(payload, &origin) else {
            return Committed::Skipped;
        };
        // A peer's 65th distinct source endpoint: malformed, stored
        // nothing, counted under its one reason (HUMAN-CHAT.md). A read the
        // check needs that fails is a commit failure (RETENTION.md).
        if let Origin::Direct { peer, endpoint } = &origin {
            match self.admits_endpoint(peer, endpoint) {
                Ok(true) => {}
                Ok(false) => {
                    self.diagnostics.malformed_endpoint_bound += 1;
                    return Committed::Skipped;
                }
                Err(_) => {
                    self.diagnostics.dropped_unstored += 1;
                    return Committed::StoreFailed;
                }
            }
        }
        let id = match &decoded {
            Decoded::Message(envelope) => envelope.app_message_id.clone(),
            Decoded::Card(card) => card.app_message_id.clone(),
        };
        let Ok(app_message_id) = AppMessageId::parse(id) else {
            self.diagnostics.malformed_invalid_envelope += 1;
            return Committed::Skipped;
        };
        let received_at = (self.wall)();
        // Classified as the listing classifies a stored row (STATE.md
        // schema v10): what is committed here decoded, as a message or a
        // card, and a row this build cannot read is never committed.
        let kind = match &decoded {
            Decoded::Message(_) => interweave_human_store::InboundKind::Text,
            Decoded::Card(_) => interweave_human_store::InboundKind::Card,
        };
        let new = NewInbound {
            app_message_id,
            origin: stored,
            media_type: payload.media_type().cloned(),
            payload: payload.bytes().to_vec(),
            received_at,
            kind,
        };
        match self.store.commit_unread_inbound(&new) {
            Ok(row) => Committed::Yes(Box::new(match decoded {
                Decoded::Message(envelope) => Inbound::Message(Received {
                    row,
                    origin,
                    envelope,
                    received_at,
                }),
                Decoded::Card(card) => {
                    // A card is decoded only from a direct origin.
                    let Origin::Direct { peer, endpoint } = origin else {
                        return Committed::Skipped;
                    };
                    Inbound::Card(ReceivedCard {
                        row,
                        peer,
                        endpoint,
                        card,
                        received_at,
                    })
                }
            })),
            Err(e) if e.is_duplicate() => Committed::Skipped,
            Err(_) => {
                self.diagnostics.dropped_unstored += 1;
                Committed::StoreFailed
            }
        }
    }

    /// The envelope or the card, or `None` having counted why
    /// (`HUMAN-CHAT.md` §Consumers: store nothing, present nothing,
    /// count it).
    ///
    /// A direct text is asked first whether it is a card: any top-level
    /// `kind` of `contact_card` makes it one, so a card that is malformed
    /// is counted as a card and never reaches the message parser, which
    /// would refuse it as an unknown kind. A card over broadcast is
    /// ignored: broadcast is not the channel a card is sent on, and its
    /// route has no endpoint to verify the signature over.
    fn decode(&mut self, payload: &Payload, origin: &Origin) -> Option<Decoded> {
        let Some(info) = payload
            .media_type()
            .and_then(|m| parse_media_type(m.as_str()).ok())
        else {
            self.diagnostics.malformed_unknown_media_type += 1;
            return None;
        };
        let Ok(text) = decode_envelope_bytes(payload.bytes(), info.encoding) else {
            self.diagnostics.malformed_undecodable += 1;
            return None;
        };
        match origin {
            Origin::Direct { peer, endpoint } => match classify(
                &text,
                CardRoute {
                    source_peer: peer,
                    source_endpoint: endpoint,
                },
            ) {
                Classified::Card(card) => return Some(Decoded::Card(card)),
                Classified::MalformedCard(_) => {
                    self.diagnostics.malformed_card += 1;
                    return None;
                }
                Classified::NotCard => {}
            },
            Origin::Channel { .. } => {
                if is_card(&text) {
                    self.diagnostics.card_over_broadcast += 1;
                    return None;
                }
            }
        }
        let Ok(envelope) = HumanChatV2::parse(&text) else {
            self.diagnostics.malformed_invalid_envelope += 1;
            return None;
        };
        Some(Decoded::Message(envelope))
    }

    // --- session transitions -----------------------------------------------

    /// The session ended under us, or lost its lease or a join: re-open
    /// with backoff. What it still holds is committed first -- a session
    /// that lost its lease is alive and may hold accepted inbound; from a
    /// dead one the read fails fast.
    async fn lose_session(&mut self, now: u64) {
        self.take_the_rest().await;
        if self.store.health() == StorageHealth::Degraded {
            // The take met a full store: degraded, not merely reconnecting.
            self.enter_degraded(now).await;
            return;
        }
        self.close_session(Some(now)).await;
        // The close settled what had answered, and a settle can meet a
        // full store: degraded then, not merely reconnecting, as for the
        // take above. No test reaches it: the store's test seams cannot
        // make a terminal delete fail under its quota.
        if self.store.health() == StorageHealth::Degraded {
            self.enter_degraded(now).await;
            return;
        }
        self.reopen_attempt = self.reopen_attempt.saturating_add(1);
        let attempt = self.reopen_attempt;
        self.set_state(SessionState::Reconnecting {
            attempt,
            next_at: now + REOPEN.delay(attempt, 1),
        });
    }

    /// The store cannot hold unread content: release the lease and the
    /// joins by closing the session, and re-check on a schedule
    /// (ADR-0044, `STATE.md` "Store health"). Never leaves `Closed`, and
    /// remembers `Refused` so a recovery returns there.
    async fn enter_degraded(&mut self, now: u64) {
        match &self.state {
            SessionState::Closed | SessionState::StorageDegraded => return,
            SessionState::Refused { problem } => self.refused_before_degrade = Some(*problem),
            SessionState::Ready { .. } | SessionState::Reconnecting { .. } => {}
        }
        self.close_session(Some(now)).await;
        self.recheck_attempt = 0;
        self.next_recheck_at = now + RECHECK.delay(1, 0);
        self.set_state(SessionState::StorageDegraded);
    }

    fn set_state(&mut self, state: SessionState) {
        if self.state != state {
            self.state = state.clone();
            self.queue.push(ClientEvent::Session(state));
        }
    }

    fn set_connectivity(&mut self, connectivity: Connectivity) {
        if self.connectivity != connectivity {
            self.connectivity = connectivity;
            self.queue.push(ClientEvent::Connectivity(connectivity));
        }
    }
}

/// What a decoded inbound text is.
enum Decoded {
    Message(HumanChatV2),
    Card(interweave_human_chat_protocol::Card),
}

enum Committed {
    // Boxed: a card is far larger than the other two answers.
    Yes(Box<Inbound>),
    Skipped,
    StoreFailed,
}

/// The connectivity a person is shown for the runtime's `health` and
/// `summary`, or `None` when they do not say: a runtime that is there but
/// gave no summary leaves the indicator as it was.
fn connectivity_of(health: Health, summary: Option<&ConnectivitySummary>) -> Option<Connectivity> {
    match (health, summary) {
        (Health::Unavailable, _) => Some(Connectivity::Offline),
        (Health::Healthy | Health::Degraded, Some(summary)) => {
            Some(match (summary.direct_inbound, summary.relay_inbound) {
                (DirectInboundState::VerifiedPublic, _) => Connectivity::OnlineDirect,
                (_, PathReadiness::Ready) => Connectivity::OnlineRelay,
                _ => Connectivity::OnlinePartial,
            })
        }
        (Health::Healthy | Health::Degraded, None) => None,
    }
}

/// Whether a failed address change was made. The methods' `Internal` is
/// their failed write of the list, which changed nothing
/// (`AdminPort::add_bootstrap_address`); only an answer lost after the
/// request left is unknown.
fn change_failure(error: TransportError) -> AddressChangeFailure {
    if may_have_reached(error) && error != TransportError::Internal {
        AddressChangeFailure::Unconfirmed(classify_address(error))
    } else {
        AddressChangeFailure::NotMade(classify_address(error))
    }
}

const fn refused(reason: AddressRefusal) -> AddressRefused {
    match reason {
        AddressRefusal::Grammar => AddressRefused::Grammar,
        AddressRefusal::SpecialUse => AddressRefused::SpecialUse,
        AddressRefusal::Bound => AddressRefused::Bound,
        AddressRefusal::ProviderDisabled => AddressRefused::ListOff,
        AddressRefusal::Authored => AddressRefused::Configured,
        AddressRefusal::OwnPeer => AddressRefused::OwnPeer,
        AddressRefusal::Superseded => AddressRefused::Superseded,
    }
}

/// The list in the daemon's order; an address authored in the
/// configuration is `Configured`, one added at runtime `AddedHere`.
async fn read_addresses<P: AdminPort>(admin: &P) -> Result<AddressList, AddressProblem> {
    let rows = admin
        .bootstrap_addresses()
        .await
        .map_err(classify_address)?;
    Ok(AddressList {
        addresses: rows
            .into_iter()
            .map(|row| PeerAddress {
                peer: PeerAddress::peer_of(&row.address),
                origin: match row.source {
                    AddressSource::Authored => AddressOrigin::Configured,
                    AddressSource::Added => AddressOrigin::AddedHere,
                },
                dialled: row.dialled,
                address: row.address,
            })
            .collect(),
    })
}

/// The neutral row in the client's words: a peer both configured and
/// administered is already `Configured` (the daemon normalises it).
fn trust_row(row: interweave_local_client_api::TrustedPeer) -> TrustRow {
    TrustRow {
        peer: row.peer,
        origin: match row.source {
            interweave_local_client_api::TrustSource::Configured => TrustOrigin::Configured,
            interweave_local_client_api::TrustSource::Administered => TrustOrigin::AddedHere,
        },
    }
}

#[cfg(test)]
mod tests {
    use interweave_human_client_api::Connectivity;
    use interweave_transport_api::Health;

    use std::collections::BTreeMap;

    use interweave_human_store::{AppMessageId, RowId};
    use interweave_transport_api::{EndpointId, TransportIdentity};

    use super::{Flight, Order, Row, attemptable, connectivity_of};

    fn peer(seed: u8) -> TransportIdentity {
        // A fixed Ed25519 public key's PeerId per seed is not needed: any
        // valid identity does, and the three are told apart by value.
        let ids = [
            "12D3KooWB1qCTb256cso59NZaR4Wcq5k1aP7N1MprmBNnhj7YNQv",
            "12D3KooWLKJUH4yb3jMugHU2EmwxiF3FAtAE5vdBwrfgwe5Lpsra",
            "12D3KooWJpdARiNPgpq2nSS7KMY4iW9Ba8fA8rKggFJjgPjNqe5t",
        ];
        TransportIdentity::parse(ids[usize::from(seed)]).expect("a peer id")
    }

    fn row(order: &Order, created_at: u64, next_at: Option<u64>) -> Row {
        Row {
            app_message_id: AppMessageId::parse(format!("{created_at:032x}")).expect("an id"),
            order: order.clone(),
            created_at,
            attempts: 0,
            attention: None,
            next_at,
            last_problem: None,
            last_code: None,
            may_have_reached: false,
        }
    }

    /// A pass is due only when a conversation's head can go: rows due
    /// behind a head waiting out its backoff, or behind one on the wire,
    /// or whose peer's slot is taken, or with the flights at the cap, make
    /// no pass. Mutation: the guard asking only whether any row is due --
    /// the first two cases pass; the guard without the cap -- the last.
    #[test]
    fn a_pass_is_due_only_for_a_head() {
        let human = EndpointId::parse("human").expect("valid");
        let a = Order::Conversation(peer(0), Some(human.clone()));
        let other_of_a = Order::Conversation(peer(0), None);
        let mut rows = BTreeMap::new();
        let mut flying = BTreeMap::new();
        // The head backs off; the row behind it is due.
        rows.insert(RowId::from_stored(1), row(&a, 10, Some(500)));
        rows.insert(RowId::from_stored(2), row(&a, 11, Some(0)));
        assert!(!attemptable(&rows, &flying, 100), "behind a backoff");
        assert!(attemptable(&rows, &flying, 500), "the head is due");
        // The head is on the wire.
        rows.insert(RowId::from_stored(1), row(&a, 10, None));
        flying.insert(RowId::from_stored(1), Flight::Peer(peer(0)));
        assert!(!attemptable(&rows, &flying, 100), "behind the wire");
        // Another conversation with the peer is due, but its slot is taken.
        rows.insert(RowId::from_stored(3), row(&other_of_a, 12, Some(0)));
        assert!(!attemptable(&rows, &flying, 100), "the peer's slot");
        // Another peer's row goes.
        let b = Order::Conversation(peer(1), Some(human));
        rows.insert(RowId::from_stored(4), row(&b, 13, Some(0)));
        assert!(attemptable(&rows, &flying, 100), "another peer");
        // A head held for attention blocks nothing.
        let mut held = BTreeMap::new();
        let mut attention = row(&a, 10, None);
        attention.attention = Some(interweave_human_client_api::SendProblem::PeerUntrusted);
        held.insert(RowId::from_stored(1), attention);
        held.insert(RowId::from_stored(2), row(&a, 11, Some(0)));
        assert!(
            attemptable(&held, &BTreeMap::new(), 100),
            "attention holds nothing"
        );
        // The flights full: another peer's due head waits for a slot.
        let full: BTreeMap<RowId, Flight> = (0..super::FLIGHT_CAP)
            .map(|n| {
                let n = i64::try_from(n).expect("small");
                let channel =
                    interweave_transport_api::ChannelId::parse(format!("c{n}")).expect("valid");
                (RowId::from_stored(1_000 + n), Flight::Channel(channel))
            })
            .collect();
        let mut one = BTreeMap::new();
        one.insert(
            RowId::from_stored(4),
            row(&Order::Conversation(peer(2), None), 13, Some(0)),
        );
        assert!(attemptable(&one, &BTreeMap::new(), 100), "the control");
        assert!(!attemptable(&one, &full, 100), "the cap");
    }

    /// A runtime that is there and gave no summary leaves the indicator as
    /// it was; one that is not is offline whatever it summarised.
    #[test]
    fn no_summary_says_only_whether_the_runtime_is_there() {
        assert_eq!(connectivity_of(Health::Healthy, None), None);
        assert_eq!(connectivity_of(Health::Degraded, None), None);
        assert_eq!(
            connectivity_of(Health::Unavailable, None),
            Some(Connectivity::Offline)
        );
    }
}
