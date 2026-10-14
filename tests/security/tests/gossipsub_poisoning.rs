// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! GossipSub message-ID suppression, over real sockets.
//!
//! threat-model.md's row: a trusted peer races or reuses another
//! publisher's envelope to poison the mesh duplicate cache. The
//! mitigation is that the mesh id is the authenticated publisher and its
//! signed wire sequence number, never the application envelope or its
//! id. `behaviour.rs` unit-tests the id function, and `broadcast_inbound`
//! unit-tests the local dedup. Neither shows that the Swarm a profile
//! runs installs that function. This does: two trusted publishers send
//! BYTE-IDENTICAL envelopes, so a mesh keyed on anything the envelope
//! carries would take the second for a duplicate of the first and drop
//! it.
//!
//! What is not constructible here, and is the unit tests' instead: two
//! publishers at one sequence number. The backend assigns the sequence
//! number itself, so no publisher, honest or hostile, chooses it
//! (`tests/pubsub`'s invalid-signature test records where).

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_profile_config::{
    ChannelsConfig, DirectoryConfig, EndpointConfig, EndpointsConfig, ProfileConfig,
    RegistrationPolicy, TrustConfig, TrustPolicyKind,
};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{
    BroadcastMessageV1, ChannelId, EndpointId, MediaType, MessageId, Payload, TransportIdentity,
};
use interweave_transport_libp2p::runtime::{
    BroadcastChannels, DirectEndpoints, SubstrateConfig, SwarmEvent, SwarmRuntime,
};
use interweave_transport_runtime::TrustSources;
use interweave_transport_runtime::session_queue::BroadcastEvent;
use interweave_trust_api::{EndpointTrustPolicy, InfrastructureSet, PeerTrustPolicy};

/// A mesh forms over several heartbeats, not one round trip.
const PATIENCE: Duration = Duration::from_secs(20);

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

fn channel() -> ChannelId {
    ChannelId::parse("general").expect("valid channel id")
}

fn profile() -> ProfileConfig {
    ProfileConfig {
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
            default_direct_endpoint: Some(EndpointId::parse("human").expect("valid")),
            directory: DirectoryConfig::default(),
            entries: vec![EndpointConfig {
                id: EndpointId::parse("human").expect("valid"),
                enabled: true,
                advertise: false,
                allowed_client_kinds: Vec::new(),
                inbound: EndpointTrustPolicy::default(),
                outbound: EndpointTrustPolicy::default(),
            }],
        },
        discovery: interweave_profile_config::DiscoveryConfig::default(),
        channels: ChannelsConfig {
            desired: vec![channel()],
        },
    }
}

/// A runtime on the shipped config that trusts `peers`, with `general`
/// joined by `session`.
async fn node(
    identity: &ProfileIdentity,
    peers: &[&TransportIdentity],
    session: &str,
) -> SwarmRuntime {
    let runtime = SwarmRuntime::start(identity, SubstrateConfig::default(), trusting(peers))
        .expect("the node starts");
    runtime
        .configure_direct(DirectEndpoints::from_profile(&profile(), 64).expect("valid"))
        .await
        .expect("direct endpoints install");
    runtime
        .configure_broadcast(BroadcastChannels::from_profile(&profile(), 64).expect("valid"))
        .await
        .expect("broadcast channels install");
    runtime
        .join(channel(), session)
        .await
        .expect("lands")
        .expect("accepted");
    runtime
}

async fn connect(dialer: &mut SwarmRuntime, listener: &mut SwarmRuntime, to: &TransportIdentity) {
    let address = listener
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("loopback"))
        .await
        .expect("listens");
    dialer
        .dial(to.clone(), address)
        .await
        .expect("the command reaches the task")
        .expect("the dial is admitted");
    for side in [&mut *dialer, &mut *listener] {
        let deadline = tokio::time::Instant::now() + PATIENCE;
        loop {
            match tokio::time::timeout_at(deadline, side.next_event())
                .await
                .expect("connects")
            {
                Some(SwarmEvent::Connected { .. }) => break,
                Some(_) => {}
                None => panic!("the runtime stopped"),
            }
        }
    }
}

/// The one envelope both publishers send, byte for byte: the same
/// application id, timestamp, media type and body.
fn the_envelope() -> BroadcastMessageV1 {
    BroadcastMessageV1 {
        message_id: MessageId::from_bytes([7; 16]),
        sent_at_ms: 1_786_600_000_000,
        payload: Payload::at_ceiling(
            Some(MediaType::parse("text/plain").expect("valid media type")),
            b"the same bytes from two publishers".to_vec(),
        )
        .expect("within the ceiling"),
    }
}

/// Publish until the mesh has had time to form, then return. A repeat
/// from one publisher is a new wire message with its own sequence
/// number, and the receiver delivers the envelope once per publisher.
async fn publish_repeatedly(publisher: &SwarmRuntime, session: &str) {
    for _ in 0..12 {
        let _ = publisher
            .publish(channel(), session, the_envelope())
            .await
            .expect("the command lands");
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn drain_until_one(runtime: &SwarmRuntime, session: &str) -> Vec<BroadcastEvent> {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        let held = runtime
            .drain_session(session)
            .await
            .expect("the task answers");
        if !held.is_empty() || tokio::time::Instant::now() >= deadline {
            return held;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_second_publisher_sending_the_same_envelope_is_not_taken_for_a_duplicate() {
    let (a_id, a_peer) = who();
    let (b_id, b_peer) = who();
    let (c_id, c_peer) = who();

    let mut a = node(&a_id, &[&c_peer], "a").await;
    let mut b = node(&b_id, &[&c_peer], "b").await;
    let mut c = node(&c_id, &[&a_peer, &b_peer], "c").await;
    connect(&mut b, &mut c, &c_peer).await;
    connect(&mut a, &mut c, &c_peer).await;

    // B FIRST. Its envelope is delivered once and fills C's duplicate
    // cache with whatever the mesh keys on.
    publish_repeatedly(&b, "b").await;
    let first = drain_until_one(&c, "c").await;
    assert_eq!(first.len(), 1, "B's envelope reaches C once: {first:?}");
    assert_eq!(first[0].source_peer, b_peer);

    // THEN A, with the same bytes. Keyed on the publisher and its
    // sequence number, A's messages are new to the mesh. Keyed on the
    // envelope, they would all be duplicates of B's.
    publish_repeatedly(&a, "a").await;
    let second = drain_until_one(&c, "c").await;
    assert_eq!(
        second.len(),
        1,
        "A's identical envelope is a different publisher's message, not a duplicate: {second:?}"
    );
    assert_eq!(
        second[0].source_peer, a_peer,
        "delivered as A's, signed by A"
    );
    assert_eq!(second[0].message_id, the_envelope().message_id);

    for n in [a, b, c] {
        n.shutdown().await.expect("stops");
    }
}
