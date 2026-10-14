// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! AutoNAT dial-back SSRF, every target class over the wire.
//!
//! threat-model.md's row: an authorized AutoNAT client uses the probe
//! server as a scanner, naming a target the server would then dial.
//! `AUTONAT.md` §7 admits only a literal IP equal to the probing
//! connection's observed source, and only a globally routable one.
//! `probe_server.rs` unit-tests that judgement on every shape, beside a
//! public-source positive control. `autonat_server_wire.rs` drives one
//! shape over real sockets: loopback. This drives the rest, each from a
//! real client through the vendored crate to `ProbeServer`: private,
//! ULA, link-local, multicast, an unrelated public IP, a DNS name and a
//! stacked address. It also drives the one case that reaches the CLASS
//! rule with the source rule met: a client on the host's private
//! address asking to be dialled back at that address.
//!
//! For every shape, the subject's Swarm never dials, the client hears a
//! failure, and the wrapper refused the request by name: the vendored
//! client carries every one of these shapes to the server (measured).
//!
//! What one host cannot show: a candidate that is probeable. Nothing
//! here has a globally routable source, so the positive control is the
//! unit table's, and that the harness sees a dial-back when one is made
//! is `autonat_outcome_wire.rs`'s, against the bare crate.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use futures::StreamExt as _;
use interweave_transport_libp2p::probe_server::{
    ProbeBudgets, ProbeRefusal, ProbeServer, ProbeServerEvent,
};
use libp2p::autonat::v2::client::{self, Behaviour as Client, Config as ClientConfig};
use libp2p::autonat::v2::server::Behaviour as Server;
use libp2p::swarm::{FromSwarm, NetworkBehaviour, NewExternalAddrCandidate, SwarmEvent};
use libp2p::{Multiaddr, Swarm, identify, identity};

const PATIENCE: Duration = Duration::from_secs(20);

fn identify(k: &identity::Keypair) -> identify::Behaviour {
    identify::Behaviour::new(identify::Config::new(
        "/interweave-security-autonat/1".to_owned(),
        k.public(),
    ))
}

#[derive(NetworkBehaviour)]
struct RawClient {
    identify: identify::Behaviour,
    client: Client,
}

#[derive(NetworkBehaviour)]
struct Subject {
    identify: identify::Behaviour,
    server: ProbeServer,
}

fn swarm<B: NetworkBehaviour>(build: impl FnOnce(&identity::Keypair) -> B) -> Swarm<B> {
    libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("the same transport stack the substrate uses")
        .with_behaviour(|k| build(k))
        .expect("behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
        .build()
}

async fn bound<B: NetworkBehaviour>(swarm: &mut Swarm<B>, at: &str) -> Multiaddr
where
    B::ToSwarm: std::fmt::Debug,
{
    swarm
        .listen_on(at.parse().expect("a listen address"))
        .expect("listens");
    loop {
        if let SwarmEvent::NewListenAddr { address, .. } =
            tokio::time::timeout(PATIENCE, swarm.select_next_some())
                .await
                .expect("binds")
        {
            return address;
        }
    }
}

/// A raw client, listening on `ip` so its connection's observed source
/// is that address, connected to the subject.
async fn client_on(
    ip: &str,
    subject: &mut Swarm<Subject>,
    subject_addr: &Multiaddr,
) -> Swarm<RawClient> {
    let mut c = swarm(|k| RawClient {
        identify: identify(k),
        client: Client::new(
            rand::make_rng::<rand::rngs::StdRng>(),
            ClientConfig::default().with_probe_interval(Duration::from_millis(200)),
        ),
    });
    // Port reuse: the client dials from its listen address, so the
    // subject observes `ip` as the source.
    let _ = bound(&mut c, &format!("/ip4/{ip}/tcp/0")).await;
    let subject_id = *subject.local_peer_id();
    c.dial(
        subject_addr
            .clone()
            .with_p2p(subject_id)
            .expect("a peer address"),
    )
    .expect("dials");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        tokio::select! {
            event = c.select_next_some() => {
                if matches!(event, SwarmEvent::ConnectionEstablished { .. }) {
                    return c;
                }
            }
            _ = subject.select_next_some() => {}
            () = tokio::time::sleep_until(deadline) => panic!("not connected"),
        }
    }
}

/// What one target came to.
struct Probed {
    /// The client's outcome for THIS target, if the crate tested it.
    outcome: Option<Result<(), String>>,
    /// A dial-back reached the client.
    dial_back: bool,
    /// The subject's Swarm dialled anything.
    dialled: bool,
    /// The wrapper's refusals that name this target.
    refusals: Vec<ProbeRefusal>,
}

/// Offer `target` as the client's candidate and run until the client
/// reports the outcome for it, or `PATIENCE` passes. The client also
/// tests the address Identify observed for it, which is loopback or the
/// private source here; those outcomes and refusals are not this
/// target's and are skipped, though a SERVED probe of any address fails
/// the test.
async fn probe(
    c: &mut Swarm<RawClient>,
    subject: &mut Swarm<Subject>,
    target: &Multiaddr,
) -> Probed {
    c.behaviour_mut()
        .client
        .on_swarm_event(FromSwarm::NewExternalAddrCandidate(
            NewExternalAddrCandidate { addr: target },
        ));
    let mut seen = Probed {
        outcome: None,
        dial_back: false,
        dialled: false,
        refusals: Vec::new(),
    };
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        tokio::select! {
            event = c.select_next_some() => match event {
                SwarmEvent::Behaviour(RawClientEvent::Client(client::Event {
                    tested_addr,
                    result,
                    ..
                })) if tested_addr == *target => {
                    seen.outcome = Some(result.map_err(|e| e.to_string()));
                    return seen;
                }
                SwarmEvent::ConnectionEstablished { endpoint, .. } if endpoint.is_listener() => {
                    seen.dial_back = true;
                }
                _ => {}
            },
            event = subject.select_next_some() => match event {
                SwarmEvent::Behaviour(SubjectEvent::Server(ProbeServerEvent::Refused {
                    address,
                    reason,
                    ..
                })) => {
                    if address.as_ref() == Some(target) {
                        seen.refusals.push(reason);
                    }
                }
                SwarmEvent::Behaviour(SubjectEvent::Server(served)) => {
                    panic!("nothing may be served here: {served:?}");
                }
                SwarmEvent::Dialing { .. } => seen.dialled = true,
                _ => {}
            },
            // The crate may never test a shape it will not carry. That is
            // still no dial, and the row says which shapes reached the
            // wrapper.
            () = tokio::time::sleep_until(deadline) => return seen,
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_dial_back_target_class_opens_a_socket() {
    let private = interweave_test_support::net::require_private_interface_v4();
    let mut subject = swarm(|k| Subject {
        identify: identify(k),
        server: ProbeServer::new(Server::default(), ProbeBudgets::default()),
    });
    let subject_addr = bound(&mut subject, "/ip4/0.0.0.0/tcp/0").await;
    let subject_port = subject_addr
        .iter()
        .find_map(|p| match p {
            libp2p::multiaddr::Protocol::Tcp(port) => Some(port),
            _ => None,
        })
        .expect("a tcp port");
    let loopback_subject: Multiaddr = format!("/ip4/127.0.0.1/tcp/{subject_port}")
        .parse()
        .expect("valid");
    let private_subject: Multiaddr = format!("/ip4/{private}/tcp/{subject_port}")
        .parse()
        .expect("valid");

    // (client source, target offered, the refusal that names it).
    // Where the source is 127.0.0.1, every other literal fails source
    // equality first; the private row is the one where the source rule
    // is met and the class rule decides.
    let rows: Vec<(String, String, ProbeRefusal)> = vec![
        (
            "127.0.0.1".into(),
            "/ip4/10.0.0.9/tcp/4001".into(),
            ProbeRefusal::SourceMismatch,
        ),
        (
            "127.0.0.1".into(),
            "/ip6/fd00::1/tcp/4001".into(),
            ProbeRefusal::SourceMismatch,
        ),
        (
            "127.0.0.1".into(),
            "/ip6/fe80::1/tcp/4001".into(),
            ProbeRefusal::SourceMismatch,
        ),
        (
            "127.0.0.1".into(),
            "/ip4/224.0.0.1/tcp/4001".into(),
            ProbeRefusal::SourceMismatch,
        ),
        (
            "127.0.0.1".into(),
            "/ip4/8.8.8.8/tcp/4001".into(),
            ProbeRefusal::SourceMismatch,
        ),
        (
            "127.0.0.1".into(),
            "/dns4/example.invalid/tcp/4001".into(),
            ProbeRefusal::NotLiteralIp,
        ),
        (
            "127.0.0.1".into(),
            "/ip4/127.0.0.1/tcp/1/dns4/example.invalid/tcp/80".into(),
            ProbeRefusal::NotLiteralIp,
        ),
        (
            private.to_string(),
            format!("/ip4/{private}/tcp/9"),
            ProbeRefusal::NotGlobal,
        ),
    ];

    for (source, target, expected) in rows {
        let at = if source == "127.0.0.1" {
            &loopback_subject
        } else {
            &private_subject
        };
        let mut c = client_on(&source, &mut subject, at).await;
        let target: Multiaddr = target.parse().expect("a target multiaddr");
        let seen = probe(&mut c, &mut subject, &target).await;

        assert!(
            matches!(seen.outcome, Some(Err(_))),
            "{target}: the client heard a failure, not a confirmation and not silence: {:?}",
            seen.outcome
        );
        assert!(!seen.dial_back, "{target}: no dial-back reached the client");
        assert!(!seen.dialled, "{target}: the subject's Swarm never dialled");
        // Each target reached the wrapper and was refused once, by the
        // rule its shape meets first. Measured: the vendored client
        // carries every one of these shapes to the server.
        assert_eq!(
            seen.refusals,
            vec![expected],
            "{target} from {source} is refused by name"
        );
    }
}
