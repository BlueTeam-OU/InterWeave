// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Two runtimes that dial each other at once hold the SAME connection
//! of the two (`transport/libp2p/CONNECTIVITY.md` §14, A 2026-11-13:
//! the one the lower peer id dialled), over two real Swarms and real
//! sockets -- not each end's table fed to `dialing::held_connections`,
//! which is the unit test's.
//!
//! WHY THE CHOICE IS READ FROM THE WIRE. Between two runtimes the pinned
//! gossipsub parks a stream on each end's first connection, so both
//! crossing connections outlive the idle timeout whatever the hold
//! chose, and an idle close cannot tell the choices apart (measured on
//! this test's first run: two connections at each end, both open past
//! 28 s against a 1 s idle timeout). What does tell them apart is the
//! ping: an end pings the connection it holds and no other
//! (`relay_keepalive`'s module note), and nothing else here sends on an
//! idle connection -- no topic is joined, no Kademlia runs, Identify
//! pushes only on a change and TCP keepalive is off. So once the first
//! ping interval has passed, the connection both ends hold has carried
//! bytes and the other has carried none; had each end held its own dial
//! (the rule's crossing-dials failure), both would have.
//!
//! The kernel's own byte counters are read (`ss -tni`, Linux), each
//! connection named by its 4-tuple: the lower peer's dial is the one
//! whose far port is the higher peer's listen port. Every direct dial
//! leaves from a fresh port (§12), so the two dials are two connections
//! and not one simultaneous open.
//!
//! That BOTH ends hold it, and not one end alone, is read from the data
//! segments each end of the held dial sends in the window: a measured
//! count, four per end with two pingers and three with one (the
//! assertion's note).
//!
//! NOT PROVED HERE: that each end holds exactly one connection per
//! peer -- the unit test
//! `exactly_one_direct_data_plane_connection_per_peer_is_held`. Two dials of ONE end established within one
//! handshake of each other, which §14 names as the rule's limit.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::net::Ipv4Addr;
use std::time::Duration;

use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_libp2p::{SubstrateConfig, SwarmRuntime};
use interweave_transport_runtime::TrustSources;
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};
use libp2p::Multiaddr;
use libp2p::multiaddr::Protocol;

const PATIENCE: Duration = Duration::from_secs(20);

/// Past the crate's 15 s ping interval (`libp2p-ping` 0.48.0's default,
/// which the keepalive keeps), so every held connection has been pinged
/// after the first reading at least once more.
const PING_WINDOW: Duration = Duration::from_secs(18);

/// Data segments each end of the held dial sends in the window when
/// both ends ping it; one pinger measured three (the assertion's note).
const BOTH_PINGING: u64 = 4;

fn trusting(peer: &TransportIdentity) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new([peer.clone()]).expect("one peer"),
        InfrastructureSet::default(),
    )
}

fn tcp_port(address: &Multiaddr) -> u16 {
    address
        .iter()
        .find_map(|p| match p {
            Protocol::Tcp(port) => Some(port),
            _ => None,
        })
        .expect("a tcp port")
}

/// One established TCP socket on this host, as `ss -tni` reports it.
#[derive(Debug)]
struct Socket {
    local: u16,
    remote: u16,
    received: u64,
    /// Segments this end sent that carried data.
    data_segments_out: u64,
}

/// Every established socket on `ip` with either end on one of `ports`.
fn sockets(ip: Ipv4Addr, ports: [u16; 2]) -> Vec<Socket> {
    let out = std::process::Command::new("ss")
        .args(["-tniH", "state", "established"])
        .output()
        .expect("ss runs: this test reads the kernel's byte counters through it");
    assert!(out.status.success(), "ss: {out:?}");
    let text = String::from_utf8(out.stdout).expect("utf-8");
    let port_of = |field: &str| -> Option<u16> {
        let (host, port) = field.rsplit_once(':')?;
        (host == ip.to_string()).then(|| port.parse().ok())?
    };
    // `ss -i` prints each socket on two lines: the addresses, then the
    // counters.
    let lines: Vec<&str> = text.lines().collect();
    lines
        .chunks(2)
        .filter_map(|pair| {
            let fields: Vec<&str> = pair.first()?.split_whitespace().collect();
            let local = port_of(fields.get(2)?)?;
            let remote = port_of(fields.get(3)?)?;
            if !ports.contains(&local) && !ports.contains(&remote) {
                return None;
            }
            // A counter still at zero is not printed.
            let counter = |name: &str| -> u64 {
                pair.get(1)
                    .and_then(|l| l.split_whitespace().find_map(|f| f.strip_prefix(name)))
                    .map_or(0, |n| n.parse().expect("a count"))
            };
            Some(Socket {
                local,
                remote,
                received: counter("bytes_received:"),
                data_segments_out: counter("data_segs_out:"),
            })
        })
        .collect()
}

/// The two connections' traffic so far.
#[derive(Debug, Clone, Copy)]
struct Traffic {
    /// The bytes both ends of the lower peer's dial received, summed.
    lower: u64,
    /// The data segments each end of the lower peer's dial sent, the
    /// dialling end's first.
    lower_segments: [u64; 2],
    /// The bytes both ends of the higher peer's dial received, summed.
    higher: u64,
}

/// [`Traffic`], read now. Panics unless there are exactly two
/// connections, one dialled each way.
fn traffic(ip: Ipv4Addr, lower_listen: u16, higher_listen: u16) -> Traffic {
    let all = sockets(ip, [lower_listen, higher_listen]);
    // A dial's own socket: its far end is the other's listen port and
    // its own is not its listen port (a fresh one, §12).
    let dial = |own: u16, far: u16| -> (u16, u16) {
        let mine: Vec<&Socket> = all
            .iter()
            .filter(|s| s.remote == far && s.local != own)
            .collect();
        assert_eq!(mine.len(), 1, "exactly one dial toward {far}: {all:?}");
        (mine[0].local, far)
    };
    let ends_of = |(a, b): (u16, u16)| -> [&Socket; 2] {
        let ends: Vec<&Socket> = all
            .iter()
            .filter(|s| (s.local, s.remote) == (a, b) || (s.local, s.remote) == (b, a))
            .collect();
        assert_eq!(
            ends.len(),
            2,
            "both ends of {a}<->{b} on this host: {all:?}"
        );
        let dialling = usize::from(ends[0].local != a);
        [ends[dialling], ends[1 - dialling]]
    };
    let lower = ends_of(dial(lower_listen, higher_listen));
    let higher = ends_of(dial(higher_listen, lower_listen));
    Traffic {
        lower: lower.iter().map(|s| s.received).sum(),
        lower_segments: lower.map(|s| s.data_segments_out),
        higher: higher.iter().map(|s| s.received).sum(),
    }
}

async fn established(runtime: &SwarmRuntime) -> usize {
    runtime
        .status(None)
        .await
        .expect("status")
        .dial_gate
        .established_connections
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_runtimes_dialling_each_other_at_once_both_hold_the_lower_peers_dial() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (one, two) = (ProfileIdentity::generate(), ProfileIdentity::generate());
    let (id_one, id_two) = (
        one.transport_identity().expect("a peer id"),
        two.transport_identity().expect("a peer id"),
    );
    let config = || SubstrateConfig {
        // Short, so a connection nothing holds would be gone long before
        // the window ends -- were gossipsub's stream not on it.
        idle_timeout: Duration::from_secs(1),
        hold_data_plane: true,
        ..SubstrateConfig::default()
    };
    let a = SwarmRuntime::start(&one, config(), trusting(&id_two)).expect("starts");
    let b = SwarmRuntime::start(&two, config(), trusting(&id_one)).expect("starts");
    let listen = format!("/ip4/{ip}/tcp/0")
        .parse::<Multiaddr>()
        .expect("valid");
    let at_a = a.listen(listen.clone()).await.expect("listens");
    let at_b = b.listen(listen).await.expect("listens");
    let _ = a
        .learn(id_two.clone(), [at_b.to_string()])
        .await
        .expect("delivered");
    let _ = b
        .learn(id_one.clone(), [at_a.to_string()])
        .await
        .expect("delivered");

    // BOTH DIAL AT ONCE.
    let (from_a, from_b) = tokio::join!(a.reconnect(id_two.clone()), b.reconnect(id_one.clone()));
    from_a.expect("delivered").expect("admitted");
    from_b.expect("delivered").expect("admitted");

    // Both dials establish: two connections at each end.
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while established(&a).await != 2 || established(&b).await != 2 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "both crossing dials established within {PATIENCE:?}: a={} b={}",
            established(&a).await,
            established(&b).await
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let (lower_listen, higher_listen) = if id_one < id_two {
        (tcp_port(&at_a), tcp_port(&at_b))
    } else {
        (tcp_port(&at_b), tcp_port(&at_a))
    };
    // Settled: the handshakes, Identify and gossipsub's hello are done.
    tokio::time::sleep(Duration::from_secs(2)).await;
    let before = traffic(ip, lower_listen, higher_listen);
    tokio::time::sleep(PING_WINDOW).await;
    let after = traffic(ip, lower_listen, higher_listen);

    // Both still open, so a silent connection is an idle one, not a
    // closed one.
    assert_eq!((established(&a).await, established(&b).await), (2, 2));
    // THE CONTROL: the held connection was pinged in the window, so the
    // counters are live and the window long enough.
    assert!(
        after.lower > before.lower,
        "the lower peer's dial carried the hold's pings: {before:?} -> {after:?}"
    );
    // THE PROPERTY, first half: neither end pinged the other dial.
    assert_eq!(
        after.higher, before.higher,
        "the higher peer's dial carried nothing, so neither end holds it: \
         {before:?} -> {after:?}"
    );
    // And the second: BOTH ends pinged the lower peer's dial. Read from
    // the data segments each end of it sent in the window, which one
    // pinger and two tell apart: MEASURED, five runs each on 2026-11-14,
    // four per end with both ends pinging and three with only the lower
    // peer's end holding (`dialing::held_connections` planted to hold
    // nothing where the local id is the higher), so the count is the
    // oracle and not a derivation. Each pinger adds one 62-byte frame
    // each way (one ping and its echo); the other two segments per end
    // appear on a pinged connection whatever the pinger count and were
    // not traced to their frames. A coalesced write would lower a count
    // and fail this, never pass it.
    let sent = [
        after.lower_segments[0] - before.lower_segments[0],
        after.lower_segments[1] - before.lower_segments[1],
    ];
    assert!(
        sent.iter().all(|&n| n >= BOTH_PINGING),
        "each end pinged the lower peer's dial: {sent:?} data segments sent, \
         dialling end first ({before:?} -> {after:?})"
    );

    a.shutdown().await.expect("clean shutdown");
    b.shutdown().await.expect("clean shutdown");
}
