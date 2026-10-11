// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Kademlia poisoning and eclipse bounds, with hostile routers on the wire.
//!
//! A routing seat is only ever a TRUSTED server (ADR-0009), so the
//! attacker section 21 asks about is a trusted router that lies. A
//! `SwarmRuntime` cannot play one, because it answers honestly. Here
//! each hostile router is libp2p's own Kademlia server speaking this
//! network's derived protocol, its table stuffed by hand: dozens of
//! strangers, and a trusted peer at an impostor's address.
//!
//! What the gate ASSERTS (architect-cto's R2, 2026-10-11):
//! - every routing seat the subject takes is a trusted peer;
//! - a query's results stay within `max_results_per_query`;
//! - no honest router's seat is lost to the attackers;
//! - the poisoned address of a trusted peer, dialled by the walk itself,
//!   is quarantined, and the peer is reached the moment its true address
//!   is known;
//! - the subject answers its own control port promptly through an
//!   inbound query flood;
//! - record and provider writes are counted and stored nowhere.
//!
//! What it RECORDS, with no threshold: the eclipse ratio (explorations
//! whose results name no honest router) and the bootstrap diversity (the
//! distinct routers seated), at the N and M this host runs. The test
//! prints both as `MEASURE` lines, and the matrix row carries them.
//! kademlia-integration.md section 18 makes no Byzantine-resistance
//! claim, and no diversity floor is configured, so neither number has a
//! pass mark.
//!
//! On the host's private address, because ADR-0052 refuses a loopback
//! candidate from a peer.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::time::Duration;

use futures::StreamExt as _;
use interweave_kademlia_control_api::{
    KademliaCommand, KademliaEvent, KademliaMode, LookupKey, QueryClass, QueryHandle,
};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_libp2p::runtime::kademlia_driver::{KademliaSettings, kad_protocol};
use interweave_transport_libp2p::{
    DialRefusal, IDENTIFY_PROTOCOL_VERSION, SubstrateConfig, SwarmEvent, SwarmRuntime,
};
use interweave_transport_runtime::{DialDenial, TrustSources};
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{Multiaddr, PeerId, StreamProtocol, Swarm, identify, kad};
use tokio::sync::mpsc;

const NETWORK: &str = "stage-18-adversarial";
const PATIENCE: Duration = Duration::from_secs(20);
/// The query parallelism, and the least M that makes disjoint paths the
/// thing measured (R2).
const PARALLELISM: usize = 3;
/// Honest routers.
const HONEST: usize = 3;
/// Hostile routers, M.
const HOSTILE: usize = 3;
/// Strangers each hostile router claims to know.
const STRANGERS: usize = 40;
/// Explorations the eclipse ratio is measured over.
const EXPLORATIONS: u64 = 12;
const MAX_RESULTS: usize = 20;

fn settings() -> KademliaSettings {
    KademliaSettings {
        mode: KademliaMode::Server,
        network_id: NETWORK.to_owned(),
        kbucket_size: NonZeroUsize::new(20).expect("nonzero"),
        query_timeout: Duration::from_secs(10),
        parallelism: NonZeroUsize::new(PARALLELISM).expect("nonzero"),
        disjoint_query_paths: true,
        max_routing_peers: 256,
        max_results_per_query: NonZeroUsize::new(MAX_RESULTS).expect("nonzero"),
        max_concurrent_queries: NonZeroUsize::new(2).expect("nonzero"),
    }
}

fn config() -> SubstrateConfig {
    SubstrateConfig {
        kademlia: Some(settings()),
        ..SubstrateConfig::default()
    }
}

fn trusting(peers: &[&TransportIdentity]) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new(peers.iter().map(|p| (*p).clone())).expect("a handful"),
        InfrastructureSet::default(),
    )
}

fn identity_of(peer: &PeerId) -> TransportIdentity {
    TransportIdentity::parse(peer.to_base58()).expect("canonical")
}

fn peer_id(peer: &TransportIdentity) -> PeerId {
    peer.as_str().parse().expect("a libp2p identity")
}

/// The 32 key bytes of an Ed25519 `PeerId`: what an exploration point
/// toward that peer is.
fn key_of(peer: &TransportIdentity) -> [u8; 32] {
    let bytes = peer_id(peer).to_bytes();
    bytes[bytes.len() - 32..]
        .try_into()
        .expect("an ed25519 identity")
}

#[derive(NetworkBehaviour)]
struct Hostile {
    identify: identify::Behaviour,
    kad: kad::Behaviour<kad::store::MemoryStore>,
}

enum Order {
    /// Ask the subject for the peers closest to `n` random keys.
    FloodFindNode(usize),
    /// Store `n` records and announce `n` provider keys, so each goes to
    /// the subject as a write.
    Write(usize),
    /// Ask the subject for the record under this key.
    Get(Vec<u8>),
    /// Learn where a peer listens. A peer that dialled this router is
    /// connected but unroutable until its listen address is known.
    Know(PeerId, Multiaddr),
}

/// A hostile router on its own task: its table already stuffed, driven
/// by `Order`s, its Kademlia events forwarded.
struct HostileRouter {
    peer: TransportIdentity,
    address: Multiaddr,
    orders: mpsc::UnboundedSender<Order>,
    events: mpsc::UnboundedReceiver<kad::Event>,
}

async fn hostile_router(
    ip: std::net::Ipv4Addr,
    stuffing: Vec<(PeerId, Multiaddr)>,
) -> HostileRouter {
    let protocol = StreamProtocol::try_from_owned(kad_protocol(NETWORK)).expect("a protocol");
    let mut swarm: Swarm<Hostile> = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("the same transport stack the subject uses")
        .with_behaviour(|k| {
            let me = k.public().to_peer_id();
            let mut kad = kad::Behaviour::with_config(
                me,
                kad::store::MemoryStore::new(me),
                kad::Config::new(protocol.clone()),
            );
            kad.set_mode(Some(kad::Mode::Server));
            Hostile {
                identify: identify::Behaviour::new(identify::Config::new(
                    IDENTIFY_PROTOCOL_VERSION.to_owned(),
                    k.public(),
                )),
                kad,
            }
        })
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
        .build();
    swarm
        .listen_on(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .expect("listens");
    let address = loop {
        if let libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } =
            tokio::time::timeout(PATIENCE, swarm.select_next_some())
                .await
                .expect("binds")
        {
            break address;
        }
    };
    for (peer, address) in stuffing {
        swarm.behaviour_mut().kad.add_address(&peer, address);
    }
    let peer = identity_of(swarm.local_peer_id());
    let (orders, mut inbox) = mpsc::unbounded_channel();
    let (forward, events) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                order = inbox.recv() => match order {
                    Some(Order::FloodFindNode(n)) => {
                        for _ in 0..n {
                            let _ = swarm.behaviour_mut().kad.get_closest_peers(PeerId::random());
                        }
                    }
                    Some(Order::Write(n)) => {
                        for i in 0..n {
                            let key = kad::RecordKey::new(&format!("planted-{i}"));
                            let _ = swarm.behaviour_mut().kad.put_record(
                                kad::Record::new(key.clone(), b"planted".to_vec()),
                                kad::Quorum::One,
                            );
                            let _ = swarm.behaviour_mut().kad.start_providing(key);
                        }
                    }
                    Some(Order::Get(key)) => {
                        let _ = swarm.behaviour_mut().kad.get_record(kad::RecordKey::new(&key));
                    }
                    Some(Order::Know(peer, address)) => {
                        swarm.behaviour_mut().kad.add_address(&peer, address);
                    }
                    None => return,
                },
                event = swarm.select_next_some() => {
                    if let libp2p::swarm::SwarmEvent::Behaviour(HostileEvent::Kad(e)) = event {
                        let _ = forward.send(e);
                    }
                }
            }
        }
    });
    HostileRouter {
        peer,
        address,
        orders,
        events,
    }
}

/// Everything the subject reported while `until` was false.
async fn drive<F>(
    subject: &mut SwarmRuntime,
    what: &str,
    window: Duration,
    mut until: F,
) -> Vec<SwarmEvent>
where
    F: FnMut(&SwarmEvent) -> bool,
{
    let mut seen = Vec::new();
    let deadline = tokio::time::Instant::now() + window;
    loop {
        match tokio::time::timeout_at(deadline, subject.next_event()).await {
            Err(_) => return seen,
            Ok(None) => panic!("the subject stopped while {what}"),
            Ok(Some(event)) => {
                let done = until(&event);
                seen.push(event);
                if done {
                    return seen;
                }
            }
        }
    }
}

async fn listening(runtime: &SwarmRuntime, ip: std::net::Ipv4Addr) -> Multiaddr {
    runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("listens")
}

/// Dial `to` at `at` and wait until the subject seats it.
async fn seat(
    subject: &mut SwarmRuntime,
    to: &TransportIdentity,
    at: Multiaddr,
) -> Vec<SwarmEvent> {
    subject
        .dial(to.clone(), at)
        .await
        .expect("delivered")
        .expect("a trusted router is admitted");
    let events = drive(subject, "seating a router", PATIENCE, |e| {
        matches!(e, SwarmEvent::Kademlia { event: KademliaEvent::RoutingPeerAdded { peer } } if peer == to)
    })
    .await;
    assert!(
        events.iter().any(|e| matches!(e, SwarmEvent::Kademlia { event: KademliaEvent::RoutingPeerAdded { peer } } if peer == to)),
        "{to:?} was never seated: {events:?}"
    );
    events
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn hostile_routers_seat_no_stranger_evict_no_honest_route_and_their_poison_costs_its_address()
{
    let ip = interweave_test_support::net::require_private_interface_v4();

    // THE TRUSTED PEER THE ATTACKERS POISON: a real node, reachable,
    // known to nobody yet. Its key is what the explorations aim at, so
    // the walk contacts it, through whatever address the answers give.
    let target_id = ProfileIdentity::generate();
    let target_peer = target_id.transport_identity().expect("peer id");
    // The impostor answers at the address the attackers hand out for
    // the target, under another key.
    let impostor = SwarmRuntime::start(
        &ProfileIdentity::generate(),
        SubstrateConfig::default(),
        TrustSources::default(),
    )
    .expect("starts");
    let poisoned = listening(&impostor, ip).await;

    let subject_id = ProfileIdentity::generate();
    let subject_peer = subject_id.transport_identity().expect("peer id");

    // M HOSTILE ROUTERS, each claiming strangers and the poisoned target.
    let mut hostiles = Vec::new();
    for h in 0..HOSTILE {
        let mut stuffing: Vec<(PeerId, Multiaddr)> = (0..STRANGERS)
            .map(|s| {
                let port = 20_000 + u16::try_from(h * STRANGERS + s).expect("small");
                (
                    PeerId::random(),
                    format!("/ip4/{ip}/tcp/{port}").parse().expect("valid"),
                )
            })
            .collect();
        stuffing.push((peer_id(&target_peer), poisoned.clone()));
        hostiles.push(hostile_router(ip, stuffing).await);
    }
    let strangers_excluded = HOSTILE * STRANGERS;

    // HONEST ROUTERS, each knowing the others, so an honest path exists.
    let honest_ids: Vec<(ProfileIdentity, TransportIdentity)> = (0..HONEST)
        .map(|_| {
            let id = ProfileIdentity::generate();
            let peer = id.transport_identity().expect("peer id");
            (id, peer)
        })
        .collect();
    let honest_peers: Vec<TransportIdentity> = honest_ids.iter().map(|(_, p)| p.clone()).collect();
    let mut honest = Vec::new();
    for (id, _) in &honest_ids {
        let mut peers: Vec<&TransportIdentity> = honest_peers.iter().collect();
        peers.push(&subject_peer);
        let runtime =
            SwarmRuntime::start(id, config(), trusting(&peers)).expect("an honest router");
        let address = listening(&runtime, ip).await;
        honest.push((runtime, address));
    }
    for i in 1..HONEST {
        let (first, rest) = honest.split_at_mut(i);
        let to = honest_peers[i - 1].clone();
        let at = first[i - 1].1.clone();
        rest[0]
            .0
            .dial(to, at)
            .await
            .expect("delivered")
            .expect("admitted");
    }

    // THE SUBJECT trusts every router and the target, nothing else.
    let mut trusted: Vec<&TransportIdentity> = honest_peers.iter().collect();
    for h in &hostiles {
        trusted.push(&h.peer);
    }
    trusted.push(&target_peer);
    let mut subject =
        SwarmRuntime::start(&subject_id, config(), trusting(&trusted)).expect("subject");
    let _ = listening(&subject, ip).await;
    let mut events = Vec::new();
    for (i, (_, address)) in honest.iter().enumerate() {
        events.extend(seat(&mut subject, &honest_peers[i], address.clone()).await);
    }
    for h in &hostiles {
        events.extend(seat(&mut subject, &h.peer, h.address.clone()).await);
    }

    // THE WALKS, aimed at the target's key: every answer the hostiles
    // give names it at the impostor, beside forty strangers each.
    let point = key_of(&target_peer);
    let mut eclipsed = 0_u64;
    for n in 0..EXPLORATIONS {
        let mut aim = point;
        aim[31] ^= u8::try_from(n).expect("small");
        subject
            .kademlia(KademliaCommand::StartQuery {
                handle: QueryHandle::commanded(n + 1),
                class: QueryClass::Exploration,
                key: LookupKey::KeySpacePoint { point: aim },
            })
            .await
            .expect("delivered");
        let handle = QueryHandle::commanded(n + 1);
        let walk = drive(&mut subject, "an exploration", PATIENCE, |e| {
            matches!(e, SwarmEvent::Kademlia { event: KademliaEvent::QueryResults { handle: h, .. } | KademliaEvent::QueryFailed { handle: h, .. } } if *h == handle)
        })
        .await;
        for e in &walk {
            if let SwarmEvent::Kademlia {
                event:
                    KademliaEvent::QueryResults {
                        candidates,
                        handle: h,
                        ..
                    },
            } = e
                && *h == handle
            {
                let named: Vec<&TransportIdentity> =
                    candidates.as_slice().iter().map(|c| &c.peer_id).collect();
                assert!(
                    named.len() <= MAX_RESULTS,
                    "a query's results stay within max_results_per_query: {}",
                    named.len()
                );
                if !named.iter().any(|p| honest_peers.contains(p)) {
                    eclipsed += 1;
                }
            }
        }
        events.extend(walk);
    }

    // SEATS: only trusted peers ever, and no honest one lost.
    let trusted_set: BTreeSet<&TransportIdentity> = trusted.iter().copied().collect();
    let mut seated = BTreeSet::new();
    for e in &events {
        match e {
            SwarmEvent::Kademlia {
                event: KademliaEvent::RoutingPeerAdded { peer },
            } => {
                assert!(
                    trusted_set.contains(peer),
                    "a stranger the attackers named was seated: {peer:?}"
                );
                seated.insert(peer.clone());
            }
            SwarmEvent::Kademlia {
                event: KademliaEvent::RoutingPeerRemoved { peer },
            } => {
                assert!(
                    !honest_peers.contains(peer),
                    "an honest router's seat was lost to the attackers: {peer:?}"
                );
            }
            _ => {}
        }
    }

    // THE POISON PAID FOR ITSELF: the walk dialled the target at the
    // impostor, the identity check failed, and the address is
    // quarantined, whoever asks for it now.
    assert_eq!(
        subject
            .dial(target_peer.clone(), poisoned.clone())
            .await
            .expect("delivered"),
        Err(DialRefusal::Policy(DialDenial::AddressQuarantined)),
        "the walk dialled the poisoned address and it is quarantined"
    );

    // RESPONSIVE through an inbound flood: every hostile asks the
    // subject for the peers closest to a hundred keys while the subject
    // answers its own control port.
    for h in &hostiles {
        h.orders.send(Order::FloodFindNode(100)).expect("alive");
    }
    for _ in 0..10 {
        tokio::time::timeout(Duration::from_secs(2), subject.status(None))
            .await
            .expect("the subject answers its control port within 2 s under the flood")
            .expect("answered");
        let _ = drive(
            &mut subject,
            "the flood",
            Duration::from_millis(100),
            |_| false,
        )
        .await;
    }

    // THE TARGET IS NOT SUPPRESSED: the moment its true address is
    // learned, the book reaches it.
    let target =
        SwarmRuntime::start(&target_id, config(), trusting(&[&subject_peer])).expect("target");
    let good = listening(&target, ip).await;
    subject
        .learn(target_peer.clone(), [good.to_string()])
        .await
        .expect("delivered");
    subject
        .dial_peer(target_peer.clone())
        .await
        .expect("delivered")
        .expect("one poisoned address does not suppress the trusted peer");
    let reached = drive(
        &mut subject,
        "the target",
        PATIENCE,
        |e| matches!(e, SwarmEvent::Connected { peer, .. } if *peer == target_peer),
    )
    .await;
    assert!(
        reached
            .iter()
            .any(|e| matches!(e, SwarmEvent::Connected { peer, .. } if *peer == target_peer)),
        "the target is reached through its true address"
    );

    // RECORDED, no threshold (R2).
    println!(
        "MEASURE kademlia eclipse: N={} M={HOSTILE} parallelism={PARALLELISM} \
         explorations={EXPLORATIONS} eclipsed={eclipsed} strangers_named={strangers_excluded} \
         distinct_routers_seated={}",
        HONEST + HOSTILE,
        seated.len()
    );

    subject.shutdown().await.expect("stops");
    target.shutdown().await.expect("stops");
    impostor.shutdown().await.expect("stops");
    for (runtime, _) in honest {
        runtime.shutdown().await.expect("stops");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_trusted_routers_record_and_provider_writes_are_counted_and_stored_nowhere() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let subject_id = ProfileIdentity::generate();
    let subject_peer = subject_id.transport_identity().expect("peer id");
    // A trusted router that writes. Once the subject seats it, the
    // writer's own table holds the subject, so its writes go there.
    let mut writer = hostile_router(ip, Vec::new()).await;
    let mut subject =
        SwarmRuntime::start(&subject_id, config(), trusting(&[&writer.peer])).expect("subject");
    let subject_address = listening(&subject, ip).await;
    let writer_peer = writer.peer.clone();
    let _ = seat(&mut subject, &writer_peer, writer.address.clone()).await;
    writer
        .orders
        .send(Order::Know(peer_id(&subject_peer), subject_address))
        .expect("alive");

    let before = subject
        .status(None)
        .await
        .expect("answered")
        .kademlia_record_writes_dropped
        .expect("the driver is on");

    // THE FLOOD: fifty records and fifty provider announcements.
    writer.orders.send(Order::Write(50)).expect("alive");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let dropped = loop {
        let _ = drive(
            &mut subject,
            "the writes",
            Duration::from_millis(200),
            |_| false,
        )
        .await;
        let now = subject
            .status(None)
            .await
            .expect("answered")
            .kademlia_record_writes_dropped
            .expect("the driver is on");
        if now > before {
            break now;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no write reached the subject's count"
        );
    };
    assert!(dropped > before, "the writes are counted");

    // STORED NOWHERE: the writer asks the subject for what it planted,
    // and gets nothing back.
    while writer.events.try_recv().is_ok() {}
    writer
        .orders
        .send(Order::Get(b"planted-0".to_vec()))
        .expect("alive");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        tokio::select! {
            event = writer.events.recv() => {
                if let Some(kad::Event::OutboundQueryProgressed {
                    result: kad::QueryResult::GetRecord(result),
                    ..
                }) = event
                {
                    match result {
                        Ok(kad::GetRecordOk::FoundRecord(found)) => {
                            assert_ne!(
                                found.peer,
                                Some(peer_id(&subject_peer)),
                                "the subject served a record it was told to store: {found:?}"
                            );
                        }
                        _ => break,
                    }
                }
            }
            _ = subject.next_event() => {}
            () = tokio::time::sleep_until(deadline) => break,
        }
    }

    subject.shutdown().await.expect("stops");
}
