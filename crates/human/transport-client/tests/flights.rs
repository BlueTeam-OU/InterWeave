// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Send attempts on the wire beside each other, one per destination and
//! at most `FLIGHT_CAP` in all (architect-cto's ruling 01a1e901, j288): a
//! peer that never answers holds no line to another peer, no inbound, and
//! no later line to itself out of order.
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

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn node_config() -> FakeConfig {
    FakeConfig {
        peer: peer(),
        endpoints: vec![FakeEndpoint::open(human(), true)],
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
        let gate = &self.gate;
        gate.started.fetch_add(1, Ordering::SeqCst);
        let open = gate.open.fetch_add(1, Ordering::SeqCst) + 1;
        gate.most_open.fetch_max(open, Ordering::SeqCst);
        loop {
            let released = gate.released.notified();
            if !gate.is_held(&destination.peer) {
                break;
            }
            released.await;
        }
        let answer = self
            .inner
            .send_direct(destination, message_id, payload)
            .await;
        gate.open.fetch_sub(1, Ordering::SeqCst);
        answer
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
            channels: Vec::new(),
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
