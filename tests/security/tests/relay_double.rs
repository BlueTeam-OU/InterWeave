// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A misbehaving relay in front of the real substrate, over real sockets
//! (plan section 21, S7; `tests/SECURITY-MATRIX.md` row S21-07).
//!
//! The relay tests elsewhere run libp2p's own relay server, which
//! answers every hop request correctly or refuses it with a status.
//! These put a relay that does neither in front of a profile that
//! relies on it:
//!
//! - a relay that accepts the hop stream and never answers RESERVE: the
//!   ask holds its place in the reservation target until libp2p-relay
//!   0.22.0's client gives the stream up (`STREAM_TIMEOUT`, 60 s,
//!   `priv_client/handler.rs`), and only then is the spare asked;
//! - a relay that answers RESERVE with a frame that does not decode: the
//!   ask fails at once and the spare is asked;
//! - a relay that answers CONNECT with a frame that does not decode: the
//!   dial fails at once, and the same dialer reaches the same
//!   destination through an honest relay;
//! - a relay that stops with a circuit open -- its process stopped, its
//!   sockets open, nothing forwarded and nothing closed: a send through
//!   it is answered `PeerUnreachable` at the direct protocol's timeout,
//!   and the end holding a reservation there finds the relay gone by
//!   its keepalive and is told the circuit's peer is gone.
//!
//! Each negative has its positive beside it in the same run: the
//! honest relay that is reserved on, or dialled through, once the
//! misbehaving one has been given up, and the send the circuit carried
//! before its relay stopped.
//!
//! What is NOT proved, and why (`tests/SECURITY-MATRIX.md` S21-07):
//! the DIALLING end of a circuit through a stopped relay is not told --
//! it holds no reservation there, so nothing pings that relay from it,
//! and the peer stays `Connected` (two minutes in the test, which pins
//! it as a limit; five in one measured run); and a relay that does not
//! speak ping is not found stopped by either end, since the keepalive
//! keeps such a relay rather than closing it
//! (`crates/transport/libp2p/tests/relay_keepalive.rs`; measured once
//! here for five minutes, not a test).
//! Every address is loopback, so nothing here speaks for a NAT.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use futures::StreamExt as _;
use interweave_local_client_api::EndpointLease;
use interweave_profile_config::{
    ChannelsConfig, DirectoryConfig, EndpointConfig, EndpointsConfig, ProfileConfig,
    RegistrationPolicy, TrustConfig, TrustPolicyKind,
};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{
    DirectMessageV2, EndpointId, MediaType, MessageId, Payload, TransportError, TransportIdentity,
};
use interweave_transport_libp2p::behaviour::DIRECT_TIMEOUT;
use interweave_transport_libp2p::runtime::DirectEndpoints;
use interweave_transport_libp2p::runtime::relay_driver::{
    CRATE_RESERVE_TIMEOUT_MS, RelayClientSettings, StaticRelay,
};
use interweave_transport_libp2p::{
    PeerPath, RelayReservationOutcome, SubstrateConfig, SwarmEvent, SwarmRuntime,
};
use interweave_transport_runtime::TrustSources;
use interweave_transport_runtime::relay::ReservationConfig;
use interweave_trust_api::{EndpointTrustPolicy, InfrastructureSet, PeerTrustPolicy};
use libp2p::swarm::NetworkBehaviour;
use libp2p::swarm::behaviour::toggle::Toggle;
use libp2p::{Multiaddr, identify, identity, ping, relay};
use tokio::sync::mpsc;

const PATIENCE: Duration = Duration::from_secs(20);

/// libp2p-relay 0.22.0's bound on a reservation request
/// (`priv_client/handler.rs`, `STREAM_TIMEOUT`), as the relay driver
/// restates it beside the horizon it outlasts.
const CRATE_STREAM_TIMEOUT: Duration = Duration::from_millis(CRATE_RESERVE_TIMEOUT_MS);

#[derive(NetworkBehaviour)]
struct RelayBehaviour {
    identify: identify::Behaviour,
    relay: relay::Behaviour,
    /// Present on a relay that answers the keepalive's pings, as this
    /// project's relay server does; absent on libp2p's bare relay.
    ping: Toggle<ping::Behaviour>,
}

fn identity_of(keys: &identity::Keypair) -> TransportIdentity {
    TransportIdentity::parse(keys.public().to_peer_id().to_base58()).expect("canonical")
}

fn trust(data_plane: &[&TransportIdentity], infra: &[&TransportIdentity]) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new(data_plane.iter().map(|p| (*p).clone())).expect("a small allowlist"),
        InfrastructureSet::new(infra.iter().map(|p| (*p).clone())).expect("a small set"),
    )
}

/// A relay client that wants ONE reservation, so a misbehaving relay
/// asked first decides alone when the spare is asked.
fn one_reservation(static_relays: Vec<StaticRelay>) -> RelayClientSettings {
    RelayClientSettings {
        static_relays,
        use_authorized_identify_relays: false,
        reservations: ReservationConfig {
            target_private_or_unknown: 1,
            target_public: 1,
            ..ReservationConfig::default()
        },
        direct_head_start_ms: 750,
    }
}

fn static_relay(peer: &TransportIdentity, address: &Multiaddr) -> StaticRelay {
    StaticRelay {
        peer: peer.clone(),
        address: format!("{address}/p2p/{}", peer.as_str()),
    }
}

fn circuit_of(
    relay_addr: &Multiaddr,
    relay: &TransportIdentity,
    to: &TransportIdentity,
) -> Multiaddr {
    format!(
        "{relay_addr}/p2p/{}/p2p-circuit/p2p/{}",
        relay.as_str(),
        to.as_str()
    )
    .parse()
    .expect("a circuit address")
}

fn relay_swarm(keys: identity::Keypair, speaks_ping: bool) -> libp2p::Swarm<RelayBehaviour> {
    libp2p::SwarmBuilder::with_existing_identity(keys)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("the same transport stack the subject uses")
        .with_behaviour(|k| RelayBehaviour {
            identify: identify::Behaviour::new(identify::Config::new(
                "/interweave-security-test-relay/1".to_owned(),
                k.public(),
            )),
            relay: relay::Behaviour::new(k.public().to_peer_id(), relay::Config::default()),
            ping: Toggle::from(speaks_ping.then(|| ping::Behaviour::new(ping::Config::new()))),
        })
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
        .build()
}

async fn bound<B: NetworkBehaviour>(swarm: &mut libp2p::Swarm<B>) -> Multiaddr {
    swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().expect("a listen address"))
        .expect("listens");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        match tokio::time::timeout_at(deadline, swarm.select_next_some()).await {
            Ok(libp2p::swarm::SwarmEvent::NewListenAddr { address, .. }) => return address,
            Ok(_) => {}
            Err(elapsed) => panic!("the listener never bound ({elapsed})"),
        }
    }
}

/// libp2p's relay server, honest and with its defaults, driven by a
/// task of its own.
async fn honest_relay(keys: identity::Keypair) -> Multiaddr {
    let mut swarm = relay_swarm(keys, false);
    let address = bound(&mut swarm).await;
    swarm.add_external_address(address.clone());
    tokio::spawn(async move {
        loop {
            swarm.select_next_some().await;
        }
    });
    address
}

/// libp2p's relay server on a thread and a single-threaded runtime of
/// its own, so that the whole relay -- its Swarm and every connection
/// task, which libp2p spawns on the runtime it runs on -- can be STOPPED
/// at once: what a process stopped by a signal, or wedged, looks like
/// from outside. Its sockets stay open and the kernel goes on
/// accepting bytes into them; nothing is forwarded and nothing closed.
fn stoppable_relay(keys: identity::Keypair, speaks_ping: bool) -> (Multiaddr, Stopper) {
    let (addr_tx, addr_rx) = std::sync::mpsc::channel();
    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
    let (stopped_tx, stopped_rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        runtime.block_on(async move {
            let mut swarm = relay_swarm(keys, speaks_ping);
            let address = bound(&mut swarm).await;
            swarm.add_external_address(address.clone());
            addr_tx.send(address).expect("the test waits for the address");
            let mut tick = tokio::time::interval(Duration::from_millis(50));
            loop {
                tokio::select! {
                    _ = swarm.select_next_some() => {}
                    _ = tick.tick() => {
                        if matches!(stop_rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Disconnected)) {
                            // Block the runtime's only thread: no task of
                            // this relay is polled again, and its sockets
                            // are never closed.
                            let _ = stopped_tx.send(());
                            loop {
                                std::thread::park();
                            }
                        }
                    }
                }
            }
        });
    });
    let address = addr_rx
        .recv_timeout(PATIENCE)
        .expect("the stoppable relay binds");
    (
        address,
        Stopper {
            stop: stop_tx,
            stopped: stopped_rx,
        },
    )
}

/// Stops a [`stoppable_relay`].
struct Stopper {
    stop: std::sync::mpsc::Sender<()>,
    stopped: std::sync::mpsc::Receiver<()>,
}

impl Stopper {
    /// Stop the relay, and return once its thread is blocked: nothing
    /// sent after this is forwarded.
    fn stop(self) {
        drop(self.stop);
        self.stopped
            .recv_timeout(PATIENCE)
            .expect("the relay's thread reports it is stopped");
    }
}

/// What a double does with the hop stream it accepts.
#[derive(Clone, Copy, Debug)]
enum Misbehaviour {
    /// Hold it open and never write.
    Silent,
    /// Write `bytes` and hold it open.
    Answer(&'static [u8]),
}

/// A length prefix of one byte, then three bytes that are no
/// `HopMessage`: field 31 with a reserved wire type.
const UNDECODABLE: &[u8] = &[0x03, 0xff, 0xff, 0xff];

/// A relay double: it completes the handshake, accepts every inbound
/// stream on the hop protocol, and does with each what it is told,
/// reporting each one it accepted on `accepted`. It runs no relay.
async fn relay_double(
    keys: identity::Keypair,
    misbehaviour: Misbehaviour,
) -> (Multiaddr, mpsc::UnboundedReceiver<tokio::time::Instant>) {
    #[derive(NetworkBehaviour)]
    struct Double {
        identify: identify::Behaviour,
        hop: acceptor::Behaviour,
    }
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keys)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("the same transport stack the subject uses")
        .with_behaviour(|k| Double {
            identify: identify::Behaviour::new(identify::Config::new(
                "/interweave-security-test-relay/1".to_owned(),
                k.public(),
            )),
            hop: acceptor::Behaviour::new(relay::HOP_PROTOCOL_NAME),
        })
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
        .build();
    let address = bound(&mut swarm).await;
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        // Held for the life of the double, so a Silent stream is never
        // closed by being dropped.
        let mut held = Vec::new();
        loop {
            if let libp2p::swarm::SwarmEvent::Behaviour(DoubleEvent::Hop(mut stream)) =
                swarm.select_next_some().await
            {
                let _ = tx.send(tokio::time::Instant::now());
                if let Misbehaviour::Answer(bytes) = misbehaviour {
                    use futures::AsyncWriteExt as _;
                    stream.write_all(bytes).await.expect("the double writes");
                    stream.flush().await.expect("the double flushes");
                }
                held.push(stream);
            }
        }
    });
    (address, rx)
}

/// An inbound-only stream acceptor for one protocol.
mod acceptor {
    use std::collections::VecDeque;
    use std::task::{Context, Poll};

    use libp2p::core::transport::PortUse;
    use libp2p::core::upgrade::{DeniedUpgrade, ReadyUpgrade};
    use libp2p::core::{Endpoint, Multiaddr};
    use libp2p::swarm::handler::{ConnectionEvent, FullyNegotiatedInbound};
    use libp2p::swarm::{
        ConnectionDenied, ConnectionHandler, ConnectionHandlerEvent, ConnectionId, FromSwarm,
        NetworkBehaviour, Stream, SubstreamProtocol, THandler, THandlerInEvent, THandlerOutEvent,
        ToSwarm,
    };
    use libp2p::{PeerId, StreamProtocol};

    pub(super) struct Behaviour {
        protocol: StreamProtocol,
        accepted: VecDeque<Stream>,
    }

    impl Behaviour {
        pub(super) fn new(protocol: StreamProtocol) -> Self {
            Self {
                protocol,
                accepted: VecDeque::new(),
            }
        }

        fn handler(&self) -> Handler {
            Handler {
                protocol: self.protocol.clone(),
                accepted: VecDeque::new(),
            }
        }
    }

    impl NetworkBehaviour for Behaviour {
        type ConnectionHandler = Handler;
        type ToSwarm = Stream;

        fn handle_established_inbound_connection(
            &mut self,
            _: ConnectionId,
            _: PeerId,
            _: &Multiaddr,
            _: &Multiaddr,
        ) -> Result<THandler<Self>, ConnectionDenied> {
            Ok(self.handler())
        }

        fn handle_established_outbound_connection(
            &mut self,
            _: ConnectionId,
            _: PeerId,
            _: &Multiaddr,
            _: Endpoint,
            _: PortUse,
        ) -> Result<THandler<Self>, ConnectionDenied> {
            Ok(self.handler())
        }

        fn on_connection_handler_event(
            &mut self,
            _: PeerId,
            _: ConnectionId,
            event: THandlerOutEvent<Self>,
        ) {
            self.accepted.push_back(event);
        }

        fn poll(
            &mut self,
            _: &mut Context<'_>,
        ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
            self.accepted
                .pop_front()
                .map_or(Poll::Pending, |s| Poll::Ready(ToSwarm::GenerateEvent(s)))
        }

        fn on_swarm_event(&mut self, _: FromSwarm<'_>) {}
    }

    pub(super) struct Handler {
        protocol: StreamProtocol,
        accepted: VecDeque<Stream>,
    }

    impl ConnectionHandler for Handler {
        type FromBehaviour = std::convert::Infallible;
        type ToBehaviour = Stream;
        type InboundProtocol = ReadyUpgrade<StreamProtocol>;
        type OutboundProtocol = DeniedUpgrade;
        type InboundOpenInfo = ();
        type OutboundOpenInfo = ();

        fn listen_protocol(&self) -> SubstreamProtocol<ReadyUpgrade<StreamProtocol>> {
            SubstreamProtocol::new(ReadyUpgrade::new(self.protocol.clone()), ())
        }

        fn connection_keep_alive(&self) -> bool {
            true
        }

        fn poll(
            &mut self,
            _: &mut Context<'_>,
        ) -> Poll<ConnectionHandlerEvent<DeniedUpgrade, (), Stream>> {
            self.accepted.pop_front().map_or(Poll::Pending, |s| {
                Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(s))
            })
        }

        fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
            match event {}
        }

        fn on_connection_event(
            &mut self,
            event: ConnectionEvent<'_, ReadyUpgrade<StreamProtocol>, DeniedUpgrade>,
        ) {
            if let ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound {
                protocol: stream,
                ..
            }) = event
            {
                self.accepted.push_back(stream);
            }
        }
    }
}

/// Two relay keypairs, the first ordering before the second as the
/// reservation manager orders static relays (by identity), so the
/// misbehaving one is the one asked first.
fn ordered_pair() -> (identity::Keypair, identity::Keypair) {
    loop {
        let a = identity::Keypair::generate_ed25519();
        let b = identity::Keypair::generate_ed25519();
        match identity_of(&a).cmp(&identity_of(&b)) {
            std::cmp::Ordering::Less => return (a, b),
            std::cmp::Ordering::Greater => return (b, a),
            std::cmp::Ordering::Equal => {}
        }
    }
}

/// A reservation event the subject reported, and when.
#[derive(Debug)]
struct Moved {
    at: tokio::time::Instant,
    relay: TransportIdentity,
    outcome: RelayReservationOutcome,
}

/// Drive `subject` until an `Accepted` from `honest` or `window` ends,
/// recording every reservation event and when the double was asked.
async fn reservations_until_honest(
    subject: &mut SwarmRuntime,
    honest: &TransportIdentity,
    asked: &mut mpsc::UnboundedReceiver<tokio::time::Instant>,
    window: Duration,
) -> (Vec<Moved>, Vec<tokio::time::Instant>) {
    let mut moved = Vec::new();
    let mut asks = Vec::new();
    let deadline = tokio::time::Instant::now() + window;
    loop {
        tokio::select! {
            event = subject.next_event() => {
                if let SwarmEvent::RelayReservationChanged { relay, outcome, .. } =
                    event.expect("the subject is alive")
                {
                    let done = relay == *honest && outcome == RelayReservationOutcome::Accepted;
                    moved.push(Moved { at: tokio::time::Instant::now(), relay, outcome });
                    if done {
                        return (moved, asks);
                    }
                }
            }
            Some(at) = asked.recv() => asks.push(at),
            () = tokio::time::sleep_until(deadline) => panic!(
                "no reservation on the honest relay within {window:?}; the double was asked at \
                 {asks:?}, and the subject reported {moved:#?}"
            ),
        }
    }
}

/// The subject: a relay client wanting one reservation, the
/// misbehaving relay first among its static relays and the honest one
/// second.
fn subject_reserving_on(
    double: (&TransportIdentity, &Multiaddr),
    honest: (&TransportIdentity, &Multiaddr),
) -> SwarmRuntime {
    let id = ProfileIdentity::generate();
    SwarmRuntime::start(
        &id,
        SubstrateConfig {
            relay_client: Some(one_reservation(vec![
                static_relay(double.0, double.1),
                static_relay(honest.0, honest.1),
            ])),
            ..SubstrateConfig::default()
        },
        trust(&[], &[double.0, honest.0]),
    )
    .expect("the subject starts")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_relay_that_never_answers_reserve_holds_the_ask_until_the_crates_timeout_and_then_the_spare_is_reserved()
 {
    let (double_keys, honest_keys) = ordered_pair();
    let (double_peer, honest_peer) = (identity_of(&double_keys), identity_of(&honest_keys));
    let (double_addr, mut asked) = relay_double(double_keys, Misbehaviour::Silent).await;
    let honest_addr = honest_relay(honest_keys).await;
    let mut subject =
        subject_reserving_on((&double_peer, &double_addr), (&honest_peer, &honest_addr));

    let (moved, asks) = reservations_until_honest(
        &mut subject,
        &honest_peer,
        &mut asked,
        CRATE_STREAM_TIMEOUT + PATIENCE,
    )
    .await;

    // The double was asked first, and once.
    let [asked_at] = asks[..] else {
        panic!("the double was asked once: {asks:?}; the subject reported {moved:#?}")
    };
    // Its ask failed, and nothing else happened before that failure:
    // while the ask was outstanding it held the reservation target, so
    // the spare was not asked.
    let first = moved.first().expect("at least the honest acceptance");
    assert!(
        first.relay == double_peer && first.outcome == RelayReservationOutcome::Failed,
        "the first thing the subject reported is the double's ask failing: {moved:#?}"
    );
    let held = first.at - asked_at;
    assert!(
        held + Duration::from_secs(2) >= CRATE_STREAM_TIMEOUT
            && held <= CRATE_STREAM_TIMEOUT + Duration::from_secs(10),
        "the silent ask is given up at the crate's stream timeout, not sooner and not never: \
         {held:?}; {moved:#?}"
    );
    // THE POSITIVE: the honest relay is reserved on soon after.
    let last = moved.last().expect("the honest acceptance");
    assert!(
        last.at - first.at < PATIENCE,
        "the spare is reserved within {PATIENCE:?} of the failure: {moved:#?}"
    );
    eprintln!("silent RESERVE: held {held:?}; reported {moved:#?}");
    subject.shutdown().await.expect("shuts down");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_relay_that_answers_reserve_with_an_undecodable_frame_fails_the_ask_at_once_and_the_spare_is_reserved()
 {
    let (double_keys, honest_keys) = ordered_pair();
    let (double_peer, honest_peer) = (identity_of(&double_keys), identity_of(&honest_keys));
    let (double_addr, mut asked) =
        relay_double(double_keys, Misbehaviour::Answer(UNDECODABLE)).await;
    let honest_addr = honest_relay(honest_keys).await;
    let mut subject =
        subject_reserving_on((&double_peer, &double_addr), (&honest_peer, &honest_addr));

    let (moved, asks) =
        reservations_until_honest(&mut subject, &honest_peer, &mut asked, PATIENCE).await;

    let [asked_at] = asks[..] else {
        panic!("the double was asked once: {asks:?}; the subject reported {moved:#?}")
    };
    let first = moved.first().expect("at least the honest acceptance");
    assert!(
        first.relay == double_peer && first.outcome == RelayReservationOutcome::Failed,
        "the first thing the subject reported is the double's ask failing: {moved:#?}"
    );
    assert!(
        first.at - asked_at < Duration::from_secs(5),
        "an undecodable answer fails the ask at once: {:?}; {moved:#?}",
        first.at - asked_at
    );
    eprintln!("undecodable RESERVE: reported {moved:#?}");
    subject.shutdown().await.expect("shuts down");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_relay_that_answers_connect_with_an_undecodable_frame_fails_the_dial_and_an_honest_circuit_connects()
 {
    let double_keys = identity::Keypair::generate_ed25519();
    let honest_keys = identity::Keypair::generate_ed25519();
    let (double_peer, honest_peer) = (identity_of(&double_keys), identity_of(&honest_keys));
    let (double_addr, mut asked) =
        relay_double(double_keys, Misbehaviour::Answer(UNDECODABLE)).await;
    let honest_addr = honest_relay(honest_keys).await;

    // The destination, reserved on the honest relay only.
    let target_id = ProfileIdentity::generate();
    let target_peer = target_id.transport_identity().expect("peer id");
    let source_id = ProfileIdentity::generate();
    let source_peer = source_id.transport_identity().expect("peer id");
    let mut target = SwarmRuntime::start(
        &target_id,
        SubstrateConfig {
            relay_client: Some(one_reservation(vec![static_relay(
                &honest_peer,
                &honest_addr,
            )])),
            ..SubstrateConfig::default()
        },
        trust(&[&source_peer], &[&honest_peer]),
    )
    .expect("the target starts");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        match tokio::time::timeout_at(deadline, target.next_event()).await {
            Ok(Some(SwarmEvent::RelayReservationChanged {
                outcome: RelayReservationOutcome::Accepted,
                ..
            })) => break,
            Ok(Some(_)) => {}
            Ok(None) => panic!("the target stopped"),
            Err(elapsed) => panic!("the target is never reserved on the honest relay ({elapsed})"),
        }
    }

    let mut source = SwarmRuntime::start(
        &source_id,
        SubstrateConfig {
            relay_client: Some(one_reservation(Vec::new())),
            ..SubstrateConfig::default()
        },
        trust(&[&target_peer], &[&double_peer, &honest_peer]),
    )
    .expect("the source starts");

    // THROUGH THE DOUBLE: its CONNECT is answered with garbage.
    let dialled_at = tokio::time::Instant::now();
    source
        .dial(
            target_peer.clone(),
            circuit_of(&double_addr, &double_peer, &target_peer),
        )
        .await
        .expect("the command reaches the task")
        .expect("a circuit to a data-plane peer is admitted");
    let mut seen = Vec::new();
    let failed = loop {
        tokio::select! {
            event = source.next_event() => match event.expect("the source is alive") {
                SwarmEvent::DialFailed { peer: Some(peer), detail, .. } if peer == target_peer => {
                    break detail;
                }
                SwarmEvent::Connected { peer, .. } if peer == target_peer => {
                    panic!("connected through a relay that answered garbage: {seen:?}")
                }
                other => seen.push(other),
            },
            event = target.next_event() => { event.expect("the target is alive"); }
            () = tokio::time::sleep_until(dialled_at + PATIENCE) => {
                panic!("the dial through the double never ended: {seen:?}")
            }
        }
    };
    assert!(
        asked.try_recv().is_ok(),
        "the double was asked: the failure is its answer, not a dial that never reached it"
    );
    assert!(
        dialled_at.elapsed() < Duration::from_secs(5),
        "an undecodable CONNECT answer fails the dial at once: {:?} ({failed})",
        dialled_at.elapsed()
    );

    // THE POSITIVE: the same source reaches the same target through the
    // honest relay.
    source
        .dial(
            target_peer.clone(),
            circuit_of(&honest_addr, &honest_peer, &target_peer),
        )
        .await
        .expect("the command reaches the task")
        .expect("admitted");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        tokio::select! {
            event = source.next_event() => match event.expect("the source is alive") {
                SwarmEvent::Connected { peer, path: PeerPath::Relayed } if peer == target_peer => break,
                SwarmEvent::DialFailed { peer: Some(peer), detail, .. } if peer == target_peer => {
                    panic!("the honest circuit failed too: {detail}")
                }
                _ => {}
            },
            event = target.next_event() => { event.expect("the target is alive"); }
            () = tokio::time::sleep_until(deadline) => panic!("the honest circuit never connected"),
        }
    }
    eprintln!("undecodable CONNECT: {failed}");
    source.shutdown().await.expect("shuts down");
    target.shutdown().await.expect("shuts down");
}

fn endpoint(name: &str) -> EndpointId {
    EndpointId::parse(name).expect("valid endpoint id")
}

/// One enabled endpoint, `human`, the default direct destination.
fn endpoints() -> DirectEndpoints {
    let profile = ProfileConfig {
        runtime: interweave_profile_config::runtime::RuntimeConfig::default(),
        ipc: interweave_profile_config::ipc::IpcConfig::default(),
        profile: None,
        identity: interweave_profile_config::sections::IdentityConfig::default(),
        observability: interweave_profile_config::sections::ObservabilityConfig::default(),
        transport: interweave_profile_config::connectivity::TransportConfig::default(),
        schema_version: 2,
        trust: TrustConfig {
            policy: TrustPolicyKind::default(),
            allowed_peers: std::collections::BTreeSet::new(),
        },
        endpoints: EndpointsConfig {
            registration_policy: RegistrationPolicy::default(),
            default_direct_endpoint: Some(endpoint("human")),
            directory: DirectoryConfig::default(),
            entries: vec![EndpointConfig {
                id: endpoint("human"),
                enabled: true,
                advertise: false,
                allowed_client_kinds: Vec::new(),
                inbound: EndpointTrustPolicy::default(),
                outbound: EndpointTrustPolicy::default(),
            }],
        },
        discovery: interweave_profile_config::DiscoveryConfig::default(),
        channels: ChannelsConfig::default(),
    };
    DirectEndpoints::from_profile(&profile, 8).expect("a valid profile")
}

async fn configure_human(runtime: &SwarmRuntime) -> EndpointLease {
    runtime
        .configure_direct(endpoints())
        .await
        .expect("the endpoints install");
    runtime
        .claim_endpoint("human", endpoint("human"), "in-process")
        .await
        .expect("the claim reaches the task")
        .expect("the endpoint is configured and free")
}

fn frame(id: u8) -> DirectMessageV2 {
    DirectMessageV2 {
        message_id: MessageId::from_bytes([id; 16]),
        sent_at_ms: 1_000,
        source_endpoint: endpoint("human"),
        destination_endpoint: None,
        payload: Payload::at_ceiling(
            Some(MediaType::parse("text/plain").expect("valid media type")),
            b"through the circuit".to_vec(),
        )
        .expect("within the ceiling"),
    }
}

/// A send from `source`, with `target` driven while it is outstanding.
async fn send_driving(
    source: &SwarmRuntime,
    lease: &EndpointLease,
    to: &TransportIdentity,
    id: u8,
    target: &mut SwarmRuntime,
    log: &mut Vec<String>,
) -> (Result<(), TransportError>, Duration) {
    let started = tokio::time::Instant::now();
    let send = source.send_direct(lease, to.clone(), frame(id));
    tokio::pin!(send);
    loop {
        tokio::select! {
            answer = &mut send => {
                let answer = answer.expect("the command reaches the task").map(|_| ());
                return (answer, started.elapsed());
            }
            event = target.next_event() => log.push(format!("target: {:?}", event.expect("alive"))),
        }
    }
}

/// What each end of a circuit was told after its relay stopped.
#[derive(Debug)]
struct AfterTheStop {
    /// The send made at once after the stop, and how long its answer took.
    send: (Result<(), TransportError>, Duration),
    /// When the source was told the target was gone.
    source_told: Option<Duration>,
    /// When the target was told the source was gone.
    target_told: Option<Duration>,
    /// When the target reported its reservation on the stopped relay lost.
    reservation_lost: Option<Duration>,
}

/// A target reserved on a stoppable relay, a source connected to it
/// through a circuit there and a send through it answered; then the
/// relay is stopped and both ends are watched for `horizon`.
async fn stop_the_relay_under_a_circuit(speaks_ping: bool, horizon: Duration) -> AfterTheStop {
    let relay_keys = identity::Keypair::generate_ed25519();
    let relay_peer = identity_of(&relay_keys);
    let (relay_addr, stop) = stoppable_relay(relay_keys, speaks_ping);

    let target_id = ProfileIdentity::generate();
    let target_peer = target_id.transport_identity().expect("peer id");
    let source_id = ProfileIdentity::generate();
    let source_peer = source_id.transport_identity().expect("peer id");
    let mut target = SwarmRuntime::start(
        &target_id,
        SubstrateConfig {
            relay_client: Some(one_reservation(vec![static_relay(
                &relay_peer,
                &relay_addr,
            )])),
            ..SubstrateConfig::default()
        },
        trust(&[&source_peer], &[&relay_peer]),
    )
    .expect("the target starts");
    let _target_lease = configure_human(&target).await;
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        match tokio::time::timeout_at(deadline, target.next_event()).await {
            Ok(Some(SwarmEvent::RelayReservationChanged {
                outcome: RelayReservationOutcome::Accepted,
                ..
            })) => break,
            Ok(Some(_)) => {}
            Ok(None) => panic!("the target stopped"),
            Err(elapsed) => panic!("the target is never reserved ({elapsed})"),
        }
    }
    let mut source = SwarmRuntime::start(
        &source_id,
        SubstrateConfig {
            relay_client: Some(one_reservation(Vec::new())),
            ..SubstrateConfig::default()
        },
        trust(&[&target_peer], &[&relay_peer]),
    )
    .expect("the source starts");
    let source_lease = configure_human(&source).await;
    source
        .dial(
            target_peer.clone(),
            circuit_of(&relay_addr, &relay_peer, &target_peer),
        )
        .await
        .expect("the command reaches the task")
        .expect("admitted");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        tokio::select! {
            event = source.next_event() => {
                if matches!(
                    event.expect("the source is alive"),
                    SwarmEvent::Connected { peer, path: PeerPath::Relayed } if peer == target_peer
                ) {
                    break;
                }
            }
            event = target.next_event() => { event.expect("the target is alive"); }
            () = tokio::time::sleep_until(deadline) => panic!("the circuit never connected"),
        }
    }
    // THE CONTROL: the circuit carries a send while the relay runs.
    let mut log = Vec::new();
    let (answer, _) = send_driving(
        &source,
        &source_lease,
        &target_peer,
        1,
        &mut target,
        &mut log,
    )
    .await;
    assert_eq!(answer, Ok(()), "the circuit carries a send: {log:#?}");

    // STOP THE RELAY with the circuit open.
    stop.stop();
    let stopped_at = tokio::time::Instant::now();
    let send = send_driving(
        &source,
        &source_lease,
        &target_peer,
        2,
        &mut target,
        &mut log,
    )
    .await;
    let (mut source_told, mut target_told, mut reservation_lost) = (None, None, None);
    while stopped_at.elapsed() < horizon
        && (source_told.is_none() || target_told.is_none() || reservation_lost.is_none())
    {
        tokio::select! {
            event = source.next_event() => {
                let event = event.expect("the source is alive");
                if matches!(&event, SwarmEvent::Disconnected { peer, .. } if *peer == target_peer) {
                    source_told.get_or_insert(stopped_at.elapsed());
                }
                log.push(format!("{:?} source: {event:?}", stopped_at.elapsed()));
            }
            event = target.next_event() => {
                let event = event.expect("the target is alive");
                match &event {
                    SwarmEvent::Disconnected { peer, .. } if *peer == source_peer => {
                        target_told.get_or_insert(stopped_at.elapsed());
                    }
                    SwarmEvent::RelayReservationChanged {
                        relay,
                        outcome: RelayReservationOutcome::Lost,
                        ..
                    } if *relay == relay_peer => {
                        reservation_lost.get_or_insert(stopped_at.elapsed());
                    }
                    _ => {}
                }
                log.push(format!("{:?} target: {event:?}", stopped_at.elapsed()));
            }
            () = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
    }
    let after = AfterTheStop {
        send,
        source_told,
        target_told,
        reservation_lost,
    };
    eprintln!("relay stopped (speaks ping: {speaks_ping}): {after:?}");
    for line in &log {
        eprintln!("  {line}");
    }
    source.shutdown().await.expect("shuts down");
    target.shutdown().await.expect("shuts down");
    after
}

/// The keepalive's worst case for a silent path: a ping every 15 s,
/// 20 s to answer, the first failure forgiven, then the ten-second
/// upgrade timeout of the next stream (`relay_keepalive.rs`, "How long a
/// dead path takes to close") -- plus room for a loaded host.
const KEEPALIVE_CLOSES_WITHIN: Duration = Duration::from_secs(90);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_relay_that_stops_under_a_circuit_bounds_the_send_and_is_found_by_the_reserving_end() {
    let after = stop_the_relay_under_a_circuit(true, Duration::from_secs(120)).await;

    // THE SEND THROUGH THE STOPPED RELAY is answered, and by the direct
    // protocol's own timeout: no exchange outlives it.
    let (answer, took) = after.send;
    assert_eq!(answer, Err(TransportError::PeerUnreachable), "{after:?}");
    assert!(
        took + Duration::from_secs(1) >= DIRECT_TIMEOUT
            && took <= DIRECT_TIMEOUT + Duration::from_secs(5),
        "the send is ended by DIRECT_TIMEOUT ({DIRECT_TIMEOUT:?}): {after:?}"
    );
    // THE RESERVING END pings its relay, so it finds the relay gone:
    // the reservation is lost and the circuit's peer disconnected.
    let lost = after.reservation_lost.expect("the reservation is lost");
    let told = after
        .target_told
        .expect("the target is told the source is gone");
    assert!(
        lost <= KEEPALIVE_CLOSES_WITHIN && told <= KEEPALIVE_CLOSES_WITHIN,
        "within the keepalive's bound: {after:?}"
    );
    // LIMIT, measured: the DIALLING end holds no reservation on the
    // relay, so the keepalive does not ping it there, and nothing tells
    // it the circuit is dead -- the peer stays `Connected` and each send
    // costs a DIRECT_TIMEOUT. `tests/SECURITY-MATRIX.md` S21-07 carries
    // it; when this fails, the limit was closed and the row says so.
    assert_eq!(
        after.source_told, None,
        "the dialling end is not told within two minutes (S21-07's limit): {after:?}"
    );
}
