// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The platform view's DNS facts on a running Swarm (`CONNECTIVITY.md`
//! §14, A 2026-10-14), between real runtimes over real sockets, with a
//! nameserver on loopback that answers one name and counts every query.
//!
//! In-crate because a view names nameservers by IP only, so they are
//! asked on port 53, which a test cannot bind: `start_with_seams` takes
//! the responder's port instead.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_runtime::TrustSources;
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};

use super::{
    NetworkView, PRIVATE_DNS_STRICT, ResolverSource, SubstrateConfig, SwarmEvent, SwarmRuntime,
};

const PATIENCE: Duration = Duration::from_secs(20);

/// The one name the responder answers, with 127.0.0.1.
const NAME: &str = "target.interweave.test";

/// A nameserver on 127.0.0.1 that answers `A` for [`NAME`] and
/// `NXDOMAIN` for anything else, counting every query it receives.
async fn responder() -> (u16, Arc<AtomicUsize>) {
    let socket = tokio::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("bind");
    let port = socket.local_addr().expect("addr").port();
    let queries = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&queries);
    tokio::spawn(async move {
        let mut buf = [0u8; 512];
        loop {
            let Ok((n, from)) = socket.recv_from(&mut buf).await else {
                return;
            };
            counted.fetch_add(1, Ordering::SeqCst);
            if let Some(answer) = answer(&buf[..n]) {
                let _ = socket.send_to(&answer, from).await;
            }
        }
    });
    (port, queries)
}

/// The response to one query: the question echoed, and an `A` record
/// pointing at it when it asks `A` for [`NAME`].
fn answer(query: &[u8]) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }
    let mut at = 12;
    let mut labels = Vec::new();
    loop {
        let len = usize::from(*query.get(at)?);
        at += 1;
        if len == 0 {
            break;
        }
        labels.push(String::from_utf8_lossy(query.get(at..at + len)?).to_lowercase());
        at += len;
    }
    let qtype = u16::from_be_bytes([*query.get(at)?, *query.get(at + 1)?]);
    let question_end = at + 4;
    let question = query.get(12..question_end)?;
    let hit = labels.join(".") == NAME && qtype == 1;
    let mut out = Vec::new();
    out.extend_from_slice(&query[..2]);
    out.extend_from_slice(if hit { &[0x81, 0x80] } else { &[0x81, 0x83] });
    out.extend_from_slice(&[0, 1, 0, u8::from(hit), 0, 0, 0, 0]);
    out.extend_from_slice(question);
    if hit {
        out.extend_from_slice(&[0xc0, 0x0c, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 127, 0, 0, 1]);
    }
    Some(out)
}

fn trusting(peers: &[&TransportIdentity]) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new(peers.iter().map(|p| (*p).clone())).expect("a handful"),
        InfrastructureSet::default(),
    )
}

async fn wait_for<F>(runtime: &mut SwarmRuntime, what: &str, mut predicate: F) -> SwarmEvent
where
    F: FnMut(&SwarmEvent) -> bool,
{
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, runtime.next_event()).await {
            Err(elapsed) => panic!("timed out waiting for {what} ({elapsed})"),
            Ok(None) => panic!("the runtime stopped while waiting for {what}"),
            Ok(Some(event)) if predicate(&event) => return event,
            Ok(Some(_)) => {}
        }
    }
}

/// Every event that arrives within `window`.
async fn quiet_for(runtime: &mut SwarmRuntime, window: Duration) -> Vec<SwarmEvent> {
    let mut seen = Vec::new();
    while let Ok(Some(event)) = tokio::time::timeout(window, runtime.next_event()).await {
        seen.push(event);
    }
    seen
}

fn named(port: u16, peer: &TransportIdentity) -> libp2p::Multiaddr {
    format!("/dns4/{NAME}/tcp/{port}/p2p/{}", peer.as_str())
        .parse()
        .expect("a dns4 address")
}

/// The dial of `peer` at `address` fails, and the reason is returned.
async fn dial_fails(
    runtime: &mut SwarmRuntime,
    peer: &TransportIdentity,
    address: libp2p::Multiaddr,
) -> String {
    runtime
        .dial(peer.clone(), address)
        .await
        .expect("command delivered")
        .expect("the gate admits the dial: this is about the resolver");
    match wait_for(
        runtime,
        "the dial to fail",
        |e| matches!(e, SwarmEvent::DialFailed { peer: Some(who), .. } if who == peer),
    )
    .await
    {
        SwarmEvent::DialFailed { detail, .. } => detail,
        other => panic!("unexpected {other:?}"),
    }
}

/// THE LIVE SWAP, end to end. A runtime started with no resolver (the
/// Android start, #277) cannot dial a name; a platform view naming a
/// nameserver installs a resolver on the RUNNING Swarm, and the same
/// kind of dial then connects; a view in strict Private DNS mode replaces
/// it with one that asks nobody -- the responder counts no new query, so
/// no plain DNS left the process -- and says so. The same view again
/// builds nothing and reports nothing, and no view here is a network
/// change, because only the DNS facts moved.
///
/// Each failing dial names a peer of its own, because a failed dial puts
/// its peer in the gate's backoff, which would refuse the next dial before
/// it reached the resolver.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_platform_view_installs_a_resolver_on_a_running_swarm() {
    let (dns_port, queries) = responder().await;
    let target_id = ProfileIdentity::generate();
    let target = target_id.transport_identity().expect("peer id");
    let before = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id");
    let strict = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id");
    let dialer_id = ProfileIdentity::generate();
    let dialer_peer = dialer_id.transport_identity().expect("peer id");

    let listener = SwarmRuntime::start(
        &target_id,
        SubstrateConfig::default(),
        trusting(&[&dialer_peer]),
    )
    .expect("the target starts");
    let bound = listener
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("loopback"))
        .await
        .expect("listen");
    let port = bound
        .iter()
        .find_map(|p| match p {
            libp2p::multiaddr::Protocol::Tcp(port) => Some(port),
            _ => None,
        })
        .expect("a tcp port");

    let mut dialer = SwarmRuntime::start_with_seams::<&str>(
        &dialer_id,
        SubstrateConfig::default(),
        trusting(&[&target, &before, &strict]),
        Err("no host resolver configuration is read here"),
        dns_port,
    )
    .expect("a node with no resolver starts");
    wait_for(&mut dialer, "the start's ResolverUnavailable", |e| {
        matches!(e, SwarmEvent::ResolverUnavailable { .. })
    })
    .await;
    assert_eq!(
        dialer.status(None).await.expect("status").resolver,
        ResolverSource::None
    );

    // THE CONTROL: with no resolver the name does not resolve, and the
    // responder is asked nothing.
    dial_fails(&mut dialer, &before, named(port, &before)).await;
    assert_eq!(queries.load(Ordering::SeqCst), 0, "no resolver asks no one");

    // THE SWAP: a view naming the responder, its address set unchanged
    // between views so only the DNS facts move.
    let addresses: Vec<IpAddr> = vec![Ipv4Addr::LOCALHOST.into()];
    let plain = NetworkView::new(addresses.clone())
        .with_dns_servers(vec![Ipv4Addr::LOCALHOST.into(), Ipv4Addr::LOCALHOST.into()]);
    dialer.network_changed(plain.clone());
    let configured = wait_for(&mut dialer, "the resolver to be configured", |e| {
        matches!(e, SwarmEvent::ResolverConfigured { .. })
    })
    .await;
    assert_eq!(
        configured,
        SwarmEvent::ResolverConfigured { nameservers: 1 },
        "the duplicate nameserver is one"
    );
    assert_eq!(
        dialer.status(None).await.expect("status").resolver,
        ResolverSource::Platform
    );
    dialer
        .dial(target.clone(), named(port, &target))
        .await
        .expect("command delivered")
        .expect("admitted");
    wait_for(
        &mut dialer,
        "the named target to connect",
        |e| matches!(e, SwarmEvent::Connected { peer, .. } if *peer == target),
    )
    .await;
    let asked = queries.load(Ordering::SeqCst);
    assert!(asked > 0, "the name was resolved through the responder");

    // The same facts again build nothing and say nothing; nor is either
    // view a network change.
    dialer.network_changed(plain);
    let quiet = quiet_for(&mut dialer, Duration::from_millis(500)).await;
    assert!(
        !quiet.iter().any(|e| matches!(
            e,
            SwarmEvent::ResolverConfigured { .. }
                | SwarmEvent::ResolverUnavailable { .. }
                | SwarmEvent::NetworkChanged { .. }
        )),
        "an unchanged view reports nothing: {quiet:?}"
    );

    // STRICT PRIVATE DNS: the same nameserver named, and refused.
    dialer.network_changed(
        NetworkView::new(addresses)
            .with_dns_servers(vec![Ipv4Addr::LOCALHOST.into()])
            .with_private_dns(Some("dns.interweave.test".to_owned())),
    );
    let refused = wait_for(&mut dialer, "strict mode to be reported", |e| {
        matches!(
            e,
            SwarmEvent::ResolverUnavailable { .. } | SwarmEvent::NetworkChanged { .. }
        )
    })
    .await;
    assert_eq!(
        refused,
        SwarmEvent::ResolverUnavailable {
            detail: PRIVATE_DNS_STRICT.to_owned()
        }
    );
    assert_eq!(
        dialer.status(None).await.expect("status").resolver,
        ResolverSource::PrivateDnsStrict
    );
    dial_fails(&mut dialer, &strict, named(port, &strict)).await;
    assert_eq!(
        queries.load(Ordering::SeqCst),
        asked,
        "strict mode sends no plain DNS query"
    );

    dialer.shutdown().await.expect("clean shutdown");
    listener.shutdown().await.expect("clean shutdown");
}
