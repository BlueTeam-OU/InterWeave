// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Two shipped daemons on a second network (plan section 21, decided
//! (7); SECURITY-MATRIX C-05, run R7's daemon and network side): A and B
//! each behind its own NAT, the relay on the core between them
//! (`interweave_test_support::e2e::netns`). A direct dial of either
//! daemon's listen address from the other FAILS, and the relayed path
//! then carries a direct message both ways.
//!
//! The controls, beside the property: each daemon's listen address takes
//! a connection from its own namespace, so a refused dial is the NAT's
//! and not a listener that is not there; and the relay takes one from
//! both sides, so the network between them is up.
//!
//! What this does NOT prove, by name: a NAT whose mapping depends on the
//! destination (nft's masquerade keeps the source port where it can); a
//! hole punch -- the route is asserted to BEGIN relayed, and what DCUtR
//! makes of these NATs afterwards is neither required nor refused; a
//! second host (Stage 19's drill); the bridge, which is
//! rust-services-dev's side of the run.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::net::SocketAddrV4;
use std::time::Duration;

use interweave_ipc_client::IpcSession;
use interweave_local_client_api::{
    DataSessionBinding as _, DataSessionPort as _, LocalSessionEvent, ROUTE_ESTABLISHED,
    SessionEvent,
};
use interweave_transport_api::{
    DirectDestination, MediaType, MessageId, Payload, PeerPath, TransportIdentity,
};

mod common;

use common::netns::{self, Net, NetRelay};
use common::{Home, PATIENCE, free_port, human, lease_request, relayed_example};

fn payload(text: &str) -> Payload {
    Payload::at_ceiling(
        Some(MediaType::parse("text/plain").expect("a media type")),
        text.as_bytes().to_vec(),
    )
    .expect("a payload")
}

/// How long a refused dial is given: the NAT drops it and answers
/// nothing, so the probe ends on this (asserted, beside the NAT's drop
/// counter).
const DIAL: Duration = Duration::from_secs(3);

fn tcp_port(address: &str) -> u16 {
    address
        .split('/')
        .skip_while(|p| *p != "tcp")
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| panic!("no TCP port in {address}"))
}

/// Send until the route exists, within `PATIENCE`.
async fn send_until_routed(
    from: &IpcSession,
    to: &TransportIdentity,
    id: u8,
    text: &str,
    logs: impl Fn() -> String,
) {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        let sent = from
            .send_direct(
                DirectDestination {
                    peer: to.clone(),
                    endpoint: Some(human()),
                },
                MessageId::from_bytes([id; 16]),
                payload(text),
            )
            .await;
        if sent.is_ok() {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no route to {}: {sent:?}\n{}",
            to.as_str(),
            logs()
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Read `session` until `text` from `source` arrives, keeping the path
/// each route began on; that path for `source`, when told.
async fn take(
    session: &IpcSession,
    source: &TransportIdentity,
    text: &str,
    begun: &mut Option<PeerPath>,
    logs: impl Fn() -> String,
) {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        let mut found = false;
        for event in session.events(64).await.expect("events") {
            match event {
                SessionEvent::Direct(m)
                    if &m.source_peer == source && m.payload.bytes() == text.as_bytes() =>
                {
                    found = true;
                }
                SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
                    peer,
                    current,
                    reason_class,
                    ..
                }) if &peer == source && reason_class == ROUTE_ESTABLISHED => {
                    begun.get_or_insert(current);
                }
                _ => {}
            }
        }
        if found {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "{text:?} from {} never arrived\n{}",
            source.as_str(),
            logs()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

const TEST: &str =
    "two_daemons_behind_their_own_nats_cannot_dial_each_other_and_talk_through_the_relay";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_daemons_behind_their_own_nats_cannot_dial_each_other_and_talk_through_the_relay() {
    if !netns::in_second_network(TEST) {
        return;
    }
    let (a, b) = (Home::new("human-desktop"), Home::new("human-desktop"));
    let (a_peer, b_peer) = (a.write_key(), b.write_key());
    let relay = NetRelay::start(&[&a_peer, &b_peer]);
    let relay_port = tcp_port(&relay.relay().address);
    let a_port = netns::in_net(Net::A, || free_port(netns::A));
    let b_port = netns::in_net(Net::B, || free_port(netns::B));
    let a_listen = format!("/ip4/{}/tcp/{a_port}", netns::A);
    let b_listen = format!("/ip4/{}/tcp/{b_port}", netns::B);
    b.write_config(&relayed_example(&[&a_peer], &b_listen, relay.relay(), None));
    a.write_config(&relayed_example(
        &[&b_peer],
        &a_listen,
        relay.relay(),
        Some(&relay.relay().circuit_to(&b_peer)),
    ));
    let mut b_daemon = b.start_in(Net::B, &[]);
    b_daemon.serving(&b).await;
    let pid = |p: &TransportIdentity| -> libp2p::PeerId { p.as_str().parse().expect("a peer id") };
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while !relay
        .relay()
        .seen()
        .await
        .reservations
        .contains(&pid(&b_peer))
    {
        assert!(
            tokio::time::Instant::now() < deadline,
            "B never reserved on the relay:\n{}",
            b_daemon.log()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let mut a_daemon = a.start_in(Net::A, &[]);
    a_daemon.serving(&a).await;
    let logs = || {
        format!(
            "A:\n{}\nB:\n{}\nthe relay: {:?}",
            a_daemon.log(),
            b_daemon.log(),
            relay.relay().seen_now()
        )
    };

    // The controls: each listener is there, and the relay is reached
    // from both sides. A daemon serves its IPC sockets before its
    // network listener has bound (measured: one run in two found it
    // not yet), so the listener is waited for; the dials refused below
    // are tried only once it is up.
    let at = |ip, port| SocketAddrV4::new(ip, port);
    for (net, own, name) in [
        (Net::A, at(netns::A, a_port), "A"),
        (Net::B, at(netns::B, b_port), "B"),
    ] {
        let deadline = tokio::time::Instant::now() + PATIENCE;
        while !netns::tcp_reaches(net, own, DIAL) {
            assert!(
                tokio::time::Instant::now() < deadline,
                "{name} never listened on {own}\n{}",
                logs()
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    for net in [Net::A, Net::B] {
        assert!(
            netns::tcp_reaches(net, at(netns::RELAY, relay_port), DIAL),
            "{net:?} reaches the relay"
        );
    }
    // The property: neither daemon's listen address is dialable from
    // the other side. The core carries each dial as far as the far NAT
    // (it routes both private ranges), so the refusal is that NAT's: it
    // answers nothing -- the dial ends on DIAL, not on an unreachable
    // network -- and its forward chain counts the drop.
    for (from, to, nat, what) in [
        (
            Net::A,
            at(netns::B, b_port),
            Net::NatB,
            "A dialled B's listen address directly",
        ),
        (
            Net::B,
            at(netns::A, a_port),
            Net::NatA,
            "B dialled A's listen address directly",
        ),
    ] {
        let before = netns::inward_drops(nat);
        let started = std::time::Instant::now();
        assert!(!netns::tcp_reaches(from, to, DIAL), "{what}\n{}", logs());
        assert!(
            started.elapsed() >= DIAL && netns::inward_drops(nat) > before,
            "{what}: refused in {:?}, not by {nat:?}'s drop ({} dropped before)",
            started.elapsed(),
            before
        );
    }
    // And NAT-B maps no port to B: B's port on NAT-B's own address is
    // NAT-B's to answer, which it refuses.
    assert!(
        !netns::tcp_reaches(Net::A, at(netns::NAT_B, b_port), DIAL),
        "NAT-B forwards B's port uninvited"
    );

    // A -> B, then B -> A, over the relay.
    let a_session = a.binding().open(lease_request()).await.expect("A leases");
    let b_session = b.binding().open(lease_request()).await.expect("B leases");
    let (mut a_begun, mut b_begun) = (None, None);
    send_until_routed(&a_session, &b_peer, 1, "a to b", logs).await;
    take(&b_session, &a_peer, "a to b", &mut b_begun, logs).await;
    send_until_routed(&b_session, &a_peer, 2, "b to a", logs).await;
    take(&a_session, &b_peer, "b to a", &mut a_begun, logs).await;
    let seen = relay.relay().seen().await;
    assert!(
        seen.circuits.contains(&(pid(&a_peer), pid(&b_peer))),
        "the relay carried A's circuit to B: {seen:?}"
    );
    assert_eq!(
        a_begun,
        Some(PeerPath::Relayed),
        "A of B: begun relayed\n{}",
        logs()
    );
    assert_eq!(
        b_begun,
        Some(PeerPath::Relayed),
        "B of A: begun relayed\n{}",
        logs()
    );

    drop((a_session, b_session));
    let a_stopped = a_daemon.terminate().await.success();
    let b_stopped = b_daemon.terminate().await.success();
    assert!(
        a_stopped && b_stopped,
        "A:\n{}\nB:\n{}",
        a_daemon.log(),
        b_daemon.log()
    );
}
