// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The relay-control keepalive (`transport/libp2p/CONNECTIVITY.md` §14
//! item 5) over real sockets, with [`RelayKeepalive`] in bare Swarms at
//! a fast interval -- the runtime's is the crate's default, fifteen
//! seconds with twenty to answer, and a missed ping would take a minute
//! to show at it.
//!
//! Proved here:
//! - no ping crosses a connection whose peer holds no reservation of
//!   the client's -- the control;
//! - switched on, the client pings and the relay ANSWERS, the relay
//!   never pinging back -- and, the other way round, a relay pings the
//!   peer holding its reservation, which answers without pinging;
//! - a path that stops carrying anything -- a proxy between the two
//!   that goes silent with both sockets open, the shape of a NAT
//!   rebinding or a carrier drop, which raises no event at either end
//!   -- has its connection CLOSED by the client, counted as missed;
//! - a relay control peer that refuses ping is a missed liveness: its
//!   connection is closed and the refusal reported once by name
//!   (`transport/libp2p/CONNECTIVITY.md` section 14, A 2026-10-25;
//!   before it, such a relay was kept, and could never be found dead).
//! - over a real circuit (A 2026-10-25), BOTH ends of a relayed
//!   connection ping it with no switch on, the dialling end and the
//!   listening one, whose connection is told only by its local address;
//!   a switch off for the peer leaves it pinging; and a relayed
//!   data-plane peer that refuses ping is counted and KEPT, the close
//!   being the control peers' rule.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::HashSet;
use std::time::Duration;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::StreamExt as _;
use interweave_transport_libp2p::relay_keepalive::{
    KeepaliveCounters, KeepaliveEvent, RelayKeepalive,
};
use libp2p::multiaddr::Protocol;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent, dummy};
use libp2p::{Multiaddr, PeerId, Swarm, identity, noise, ping, relay, tcp, yamux};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};

const FAST: Duration = Duration::from_millis(200);

fn swarm<B: libp2p::swarm::NetworkBehaviour>(behaviour: B) -> Swarm<B> {
    swarm_idle(behaviour, Duration::from_secs(600))
}

fn swarm_idle<B: libp2p::swarm::NetworkBehaviour>(behaviour: B, idle: Duration) -> Swarm<B> {
    libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("tcp")
        .with_behaviour(|_| behaviour)
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(idle))
        .build()
}

fn keepalive() -> RelayKeepalive {
    RelayKeepalive::with_config(ping::Config::new().with_interval(FAST).with_timeout(FAST))
}

async fn listening<B: libp2p::swarm::NetworkBehaviour>(s: &mut Swarm<B>) -> Multiaddr {
    s.listen_on("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .expect("listens");
    loop {
        if let SwarmEvent::NewListenAddr { address, .. } = s.select_next_some().await {
            return address;
        }
    }
}

/// A TCP proxy in front of `target` that forwards until `silent` is
/// set and then forwards nothing, holding both sockets open: no FIN, no
/// RST, nothing either end can see but silence.
async fn proxy(target: std::net::SocketAddr, silent: Arc<AtomicBool>) -> Multiaddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
    let port = listener.local_addr().expect("bound").port();
    tokio::spawn(async move {
        let (inbound, _) = listener.accept().await.expect("the client arrives");
        let outbound = TcpStream::connect(target).await.expect("the relay answers");
        let (in_r, in_w) = inbound.into_split();
        let (out_r, out_w) = outbound.into_split();
        let pipe = |silent: Arc<AtomicBool>| {
            move |mut r: tokio::net::tcp::OwnedReadHalf, mut w: tokio::net::tcp::OwnedWriteHalf| async move {
                let mut buf = [0u8; 4096];
                while let Ok(n) = r.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    if silent.load(Ordering::Acquire) {
                        // Swallowed, and the halves held: the path is
                        // dead and neither end is told.
                        std::future::pending::<()>().await;
                    }
                    if w.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            }
        };
        tokio::spawn(pipe(silent.clone())(in_r, out_w));
        pipe(silent)(out_r, in_w).await;
    });
    format!("/ip4/127.0.0.1/tcp/{port}").parse().expect("valid")
}

/// What the client saw of the relay's connection.
#[derive(Debug, Default)]
struct Seen {
    established: usize,
    closed: usize,
    /// Refusals the keepalive reported, by peer.
    refused: Vec<PeerId>,
}

/// Drive the client, and the relay when `relay` is given, for `window`
/// or until `done` holds of what the client saw.
async fn drive<R: libp2p::swarm::NetworkBehaviour>(
    client: &mut Swarm<RelayKeepalive>,
    mut relay: Option<&mut Swarm<R>>,
    seen: &mut Seen,
    window: Duration,
    done: impl Fn(&Seen, KeepaliveCounters) -> bool,
) {
    let deadline = tokio::time::Instant::now() + window;
    while !done(seen, client.behaviour().counters()) {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return;
        }
        let relay_next = async {
            match relay.as_mut() {
                Some(r) => {
                    let _ = r.select_next_some().await;
                }
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            event = client.select_next_some() => match event {
                SwarmEvent::ConnectionEstablished { .. } => seen.established += 1,
                SwarmEvent::ConnectionClosed { .. } => seen.closed += 1,
                SwarmEvent::Behaviour(KeepaliveEvent::Refused { peer }) => seen.refused.push(peer),
                _ => {}
            },
            () = relay_next => {}
            // Re-read the counters often: a ping raises no Swarm event.
            () = tokio::time::sleep(remaining.min(Duration::from_millis(50))) => {}
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_control_connection_is_pinged_and_closed_when_the_relay_stops_answering() {
    let mut relay = swarm(keepalive());
    let mut client = swarm(keepalive());
    let relay_peer: PeerId = *relay.local_peer_id();
    let addr = listening(&mut relay).await;
    let Some(libp2p::multiaddr::Protocol::Tcp(port)) = addr.iter().nth(1) else {
        panic!("a tcp listener: {addr}");
    };
    let silent = Arc::new(AtomicBool::new(false));
    let through = proxy(([127, 0, 0, 1], port).into(), silent.clone()).await;
    client.dial(through).expect("dials");
    let mut seen = Seen::default();
    drive(
        &mut client,
        Some(&mut relay),
        &mut seen,
        Duration::from_secs(10),
        |s, _| s.established == 1,
    )
    .await;
    assert_eq!(seen.established, 1, "connected");

    // THE CONTROL: no reservation, no ping -- ten intervals of nothing.
    drive(
        &mut client,
        Some(&mut relay),
        &mut seen,
        FAST * 10,
        |_, _| false,
    )
    .await;
    assert_eq!(client.behaviour().counters(), KeepaliveCounters::default());
    assert_eq!(relay.behaviour().counters().echoed, 0);

    // SWITCHED ON: the client pings, the relay answers and never pings.
    client
        .behaviour_mut()
        .set_relays(HashSet::from([relay_peer]));
    drive(
        &mut client,
        Some(&mut relay),
        &mut seen,
        Duration::from_secs(10),
        |_, c| c.answered >= 3,
    )
    .await;
    let answered = client.behaviour().counters().answered;
    assert!(answered >= 3, "{:?}", client.behaviour().counters());
    let relay_counts = relay.behaviour().counters();
    assert!(relay_counts.echoed >= 3, "{relay_counts:?}");
    assert_eq!(relay_counts.answered, 0, "the relay never pings");

    // THE PATH GOES SILENT: nothing crosses it, nothing closes it. The
    // client closes the connection -- once the crate reports its SECOND
    // failure (the first is forgiven), and that second one is the
    // stream it opens anew failing to negotiate, which takes the
    // Swarm's ten-second upgrade timeout whatever the ping interval.
    silent.store(true, Ordering::Release);
    drive(
        &mut client,
        Some(&mut relay),
        &mut seen,
        Duration::from_secs(30),
        |s, _| s.closed == 1,
    )
    .await;
    assert_eq!(
        seen.closed,
        1,
        "closed for the missed ping: {seen:?} {:?}",
        client.behaviour().counters()
    );
    assert_eq!(client.behaviour().counters().missed, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_relay_pings_the_peer_holding_its_reservation_which_only_answers() {
    let mut relay = swarm(keepalive());
    let mut holder = swarm(keepalive());
    let holder_peer: PeerId = *holder.local_peer_id();
    let addr = listening(&mut relay).await;
    holder.dial(addr).expect("dials");
    let mut seen = Seen::default();
    drive(
        &mut relay,
        Some(&mut holder),
        &mut seen,
        Duration::from_secs(10),
        |s, _| s.established == 1,
    )
    .await;
    relay
        .behaviour_mut()
        .set_reserved(HashSet::from([holder_peer]));
    drive(
        &mut relay,
        Some(&mut holder),
        &mut seen,
        Duration::from_secs(10),
        |_, c| c.answered >= 3,
    )
    .await;
    assert!(
        relay.behaviour().counters().answered >= 3,
        "{:?}",
        relay.behaviour().counters()
    );
    let holder_counts = holder.behaviour().counters();
    assert!(holder_counts.echoed >= 3, "{holder_counts:?}");
    assert_eq!(holder_counts.answered, 0, "the holder never pings");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn answering_pings_does_not_keep_an_idle_connection_open() {
    // Liveness, not retention (#129 review F5): an end that only echoes,
    // with a one-second idle timeout and nothing else on the connection,
    // still closes it while the other end pings on.
    let mut echoing = swarm_idle(keepalive(), Duration::from_secs(1));
    let mut pinging = swarm(keepalive());
    let echoing_peer: PeerId = *echoing.local_peer_id();
    let addr = listening(&mut echoing).await;
    pinging.dial(addr).expect("dials");
    pinging
        .behaviour_mut()
        .set_relays(HashSet::from([echoing_peer]));
    let mut seen = Seen::default();
    drive(
        &mut pinging,
        Some(&mut echoing),
        &mut seen,
        Duration::from_secs(10),
        |s, _| s.closed == 1,
    )
    .await;
    let counts = pinging.behaviour().counters();
    assert!(
        counts.answered >= 1,
        "it was pinged and answered: {counts:?}"
    );
    assert_eq!(
        seen.closed, 1,
        "and closed for idleness all the same: {seen:?}"
    );
    assert_eq!(counts.missed, 0, "not for a missed ping: {counts:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_control_peer_that_refuses_ping_is_closed_and_reported() {
    let mut relay = swarm(dummy::Behaviour);
    let mut client = swarm(keepalive());
    let relay_peer: PeerId = *relay.local_peer_id();
    let addr = listening(&mut relay).await;
    client.dial(addr).expect("dials");
    let mut seen = Seen::default();
    drive(
        &mut client,
        Some(&mut relay),
        &mut seen,
        Duration::from_secs(10),
        |s, _| s.established == 1,
    )
    .await;
    // THE CONTROL: not a control peer yet, so nothing pings it and its
    // refusal is never asked -- ten intervals, nothing closed.
    drive(
        &mut client,
        Some(&mut relay),
        &mut seen,
        FAST * 10,
        |_, _| false,
    )
    .await;
    assert_eq!(seen.closed, 0, "kept while it is no control peer: {seen:?}");
    assert_eq!(client.behaviour().counters().unsupported, 0);

    // SWITCHED ON: the first ping is refused, and that is a missed
    // liveness -- closed, and said once by name.
    client
        .behaviour_mut()
        .set_relays(HashSet::from([relay_peer]));
    drive(
        &mut client,
        Some(&mut relay),
        &mut seen,
        Duration::from_secs(10),
        |s, _| s.closed == 1 && !s.refused.is_empty(),
    )
    .await;
    let counts = client.behaviour().counters();
    assert_eq!(counts.unsupported, 1, "{counts:?}");
    assert_eq!(
        seen.closed, 1,
        "closed for the refusal: {seen:?} {counts:?}"
    );
    assert_eq!(
        seen.refused,
        vec![relay_peer],
        "reported once, by peer: {seen:?}"
    );
}

/// A relay client with the keepalive: one end of a circuit.
#[derive(NetworkBehaviour)]
struct CircuitEnd {
    relay: relay::client::Behaviour,
    keepalive: RelayKeepalive,
}

/// A relay client that speaks no ping: a far end that refuses it.
#[derive(NetworkBehaviour)]
struct MuteEnd {
    relay: relay::client::Behaviour,
}

fn relay_client_swarm<B: NetworkBehaviour>(
    make: impl FnOnce(relay::client::Behaviour) -> B,
) -> Swarm<B> {
    libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
        .expect("tcp")
        .with_relay_client(noise::Config::new, yamux::Config::default)
        .expect("the relay transport")
        .with_behaviour(|_, client| make(client))
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
        .build()
}

/// The crate's own relay server on loopback, told its listen address as
/// its external one, driven on a task of its own; its circuit address
/// for a destination is `<returned>/p2p-circuit/p2p/<destination>`.
async fn bare_relay() -> Multiaddr {
    let keys = identity::Keypair::generate_ed25519();
    let peer = keys.public().to_peer_id();
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keys)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
        .expect("tcp")
        .with_behaviour(|_| relay::Behaviour::new(peer, relay::Config::default()))
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
        .build();
    let addr = listening(&mut swarm).await;
    swarm.add_external_address(addr.clone());
    tokio::spawn(async move {
        loop {
            let _ = swarm.select_next_some().await;
        }
    });
    addr.with(Protocol::P2p(peer))
}

/// Reserve `dest` on the relay at `relay`, then dial it from `source`
/// through the circuit, driving both until the connection stands.
async fn circuit<S: NetworkBehaviour>(
    relay: &Multiaddr,
    dest: &mut Swarm<CircuitEnd>,
    source: &mut Swarm<S>,
) {
    dest.listen_on(relay.clone().with(Protocol::P2pCircuit))
        .expect("listens through the relay");
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let SwarmEvent::Behaviour(CircuitEndEvent::Relay(
                relay::client::Event::ReservationReqAccepted { .. },
            )) = dest.select_next_some().await
            {
                return;
            }
        }
    })
    .await
    .expect("the reservation is granted");
    let dest_peer = *dest.local_peer_id();
    source
        .dial(
            relay
                .clone()
                .with(Protocol::P2pCircuit)
                .with(Protocol::P2p(dest_peer)),
        )
        .expect("dials the circuit");
    tokio::time::timeout(Duration::from_secs(10), async {
        while !(dest.is_connected(source.local_peer_id()) && source.is_connected(&dest_peer)) {
            tokio::select! {
                _ = dest.select_next_some() => {}
                _ = source.select_next_some() => {}
            }
        }
    })
    .await
    .expect("the circuit connects");
}

/// Drive both ends for `window`.
async fn drive_both<A: NetworkBehaviour, B: NetworkBehaviour>(
    a: &mut Swarm<A>,
    b: &mut Swarm<B>,
    window: Duration,
) {
    let deadline = tokio::time::sleep(window);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            _ = a.select_next_some() => {}
            _ = b.select_next_some() => {}
            () = &mut deadline => return,
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn both_ends_of_a_circuit_ping_it_and_a_switch_off_leaves_it_pinging() {
    let relay = bare_relay().await;
    let mut dest = relay_client_swarm(|relay| CircuitEnd {
        relay,
        keepalive: keepalive(),
    });
    let mut source = relay_client_swarm(|relay| CircuitEnd {
        relay,
        keepalive: keepalive(),
    });
    circuit(&relay, &mut dest, &mut source).await;
    // Neither end switched anything on, and the destination's connection
    // to the relay -- a direct one to no control peer -- pings nothing:
    // what each end counts is the circuit's.
    drive_both(&mut dest, &mut source, FAST * 10).await;
    let (d, s) = (
        dest.behaviour().keepalive.counters(),
        source.behaviour().keepalive.counters(),
    );
    assert!(d.answered > 0, "the listening end pings the circuit: {d:?}");
    assert!(s.answered > 0, "the dialling end pings the circuit: {s:?}");
    // A switch on and off for the far peer leaves its circuit pinging.
    let source_peer = *source.local_peer_id();
    dest.behaviour_mut()
        .keepalive
        .set_relays(HashSet::from([source_peer]));
    drive_both(&mut dest, &mut source, FAST * 2).await;
    dest.behaviour_mut().keepalive.set_relays(HashSet::new());
    drive_both(&mut dest, &mut source, FAST * 2).await;
    let before = dest.behaviour().keepalive.counters().answered;
    drive_both(&mut dest, &mut source, FAST * 10).await;
    let after = dest.behaviour().keepalive.counters();
    assert!(
        after.answered > before,
        "still pinging after the switch off: {before} then {after:?}"
    );
    assert!(dest.is_connected(&source_peer), "and kept");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_relayed_data_plane_peer_that_refuses_ping_is_counted_and_kept() {
    let relay = bare_relay().await;
    let mut dest = relay_client_swarm(|relay| CircuitEnd {
        relay,
        keepalive: keepalive(),
    });
    let mut source = relay_client_swarm(|relay| MuteEnd { relay });
    circuit(&relay, &mut dest, &mut source).await;
    let source_peer = *source.local_peer_id();
    drive_both(&mut dest, &mut source, FAST * 10).await;
    let counts = dest.behaviour().keepalive.counters();
    // The listening end pinged, which is what found the refusal...
    assert_eq!(counts.unsupported, 1, "{counts:?}");
    // ...and the refusal is no missed liveness for a data-plane peer.
    assert!(
        dest.is_connected(&source_peer),
        "kept: the close is the control peers' rule: {counts:?}"
    );
    assert_eq!(counts.missed, 0, "{counts:?}");
}
