// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Send attempts on the wire beside each other, one per peer or channel
//! and at most `FLIGHT_CAP` in all, with order held per conversation
//! (architect-cto's rulings 01a1e901 and 01a1e96d, j288): a peer that
//! never answers holds no line to another peer and no inbound; lines in
//! one conversation go in the order written, a refused one's backoff
//! included, and a dead endpoint holds only its own conversation; a
//! session lost or a row cancelled with a send on the wire leaves no row
//! lost or reported accepted.
//!
//! The fake answers every send at once, so the sends here go through
//! `Holding`, a binding over the fake whose `send_direct` to a held peer
//! waits until the test lets it go. What it does not show: a real
//! exchange's timeout (`tests/desktop-e2e`'s `fleet::fan_out` run times
//! one, frozen with SIGSTOP).

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{
    ClientConfig, ClientEvent, Destination, OutboundStatus, SessionState, TransportClient,
};
use interweave_local_client_api::{
    DataCapability, DataSessionBinding, DataSessionPort, SessionEvent, SessionLosses,
    SessionRequest,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{
    BroadcastMessageV1, ChannelId, DirectDestination, EndpointDirectoryV1, EndpointId, MediaType,
    MessageId, Payload, TransportError, TransportIdentity,
};

/// The flight cap, as the crate states it; a change there is a change
/// here, on purpose.
const FLIGHT_CAP: usize = 32;

fn human() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn agent() -> EndpointId {
    EndpointId::parse("agent").expect("valid")
}

fn room() -> ChannelId {
    ChannelId::parse("room").expect("valid")
}

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn node_config() -> FakeConfig {
    FakeConfig {
        peer: peer(),
        endpoints: vec![
            FakeEndpoint::open(human(), true),
            FakeEndpoint::open(agent(), true),
        ],
        default_endpoint: Some(human()),
        queue_bound: 64,
    }
}

/// What the held sends share with the test.
#[derive(Default)]
struct Gate {
    /// Peers whose sends wait.
    held: Mutex<BTreeSet<TransportIdentity>>,
    released: tokio::sync::Notify,
    /// Sends begun, and sends begun and not yet answered.
    started: AtomicUsize,
    open: AtomicUsize,
    most_open: AtomicUsize,
}

impl Gate {
    fn hold(&self, peer: &TransportIdentity) {
        self.held.lock().expect("lock").insert(peer.clone());
    }

    fn release(&self, peer: &TransportIdentity) {
        self.held.lock().expect("lock").remove(peer);
        self.released.notify_waiters();
    }

    fn is_held(&self, peer: &TransportIdentity) -> bool {
        self.held.lock().expect("lock").contains(peer)
    }
}

#[derive(Clone)]
struct Holding {
    node: FakeNode,
    gate: Arc<Gate>,
}

struct HoldingSession {
    inner: <FakeNode as DataSessionBinding>::Session,
    gate: Arc<Gate>,
}

impl DataSessionBinding for Holding {
    type Session = HoldingSession;

    async fn open(&self, request: SessionRequest) -> Result<HoldingSession, TransportError> {
        Ok(HoldingSession {
            inner: self.node.open(request).await?,
            gate: Arc::clone(&self.gate),
        })
    }
}

impl DataSessionPort for HoldingSession {
    fn session(&self) -> &interweave_local_client_api::LocalDataSession {
        self.inner.session()
    }

    async fn join(&self, channel: ChannelId) -> Result<(), TransportError> {
        self.inner.join(channel).await
    }

    async fn leave(&self, channel: ChannelId) -> Result<(), TransportError> {
        self.inner.leave(channel).await
    }

    async fn broadcast(
        &self,
        channel: ChannelId,
        message: BroadcastMessageV1,
    ) -> Result<(), TransportError> {
        self.inner.broadcast(channel, message).await
    }

    async fn send_direct(
        &self,
        destination: DirectDestination,
        message_id: MessageId,
        payload: Payload,
    ) -> Result<EndpointId, TransportError> {
        /// Counts the send open until it answers or is dropped: a closed
        /// session drops the attempts it takes off the wire.
        struct Open<'g>(&'g Gate);
        impl Drop for Open<'_> {
            fn drop(&mut self) {
                self.0.open.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let gate = &self.gate;
        gate.started.fetch_add(1, Ordering::SeqCst);
        let open = gate.open.fetch_add(1, Ordering::SeqCst) + 1;
        gate.most_open.fetch_max(open, Ordering::SeqCst);
        let _open = Open(gate);
        loop {
            let released = gate.released.notified();
            if !gate.is_held(&destination.peer) {
                break;
            }
            released.await;
        }
        self.inner
            .send_direct(destination, message_id, payload)
            .await
    }

    async fn events(&self, max: usize) -> Result<Vec<SessionEvent>, TransportError> {
        self.inner.events(max).await
    }

    async fn ready(&self) -> Result<(), TransportError> {
        self.inner.ready().await
    }

    async fn query_endpoints(
        &self,
        peer: TransportIdentity,
    ) -> Result<EndpointDirectoryV1, TransportError> {
        self.inner.query_endpoints(peer).await
    }

    async fn losses(&self) -> Result<SessionLosses, TransportError> {
        self.inner.losses().await
    }

    async fn close(self) -> Result<(), TransportError> {
        self.inner.close().await
    }
}

type Client = TransportClient<Holding, FakeNode>;

fn client(node: &FakeNode, gate: &Arc<Gate>) -> Client {
    TransportClient::new(
        Holding {
            node: node.clone(),
            gate: Arc::clone(gate),
        },
        node.clone(),
        HumanStore::open_in_memory(StoreOptions::default()).expect("store"),
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![human()],
            channels: vec![room()],
            max_payload_bytes: 49_152,
        },
        Box::new(|| 1_786_600_000_000),
        0,
    )
    .expect("pending rows read")
}

fn envelope(text: &str) -> HumanChatV2 {
    use std::sync::atomic::AtomicU64;
    static NEXT: AtomicU64 = AtomicU64::new(1);
    HumanChatV2 {
        v: 2,
        kind: MessageKind::Text,
        app_message_id: format!(
            "{:032x}",
            u128::from(NEXT.fetch_add(1, Ordering::Relaxed)) << 64 | 0xf1
        ),
        text: text.to_owned(),
        reply_to: None,
        sent_at_ms: None,
        from_endpoint: None,
    }
}

/// `peer`'s `endpoint`, named.
fn at(peer: &TransportIdentity, endpoint: EndpointId) -> Destination {
    Destination::Direct {
        peer: peer.clone(),
        endpoint: Some(endpoint),
    }
}

fn to(peer: &TransportIdentity) -> Destination {
    Destination::Direct {
        peer: peer.clone(),
        endpoint: None,
    }
}

/// The statuses reported since the last call, by row.
fn statuses(c: &mut Client) -> Vec<(interweave_human_store::RowId, OutboundStatus)> {
    std::iter::from_fn(|| c.next_event())
        .filter_map(|e| match e {
            ClientEvent::Outbound(u) => Some((u.row, u.status)),
            _ => None,
        })
        .collect()
}

fn accepted(
    seen: &[(interweave_human_store::RowId, OutboundStatus)],
) -> Vec<interweave_human_store::RowId> {
    seen.iter()
        .filter(|(_, s)| matches!(s, OutboundStatus::Accepted { .. }))
        .map(|(row, _)| *row)
        .collect()
}

/// A raw session on `node` holding `human`, as the receiving client.
async fn receiver(node: &FakeNode) -> impl DataSessionPort {
    node.open(
        SessionRequest::new(
            "raw",
            Some(human()),
            [DataCapability::Commands, DataCapability::Events],
        )
        .expect("bounds"),
    )
    .await
    .expect("opens")
}

/// The texts of the direct messages `session` holds, in arrival order.
async fn texts(session: &impl DataSessionPort) -> Vec<String> {
    session
        .events(usize::MAX)
        .await
        .expect("events")
        .into_iter()
        .filter_map(|e| match e {
            SessionEvent::Direct(d) => {
                let text = std::str::from_utf8(d.payload.bytes()).expect("utf-8");
                Some(HumanChatV2::parse(text).expect("an envelope").text)
            }
            _ => None,
        })
        .collect()
}

async fn ready(c: &mut Client) {
    c.tick(0).await;
    assert!(
        matches!(c.session_state(), SessionState::Ready { .. }),
        "{:?}",
        c.session_state()
    );
}

/// A send to a silent peer is on the wire while a later send to another
/// peer is made and accepted, and while inbound is taken. Mutation: the
/// attempt awaited in the tick (the facade before j288) -- the tick never
/// returns while the peer is silent.
#[tokio::test]
async fn a_silent_peer_holds_no_send_to_another_peer() {
    let (a, b) = FakeNetwork::pair(node_config(), node_config());
    let gate = Arc::new(Gate::default());
    let silent = peer();
    gate.hold(&silent);
    let at_a = receiver(&a).await;
    let mut sender = client(&b, &gate);
    ready(&mut sender).await;

    let held = sender
        .send(to(&silent), &envelope("to the silent one"), 1)
        .await
        .expect("row");
    tokio::time::timeout(std::time::Duration::from_secs(2), Box::pin(sender.tick(1)))
        .await
        .expect("the tick returns while the attempt is held");
    let answered = sender
        .send(to(a.peer()), &envelope("to a"), 2)
        .await
        .expect("row");
    tokio::time::timeout(std::time::Duration::from_secs(2), Box::pin(sender.tick(2)))
        .await
        .expect("the tick returns");
    let seen = statuses(&mut sender);
    assert_eq!(accepted(&seen), [answered], "{seen:?}");
    assert!(!accepted(&seen).contains(&held));
    assert_eq!(texts(&at_a).await, ["to a"]);

    // Inbound is taken while the attempt is still held.
    let from_a = DirectDestination::to_default(b.peer().clone());
    at_a.send_direct(
        from_a,
        MessageId::from_bytes([7; 16]),
        Payload::at_ceiling(
            Some(
                MediaType::parse("application/vnd.interweave-human-chat+json;v=2").expect("valid"),
            ),
            serde_json::to_vec(&envelope("back from a")).expect("json"),
        )
        .expect("fits"),
    )
    .await
    .expect("queued");
    let drained = sender.drain(16, 3).await;
    assert_eq!(drained.len(), 1, "taken while a send is held");
    assert_eq!(
        gate.open.load(Ordering::SeqCst),
        1,
        "the held one still open"
    );

    gate.release(&silent);
    tokio::task::yield_now().await;
    sender.tick(4).await;
    let seen = statuses(&mut sender);
    assert_ne!(
        seen.iter().find(|(row, _)| *row == held).map(|(_, s)| s),
        None,
        "the held send lands once it is answered: {seen:?}"
    );
}

/// Two lines to one peer go one at a time, in the order written: the
/// second is not on the wire while the first is, and arrives after it.
/// Mutation: no one-per-destination rule -- the second starts beside the
/// first (`started` is 2 while the first is held).
#[tokio::test]
async fn two_lines_to_one_peer_arrive_in_the_order_written() {
    let (a, b) = FakeNetwork::pair(node_config(), node_config());
    let gate = Arc::new(Gate::default());
    let at_a = receiver(&a).await;
    let mut sender = client(&b, &gate);
    ready(&mut sender).await;
    gate.hold(a.peer());
    let first = sender
        .send(to(a.peer()), &envelope("first"), 1)
        .await
        .expect("row");
    let second = sender
        .send(to(a.peer()), &envelope("second"), 1)
        .await
        .expect("row");
    sender.tick(1).await;
    sender.tick(2).await;
    assert_eq!(gate.started.load(Ordering::SeqCst), 1, "one at a time to a");
    gate.release(a.peer());
    tokio::task::yield_now().await;
    sender.tick(3).await;
    sender.tick(4).await;
    let seen = statuses(&mut sender);
    assert_eq!(accepted(&seen), [first, second], "{seen:?}");
    assert_eq!(texts(&at_a).await, ["first", "second"]);
}

/// At most `FLIGHT_CAP` sends are on the wire at once, and the next one
/// starts once one lands. Mutation: no cap -- every silent peer's send
/// starts at once.
#[tokio::test]
async fn at_most_the_flight_cap_of_sends_is_in_flight() {
    let (_a, b) = FakeNetwork::pair(node_config(), node_config());
    let gate = Arc::new(Gate::default());
    let mut sender = client(&b, &gate);
    ready(&mut sender).await;
    let silent: Vec<TransportIdentity> = (0..=FLIGHT_CAP).map(|_| peer()).collect();
    for p in &silent {
        gate.hold(p);
        sender.send(to(p), &envelope("held"), 1).await.expect("row");
    }
    sender.tick(1).await;
    sender.tick(2).await;
    assert_eq!(gate.started.load(Ordering::SeqCst), FLIGHT_CAP);
    assert_eq!(gate.most_open.load(Ordering::SeqCst), FLIGHT_CAP);
    // One answered (refused: the stranger is no peer of the fake's), and
    // the one past the cap starts.
    gate.release(&silent[0]);
    tokio::task::yield_now().await;
    sender.tick(3).await;
    assert_eq!(gate.started.load(Ordering::SeqCst), FLIGHT_CAP + 1);
    assert!(gate.most_open.load(Ordering::SeqCst) <= FLIGHT_CAP);
}

/// A row waiting out its backoff holds the later row of its conversation
/// (`<peer>/<endpoint>`): after the first line's attempt is refused (not
/// reached), the second is not attempted before the first's retry, and
/// the endpoint gets them in the order written. Mutation: a conversation
/// blocked only while a row is on the wire -- the second goes during the
/// first's backoff.
#[tokio::test]
async fn a_row_waiting_its_backoff_holds_the_later_row_to_its_peer() {
    let (a, b) = FakeNetwork::pair(node_config(), node_config());
    let gate = Arc::new(Gate::default());
    let at_a = receiver(&a).await;
    let mut sender = client(&b, &gate);
    ready(&mut sender).await;
    b.inject_send(TransportError::Overloaded);
    let first = sender
        .send(at(a.peer(), human()), &envelope("first"), 1)
        .await
        .expect("row");
    let second = sender
        .send(at(a.peer(), human()), &envelope("second"), 1)
        .await
        .expect("row");
    sender.tick(1).await;
    sender.tick(2).await;
    assert_eq!(
        gate.started.load(Ordering::SeqCst),
        1,
        "the second waits behind the first's backoff"
    );
    assert!(texts(&at_a).await.is_empty());
    for now in [100_000, 100_001, 100_002] {
        sender.tick(now).await;
    }
    let seen = statuses(&mut sender);
    assert_eq!(accepted(&seen), [first, second], "{seen:?}");
    assert_eq!(texts(&at_a).await, ["first", "second"]);
}

/// A session lost while a send is on the wire takes that send off it: its
/// row is outcome-unknown and due again, and after the re-open it is sent
/// once. Mutation: the close drops the attempts without failing their
/// rows -- the row is never due again and never sent.
#[tokio::test]
async fn a_session_lost_with_a_send_on_the_wire_retries_it() {
    let (a, b) = FakeNetwork::pair(node_config(), node_config());
    let gate = Arc::new(Gate::default());
    let at_a = receiver(&a).await;
    let mut sender = client(&b, &gate);
    ready(&mut sender).await;
    gate.hold(a.peer());
    let held = sender
        .send(to(a.peer()), &envelope("held"), 1)
        .await
        .expect("row");
    sender.tick(1).await;
    assert_eq!(gate.open.load(Ordering::SeqCst), 1, "on the wire");
    // A send to another peer answers a lost lease: the session goes.
    b.inject_send(TransportError::EndpointNotRegistered);
    sender
        .send(to(&peer()), &envelope("loses the lease"), 2)
        .await
        .expect("row");
    sender.tick(2).await;
    assert!(
        matches!(sender.session_state(), SessionState::Reconnecting { .. }),
        "{:?}",
        sender.session_state()
    );
    let seen = statuses(&mut sender);
    let last_of_held = seen
        .iter()
        .rev()
        .find(|(row, _)| *row == held)
        .map(|(_, s)| s.clone());
    assert!(
        matches!(
            last_of_held,
            Some(OutboundStatus::Unconfirmed {
                next_retry_at: Some(_),
                ..
            })
        ),
        "outcome unknown and due again: {seen:?}"
    );
    assert_eq!(gate.open.load(Ordering::SeqCst), 0, "off the wire");
    gate.release(a.peer());
    for now in [100_000, 200_000, 300_000] {
        sender.tick(now).await;
    }
    let seen = statuses(&mut sender);
    assert!(
        accepted(&seen).contains(&held),
        "sent after the re-open: {seen:?}"
    );
    assert_eq!(texts(&at_a).await, ["held"], "once");
}

/// A row cancelled while its attempt is on the wire is cancelled as one
/// that may have reached, and the attempt's late answer changes nothing.
/// Mutation: the cancel reads only the row's own may-have-reached -- it
/// says the cancelled line did not reach.
#[tokio::test]
async fn a_row_cancelled_on_the_wire_is_cancelled_as_may_have_reached() {
    let (a, b) = FakeNetwork::pair(node_config(), node_config());
    let gate = Arc::new(Gate::default());
    let _at_a = receiver(&a).await;
    let mut sender = client(&b, &gate);
    ready(&mut sender).await;
    gate.hold(a.peer());
    let row = sender
        .send(to(a.peer()), &envelope("cancelled on the wire"), 1)
        .await
        .expect("row");
    sender.tick(1).await;
    let _ = statuses(&mut sender);
    sender.cancel(row).expect("cancelled");
    let seen = statuses(&mut sender);
    assert!(
        seen.iter().any(|(r, s)| *r == row
            && matches!(
                s,
                OutboundStatus::Cancelled {
                    may_have_reached: true
                }
            )),
        "{seen:?}"
    );
    gate.release(a.peer());
    tokio::task::yield_now().await;
    sender.tick(2).await;
    sender.tick(3).await;
    let seen = statuses(&mut sender);
    assert!(
        seen.iter().all(|(r, _)| *r != row),
        "the late answer changes nothing: {seen:?}"
    );
}

/// An endpoint that never takes a line holds only its own conversation:
/// a row to the peer's other endpoint goes and is accepted while the dead
/// one retries (architect-cto's ruling 01a1e96d). Mutation: the order
/// unit the peer -- the live endpoint's line waits behind the dead one.
#[tokio::test]
async fn an_endpoint_that_never_takes_holds_only_its_own_conversation() {
    let (a, b) = FakeNetwork::pair(node_config(), node_config());
    let gate = Arc::new(Gate::default());
    // Only `human` is held at a: `agent` answers that no client holds it.
    let at_a = receiver(&a).await;
    let mut sender = client(&b, &gate);
    ready(&mut sender).await;
    let dead = sender
        .send(at(a.peer(), agent()), &envelope("to nobody"), 1)
        .await
        .expect("row");
    let live = sender
        .send(at(a.peer(), human()), &envelope("to the live one"), 1)
        .await
        .expect("row");
    for now in [1, 2, 3] {
        sender.tick(now).await;
    }
    let seen = statuses(&mut sender);
    assert_eq!(accepted(&seen), [live], "{seen:?}");
    assert!(!accepted(&seen).contains(&dead));
    assert_eq!(texts(&at_a).await, ["to the live one"]);
}

/// A person's Retry on a row waiting behind an earlier row's backoff
/// brings the conversation due: the head goes on the next tick, the row
/// after it, in order. Mutation: Retry making only its own row due --
/// nothing goes until the head's backoff ends.
#[tokio::test]
async fn a_retry_behind_a_backoff_brings_its_conversation_due() {
    let (a, b) = FakeNetwork::pair(node_config(), node_config());
    let gate = Arc::new(Gate::default());
    let at_a = receiver(&a).await;
    let mut sender = client(&b, &gate);
    ready(&mut sender).await;
    b.inject_send(TransportError::Overloaded);
    let first = sender
        .send(at(a.peer(), human()), &envelope("first"), 1)
        .await
        .expect("row");
    let second = sender
        .send(at(a.peer(), human()), &envelope("second"), 1)
        .await
        .expect("row");
    sender.tick(1).await;
    assert_eq!(gate.started.load(Ordering::SeqCst), 1);
    sender.retry(second, 2).expect("pending");
    sender.tick(2).await;
    sender.tick(3).await;
    let seen = statuses(&mut sender);
    assert_eq!(accepted(&seen), [first, second], "{seen:?}");
    assert_eq!(texts(&at_a).await, ["first", "second"]);
}

/// A closing session settles the attempts that have answered before it
/// takes the rest off the wire: a broadcast already published when a
/// direct send's lost lease closes the session is reported published,
/// once, not outcome-unknown. Mutation: the close dropping every attempt
/// -- the broadcast reads `Unconfirmed` and is sent again.
#[tokio::test]
async fn a_close_settles_the_attempts_that_had_answered() {
    let (a, b) = FakeNetwork::pair(node_config(), node_config());
    let gate = Arc::new(Gate::default());
    let _at_a = receiver(&a).await;
    let mut sender = client(&b, &gate);
    ready(&mut sender).await;
    // The direct row first: its attempt is polled first and takes the
    // injected lost lease; the broadcast has published by then.
    b.inject_send(TransportError::EndpointNotRegistered);
    sender
        .send(at(a.peer(), human()), &envelope("loses the lease"), 1)
        .await
        .expect("row");
    let published = sender
        .send(Destination::Broadcast(room()), &envelope("published"), 1)
        .await
        .expect("row");
    sender.tick(1).await;
    assert!(
        matches!(sender.session_state(), SessionState::Reconnecting { .. }),
        "{:?}",
        sender.session_state()
    );
    let seen = statuses(&mut sender);
    let of_published: Vec<&OutboundStatus> = seen
        .iter()
        .filter(|(row, _)| *row == published)
        .map(|(_, s)| s)
        .collect();
    assert!(
        matches!(of_published.last(), Some(OutboundStatus::Published)),
        "settled by its answer: {seen:?}"
    );
}

/// A row held for the person's attention holds nothing behind it: the
/// next line of its conversation goes and is accepted while the first
/// waits for the person. Mutation: the pass letting an attention row
/// claim its conversation -- the second line is never sent.
#[tokio::test]
async fn a_row_held_for_attention_holds_nothing_behind_it() {
    let (a, b) = FakeNetwork::pair(node_config(), node_config());
    let gate = Arc::new(Gate::default());
    let at_a = receiver(&a).await;
    let mut sender = client(&b, &gate);
    ready(&mut sender).await;
    b.inject_send(TransportError::UnauthorizedPeer);
    let held = sender
        .send(at(a.peer(), human()), &envelope("for the person"), 1)
        .await
        .expect("row");
    let after = sender
        .send(at(a.peer(), human()), &envelope("after it"), 1)
        .await
        .expect("row");
    for now in [1, 2, 3] {
        sender.tick(now).await;
    }
    let seen = statuses(&mut sender);
    assert!(
        seen.iter()
            .any(|(r, s)| *r == held && matches!(s, OutboundStatus::NeedsAttention { .. })),
        "{seen:?}"
    );
    assert_eq!(accepted(&seen), [after], "{seen:?}");
    assert_eq!(texts(&at_a).await, ["after it"]);
}
