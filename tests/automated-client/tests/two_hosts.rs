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
use interweave_transport_api::{EndpointId, MessageId, TransportError, TransportIdentity};

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
    hosts_with(entry, entry, listen).await
}

/// The same, with each host's static entry for the other chosen apart:
/// `a_entry` for A's, `b_entry` for B's.
async fn hosts_with(
    a_entry: bool,
    b_entry: bool,
    listen: Listen,
) -> (tempfile::TempDir, Host, Host) {
    let base = private_tempdir();
    let mut a = Host::mint(base.path(), "host-a", Net::A, netns::OVERLAY_A);
    let mut b = Host::mint(base.path(), "host-b", Net::B, netns::OVERLAY_B);
    a.render(&b, a_entry, listen);
    b.render(&a, b_entry, listen);
    // B FIRST: A dials B as it starts, and a dial into a host not yet
    // listening puts B under the 30 s peer backoff, the length of a
    // test's patience -- A's entry alone (`hosts_with(true, false, ..)`)
    // then connects nothing in time. With both entries, B's early dial
    // backs off instead and A's connects.
    b.start().await;
    a.start().await;
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

const PAST_THE_IDLE_TIMEOUT: &str =
    "an_idle_connection_the_far_host_cannot_dial_back_outlives_the_idle_timeout";

/// Past the daemon's idle timeout, sixty seconds
/// (`SubstrateConfig::idle_timeout`), with margin.
const IDLE_PAST: Duration = Duration::from_secs(75);

/// The fleet's shape: only A holds a static entry, so B knows no
/// address for A and reaches it only over the connection A dialled.
/// Idle -- no client and no channel on either host -- for longer than
/// the idle timeout, the connection is the SAME one throughout: no
/// close logged at either end, and both still connected. B's direct
/// message then lands at A over it.
///
/// This pins the property, not the mechanism. Between two daemons the
/// pinned gossipsub holds every connection open by a stream of its own,
/// so this case would pass with the data-plane hold removed; the hold's
/// own proof, with the control that closes at the idle timeout, is
/// `crates/transport/libp2p/tests/held_data_plane.rs`, against a peer
/// that speaks no gossipsub. What this case adds is the guarantee the
/// fleet relies on, end to end, whichever of the two holds it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_idle_connection_the_far_host_cannot_dial_back_outlives_the_idle_timeout() {
    if !netns::in_overlay(PAST_THE_IDLE_TIMEOUT) {
        return;
    }
    let (_base, mut a, mut b) = hosts_with(true, false, Listen::Wildcard).await;
    until_both(&a, &b, true, PATIENCE, "connected by A's entry").await;

    // IDLE, watched every quarter second: a close that A redialled
    // within a round would still leave its line in a log.
    let deadline = tokio::time::Instant::now() + IDLE_PAST;
    while tokio::time::Instant::now() < deadline {
        assert!(
            a.connected_to(&b.peer).await && b.connected_to(&a.peer).await,
            "the idle connection dropped\nA:\n{}\nB:\n{}",
            a.log(),
            b.log()
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    for (name, host) in [("A", &a), ("B", &b)] {
        assert!(
            !host.log().contains("peer disconnected"),
            "{name} logged a close while idle:\n{}",
            host.log()
        );
    }

    // B, which cannot dial A, sends over what A dialled.
    let mut at_a = a.client().await;
    let mut from_b = b.client().await;
    lands(
        &mut from_b,
        &mut at_a,
        (&b, &a),
        1,
        "b to a, past the idle timeout",
    )
    .await;

    drop((at_a, from_b));
    a.stop().await;
    b.stop().await;
}

/// What [`after_a_close`] saw at B, the side with no entry for A.
#[derive(Debug)]
struct AfterAClose {
    /// B's `dial failed` lines in the watch after the close (no send yet)
    /// and in the watch after the link returned.
    dials: [usize; 2],
    /// What B's send to A answered while the link was down, and how long
    /// the answer took; `None` when nothing was sent.
    sent: Option<(Result<EndpointId, TransportError>, Duration)>,
    /// Whether B's gate scheduled a retry for A while the link was down.
    retry_scheduled: bool,
    /// What a send from B answered after the link was lost a second
    /// time, once both watches were read: `PeerUnknown` exactly when B's
    /// book held no address for A, which nothing can add to while the
    /// pair is apart.
    probe: Result<EndpointId, TransportError>,
    /// Whether a dial B's reconnect scheduler made failed in the watch
    /// after the link returned.
    scheduler_dialled: bool,
    /// B's log, for a failing assertion.
    log: String,
}

/// The fleet's shape again (only A holds an entry), closed by the link's
/// loss; `send` says whether B's client sends to A while the link is
/// down. When `learnable`, each host also holds a private address on its
/// loopback (`netns::add_unrouted_address`): A's is advertised by its
/// Identify and admitted into B's address book, since B then holds a
/// private listener of the family (ADR-0052 rule 3), and B has no route
/// to it -- an address of A on a network B is not on, as a host's LAN
/// address is to a host at another site. The overlay's own address is
/// never learned: Identify withholds `100.64.0.0/10` and rule 2 refuses
/// it from a peer, so without `learnable` B's book for A stays empty.
async fn after_a_close(learnable: bool, send: bool) -> AfterAClose {
    if learnable {
        netns::add_unrouted_address(Net::A, "10.99.0.1/24");
        netns::add_unrouted_address(Net::B, "10.88.0.2/24");
    }
    let (_base, mut a, mut b) = hosts_with(true, false, Listen::Wildcard).await;
    until_both(&a, &b, true, PATIENCE, "connected by A's entry").await;
    let from_b = b.client().await;
    // Identify runs at establishment at both ends; a second is margin.
    tokio::time::sleep(Duration::from_secs(1)).await;
    let dials = |b: &Host| b.log().matches("dial failed").count();
    let connected = dials(&b);

    netns::overlay_down();
    until_both(&a, &b, false, CLOSE, "both closed on the link's loss").await;
    tokio::time::sleep(WATCH).await;
    let closed = dials(&b);

    let answered = if send {
        let started = tokio::time::Instant::now();
        let answer = from_b.send_once(&a.peer, id(1), "after the close").await;
        Some((answer, started.elapsed()))
    } else {
        None
    };
    // The send's failed dial is logged with its reply already sent, by
    // another path than the reply's: waited for, then counted, so the
    // send's own line is never read as one the link's return started.
    // A send answered at once dialled nothing, and nothing is awaited.
    if let Some((Err(TransportError::PeerUnreachable), _)) = answered {
        until_logged(&b, "retry scheduled").await;
    }
    let before_up = dials(&b);
    let retry_scheduled = b.log().contains("retry scheduled");

    // A redials by its entry once the link returns; B's dials in the
    // watch after that are what the link's return started at B.
    netns::overlay_up();
    until_both(&a, &b, true, PATIENCE, "A redialled once the link returned").await;
    // The watch, ended early by the line a scheduler dial's failure
    // writes (a failed dial's own `origin` is the one that dialled).
    let deadline = tokio::time::Instant::now() + WATCH;
    while tokio::time::Instant::now() < deadline && !b.log().contains(SCHEDULER_DIAL) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let returned = dials(&b) - before_up;
    let scheduler_dialled = b.log().contains(SCHEDULER_DIAL);

    netns::overlay_down();
    until_both(&a, &b, false, CLOSE, "both closed on the second loss").await;
    let probe = from_b
        .send_once(&a.peer, id(2), "after the second close")
        .await;
    netns::overlay_up();
    let log = b.log();

    drop(from_b);
    a.stop().await;
    b.stop().await;
    AfterAClose {
        dials: [closed - connected, returned],
        sent: answered,
        retry_scheduled,
        probe,
        scheduler_dialled,
        log,
    }
}

/// The line B's gate writes when a dial the reconnect scheduler made
/// fails: the retry is rescheduled under the failed dial's own origin,
/// and a send's dial is `manual`.
const SCHEDULER_DIAL: &str = "origin=\"connection_manager\"";

/// Wait, within the patience, until `host`'s log holds `line`.
async fn until_logged(host: &Host, line: &str) {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while !host.log().contains(line) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "{line:?} never logged\n{}",
            host.log()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// How soon "at once" is: a send with nothing to dial is answered by the
/// runtime's own book, with no socket opened -- milliseconds measured;
/// the bound is load margin, far under the send's dial horizon (10 s).
const AT_ONCE: Duration = Duration::from_secs(2);

const NO_ADDRESS_AFTER_A_CLOSE: &str =
    "the_side_with_no_address_never_dials_and_its_send_is_answered_peer_unknown_at_once";

/// `CONNECTIVITY.md` §14's sentence on the hold, measured where it holds:
/// B never learned an address for A, so after the close it dials nothing
/// -- neither in the watch after it nor when the link returns -- and its
/// send is answered `PeerUnknown` at once. The control for the two cases
/// below: the same hosts and close, with no address B could learn.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_side_with_no_address_never_dials_and_its_send_is_answered_peer_unknown_at_once() {
    if !netns::in_overlay(NO_ADDRESS_AFTER_A_CLOSE) {
        return;
    }
    let seen = after_a_close(false, true).await;
    let (answer, took) = seen.sent.as_ref().expect("sent");
    assert!(
        matches!(answer, Err(TransportError::PeerUnknown)) && *took < AT_ONCE,
        "{seen:?}"
    );
    assert_eq!(seen.dials, [0, 0], "{}", seen.log);
    assert!(!seen.retry_scheduled, "{}", seen.log);
    assert!(
        matches!(seen.probe, Err(TransportError::PeerUnknown)),
        "{seen:?}"
    );
}

const LEARNED_NO_SEND: &str = "the_side_that_learned_an_address_dials_nothing_until_it_sends";

/// B learned A's private address from A's Identify, and sends nothing:
/// a close schedules no retry (`record_connection_closed` releases the
/// slot and nothing else), so B dials nothing after the close, nor when
/// the link returns -- the network addition lifts a held-off peer's
/// backoff, and B holds none for A. That B held the address all along
/// is the probe's: a send after a second loss, once both watches are
/// read, is answered `PeerUnreachable`, which an empty book answers
/// `PeerUnknown` (the first case). The learning is read after the
/// watches rather than before them, since a send would start the
/// retries the watches look for; what B learned at the first connection
/// it learned again at the second, from the same Identify. The case
/// below is the positive control that the dial count is live in this
/// shape.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_side_that_learned_an_address_dials_nothing_until_it_sends() {
    if !netns::in_overlay(LEARNED_NO_SEND) {
        return;
    }
    let seen = after_a_close(true, false).await;
    assert_eq!(seen.dials, [0, 0], "{}", seen.log);
    assert!(!seen.retry_scheduled, "{}", seen.log);
    assert!(
        matches!(seen.probe, Err(TransportError::PeerUnreachable)),
        "B never learned A's address\n{seen:?}"
    );
}

const LEARNED_SEND: &str = "the_side_that_learned_an_address_dials_it_on_a_send_and_retries";

/// B learned A's private address, and its client sends to A after the
/// close: the send is not answered `PeerUnknown` -- B's book holds an
/// address, so the runtime dials it once (`DIRECT.md`'s usable-candidate
/// case) and answers `PeerUnreachable` when the dial fails. That failure
/// schedules a retry like any dial's (30 s, doubling to five minutes),
/// so from the send on B redials A from its book: here the link's return
/// lifts the backoff and the scheduler dials at once. An unrouted
/// address fails in milliseconds; one the network drops silently holds
/// the send until the dial fails or the send's dial horizon (10 s)
/// passes, which this case does not measure.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_side_that_learned_an_address_dials_it_on_a_send_and_retries() {
    if !netns::in_overlay(LEARNED_SEND) {
        return;
    }
    let seen = after_a_close(true, true).await;
    let (answer, _) = seen.sent.as_ref().expect("sent");
    assert!(
        matches!(answer, Err(TransportError::PeerUnreachable)),
        "{seen:?}"
    );
    assert_eq!(seen.dials[0], 0, "no dial before the send\n{}", seen.log);
    assert!(seen.retry_scheduled, "{}", seen.log);
    // The scheduler's own dial, told apart from the send's by the origin
    // its failure is rescheduled under, and counted: the live control for
    // the zero counts of the two cases above.
    assert!(
        seen.scheduler_dialled && seen.dials[1] >= 1,
        "a retry when the link returned\n{seen:?}"
    );
}
