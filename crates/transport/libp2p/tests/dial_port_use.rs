// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A manual dial binds a fresh local port, not the listen port
//! (`CONNECTIVITY.md` §12, "Which local port a dial uses"), observed at
//! the far socket: the unit test `each_dial_origin_binds_the_port_section_12_says`
//! pins which origin gets which policy, and this test pins that the
//! policy reaches the dial libp2p makes.
//!
//! Loopback to loopback, where libp2p-tcp would otherwise reuse the
//! listen port. With the fresh-port request dropped from the admitted
//! dial, the source port observed is the listen port and this fails.

#![allow(clippy::expect_used)]

use std::time::Duration;

use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_libp2p::{SubstrateConfig, SwarmRuntime};
use interweave_transport_runtime::TrustSources;
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};
use libp2p::Multiaddr;
use libp2p::multiaddr::Protocol;

const PATIENCE: Duration = Duration::from_secs(20);

fn trusting(peers: &[&TransportIdentity]) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new(peers.iter().map(|p| (*p).clone())).expect("a handful"),
        InfrastructureSet::default(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_manual_dial_binds_a_fresh_local_port_not_the_listen_port() {
    let id = ProfileIdentity::generate();
    let target = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id");
    let subject =
        SwarmRuntime::start(&id, SubstrateConfig::default(), trusting(&[&target])).expect("starts");
    let listening = subject
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("the subject listens");
    let listen_port = listening
        .iter()
        .find_map(|p| match p {
            Protocol::Tcp(port) => Some(port),
            _ => None,
        })
        .expect("a tcp port");

    let far = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("binds");
    let far_port = far.local_addr().expect("an address").port();
    let address: Multiaddr = format!("/ip4/127.0.0.1/tcp/{far_port}")
        .parse()
        .expect("valid");
    subject
        .dial(target, address)
        .await
        .expect("delivered")
        .expect("admitted");

    let (_stream, source) = tokio::time::timeout(PATIENCE, far.accept())
        .await
        .expect("the dial arrives")
        .expect("accepted");
    assert_ne!(
        source.port(),
        listen_port,
        "a manual dial carries a fresh source port"
    );
    subject.shutdown().await.expect("clean shutdown");
}

/// The hub of the Kademlia test below: a raw libp2p node serving this
/// network's Kademlia protocol, holding one peer it will name to a
/// walk.
#[derive(libp2p::swarm::NetworkBehaviour)]
struct Hub {
    identify: libp2p::identify::Behaviour,
    kad: libp2p::kad::Behaviour<libp2p::kad::store::MemoryStore>,
}

/// A Kademlia walk's own dial binds a fresh local port (§12, A
/// 2026-10-14), observed at the far socket. The dial is the crate's --
/// `DialOpts::peer_id(peer).build()`, the listen port by default -- so
/// what moves it is the runtime's rebuild of it
/// (`Attributing::dialling_peers_from_a_fresh_port`); without the
/// rebuild the source port observed is the listen port and this fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_kademlia_walk_dials_from_a_fresh_local_port() {
    use futures::StreamExt as _;
    use interweave_kademlia_control_api::{
        KademliaCommand, KademliaMode, LookupKey, QueryClass, QueryHandle,
    };
    use interweave_transport_libp2p::runtime::kademlia_driver::{
        KademliaSettings, build_behaviour,
    };
    use std::num::NonZeroUsize;

    let ip = interweave_test_support::net::require_private_interface_v4();
    let settings = KademliaSettings {
        mode: KademliaMode::Server,
        network_id: "kad-port".to_owned(),
        kbucket_size: NonZeroUsize::new(20).expect("nonzero"),
        query_timeout: Duration::from_secs(10),
        parallelism: NonZeroUsize::new(3).expect("nonzero"),
        disjoint_query_paths: true,
        max_routing_peers: 256,
        max_results_per_query: NonZeroUsize::new(20).expect("nonzero"),
        max_concurrent_queries: NonZeroUsize::new(2).expect("nonzero"),
    };

    // THE FAR SOCKET, named by the hub as a peer the walk should meet.
    let far = tokio::net::TcpListener::bind((ip, 0)).await.expect("binds");
    let far_addr: Multiaddr = format!("/ip4/{ip}/tcp/{}", far.local_addr().expect("addr").port())
        .parse()
        .expect("valid");
    let far_identity = ProfileIdentity::generate();
    let far_peer = far_identity.transport_identity().expect("peer id");
    let far_pid: libp2p::PeerId = far_peer.as_str().parse().expect("a libp2p peer id");

    let mut hub = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("tcp")
        .with_behaviour(|key| {
            let mut kad = build_behaviour(&settings, key.public().to_peer_id()).expect("kad");
            kad.add_address(&far_pid, far_addr.clone());
            Hub {
                identify: libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
                    interweave_transport_libp2p::behaviour::IDENTIFY_PROTOCOL_VERSION.to_owned(),
                    key.public(),
                )),
                kad,
            }
        })
        .expect("behaviour")
        .build();
    hub.listen_on(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .expect("listens");
    let hub_addr = loop {
        if let libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } =
            hub.select_next_some().await
        {
            break address;
        }
    };
    // The hub's own Kademlia dials the far socket too, from its listen
    // port (a raw node, libp2p's default): those are not the subject's.
    let hub_port = hub_addr
        .iter()
        .find_map(|p| match p {
            Protocol::Tcp(port) => Some(port),
            _ => None,
        })
        .expect("a tcp port");
    let hub_peer: TransportIdentity =
        TransportIdentity::parse(hub.local_peer_id().to_string()).expect("peer id");
    tokio::spawn(async move {
        loop {
            let _ = hub.select_next_some().await;
        }
    });

    let subject = SwarmRuntime::start(
        &ProfileIdentity::generate(),
        SubstrateConfig {
            kademlia: Some(settings.clone()),
            ..SubstrateConfig::default()
        },
        trusting(&[&hub_peer, &far_peer]),
    )
    .expect("starts");
    let listening = subject
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the subject listens");
    let listen_port = listening
        .iter()
        .find_map(|p| match p {
            Protocol::Tcp(port) => Some(port),
            _ => None,
        })
        .expect("a tcp port");
    subject
        .dial(hub_peer, hub_addr)
        .await
        .expect("delivered")
        .expect("admitted");

    // Walk until the hub's answer sends the subject to the far socket;
    // the hub may not be routed at the first command, so ask again.
    let source = tokio::time::timeout(PATIENCE, async {
        let mut n = 0_u64;
        loop {
            n += 1;
            subject
                .kademlia(KademliaCommand::StartQuery {
                    handle: QueryHandle::commanded(n),
                    class: QueryClass::Exploration,
                    key: LookupKey::KeySpacePoint { point: [0x42; 32] },
                })
                .await
                .expect("command delivered");
            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            while let Ok(Ok((_stream, source))) =
                tokio::time::timeout_at(deadline, far.accept()).await
            {
                if source.port() != hub_port {
                    return source;
                }
            }
        }
    })
    .await
    .expect("the walk dials the peer the hub named");
    assert_eq!(source.ip(), std::net::IpAddr::V4(ip));
    assert_ne!(
        source.port(),
        listen_port,
        "a Kademlia walk's dial carries a fresh source port"
    );
    subject.shutdown().await.expect("clean shutdown");
}
