// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Claude Code Channel bridge against two real daemons (plan §19
//! step 4). The shipped `claude-channel` binary runs as Claude Code runs
//! it: a child process speaking JSON-RPC on its stdio, started with
//! `--profile` and `--endpoint` in its daemon's XDG tree (A's, except in
//! the second-network case, where the sides swap), so the path is:
//! - host lines in, through the bridge's IPC client and its daemon;
//! - then libp2p to the far daemon and an IPC session leased on its
//!   `human`;
//! - and back.
//!
//! Each notification's `meta` is held to the contract's key list and the
//! host's grammar (CHANNEL-EVENT.md; P1).
//!
//! What this does not prove:
//! - the host itself (step 5: the bridge loaded as a plugin in the
//!   installed Claude Code);
//! - a network other than one host's private address, except in one
//!   case (`the_bridge_behind_its_own_nat_exchanges_with_the_facade_through_the_relay`),
//!   where the bridge and the facade are each behind their own NAT and
//!   talk through a relay (plan section 21, decided (7)).
//!
//! The far end is a plain data session in most cases here. In two cases
//! (`the_human_clients_facade_exchanges_with_the_bridge_plain_and_compressed`
//! and the second-network one) it is the human client's own facade, with
//! its store, over its IPC binding. The facade, not the test, decides how
//! each envelope travels, and the test reads that choice from the
//! facade's pending row. The bridge decodes `;ce=br` and forwards the
//! envelope's JSON as text, reading nothing of what it means. Toward the
//! facade the bridge sends plain only: a tool's `content` is a UTF-8
//! string, so the model cannot hand it compressed bytes.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::VecDeque;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use interweave_ipc_client::IpcSession;
use interweave_local_client_api::{
    DataCapability, DataSessionBinding as _, DataSessionPort as _, SessionEvent, SessionRequest,
};
use interweave_transport_api::{
    BroadcastMessageV1, ChannelId, DirectDestination, EndpointId, MAX_PAYLOAD_BYTES, MediaType,
    MessageId, Payload, TransportIdentity,
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout};

mod common;
use common::{Daemon, Home, PATIENCE, example, free_port, human, workspace_binary};

/// CHANNEL-EVENT.md §Metadata's table, in its order.
const META_KEYS: [&str; 10] = [
    "delivery_mode",
    "source_peer",
    "source_endpoint",
    "destination_endpoint",
    "message_id",
    "received_at",
    "channel",
    "reply_token",
    "payload_encoding",
    "content_type",
];

fn claude() -> EndpointId {
    EndpointId::parse("claude").expect("endpoint")
}

fn general() -> ChannelId {
    ChannelId::parse("general").expect("channel")
}

fn bridge_binary() -> std::path::PathBuf {
    workspace_binary("claude-channel", "interweave-claude-channel")
}

/// The shipped bridge, started in `home`'s environment.
struct Bridge {
    child: Child,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
    /// Notifications read while waiting for an answer, each with the line
    /// it came on, whose text holds `meta`'s order.
    notifications: VecDeque<(Value, String)>,
    next_id: u64,
}

impl Bridge {
    fn start(home: &Home) -> Self {
        Self::start_in(home, "push")
    }

    /// The bridge as a plain MCP host would start it: `--delivery pull`.
    fn start_pull(home: &Home) -> Self {
        Self::start_in(home, "pull")
    }

    fn start_in(home: &Home, delivery: &str) -> Self {
        Self::spawn(
            home,
            delivery,
            tokio::process::Command::new(bridge_binary()),
        )
    }

    /// The bridge in `net`'s namespace, beside its daemon there
    /// (`Home::start_in`). By path: the environment is cleared.
    #[cfg(target_os = "linux")]
    fn start_in_net(home: &Home, net: common::netns::Net) -> Self {
        let mut command = tokio::process::Command::new("/usr/bin/nsenter");
        command
            .arg(format!("--net={}", net.path().display()))
            .arg("--")
            .arg(bridge_binary());
        Self::spawn(home, "push", command)
    }

    fn spawn(home: &Home, delivery: &str, mut command: tokio::process::Command) -> Self {
        let env = |p: &Path| p.as_os_str().to_owned();
        let mut child = command
            .args([
                "--profile",
                home.paths.profile(),
                "--endpoint",
                "claude",
                "--delivery",
                delivery,
            ])
            .env_clear()
            .env("XDG_CONFIG_HOME", env(&home.roots.config_home))
            .env("XDG_DATA_HOME", env(&home.roots.data_home))
            .env("XDG_STATE_HOME", env(&home.roots.state_home))
            .env("XDG_CACHE_HOME", env(&home.roots.cache_home))
            .env(
                "XDG_RUNTIME_DIR",
                env(home.roots.runtime_dir.as_deref().expect("a runtime dir")),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("the bridge starts");
        let stdin = child.stdin.take().expect("stdin");
        let lines = BufReader::new(child.stdout.take().expect("stdout")).lines();
        Self {
            child,
            stdin,
            lines,
            notifications: VecDeque::new(),
            next_id: 0,
        }
    }

    async fn line(&mut self) -> (Value, String) {
        let line = tokio::time::timeout(PATIENCE, self.lines.next_line())
            .await
            .expect("the bridge writes in time")
            .expect("readable")
            .expect("the bridge is running");
        let value = serde_json::from_str(&line).expect("one JSON value per line");
        (value, line)
    }

    async fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let line = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.stdin
            .write_all(format!("{line}\n").as_bytes())
            .await
            .expect("written");
        loop {
            let (value, line) = self.line().await;
            if value["id"] == json!(id) {
                return value;
            }
            if value["method"] == json!("notifications/claude/channel") {
                self.notifications
                    .push_back((value["params"].clone(), line));
            }
        }
    }

    async fn tool(&mut self, name: &str, arguments: Value) -> (String, bool) {
        let answer = self
            .request("tools/call", json!({"name": name, "arguments": arguments}))
            .await;
        let text = answer["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("a tool result: {answer}"))
            .to_owned();
        (text, answer["result"]["isError"] == json!(true))
    }

    async fn status(&mut self) -> Value {
        let (text, _) = self.tool("status", json!({})).await;
        serde_json::from_str(&text).expect("status is JSON")
    }

    /// Until the bridge holds its lease on a live daemon.
    async fn wait_leased(&mut self, daemons: &[&Daemon]) {
        let deadline = tokio::time::Instant::now() + PATIENCE;
        while self.status().await["endpoint_lease_state"] != json!("held") {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the bridge never leased claude\n{}",
                logs(daemons)
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// The next channel notification, held to the contract (P1).
    async fn notification(&mut self) -> Value {
        let (params, line) = match self.notifications.pop_front() {
            Some(held) => held,
            None => loop {
                let (value, line) = self.line().await;
                if value["method"] == json!("notifications/claude/channel") {
                    break (value["params"].clone(), line);
                }
            },
        };
        meta_is_the_contracts(&params["meta"], &line);
        params
    }
}

/// Every key one of the table's, in the table's order AS WRITTEN on
/// `line` -- a parsed `Value`'s map sorts its keys, so the order is read
/// from the text; each a match for the host's grammar; none `source`;
/// every value a string.
fn meta_is_the_contracts(meta: &Value, line: &str) {
    let object = meta.as_object().expect("meta is an object");
    let written = &line[line.find("\"meta\"").expect("a meta key")..];
    let mut order: Vec<(usize, &str)> = object
        .keys()
        .map(|key| {
            let at = written
                .find(&format!("\"{key}\":"))
                .unwrap_or_else(|| panic!("{key} on the line"));
            (at, key.as_str())
        })
        .collect();
    order.sort_unstable();
    let mut last = None;
    for (_, key) in order {
        let at = META_KEYS
            .iter()
            .position(|k| *k == key)
            .unwrap_or_else(|| panic!("{key} is not a CHANNEL-EVENT.md key"));
        assert!(
            last.is_none_or(|l| l < at),
            "{key} out of table order: {line}"
        );
        last = Some(at);
    }
    // The table's "direct only" and "only for broadcast" rows, both ways.
    let has = |key: &str| object.contains_key(key);
    match meta["delivery_mode"].as_str() {
        Some("direct") => {
            assert!(
                has("source_endpoint") && has("destination_endpoint"),
                "{meta}"
            );
            assert!(!has("channel"), "no channel on a direct message: {meta}");
        }
        Some("broadcast") => {
            assert!(has("channel"), "{meta}");
            assert!(
                !has("source_endpoint") && !has("destination_endpoint"),
                "no endpoint on a broadcast: {meta}"
            );
        }
        other => panic!("delivery_mode {other:?}"),
    }
    for (key, value) in object {
        assert!(value.is_string(), "{key} is a string: {meta}");
        let mut chars = key.chars();
        assert!(
            chars
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        );
        assert!(chars.all(|c| c.is_ascii_alphanumeric() || c == '_'));
        assert_ne!(key, "source");
    }
}

fn logs(daemons: &[&Daemon]) -> String {
    daemons
        .iter()
        .enumerate()
        .map(|(i, d)| format!("daemon {i}:\n{}", d.log()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Two daemons from the shipped desktop example (endpoints `human` and
/// `claude`), each with a static route to the other, both serving. Not
/// yet connected: the first exchange waits on that (`send_until_accepted`,
/// `broadcast_until`).
async fn two_daemons() -> (
    Home,
    Daemon,
    TransportIdentity,
    Home,
    Daemon,
    TransportIdentity,
) {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (a, b) = (Home::new("human-desktop"), Home::new("human-desktop"));
    let (a_peer, b_peer) = (a.write_key(), b.write_key());
    let (a_port, b_port) = (free_port(ip), free_port(ip));
    let at = |port: u16| format!("/ip4/{ip}/tcp/{port}");
    let route = |port: u16, peer: &TransportIdentity| format!("{}/p2p/{}", at(port), peer.as_str());
    a.write_config(&example(
        "human-desktop.yaml",
        &b_peer,
        &at(a_port),
        Some(&route(b_port, &b_peer)),
    ));
    b.write_config(&example(
        "human-desktop.yaml",
        &a_peer,
        &at(b_port),
        Some(&route(a_port, &a_peer)),
    ));
    let mut a_daemon = a.start(&[]);
    a_daemon.serving(&a).await;
    let mut b_daemon = b.start(&[]);
    b_daemon.serving(&b).await;
    (a, a_daemon, a_peer, b, b_daemon, b_peer)
}

/// A human-client session leased on `home`'s `endpoint`.
async fn session(home: &Home, endpoint: EndpointId) -> IpcSession {
    home.binding()
        .open(
            SessionRequest::new(
                "human-client",
                Some(endpoint),
                [DataCapability::Events, DataCapability::Commands],
            )
            .expect("request"),
        )
        .await
        .expect("a session")
}

/// A session leased on `home`'s `claude`, under the client kind that
/// endpoint admits.
async fn claude_session(home: &Home) -> IpcSession {
    home.binding()
        .open(
            SessionRequest::new(
                "claude-channel",
                Some(claude()),
                [DataCapability::Events, DataCapability::Commands],
            )
            .expect("request"),
        )
        .await
        .expect("a session on claude")
}

/// Publish on `general` from `from` until the bridge is notified of it --
/// the mesh forms after the join, so the first publishes may reach no one
/// -- and return that notification.
async fn broadcast_until(
    from: &IpcSession,
    bridge: &mut Bridge,
    content: &str,
    daemons: &[&Daemon],
) -> Value {
    let deadline = tokio::time::Instant::now() + PATIENCE * 2;
    loop {
        from.broadcast(
            general(),
            BroadcastMessageV1 {
                message_id: MessageId::from_bytes(rand_bytes()),
                sent_at_ms: 0,
                payload: text(content),
            },
        )
        .await
        .expect("accepted for local publish");
        let _ = bridge.status().await;
        // Only this content's notification counts: an earlier publish,
        // delivered late, must not stand in for it.
        while let Some((n, line)) = bridge.notifications.pop_front() {
            if n["content"] == json!(content) {
                meta_is_the_contracts(&n["meta"], &line);
                return n;
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no broadcast reached the bridge\n{}",
            logs(daemons)
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

fn text(content: &str) -> Payload {
    Payload::new(None, content.as_bytes().to_vec(), MAX_PAYLOAD_BYTES).expect("payload")
}

/// Send from `from` to `peer`/`endpoint` until the far transport accepts:
/// the first exchange waits on the pair's connection, which the retry
/// scheduler makes within its base (plan §19; the restart and start-up
/// cases rust-ui-dev measured).
async fn send_until_accepted(
    from: &IpcSession,
    peer: &TransportIdentity,
    endpoint: EndpointId,
    content: &str,
    daemons: &[&Daemon],
) -> EndpointId {
    send_payload_until_accepted(from, peer, endpoint, &text(content), daemons).await
}

/// [`send_until_accepted`] for a payload of any media type.
async fn send_payload_until_accepted(
    from: &IpcSession,
    peer: &TransportIdentity,
    endpoint: EndpointId,
    payload: &Payload,
    daemons: &[&Daemon],
) -> EndpointId {
    let deadline = tokio::time::Instant::now() + PATIENCE * 3;
    loop {
        match from
            .send_direct(
                DirectDestination {
                    peer: peer.clone(),
                    endpoint: Some(endpoint.clone()),
                },
                MessageId::from_bytes(rand_bytes()),
                payload.clone(),
            )
            .await
        {
            Ok(accepted) => return accepted,
            Err(e) => assert!(
                tokio::time::Instant::now() < deadline,
                "never accepted ({e:?})\n{}",
                logs(daemons)
            ),
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn rand_bytes() -> [u8; 16] {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let mut bytes = [0x5a; 16];
    bytes[..8].copy_from_slice(&n.to_be_bytes());
    bytes
}

/// Whether `events_of` has received a direct message carrying `content`.
async fn direct_from(events_of: &IpcSession, content: &[u8]) -> bool {
    events_of
        .events(usize::MAX)
        .await
        .expect("events")
        .iter()
        .any(|e| matches!(e, SessionEvent::Direct(d) if d.payload.bytes() == content))
}

/// Incoming direct → a channel event with both endpoints, `meta` held to
/// the contract; `reply` takes the exact route back to B's `claude`, not
/// to B's default `human`, which stays leased so a fallback would land
/// somewhere (§19 required tests).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_direct_message_is_notified_and_replied_to_across_two_daemons() {
    let (a, a_daemon, a_peer, b, b_daemon, b_peer) = two_daemons().await;
    let daemons = [&a_daemon, &b_daemon];
    let mut bridge = Bridge::start(&a);
    bridge.wait_leased(&daemons).await;
    let b_human = session(&b, human()).await;
    let b_claude = claude_session(&b).await;

    let accepted =
        send_until_accepted(&b_claude, &a_peer, claude(), "hello claude", &daemons).await;
    assert_eq!(accepted, claude());
    let n = bridge.notification().await;
    assert_eq!(n["content"], json!("hello claude"));
    let meta = &n["meta"];
    assert_eq!(meta["delivery_mode"], json!("direct"));
    assert_eq!(meta["source_peer"], json!(b_peer.as_str()));
    assert_eq!(meta["source_endpoint"], json!("claude"));
    assert_eq!(meta["destination_endpoint"], json!("claude"));
    assert_eq!(meta["payload_encoding"], json!("utf8"));
    let token = meta["reply_token"].as_str().expect("a token").to_owned();

    let (answer, error) = bridge
        .tool(
            "reply",
            json!({"reply_token": token, "content": "hello human"}),
        )
        .await;
    assert!(!error, "{answer}\n{}", logs(&daemons));
    assert_eq!(answer, "remote transport accepted at endpoint claude");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while !direct_from(&b_claude, b"hello human").await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the reply never arrived"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        !direct_from(&b_human, b"hello human").await,
        "the reply went to the sender's endpoint, not B's default"
    );

    // `send` names an endpoint other than B's default (`human`), and the
    // message lands there, not on the default.
    let (answer, error) = bridge
        .tool(
            "send",
            json!({"peer": b_peer.as_str(), "endpoint": "claude", "content": "named"}),
        )
        .await;
    assert!(!error, "{answer}");
    assert_eq!(answer, "remote transport accepted at endpoint claude");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while !direct_from(&b_claude, b"named").await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the named send never arrived at B's claude"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        !direct_from(&b_human, b"named").await,
        "and not at B's default"
    );
}

/// One `PeerId`, two endpoints: B's message to A's `claude` reaches the
/// bridge alone, and its message to A's `human` reaches A's human
/// session alone (§19 required tests).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_human_and_claude_endpoints_of_one_peer_are_routed_apart() {
    let (a, a_daemon, a_peer, b, b_daemon, _) = two_daemons().await;
    let daemons = [&a_daemon, &b_daemon];
    let mut bridge = Bridge::start(&a);
    bridge.wait_leased(&daemons).await;
    let a_human = session(&a, human()).await;
    let b_human = session(&b, human()).await;

    send_until_accepted(&b_human, &a_peer, claude(), "to claude", &daemons).await;
    send_until_accepted(&b_human, &a_peer, human(), "to human", &daemons).await;
    let n = bridge.notification().await;
    assert_eq!(n["content"], json!("to claude"));
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let mut seen = Vec::new();
    while seen.is_empty() {
        seen.extend(
            a_human
                .events(usize::MAX)
                .await
                .expect("events")
                .into_iter()
                .filter_map(|e| match e {
                    SessionEvent::Direct(d) => Some(d.payload.bytes().to_vec()),
                    _ => None,
                }),
        );
        assert!(
            tokio::time::Instant::now() < deadline,
            "the human message never came"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(seen, [b"to human".to_vec()], "A's human saw only its own");
    // And the bridge nothing more: a status round trip flushes any
    // notification written before it.
    let _ = bridge.status().await;
    assert!(
        bridge.notifications.is_empty(),
        "the bridge saw only claude's: {:?}",
        bridge.notifications
    );
}

/// Broadcast: the bridge joins, receives B's publish, replies on the
/// channel; once it leaves, the reply fails `ChannelNotJoined`
/// (§19 required tests).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn broadcast_join_publish_reply_and_channel_not_joined() {
    let (a, a_daemon, a_peer, b, b_daemon, _) = two_daemons().await;
    let daemons = [&a_daemon, &b_daemon];
    let mut bridge = Bridge::start(&a);
    bridge.wait_leased(&daemons).await;
    let b_human = session(&b, human()).await;
    // A direct exchange first: the pair is connected, so the mesh can form.
    send_until_accepted(&b_human, &a_peer, claude(), "warm", &daemons).await;
    let _ = bridge.notification().await;

    let (answer, error) = bridge.tool("join", json!({"channel": "general"})).await;
    assert!(!error, "{answer}");
    b_human.join(general()).await.expect("B joins");
    let deadline = tokio::time::Instant::now() + PATIENCE * 2;
    let n = loop {
        b_human
            .broadcast(
                general(),
                BroadcastMessageV1 {
                    message_id: MessageId::from_bytes(rand_bytes()),
                    sent_at_ms: 0,
                    payload: text("hi all"),
                },
            )
            .await
            .expect("accepted for local publish");
        let _ = bridge.status().await;
        if let Some((n, line)) = bridge.notifications.pop_front() {
            meta_is_the_contracts(&n["meta"], &line);
            break n;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no broadcast reached the bridge\n{}",
            logs(&daemons)
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    assert_eq!(n["meta"]["delivery_mode"], json!("broadcast"));
    assert_eq!(n["meta"]["channel"], json!("general"));
    assert!(n["meta"].get("source_endpoint").is_none());
    let token = n["meta"]["reply_token"].as_str().expect("token").to_owned();
    let (answer, error) = bridge
        .tool("reply", json!({"reply_token": token, "content": "hi back"}))
        .await;
    assert!(!error, "{answer}");
    assert_eq!(answer, "accepted for local publish");

    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        let arrived = b_human
            .events(usize::MAX)
            .await
            .expect("events")
            .iter()
            .any(|e| {
                matches!(e, SessionEvent::Broadcast(m)
                if m.channel == general() && m.payload.bytes() == b"hi back")
            });
        if arrived {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the bridge's broadcast never reached B"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let (answer, error) = bridge.tool("leave", json!({"channel": "general"})).await;
    assert!(!error, "{answer}");
    let (answer, error) = bridge
        .tool("reply", json!({"reply_token": token, "content": "after"}))
        .await;
    assert!(error && answer.starts_with("ChannelNotJoined"), "{answer}");
}

/// The daemon away and back: A's daemon stopped, the bridge keeps
/// answering `status` and refuses network tools clearly; restarted, the
/// bridge claims afresh, and a reply token from before fails rather than
/// switching routes (§19 required tests, LIFECYCLE.md).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_daemon_away_and_back_with_a_stale_token() {
    let (a, mut a_daemon, a_peer, b, b_daemon, b_peer) = two_daemons().await;
    let mut bridge = Bridge::start(&a);
    bridge.wait_leased(&[&a_daemon, &b_daemon]).await;
    let b_human = session(&b, human()).await;
    send_until_accepted(
        &b_human,
        &a_peer,
        claude(),
        "before",
        &[&a_daemon, &b_daemon],
    )
    .await;
    let token = bridge.notification().await["meta"]["reply_token"]
        .as_str()
        .expect("token")
        .to_owned();
    let before = bridge.status().await["endpoint_lease_epoch"].clone();
    let (answer, error) = bridge.tool("join", json!({"channel": "general"})).await;
    assert!(!error, "{answer}");
    b_human.join(general()).await.expect("B joins");
    // The control: the join carries a broadcast before the restart.
    broadcast_until(
        &b_human,
        &mut bridge,
        "before restart",
        &[&a_daemon, &b_daemon],
    )
    .await;

    a_daemon.terminate().await;
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while bridge.status().await["endpoint_lease_state"] != json!("daemon unavailable") {
        assert!(
            tokio::time::Instant::now() < deadline,
            "never saw the daemon go"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (answer, error) = bridge
        .tool("send", json!({"peer": b_peer.as_str(), "content": "x"}))
        .await;
    assert!(
        error && answer.starts_with("BackendUnavailable"),
        "{answer}"
    );

    let mut a_daemon = a.start(&[]);
    a_daemon.serving(&a).await;
    bridge.wait_leased(&[&a_daemon, &b_daemon]).await;
    assert_ne!(
        bridge.status().await["endpoint_lease_epoch"],
        before,
        "a fresh claim"
    );
    let (answer, error) = bridge
        .tool("reply", json!({"reply_token": token, "content": "stale"}))
        .await;
    assert!(error && answer.starts_with("InvalidArgument"), "{answer}");
    // Fresh joins: the restarted daemon knew nothing of the bridge's join,
    // and B's broadcast reaches the bridge again.
    assert_eq!(bridge.status().await["joined_channels"], json!(["general"]));
    let n = broadcast_until(
        &b_human,
        &mut bridge,
        "after restart",
        &[&a_daemon, &b_daemon],
    )
    .await;
    assert_eq!(n["content"], json!("after restart"));
    assert!(
        bridge.child.try_wait().expect("waitable").is_none(),
        "the bridge never exited"
    );
}

/// The human client's envelope, as its facade sends it, reaches the bridge
/// as the envelope's own JSON text, compressed or not (CHANNEL-EVENT.md
/// §Content; the facade far end Stage 16's close carried as a deviation).
/// The facade compresses an envelope past the payload limit to `;ce=br`
/// (ADR-0050), and the bridge decodes it before classifying it, so the
/// model reads the same text either way, and `meta.content_type` is the
/// media type without its `ce` parameter. The plain envelope is the
/// control for the compressed one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_human_chat_envelope_is_notified_as_its_decoded_text() {
    use interweave_human_chat_protocol::{
        HumanChatV2, MEDIA_TYPE_V2, MessageKind, encode_outbound,
    };
    let (a, a_daemon, a_peer, b, b_daemon, _b_peer) = two_daemons().await;
    let daemons = [&a_daemon, &b_daemon];
    let mut bridge = Bridge::start(&a);
    bridge.wait_leased(&daemons).await;
    let b_human = session(&b, human()).await;

    let envelope = HumanChatV2 {
        v: 2,
        kind: MessageKind::Text,
        app_message_id: "0123456789abcdef0123456789abcdef".to_owned(),
        // Repetitive, so it compresses well below half its size.
        text: "the same line again, ".repeat(400),
        reply_to: None,
        sent_at_ms: None,
        from_endpoint: None,
    };
    let raw = serde_json::to_string(&envelope).expect("an envelope serializes");
    let plain = encode_outbound(&envelope, MAX_PAYLOAD_BYTES).expect("fits as it is");
    let compressed = encode_outbound(&envelope, raw.len() / 2).expect("fits compressed");
    assert_eq!(
        plain.media_type, MEDIA_TYPE_V2,
        "the control is uncompressed"
    );
    assert!(
        compressed.media_type.ends_with(";ce=br") && compressed.bytes.len() < raw.len() / 2,
        "{}",
        compressed.media_type
    );

    for encoded in [plain, compressed] {
        let payload = Payload::new(
            Some(MediaType::parse(&encoded.media_type).expect("a media type")),
            encoded.bytes.clone(),
            MAX_PAYLOAD_BYTES,
        )
        .expect("a payload");
        send_payload_until_accepted(&b_human, &a_peer, claude(), &payload, &daemons).await;
        let n = bridge.notification().await;
        assert_eq!(n["content"], json!(raw), "{}", encoded.media_type);
        assert_eq!(n["meta"]["payload_encoding"], json!("utf8"));
        assert_eq!(n["meta"]["content_type"], json!(MEDIA_TYPE_V2));
    }
}

/// The human client's own facade on a home's daemon: its store and its
/// IPC binding, leased on that daemon's `human`, driven as a composition
/// root drives it.
struct Facade {
    client: interweave_human_transport_client::TransportClient<
        interweave_ipc_client::IpcBinding,
        interweave_ipc_client::IpcBinding,
    >,
    received: Vec<interweave_human_transport_client::Received>,
    /// Each route the facade was told of, in order: the peer and the path.
    paths: Vec<(TransportIdentity, interweave_transport_api::PeerPath)>,
    _store_dir: tempfile::TempDir,
}

impl Facade {
    fn new(home: &Home) -> Self {
        use interweave_human_store::{HumanStore, StoreOptions};
        use interweave_human_transport_client::{ClientConfig, TransportClient};
        use std::os::unix::fs::PermissionsExt as _;
        // Owner-only whatever the umask, which the store asks of every
        // ancestor it makes (ADR-0028 A 2026-10-08).
        let store_dir = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .expect("a store directory");
        let store = HumanStore::open(
            &store_dir.path().join("state/human.sqlite"),
            StoreOptions::default(),
        )
        .expect("a store");
        let client = TransportClient::new(
            home.binding(),
            home.binding(),
            store,
            ClientConfig {
                client_kind: "human-client".to_owned(),
                endpoints: vec![human()],
                channels: vec![],
                max_payload_bytes: MAX_PAYLOAD_BYTES,
            },
            Box::new(wall_ms),
            wall_ms(),
        )
        .expect("an empty store's pending rows");
        Self {
            client,
            received: Vec::new(),
            paths: Vec::new(),
            _store_dir: store_dir,
        }
    }

    /// One turn of the root's loop: tick, then drain what arrived.
    async fn step(&mut self) {
        let now = wall_ms();
        Box::pin(self.client.tick(now)).await;
        let drained = Box::pin(self.client.drain(64, now)).await;
        self.received.extend(drained);
        while let Some(event) = self.client.next_event() {
            if let interweave_human_transport_client::ClientEvent::PeerPath { peer, path } = event {
                self.paths.push((peer, path));
            }
        }
    }

    async fn ready(&mut self, daemons: &[&Daemon]) {
        use interweave_human_transport_client::SessionState;
        let deadline = tokio::time::Instant::now() + PATIENCE;
        while !matches!(self.client.session_state(), SessionState::Ready { .. }) {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the facade never leased its daemon's human\n{}",
                logs(daemons)
            );
            self.step().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

fn wall_ms() -> u64 {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock")
            .as_millis(),
    )
    .expect("a Unix time in ms")
}

/// A text envelope; with `raw_len`, its text padded so its JSON is
/// exactly that many bytes.
fn chat(
    serial: u8,
    text: &str,
    raw_len: Option<usize>,
) -> interweave_human_chat_protocol::HumanChatV2 {
    use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
    let mut envelope = HumanChatV2 {
        v: 2,
        kind: MessageKind::Text,
        app_message_id: format!("{serial:02x}").repeat(16),
        text: text.to_owned(),
        reply_to: None,
        sent_at_ms: None,
        from_endpoint: None,
    };
    if let Some(len) = raw_len {
        let base = serde_json::to_string(&envelope).expect("serializes").len();
        // Repetitive and escape-free, so the JSON grows byte for byte and
        // brotli brings it far under the payload limit.
        let pad = "the same line again, ".repeat(len / 21 + 1);
        envelope.text.push_str(&pad[..len - base]);
        assert_eq!(
            serde_json::to_string(&envelope).expect("serializes").len(),
            len
        );
    }
    envelope
}

/// The human client's facade as the far end (plan §21 C-05, the Stage 16
/// carry). Two messages go from the facade on B to the bridge on A. The
/// first is plain. The second's JSON is exactly
/// `MAX_DECOMPRESSED_BYTES`: the largest the facade may send, and the
/// most the bridge decodes. The facade's pending rows show it chose plain
/// for the first and `;ce=br` for the second. The bridge forwards each as
/// the envelope's own JSON, byte for byte, and `meta.content_type` is the
/// media type without `ce`. Then the model replies to the facade twice:
/// once as `HumanChatV2`, which the facade stores and hands over as from
/// A's `claude`, and once as plain text, which the facade presents
/// nothing of and counts as an unknown media type. The plain text is the
/// control that the facade is not taking whatever arrives.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_human_clients_facade_exchanges_with_the_bridge_plain_and_compressed() {
    let (a, a_daemon, a_peer, b, b_daemon, b_peer) = two_daemons().await;
    let daemons = [&a_daemon, &b_daemon];
    let mut bridge = Bridge::start(&a);
    bridge.wait_leased(&daemons).await;
    let mut facade = Facade::new(&b);
    facade.ready(&daemons).await;
    exchange_with_the_facade(&mut bridge, &mut facade, &a_peer, &b_peer, &daemons).await;
}

/// How long a dial in the second network is given. A dial toward the
/// other side's private address ends sooner, on the core's unreachable
/// answer, so this bounds only a dial that hangs.
#[cfg(target_os = "linux")]
const DIAL: Duration = Duration::from_secs(3);

#[cfg(target_os = "linux")]
const SECOND_NETWORK: &str =
    "the_bridge_behind_its_own_nat_exchanges_with_the_facade_through_the_relay";

/// The bridge on a second network (plan section 21, decided (7); C-05's
/// run R7, the bridge's side). The bridge and its daemon sit in B's
/// namespace, behind NAT-B. The human client's facade and its daemon sit
/// in A's, behind NAT-A, and the relay is on the core between them
/// (`interweave_test_support::e2e::netns`). B reserves on the relay, and
/// A's only route to B is a circuit. Neither daemon's listen address
/// takes a dial from the other side, since the core routes neither
/// private range. The control is that each takes one from its own
/// namespace, so the refusal is the topology's and not a listener that
/// is not up. Then the facade exchange of the one-host test runs, the
/// facade is told its route to the bridge's peer began relayed, and the
/// relay reports carrying A's circuit to B.
///
/// What this does not prove: that a NAT refuses a new inbound
/// connection, since the core routes neither private range and so no
/// dial reaches a NAT's forward rule; a NAT whose mapping depends on the
/// destination; a hole punch; a second host.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_bridge_behind_its_own_nat_exchanges_with_the_facade_through_the_relay() {
    use std::net::SocketAddrV4;

    use common::netns::{self, Net, NetRelay};
    use common::relayed_example;

    if !netns::in_second_network(SECOND_NETWORK) {
        return;
    }
    let (a, b) = (Home::new("human-desktop"), Home::new("human-desktop"));
    let (a_peer, b_peer) = (a.write_key(), b.write_key());
    let relay = NetRelay::start(&[&a_peer, &b_peer]);
    let a_port = netns::in_net(Net::A, || free_port(netns::A));
    let b_port = netns::in_net(Net::B, || free_port(netns::B));
    b.write_config(&relayed_example(
        &[&a_peer],
        &format!("/ip4/{}/tcp/{b_port}", netns::B),
        relay.relay(),
        None,
    ));
    a.write_config(&relayed_example(
        &[&b_peer],
        &format!("/ip4/{}/tcp/{a_port}", netns::A),
        relay.relay(),
        Some(&relay.relay().circuit_to(&b_peer)),
    ));
    let pid = |p: &TransportIdentity| -> libp2p::PeerId { p.as_str().parse().expect("a peer id") };

    let mut b_daemon = b.start_in(Net::B, &[]);
    b_daemon.serving(&b).await;
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

    // A daemon serves its IPC sockets before its network listener binds,
    // so each listener is waited for before any dial is judged.
    let at = SocketAddrV4::new;
    for (net, own) in [
        (Net::A, at(netns::A, a_port)),
        (Net::B, at(netns::B, b_port)),
    ] {
        let deadline = tokio::time::Instant::now() + PATIENCE;
        while !netns::tcp_reaches(net, own, DIAL) {
            assert!(
                tokio::time::Instant::now() < deadline,
                "{net:?} never listened on {own}\n{}",
                logs(&[&a_daemon, &b_daemon])
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    assert!(
        !netns::tcp_reaches(Net::A, at(netns::B, b_port), DIAL),
        "the facade's side dialled the bridge's daemon directly"
    );
    assert!(
        !netns::tcp_reaches(Net::B, at(netns::A, a_port), DIAL),
        "the bridge's side dialled the facade's daemon directly"
    );

    {
        let daemons = [&a_daemon, &b_daemon];
        let mut bridge = Bridge::start_in_net(&b, Net::B);
        bridge.wait_leased(&daemons).await;
        let mut facade = Facade::new(&a);
        facade.ready(&daemons).await;
        exchange_with_the_facade(&mut bridge, &mut facade, &b_peer, &a_peer, &daemons).await;
        assert_eq!(
            facade
                .paths
                .iter()
                .find(|(peer, _)| peer == &b_peer)
                .map(|(_, path)| *path),
            Some(interweave_transport_api::PeerPath::Relayed),
            "the facade's route to the bridge's peer began relayed: {:?}",
            facade.paths
        );
        let seen = relay.relay().seen().await;
        assert!(
            seen.circuits.contains(&(pid(&a_peer), pid(&b_peer))),
            "the relay carried A's circuit to B: {seen:?}"
        );
    }

    let a_stopped = a_daemon.terminate().await.success();
    let b_stopped = b_daemon.terminate().await.success();
    assert!(a_stopped && b_stopped, "{}", logs(&[&a_daemon, &b_daemon]));
}

/// The exchange both facade tests run, the bridge on `bridge_peer` and
/// the facade on `facade_peer`, both already up: two envelopes to the
/// bridge, plain and at the decoded ceiling, then two replies to the
/// facade (`the_human_clients_facade_exchanges_with_the_bridge_plain_and_compressed`).
async fn exchange_with_the_facade(
    bridge: &mut Bridge,
    facade: &mut Facade,
    bridge_peer: &TransportIdentity,
    facade_peer: &TransportIdentity,
    daemons: &[&Daemon],
) {
    use interweave_human_chat_protocol::{HumanChatV2, MAX_DECOMPRESSED_BYTES, MEDIA_TYPE_V2};
    use interweave_human_transport_client::{Destination, Origin};

    let to_claude = Destination::Direct {
        peer: bridge_peer.clone(),
        endpoint: Some(claude()),
    };

    let plain = chat(1, "hello claude, from the facade", None);
    let edge = chat(2, "", Some(MAX_DECOMPRESSED_BYTES));
    let mut token = None;
    for (envelope, at_the_edge) in [(&plain, false), (&edge, true)] {
        let row = Box::pin(facade.client.send(to_claude.clone(), envelope, wall_ms()))
            .await
            .expect("the facade commits it");
        // How the facade chose to send it, read from its pending row
        // before the first tick attempts it: once accepted the row is gone.
        let chosen = facade
            .client
            .store_mut()
            .pending_outbound_row(row)
            .expect("the store reads")
            .expect("pending until attempted")
            .media_type
            .map(|m| m.as_str().to_owned());
        let expected = if at_the_edge {
            format!("{MEDIA_TYPE_V2};ce=br")
        } else {
            MEDIA_TYPE_V2.to_owned()
        };
        assert_eq!(
            chosen.as_deref(),
            Some(expected.as_str()),
            "the facade's encoding"
        );
        let deadline = tokio::time::Instant::now() + PATIENCE * 3;
        let n = loop {
            facade.step().await;
            let _ = bridge.status().await;
            if let Some(at) = bridge.notifications.iter().position(|(n, _)| {
                HumanChatV2::parse(n["content"].as_str().unwrap_or(""))
                    .is_ok_and(|got| got.app_message_id == envelope.app_message_id)
            }) {
                let (n, line) = bridge.notifications.remove(at).expect("held");
                meta_is_the_contracts(&n["meta"], &line);
                break n;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the bridge was never notified of {}\n{}",
                envelope.app_message_id,
                logs(daemons)
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        };
        let content = n["content"].as_str().expect("text content");
        assert_eq!(
            content,
            serde_json::to_string(envelope).expect("serializes"),
            "the envelope's JSON, byte for byte"
        );
        let meta = &n["meta"];
        assert_eq!(meta["delivery_mode"], json!("direct"));
        assert_eq!(meta["source_peer"], json!(facade_peer.as_str()));
        assert_eq!(meta["source_endpoint"], json!("human"));
        assert_eq!(meta["destination_endpoint"], json!("claude"));
        assert_eq!(meta["payload_encoding"], json!("utf8"));
        assert_eq!(meta["content_type"], json!(MEDIA_TYPE_V2));
        token = Some(meta["reply_token"].as_str().expect("a token").to_owned());
    }
    let token = token.expect("a token");

    let reply = chat(3, "hello facade, from claude", None);
    let (answer, error) = bridge
        .tool(
            "reply",
            json!({
                "reply_token": token,
                "content": serde_json::to_string(&reply).expect("serializes"),
                "content_type": MEDIA_TYPE_V2,
            }),
        )
        .await;
    assert!(!error, "{answer}\n{}", logs(daemons));
    assert_eq!(answer, "remote transport accepted at endpoint human");
    let (answer, error) = bridge
        .tool(
            "reply",
            json!({"reply_token": token, "content": "plain words, no envelope"}),
        )
        .await;
    assert!(!error, "{answer}");

    // Two direct sends are two streams, so either may land first: wait
    // for both.
    let deadline = tokio::time::Instant::now() + PATIENCE * 3;
    while facade.received.is_empty()
        || facade.client.diagnostics().malformed_unknown_media_type == 0
    {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the replies never both arrived ({} received, {:?})\n{}",
            facade.received.len(),
            facade.client.diagnostics(),
            logs(daemons)
        );
        facade.step().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(
        facade.received.len(),
        1,
        "the plain reply is presented as nothing"
    );
    let got = &facade.received[0];
    assert_eq!(got.envelope, reply, "stored and handed over as composed");
    assert_eq!(
        got.origin,
        Origin::Direct {
            peer: bridge_peer.clone(),
            endpoint: claude(),
        }
    );
    let diagnostics = facade.client.diagnostics();
    assert_eq!(
        (
            diagnostics.malformed_unknown_media_type,
            diagnostics.malformed_undecodable,
            diagnostics.malformed_invalid_envelope
        ),
        (1, 0, 0),
        "only the plain reply is malformed"
    );
}

/// A plain MCP host -- no channel extension -- drives the bridge in pull
/// mode across two daemons (architect-cto's ruling, relay seq 18691):
/// `initialize` advertises no `claude/channel` and `tools/list` adds
/// `receive`; B's direct and broadcast reach the host only through
/// `receive`, oldest first, each with the meta the push would carry;
/// nothing is pushed; and a reply by its token reaches B. The push-mode
/// tests above, on the same harness, are the control.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_plain_mcp_host_receives_by_pull_across_two_daemons() {
    let (a, a_daemon, a_peer, b, b_daemon, b_peer) = two_daemons().await;
    let daemons = [&a_daemon, &b_daemon];
    let mut bridge = Bridge::start_pull(&a);
    let init = bridge
        .request(
            "initialize",
            json!({"protocolVersion": "2025-11-25", "capabilities": {}}),
        )
        .await;
    assert!(
        init["result"]["capabilities"].get("experimental").is_none(),
        "no channel extension advertised: {init}"
    );
    let tools = bridge.request("tools/list", json!({})).await;
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names.len(), 8, "{names:?}");
    assert!(names.contains(&"receive"));
    bridge.wait_leased(&daemons).await;

    let b_human = session(&b, human()).await;
    send_until_accepted(&b_human, &a_peer, claude(), "pull one", &daemons).await;
    let (answer, error) = bridge.tool("join", json!({"channel": "general"})).await;
    assert!(!error, "{answer}");
    b_human.join(general()).await.expect("B joins");

    // Received in order: the direct first, then the broadcast once the
    // mesh carries it.
    let mut received: Vec<Value> = Vec::new();
    let deadline = tokio::time::Instant::now() + PATIENCE * 2;
    while !received.iter().any(|e| e["content"] == json!("pull all")) {
        b_human
            .broadcast(
                general(),
                BroadcastMessageV1 {
                    message_id: MessageId::from_bytes(rand_bytes()),
                    sent_at_ms: 0,
                    payload: text("pull all"),
                },
            )
            .await
            .expect("accepted for local publish");
        // How a host sees the answer: one text content item holding the
        // structured JSON, not an error (relay seq 18784 asks it recorded).
        let framed = bridge
            .request("tools/call", json!({"name": "receive", "arguments": {}}))
            .await;
        assert_eq!(framed["result"]["isError"], json!(false), "{framed}");
        assert_eq!(
            framed["result"]["content"][0]["type"],
            json!("text"),
            "{framed}"
        );
        let answer = framed["result"]["content"][0]["text"]
            .as_str()
            .expect("text")
            .to_owned();
        let take: Value = serde_json::from_str(&answer).expect("receive is JSON");
        assert_eq!(take["paused"], json!(false), "{take}");
        received.extend(take["events"].as_array().expect("events").iter().cloned());
        assert!(
            tokio::time::Instant::now() < deadline,
            "no broadcast reached the host by pull\n{}",
            logs(&daemons)
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let direct = &received[0];
    assert_eq!(
        direct["kind"],
        json!("direct"),
        "oldest first: {received:?}"
    );
    assert_eq!(direct["content"], json!("pull one"));
    assert_eq!(direct["meta"]["delivery_mode"], json!("direct"));
    assert_eq!(direct["meta"]["source_peer"], json!(b_peer.as_str()));
    assert_eq!(direct["meta"]["destination_endpoint"], json!("claude"));
    let broadcast = received
        .iter()
        .find(|e| e["content"] == json!("pull all"))
        .expect("the broadcast");
    assert_eq!(broadcast["kind"], json!("broadcast"));
    assert_eq!(broadcast["meta"]["channel"], json!("general"));
    assert!(
        bridge.notifications.is_empty(),
        "pull mode pushes nothing: {:?}",
        bridge.notifications
    );

    let token = direct["meta"]["reply_token"]
        .as_str()
        .expect("a token")
        .to_owned();
    let (answer, error) = bridge
        .tool(
            "reply",
            json!({"reply_token": token, "content": "pull back"}),
        )
        .await;
    assert!(!error, "{answer}");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while !direct_from(&b_human, b"pull back").await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the reply never reached B"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
