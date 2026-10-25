// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An AutoNAT server that never answers a dial request, or answers it
//! with a frame that does not decode, in front of the vendored client
//! (plan section 21, S8; `tests/SECURITY-MATRIX.md` row S21-08).
//!
//! The runtime cannot be the client here: on loopback `AUTONAT.md` §6
//! refuses every candidate, so a substrate never sends a dial request at
//! all (`tests/connectivity/tests/autonat_client.rs` says the same). So,
//! as `autonat_outcome_wire.rs` does, a raw Swarm runs the vendored
//! client, fed a candidate directly, and what it produces is what the
//! adapter would be handed.
//!
//! What is proved, for each misbehaviour: the double is asked; the
//! client produces NO outcome for the candidate -- past its own ten
//! second bound on a dial request (`v2/client/handler/dial_request.rs`)
//! -- so nothing reaches `classify_outcome` and no failure vote is cast
//! for a server's silence or garbage; and the candidate is not used up:
//! once the double is gone, an honest server probes the same candidate
//! and its outcome is classified. That last step is the positive
//! control: the same client, the same candidate, and an outcome the
//! classifier counts.
//!
//! The candidate is handed back with `retest`, as the adapter does for
//! one silent past `SILENCE_MS`: a probe in flight on a connection that
//! closes is never reported by the crate and stays `Pending`, which its
//! sweep never takes again (measured here first, without the `retest`:
//! after the silent double, the honest server never probed it). `autonat_driver.rs`
//! `a_candidate_with_no_outcome_is_retested_after_the_silence_bound`
//! pins the adapter's half.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use futures::StreamExt as _;
use interweave_transport_libp2p::runtime::autonat_driver::classify_outcome;
use interweave_transport_runtime::reachability::ProbeOutcome;
use libp2p::autonat::v2::client::{Behaviour as Client, Config as ClientConfig};
use libp2p::autonat::v2::server::Behaviour as Server;
use libp2p::swarm::{FromSwarm, NetworkBehaviour, NewExternalAddrCandidate, SwarmEvent};
use libp2p::{Multiaddr, PeerId, StreamProtocol, Swarm, identify, identity};
use tokio::sync::mpsc;

const PATIENCE: Duration = Duration::from_secs(20);

/// The vendored client's bound on a dial request
/// (`v2/client/handler/dial_request.rs`), plus room to see it pass.
const PAST_THE_CLIENTS_BOUND: Duration = Duration::from_secs(14);

const DIAL_REQUEST: &str = "/libp2p/autonat/2/dial-request";

/// A length prefix of one byte, then three bytes that are no AutoNAT v2
/// `Message`: field 31 with a reserved wire type.
const UNDECODABLE: &[u8] = &[0x03, 0xff, 0xff, 0xff];

fn identify(k: &identity::Keypair) -> identify::Behaviour {
    identify::Behaviour::new(identify::Config::new(
        "/interweave-autonat-misbehaving-test/1".to_owned(),
        k.public(),
    ))
}

#[derive(NetworkBehaviour)]
struct RawClient {
    identify: identify::Behaviour,
    client: Client,
}

#[derive(NetworkBehaviour)]
struct RawServer {
    identify: identify::Behaviour,
    server: Server,
}

#[derive(NetworkBehaviour)]
struct Double {
    identify: identify::Behaviour,
    request: acceptor::Behaviour,
}

fn swarm<B: NetworkBehaviour>(build: impl FnOnce(&identity::Keypair) -> B) -> Swarm<B> {
    libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("the same transport stack the substrate uses")
        .with_behaviour(|k| build(k))
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
        .build()
}

async fn bound<B: NetworkBehaviour>(swarm: &mut Swarm<B>) -> Multiaddr {
    swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().expect("a listen address"))
        .expect("listens");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        match tokio::time::timeout_at(deadline, swarm.select_next_some()).await {
            Ok(SwarmEvent::NewListenAddr { address, .. }) => return address,
            Ok(_) => {}
            Err(elapsed) => panic!("no listen address within {PATIENCE:?} ({elapsed})"),
        }
    }
}

/// What the double does with a dial request stream.
#[derive(Clone, Copy, Debug)]
enum Misbehaviour {
    Silent,
    Answer(&'static [u8]),
}

/// The double, driven by a task of its own; each stream it accepted is
/// reported on the channel and then held, so a silent one is never
/// closed by being dropped.
async fn double(misbehaviour: Misbehaviour) -> (PeerId, Multiaddr, mpsc::UnboundedReceiver<()>) {
    let mut swarm = swarm(|k| Double {
        identify: identify(k),
        request: acceptor::Behaviour::new(StreamProtocol::new(DIAL_REQUEST)),
    });
    let addr = bound(&mut swarm).await;
    let peer = *swarm.local_peer_id();
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut held = Vec::new();
        loop {
            if let SwarmEvent::Behaviour(DoubleEvent::Request(mut stream)) =
                swarm.select_next_some().await
            {
                let _ = tx.send(());
                if let Misbehaviour::Answer(bytes) = misbehaviour {
                    use futures::AsyncWriteExt as _;
                    stream.write_all(bytes).await.expect("the double writes");
                    stream.flush().await.expect("the double flushes");
                }
                held.push(stream);
            }
        }
    });
    (peer, addr, rx)
}

/// The vendored server, honest, driven by a task of its own.
async fn honest() -> (PeerId, Multiaddr) {
    let mut swarm = swarm(|k| RawServer {
        identify: identify(k),
        server: Server::default(),
    });
    let addr = bound(&mut swarm).await;
    let peer = *swarm.local_peer_id();
    tokio::spawn(async move {
        loop {
            swarm.select_next_some().await;
        }
    });
    (peer, addr)
}

/// A raw client that sweeps every 200 ms.
fn client() -> Swarm<RawClient> {
    swarm(|k| RawClient {
        identify: identify(k),
        client: Client::new(
            rand::make_rng::<rand::rngs::StdRng>(),
            ClientConfig::default().with_probe_interval(Duration::from_millis(200)),
        ),
    })
}

async fn connect(client: &mut Swarm<RawClient>, peer: PeerId, at: &Multiaddr) {
    client
        .dial(
            libp2p::swarm::dial_opts::DialOpts::peer_id(peer)
                .addresses(vec![at.clone()])
                .allocate_new_port()
                .build(),
        )
        .expect("dials");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        match tokio::time::timeout_at(deadline, client.select_next_some()).await {
            Ok(SwarmEvent::ConnectionEstablished { peer_id, .. }) if peer_id == peer => return,
            Ok(SwarmEvent::OutgoingConnectionError { error, .. }) => {
                panic!("the dial failed: {error}")
            }
            Ok(_) => {}
            Err(elapsed) => panic!("not connected within {PATIENCE:?} ({elapsed})"),
        }
    }
}

/// Drive `client` for `window` and return every probe outcome it
/// produced, by tested address.
async fn outcomes_for(
    client: &mut Swarm<RawClient>,
    window: Duration,
) -> Vec<(Multiaddr, Option<ProbeOutcome>)> {
    let mut out = Vec::new();
    let deadline = tokio::time::Instant::now() + window;
    while let Ok(event) = tokio::time::timeout_at(deadline, client.select_next_some()).await {
        if let SwarmEvent::Behaviour(RawClientEvent::Client(e)) = event {
            out.push((e.tested_addr.clone(), classify_outcome(&e.result)));
        }
    }
    out
}

async fn casts_no_vote_and_leaves_the_candidate_to_an_honest_server(misbehaviour: Misbehaviour) {
    let (double_peer, double_addr, mut asked) = double(misbehaviour).await;
    let mut client = client();
    connect(&mut client, double_peer, &double_addr).await;
    // A candidate nothing listens on: an honest server's dial-back to it
    // fails, which the classifier counts as `Unreachable`.
    let dead: Multiaddr = "/ip4/127.0.0.1/tcp/1".parse().expect("a literal");
    client
        .behaviour_mut()
        .client
        .on_swarm_event(FromSwarm::NewExternalAddrCandidate(
            NewExternalAddrCandidate { addr: &dead },
        ));

    // THE DOUBLE: asked, and no outcome at all -- for the candidate or
    // any other -- past the client's own bound on the request.
    let seen = outcomes_for(&mut client, PAST_THE_CLIENTS_BOUND).await;
    assert!(
        asked.try_recv().is_ok(),
        "{misbehaviour:?}: the double was asked, so the silence below is its answer's"
    );
    assert!(
        seen.is_empty(),
        "{misbehaviour:?}: a server's silence or garbage is no outcome, so no vote: {seen:?}"
    );

    // THE POSITIVE: the double gone, an honest server probes the same
    // candidate, and the classifier counts its answer.
    let _ = client.disconnect_peer_id(double_peer);
    let (honest_peer, honest_addr) = honest().await;
    connect(&mut client, honest_peer, &honest_addr).await;
    // The adapter's silence bound, applied at once.
    let _ = client.behaviour_mut().client.retest(&dead);
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let mut seen = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(
            !remaining.is_zero(),
            "{misbehaviour:?}: the honest server never probed the candidate: {seen:?}"
        );
        seen.extend(outcomes_for(&mut client, remaining.min(Duration::from_secs(1))).await);
        if let Some((_, outcome)) = seen.iter().find(|(a, _)| *a == dead) {
            assert_eq!(
                *outcome,
                Some(ProbeOutcome::Unreachable),
                "{misbehaviour:?}: the honest server's answer for the candidate is a vote"
            );
            return;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_server_that_never_answers_a_dial_request_casts_no_vote() {
    casts_no_vote_and_leaves_the_candidate_to_an_honest_server(Misbehaviour::Silent).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_server_that_answers_a_dial_request_with_an_undecodable_frame_casts_no_vote() {
    casts_no_vote_and_leaves_the_candidate_to_an_honest_server(Misbehaviour::Answer(UNDECODABLE))
        .await;
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
