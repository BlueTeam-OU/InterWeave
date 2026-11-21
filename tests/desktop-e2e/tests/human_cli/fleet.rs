// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `interweave-human listen --json --input --consume --endpoint
//! fabric-control` as the fleet runs it (architect-cto's REQUEST
//! 01a1e8b4, j288): one control child per account, every profile on
//! loopback with a static entry for each peer it trusts, `fabric-control`
//! its human endpoint and default.
//!
//! What this does not show: a second host, a peer that drops packets
//! rather than refusing a dial (on loopback a stopped daemon's port
//! answers a reset at once), and the fabric's own process reading the
//! lines.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::process::ChildStdin;
use std::time::{Duration, Instant};

use interweave_transport_api::TransportIdentity;
use serde_json::Value;

use super::common::{Daemon, Home, PATIENCE, free_port};
use super::{Listener, runtime_up};

/// The fleet's human endpoint, the one every control child leases.
const CONTROL: &str = "fabric-control";

/// One account's profile: its home, `PeerId`, loopback port and daemon.
struct Account {
    home: Home,
    peer: TransportIdentity,
    port: u16,
    daemon: Option<Daemon>,
}

impl Account {
    fn mint() -> Self {
        let home = Home::new("gzcoord");
        let peer = home.write_key();
        Self {
            home,
            peer,
            port: free_port(std::net::Ipv4Addr::LOCALHOST),
            daemon: None,
        }
    }

    /// The fleet roster's rendering: `trusted` allowed, each with a
    /// loopback static entry.
    fn render(&self, trusted: &[&Self]) {
        let allowed: Vec<String> = trusted
            .iter()
            .map(|t| format!("\"{}\"", t.peer.as_str()))
            .collect();
        let entries: Vec<String> = trusted
            .iter()
            .map(|t| format!("\"/ip4/127.0.0.1/tcp/{}/p2p/{}\"", t.port, t.peer.as_str()))
            .collect();
        self.home.write_config(&format!(
            "schema_version: 2
profile: {{ name: {name} }}
trust:
  policy: static-allowlist
  allowed_peers: [{allowed}]
endpoints:
  default_direct_endpoint: {CONTROL}
  entries:
    - {{ id: {CONTROL}, enabled: true, advertise: false, allowed_client_kinds: [human-client] }}
channels: {{ desired: [] }}
discovery:
  providers:
    - type: static-bootstrap
      enabled: true
      priority: 5
      config:
        peers: [{entries}]
transport: {{ listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/{port}\"] }} }}
observability: {{ log_level: debug }}
",
            name = self.home.paths.profile(),
            allowed = allowed.join(", "),
            entries = entries.join(", "),
            port = self.port,
        ));
    }

    async fn start(&mut self) {
        let mut daemon = self.home.start(&[]);
        daemon.serving(&self.home).await;
        runtime_up(&mut daemon).await;
        self.daemon = Some(daemon);
    }

    /// The control child, its session ready.
    fn control_child(&self) -> (Listener, ChildStdin) {
        let (listener, input) = Listener::with(
            &self.home,
            &[
                "listen",
                "--json",
                "--input",
                "--consume",
                "--endpoint",
                CONTROL,
            ],
        );
        listener.until("the session ready", |l| {
            l["event"] == "session" && l["session"] == "ready"
        });
        (listener, input)
    }

    /// `peer/fabric-control`, the conversation a control child addresses.
    fn control(&self) -> String {
        format!("{}/{CONTROL}", self.peer.as_str())
    }
}

/// `kill <signal> <pid>`.
fn signal(signal: &str, pid: &str) {
    let sent = std::process::Command::new("kill")
        .args([signal, pid])
        .status()
        .expect("kill runs");
    assert!(sent.success(), "kill {signal} {pid}");
}

/// Bytes waiting unread in the receive queues of `pid`'s established TCP
/// sockets (`/proc/net/tcp`, IPv4: every profile here is on 127.0.0.1).
/// A frozen process reads nothing, so what arrives for it stays counted.
fn queued_at(pid: &str) -> u64 {
    let inodes: BTreeSet<String> = std::fs::read_dir(format!("/proc/{pid}/fd"))
        .expect("the frozen daemon's descriptors")
        .flatten()
        .filter_map(|fd| std::fs::read_link(fd.path()).ok())
        .filter_map(|link| {
            let link = link.to_string_lossy();
            Some(link.strip_prefix("socket:[")?.strip_suffix(']')?.to_owned())
        })
        .collect();
    let table = std::fs::read_to_string("/proc/net/tcp").expect("the TCP table");
    table
        .lines()
        .skip(1)
        .filter_map(|row| {
            let fields: Vec<&str> = row.split_whitespace().collect();
            // Field 3 is the state (01 established), 4 `tx:rx` queues in
            // hex, 9 the socket's inode.
            let established = *fields.get(3)? == "01";
            (established && inodes.contains(*fields.get(9)?))
                .then(|| u64::from_str_radix(fields.get(4)?.rsplit(':').next()?, 16).ok())?
        })
        .sum()
}

/// How long b2's request may take to reach the frozen D: about 20 ms on
/// an idle host. Not `PATIENCE`: B's own traffic on the idle connection
/// reaches D too, 92 bytes some 14.5 s after the freeze (measured with no
/// b2 sent, j300), and a wait that long would read that as the request.
const ON_THE_WIRE: Duration = Duration::from_secs(5);

/// Until more than `before` bytes wait unread at `pid`, within
/// [`ON_THE_WIRE`].
fn until_queued(pid: &str, before: u64) {
    let deadline = Instant::now() + ON_THE_WIRE;
    loop {
        assert!(
            Instant::now() < deadline,
            "nothing reached the frozen daemon {pid} within {ON_THE_WIRE:?} ({before} bytes before)"
        );
        if queued_at(pid) > before {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// One `--input` line, written and flushed.
fn send_line(input: &mut ChildStdin, to: &str, text: &str, reference: &str) {
    writeln!(
        input,
        r#"{{"send":{{"conversation":"{to}","text":"{text}","ref":"{reference}"}}}}"#
    )
    .expect("written");
    input.flush().expect("flushed");
}

/// The first line `matches` takes, every `session` line read on the way
/// kept in `sessions`: a wait that skips lines must not skip the one
/// saying the session moved.
fn noting_sessions(
    listener: &Listener,
    what: &str,
    matches: impl Fn(&Value) -> bool,
    sessions: &mut Vec<Value>,
) -> Value {
    loop {
        let line = listener.until(what, |l| l["event"] == "session" || matches(l));
        if line["event"] == "session" && !matches(&line) {
            sessions.push(line);
            continue;
        }
        return line;
    }
}

/// The line's `outbound` answers until one is `accepted`, failing on a
/// `needs-attention`; each `session` line read on the way is kept in
/// `sessions`.
fn accepted(listener: &Listener, reference: &str, sessions: &mut Vec<Value>) -> Value {
    loop {
        let line = noting_sessions(
            listener,
            "an answer",
            |l| l["event"] == "outbound" && l["ref"] == reference,
            sessions,
        );
        // Said, for the run's record: what the sender was told before.
        eprintln!("{reference}: {line}");
        match line["status"].as_str() {
            Some("accepted") => return line,
            Some("needs-attention") => panic!("{reference} held for attention: {line}"),
            _ => {}
        }
    }
}

/// Item 1. A peer whose daemon is draining refuses `shutting_down`, which
/// reaches the sender as `BackendUnavailable` over its live connection
/// (`TRANSPORT.md` §Error model): the PEER is going, not this session. A
/// daemon drains only while its shutdown waits out exchanges already on
/// the wire (with none it stops at once, and a sender meets a reset), so
/// B is given one: a line to D, whose daemon is frozen (`SIGSTOP`) once
/// the route is up, and B then asked to stop with a grace. A sends to B
/// in that window; A's control child keeps its lease -- it prints no
/// `session` line -- and C's line to A in the window is accepted.
/// Mutation: `ends_session` taking a send's `BackendUnavailable` as the
/// session's end with no probe of the session (the code before j288): A
/// prints `reconnecting` and re-opens.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_peer_shutting_down_leaves_the_senders_session_ready() {
    let (mut a, mut b, mut c, mut d) = (
        Account::mint(),
        Account::mint(),
        Account::mint(),
        Account::mint(),
    );
    a.render(&[&b, &c]);
    b.render(&[&a, &d]);
    c.render(&[&a]);
    d.render(&[&b]);
    for account in [&mut a, &mut b, &mut c, &mut d] {
        account.start().await;
    }
    let (a_child, mut a_input) = a.control_child();
    let (b_child, mut b_input) = b.control_child();
    let (c_child, mut c_input) = c.control_child();
    let (d_child, _d_input) = d.control_child();
    let mut a_sessions = Vec::new();

    // Each route up, before the window opens.
    send_line(&mut a_input, &b.control(), "before the drain", "a1");
    accepted(&a_child, "a1", &mut a_sessions);
    b_child.until("A's line at B", |l| l["text"] == "before the drain");
    send_line(&mut c_input, &a.control(), "warm", "c1");
    accepted(&c_child, "c1", &mut Vec::new());
    noting_sessions(
        &a_child,
        "C's warm line at A",
        |l| l["text"] == "warm",
        &mut a_sessions,
    );
    send_line(&mut b_input, &d.control(), "warm", "b1");
    accepted(&b_child, "b1", &mut Vec::new());
    d_child.until("B's line at D", |l| l["text"] == "warm");

    // D frozen: B's next line to it is written and never answered.
    let d_pid = d.daemon.as_ref().expect("running").child.id().to_string();
    signal("-STOP", &d_pid);
    let before = queued_at(&d_pid);
    send_line(&mut b_input, &d.control(), "held", "b2");
    // ON THE WIRE, not issued: B's `sending` line is its store taking the
    // line, before its daemon has the exchange, and a daemon asked to stop
    // with nothing in flight stops at once -- A then meets a closed
    // connection, not a drain (j300: a full `cargo xtask ci` run, a 300 ms
    // sleep here). D's kernel holding bytes D never read is b2's request
    // written to it.
    until_queued(&d_pid, before);
    // B drains for its grace: it refuses new direct work `shutting_down`,
    // keeping its connections, while the line to D is in flight.
    let out = b.home.transportctl(&["shutdown", "--grace", "8000"], "");
    assert!(out.status.success(), "{out:?}");
    let window = Instant::now();
    send_line(&mut a_input, &b.control(), "into the drain", "a2");
    // The attempt meets B draining: retried, and the session as it was.
    let refused = noting_sessions(
        &a_child,
        "a2 refused by the drain",
        |l| l["event"] == "outbound" && l["ref"] == "a2" && l["problem"] == "service-unavailable",
        &mut a_sessions,
    );
    assert_ne!(refused["status"], "accepted", "B was draining: {refused}");

    send_line(&mut c_input, &a.control(), "in the window", "c2");
    let c_answer = accepted(&c_child, "c2", &mut Vec::new());
    noting_sessions(
        &a_child,
        "C's line at A",
        |l| l["text"] == "in the window",
        &mut a_sessions,
    );
    eprintln!("A's session lines before C's line arrived: {a_sessions:?}");
    assert!(
        window.elapsed() < Duration::from_secs(8),
        "C's line was answered inside B's grace: {c_answer}"
    );
    // What A printed in the window, read to its end.
    while let Ok(line) = a_child.lines.recv_timeout(Duration::from_millis(500)) {
        if line["event"] == "session" {
            a_sessions.push(line);
        }
    }
    eprintln!("A's session lines in the window: {a_sessions:?}");
    assert!(
        a_sessions.is_empty(),
        "a peer's shutdown is not A's session ending: {a_sessions:?}"
    );

    a_child.stop();
    c_child.stop();
    b_child.stop();
    signal("-CONT", &d_pid);
    d_child.stop();
    for account in [&mut a, &mut c, &mut d] {
        if let Some(mut daemon) = account.daemon.take() {
            daemon.terminate().await;
        }
    }
    // B ends at the end of its grace, as asked.
    if let Some(mut daemon) = b.daemon.take() {
        daemon.exit().await;
    }
}

/// Every line of `references` accepted, in whatever order, within
/// `patience`: sends to different peers answer in any order, so a wait
/// for one that drops the lines of another would lose them.
fn all_accepted(listener: &Listener, mut references: BTreeSet<String>, patience: Duration) {
    let deadline = Instant::now() + patience;
    while !references.is_empty() {
        let left = deadline.saturating_duration_since(Instant::now());
        let line = listener
            .lines
            .recv_timeout(left)
            .unwrap_or_else(|e| panic!("{references:?} not accepted within {patience:?}: {e}"));
        if line["event"] == "outbound" {
            assert!(
                line["status"] != "needs-attention",
                "held for attention: {line}"
            );
            if line["status"] == "accepted"
                && let Some(reference) = line["ref"].as_str()
            {
                references.remove(reference);
            }
        }
    }
}

/// How a spoke that does not answer is not answering.
#[derive(Clone, Copy, Debug)]
enum Down {
    /// Its daemon never started: on loopback the dial is refused at once.
    NotRunning,
    /// Its daemon frozen (`SIGSTOP`) once its route is up: the request is
    /// written and never answered, as a peer whose host hangs.
    Frozen,
}

/// Item 2's measurement, a run record rather than a gate (`--ignored`):
/// one control child writes one `--input` line to each of 30 peers, with
/// the first `down` of them not answering, and the time from the first
/// line written to the last `accepted` of a peer that answers is printed
/// with the host's load, which the figure depends on. Each run has
/// a hub of its own, so no earlier run's pending rows retry beside it.
///
/// `cargo test -p interweave-desktop-e2e-tests --test human_cli
/// fleet::fan_out -- --ignored --nocapture`
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "a measurement: 35 daemons, run by hand for its record"]
async fn fan_out_of_thirty_lines_timed() {
    const PEERS: usize = 30;
    // Frozen first: a stopped spoke stays stopped for the runs after it,
    // and every frozen one is thawed when its run ends.
    let runs: [(usize, Down); 5] = [
        (1, Down::Frozen),
        (3, Down::Frozen),
        (0, Down::NotRunning),
        (1, Down::NotRunning),
        (3, Down::NotRunning),
    ];
    let mut hubs: Vec<Account> = runs.iter().map(|_| Account::mint()).collect();
    let mut spokes: Vec<Account> = (0..PEERS).map(|_| Account::mint()).collect();
    {
        let trusted: Vec<&Account> = hubs.iter().collect();
        for spoke in &spokes {
            spoke.render(&trusted);
        }
        let trusted: Vec<&Account> = spokes.iter().collect();
        for hub in &hubs {
            hub.render(&trusted);
        }
    }
    for spoke in &mut spokes {
        spoke.start().await;
    }
    let mut children: Vec<(Listener, ChildStdin)> =
        spokes.iter().map(Account::control_child).collect();
    let mut stopped: Vec<usize> = Vec::new();
    let mut frozen: Vec<String> = Vec::new();
    let mut record = Vec::new();
    for (n, ((down, how), hub)) in runs.iter().zip(hubs.iter_mut()).enumerate() {
        // The FIRST `down` spokes do not answer this run: their lines go
        // first, so whatever they hold up is every line after them.
        let silent: Vec<usize> = (0..*down).collect();
        hub.start().await;
        let (hub_child, mut hub_input) = hub.control_child();
        // Routes up first, as the fleet's would be: each spoke once.
        for (i, spoke) in spokes.iter().enumerate() {
            if !stopped.contains(&i) {
                send_line(
                    &mut hub_input,
                    &spoke.control(),
                    "warm",
                    &format!("w{n}-{i}"),
                );
            }
        }
        // Answers come in any order: attempts to peers go beside each other.
        let warm: BTreeSet<String> = (0..PEERS)
            .filter(|i| !stopped.contains(i))
            .map(|i| format!("w{n}-{i}"))
            .collect();
        all_accepted(&hub_child, warm, PATIENCE);
        for &i in &silent {
            let pid = spokes[i].daemon.as_ref().map(|d| d.child.id().to_string());
            match (how, pid) {
                (Down::NotRunning, Some(_)) => {
                    if let Some(mut daemon) = spokes[i].daemon.take() {
                        daemon.terminate().await;
                    }
                    stopped.push(i);
                }
                (Down::Frozen, Some(pid)) if !frozen.contains(&pid) => {
                    signal("-STOP", &pid);
                    frozen.push(pid);
                }
                _ => {}
            }
        }
        let first = Instant::now();
        for (i, spoke) in spokes.iter().enumerate() {
            send_line(
                &mut hub_input,
                &spoke.control(),
                "fan-out",
                &format!("f{n}-{i}"),
            );
        }
        // Two lines to the last spoke, after the rest: the spoke must
        // print them in the order written.
        let last_spoke = PEERS - 1;
        for k in 1..=2 {
            send_line(
                &mut hub_input,
                &spokes[last_spoke].control(),
                &format!("order-{n}-{k}"),
                &format!("o{n}-{k}"),
            );
        }
        // While the silent peers' lines are held, the first answering
        // spoke sends the hub a line: inbound must not wait on them.
        let back = format!("back-{n}");
        let back_from = *down;
        let back_sent = Instant::now();
        send_line(
            &mut children[back_from].1,
            &hub.control(),
            &back,
            &format!("b{n}"),
        );
        let mut waiting: BTreeSet<String> = (0..PEERS)
            .filter(|i| !silent.contains(i))
            .map(|i| format!("f{n}-{i}"))
            .chain((1..=2).map(|k| format!("o{n}-{k}")))
            .collect();
        let mut last = Duration::ZERO;
        let mut back_at: Option<Duration> = None;
        let deadline = first + Duration::from_secs(600);
        while !waiting.is_empty() || back_at.is_none() {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = hub_child
                .lines
                .recv_timeout(left)
                .unwrap_or_else(|e| panic!("run {n}: {waiting:?} and back={back_at:?}: {e}"));
            if line["event"] == "outbound"
                && line["status"] == "accepted"
                && let Some(reference) = line["ref"].as_str()
                && waiting.remove(reference)
                && reference.starts_with('f')
            {
                last = last.max(first.elapsed());
            }
            assert!(
                !(line["event"] == "outbound" && line["status"] == "needs-attention"),
                "run {n}: held for attention: {line}"
            );
            if line["event"] == "message" && line["text"] == back.as_str() {
                back_at = Some(back_sent.elapsed());
            }
        }
        let spoke_child = &children[last_spoke].0;
        let first_seen = spoke_child.until("an order line", |l| {
            l["event"] == "message"
                && l["text"]
                    .as_str()
                    .is_some_and(|t| t.starts_with(&format!("order-{n}-")))
        });
        assert_eq!(
            first_seen["text"],
            format!("order-{n}-1"),
            "two lines to one peer arrive in the order written"
        );
        spoke_child.until("the second order line", |l| {
            l["event"] == "message" && l["text"] == format!("order-{n}-2").as_str()
        });
        let load = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
        let line = format!(
            "{down} of {PEERS} {how:?}: last accepted of an answering peer {last:?} after the first line; \
             a line from a spoke reached the hub {:?} after it was written; two lines to one spoke in order (loadavg {})",
            back_at.unwrap_or_default(),
            load.trim()
        );
        eprintln!("{line}");
        record.push(line);
        hub_child.stop();
        if let Some(mut daemon) = hub.daemon.take() {
            daemon.terminate().await;
        }
        // A frozen spoke thawed for the next run's own choice.
        for pid in frozen.drain(..) {
            signal("-CONT", &pid);
        }
    }
    eprintln!("RECORD\n{}", record.join("\n"));
    for (child, _) in children {
        drop(child);
    }
    for spoke in &mut spokes {
        if let Some(mut daemon) = spoke.daemon.take() {
            daemon.terminate().await;
        }
    }
}
