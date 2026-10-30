// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A relayed inbound is charged to its relay, over real sockets.
//!
//! `contracts/CONNECTIVITY.md` §10 requires a circuit's pre-auth cost
//! to be charged to the authenticated relay and never to the source
//! identity the circuit carries, since identities are free to mint. The
//! unit tests in `preauth_gate.rs` prove `source_label` on the address
//! shapes SPIKE-004 measured. This proves the hook is handed those
//! shapes by the relay transport a destination actually runs, so a
//! flood of fresh identities through one relay spends one bucket.
//!
//! The per-source start rate is narrowed to two a window, so that the
//! third start through the relay is refused by the rate and not by a
//! ceiling the test would have to fill. The relay is libp2p's own
//! server with no policy. The destination and the sources are what is
//! under test.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use futures::StreamExt as _;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_libp2p::runtime::relay_driver::{RelayClientSettings, StaticRelay};
use interweave_transport_libp2p::{
    RelayReservationOutcome, SubstrateConfig, SwarmEvent, SwarmRuntime,
};
use interweave_transport_runtime::TrustSources;
use interweave_transport_runtime::preauth::PreAuthLimitsBuilder;
use interweave_transport_runtime::relay::ReservationConfig;
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{Multiaddr, identify, identity, relay};

const PATIENCE: Duration = Duration::from_secs(20);
/// Long enough for a reservation after one failed dial to the relay:
/// the dial gate then holds the relay for `RETRY_BASE_MS` (30 s), the
/// relay client's own ladder (5 s, doubling, up to 5 s of jitter) asks
/// at about 5-10 s and 15-25 s and is refused at the gate each time, and
/// the first ask the gate admits is the rung after its floor, at most
/// about 50 s in. One more failed dial doubles the gate's hold past this.
const RESERVATION_PATIENCE: Duration = Duration::from_secs(60);

#[derive(NetworkBehaviour)]
struct RelayBehaviour {
    identify: identify::Behaviour,
    relay: relay::Behaviour,
    /// Answers the keepalive's pings, as this project's relays do: a
    /// relay that refuses ping is closed (`transport/libp2p/
    /// CONNECTIVITY.md` section 14, A 2026-10-25).
    ping: libp2p::ping::Behaviour,
}

/// libp2p's relay server with its defaults, listening on loopback and
/// driven by a task of its own for the rest of the test.
async fn relay_server() -> (TransportIdentity, Multiaddr) {
    let keys = identity::Keypair::generate_ed25519();
    let peer = TransportIdentity::parse(keys.public().to_peer_id().to_base58()).expect("canonical");
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keys)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("the same transport stack the subjects use")
        .with_behaviour(|k| RelayBehaviour {
            identify: identify::Behaviour::new(identify::Config::new(
                "/interweave-security-test-relay/1".to_owned(),
                k.public(),
            )),
            relay: relay::Behaviour::new(k.public().to_peer_id(), relay::Config::default()),
            ping: libp2p::ping::Behaviour::new(libp2p::ping::Config::new()),
        })
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
        .build();
    swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().expect("a listen address"))
        .expect("listens");
    let address = loop {
        if let libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } =
            tokio::time::timeout(PATIENCE, swarm.select_next_some())
                .await
                .expect("the relay binds")
        {
            break address;
        }
    };
    swarm.add_external_address(address.clone());
    tokio::spawn(async move {
        loop {
            swarm.select_next_some().await;
        }
    });
    (peer, address)
}

fn trust(data_plane: &[&TransportIdentity], infra: &[&TransportIdentity]) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new(data_plane.iter().map(|p| (*p).clone())).expect("a small allowlist"),
        InfrastructureSet::new(infra.iter().map(|p| (*p).clone())).expect("a small set"),
    )
}

fn relay_client(static_relays: Vec<StaticRelay>) -> RelayClientSettings {
    RelayClientSettings {
        static_relays,
        use_authorized_identify_relays: false,
        reservations: ReservationConfig::default(),
        direct_head_start_ms: 750,
    }
}

/// Wait for `subject` to be told it connected to `to`, or that its
/// dial to `to` failed, draining `target` meanwhile so that neither
/// runtime's event queue stalls the other. Only `to`'s outcome counts:
/// a source dialling a circuit connects to the relay first, and that
/// connection says nothing about the destination.
async fn outcome(
    subject: &mut SwarmRuntime,
    to: &TransportIdentity,
    target: &mut SwarmRuntime,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        tokio::select! {
            event = subject.next_event() => match event.expect("the subject is alive") {
                SwarmEvent::Connected { peer, .. } if peer == *to => return Ok(()),
                SwarmEvent::DialFailed { peer: Some(peer), detail, .. } if peer == *to => {
                    return Err(detail);
                }
                _ => {}
            },
            event = target.next_event() => {
                event.expect("the target is alive");
            }
            () = tokio::time::sleep_until(deadline) => panic!("no outcome within {PATIENCE:?}"),
        }
    }
}

#[tokio::test]
async fn fresh_identities_through_one_relay_share_its_bucket_and_a_direct_peer_from_its_host_does_not()
 {
    let (relay_peer, relay_addr) = relay_server().await;

    let sources: Vec<(ProfileIdentity, TransportIdentity)> = (0..4)
        .map(|_| {
            let id = ProfileIdentity::generate();
            let peer = id.transport_identity().expect("peer id");
            (id, peer)
        })
        .collect();
    let trusted: Vec<&TransportIdentity> = sources.iter().map(|(_, p)| p).collect();

    // THE DESTINATION: reserved on the relay, every source trusted, and
    // two starts a window from any one bucket.
    let target_id = ProfileIdentity::generate();
    let target_peer = target_id.transport_identity().expect("peer id");
    let mut target = SwarmRuntime::start(
        &target_id,
        SubstrateConfig {
            relay_client: Some(relay_client(vec![StaticRelay {
                peer: relay_peer.clone(),
                address: format!("{relay_addr}/p2p/{}", relay_peer.as_str()),
            }])),
            preauth: PreAuthLimitsBuilder {
                max_attempts_per_window: 2,
                ..PreAuthLimitsBuilder::default()
            }
            .build()
            .expect("two starts a window is a narrowing of the specified policy"),
            ..SubstrateConfig::default()
        },
        trust(&trusted, &[&relay_peer]),
    )
    .expect("the target starts");
    let direct = target
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("binds");
    let circuit: Multiaddr = format!(
        "{relay_addr}/p2p/{}/p2p-circuit/p2p/{}",
        relay_peer.as_str(),
        target_peer.as_str()
    )
    .parse()
    .expect("a circuit address");
    // What the target reported while waiting goes into the failure, so
    // a reservation that never comes says why: refused, retried, or
    // never asked for. The wait covers one failed dial to the relay
    // (`RESERVATION_PATIENCE` says how it plays out): on a busy runner
    // the first one failed with EADDRINUSE on loopback (CI run
    // 38165150729). This test is about the bucket, not about a first
    // dial's luck.
    let deadline = tokio::time::Instant::now() + RESERVATION_PATIENCE;
    let mut waited = Vec::new();
    loop {
        let Ok(event) = tokio::time::timeout_at(deadline, target.next_event()).await else {
            panic!(
                "no reservation accepted within {RESERVATION_PATIENCE:?}; the target saw {waited:#?}"
            )
        };
        match event {
            Some(SwarmEvent::RelayReservationChanged {
                outcome: RelayReservationOutcome::Accepted,
                ..
            }) => break,
            Some(other) => waited.push(other),
            None => panic!("the target stopped; it saw {waited:#?}"),
        }
    }

    // Three different identities through the one relay. Each is a
    // trusted, well-formed dial, so only the pre-auth bucket can tell
    // them apart from an ordinary peer.
    let mut through_relay = Vec::new();
    for (id, _) in sources.iter().take(3) {
        let mut source = SwarmRuntime::start(
            id,
            SubstrateConfig {
                relay_client: Some(relay_client(Vec::new())),
                ..SubstrateConfig::default()
            },
            trust(&[&target_peer], &[&relay_peer]),
        )
        .expect("a source starts");
        source
            .dial(target_peer.clone(), circuit.clone())
            .await
            .expect("the command reaches the task")
            .expect("a circuit to a data-plane peer is admitted by the source");
        let result = outcome(&mut source, &target_peer, &mut target).await;
        through_relay.push(result.is_ok());
        source.shutdown().await.expect("shuts down");
    }
    assert_eq!(
        through_relay,
        [true, true, false],
        "every circuit through one relay is one bucket: the third identity is refused by the \
         rate the first two spent, however fresh its PeerId"
    );

    // THE CONTROL: a direct inbound from the relay's own host (all of
    // this is 127.0.0.1) is a different bucket, so the relay's spent
    // window does not refuse it -- and the refusal above was the
    // relay's bucket, not the destination refusing everyone.
    let (id, _) = &sources[3];
    let mut neighbour =
        SwarmRuntime::start(id, SubstrateConfig::default(), trust(&[&target_peer], &[]))
            .expect("starts");
    neighbour
        .dial(target_peer.clone(), direct)
        .await
        .expect("the command reaches the task")
        .expect("admitted");
    assert_eq!(
        outcome(&mut neighbour, &target_peer, &mut target).await,
        Ok(()),
        "a direct peer from the relay's host is not charged to the relay"
    );

    neighbour.shutdown().await.expect("shuts down");
    target.shutdown().await.expect("shuts down");
}
