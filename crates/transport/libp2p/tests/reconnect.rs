// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `SwarmRuntime::reconnect`: the composition root's dial toward a peer
//! discovery found, under `DialOrigin::DiscoveryReconnect`
//! (`transport/libp2p/CONNECTIVITY.md` §11).
//!
//! - it reaches a data-plane peer through the book, over real sockets;
//! - it dials nothing while the peer holds a connection;
//! - toward an infrastructure-only peer it is refused at the gate, a
//!   policy refusal though the book holds a route -- a reconnection loop
//!   does not re-establish infrastructure (§4).
//! - a route that is this node's own listen address -- two nodes bound
//!   to one port, which libp2p-tcp's `SO_REUSEPORT` lets a uid do -- is
//!   refused by the Swarm before any socket, and the refusal is its own
//!   class, `own_listen_address`, not `dial_failed`; the control is a
//!   peer on a port of its own, reached by the same call.
//!
//! NOT OBSERVED HERE: the origin itself. `DiscoveryReconnect` and
//! `Manual` are admitted alike today (both name an application
//! destination), so a reconnect dialled under `Manual` would pass these
//! tests; the origin is the label plan §11 asks every dial to carry, and
//! nothing yet reports a command-path dial's origin.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_libp2p::{
    DialFailureClass, DialRefusal, SubstrateConfig, SwarmEvent, SwarmRuntime,
};
use interweave_transport_runtime::TrustSources;
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};

const PATIENCE: Duration = Duration::from_secs(20);

fn data_plane(peer: &TransportIdentity) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new([peer.clone()]).expect("one peer"),
        InfrastructureSet::default(),
    )
}

async fn wait_connected(runtime: &mut SwarmRuntime, peer: &TransportIdentity) {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, runtime.next_event()).await {
            Ok(Some(SwarmEvent::Connected { peer: got, .. })) if &got == peer => return,
            Ok(Some(_)) => {}
            Ok(None) => panic!("the runtime stopped"),
            Err(elapsed) => panic!("no connection within {PATIENCE:?} ({elapsed})"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reconnect_reaches_a_learned_peer_and_dials_nothing_once_connected() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let subject_id = ProfileIdentity::generate();
    let target_id = ProfileIdentity::generate();
    let subject_peer = subject_id.transport_identity().expect("peer id");
    let target_peer = target_id.transport_identity().expect("peer id");
    let mut subject = SwarmRuntime::start(
        &subject_id,
        SubstrateConfig::default(),
        data_plane(&target_peer),
    )
    .expect("starts");
    let target = SwarmRuntime::start(
        &target_id,
        SubstrateConfig::default(),
        data_plane(&subject_peer),
    )
    .expect("starts");
    let _ = subject
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("listens");
    let target_addr = target
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("listens");

    assert_eq!(
        subject
            .learn(target_peer.clone(), [target_addr.to_string()])
            .await
            .expect("delivered"),
        1
    );
    subject
        .reconnect(target_peer.clone())
        .await
        .expect("delivered")
        .expect("admitted");
    wait_connected(&mut subject, &target_peer).await;

    // CONNECTED, SO NOTHING IS DIALLED: no pending dial and one slot.
    subject
        .reconnect(target_peer.clone())
        .await
        .expect("delivered")
        .expect("answered without a dial");
    let gate = subject.status(None).await.expect("answered").dial_gate;
    assert_eq!(
        (gate.established_connections, gate.pending_dials),
        (1, 0),
        "{gate:?}"
    );

    subject.shutdown().await.expect("clean shutdown");
    target.shutdown().await.expect("clean shutdown");
}

/// ADR-0011 A 2026-11-14, the owner's word: no more than 30 s between
/// the settlement of one failed attempt and the start of the next,
/// however many have failed, so a far end that returns is dialled within
/// 30 s of its return. Over real sockets on the configured stack: the
/// far end is up and connected (the control that the route works), goes
/// down for SIX failed attempts -- past every step of the 30 s .. 5 min
/// ladder this replaced, whose second wait was already 60 s -- and
/// returns on the same address with the same identity just after a
/// failure, the worst moment, when the subject reconnects within 30 s.
///
/// Paused time, so six steps cost nothing; the refusals are the
/// kernel's, instant on a port nobody holds. The subject's `retry_tick`
/// is an HOUR, so the only thing that can start a retry inside the bound
/// is the scheduler's wake at the earliest due retry: a tick alone added
/// up to one more second (#672's +30.99 s), and in paused time a failure
/// settled on a tick makes a tick-only scheduler look exact. What this
/// does not exercise: the two exceptions the amendment names (the
/// per-tick budget and the pending-dial ceiling, both empty here), and a
/// route that times out rather than being refused, whose dial is longer
/// but whose wait after settlement is the same.
#[tokio::test(start_paused = true)]
async fn a_peer_down_past_several_steps_is_redialled_within_30_s_of_its_return() {
    const BOUND: Duration = Duration::from_secs(30);
    let ip = interweave_test_support::net::require_private_interface_v4();
    let subject_id = ProfileIdentity::generate();
    let target_id = ProfileIdentity::generate();
    let subject_peer = subject_id.transport_identity().expect("peer id");
    let target_peer = target_id.transport_identity().expect("peer id");
    let mut subject = SwarmRuntime::start(
        &subject_id,
        SubstrateConfig {
            retry_tick: Duration::from_secs(3_600),
            ..SubstrateConfig::default()
        },
        data_plane(&target_peer),
    )
    .expect("starts");
    let target = SwarmRuntime::start(
        &target_id,
        SubstrateConfig::default(),
        data_plane(&subject_peer),
    )
    .expect("starts");
    // A private listener of the family, or ADR-0052 refuses to learn a
    // private address at all.
    let _ = subject
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("listens");
    let target_addr = target
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("listens");
    assert_eq!(
        subject
            .learn(target_peer.clone(), [target_addr.to_string()])
            .await
            .expect("delivered"),
        1
    );
    subject
        .reconnect(target_peer.clone())
        .await
        .expect("delivered")
        .expect("admitted");
    wait_connected(&mut subject, &target_peer).await;

    // DOWN. One reconnect after the loss, as the composition's discovery
    // round would ask; every later attempt is the scheduler's.
    target.shutdown().await.expect("clean shutdown");
    loop {
        match tokio::time::timeout(PATIENCE, subject.next_event()).await {
            Ok(Some(SwarmEvent::Disconnected { peer, .. })) if peer == target_peer => break,
            Ok(Some(_)) => {}
            Ok(None) => panic!("the runtime stopped"),
            Err(elapsed) => panic!("the loss was never seen ({elapsed})"),
        }
    }
    subject
        .reconnect(target_peer.clone())
        .await
        .expect("delivered")
        .expect("admitted");

    // Each failure's settlement, and the delay the retry it scheduled
    // carries. A DialFailed is reported at settlement; the next one is
    // its attempt's start plus a kernel refusal, which takes no virtual
    // time, so the gap between two is the wait the rule bounds.
    let mut settled: Vec<tokio::time::Instant> = Vec::new();
    let mut delays: Vec<u64> = Vec::new();
    while settled.len() < 6 {
        match tokio::time::timeout(Duration::from_mins(50), subject.next_event()).await {
            Ok(Some(SwarmEvent::DialFailed { peer: Some(p), .. })) if p == target_peer => {
                settled.push(tokio::time::Instant::now());
            }
            Ok(Some(SwarmEvent::RetryScheduled { peer, delay_ms, .. })) if peer == target_peer => {
                delays.push(delay_ms);
            }
            Ok(Some(_)) => {}
            Ok(None) => panic!("the runtime stopped"),
            Err(elapsed) => panic!(
                "no retry within 50 min after {} failures ({elapsed})",
                settled.len()
            ),
        }
    }
    let gaps: Vec<Duration> = settled.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        gaps.iter().all(|g| *g <= BOUND),
        "a wait passed 30 s: {gaps:?}"
    );
    assert!(
        gaps.iter().all(|g| *g >= Duration::from_millis(29_999)),
        "and none came early, the gate's backoff being the same 30 s: {gaps:?}"
    );
    assert!(
        delays.iter().all(|d| *d == 30_000),
        "every retry scheduled is 30 s, the sixth as the first: {delays:?}"
    );

    // BACK, on the same address with the same identity, just after the
    // sixth failure settled: the longest the rule lets it wait.
    let target = SwarmRuntime::start(
        &target_id,
        SubstrateConfig::default(),
        data_plane(&subject_peer),
    )
    .expect("restarts");
    assert_eq!(
        target.listen(target_addr.clone()).await.expect("listens"),
        target_addr,
        "the same address"
    );
    let back = tokio::time::Instant::now();
    loop {
        match tokio::time::timeout(BOUND + PATIENCE, subject.next_event()).await {
            Ok(Some(SwarmEvent::Connected { peer, .. })) if peer == target_peer => break,
            Ok(Some(_)) => {}
            Ok(None) => panic!("the runtime stopped"),
            Err(elapsed) => panic!("never reconnected ({elapsed})"),
        }
    }
    let took = back.elapsed();
    // The bound plus the dial itself, which over loopback-speed sockets
    // in paused time is well under the second allowed for it.
    assert!(
        took <= BOUND + Duration::from_secs(1),
        "reconnected {took:?} after the return"
    );

    subject.shutdown().await.expect("clean shutdown");
    target.shutdown().await.expect("clean shutdown");
}

#[tokio::test]
async fn a_reconnect_toward_an_infrastructure_only_peer_is_refused_under_its_own_origin() {
    let subject = ProfileIdentity::generate();
    let infra = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id");
    let trust = TrustSources::new(
        PeerTrustPolicy::new(std::iter::empty()).expect("an empty allowlist"),
        InfrastructureSet::new([infra.clone()]).expect("one peer"),
    );
    let runtime = SwarmRuntime::start(&subject, SubstrateConfig::default(), trust).expect("starts");
    assert_eq!(
        runtime
            .learn(infra.clone(), ["/ip4/1.2.3.4/tcp/4001".to_owned()])
            .await
            .expect("delivered"),
        1,
        "the book keys an infrastructure-only peer: it is classified"
    );

    let refused = runtime.reconnect(infra.clone()).await.expect("delivered");
    assert!(
        matches!(refused, Err(DialRefusal::Policy(_))),
        "the gate refuses it -- a book entry exists, so this is not NoKnownAddress: {refused:?}"
    );
    assert_eq!(
        runtime
            .status(None)
            .await
            .expect("answered")
            .dial_gate
            .pending_dials,
        0,
        "and no dial is in flight toward it"
    );

    runtime.shutdown().await.expect("clean shutdown");
}

/// Repeated reconnects to an address that never answers hold ONE dial,
/// not one per ask (#137 review F1): the composition root asks every
/// round, and a dial per round filled the pending-dial ceiling and
/// settled one outage as one failure per round. A documentation-range
/// address the host routes and nothing answers keeps the dial pending
/// across the asks.
#[tokio::test]
async fn repeated_reconnects_to_an_unanswering_peer_hold_one_pending_dial() {
    let subject = ProfileIdentity::generate();
    let peer = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id");
    let runtime = SwarmRuntime::start(&subject, SubstrateConfig::default(), data_plane(&peer))
        .expect("starts");
    assert_eq!(
        runtime
            .learn(peer.clone(), ["/ip4/192.0.2.1/tcp/4001".to_owned()])
            .await
            .expect("delivered"),
        0,
        "the discovery door refuses a documentation-range address"
    );
    // The operator's door holds it, so the book has a route to dial.
    assert!(
        runtime
            .add_address(
                peer.clone(),
                "/ip4/192.0.2.1/tcp/4001".parse().expect("valid")
            )
            .await
            .expect("delivered")
    );

    for _ in 0..3 {
        runtime
            .reconnect(peer.clone())
            .await
            .expect("delivered")
            .expect("admitted or already in flight");
    }
    let gate = runtime.status(None).await.expect("answered").dial_gate;
    assert_eq!(
        (gate.pending_dials, gate.connection_slots),
        (1, 1),
        "three asks, one dial: {gate:?}"
    );

    runtime.shutdown().await.expect("clean shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_route_that_is_this_nodes_own_listen_address_is_refused_as_its_own_class() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, shared_id, apart_id) = (
        ProfileIdentity::generate(),
        ProfileIdentity::generate(),
        ProfileIdentity::generate(),
    );
    let subject_peer = subject_id.transport_identity().expect("peer id");
    let shared_peer = shared_id.transport_identity().expect("peer id");
    let apart_peer = apart_id.transport_identity().expect("peer id");
    let mut subject = SwarmRuntime::start(
        &subject_id,
        SubstrateConfig::default(),
        TrustSources::new(
            PeerTrustPolicy::new([shared_peer.clone(), apart_peer.clone()]).expect("two peers"),
            InfrastructureSet::default(),
        ),
    )
    .expect("starts");
    let shared = SwarmRuntime::start(
        &shared_id,
        SubstrateConfig::default(),
        data_plane(&subject_peer),
    )
    .expect("starts");
    let apart = SwarmRuntime::start(
        &apart_id,
        SubstrateConfig::default(),
        data_plane(&subject_peer),
    )
    .expect("starts");
    let own = subject
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("listens");
    // THE SAME PORT, as the two daemons of #276's flake held it.
    assert_eq!(
        shared.listen(own.clone()).await.expect("binds beside it"),
        own
    );
    let apart_addr = apart
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("listens");
    for (peer, address) in [(&shared_peer, &own), (&apart_peer, &apart_addr)] {
        assert_eq!(
            subject
                .learn(peer.clone(), [address.to_string()])
                .await
                .expect("delivered"),
            1
        );
    }

    // CONTROL: the same call toward a route of its own is admitted and
    // connects, so what follows is the route and not the call.
    subject
        .reconnect(apart_peer.clone())
        .await
        .expect("delivered")
        .expect("admitted");
    wait_connected(&mut subject, &apart_peer).await;

    let refused = subject
        .reconnect(shared_peer.clone())
        .await
        .expect("delivered")
        .expect_err("libp2p strips its own listen address and has none left");
    assert!(
        matches!(&refused, DialRefusal::Backend(b) if b.class() == DialFailureClass::OwnListenAddress),
        "{refused:?}"
    );
    assert_eq!(
        DialFailureClass::of_refusal(&refused).label(),
        "own_listen_address"
    );
    // The route left the book with it, so the next round has nothing to
    // dial -- which is what an operator reads next.
    assert_eq!(
        subject
            .reconnect(shared_peer.clone())
            .await
            .expect("delivered"),
        Err(DialRefusal::NoKnownAddress)
    );

    for runtime in [subject, shared, apart] {
        runtime.shutdown().await.expect("clean shutdown");
    }
}
