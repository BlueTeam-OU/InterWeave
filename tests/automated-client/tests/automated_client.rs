// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! ADR-0055 decisions 2 and 5 and plan section 28 step 2, against real
//! daemons: each profile listens on `127.0.0.1` alone, knows every other
//! only as a static-bootstrap entry on loopback and allows it, and has one
//! endpoint that only one client kind may lease -- a one-host deployment
//! as decision 5 renders it. No other discovery provider is configured, so
//! every connection here IS a static-bootstrap dial over loopback, which no
//! test on `main` made before (the two-daemon tests elsewhere use the
//! host's private address).
//!
//! The client is `interweave_automated_client_tests::Client`, a test's,
//! and the three names it claims are this file's: the transport gives
//! none of them a meaning. What the suite does not prove: a second host,
//! any connectivity behaviour, a durable client record, a production retry
//! schedule, or the cross-account half of decision 5 (a second account's
//! daemon refused a taken port), which needs a second uid this suite does
//! not have.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_automated_client_tests::{Client, ClientConfig, Inbound};
use interweave_test_support::e2e::{Daemon, Home, PATIENCE, free_port, private_tempdir};
use interweave_transport_api::{EndpointId, MessageId, TransportError, TransportIdentity};

/// The endpoint every profile renders; any name the grammar admits.
const ENDPOINT: &str = "automation";
/// The one client kind the endpoint admits.
const CLIENT_KIND: &str = "automated-test-client";
/// The channel every client joins.
const CHANNEL: &str = "example:automation";

fn config() -> ClientConfig {
    ClientConfig::new(ENDPOINT, CLIENT_KIND, CHANNEL)
}

fn endpoint() -> EndpointId {
    config().endpoint
}

/// One profile of the deployment: its home, `PeerId`, port and daemon.
struct Profile {
    home: Home,
    peer: TransportIdentity,
    port: u16,
    daemon: Option<Daemon>,
}

fn loopback() -> std::net::Ipv4Addr {
    std::net::Ipv4Addr::LOCALHOST
}

impl Profile {
    /// A profile under `base`, its key minted by `transportctl identity
    /// init` before any configuration exists (ADR-0055 decision 4).
    fn mint(base: &std::path::Path, name: &str) -> Self {
        let home = Home::within(base, name);
        let out = home.transportctl(&["identity", "init"], "");
        assert!(
            out.status.success(),
            "identity init: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let printed = String::from_utf8(out.stdout).expect("utf-8");
        let peer = TransportIdentity::parse(printed.trim_end()).expect("a PeerId printed");
        Self {
            home,
            peer,
            port: free_port(loopback()),
            daemon: None,
        }
    }

    fn address(&self) -> String {
        format!(
            "/ip4/127.0.0.1/tcp/{}/p2p/{}",
            self.port,
            self.peer.as_str()
        )
    }

    /// The profile a deployment renders: every other profile allowed and a
    /// static-bootstrap entry, this one's own port on loopback.
    fn render(&self, others: &[&Profile]) {
        let allowed = others
            .iter()
            .map(|o| format!("\"{}\"", o.peer.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        let entries = others
            .iter()
            .map(|o| format!("\"{}\"", o.address()))
            .collect::<Vec<_>>()
            .join(", ");
        self.home.write_config(&format!(
            "schema_version: 2
profile: {{ name: {name} }}
trust: {{ policy: static-allowlist, allowed_peers: [{allowed}] }}
endpoints:
  entries:
    - {{ id: {ENDPOINT}, enabled: true, advertise: false, allowed_client_kinds: [{CLIENT_KIND}] }}
channels: {{ desired: [] }}
discovery:
  providers:
    - {{ type: static-bootstrap, enabled: true, priority: 5, config: {{ peers: [{entries}] }} }}
transport: {{ listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/{port}\"] }} }}
observability: {{ log_level: debug }}
",
            name = self.home.paths.profile(),
            port = self.port,
        ));
    }

    async fn start(&mut self) {
        let mut daemon = self.home.start(&[]);
        daemon.serving(&self.home).await;
        self.daemon = Some(daemon);
    }

    /// A clean stop: `SIGTERM`, the daemon's own shutdown.
    async fn stop(&mut self) {
        let mut daemon = self.daemon.take().expect("running");
        assert!(daemon.terminate().await.success(), "{}", daemon.log());
    }

    /// A vanished daemon: `SIGKILL` and reaped, nothing said to a client.
    fn kill(&mut self) {
        drop(self.daemon.take().expect("running"));
    }

    fn log(&self) -> String {
        self.daemon.as_ref().map(Daemon::log).unwrap_or_default()
    }

    async fn client(&self) -> Client {
        let mut client = Client::new(config(), self.home.binding());
        client.connect(PATIENCE).await.expect("the client leases");
        client
    }
}

/// `n` profiles under one scratch tree, each rendered against the others
/// and started.
async fn deployment(n: usize) -> (tempfile::TempDir, Vec<Profile>) {
    let base = private_tempdir();
    let mut profiles: Vec<Profile> = (0..n)
        .map(|i| Profile::mint(base.path(), &format!("profile-{i}")))
        .collect();
    for i in 0..n {
        let others: Vec<&Profile> = profiles
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, p)| p)
            .collect();
        profiles[i].render(&others);
    }
    for profile in &mut profiles {
        profile.start().await;
    }
    (base, profiles)
}

fn id(n: u8) -> MessageId {
    MessageId::from_bytes([n; 16])
}

fn direct_from(received: &[Inbound], peer: &TransportIdentity) -> Vec<Inbound> {
    received
        .iter()
        .filter(|m| !m.broadcast && &m.source == peer)
        .cloned()
        .collect()
}

/// A direct message A's client sends lands at B's client, from A's
/// authenticated `PeerId`, on the configured endpoint -- over the static
/// loopback entry, the only route either has.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_direct_message_lands_over_a_static_loopback_entry() {
    let (_base, profiles) = deployment(2).await;
    let (a, b) = (&profiles[0], &profiles[1]);
    let mut from = a.client().await;
    let mut to = b.client().await;

    let sent = from.send(&b.peer, id(1), "one", PATIENCE).await;
    assert_eq!(
        sent.outcome,
        Ok(endpoint()),
        "accepted at B's endpoint after {:?}\nA:\n{}\nB:\n{}",
        sent.retried,
        a.log(),
        b.log()
    );
    assert!(
        to.pump_until(PATIENCE, |r| !r.inbound.is_empty()).await,
        "B's client holds it"
    );
    let got = direct_from(&to.received.inbound, &a.peer);
    assert_eq!(got.len(), 1, "{:?}", to.received.inbound);
    assert_eq!(got[0].message_id, id(1));
    assert_eq!(got[0].bytes, b"one");
}

/// A broadcast A publishes on the channel reaches B's client, from A's
/// authenticated `PeerId`: the positive control for the burst case below,
/// which counts what the same path drops.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_broadcast_on_the_channel_reaches_the_other_client() {
    let (_base, profiles) = deployment(2).await;
    let (a, b) = (&profiles[0], &profiles[1]);
    let from = a.client().await;
    let mut to = b.client().await;

    // Published until the mesh has formed: before it, a publish is
    // accepted locally and reaches nobody.
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let mut n = 0_u8;
    loop {
        n += 1;
        from.publish(id(n), "all").await.expect("accepted locally");
        if to
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
    let got: Vec<_> = to.received.inbound.iter().filter(|m| m.broadcast).collect();
    assert!(got.iter().all(|m| m.source == a.peer), "{got:?}");
    assert!(got.iter().all(|m| m.bytes == b"all"));
}

/// An application receipt (ADR-0055 decision 3, the client's own): B's
/// client answers A's message with a direct send of its own AFTER it has
/// kept the message, and A's client receives it. The transport sees two
/// opaque messages; `AcceptedV2` alone said only that B's queue took the
/// first.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_application_receipt_reaches_the_sender_after_the_receiver_kept_it() {
    let (_base, profiles) = deployment(2).await;
    let (a, b) = (&profiles[0], &profiles[1]);
    let mut from = a.client().await;
    let mut to = b.client().await;

    let sent = from.send(&b.peer, id(1), "please receipt", PATIENCE).await;
    assert_eq!(sent.outcome, Ok(endpoint()), "{:?}", sent.retried);
    assert!(to.pump_until(PATIENCE, |r| r.inbound.len() == 1).await);
    let kept = to.received.inbound[0].clone();
    let receipt = format!("receipt {:?}", kept.message_id);
    let back = to.send(&a.peer, id(101), &receipt, PATIENCE).await;
    assert_eq!(back.outcome, Ok(endpoint()), "{:?}", back.retried);
    assert!(
        from.pump_until(PATIENCE, |r| {
            r.inbound
                .iter()
                .any(|m| m.source == b.peer && m.bytes == receipt.as_bytes())
        })
        .await,
        "A's client holds B's receipt"
    );
}

/// B's lease withheld: A's send is refused `RemoteEndpointUnavailable`,
/// and A's client retries under the SAME `message_id` until B's client
/// leases again -- then it lands once. A re-send of that id inside the
/// receiver's dedup window is accepted and NOT delivered again (the
/// transport's suppression); after B's daemon restarts, the same re-send
/// IS delivered again, and only B's client collapses it (ADR-0055
/// decision 2: suppression is bounded and in memory).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_withheld_lease_is_retried_under_one_id_and_lands_once() {
    let (_base, mut profiles) = deployment(2).await;
    let mut from = profiles[0].client().await;
    let mut to = profiles[1].client().await;
    let (a_peer, b_peer) = (profiles[0].peer.clone(), profiles[1].peer.clone());

    // The route first, so the refusal below is the lease's, not the dial's.
    let warm = from.send(&b_peer, id(1), "warm", PATIENCE).await;
    assert_eq!(warm.outcome, Ok(endpoint()), "{:?}", warm.retried);
    assert!(to.pump_until(PATIENCE, |r| r.inbound.len() == 1).await);

    to.disconnect();
    let refused = from.send_once(&b_peer, id(2), "while away").await;
    assert_eq!(refused, Err(TransportError::RemoteEndpointUnavailable));

    // B's client returns while A's retries run; A's retries carry id 2.
    let sending = async { from.send(&b_peer, id(2), "while away", PATIENCE).await };
    let returning = async {
        tokio::time::sleep(Duration::from_millis(700)).await;
        to.connect(PATIENCE).await.expect("B leases again");
    };
    let (sent, ()) = tokio::join!(sending, returning);
    assert_eq!(sent.outcome, Ok(endpoint()), "{:?}", sent.retried);
    assert!(
        sent.retried
            .iter()
            .all(|e| *e == TransportError::RemoteEndpointUnavailable),
        "only the lease refused it: {:?}",
        sent.retried
    );
    assert!(
        to.pump_until(PATIENCE, |r| r.inbound.len() == 2).await,
        "id 2 landed"
    );

    // Inside the window: accepted again, delivered once only.
    let again = from.send_once(&b_peer, id(2), "while away").await;
    assert_eq!(again, Ok(endpoint()), "the stored acceptance answered");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let _ = to.pump(PATIENCE).await;
    assert_eq!(direct_from(&to.received.inbound, &a_peer).len(), 2);
    assert_eq!(to.received.duplicates, 0, "the transport suppressed it");

    // Past B's restart: delivered a second time, collapsed by B's client.
    profiles[1].stop().await;
    profiles[1].start().await;
    to.connect(PATIENCE)
        .await
        .expect("B's client leases its new daemon");
    let after = from.send(&b_peer, id(2), "while away", PATIENCE).await;
    assert_eq!(after.outcome, Ok(endpoint()), "{:?}", after.retried);
    assert!(
        to.pump_until(PATIENCE, |r| r.duplicates == 1).await,
        "a second delivery, collapsed as a duplicate:\nB:\n{}",
        profiles[1].log()
    );
    assert_eq!(direct_from(&to.received.inbound, &a_peer).len(), 2);
}

/// A fan-out as decision 3 has a client make it when every recipient must
/// receive: one direct send per recipient under ONE `message_id`. The
/// transport's suppression is keyed per destination, so each receiver
/// lands it once -- and a second fan-out of the same id lands nowhere
/// again inside the window.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_fan_out_under_one_id_lands_once_at_each_receiver() {
    let (_base, profiles) = deployment(3).await;
    let mut from = profiles[0].client().await;
    let mut b = profiles[1].client().await;
    let mut c = profiles[2].client().await;
    for _ in 0..2 {
        for to in [&profiles[1].peer, &profiles[2].peer] {
            let sent = from.send(to, id(9), "to each", PATIENCE).await;
            assert_eq!(sent.outcome, Ok(endpoint()), "{:?}", sent.retried);
        }
    }
    for (name, client) in [("B", &mut b), ("C", &mut c)] {
        assert!(
            client.pump_until(PATIENCE, |r| r.inbound.len() == 1).await,
            "{name} holds it"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
        let _ = client.pump(PATIENCE).await;
        assert_eq!(client.received.inbound.len(), 1, "{name}: once");
        assert_eq!(
            client.received.duplicates, 0,
            "{name}: suppressed in-window"
        );
        assert_eq!(client.received.inbound[0].message_id, id(9));
    }
}

/// A's daemon gone twice under a live client, each way decision 2 names:
/// a clean stop, after which the old session answers `ShuttingDown`, and a
/// vanished daemon (`SIGKILL`), after which it answers
/// `BackendUnavailable`. Each time the client opens a fresh session -- a
/// fresh lease and join -- and its next message lands.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_daemon_gone_under_a_live_client_is_reopened_either_way() {
    let (_base, mut profiles) = deployment(2).await;
    let mut from = profiles[0].client().await;
    let mut to = profiles[1].client().await;
    let b_peer = profiles[1].peer.clone();

    let first = from.send(&b_peer, id(1), "before", PATIENCE).await;
    assert_eq!(first.outcome, Ok(endpoint()), "{:?}", first.retried);

    let mut opened = from.sessions_opened;
    for (n, stop, said) in [
        (2_u8, "a clean stop", TransportError::ShuttingDown),
        (
            3_u8,
            "a vanished daemon",
            TransportError::BackendUnavailable,
        ),
    ] {
        if said == TransportError::ShuttingDown {
            profiles[0].stop().await;
        } else {
            profiles[0].kill();
        }
        profiles[0].start().await;
        let next = from.send(&b_peer, id(n), stop, PATIENCE).await;
        assert_eq!(
            next.outcome,
            Ok(endpoint()),
            "after {stop}: {:?}\nA:\n{}",
            next.retried,
            profiles[0].log()
        );
        assert_eq!(
            next.retried.first().copied(),
            Some(said),
            "after {stop}, the old session said so: {:?}",
            next.retried
        );
        assert_eq!(
            from.sessions_opened,
            opened + 1,
            "after {stop}, one fresh session"
        );
        opened = from.sessions_opened;
        assert!(
            to.pump_until(PATIENCE, |r| r
                .inbound
                .iter()
                .any(|m| m.message_id == id(n)))
                .await,
            "after {stop}, the next message landed"
        );
    }
}

/// The `message_id` of the `n`th message of a burst, tagged by `kind` so
/// a burst's messages are told apart from the case's others.
fn burst_id(n: u32, kind: u8) -> MessageId {
    let mut bytes = [0_u8; 16];
    bytes[..4].copy_from_slice(&n.to_be_bytes());
    bytes[15] = kind;
    MessageId::from_bytes(bytes)
}

/// The burst number of a message `burst_id` made with `kind`, if it is one.
fn burst_number(message: &Inbound, kind: u8) -> Option<u32> {
    let bytes = message.message_id.as_bytes();
    (bytes[15] == kind).then(|| u32::from_be_bytes(bytes[..4].try_into().expect("4 bytes")))
}

const BURST: u32 = 200;

/// 200 broadcasts in a burst, the receiver draining. What arrives is
/// bounded by B's broadcast-ingress bucket for A (ADR-0026: a burst of
/// `DEFAULT_PER_PEER_BURST`, refilled at `DEFAULT_PER_PEER_PER_MINUTE`),
/// not by the client queue of 256: the rest is dropped before local
/// delivery, silently -- no refusal reaches A, and the daemon keeps no
/// drop count to read (`ipc.events_dropped_total` is omitted while no
/// binding keeps one). What arrives is A's, once each.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_burst_of_200_broadcasts_is_bounded_by_the_ingress_bucket() {
    use interweave_transport_runtime::ingress::{
        DEFAULT_PER_PEER_BURST, DEFAULT_PER_PEER_PER_MINUTE,
    };
    const KIND: u8 = 0xb5;
    let (_base, profiles) = deployment(2).await;
    let (a, b) = (&profiles[0], &profiles[1]);
    let from = a.client().await;
    let mut to = b.client().await;

    // The mesh first: one broadcast seen. Each one B took spent a token.
    let started = tokio::time::Instant::now();
    loop {
        from.publish(id(0), "mesh").await.expect("accepted locally");
        if to
            .pump_until(Duration::from_millis(500), |r| !r.inbound.is_empty())
            .await
        {
            break;
        }
        assert!(started.elapsed() < PATIENCE, "no mesh");
    }
    let spent = to.received.inbound.len();

    for n in 1..=BURST {
        from.publish(burst_id(n, KIND), &format!("burst {n}"))
            .await
            .expect("accepted locally: the drop is the receiver's");
    }
    let total = spent + usize::try_from(BURST).expect("n");
    let _ = to
        .pump_until(Duration::from_secs(3), |r| r.inbound.len() >= total)
        .await;
    let elapsed = started.elapsed();
    let burst: Vec<u32> = to.received.inbound[spent..]
        .iter()
        .filter_map(|m| burst_number(m, KIND))
        .collect();
    let refilled = u64::from(DEFAULT_PER_PEER_PER_MINUTE)
        * u64::try_from(elapsed.as_millis()).expect("ms")
        / 60_000;
    let ceiling = usize::try_from(u64::from(DEFAULT_PER_PEER_BURST) + refilled + 1).expect("n");
    let floor = usize::try_from(DEFAULT_PER_PEER_BURST)
        .expect("n")
        .saturating_sub(spent);
    println!(
        "broadcast burst: {BURST} published, {} received ({floor}..={ceiling} allowed by the \
         bucket over {elapsed:?}), {} dropped unseen",
        burst.len(),
        usize::try_from(BURST).expect("n") - burst.len()
    );
    assert!(
        (floor..=ceiling).contains(&burst.len()),
        "received {} outside the bucket's {floor}..={ceiling}",
        burst.len()
    );
    // Each once; the ORDER is not asserted, because nothing promises it:
    // a run here once took a burst out of A's publish order.
    let mut distinct = burst.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), burst.len(), "each once: {burst:?}");
    println!(
        "broadcast burst: in A's order: {}",
        burst.windows(2).all(|w| w[0] < w[1])
    );
    assert_eq!(to.received.duplicates, 0);
    assert!(
        to.received.inbound.iter().all(|m| m.source == a.peer),
        "only A's"
    );
}

/// The same burst of 200 as DIRECT sends -- the control beside the case
/// above, and decision 3's fan-out path. Past B's per-peer bucket B answers
/// `Overloaded` (ENDPOINTS.md, admission step 3), a refusal A SEES, which
/// A's client retries under the same id: every message lands once, later,
/// none lost. The refusal count is the control that the bucket was reached.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_same_burst_as_direct_sends_lands_whole_through_overloaded_retries() {
    const KIND: u8 = 0xd1;
    let (_base, profiles) = deployment(2).await;
    let (a, b) = (&profiles[0], &profiles[1]);
    let mut from = a.client().await;
    let mut to = b.client().await;
    let started = tokio::time::Instant::now();
    let mut overloaded = 0_usize;
    for n in 1..=BURST {
        let sent = from
            .send(&b.peer, burst_id(n, KIND), &format!("burst {n}"), PATIENCE)
            .await;
        assert_eq!(sent.outcome, Ok(endpoint()), "{n}: {:?}", sent.retried);
        overloaded += sent
            .retried
            .iter()
            .filter(|e| **e == TransportError::Overloaded)
            .count();
        // Drained as it goes, as a client holding its lease must be: the
        // client queue of 256 is not what this case measures.
        let _ = to.pump(PATIENCE).await;
    }
    let count = usize::try_from(BURST).expect("n");
    assert!(
        to.pump_until(PATIENCE, |r| r.inbound.len() == count).await,
        "all {count} landed: {}",
        to.received.inbound.len()
    );
    let landed: Vec<u32> = to
        .received
        .inbound
        .iter()
        .filter_map(|m| burst_number(m, KIND))
        .collect();
    assert_eq!(
        landed,
        (1..=BURST).collect::<Vec<_>>(),
        "each once, in order"
    );
    assert_eq!(to.received.duplicates, 0);
    println!(
        "direct burst: {count} sent, {count} landed, {overloaded} Overloaded refusals retried, \
         over {:?}",
        started.elapsed()
    );
    assert!(
        overloaded > 0,
        "the bucket refused some: the control that it was reached"
    );
}

/// The inodes of the TCP sockets listening on `port` on 127.0.0.1, read
/// from the kernel's own table (`/proc/net/tcp`: local address in hex,
/// state `0A` is LISTEN). The address is the kernel's u32 printed in host
/// order, so `127.0.0.1` reads `0100007F` on a little-endian host, which
/// every host this suite runs on is.
#[cfg(target_os = "linux")]
fn listening_inodes(port: u16) -> Vec<String> {
    let local = format!("0100007F:{port:04X}");
    std::fs::read_to_string("/proc/net/tcp")
        .expect("the kernel's TCP table")
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            (fields.get(1) == Some(&local.as_str()) && fields.get(3) == Some(&"0A"))
                .then(|| fields.get(9).map(|inode| (*inode).to_owned()))
                .flatten()
        })
        .collect()
}

/// Whether process `pid` holds the socket `inode`.
#[cfg(target_os = "linux")]
fn holds(pid: u32, inode: &str) -> bool {
    let wanted = format!("socket:[{inode}]");
    std::fs::read_dir(format!("/proc/{pid}/fd")).is_ok_and(|fds| {
        fds.filter_map(Result::ok)
            .filter_map(|fd| std::fs::read_link(fd.path()).ok())
            .any(|target| target.as_os_str() == wanted.as_str())
    })
}

/// Two daemons of ONE account given the same listen port both start and
/// each holds a listening socket on it, as the kernel's own table shows:
/// the TCP transport listens with `SO_REUSEPORT`, so a second socket of
/// the same uid binds beside the first (ADR-0055 decision 5, which is why a
/// deployment runs one profile per account on a port and never two). One
/// socket per process, each held, is the control that the second did not
/// fail to bind and serve its IPC sockets regardless.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_daemons_of_one_account_share_a_port() {
    let base = private_tempdir();
    let mut a = Profile::mint(base.path(), "profile-0");
    let mut b = Profile::mint(base.path(), "profile-1");
    b.port = a.port;
    a.render(&[&b]);
    b.render(&[&a]);
    a.start().await;
    b.start().await;
    let pids: Vec<u32> = [&a, &b]
        .iter()
        .map(|p| p.daemon.as_ref().expect("running").child.id())
        .collect();
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let inodes = loop {
        let inodes = listening_inodes(a.port);
        let each_holds_one = pids
            .iter()
            .all(|pid| inodes.iter().filter(|i| holds(*pid, i)).count() == 1);
        if inodes.len() == 2 && each_holds_one {
            break inodes;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "listening on {}: {inodes:?}, wanted one held by each of {pids:?}
A:
{}
B:
{}",
            a.port,
            a.log(),
            b.log()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    println!(
        "shared port: {} listening sockets on 127.0.0.1:{}, one per daemon",
        inodes.len(),
        a.port
    );
    for p in [&mut a, &mut b] {
        let daemon = p.daemon.as_mut().expect("running");
        assert!(
            daemon.child.try_wait().expect("a status").is_none(),
            "still running:\n{}",
            daemon.log()
        );
    }
}

/// A port held by a socket of this account WITHOUT `SO_REUSEPORT` refuses
/// the daemon's listen (`EADDRINUSE`: the kernel lets a second socket
/// share a port only when every holder set the option), and the daemon
/// exits 1 -- the code its unit's `Restart=on-failure` retries and
/// `RestartPreventExitStatus=2` does not exclude (ADR-0055 decision 5).
/// `std::net::TcpListener` sets `SO_REUSEADDR` and never `SO_REUSEPORT`,
/// so it is the plain holder. The same profile started once the holder
/// lets go is the control: the configuration was never the refusal.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_port_held_without_reuseport_refuses_the_start() {
    let base = private_tempdir();
    let mut a = Profile::mint(base.path(), "profile-0");
    let b = Profile::mint(base.path(), "profile-1");
    a.render(&[&b]);
    let holder = std::net::TcpListener::bind((loopback(), a.port)).expect("the port is free");
    let mut refused = a.home.start(&[]);
    let status = refused.exit().await;
    assert_eq!(status.code(), Some(1), "{}", refused.log());
    assert!(
        refused.log().contains("Address already in use"),
        "refused for the held port, not another cause:\n{}",
        refused.log()
    );
    drop(holder);
    a.start().await;
    a.stop().await;
}
