// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Address poisoning of a trusted `PeerId`, through the discovery door.
//!
//! threat-model.md's row: an attacker gets a wrong address associated
//! with a trusted `PeerId`, so the handshake fails or names someone else,
//! and the whole peer is suppressed. Stage 5 proved the quarantine for
//! an address the operator dialled by hand (`Dial`). Every discovery
//! provider (static entries, the cache, mDNS) reaches the address book
//! through one other door, `SwarmRuntime::learn`, and the dial that
//! follows is the book's (`dial_peer`), not the operator's. This is that
//! path. The poisoned candidate is an impostor answering with another
//! key, learned for the trusted peer. It must cost that address its
//! place and leave the peer dialable the moment a true address is known.
//!
//! On the host's private address, because ADR-0052 refuses a loopback
//! candidate from a peer.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_libp2p::{DialRefusal, SubstrateConfig, SwarmEvent, SwarmRuntime};
use interweave_transport_runtime::{DialDenial, TrustSources};
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};

const PATIENCE: Duration = Duration::from_secs(20);

fn trusting(peers: &[&TransportIdentity]) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new(peers.iter().map(|p| (*p).clone())).expect("a handful"),
        InfrastructureSet::default(),
    )
}

fn who() -> (ProfileIdentity, TransportIdentity) {
    let id = ProfileIdentity::generate();
    let peer = id.transport_identity().expect("peer id");
    (id, peer)
}

/// The next outcome for `peer`: connected, or a dial that failed with
/// its detail.
async fn outcome(runtime: &mut SwarmRuntime, peer: &TransportIdentity) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        match tokio::time::timeout_at(deadline, runtime.next_event())
            .await
            .expect("an outcome within the deadline")
        {
            Some(SwarmEvent::Connected { peer: got, .. }) if &got == peer => return Ok(()),
            Some(SwarmEvent::DialFailed {
                peer: Some(got),
                detail,
                ..
            }) if &got == peer => return Err(detail),
            Some(_) => {}
            None => panic!("the runtime stopped"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_poisoned_candidate_costs_its_address_and_never_the_trusted_peer() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, subject_peer) = who();
    let (target_id, target_peer) = who();

    let mut subject = SwarmRuntime::start(
        &subject_id,
        SubstrateConfig::default(),
        trusting(&[&target_peer]),
    )
    .expect("starts");
    // ADR-0052 rule 3: a private candidate is admitted only beside a
    // private listener of the same family.
    let _ = subject
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the subject listens on the private address");
    let target = SwarmRuntime::start(
        &target_id,
        SubstrateConfig::default(),
        trusting(&[&subject_peer]),
    )
    .expect("starts");
    let good = target
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the target listens");

    // The impostor: a real listener that answers, under another key.
    let (impostor_id, _) = who();
    let impostor = SwarmRuntime::start(
        &impostor_id,
        SubstrateConfig::default(),
        TrustSources::default(),
    )
    .expect("starts");
    let poisoned = impostor
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the impostor listens");

    // THE POISON, through the door every discovery provider uses, and
    // the book's own dial.
    assert_eq!(
        subject
            .learn(target_peer.clone(), [poisoned.to_string()])
            .await
            .expect("delivered"),
        1,
        "a lawful private candidate enters the book"
    );
    subject
        .dial_peer(target_peer.clone())
        .await
        .expect("delivered")
        .expect("the book's route is admitted: it has no history yet");
    let failed = outcome(&mut subject, &target_peer).await;
    assert!(
        matches!(&failed, Err(detail) if detail.contains("Unexpected peer ID")),
        "the impostor fails the IDENTITY check, not the transport, or nothing below is \
         about poisoning: {failed:?}"
    );

    // THE ADDRESS PAYS: it is refused from now on, whoever asks for it.
    assert_eq!(
        subject
            .dial(target_peer.clone(), poisoned)
            .await
            .expect("delivered"),
        Err(DialRefusal::Policy(DialDenial::AddressQuarantined)),
        "the poisoned candidate is quarantined"
    );

    // THE PEER DOES NOT: the moment a true address is learned, the book
    // reaches it. Peer-wide backoff would refuse this dial.
    subject
        .learn(target_peer.clone(), [good.to_string()])
        .await
        .expect("delivered");
    subject
        .dial_peer(target_peer.clone())
        .await
        .expect("delivered")
        .expect("the trusted peer is not suppressed by one poisoned address");
    assert_eq!(
        outcome(&mut subject, &target_peer).await,
        Ok(()),
        "reached through its true address"
    );

    subject.shutdown().await.expect("shuts down");
    target.shutdown().await.expect("shuts down");
    impostor.shutdown().await.expect("shuts down");
}
