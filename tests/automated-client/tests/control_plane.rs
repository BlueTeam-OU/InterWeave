// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The fleet's control plane over InterWeave (architect-cto's REQUEST of
//! 2026-10-27, ADR-0055, ENDPOINTS.md: two endpoints, two sessions): one
//! profile and one daemon carry two clients at once. The fleet's carrier
//! holds the lease of `gzcoord` (an `automated-client` endpoint, rendered
//! as the fleet's roster renders it), and `interweave-human listen --json
//! --input --consume --endpoint fabric-control` holds the lease of a second,
//! human endpoint, sending and receiving JSON lines. Two such profiles, A
//! and B, on loopback static entries.
//!
//! What the case checks, beside what `tests/desktop-e2e`'s
//! `the_control_plane_carrier_sends_consumes_and_names_its_endpoint` already
//! shows of the CLI alone:
//! - both leases held on one daemon at once, and each carrier holding
//!   exactly the frames sent to its endpoint, so no `fabric-control` line
//!   reaches `gzcoord`. The other direction is not observable here: the
//!   CLI's facade drops, unprinted, a payload that is not a HumanChatV2
//!   envelope, so a carrier frame misrouted to `fabric-control` would show
//!   nothing; that direction rests on direct v2 resolving to exactly one
//!   destination endpoint;
//! - no message row left in either profile's human store once the lines
//!   were consumed and the sends accepted;
//! - a `send --no-pending` that expired while the peer's daemon was down
//!   never reaches the peer, not even after it comes back: a send made
//!   after it does (the positive control).
//!
//! What it does not: a second host (`two_hosts.rs`), the fabric's own
//! carrier process (the test's `Client` stands in for it, on the same
//! endpoint, client kind and data socket), or the agents' own wire
//! format, which the transport carries as opaque bytes.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::io::{BufRead as _, BufReader, Write as _};
use std::process::{Child, ChildStdin, Command, Output, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use interweave_automated_client_tests::{Client, ClientConfig};
use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort as _};
use interweave_test_support::e2e::{
    Daemon, Home, PATIENCE, free_port, private_tempdir, workspace_binary,
};
use interweave_transport_api::{MessageId, TransportIdentity};
use serde_json::Value;

/// The fleet's endpoint, client kind and (unused) channel.
const CARRIER: &str = "gzcoord";
const CARRIER_KIND: &str = "automated-client";
const CHANNEL: &str = "example:control";
/// The second, human endpoint the CLI leases.
const CONTROL: &str = "fabric-control";

/// How long a peer that could receive a send gets to show it did not:
/// the control send made after it lands well inside this.
const QUIET: Duration = Duration::from_secs(3);

fn carrier_config() -> ClientConfig {
    ClientConfig::new(CARRIER, CARRIER_KIND, CHANNEL)
}

/// One profile: its home, `PeerId`, port and daemon.
struct Profile {
    home: Home,
    peer: TransportIdentity,
    port: u16,
    daemon: Option<Daemon>,
}

impl Profile {
    fn mint(base: &std::path::Path, name: &str) -> Self {
        let home = Home::within(base, name);
        let out = home.transportctl(&["identity", "init"], "");
        assert!(out.status.success(), "{}", said(&out));
        let printed = String::from_utf8(out.stdout).expect("utf-8");
        let peer = TransportIdentity::parse(printed.trim_end()).expect("a PeerId printed");
        Self {
            home,
            peer,
            port: free_port(std::net::Ipv4Addr::LOCALHOST),
            daemon: None,
        }
    }

    /// The fleet roster's rendering (agent-fabric's carrier, one host:
    /// loopback listener and static entry, `gzcoord` for `automated-client`),
    /// with `fabric-control` beside it for `human-client`.
    fn render(&self, other: &Self) {
        self.home.write_config(&format!(
            "schema_version: 2
profile: {{ name: {name} }}
trust:
  policy: static-allowlist
  allowed_peers: [\"{other}\"]
endpoints:
  default_direct_endpoint: {CARRIER}
  entries:
    - {{ id: {CARRIER}, enabled: true, advertise: false, allowed_client_kinds: [{CARRIER_KIND}] }}
    - {{ id: {CONTROL}, enabled: true, advertise: false, allowed_client_kinds: [human-client] }}
channels: {{ desired: [] }}
discovery:
  providers:
    - type: static-bootstrap
      enabled: true
      priority: 5
      config:
        peers: [\"/ip4/127.0.0.1/tcp/{other_port}/p2p/{other}\"]
transport: {{ listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/{port}\"] }} }}
observability: {{ log_level: debug }}
",
            name = self.home.paths.profile(),
            other = other.peer.as_str(),
            other_port = other.port,
            port = self.port,
        ));
    }

    async fn start(&mut self) {
        let mut daemon = self.home.start(&[]);
        daemon.serving(&self.home).await;
        self.daemon = Some(daemon);
    }

    async fn stop(&mut self) {
        let mut daemon = self.daemon.take().expect("running");
        assert!(daemon.terminate().await.success(), "{}", daemon.log());
    }

    fn log(&self) -> String {
        self.daemon.as_ref().map(Daemon::log).unwrap_or_default()
    }

    /// The endpoints whose lease is held, read through the admin port.
    async fn leased(&self) -> Vec<String> {
        let admin = self
            .home
            .binding()
            .admin([AdminCapability::Endpoints].into())
            .await
            .expect("an admin port");
        let mut held: Vec<String> = admin
            .leases()
            .await
            .expect("the leases")
            .into_iter()
            .filter(|v| v.lease.is_some())
            .map(|v| v.endpoint.as_str().to_owned())
            .collect();
        held.sort();
        held
    }

    async fn carrier(&self) -> Client {
        let mut client = Client::new(carrier_config(), self.home.binding());
        client.connect(PATIENCE).await.expect("the carrier leases");
        client
    }

    /// `peer/fabric-control`, the conversation a CLI addresses.
    fn control(&self) -> String {
        format!("{}/{CONTROL}", self.peer.as_str())
    }
}

/// `interweave-human` for `home`'s profile, its environment cleared and
/// the home's XDG roots set, as `tests/desktop-e2e` runs it.
fn human_command(home: &Home, args: &[&str]) -> Command {
    let mut command = Command::new(workspace_binary("interweave-human", "interweave-human-cli"));
    let roots = &home.roots;
    command
        .env_clear()
        .env("XDG_CONFIG_HOME", &roots.config_home)
        .env("XDG_DATA_HOME", &roots.data_home)
        .env("XDG_STATE_HOME", &roots.state_home)
        .env("XDG_CACHE_HOME", &roots.cache_home)
        .env(
            "XDG_RUNTIME_DIR",
            roots.runtime_dir.as_deref().expect("a runtime dir"),
        )
        .args(["--profile", home.paths.profile()])
        .args(args);
    command
}

/// A verb run to its end.
fn human(home: &Home, args: &[&str]) -> Output {
    human_command(home, args)
        .stdin(Stdio::null())
        .output()
        .expect("interweave-human runs")
}

fn said(out: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn json_lines(out: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{l:?}: {e}")))
        .collect()
}

/// `listen --json --input --consume --endpoint fabric-control`, its lines
/// read as they come and every one kept.
struct Listener {
    child: Child,
    input: Option<ChildStdin>,
    lines: mpsc::Receiver<Value>,
    seen: Vec<Value>,
}

impl Listener {
    fn start(home: &Home) -> Self {
        let mut child = human_command(
            home,
            &[
                "listen",
                "--json",
                "--input",
                "--consume",
                "--endpoint",
                CONTROL,
            ],
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("listen runs");
        let input = child.stdin.take();
        let stdout = child.stdout.take().expect("stdout");
        let (send, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                let value = serde_json::from_str(&line)
                    .unwrap_or_else(|e| panic!("listen printed {line:?}: {e}"));
                if send.send(value).is_err() {
                    return;
                }
            }
        });
        let mut listener = Self {
            child,
            input,
            lines,
            seen: Vec::new(),
        };
        listener.until("the session ready", |l| {
            l["event"] == "session" && l["session"] == "ready"
        });
        listener
    }

    /// Every line printed within `patience`, kept, until one `matches`.
    fn within(&mut self, patience: Duration, matches: impl Fn(&Value) -> bool) -> Option<Value> {
        let deadline = Instant::now() + patience;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) => {
                    self.seen.push(line.clone());
                    if matches(&line) {
                        return Some(line);
                    }
                }
                Err(_) => return None,
            }
        }
    }

    fn until(&mut self, what: &str, matches: impl Fn(&Value) -> bool) -> Value {
        self.within(PATIENCE, matches)
            .unwrap_or_else(|| panic!("listen never printed {what}; it printed {:?}", self.seen))
    }

    /// One `--input` line sending `text` to `to`, answered by its ref;
    /// resent until accepted, since the peers may still be connecting.
    fn send_accepted(&mut self, to: &str, text: &str, reference: &str) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let line = serde_json::json!({
                "send": { "conversation": to, "text": text, "ref": reference }
            });
            let input = self.input.as_mut().expect("stdin open");
            writeln!(input, "{line}").expect("written");
            input.flush().expect("flushed");
            let answer = self.until("the send's answer", |l| {
                l["event"] == "outbound" && l["ref"] == reference
            });
            let id = answer["id"].as_str().expect("an id").to_owned();
            let last = self.until("its last status", |l| {
                l["event"] == "outbound"
                    && l["id"] == id.as_str()
                    && matches!(l["status"].as_str(), Some("accepted" | "needs-attention"))
            });
            if last["status"] == "accepted" {
                return;
            }
            assert!(Instant::now() < deadline, "never accepted: {last}");
        }
    }

    /// Whether a `message` event with `text` was printed.
    fn heard(&self, text: &str) -> bool {
        self.seen
            .iter()
            .any(|l| l["event"] == "message" && l["text"] == text)
    }

    /// Stop it as a service manager does: SIGTERM, exit 0.
    fn stop(mut self) {
        drop(self.input.take());
        let killed = Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()
            .expect("kill runs");
        assert!(killed.success());
        let status = self.child.wait().expect("listen ends");
        assert_eq!(status.code(), Some(0), "listen ends cleanly on SIGTERM");
    }
}

/// A listener the test did not stop -- it panicked first -- is killed and
/// reaped: stdin's end does not end `listen`.
impl Drop for Listener {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// No message row in `home`'s store for `conversation`, as `messages`
/// lists the store's rows of it, and none unread or pending in the whole
/// store, as `status` counts them (the key is the one the peer's message
/// event named, asserted where it was printed).
fn no_rows(home: &Home, conversation: &str) {
    let out = human(home, &["messages", conversation, "--json"]);
    assert!(out.status.success(), "{}", said(&out));
    assert!(
        json_lines(&out).is_empty(),
        "rows left for {conversation}: {}",
        said(&out)
    );
    let out = human(home, &["status", "--json"]);
    let status = json_lines(&out);
    assert_eq!(
        (&status[0]["unread"], &status[0]["pending"]),
        (&Value::from(0), &Value::from(0)),
        "{}",
        said(&out)
    );
}

fn id(n: u8) -> MessageId {
    MessageId::from_bytes([n; 16])
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_carrier_and_the_control_cli_share_one_profile_and_leave_no_row() {
    let base = private_tempdir();
    let mut a = Profile::mint(base.path(), "agent-a");
    let mut b = Profile::mint(base.path(), "agent-b");
    a.render(&b);
    b.render(&a);
    a.start().await;
    b.start().await;

    // Two endpoints, two sessions, one daemon each.
    let mut carrier_a = a.carrier().await;
    let mut carrier_b = b.carrier().await;
    let mut cli_a = Listener::start(&a.home);
    let mut cli_b = Listener::start(&b.home);
    for p in [&a, &b] {
        assert_eq!(
            p.leased().await,
            [CONTROL, CARRIER],
            "both leases held\n{}",
            p.log()
        );
    }

    // The carrier's traffic, endpoint to endpoint.
    let sent = carrier_a
        .send(&b.peer, id(1), "carrier frame", PATIENCE)
        .await;
    assert!(sent.outcome.is_ok(), "{sent:?}\n{}", a.log());
    assert!(
        carrier_b
            .pump_until(PATIENCE, |r| r
                .inbound
                .iter()
                .any(|m| m.message_id == id(1)))
            .await,
        "the carrier frame reached B's carrier\n{}",
        b.log()
    );

    // The CLI's lines, each way, on fabric-control.
    cli_a.send_accepted(&b.control(), "control a to b", "r1");
    let at_b = cli_b.until("A's line at B", |l| {
        l["event"] == "message" && l["text"] == "control a to b"
    });
    assert_eq!(at_b["local_endpoint"], CONTROL, "{at_b}");
    cli_b.send_accepted(&a.control(), "control b to a", "r2");
    let at_a = cli_a.until("B's line at A", |l| {
        l["event"] == "message" && l["text"] == "control b to a"
    });
    assert_eq!(at_a["local_endpoint"], CONTROL, "{at_a}");

    assert_eq!(at_b["conversation"], a.control(), "{at_b}");
    assert_eq!(at_a["conversation"], b.control(), "{at_a}");

    // Each carrier holds exactly what was sent to its endpoint: a CLI
    // line routed to gzcoord too would be one more inbound, whatever its
    // bytes.
    carrier_b.pump(PATIENCE).await.expect("pumped");
    carrier_a.pump(PATIENCE).await.expect("pumped");
    let ids = |c: &Client| -> Vec<MessageId> {
        c.received.inbound.iter().map(|m| m.message_id).collect()
    };
    assert_eq!(ids(&carrier_b), [id(1)], "{:?}", carrier_b.received.inbound);
    assert!(
        ids(&carrier_a).is_empty(),
        "{:?}",
        carrier_a.received.inbound
    );

    // Consumed and accepted: no row left on either side.
    cli_a.stop();
    cli_b.stop();
    no_rows(&a.home, &b.control());
    no_rows(&b.home, &a.control());

    // An ephemeral send that expires while B's daemon is down...
    drop(carrier_b);
    b.stop().await;
    let out = human(
        &a.home,
        &[
            "--wait",
            "1500",
            "--endpoint",
            CONTROL,
            "send",
            &b.control(),
            "expired",
            "--no-pending",
            "--json",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", said(&out));
    assert_eq!(json_lines(&out)[0]["status"], "cancelled", "{}", said(&out));
    no_rows(&a.home, &b.control());

    // ...never reaches B once it is back; a send made after it does.
    b.start().await;
    let mut cli_b = Listener::start(&b.home);
    let mut cli_a = Listener::start(&a.home);
    cli_a.send_accepted(&b.control(), "after the expiry", "r3");
    cli_b.until("the control send", |l| {
        l["event"] == "message" && l["text"] == "after the expiry"
    });
    let late = cli_b.within(QUIET, |l| l["event"] == "message" && l["text"] == "expired");
    assert!(
        late.is_none() && !cli_b.heard("expired"),
        "the expired send reached B: {:?}\nA:\n{}",
        cli_b.seen,
        a.log()
    );

    cli_a.stop();
    cli_b.stop();
    drop(carrier_a);
    a.stop().await;
    b.stop().await;
}
