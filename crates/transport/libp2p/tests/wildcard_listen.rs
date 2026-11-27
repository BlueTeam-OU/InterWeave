// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A wildcard listen answers once its listener has reported every
//! interface the host holds, so ADR-0052 rule 3's own-listener boundary
//! already counts the host's private address when `listen` returns.
//!
//! As this was found (j237), `listen` answered at the listener's FIRST
//! address, its loopback one, and a private candidate learned as it
//! returned raced the LAN address's report: refused as
//! `private_without_private_listener` in about one start in six on an
//! idle host. A configured peer met that refusal at
//! `ComposedRuntime::start` and a send made as it returned was answered
//! `PeerUnknown`.
//!
//! The race is why each case runs many starts: the reverted answer
//! passes a single start more often than not. Multi-threaded, because on
//! one thread the Swarm task handles every ready event before the caller
//! resumes and the reverted answer passes too.

#![allow(clippy::expect_used, clippy::panic)]

use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_libp2p::{SubstrateConfig, SwarmRuntime};
use interweave_transport_runtime::TrustSources;
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};
use libp2p::Multiaddr;
use libp2p::multiaddr::Protocol;

/// Starts per case: the reverted answer lost the race in about one in
/// six, so thirty make its passing a run about one in 240.
const STARTS: usize = 30;

fn trusting(peer: &TransportIdentity) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new([peer.clone()]).expect("one peer"),
        InfrastructureSet::default(),
    )
}

fn peer_id() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

/// Start a runtime trusting `peer`, listen on `listen`, and report the
/// widening count and how many of `candidate` the book took, both read
/// as `listen` returned.
async fn listen_then_learn(
    listen: &str,
    peer: &TransportIdentity,
    candidate: &str,
) -> (Multiaddr, u64, usize) {
    let subject = ProfileIdentity::generate();
    let runtime =
        SwarmRuntime::start(&subject, SubstrateConfig::default(), trusting(peer)).expect("starts");
    let widenings = runtime.listener_widenings();
    let bound = runtime
        .listen(listen.parse().expect("valid"))
        .await
        .expect("listens");
    let widened = widenings.count();
    let learned = runtime
        .learn(peer.clone(), [candidate.to_owned()])
        .await
        .expect("the command reaches the task");
    runtime.shutdown().await.expect("clean shutdown");
    (bound, widened, learned)
}

fn assigned_port(address: &Multiaddr) -> Option<u16> {
    address.iter().find_map(|p| match p {
        Protocol::Tcp(port) => Some(port),
        _ => None,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_wildcard_listen_answers_once_its_lan_address_is_observed() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let peer = peer_id();
    // A port nothing listens on: the book's door judges the address and
    // nothing dials it.
    let candidate = format!("/ip4/{ip}/tcp/9");
    for start in 0..STARTS {
        let (bound, widened, learned) =
            listen_then_learn("/ip4/0.0.0.0/tcp/0", &peer, &candidate).await;
        assert!(
            assigned_port(&bound).is_some_and(|port| port != 0),
            "the answer is a bound address, not the wildcard asked for: {bound}"
        );
        // The LAN address widened rule 3's boundary before the answer,
        // whichever interface the kernel named first: a first private
        // observation widens too
        // (`a_private_listener_observed_first_widens_the_boundary`).
        assert!(widened >= 1, "start {start}: widened {widened} times");
        assert_eq!(
            learned, 1,
            "start {start}: a private candidate is admitted as `listen` returns"
        );
    }

    // THE CONTROL: a loopback listener alone is no private listener, so
    // the same candidate is refused and nothing widened -- the admission
    // above was the LAN address's.
    let (_, widened, learned) = listen_then_learn("/ip4/127.0.0.1/tcp/0", &peer, &candidate).await;
    assert_eq!(widened, 0, "loopback widens nothing");
    assert_eq!(learned, 0, "and rule 3 refuses the private candidate");
}

/// A listen on a specific IP is answered with that IP's bound address,
/// as before the wildcard hold, and rule 3 already counts it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_specific_ip_listen_answers_with_its_address() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let peer = peer_id();
    let (bound, widened, learned) = listen_then_learn(
        &format!("/ip4/{ip}/tcp/0"),
        &peer,
        &format!("/ip4/{ip}/tcp/9"),
    )
    .await;
    assert_eq!(bound.iter().next(), Some(Protocol::Ip4(ip)));
    assert!(assigned_port(&bound).is_some_and(|port| port != 0));
    // Not asserted: a specific-IP listen is answered inside the event
    // that reports its address, before the task records the listener, so
    // the count read as `listen` returns need not have moved (measured:
    // 0). The `learn` the task handles after that event is the reading.
    let _ = widened;
    assert_eq!(learned, 1);
}
