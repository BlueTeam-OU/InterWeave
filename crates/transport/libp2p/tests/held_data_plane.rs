// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `SubstrateConfig::hold_data_plane` through the runtime, over real
//! sockets: the runtime names a data-plane peer's direct connection to
//! the keepalive, which holds it open past the idle timeout.
//!
//! The far end is a BARE Swarm answering ping and nothing else, on
//! purpose. Between two substrates the pinned gossipsub keeps every
//! connection open by a stream of its own (the `Extensions` hello it
//! parks on a peer's first connection), so a hold between two of them
//! would pass with the hold removed; against a peer that speaks no
//! gossipsub, the hold is the only thing that can keep the connection,
//! and the control -- the same run with the hold off -- closes at the
//! idle timeout.
//!
//! NOT OBSERVED HERE: a missed ping closing a held connection, and the
//! choice of one connection per peer -- `tests/relay_keepalive.rs` and
//! the unit test of `dialing::held_connections`.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use futures::StreamExt as _;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_libp2p::{SubstrateConfig, SwarmEvent, SwarmRuntime};
use interweave_transport_runtime::TrustSources;
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};
use libp2p::{Multiaddr, Swarm, ping};

const IDLE: Duration = Duration::from_secs(1);
const PATIENCE: Duration = Duration::from_secs(20);

fn bare_far() -> Swarm<ping::Behaviour> {
    libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("tcp")
        .with_behaviour(|_| ping::Behaviour::new(ping::Config::new()))
        .expect("behaviour")
        // Ten minutes: every close below is the SUBJECT's.
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
        .build()
}

/// Run the subject against a fresh bare peer for `window` after it
/// connects; whether it saw the peer disconnect.
async fn disconnected_within(hold_data_plane: bool, window: Duration) -> bool {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let mut far = bare_far();
    far.listen_on(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .expect("listens");
    let far_addr: Multiaddr = loop {
        if let libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } =
            far.select_next_some().await
        {
            break address;
        }
    };
    let far_peer =
        TransportIdentity::parse(far.local_peer_id().to_base58()).expect("a valid peer id");
    let far_task = tokio::spawn(async move {
        loop {
            let _ = far.select_next_some().await;
        }
    });

    let subject_id = ProfileIdentity::generate();
    let mut subject = SwarmRuntime::start(
        &subject_id,
        SubstrateConfig {
            idle_timeout: IDLE,
            hold_data_plane,
            ..SubstrateConfig::default()
        },
        TrustSources::new(
            PeerTrustPolicy::new([far_peer.clone()]).expect("one peer"),
            InfrastructureSet::default(),
        ),
    )
    .expect("starts");
    let _ = subject
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("listens");
    let _ = subject
        .learn(far_peer.clone(), [far_addr.to_string()])
        .await
        .expect("delivered");
    subject
        .reconnect(far_peer.clone())
        .await
        .expect("delivered")
        .expect("admitted");

    let mut connected = false;
    let mut deadline = tokio::time::Instant::now() + PATIENCE;
    let gone = loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, subject.next_event()).await {
            Ok(Some(SwarmEvent::Connected { peer, .. })) if peer == far_peer => {
                connected = true;
                deadline = tokio::time::Instant::now() + window;
            }
            Ok(Some(SwarmEvent::Disconnected { peer, .. })) if peer == far_peer => break true,
            Ok(Some(_)) => {}
            Ok(None) => panic!("the runtime stopped"),
            Err(_) => break false,
        }
    };
    assert!(connected, "the subject never connected");
    far_task.abort();
    subject.shutdown().await.expect("clean shutdown");
    gone
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_held_data_plane_connection_outlives_the_idle_timeout() {
    // THE CONTROL: the same peer, the same stack, the hold off -- closed
    // at the one-second idle timeout. Without it, a pass below could be
    // anything else keeping the connection.
    assert!(
        disconnected_within(false, IDLE * 5).await,
        "unheld, an idle data-plane connection closes at the idle timeout"
    );
    // HELD: five idle timeouts and still connected.
    assert!(
        !disconnected_within(true, IDLE * 5).await,
        "held, it outlives the idle timeout"
    );
}
