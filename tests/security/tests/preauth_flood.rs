// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Pre-Noise handshake floods against the SHIPPED pre-auth limits.
//!
//! Stage 5 proved each mechanism with a narrowed limit (a ceiling of
//! one, two starts a window), which makes the mechanism visible. Here
//! the limits are `SubstrateConfig::default()`'s, the numbers a user
//! runs: 64 handshakes in flight, 8 per source, a 10 s handshake
//! timeout, and 600 starts a minute across all sources.
//!
//! The attacker is raw TCP that completes the TCP handshake and then
//! says nothing. That is the cheapest flood there is, and a `SwarmRuntime`
//! cannot play it because it would complete the Noise handshake. Each
//! flooding socket is bound to its own loopback address (127.0.0.0/8
//! all reaches `lo` on Linux), and IPv4 buckets by address, so one host
//! plays many sources over real sockets.
//!
//! What a socket's fate looks like from outside: a refused start is
//! closed by the listener at once, and a held one stays open and silent
//! until the handshake timeout. A read that ends (EOF or reset) within
//! the observation window is a refusal. A read still pending is a held
//! slot.

#![allow(clippy::expect_used, clippy::panic)]

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_libp2p::{SubstrateConfig, SwarmEvent, SwarmRuntime};
use interweave_transport_runtime::TrustSources;
use interweave_transport_runtime::preauth::{
    DEFAULT_HANDSHAKE_TIMEOUT_MS, DEFAULT_MAX_GLOBAL_ATTEMPTS_PER_WINDOW,
    DEFAULT_MAX_PENDING_PER_SOURCE, DEFAULT_MAX_PENDING_TOTAL,
};
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};
use libp2p::Multiaddr;
use libp2p::multiaddr::Protocol;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpSocket, TcpStream};
use tokio::task::JoinSet;

/// How long a socket is watched before it counts as held. A refusal
/// arrives as soon as the listener processes the connection. This is
/// several times that, and far below the 10 s timeout that would also
/// end a held socket.
const OBSERVE: Duration = Duration::from_millis(2_000);

fn who() -> (ProfileIdentity, TransportIdentity) {
    let id = ProfileIdentity::generate();
    let peer = id.transport_identity().expect("peer id");
    (id, peer)
}

fn trusting(peers: &[&TransportIdentity]) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new(peers.iter().map(|p| (*p).clone())).expect("a handful"),
        InfrastructureSet::default(),
    )
}

/// A listener on the shipped limits that trusts `peer`, and where it
/// is bound.
async fn listener_trusting(peer: &TransportIdentity) -> (SwarmRuntime, Multiaddr, SocketAddr) {
    let listener = SwarmRuntime::start(
        &ProfileIdentity::generate(),
        SubstrateConfig::default(),
        trusting(&[peer]),
    )
    .expect("starts");
    let bound = listener
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("binds");
    let socket = socket_addr_of(&bound);
    (listener, bound, socket)
}

fn socket_addr_of(address: &Multiaddr) -> SocketAddr {
    let mut ip = None;
    let mut port = None;
    for component in address {
        match component {
            Protocol::Ip4(v4) => ip = Some(std::net::IpAddr::V4(v4)),
            Protocol::Tcp(p) => port = Some(p),
            _ => {}
        }
    }
    SocketAddr::new(
        ip.expect("an ip4 component"),
        port.expect("a tcp component"),
    )
}

/// The `n`th flooding source, 127.1.0.1 onwards. 127.0.0.1 stays free
/// for the trusted peer, whose dial the kernel sources from there.
fn source(n: u16) -> Ipv4Addr {
    let [hi, lo] = n.to_be_bytes();
    Ipv4Addr::new(127, 1 + hi, lo, 1)
}

/// One silent TCP connection from `from` to `to`, which never speaks.
async fn silent(from: Ipv4Addr, to: SocketAddr) -> TcpStream {
    let socket = TcpSocket::new_v4().expect("a socket");
    socket
        .bind(SocketAddr::new(from.into(), 0))
        .expect("every 127/8 address is local on Linux");
    socket.connect(to).await.expect("the kernel completes TCP")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fate {
    /// Still open and silent after `OBSERVE`: the listener holds a slot.
    Held,
    /// Closed by the listener within `OBSERVE`: the start was refused.
    Refused,
}

/// Watch every socket at once for `OBSERVE`. Returns each socket's fate
/// in the order given, and the sockets that are still held, so that
/// keeping them open keeps their slots taken.
async fn fates(streams: Vec<TcpStream>) -> (Vec<Fate>, Vec<TcpStream>) {
    let mut watch = JoinSet::new();
    for (index, mut stream) in streams.into_iter().enumerate() {
        watch.spawn(async move {
            let mut byte = [0_u8; 1];
            match tokio::time::timeout(OBSERVE, stream.read(&mut byte)).await {
                Err(_) => (index, Fate::Held, Some(stream)),
                // The listener spoke rather than hanging up, so it is
                // still holding the connection.
                Ok(Ok(n)) if n > 0 => (index, Fate::Held, Some(stream)),
                Ok(_) => (index, Fate::Refused, None),
            }
        });
    }
    let mut out = Vec::new();
    while let Some(joined) = watch.join_next().await {
        out.push(joined.expect("a watcher does not panic"));
    }
    out.sort_by_key(|(index, _, _)| *index);
    let fates = out.iter().map(|(_, fate, _)| *fate).collect();
    let held = out
        .into_iter()
        .filter_map(|(_, _, stream)| stream)
        .collect();
    (fates, held)
}

/// Whether a trusted peer gets a connection to the listener within
/// `within`.
async fn trusted_connects(
    id: &ProfileIdentity,
    listener_peer: &TransportIdentity,
    bound: &Multiaddr,
    within: Duration,
) -> bool {
    let mut peer = SwarmRuntime::start(id, SubstrateConfig::default(), trusting(&[listener_peer]))
        .expect("starts");
    peer.dial(listener_peer.clone(), bound.clone())
        .await
        .expect("the command reaches the task")
        .expect("the dialer's own gate has no objection");
    let deadline = tokio::time::Instant::now() + within;
    let connected = loop {
        match tokio::time::timeout_at(deadline, peer.next_event()).await {
            Ok(Some(SwarmEvent::Connected { .. })) => break true,
            Ok(Some(SwarmEvent::DialFailed { .. })) | Err(_) => break false,
            Ok(Some(_)) => {}
            Ok(None) => panic!("the substrate stopped"),
        }
    };
    peer.shutdown().await.expect("shuts down");
    connected
}

#[tokio::test]
async fn one_source_flooding_at_the_shipped_limits_holds_its_share_and_shuts_out_nobody_else() {
    let (peer_id, peer) = who();
    let (listener, bound, socket) = listener_trusting(&peer).await;
    let listener_peer = listener.local_peer().clone();

    let attacker = source(0);
    let mut flood = Vec::new();
    for _ in 0..=DEFAULT_MAX_PENDING_PER_SOURCE {
        flood.push(silent(attacker, socket).await);
    }
    let (fate, held) = fates(flood).await;

    let mut expected = vec![Fate::Held; DEFAULT_MAX_PENDING_PER_SOURCE];
    expected.push(Fate::Refused);
    assert_eq!(
        fate, expected,
        "one source holds exactly its share of handshakes in flight, and its next is refused"
    );

    // The control that makes the refusal mean "per source": a trusted
    // peer from another address is admitted while the attacker's share
    // is still held.
    assert_eq!(held.len(), DEFAULT_MAX_PENDING_PER_SOURCE);
    assert!(
        trusted_connects(&peer_id, &listener_peer, &bound, Duration::from_secs(10)).await,
        "a flooding source must not shut out another source"
    );

    drop(held);
    listener.shutdown().await.expect("shuts down");
}

#[tokio::test]
async fn a_distributed_flood_fills_the_global_ceiling_and_the_timeout_gives_it_back() {
    let (peer_id, peer) = who();
    let (listener, bound, socket) = listener_trusting(&peer).await;
    let listener_peer = listener.local_peer().clone();

    // Every source stays inside its own share, so only the global
    // ceiling can refuse the last one.
    let sources = DEFAULT_MAX_PENDING_TOTAL / DEFAULT_MAX_PENDING_PER_SOURCE;
    let mut flood = Vec::new();
    for s in 0..sources {
        for _ in 0..DEFAULT_MAX_PENDING_PER_SOURCE {
            flood.push(silent(source(u16::try_from(s).expect("small")), socket).await);
        }
    }
    let fresh = source(u16::try_from(sources).expect("small"));
    flood.push(silent(fresh, socket).await);
    let (fate, held) = fates(flood).await;

    let mut expected = vec![Fate::Held; DEFAULT_MAX_PENDING_TOTAL];
    expected.push(Fate::Refused);
    assert_eq!(
        fate, expected,
        "the ceiling holds exactly {DEFAULT_MAX_PENDING_TOTAL} in flight across sources, and refuses a fresh source past it"
    );

    // The residual threat-model.md names: a distributed flood at the
    // ceiling refuses an honest peer too, until the timeout frees it.
    assert!(
        !trusted_connects(&peer_id, &listener_peer, &bound, Duration::from_secs(3)).await,
        "while the ceiling is full a trusted peer is refused as well"
    );

    // The timeout gives every slot back. The held sockets are closed BY
    // THE LISTENER, which is the reclaim and not merely a client giving
    // up: each one reads EOF.
    let timeout = Duration::from_millis(DEFAULT_HANDSHAKE_TIMEOUT_MS);
    let mut reclaimed = JoinSet::new();
    for mut stream in held {
        reclaimed.spawn(async move {
            let mut byte = [0_u8; 1];
            tokio::time::timeout(timeout * 2, stream.read(&mut byte)).await
        });
    }
    while let Some(read) = reclaimed.join_next().await {
        let read = read.expect("a watcher does not panic");
        assert!(
            matches!(read, Ok(Ok(0) | Err(_))),
            "a silent handshake is closed by the listener at the timeout, got {read:?}"
        );
    }
    assert!(
        trusted_connects(&peer_id, &listener_peer, &bound, Duration::from_secs(10)).await,
        "once the timeout reclaims the slots, a trusted peer is admitted"
    );

    listener.shutdown().await.expect("shuts down");
}

#[tokio::test]
async fn the_global_start_rate_refuses_a_fresh_source_once_the_window_is_spent() {
    // 600 starts a minute across all sources, at the shipped limits.
    // The starts are spread so that neither handshake ceiling refuses
    // any of them: batches of half the global ceiling, four sources of
    // eight, each batch confirmed held and then closed before the next.
    // Every source is used once, far below its own 30-a-minute rate.
    let (_, peer) = who();
    let (listener, _, socket) = listener_trusting(&peer).await;

    let budget = usize::try_from(DEFAULT_MAX_GLOBAL_ATTEMPTS_PER_WINDOW).expect("fits");
    let batch = DEFAULT_MAX_PENDING_TOTAL / 2;
    let mut started = 0_usize;
    let mut next_source = 0_u16;
    while started < budget {
        let this_batch = batch.min(budget - started);
        let mut flood = Vec::new();
        for i in 0..this_batch {
            if i % DEFAULT_MAX_PENDING_PER_SOURCE == 0 {
                next_source += 1;
            }
            flood.push(silent(source(next_source), socket).await);
        }
        let (fate, held) = fates(flood).await;
        assert!(
            fate.iter().all(|f| *f == Fate::Held),
            "every start inside the window is admitted (after {started}): {fate:?}"
        );
        started += this_batch;
        // Closing gives the slots back. The two-batch headroom under the
        // ceiling covers a release the listener has not processed yet.
        drop(held);
    }

    next_source += 1;
    let (fate, _held) = fates(vec![silent(source(next_source), socket).await]).await;
    assert_eq!(
        fate,
        vec![Fate::Refused],
        "start {} in the window is refused, from a source that never started before",
        budget + 1
    );

    listener.shutdown().await.expect("shuts down");
}
