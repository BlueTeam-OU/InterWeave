// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Kademlia poisoning and eclipse bounds, with hostile routers on the wire.
//!
//! A routing seat is only ever a TRUSTED server (ADR-0009), so the
//! attacker section 21 asks about is a trusted router that lies. A
//! `SwarmRuntime` cannot play one, because it answers honestly. Here
//! each hostile router is libp2p's own Kademlia server speaking this
//! network's derived protocol, its table stuffed by hand: strangers
//! that really listen and really serve Kademlia, so a dial to one would
//! succeed if it were admitted, and a trusted peer at an impostor's
//! address.
//!
//! What the gate ASSERTS (architect-cto's R2, 2026-10-11):
//! - every routing seat the subject takes is a trusted peer, while the
//!   gate is seen refusing the walk's dials to the strangers;
//! - a query's results stay within `max_results_per_query`, which is
//!   below the routers that answer, and some query reaches it;
//! - the poisoned address of a trusted peer, dialled by the walk itself,
//!   is quarantined, and the peer is reached the moment its true address
//!   is known;
//! - the subject answers its own control port promptly while every
//!   hostile router floods it with `FIND_NODE`, and the hostiles' own
//!   results show the subject answered them;
//! - record and provider writes are counted and stored nowhere, and the
//!   writer's read of one ends with the subject having answered.
//! - a bucket full of connected honest seats gives no seat to a trusted
//!   newcomer in it, nor to a peer its answers name, and gives up none,
//!   while a seat freed by a revocation is taken by the next peer;
//! - a trusted router opening far more Kademlia streams than libp2p-kad
//!   serves on one connection has exactly that many kept, one of them
//!   answering a `FIND_NODE`, while another router is still answered.
//!
//! What it RECORDS, with no threshold: how many explorations ended with
//! results naming no honest router, and the distinct routers seated, at
//! the N and M this host runs. The test prints both as `MEASURE` lines.
//! Both are fixed by this topology rather than measured against an
//! attacker's skill: the test seats every honest router by its own dial,
//! so every walk starts from them, and a result names only peers that
//! answered. kademlia-integration.md section 18 makes no
//! Byzantine-resistance claim, and no diversity floor is configured, so
//! neither number has a pass mark.
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
    DialFailureClass, DialRefusal, IDENTIFY_PROTOCOL_VERSION, SubstrateConfig, SwarmEvent,
    SwarmRuntime,
};
use interweave_transport_runtime::{DialDenial, DialOrigin, TrustSources};
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{Multiaddr, PeerId, StreamProtocol, Swarm, identify, identity, kad};
use tokio::sync::{mpsc, oneshot};

const NETWORK: &str = "stage-18-adversarial";
const PATIENCE: Duration = Duration::from_secs(20);
/// The query parallelism, and the least M that makes disjoint paths the
/// thing measured (R2).
const PARALLELISM: usize = 3;
/// Honest routers.
const HONEST: usize = 3;
/// Hostile routers, M.
const HOSTILE: usize = 3;
/// Strangers each hostile router claims to know: real nodes, listening
/// and serving Kademlia, that the subject does not trust.
const STRANGERS: usize = 4;
/// Explorations the walk counts are taken over.
const EXPLORATIONS: u64 = 12;
/// Below the six routers that answer, so the cap is what limits a
/// result.
const MAX_RESULTS: usize = 4;

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
    /// Answered once the table holds it, so a caller that goes on to
    /// start a walk on another task knows the router can name it
    /// (`HostileRouter::know`).
    Know(PeerId, Multiaddr, oneshot::Sender<()>),
}

/// A hostile router on its own task: its table already stuffed, driven
/// by `Order`s, its Kademlia events forwarded.
struct HostileRouter {
    peer: TransportIdentity,
    address: Multiaddr,
    orders: mpsc::UnboundedSender<Order>,
    events: mpsc::UnboundedReceiver<kad::Event>,
}

impl HostileRouter {
    /// Tell the router where `peer` listens, and wait until it has it.
    async fn know(&self, peer: PeerId, address: Multiaddr) {
        let (done, known) = oneshot::channel();
        self.orders
            .send(Order::Know(peer, address, done))
            .expect("alive");
        known.await.expect("the router's task answers");
    }
}

async fn hostile_router(
    ip: std::net::Ipv4Addr,
    stuffing: Vec<(PeerId, Multiaddr)>,
) -> HostileRouter {
    hostile_router_as(identity::Keypair::generate_ed25519(), ip, stuffing).await
}

/// `hostile_router` under keys the caller chose, so its place in the
/// subject's routing table is known in advance.
async fn hostile_router_as(
    keys: identity::Keypair,
    ip: std::net::Ipv4Addr,
    stuffing: Vec<(PeerId, Multiaddr)>,
) -> HostileRouter {
    let protocol = StreamProtocol::try_from_owned(kad_protocol(NETWORK)).expect("a protocol");
    let mut swarm: Swarm<Hostile> = libp2p::SwarmBuilder::with_existing_identity(keys)
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
                    Some(Order::Know(peer, address, done)) => {
                        swarm.behaviour_mut().kad.add_address(&peer, address);
                        let _ = done.send(());
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
#[expect(
    clippy::too_many_lines,
    reason = "one scenario over real sockets; its length is its steps"
)]
async fn hostile_routers_seat_no_stranger_and_their_poison_costs_its_address() {
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
    // A stranger is a keyed node that listens and serves this network's
    // Kademlia: were the walk's dial to it admitted, it would connect,
    // identify and offer itself for a seat. Held alive to the end.
    let mut strangers = Vec::new();
    let mut hostiles = Vec::new();
    for _ in 0..HOSTILE {
        let mut stuffing: Vec<(PeerId, Multiaddr)> = Vec::new();
        for _ in 0..STRANGERS {
            let stranger = hostile_router(ip, Vec::new()).await;
            stuffing.push((peer_id(&stranger.peer), stranger.address.clone()));
            strangers.push(stranger);
        }
        stuffing.push((peer_id(&target_peer), poisoned.clone()));
        hostiles.push(hostile_router(ip, stuffing).await);
    }
    let strangers_named = strangers.len();

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
    let hostile_peers: Vec<TransportIdentity> = hostiles.iter().map(|h| h.peer.clone()).collect();
    let mut trusted: Vec<&TransportIdentity> = honest_peers.iter().collect();
    trusted.extend(hostile_peers.iter());
    trusted.push(&target_peer);
    let mut subject =
        SwarmRuntime::start(&subject_id, config(), trusting(&trusted)).expect("subject");
    let subject_address = listening(&subject, ip).await;
    let mut events = Vec::new();
    for (i, (_, address)) in honest.iter().enumerate() {
        events.extend(seat(&mut subject, &honest_peers[i], address.clone()).await);
    }
    for h in &hostiles {
        events.extend(seat(&mut subject, &h.peer, h.address.clone()).await);
    }

    // THE WALKS, aimed at the target's key: every answer the hostiles
    // give names it at the impostor, beside its strangers.
    let point = key_of(&target_peer);
    let (mut answered, mut eclipsed, mut failed, mut at_cap) = (0_u64, 0_u64, 0_u64, 0_u64);
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
            match e {
                SwarmEvent::Kademlia {
                    event:
                        KademliaEvent::QueryResults {
                            candidates,
                            handle: h,
                            ..
                        },
                } if *h == handle => {
                    let named: Vec<&TransportIdentity> =
                        candidates.as_slice().iter().map(|c| &c.peer_id).collect();
                    assert!(
                        named.len() <= MAX_RESULTS,
                        "a query's results stay within max_results_per_query: {}",
                        named.len()
                    );
                    answered += 1;
                    if named.len() == MAX_RESULTS {
                        at_cap += 1;
                    }
                    if !named.iter().any(|p| honest_peers.contains(p)) {
                        eclipsed += 1;
                    }
                }
                SwarmEvent::Kademlia {
                    event: KademliaEvent::QueryFailed { handle: h, .. },
                } if *h == handle => failed += 1,
                _ => {}
            }
        }
        events.extend(walk);
    }
    // The control for the cap: more routers answered than it admits, so
    // some result was cut to it. Without this, a cap no walk can reach
    // passes the bound above for free.
    assert!(
        at_cap > 0,
        "some walk's results reached max_results_per_query ({answered} answered, {failed} failed)"
    );

    // THE WALK'S DIAL HAS ENDED before the address is asked about. A walk
    // settles on its results, and the dial it made to the poisoned
    // address can still be in its handshake then: under a loaded
    // full-workspace run the assertion below once ran ahead of it (j130).
    // Waited for on what the subject itself reports, and never by
    // dialling: a dial of the test's own would quarantine the address
    // and prove nothing about the walk. Either signal will do, since the
    // event is dropped when the outbox is full and the gate row counts
    // only addresses in the book. The subject keeps draining meanwhile,
    // or its Swarm stops being polled and the handshake never ends.
    let mismatched = |e: &SwarmEvent| matches!(e, SwarmEvent::DialFailed { peer: Some(p), class: DialFailureClass::IdentityMismatch, .. } if *p == target_peer);
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while !events.iter().any(mismatched) {
        let rows = subject
            .peer_gates(vec![target_peer.clone()])
            .await
            .expect("the task answers");
        if rows.iter().any(|row| row.quarantined_until_ms.is_some()) {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no walk's dial of the poisoned address ended within {PATIENCE:?}: {rows:?}"
        );
        events.extend(
            drive(
                &mut subject,
                "waiting for the walk's dial to end",
                Duration::from_millis(100),
                &mismatched,
            )
            .await,
        );
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

    // RESPONSIVE through an inbound flood: every hostile asks for the
    // peers closest to a hundred keys while the subject answers its own
    // control port. A hostile holds the subject's connection as an
    // inbound, which leaves it unroutable there until its listen address
    // is known, so each is told first, or no FIND_NODE would reach it.
    let subject_libp2p = peer_id(&subject_peer);
    for h in &hostiles {
        h.know(subject_libp2p, subject_address.clone()).await;
        h.orders.send(Order::FloodFindNode(100)).expect("alive");
    }
    for _ in 0..10 {
        tokio::time::timeout(Duration::from_secs(2), subject.status(None))
            .await
            .expect("the subject answers its control port within 2 s under the flood")
            .expect("answered");
        events.extend(
            drive(
                &mut subject,
                "the flood",
                Duration::from_millis(100),
                |_| false,
            )
            .await,
        );
    }
    // The control: the flood landed. A query's result names only peers
    // that answered it, so a hostile result naming the subject is the
    // subject having served a FIND_NODE of the flood.
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let mut served = 0_usize;
    while served == 0 {
        for h in &mut hostiles {
            while let Ok(e) = h.events.try_recv() {
                if let kad::Event::OutboundQueryProgressed {
                    result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                    ..
                } = e
                    && ok.peers.iter().any(|p| p.peer_id == subject_libp2p)
                {
                    served += 1;
                }
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no hostile query of the flood was answered by the subject"
        );
        events.extend(
            drive(
                &mut subject,
                "the flood's answers",
                Duration::from_millis(100),
                |_| false,
            )
            .await,
        );
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
    events.extend(reached);

    // SEATS: only trusted peers, over everything the subject reported:
    // seating, the walks, the flood and the target.
    let trusted_set: BTreeSet<&TransportIdentity> = trusted.iter().copied().collect();
    let mut seated = BTreeSet::new();
    for e in &events {
        if let SwarmEvent::Kademlia {
            event: KademliaEvent::RoutingPeerAdded { peer },
        } = e
        {
            assert!(
                trusted_set.contains(peer),
                "a stranger the attackers named was seated: {peer:?}"
            );
            seated.insert(peer.clone());
        }
    }
    // The control for the seats above: the walks MET the strangers and
    // the gate refused the dials, so "no stranger seated" is a refusal
    // observed, not a stranger nobody tried.
    let refused = subject
        .dial_refusals()
        .counts()
        .get(&(
            Some(DialOrigin::KademliaQuery),
            Some(DialDenial::Unauthorized),
        ))
        .copied()
        .unwrap_or(0);
    assert!(
        refused > 0,
        "the gate refused a walk's dial to a stranger: {:?}",
        subject.dial_refusals().recent()
    );

    // RECORDED, no threshold (R2): every walk is counted by how it
    // ended, so the denominator printed is the one observed.
    println!(
        "MEASURE kademlia eclipse: N={} M={HOSTILE} parallelism={PARALLELISM} \
         max_results={MAX_RESULTS} explorations={EXPLORATIONS} answered={answered} \
         failed={failed} unanswered={} eclipsed={eclipsed} at_cap={at_cap} \
         strangers_named={strangers_named} refused_walk_dials={refused} \
         flood_answers_seen={served} distinct_routers_seated={}",
        HONEST + HOSTILE,
        EXPLORATIONS - answered - failed,
        seated.len()
    );

    subject.shutdown().await.expect("stops");
    target.shutdown().await.expect("stops");
    impostor.shutdown().await.expect("stops");
    for (runtime, _) in honest {
        runtime.shutdown().await.expect("stops");
    }
    drop(strangers);
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
    writer.know(peer_id(&subject_peer), subject_address).await;

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
    // and gets nothing back. The read must END, with the subject among
    // those that answered: a subject that never answered would look the
    // same as one that holds nothing.
    while writer.events.try_recv().is_ok() {}
    writer
        .orders
        .send(Order::Get(b"planted-0".to_vec()))
        .expect("alive");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let answered_by = loop {
        tokio::select! {
            event = writer.events.recv() => {
                if let Some(kad::Event::OutboundQueryProgressed {
                    result: kad::QueryResult::GetRecord(result),
                    step,
                    stats,
                    ..
                }) = event
                {
                    if let Ok(kad::GetRecordOk::FoundRecord(found)) = &result {
                        assert_ne!(
                            found.peer,
                            Some(peer_id(&subject_peer)),
                            "the subject served a record it was told to store: {found:?}"
                        );
                    }
                    if step.last {
                        break stats.num_successes();
                    }
                }
            }
            _ = subject.next_event() => {}
            () = tokio::time::sleep_until(deadline) => {
                panic!("the writer's read of its planted record never ended")
            }
        }
    };
    assert!(
        answered_by > 0,
        "the read ended with the subject having answered it"
    );

    subject.shutdown().await.expect("stops");
}

/// Routers in the subject's farthest bucket: one bucket's worth of
/// honest seats, and the peers that would take one.
const BUCKET: usize = 8;
const NEWCOMERS: usize = 2;
const OBSERVED: usize = 2;
/// How long a seat a full bucket refused has to show up anyway. The
/// control's seat, through the same path with room, must land inside it.
const SETTLE: Duration = Duration::from_secs(3);

/// Fresh keys whose `PeerId` falls in `subject`'s bucket 255, the half
/// of the key space farthest from it, so every one competes for the
/// same `BUCKET` slots.
fn far_keys(subject: &PeerId, n: usize) -> Vec<identity::Keypair> {
    let me = kad::KBucketKey::from(*subject);
    std::iter::repeat_with(identity::Keypair::generate_ed25519)
        .filter(|k| {
            kad::KBucketKey::from(k.public().to_peer_id())
                .distance(&me)
                .ilog2()
                == Some(255)
        })
        .take(n)
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_full_bucket_of_connected_seats_gives_newcomers_and_their_observations_no_seat() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let subject_id = ProfileIdentity::generate();
    let subject_peer = subject_id.transport_identity().expect("peer id");
    let subject_libp2p = peer_id(&subject_peer);
    let mut keys = far_keys(&subject_libp2p, BUCKET + NEWCOMERS + OBSERVED + 1).into_iter();

    // OBSERVED: trusted, listening, serving Kademlia, and known to the
    // subject only through what the newcomers answer its walks.
    let mut observed = Vec::new();
    for k in keys.by_ref().take(OBSERVED) {
        observed.push(hostile_router_as(k, ip, Vec::new()).await);
    }
    // NEWCOMERS: trusted routers that connect and flood, and whose
    // answers name the observed.
    let named: Vec<(PeerId, Multiaddr)> = observed
        .iter()
        .map(|o| (peer_id(&o.peer), o.address.clone()))
        .collect();
    let mut newcomers = Vec::new();
    for k in keys.by_ref().take(NEWCOMERS) {
        newcomers.push(hostile_router_as(k, ip, named.clone()).await);
    }
    // HONEST: one full bucket, naming nobody until every one is seated.
    // libp2p-kad starts a bootstrap 500 ms after a seat is taken, and an
    // honest router that already named the newcomers would have it dial
    // and seat them while the bucket still had room -- the last honest
    // router then never seated (j92: 3 runs in 6).
    let mut honest = Vec::new();
    for k in keys.by_ref().take(BUCKET) {
        honest.push(hostile_router_as(k, ip, Vec::new()).await);
    }
    // LATE: the control's newcomer, held back until a seat is free.
    let late = hostile_router_as(keys.next().expect("one more far key"), ip, Vec::new()).await;

    let honest_peers: Vec<TransportIdentity> = honest.iter().map(|h| h.peer.clone()).collect();
    let mut outsiders: Vec<TransportIdentity> = newcomers.iter().map(|n| n.peer.clone()).collect();
    outsiders.extend(observed.iter().map(|o| o.peer.clone()));
    let mut everyone: Vec<&TransportIdentity> = honest_peers.iter().collect();
    everyone.extend(outsiders.iter());
    everyone.push(&late.peer);
    let mut subject = SwarmRuntime::start(
        &subject_id,
        SubstrateConfig {
            kademlia: Some(KademliaSettings {
                kbucket_size: NonZeroUsize::new(BUCKET).expect("nonzero"),
                ..settings()
            }),
            // A seat is evictable once its connection is gone (libp2p-kad
            // queues a newcomer behind a disconnected occupant), so the
            // honest connections are kept for the whole run: what is
            // under test is the connected seat.
            idle_timeout: Duration::from_secs(600),
            ..SubstrateConfig::default()
        },
        trusting(&everyone),
    )
    .expect("subject");
    let subject_address = listening(&subject, ip).await;

    let mut events = Vec::new();
    for h in &honest {
        events.extend(seat(&mut subject, &h.peer, h.address.clone()).await);
    }
    // The bucket is full: now each honest router names the newcomers, so
    // a walk that starts at an honest seat goes on to ask a newcomer.
    for h in &honest {
        for n in &newcomers {
            h.know(peer_id(&n.peer), n.address.clone()).await;
        }
    }

    // THE NEWCOMERS CONNECT and identify as servers of this network,
    // with every seat in their bucket held by a connected peer.
    for n in &newcomers {
        subject
            .dial(n.peer.clone(), n.address.clone())
            .await
            .expect("delivered")
            .expect("a trusted router is admitted");
        events.extend(
            drive(
                &mut subject,
                "a newcomer connecting",
                PATIENCE,
                |e| matches!(e, SwarmEvent::Connected { peer, .. } if *peer == n.peer),
            )
            .await,
        );
        n.know(subject_libp2p, subject_address.clone()).await;
        n.orders.send(Order::FloodFindNode(50)).expect("alive");
    }

    // THE WALKS: the honest seats name the newcomers, the newcomers
    // name the observed, and the walk dials them.
    for n in 0..4_u64 {
        let mut aim = key_of(&outsiders[usize::try_from(n).expect("small") % outsiders.len()]);
        aim[31] ^= u8::try_from(n).expect("small");
        let handle = QueryHandle::commanded(n + 1);
        subject
            .kademlia(KademliaCommand::StartQuery {
                handle,
                class: QueryClass::Exploration,
                key: LookupKey::KeySpacePoint { point: aim },
            })
            .await
            .expect("delivered");
        events.extend(
            drive(&mut subject, "an exploration", PATIENCE, |e| {
                matches!(e, SwarmEvent::Kademlia { event: KademliaEvent::QueryResults { handle: h, .. } | KademliaEvent::QueryFailed { handle: h, .. } } if *h == handle)
            })
            .await,
        );
    }
    // Time for any seat the newcomers or the observed could take to be
    // reported; the control below is held to taking one inside it.
    events.extend(drive(&mut subject, "settling", SETTLE, |_| false).await);

    // The controls: the observations reached the subject's seat path,
    // since the walk connected to every observed peer, and the flood
    // was answered.
    for o in &observed {
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SwarmEvent::Connected { peer, .. } if *peer == o.peer)),
            "the walk never connected to observed {:?}: {events:?}",
            o.peer
        );
    }
    let mut served = false;
    for n in &mut newcomers {
        while let Ok(e) = n.events.try_recv() {
            if let kad::Event::OutboundQueryProgressed {
                result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                ..
            } = e
                && ok.peers.iter().any(|p| p.peer_id == subject_libp2p)
            {
                served = true;
            }
        }
    }
    assert!(served, "the subject answered the newcomers' flood");

    // THE BOUND: no honest seat was given up, and no newcomer or
    // observed peer took one. And the premise: every occupant stayed
    // connected. A disconnected one is replaced only after libp2p-kad's
    // 60 s pending timeout, far past this run, so without this check the
    // bound would pass for a bucket of disconnected seats as well.
    for e in &events {
        match e {
            SwarmEvent::Disconnected { peer, .. } if honest_peers.contains(peer) => {
                panic!("occupant {peer:?} disconnected, so its seat was not a connected one")
            }
            SwarmEvent::Kademlia {
                event: KademliaEvent::RoutingPeerRemoved { peer },
            } => panic!("seat {peer:?} was given up under a full bucket"),
            SwarmEvent::Kademlia {
                event: KademliaEvent::RoutingPeerAdded { peer },
            } if outsiders.contains(peer) => {
                panic!("{peer:?} took a seat in a full bucket of connected peers")
            }
            _ => {}
        }
    }

    // THE CONTROL FOR THE BOUND: a seat freed by revoking one honest
    // router's trust goes to the next peer in that bucket that asks, so
    // the newcomers were kept out by the full bucket and not by a seat
    // path this run never reached.
    let freed = honest_peers[0].clone();
    let mut kept: Vec<&TransportIdentity> = everyone.clone();
    kept.retain(|p| **p != freed);
    subject.set_trust(trusting(&kept)).await.expect("applied");
    let removal = drive(&mut subject, "the revocation", PATIENCE, |e| {
        matches!(e, SwarmEvent::Kademlia { event: KademliaEvent::RoutingPeerRemoved { peer } } if *peer == freed)
    })
    .await;
    assert!(
        removal.iter().any(|e| matches!(e, SwarmEvent::Kademlia { event: KademliaEvent::RoutingPeerRemoved { peer } } if *peer == freed)),
        "revoking {freed:?} frees its seat: {removal:?}"
    );
    let started = tokio::time::Instant::now();
    let _ = seat(&mut subject, &late.peer, late.address.clone()).await;
    assert!(
        started.elapsed() < SETTLE,
        "a seat with room lands inside the settling window, so the window could have seen one: \
         {:?}",
        started.elapsed()
    );

    subject.shutdown().await.expect("stops");
    drop((honest, newcomers, observed, late));
}

/// libp2p-kad's inbound stream ceiling per connection (`MAX_NUM_STREAMS`
/// in `handler.rs`, read at 0.49.0). Not configurable, and the driver
/// has no inbound budget of its own, so this is the bound the stack has.
const KAD_INBOUND_STREAMS: usize = 32;
const FLOOD_STREAMS: usize = 100;

/// A raw stream opener: on its connection to `target` it opens `count`
/// outbound streams on `protocol` and hands each over unused. A stock
/// Kademlia peer cannot do this, since its own outbound is capped at the
/// same 32 streams; a hostile router is not a stock peer.
mod stream_flood {
    use std::collections::VecDeque;
    use std::task::{Context, Poll};

    use libp2p::core::transport::PortUse;
    use libp2p::core::upgrade::{DeniedUpgrade, ReadyUpgrade};
    use libp2p::core::{Endpoint, Multiaddr};
    use libp2p::swarm::handler::{ConnectionEvent, DialUpgradeError, FullyNegotiatedOutbound};
    use libp2p::swarm::{
        ConnectionDenied, ConnectionHandler, ConnectionHandlerEvent, ConnectionId, FromSwarm,
        NetworkBehaviour, Stream, SubstreamProtocol, THandler, THandlerInEvent, THandlerOutEvent,
        ToSwarm,
    };
    use libp2p::{PeerId, StreamProtocol};

    #[derive(Debug)]
    pub(super) enum Opened {
        Stream(Stream),
        Failed(String),
    }

    pub(super) struct Behaviour {
        target: PeerId,
        protocol: StreamProtocol,
        count: usize,
        events: VecDeque<Opened>,
    }

    impl Behaviour {
        pub(super) fn new(target: PeerId, protocol: StreamProtocol, count: usize) -> Self {
            Self {
                target,
                protocol,
                count,
                events: VecDeque::new(),
            }
        }

        fn handler(&mut self, peer: PeerId) -> Handler {
            // One connection's worth: a second connection to the target
            // opens nothing, so the count is one connection's.
            let to_open = if peer == self.target {
                std::mem::take(&mut self.count)
            } else {
                0
            };
            Handler {
                protocol: self.protocol.clone(),
                to_open,
                opened: VecDeque::new(),
            }
        }
    }

    impl NetworkBehaviour for Behaviour {
        type ConnectionHandler = Handler;
        type ToSwarm = Opened;

        fn handle_established_inbound_connection(
            &mut self,
            _: ConnectionId,
            peer: PeerId,
            _: &Multiaddr,
            _: &Multiaddr,
        ) -> Result<THandler<Self>, ConnectionDenied> {
            Ok(self.handler(peer))
        }

        fn handle_established_outbound_connection(
            &mut self,
            _: ConnectionId,
            peer: PeerId,
            _: &Multiaddr,
            _: Endpoint,
            _: PortUse,
        ) -> Result<THandler<Self>, ConnectionDenied> {
            Ok(self.handler(peer))
        }

        fn on_connection_handler_event(
            &mut self,
            _: PeerId,
            _: ConnectionId,
            event: THandlerOutEvent<Self>,
        ) {
            self.events.push_back(event);
        }

        fn poll(
            &mut self,
            _: &mut Context<'_>,
        ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
            self.events
                .pop_front()
                .map_or(Poll::Pending, |e| Poll::Ready(ToSwarm::GenerateEvent(e)))
        }

        fn on_swarm_event(&mut self, _: FromSwarm<'_>) {}
    }

    pub(super) struct Handler {
        protocol: StreamProtocol,
        to_open: usize,
        opened: VecDeque<Opened>,
    }

    impl ConnectionHandler for Handler {
        type FromBehaviour = std::convert::Infallible;
        type ToBehaviour = Opened;
        type InboundProtocol = DeniedUpgrade;
        type OutboundProtocol = ReadyUpgrade<StreamProtocol>;
        type InboundOpenInfo = ();
        type OutboundOpenInfo = ();

        fn listen_protocol(&self) -> SubstreamProtocol<DeniedUpgrade> {
            SubstreamProtocol::new(DeniedUpgrade, ())
        }

        fn connection_keep_alive(&self) -> bool {
            true
        }

        fn poll(
            &mut self,
            _: &mut Context<'_>,
        ) -> Poll<ConnectionHandlerEvent<ReadyUpgrade<StreamProtocol>, (), Opened>> {
            if let Some(opened) = self.opened.pop_front() {
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(opened));
            }
            if self.to_open > 0 {
                self.to_open -= 1;
                return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                    protocol: SubstreamProtocol::new(ReadyUpgrade::new(self.protocol.clone()), ()),
                });
            }
            Poll::Pending
        }

        fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
            match event {}
        }

        fn on_connection_event(
            &mut self,
            event: ConnectionEvent<'_, DeniedUpgrade, ReadyUpgrade<StreamProtocol>>,
        ) {
            match event {
                ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                    protocol: stream,
                    ..
                }) => self.opened.push_back(Opened::Stream(stream)),
                ConnectionEvent::DialUpgradeError(DialUpgradeError { error, .. }) => {
                    self.opened.push_back(Opened::Failed(error.to_string()));
                }
                _ => {}
            }
        }
    }
}

#[derive(NetworkBehaviour)]
struct Flooder {
    identify: identify::Behaviour,
    flood: stream_flood::Behaviour,
}

/// Whether `stream` is still open: a read the subject's end has not
/// closed stays pending, so `window` passing without one is "open".
async fn still_open(stream: &mut libp2p::Stream, window: Duration) -> bool {
    use futures::AsyncReadExt as _;
    let mut byte = [0_u8; 1];
    tokio::time::timeout(window, stream.read(&mut byte))
        .await
        .is_err()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_inbound_stream_flood_keeps_libp2p_kads_ceiling_and_the_subject_answers() {
    use futures::{AsyncReadExt as _, AsyncWriteExt as _};

    let ip = interweave_test_support::net::require_private_interface_v4();
    let subject_id = ProfileIdentity::generate();
    let subject_peer = subject_id.transport_identity().expect("peer id");
    let subject_libp2p = peer_id(&subject_peer);
    let protocol = StreamProtocol::try_from_owned(kad_protocol(NETWORK)).expect("a protocol");

    let mut flooder: Swarm<Flooder> = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("the same transport stack the subject uses")
        .with_behaviour(|k| Flooder {
            identify: identify::Behaviour::new(identify::Config::new(
                IDENTIFY_PROTOCOL_VERSION.to_owned(),
                k.public(),
            )),
            flood: stream_flood::Behaviour::new(subject_libp2p, protocol.clone(), FLOOD_STREAMS),
        })
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
        .build();
    let flooder_peer = identity_of(flooder.local_peer_id());
    // The witness: a second trusted router on a connection of its own.
    let mut witness = hostile_router(ip, Vec::new()).await;

    let mut subject = SwarmRuntime::start(
        &subject_id,
        config(),
        trusting(&[&flooder_peer, &witness.peer]),
    )
    .expect("subject");
    let subject_address = listening(&subject, ip).await;
    let _ = seat(&mut subject, &witness.peer, witness.address.clone()).await;
    witness.know(subject_libp2p, subject_address.clone()).await;

    // THE FLOOD: one connection, a hundred Kademlia streams, none of
    // which ever sends a request. The subject is driven meanwhile.
    flooder
        .dial(subject_address.clone())
        .expect("the flooder dials");
    let mut streams = Vec::new();
    let mut failed = Vec::new();
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while streams.len() + failed.len() < FLOOD_STREAMS {
        tokio::select! {
            event = flooder.select_next_some() => {
                if let libp2p::swarm::SwarmEvent::Behaviour(FlooderEvent::Flood(opened)) = event {
                    match opened {
                        stream_flood::Opened::Stream(s) => streams.push(s),
                        stream_flood::Opened::Failed(e) => failed.push(e),
                    }
                }
            }
            _ = subject.next_event() => {}
            () = tokio::time::sleep_until(deadline) => panic!(
                "{} of {FLOOD_STREAMS} streams opened, {} failed: {failed:?}",
                streams.len(),
                failed.len()
            ),
        }
    }
    assert!(failed.is_empty(), "every stream was opened: {failed:?}");
    let flooder_task = tokio::spawn(async move {
        loop {
            flooder.select_next_some().await;
        }
    });

    // RESPONSIVE: the control port, and the witness's query answered on
    // its own connection while the flood's streams are held.
    tokio::time::timeout(Duration::from_secs(2), subject.status(None))
        .await
        .expect("the subject answers its control port within 2 s of the flood")
        .expect("answered");
    witness.orders.send(Order::FloodFindNode(1)).expect("alive");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        tokio::select! {
            event = witness.events.recv() => {
                if let Some(kad::Event::OutboundQueryProgressed {
                    result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                    ..
                }) = event
                    && ok.peers.iter().any(|p| p.peer_id == subject_libp2p)
                {
                    break;
                }
            }
            _ = subject.next_event() => {}
            () = tokio::time::sleep_until(deadline) => {
                panic!("the witness's query was not answered by the subject during the flood")
            }
        }
    }

    // THE CEILING: what the subject kept of the hundred. A stream it
    // dropped reads end-of-stream; one it holds waits for a request.
    let mut kept = Vec::new();
    for mut stream in streams {
        if still_open(&mut stream, Duration::from_millis(50)).await {
            kept.push(stream);
        }
    }
    assert_eq!(
        kept.len(),
        KAD_INBOUND_STREAMS,
        "one connection keeps libp2p-kad's ceiling of inbound streams and no more"
    );

    // THE CONTROL: a kept stream is Kademlia's, serving. A FIND_NODE
    // (type 4, a 32-byte key), length-prefixed, gets an answer back.
    let request: Vec<u8> = [0x08, 0x04, 0x12, 0x20]
        .into_iter()
        .chain(std::iter::repeat_n(0x5a, 32))
        .collect();
    let stream = &mut kept[0];
    stream
        .write_all(&[u8::try_from(request.len()).expect("one varint byte")])
        .await
        .expect("written");
    stream.write_all(&request).await.expect("written");
    stream.flush().await.expect("flushed");
    let mut answer = [0_u8; 1];
    let read = tokio::time::timeout(PATIENCE, stream.read(&mut answer))
        .await
        .expect("a kept stream answers its FIND_NODE")
        .expect("read");
    assert_eq!(read, 1, "the answer has a length prefix");

    flooder_task.abort();
    subject.shutdown().await.expect("stops");
}
