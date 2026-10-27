// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! ADR-0055 decision 5b, against real daemons: two hosts over a private
//! overlay the deployment provides. Each daemon runs in a network
//! namespace of its own -- its own interfaces, addresses, routing table,
//! port space and network-change detector -- and the two are joined by
//! one point-to-point link on `100.64.0.0/30`
//! (`interweave_test_support::e2e::netns`'s overlay), with no loopback
//! between them. Each listens on a wildcard and knows the other only as
//! a static-bootstrap entry at its overlay address, admitted under
//! ADR-0052 rule 9's operator door although the literal is carrier-grade;
//! no other discovery provider, no Kademlia and no connectivity role.
//! The third case is why 5b names the wildcard alone: a listener on the
//! overlay literal keeps its connection when the link goes.
//!
//! What this does NOT prove, by name (5b's known limits): one kernel, so
//! no real latency, loss, MTU or clock skew from a tunnel and no real
//! overlay in the path; no NAT; no second uid in the case itself
//! (decision 5's port rule is measured on the host,
//! `packaging/linux/evidence/2026-10-27-second-uid-port.md`, and still
//! owed as a CI case); a real second host, which needs a runner on a
//! second machine or the hosted edge itself as a test peer.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use interweave_automated_client_tests::{Client, ClientConfig};
use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort as _};
use interweave_test_support::e2e::netns::{self, Net};
use interweave_test_support::e2e::{Daemon, Home, PATIENCE, free_port, private_tempdir};
use interweave_transport_api::{MessageId, TransportIdentity};

const ENDPOINT: &str = "automation";
const CLIENT_KIND: &str = "automated-test-client";
const CHANNEL: &str = "example:automation";

/// How long the link's loss may take to close the connection at both
/// ends. The close is the network-change detector's, on the address
/// leaving (`CONNECTIVITY.md` §14); nothing else in the stack ends an
/// idle TCP connection whose path vanished in this time, so a pass here
/// is that detector's (the mutation of `network_change::closes` to
/// `false` fails it).
const CLOSE: Duration = Duration::from_secs(10);

/// How long the control watches two daemons that cannot learn each
/// other: a static entry is dialled at start, so a pair that could
/// connect does within it (measured: well under a second here).
const WATCH: Duration = Duration::from_secs(5);

/// What a host's daemon listens on.
#[derive(Clone, Copy)]
enum Listen {
    /// `0.0.0.0`, 5b's shape.
    Wildcard,
    /// The host's own overlay address alone, the shape 5b rules out.
    OverlayLiteral,
}

fn config() -> ClientConfig {
    ClientConfig::new(ENDPOINT, CLIENT_KIND, CHANNEL)
}

/// One host of the deployment: its home, `PeerId`, namespace, overlay
/// address and port, and its daemon.
struct Host {
    home: Home,
    peer: TransportIdentity,
    net: Net,
    ip: Ipv4Addr,
    port: u16,
    daemon: Option<Daemon>,
}

impl Host {
    fn mint(base: &std::path::Path, name: &str, net: Net, ip: Ipv4Addr) -> Self {
        let home = Home::within(base, name);
        let out = home.transportctl(&["identity", "init"], "");
        assert!(
            out.status.success(),
            "identity init: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let printed = String::from_utf8(out.stdout).expect("utf-8");
        let peer = TransportIdentity::parse(printed.trim_end()).expect("a PeerId printed");
        let port = netns::in_net(net, || free_port(ip));
        Self {
            home,
            peer,
            net,
            ip,
            port,
            daemon: None,
        }
    }

    /// The other host allowed, a listener on this one's port as `listen`
    /// says, and -- when `entry` -- the other's overlay address as the one
    /// static-bootstrap entry; without it, no provider at all.
    fn render(&self, other: &Self, entry: bool, listen: Listen) {
        let ip = match listen {
            Listen::Wildcard => Ipv4Addr::UNSPECIFIED,
            Listen::OverlayLiteral => self.ip,
        };
        let providers = if entry {
            format!(
                "[ {{ type: static-bootstrap, enabled: true, priority: 5, config: {{ peers: \
                 [\"/ip4/{}/tcp/{}/p2p/{}\"] }} }} ]",
                other.ip,
                other.port,
                other.peer.as_str()
            )
        } else {
            "[]".to_owned()
        };
        self.home.write_config(&format!(
            "schema_version: 2
profile: {{ name: {name} }}
trust: {{ policy: static-allowlist, allowed_peers: [\"{other}\"] }}
endpoints:
  entries:
    - {{ id: {ENDPOINT}, enabled: true, advertise: false, allowed_client_kinds: [{CLIENT_KIND}] }}
channels: {{ desired: [] }}
discovery: {{ providers: {providers} }}
transport: {{ listen: {{ addresses: [\"/ip4/{ip}/tcp/{port}\"] }} }}
observability: {{ log_level: debug }}
",
            name = self.home.paths.profile(),
            other = other.peer.as_str(),
            port = self.port,
        ));
    }

    async fn start(&mut self) {
        let mut daemon = self.home.start_in(self.net, &[]);
        daemon.serving(&self.home).await;
        self.daemon = Some(daemon);
    }

    fn log(&self) -> String {
        self.daemon.as_ref().map(Daemon::log).unwrap_or_default()
    }

    /// Whether the dial gate holds a connection to `peer` open, read
    /// through this daemon's admin port.
    async fn connected_to(&self, peer: &TransportIdentity) -> bool {
        let admin = self
            .home
            .binding()
            .admin([AdminCapability::Status].into())
            .await
            .expect("an admin port");
        admin
            .peers()
            .await
            .expect("admin.peers.list")
            .into_iter()
            .any(|v| &v.peer == peer && v.connected)
    }

    async fn client(&self) -> Client {
        let mut client = Client::new(config(), self.home.binding());
        client.connect(PATIENCE).await.expect("the client leases");
        client
    }

    async fn stop(&mut self) {
        let mut daemon = self.daemon.take().expect("running");
        assert!(daemon.terminate().await.success(), "{}", daemon.log());
    }
}

/// A and B minted under one scratch tree, each rendered against the
/// other, and started.
async fn hosts(entry: bool, listen: Listen) -> (tempfile::TempDir, Host, Host) {
    let base = private_tempdir();
    let mut a = Host::mint(base.path(), "host-a", Net::A, netns::OVERLAY_A);
    let mut b = Host::mint(base.path(), "host-b", Net::B, netns::OVERLAY_B);
    a.render(&b, entry, listen);
    b.render(&a, entry, listen);
    a.start().await;
    b.start().await;
    (base, a, b)
}

fn id(n: u8) -> MessageId {
    MessageId::from_bytes([n; 16])
}

/// Wait until `a` and `b` agree that they are `connected` (or not),
/// within `patience`.
async fn until_both(a: &Host, b: &Host, connected: bool, patience: Duration, what: &str) {
    let deadline = tokio::time::Instant::now() + patience;
    while a.connected_to(&b.peer).await != connected || b.connected_to(&a.peer).await != connected {
        assert!(
            tokio::time::Instant::now() < deadline,
            "{what} within {patience:?}\nA:\n{}\nB:\n{}",
            a.log(),
            b.log()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// A direct message from `from`'s client lands at `to`'s, from `from`'s
/// authenticated `PeerId`.
async fn lands(from: &mut Client, to: &mut Client, hosts: (&Host, &Host), n: u8, text: &str) {
    let (src, dst) = hosts;
    let sent = from.send(&dst.peer, id(n), text, PATIENCE).await;
    assert!(
        sent.outcome.is_ok(),
        "{text:?} accepted after {:?}: {:?}\nfrom:\n{}\nto:\n{}",
        sent.retried,
        sent.outcome,
        src.log(),
        dst.log()
    );
    assert!(
        to.pump_until(PATIENCE, |r| r
            .inbound
            .iter()
            .any(|m| !m.broadcast && m.message_id == id(n)))
            .await,
        "{text:?} never reached the other client"
    );
    let got = to
        .received
        .inbound
        .iter()
        .find(|m| m.message_id == id(n))
        .expect("held");
    assert_eq!(
        (&got.source, got.bytes.as_slice()),
        (&src.peer, text.as_bytes())
    );
}

const OVER_THE_OVERLAY: &str = "two_hosts_talk_over_the_overlay_and_redial_when_its_link_returns";

/// A direct message each way and a broadcast over the static entries;
/// then the link deleted, both ends closing the connection on the
/// address leaving, and the link made again, both redialled and a
/// direct message landing over the new connection.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_hosts_talk_over_the_overlay_and_redial_when_its_link_returns() {
    if !netns::in_overlay(OVER_THE_OVERLAY) {
        return;
    }
    let (_base, mut a, mut b) = hosts(true, Listen::Wildcard).await;
    let mut from_a = a.client().await;
    let mut from_b = b.client().await;

    lands(&mut from_a, &mut from_b, (&a, &b), 1, "a to b").await;
    lands(&mut from_b, &mut from_a, (&b, &a), 2, "b to a").await;

    // Published until the mesh has formed: before it, a publish is
    // accepted locally and reaches nobody.
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let mut n = 10_u8;
    loop {
        n += 1;
        from_a
            .publish(id(n), "all")
            .await
            .expect("accepted locally");
        if from_b
            .pump_until(Duration::from_millis(500), |r| {
                r.inbound.iter().any(|m| m.broadcast)
            })
            .await
        {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no broadcast arrived\nA:\n{}\nB:\n{}",
            a.log(),
            b.log()
        );
    }
    let got: Vec<_> = from_b
        .received
        .inbound
        .iter()
        .filter(|m| m.broadcast)
        .collect();
    assert!(
        got.iter().all(|m| m.source == a.peer && m.bytes == b"all"),
        "{got:?}"
    );
    until_both(&a, &b, true, PATIENCE, "connected over the overlay").await;

    netns::overlay_down();
    until_both(&a, &b, false, CLOSE, "both closed on the link's loss").await;

    netns::overlay_up();
    until_both(
        &a,
        &b,
        true,
        PATIENCE,
        "both redialled once the link returned",
    )
    .await;
    lands(&mut from_a, &mut from_b, (&a, &b), 100, "a to b again").await;

    drop((from_a, from_b));
    a.stop().await;
    b.stop().await;
}

const WITHOUT_THE_ENTRY: &str = "without_the_static_entry_the_two_hosts_never_connect";

/// The control: the same two hosts, the same trust and the same link,
/// without the static entry, never connect -- so the connection above is
/// the entry's, and the overlay's address is learned from nowhere else.
/// Beside it, the link carries TCP to each listener, so the absence is
/// not a link or a listener that is not there.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_the_static_entry_the_two_hosts_never_connect() {
    if !netns::in_overlay(WITHOUT_THE_ENTRY) {
        return;
    }
    let (_base, mut a, mut b) = hosts(false, Listen::Wildcard).await;
    for (from, to) in [(&a, &b), (&b, &a)] {
        let at = SocketAddrV4::new(to.ip, to.port);
        let deadline = tokio::time::Instant::now() + PATIENCE;
        while !netns::tcp_reaches(from.net, at, Duration::from_secs(1)) {
            assert!(
                tokio::time::Instant::now() < deadline,
                "{at} never took a connection over the link\n{}",
                to.log()
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    let from_a = a.client().await;
    let deadline = tokio::time::Instant::now() + WATCH;
    while tokio::time::Instant::now() < deadline {
        assert!(
            !a.connected_to(&b.peer).await && !b.connected_to(&a.peer).await,
            "connected without an entry\nA:\n{}\nB:\n{}",
            a.log(),
            b.log()
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let refused = from_a.send_once(&b.peer, id(1), "nowhere").await;
    assert!(refused.is_err(), "a send with no route: {refused:?}");

    drop(from_a);
    a.stop().await;
    b.stop().await;
}

const ON_THE_LITERAL: &str =
    "a_listener_on_the_overlay_literal_keeps_its_connection_past_the_close_window";

/// Why 5b names the wildcard alone: the same two hosts, each listening on
/// its overlay literal instead, keep the connection when the link is
/// deleted, past the window the wildcard case closes within. A listener
/// bound to one address reports no change when that address leaves, and
/// a daemon has no platform view, so the detector never sees the loss.
/// The wildcard case above is the positive control. This case failing --
/// the connection closed in the window -- means the stack now sees a
/// literal listener's loss, and 5b's reason is gone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_listener_on_the_overlay_literal_keeps_its_connection_past_the_close_window() {
    if !netns::in_overlay(ON_THE_LITERAL) {
        return;
    }
    let (_base, mut a, mut b) = hosts(true, Listen::OverlayLiteral).await;
    until_both(&a, &b, true, PATIENCE, "connected over the overlay").await;

    netns::overlay_down();
    tokio::time::sleep(CLOSE).await;
    assert!(
        a.connected_to(&b.peer).await && b.connected_to(&a.peer).await,
        "a literal listener's connection closed within {CLOSE:?} of the link's loss\nA:\n{}\nB:\n{}",
        a.log(),
        b.log()
    );

    netns::overlay_up();
    a.stop().await;
    b.stop().await;
}
