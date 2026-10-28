// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The command-line human client against real daemons (j41; plan section
//! 29 step 4's command half): every batch-1 verb run as the shipped
//! binary, as a script runs it, with its exit code and output read back.
//! Two daemons from the shipped desktop example, each the other's trusted
//! peer; B's client is `listen` when B must receive.
//!
//! What this does not show: a person reading the output (nothing here
//! looks at a terminal), and another host. Run with
//! `INTERWEAVE_PACKAGE_BIN` naming an installed package's `bin/`, it
//! runs that package's `interweave-human`.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::io::{BufRead as _, BufReader};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use interweave_transport_api::TransportIdentity;
use serde_json::Value;

use common::{Daemon, Home, PATIENCE, example, free_port, workspace_binary};

/// The binary under test: the installed package's when
/// `INTERWEAVE_PACKAGE_BIN` names one, as `human_app`'s cases take theirs
/// (`human_app/package.rs`), else the workspace's own. A package without
/// it fails by name, never falls back to the workspace: that would test
/// a build the package does not ship.
fn binary() -> std::path::PathBuf {
    match std::env::var_os("INTERWEAVE_PACKAGE_BIN") {
        Some(dir) => {
            let bin = std::path::Path::new(&dir).join("interweave-human");
            assert!(
                bin.is_file(),
                "INTERWEAVE_PACKAGE_BIN names {}, which holds no interweave-human",
                std::path::Path::new(&dir).display()
            );
            bin
        }
        None => workspace_binary("interweave-human", "interweave-human-cli"),
    }
}

/// `interweave-human` for `home`'s profile, with its environment cleared
/// and the home's XDG roots set, as the harness runs `transportctl`.
fn command(home: &Home, args: &[&str]) -> Command {
    let mut command = Command::new(binary());
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

/// Run a verb to its end, `input` on stdin.
fn human(home: &Home, args: &[&str], input: &str) -> Output {
    use std::io::Write as _;
    let mut child = command(home, args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("interweave-human runs");
    let _ = child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes());
    child.wait_with_output().expect("interweave-human ends")
}

fn code(out: &Output) -> Option<i32> {
    out.status.code()
}

fn said(out: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn lines(out: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{l:?}: {e}")))
        .collect()
}

/// `listen --json` running for a profile, its lines read as they come.
struct Listener {
    child: Child,
    lines: mpsc::Receiver<Value>,
}

impl Listener {
    fn start(home: &Home) -> Self {
        Self::with(home, &["listen", "--json"]).0
    }

    /// `listen` with `args`, its stdin piped for `--input` lines.
    fn with(home: &Home, args: &[&str]) -> (Self, std::process::ChildStdin) {
        let mut child = command(home, args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("listen runs");
        let stdin = child.stdin.take().expect("stdin");
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
        (Self { child, lines }, stdin)
    }

    /// The first line `matches` takes, within `PATIENCE`.
    fn until(&self, what: &str, matches: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + PATIENCE;
        let mut seen = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) if matches(&line) => return line,
                Ok(line) => seen.push(line),
                Err(e) => panic!("listen never printed {what}: {e}; it printed {seen:?}"),
            }
        }
    }

    /// Stop it as a service manager would, and wait for it: exit 0.
    fn stop(mut self) {
        let pid = self.child.id().to_string();
        let killed = Command::new("kill")
            .args(["-TERM", &pid])
            .status()
            .expect("kill runs");
        assert!(killed.success());
        let status = self.child.wait().expect("listen ends");
        assert_eq!(status.code(), Some(0), "listen ends cleanly on SIGTERM");
    }
}

/// A case that fails before `stop` leaves no `listen` running: it goes on
/// after stdin ends, so nothing else would end it.
impl Drop for Listener {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Until `daemon` has logged that its runtime serves.
async fn runtime_up(daemon: &mut Daemon) {
    let deadline = Instant::now() + PATIENCE;
    while !daemon.log().contains("serving") {
        if let Some(status) = daemon.child.try_wait().expect("a status") {
            panic!("the daemon exited ({status}):\n{}", daemon.log());
        }
        assert!(Instant::now() < deadline, "no runtime:\n{}", daemon.log());
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Two daemons from the shipped desktop example, each with a static
/// route to the other; B started once A's runtime is up, so B's start-up
/// dial finds it listening.
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
    let route_to =
        |port: u16, peer: &TransportIdentity| format!("{}/p2p/{}", at(port), peer.as_str());
    a.write_config(&example(
        "human-desktop.yaml",
        &b_peer,
        &at(a_port),
        Some(&route_to(b_port, &b_peer)),
    ));
    b.write_config(&example(
        "human-desktop.yaml",
        &a_peer,
        &at(b_port),
        Some(&route_to(a_port, &a_peer)),
    ));
    let mut a_daemon = a.start(&[]);
    a_daemon.serving(&a).await;
    runtime_up(&mut a_daemon).await;
    let mut b_daemon = b.start(&[]);
    b_daemon.serving(&b).await;
    runtime_up(&mut b_daemon).await;
    (a, a_daemon, a_peer, b, b_daemon, b_peer)
}

/// Send until accepted: the two daemons' connection may still be coming
/// up, and a send that ends `--wait` still on its way is exit 6, with its
/// id to cancel before the next try.
fn send_accepted(a: &Home, to: &str, text: &str) -> String {
    let deadline = Instant::now() + PATIENCE;
    loop {
        let out = human(a, &["--wait", "3000", "send", to, "-"], text);
        let printed = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        if code(&out) == Some(0) {
            let (id, status) = printed.split_once(' ').expect("id and status");
            assert_eq!(status, "accepted", "{}", said(&out));
            return id.to_owned();
        }
        assert_eq!(code(&out), Some(6), "{}", said(&out));
        let id = printed.split_whitespace().next().expect("the id printed");
        let cancelled = human(a, &["cancel", id], "");
        assert_eq!(code(&cancelled), Some(0), "{}", said(&cancelled));
        assert!(Instant::now() < deadline, "never accepted: {}", said(&out));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_batch_one_verb_against_two_daemons() {
    let (a, mut a_daemon, a_peer, b, mut b_daemon, b_peer) = two_daemons().await;
    let to_b = format!("{}/human", b_peer.as_str());

    // status: ready, on the profile's human endpoint.
    let out = human(&a, &["status", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let status = &lines(&out)[0];
    assert_eq!(status["session"], "ready", "{}", said(&out));
    assert_eq!(status["endpoint"], "human");

    // identities: the human endpoint, enabled; disabled, enabled and made
    // the default again over the admin socket.
    let out = human(&a, &["identities", "list", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let rows = lines(&out);
    assert_eq!(rows.len(), 1, "{}", said(&out));
    assert_eq!(rows[0]["endpoint"], "human");
    assert_eq!(rows[0]["enabled"], true);
    for args in [
        ["identities", "disable", "human"],
        ["identities", "enable", "human"],
        ["identities", "default", "human"],
    ] {
        let out = human(&a, &args, "");
        assert_eq!(code(&out), Some(0), "{args:?}: {}", said(&out));
    }
    let out = human(&a, &["identities", "list", "--json"], "");
    assert_eq!(lines(&out)[0]["enabled"], true, "{}", said(&out));
    assert_eq!(lines(&out)[0]["default"], true, "{}", said(&out));
    // Another client's default is not this client's to clear.
    let set = a.transportctl(&["endpoints", "default", "claude"], "");
    assert!(set.status.success(), "{set:?}");
    let out = human(&a, &["identities", "default", "--none"], "");
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    let listed = a.transportctl(&["endpoints", "list", "--json"], "");
    let listed: Value = serde_json::from_slice(&listed.stdout).expect("the list");
    assert!(
        listed["endpoints"]
            .as_array()
            .expect("rows")
            .iter()
            .any(|row| row["id"] == "claude" && row["default"] == true),
        "claude is still the default: {listed}"
    );
    let out = human(&a, &["identities", "default", "human"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));

    // trust: B is configured; revoked and allowed again.
    let out = human(&a, &["trust", "list", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert!(
        lines(&out).iter().any(|r| r["peer"] == b_peer.as_str()),
        "{}",
        said(&out)
    );
    for verb in ["revoke", "allow"] {
        let out = human(&a, &["trust", verb, b_peer.as_str()], "");
        assert_eq!(code(&out), Some(0), "{verb}: {}", said(&out));
    }

    // send to B while B's client listens; listen prints it, unread.
    let listener = Listener::start(&b);
    listener.until("the session ready", |l| {
        l["event"] == "session" && l["session"] == "ready"
    });
    // One client of B's store at a time: a second is exit 4.
    let out = human(&b, &["status"], "");
    assert_eq!(code(&out), Some(4), "{}", said(&out));
    send_accepted(&a, &to_b, "hello from the command line");
    let heard = listener.until("the message", |l| l["event"] == "message");
    assert_eq!(heard["text"], "hello from the command line");
    assert_eq!(heard["status"], "unread");
    listener.stop();

    // conversations, messages, read with --keep; unkeep.
    let out = human(&b, &["conversations", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let conversation = lines(&out)[0]["conversation"]
        .as_str()
        .expect("a conversation")
        .to_owned();
    assert_eq!(lines(&out)[0]["unread"], 1);
    let out = human(&b, &["messages", &conversation, "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    assert_eq!(lines(&out)[0]["status"], "unread");
    let out = human(&b, &["read", &conversation, "--keep", &id, "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(lines(&out)[0]["status"], "kept", "{}", said(&out));
    let out = human(&b, &["messages", &conversation, "--json"], "");
    let kept = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    assert!(kept.starts_with('k'), "the next run sees it kept: {kept}");
    let out = human(&b, &["unkeep", &kept], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let out = human(&b, &["messages", &conversation, "--json"], "");
    assert!(lines(&out).is_empty(), "unkept is gone: {}", said(&out));

    // With no client of B's leasing its endpoint, a send stays on its way:
    // exit 6, its id printed; cancel takes it, and retry sends one once B
    // listens again.
    let out = human(&a, &["--wait", "1500", "send", &to_b, "nobody listens"], "");
    assert_eq!(code(&out), Some(6), "{}", said(&out));
    let stuck = String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .expect("the id")
        .to_owned();
    let out = human(&a, &["cancel", &stuck], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let out = human(&a, &["--wait", "1500", "send", &to_b, "later"], "");
    assert_eq!(code(&out), Some(6), "{}", said(&out));
    let later = String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .expect("the id")
        .to_owned();
    let listener = Listener::start(&b);
    listener.until("the session ready", |l| {
        l["event"] == "session" && l["session"] == "ready"
    });
    let out = human(&a, &["--wait", "15000", "retry", &later], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("{later} accepted\n")
    );
    listener.stop();

    // store health.
    let out = human(&b, &["store", "health", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(lines(&out)[0]["health"], "healthy");

    // A usage error is 2; with A's daemon gone, status says so and a send
    // is exit 3.
    assert_eq!(code(&human(&a, &["launch"], "")), Some(2));
    a_daemon.terminate().await;
    let out = human(&a, &["status", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(lines(&out)[0]["session"], "no-daemon", "{}", said(&out));
    let out = human(&a, &["send", &to_b, "no daemon"], "");
    assert_eq!(code(&out), Some(3), "{}", said(&out));
    let _ = a_peer;
    b_daemon.terminate().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_card_shared_accepted_edited_and_deleted_across_two_daemons() {
    let (a, mut a_daemon, a_peer, b, mut b_daemon, b_peer) = two_daemons().await;
    let to_b = format!("{}/human", b_peer.as_str());
    let to_a = format!("{}/human", a_peer.as_str());

    // A's own card, its key made at A's first run.
    let out = human(&a, &["card", "set", "Ada", "Lovelace"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let out = human(&a, &["card", "show", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let card = &lines(&out)[0];
    assert_eq!(card["name"], "Ada Lovelace");
    let fingerprint = card["fingerprint"]
        .as_str()
        .expect("fingerprint")
        .to_owned();

    // Shared while B listens: B hears it as a proposal, unread.
    let listener = Listener::start(&b);
    listener.until("the session ready", |l| {
        l["event"] == "session" && l["session"] == "ready"
    });
    let deadline = Instant::now() + PATIENCE;
    loop {
        let out = human(
            &a,
            &["--wait", "3000", "card", "share", &to_b, "--json"],
            "",
        );
        if code(&out) == Some(0) {
            assert_eq!(lines(&out)[0]["status"], "accepted", "{}", said(&out));
            break;
        }
        assert_eq!(code(&out), Some(6), "{}", said(&out));
        let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
        assert_eq!(code(&human(&a, &["cancel", &id], "")), Some(0));
        assert!(Instant::now() < deadline, "never accepted: {}", said(&out));
    }
    let heard = listener.until("the proposal", |l| l["event"] == "proposal");
    assert_eq!(heard["name"], "Ada Lovelace");
    assert_eq!(heard["peer"], a_peer.as_str());
    assert_eq!(heard["fingerprint"], fingerprint.as_str());
    listener.stop();

    // Listed without being read, then read and accepted in one run.
    let out = human(&b, &["proposals", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    let out = human(
        &b,
        &["proposals", "read", &to_a, "--accept", &id, "--json"],
        "",
    );
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(lines(&out)[0]["status"], "accepted", "{}", said(&out));
    let out = human(&b, &["proposals", "--json"], "");
    assert!(lines(&out).is_empty(), "decided: {}", said(&out));

    // The contact, its route and key; renamed, then deleted.
    let out = human(&b, &["contacts", "--json"], "");
    let contact = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    let out = human(&b, &["contacts", "show", &contact, "--json"], "");
    let shown = &lines(&out)[0];
    assert_eq!(shown["name"], "Ada Lovelace");
    assert_eq!(shown["routes"][0]["peer"], a_peer.as_str());
    assert_eq!(shown["routes"][0]["fingerprint"], fingerprint.as_str());
    // A second card under another name, declined before any edit of the
    // contact: an accept would make the contact's name the new one.
    let out = human(&a, &["card", "set", "Augusta", "King"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let listener = Listener::start(&b);
    listener.until("the session ready", |l| {
        l["event"] == "session" && l["session"] == "ready"
    });
    let out = human(&a, &["--wait", "15000", "card", "share", &to_b], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    listener.until("the second proposal", |l| l["event"] == "proposal");
    listener.stop();
    let out = human(&b, &["proposals", "--json"], "");
    let second = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    let out = human(
        &b,
        &["proposals", "read", &to_a, "--decline", &second, "--json"],
        "",
    );
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(lines(&out)[0]["status"], "declined", "{}", said(&out));
    let out = human(&b, &["proposals", "--json"], "");
    assert!(
        lines(&out).is_empty(),
        "the declined card is gone: {}",
        said(&out)
    );
    let out = human(&b, &["contacts", "--json"], "");
    assert_eq!(
        lines(&out).len(),
        1,
        "declining made no contact: {}",
        said(&out)
    );
    let out = human(&b, &["contacts", "show", &contact, "--json"], "");
    assert_eq!(
        lines(&out)[0]["routes"][0]["fingerprint"],
        fingerprint.as_str(),
        "nor changed the route: {}",
        said(&out)
    );

    let out = human(&b, &["contacts", "show", &contact, "--json"], "");
    assert_eq!(
        lines(&out)[0]["name"],
        "Ada Lovelace",
        "declined, not accepted: {}",
        said(&out)
    );
    assert_eq!(lines(&out)[0]["name_from"], "card", "{}", said(&out));

    let out = human(&b, &["contacts", "rename", &contact, "Countess"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let out = human(&b, &["contacts", "show", &contact, "--json"], "");
    assert_eq!(lines(&out)[0]["name_from"], "own", "{}", said(&out));
    let out = human(&b, &["contacts", "photo", &contact, "--none"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let out = human(&b, &["contacts", "show", &contact, "--json"], "");
    assert_eq!(lines(&out)[0]["photo_from"], "own", "{}", said(&out));

    let out = human(&b, &["contacts", "delete", &contact], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let out = human(&b, &["contacts", "--json"], "");
    assert!(lines(&out).is_empty(), "deleted: {}", said(&out));

    a_daemon.terminate().await;
    b_daemon.terminate().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_peer_added_by_address_is_listed_refused_by_class_and_removed() {
    let (a, mut a_daemon, a_peer, _b, mut b_daemon, b_peer) = two_daemons().await;
    let ip = interweave_test_support::net::require_private_interface_v4();
    let address = format!("/ip4/{ip}/tcp/4999/p2p/{}", b_peer.as_str());

    // This node's own addresses, as transportctl prints them, line for line.
    let out = human(&a, &["peers", "show"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let shown = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(!shown.is_empty(), "a listening node has an address");
    assert!(
        shown
            .lines()
            .all(|l| l.ends_with(&format!("/p2p/{}", a_peer.as_str()))),
        "{shown}"
    );
    let ctl = a.transportctl(&["peers", "show"], "");
    assert_eq!(String::from_utf8_lossy(&ctl.stdout), shown, "{ctl:?}");
    let out = human(&a, &["peers", "show", "--json"], "");
    let own = &lines(&out)[0];
    assert_eq!(own["peer"], a_peer.as_str(), "{}", said(&out));
    assert_eq!(own["truncated"], false, "{}", said(&out));

    let out = human(&a, &["peers", "add", &address], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "added\n");
    let out = human(&a, &["peers", "add", &address], "");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "unchanged\n",
        "{}",
        said(&out)
    );
    let out = human(&a, &["peers", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert!(
        lines(&out)
            .iter()
            .any(|row| row["address"] == address.as_str() && row["source"] == "added"),
        "{}",
        said(&out)
    );
    // transportctl lists what the client added, as ADR-0062's gate asks.
    let listed = a.transportctl(&["peers", "addresses", "--json"], "");
    assert!(
        String::from_utf8_lossy(&listed.stdout).contains(&address),
        "{listed:?}"
    );

    let loopback = format!("/ip4/127.0.0.1/tcp/4999/p2p/{}", b_peer.as_str());
    let out = human(&a, &["peers", "add", &loopback], "");
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    let said_err = String::from_utf8_lossy(&out.stderr);
    assert!(said_err.contains("special_use"), "{said_err}");

    let out = human(&a, &["peers", "remove", &address], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "removed\n");
    let out = human(&a, &["peers", "--json"], "");
    assert!(
        !lines(&out)
            .iter()
            .any(|row| row["address"] == address.as_str()),
        "{}",
        said(&out)
    );

    a_daemon.terminate().await;
    b_daemon.terminate().await;
}

/// `peers state` names each peer A trusts, one line each from its dial
/// gate: B, whose daemon runs, connected and seen; C, trusted with no
/// daemon, not connected and never seen -- with no `last_seen` member,
/// since this daemon reports the time (j76, G8). Mutation: the
/// `last_seen` member written whatever the daemon reports.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peers_state_says_which_trusted_peer_is_connected_and_when_each_was_seen() {
    let (a, mut a_daemon, _a_peer, _b, mut b_daemon, b_peer) = two_daemons().await;
    let c_peer = Home::new("human-desktop").write_key();
    let out = human(&a, &["trust", "allow", c_peer.as_str()], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));

    let row = |rows: &[Value], peer: &TransportIdentity| {
        rows.iter()
            .find(|r| r["peer"] == peer.as_str())
            .cloned()
            .unwrap_or_else(|| panic!("no row for {}: {rows:?}", peer.as_str()))
    };
    let deadline = Instant::now() + PATIENCE;
    let rows = loop {
        let out = human(&a, &["peers", "state", "--json"], "");
        assert_eq!(code(&out), Some(0), "{}", said(&out));
        let rows = lines(&out);
        if row(&rows, &b_peer)["connected"] == true {
            break rows;
        }
        assert!(Instant::now() < deadline, "B never connected: {rows:?}");
        tokio::time::sleep(Duration::from_millis(200)).await;
    };
    assert_eq!(rows.len(), 2, "one row per trusted peer: {rows:?}");
    let b_row = row(&rows, &b_peer);
    assert!(b_row["last_seen_ms"].is_u64(), "{b_row}");
    assert!(b_row.get("last_seen").is_none(), "{b_row}");
    let c_row = row(&rows, &c_peer);
    assert_eq!(c_row["connected"], false, "{c_row}");
    assert_eq!(c_row["last_seen_ms"], Value::Null, "{c_row}");
    assert!(
        c_row.get("last_seen").is_none(),
        "never, not unreported: {c_row}"
    );

    let out = human(&a, &["peers", "state"], "");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        text.contains(&format!("{}  connected", b_peer.as_str())),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "{}  not connected, last seen never",
            c_peer.as_str()
        )),
        "{text}"
    );

    a_daemon.terminate().await;
    b_daemon.terminate().await;
}

/// Share A's card to `to_b` until B's daemon accepts it, B listening so
/// its store takes the card: the proposal B hears.
fn share_heard(a: &Home, b: &Home, to_b: &str) -> Value {
    let listener = Listener::start(b);
    listener.until("the session ready", |l| {
        l["event"] == "session" && l["session"] == "ready"
    });
    let deadline = Instant::now() + PATIENCE;
    loop {
        let out = human(a, &["--wait", "3000", "card", "share", to_b, "--json"], "");
        if code(&out) == Some(0) {
            break;
        }
        assert_eq!(code(&out), Some(6), "{}", said(&out));
        let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
        assert_eq!(code(&human(a, &["cancel", &id], "")), Some(0));
        assert!(Instant::now() < deadline, "never accepted: {}", said(&out));
    }
    let heard = listener.until("the proposal", |l| l["event"] == "proposal");
    listener.stop();
    heard
}

/// ADR-0063 decisions 4 and 6, and ADR-0059 decision 9's route deletion,
/// as a shell runs them across two daemons: A's identity shows its
/// fingerprint, B accepts A's card, A's key is damaged and A makes a new
/// one only on its confirmation flag, and B accepts the changed key only
/// with the flag named for it. B's contact has one route, A's, so its
/// deletion is refused as a contact delete and nothing changes: no flow
/// makes a second route yet (moving one is Stage 22's).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_key_change_is_made_and_accepted_only_on_its_flags_across_two_daemons() {
    let (a, mut a_daemon, a_peer, b, mut b_daemon, b_peer) = two_daemons().await;
    let to_b = format!("{}/human", b_peer.as_str());
    let to_a = format!("{}/human", a_peer.as_str());

    let out = human(&a, &["card", "set", "Ada"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let out = human(&a, &["identity", "show", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let shown = &lines(&out)[0];
    assert_eq!(shown["endpoint"], "human");
    let old = shown["fingerprint"]
        .as_str()
        .expect("fingerprint")
        .to_owned();
    let out = human(&a, &["card", "show", "--json"], "");
    assert_eq!(
        lines(&out)[0]["fingerprint"],
        old.as_str(),
        "one key, one fingerprint"
    );
    let out = human(&a, &["identity", "show", "claude"], "");
    assert_eq!(code(&out), Some(1), "not a human endpoint: {}", said(&out));
    let out = human(&a, &["identity", "new-key", "--confirm", "key-change"], "");
    assert_eq!(code(&out), Some(1), "a usable key stays: {}", said(&out));

    let heard = share_heard(&a, &b, &to_b);
    assert_eq!(heard["key"]["state"], "new");
    let id = heard["id"].as_str().expect("id").to_owned();
    let out = human(
        &b,
        &["proposals", "read", &to_a, "--accept-key-change", &id],
        "",
    );
    assert_eq!(
        code(&out),
        Some(1),
        "a first card is no key change: {}",
        said(&out)
    );
    let out = human(&b, &["proposals", "read", &to_a, "--accept", &id], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let out = human(&b, &["contacts", "--json"], "");
    let contact = lines(&out)[0]["id"].as_str().expect("id").to_owned();

    // The contact's only route: refused as a contact delete, unchanged.
    let out = human(&b, &["contacts", "route", "delete", &contact, &to_a], "");
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(&format!("contacts delete {contact}")),
        "{}",
        said(&out)
    );
    let out = human(&b, &["contacts", "show", &contact, "--json"], "");
    let routes = &lines(&out)[0]["routes"];
    assert_eq!(routes.as_array().map(Vec::len), Some(1), "{}", said(&out));
    assert_eq!(routes[0]["fingerprint"], old.as_str());

    // A's key file damaged: shown as such, and a new key made only once
    // confirmed.
    let file = a.paths.state_dir().join("identity-keys").join("human.key");
    std::fs::write(&file, "not a record\n").expect("damaged");
    let damaged = std::fs::read(&file).expect("read");
    let out = human(&a, &["identity", "show", "--json"], "");
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    assert_eq!(lines(&out)[0]["key_problem"], "damaged", "{}", said(&out));
    let out = human(&a, &["identity", "new-key", "--json"], "");
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    assert_eq!(lines(&out)[0]["deletes_old_key"], true, "{}", said(&out));
    assert_eq!(
        std::fs::read(&file).expect("read"),
        damaged,
        "nothing removed"
    );
    let out = human(
        &a,
        &["identity", "new-key", "--confirm", "key-change", "--json"],
        "",
    );
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let made = &lines(&out)[0];
    let new = made["fingerprint"]
        .as_str()
        .expect("fingerprint")
        .to_owned();
    assert_ne!(new, old);
    assert_eq!(made["share_again"][0], to_b.as_str(), "{}", said(&out));

    // B is told the key changed, and accepts it only with the flag.
    let heard = share_heard(&a, &b, &to_b);
    assert_eq!(heard["key"]["state"], "changed");
    assert_eq!(heard["key"]["recorded"], old.as_str());
    assert_eq!(heard["fingerprint"], new.as_str());
    let id = heard["id"].as_str().expect("id").to_owned();
    let out = human(&b, &["proposals", "read", &to_a, "--accept", &id], "");
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("--accept-key-change"),
        "{}",
        said(&out)
    );
    let out = human(&b, &["proposals", "--json"], "");
    assert_eq!(
        lines(&out)[0]["status"],
        "unread",
        "refused before the read"
    );
    let out = human(
        &b,
        &[
            "proposals",
            "read",
            &to_a,
            "--accept-key-change",
            &id,
            "--json",
        ],
        "",
    );
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(lines(&out)[0]["status"], "accepted", "{}", said(&out));
    let out = human(&b, &["contacts", "show", &contact, "--json"], "");
    assert_eq!(
        lines(&out)[0]["routes"][0]["fingerprint"],
        new.as_str(),
        "{}",
        said(&out)
    );

    a_daemon.terminate().await;
    b_daemon.terminate().await;
}

/// A list of addresses as a script pipes one in (ADR-0062, IPC 2.9's
/// `add_addresses`): added whole, again unchanged, and a list with one
/// refused line adds none of it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_list_of_addresses_is_added_whole_or_refused_by_its_line() {
    let (a, mut a_daemon, _a_peer, _b, mut b_daemon, b_peer) = two_daemons().await;
    let ip = interweave_test_support::net::require_private_interface_v4();
    let at = |port: u16| format!("/ip4/{ip}/tcp/{port}/p2p/{}", b_peer.as_str());
    let list = format!("{}\n\n  {}\n", at(4997), at(4998));

    let out = human(&a, &["peers", "add", "-"], &list);
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("added {}\nadded {}\n", at(4997), at(4998))
    );
    let file = a.paths.state_dir().join("list.txt");
    std::fs::write(&file, &list).expect("written");
    let path = file.to_str().expect("a UTF-8 path");
    let out = human(&a, &["peers", "add", "--file", path], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("unchanged {}\nunchanged {}\n", at(4997), at(4998))
    );

    let loopback = format!("/ip4/127.0.0.1/tcp/4999/p2p/{}", b_peer.as_str());
    let refused = format!("{}\n{loopback}\n", at(4996));
    let out = human(&a, &["peers", "add", "-"], &refused);
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("address 2 of 2") && err.contains("special_use"),
        "{err}"
    );
    let out = human(&a, &["peers", "--json"], "");
    let listed = lines(&out);
    assert!(
        listed.iter().any(|row| row["address"] == at(4997).as_str()),
        "{}",
        said(&out)
    );
    assert!(
        !listed.iter().any(|row| row["address"] == at(4996).as_str()),
        "all or none: {}",
        said(&out)
    );
    let out = human(&a, &["peers", "add", "-"], "\n \n");
    assert_eq!(code(&out), Some(1), "an empty list: {}", said(&out));

    a_daemon.terminate().await;
    b_daemon.terminate().await;
}

/// With no daemon at all: `identity show` names any human endpoint of the
/// profile -- the one in front by name or not, another one's key file
/// read and never made -- and a list of addresses this client refuses is
/// refused before any socket, exit 1 rather than 3.
#[test]
fn identity_show_names_any_human_endpoint_and_a_list_is_judged_before_any_daemon() {
    let home = Home::new("human-desktop");
    let peer = Home::new("human-desktop").write_key();
    home.write_key();
    let ip = interweave_test_support::net::require_private_interface_v4();
    let shipped = example(
        "human-desktop.yaml",
        &peer,
        &format!("/ip4/{ip}/tcp/{}", free_port(ip)),
        None,
    );
    let claude = "    - { id: claude,";
    assert_eq!(shipped.matches(claude).count(), 1, "the example's shape");
    home.write_config(&shipped.replace(
        claude,
        "    - { id: human-spare, enabled: true, advertise: true, \
         allowed_client_kinds: [human-client], inbound: inherit_profile_trust, \
         outbound: inherit_profile_trust }\n    - { id: claude,",
    ));

    let out = human(&home, &["identity", "show", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let front = lines(&out)[0]["fingerprint"].clone();
    assert!(front.is_string(), "{}", said(&out));
    let out = human(&home, &["identity", "show", "human", "--json"], "");
    assert_eq!(
        code(&out),
        Some(0),
        "named, the one in front: {}",
        said(&out)
    );
    assert_eq!(lines(&out)[0]["fingerprint"], front);
    let out = human(&home, &["identity", "show", "human-spare", "--json"], "");
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    assert_eq!(lines(&out)[0]["key_problem"], "absent", "{}", said(&out));
    assert!(
        !home
            .paths
            .state_dir()
            .join("identity-keys")
            .join("human-spare.key")
            .exists(),
        "no key is made for it"
    );
    let out = human(&home, &["identity", "show", "claude"], "");
    assert_eq!(code(&out), Some(1), "{}", said(&out));

    let list = home.paths.state_dir().join("empty.txt");
    std::fs::write(&list, "\n  \n").expect("written");
    let path = list.to_str().expect("a UTF-8 path");
    let out = human(&home, &["peers", "add", "--file", path], "");
    assert_eq!(
        code(&out),
        Some(1),
        "refused before any daemon: {}",
        said(&out)
    );
    let seventeen = (1..=17)
        .map(|n| format!("/ip4/{ip}/tcp/{n}"))
        .collect::<Vec<_>>()
        .join("\n");
    let out = human(&home, &["peers", "add", "-"], &seventeen);
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    let out = human(&home, &["peers", "add", "-"], &format!("/ip4/{ip}/tcp/1\n"));
    assert_eq!(
        code(&out),
        Some(3),
        "the control: a list it takes needs the daemon: {}",
        said(&out)
    );
}

/// ADR-0060 decision 6 as a shell runs it: a store whose key is gone is
/// reported needing recovery, its discard is refused without the scope it
/// lists, and the confirmed discard recreates it empty. The daemon is A's
/// alone, so the message A sends to B stays pending in A's store.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_store_without_its_key_is_discarded_only_on_its_confirmed_scope() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (a, b) = (Home::new("human-desktop"), Home::new("human-desktop"));
    let (_a_peer, b_peer) = (a.write_key(), b.write_key());
    let (a_port, b_port) = (free_port(ip), free_port(ip));
    a.write_config(&example(
        "human-desktop.yaml",
        &b_peer,
        &format!("/ip4/{ip}/tcp/{a_port}"),
        Some(&format!("/ip4/{ip}/tcp/{b_port}/p2p/{}", b_peer.as_str())),
    ));
    let mut a_daemon = a.start(&[]);
    a_daemon.serving(&a).await;
    runtime_up(&mut a_daemon).await;

    let out = human(
        &a,
        &["send", b_peer.as_str(), "to nobody yet", "--wait", "500"],
        "",
    );
    assert!(matches!(code(&out), Some(1 | 6)), "{}", said(&out));
    let out = human(&a, &["store", "health", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(lines(&out)[0]["pending"], 1, "{}", said(&out));

    std::fs::remove_file(a.paths.state_dir().join("human-store.key")).expect("the store key");
    let out = human(&a, &["store", "health", "--json"], "");
    assert_eq!(code(&out), Some(5), "{}", said(&out));
    let report = &lines(&out)[0];
    assert_eq!(report["health"], "needs-recovery", "{}", said(&out));
    assert_eq!(report["key"], "absent", "{}", said(&out));

    let out = human(&a, &["store", "discard"], "");
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("discard     everything"),
        "{}",
        said(&out)
    );
    let out = human(
        &a,
        &["store", "discard", "--confirm", "unfinished-plaintext"],
        "",
    );
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    let out = human(&a, &["store", "health", "--json"], "");
    assert_eq!(lines(&out)[0]["health"], "needs-recovery", "refused, kept");

    let out = human(
        &a,
        &["store", "discard", "--confirm", "everything", "--json"],
        "",
    );
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(lines(&out)[0]["recreated"], true, "{}", said(&out));
    let out = human(&a, &["store", "health", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let report = &lines(&out)[0];
    assert_eq!(report["key"], "present", "{}", said(&out));
    assert_eq!(report["pending"], 0, "{}", said(&out));

    a_daemon.terminate().await;
}

/// One direction of one connection, as the tap copied it.
type Stream = std::sync::Arc<std::sync::Mutex<Vec<u8>>>;

/// A data-socket stand-in that passes every byte both ways to the
/// daemon's socket and keeps a copy of each direction of each connection
/// apart, so its frames can be read back whole: what a client sends over
/// IPC, read at the client's own socket path. Stopped by [`Tap::stop`].
struct Tap {
    runtime: tempfile::TempDir,
    seen: std::sync::Arc<std::sync::Mutex<Vec<Stream>>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    at: std::path::PathBuf,
    accept: Option<std::thread::JoinHandle<()>>,
}

impl Tap {
    /// A runtime directory of its own whose data socket for `home`'s
    /// profile is the tap, in front of the daemon's.
    fn start(home: &Home) -> Self {
        use std::os::unix::fs::PermissionsExt as _;
        use std::os::unix::net::{UnixListener, UnixStream};
        use std::sync::atomic::Ordering;
        let upstream = home.paths.data_socket().expect("a data socket");
        let relative = upstream
            .strip_prefix(home.roots.runtime_dir.as_deref().expect("a runtime dir"))
            .expect("the socket under the runtime dir")
            .to_owned();
        let runtime = tempfile::tempdir().expect("a runtime dir");
        std::fs::set_permissions(runtime.path(), std::fs::Permissions::from_mode(0o700))
            .expect("private");
        let at = runtime.path().join(relative);
        let parent = at.parent().expect("a parent");
        std::fs::create_dir_all(parent).expect("the run dir");
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).expect("private");
        let listener = UnixListener::bind(&at).expect("the tap listens");
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (s, st) = (std::sync::Arc::clone(&seen), std::sync::Arc::clone(&stop));
        let accept = std::thread::spawn(move || {
            for client in listener.incoming() {
                if st.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(client) = client else { return };
                let Ok(daemon) = UnixStream::connect(&upstream) else {
                    continue;
                };
                for (mut from, mut to) in [
                    (
                        client.try_clone().expect("clone"),
                        daemon.try_clone().expect("clone"),
                    ),
                    (daemon, client),
                ] {
                    let seen = Stream::default();
                    s.lock().expect("lock").push(std::sync::Arc::clone(&seen));
                    std::thread::spawn(move || {
                        use std::io::{Read as _, Write as _};
                        let mut buf = [0_u8; 16 * 1024];
                        loop {
                            match from.read(&mut buf) {
                                Ok(0) | Err(_) => {
                                    let _ = to.shutdown(std::net::Shutdown::Write);
                                    return;
                                }
                                Ok(n) => {
                                    seen.lock().expect("lock").extend_from_slice(&buf[..n]);
                                    if to.write_all(&buf[..n]).is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    });
                }
            }
        });
        Self {
            runtime,
            seen,
            stop,
            at,
            accept: Some(accept),
        }
    }

    /// `interweave-human` for `home`'s profile, through the tap.
    fn human(&self, home: &Home, args: &[&str]) -> Output {
        let mut command = command(home, args);
        command.env("XDG_RUNTIME_DIR", self.runtime.path());
        command
            .stdin(Stdio::null())
            .output()
            .expect("interweave-human runs")
    }

    /// Every direction of every connection so far, each as one buffer.
    fn seen(&self) -> Vec<Vec<u8>> {
        self.seen
            .lock()
            .expect("lock")
            .iter()
            .map(|s| s.lock().expect("lock").clone())
            .collect()
    }

    fn stop(mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = std::os::unix::net::UnixStream::connect(&self.at);
        if let Some(accept) = self.accept.take() {
            accept.join().expect("the tap's accept ends");
        }
    }
}

/// A picture larger than the card's 256-pixel bound, as a PNG carrying a
/// text block a camera or an editor might leave.
fn picture_file(dir: &std::path::Path) -> std::path::PathBuf {
    let image = image::RgbImage::from_fn(320, 200, |x, y| {
        image::Rgb([
            u8::try_from(x % 256).unwrap_or(0),
            u8::try_from(y % 256).unwrap_or(0),
            128,
        ])
    });
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("a PNG");
    let path = dir.join("portrait.png");
    std::fs::write(&path, &png).expect("written");
    path
}

/// The forms the secret of `home`'s first identity's key could take
/// anywhere else: its bytes, their base64 and their hex. The record's last
/// 32 bytes are the secret at the desktop's filesystem floor (IDK1).
fn key_forms(home: &Home) -> Vec<Vec<u8>> {
    use base64::Engine as _;
    let human = interweave_transport_api::EndpointId::parse("human").expect("valid");
    let line = std::fs::read_to_string(interweave_human_store::identity_key::key_path(
        &home.paths,
        &human,
    ))
    .expect("the key file");
    let record = base64::engine::general_purpose::STANDARD
        .decode(line.trim())
        .expect("base64");
    let secret = record[record.len() - 32..].to_vec();
    let hex = secret.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    });
    vec![
        base64::engine::general_purpose::STANDARD
            .encode(&secret)
            .into_bytes(),
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(&secret)
            .into_bytes(),
        hex.into_bytes(),
        secret,
    ]
}

/// What one direction of an IPC connection carried, made plain: each
/// frame's JSON body (LOCAL-IPC.md: four bytes of length, then the body),
/// each payload's bytes decoded from their base64url, and each
/// `HumanChatV2` payload decoded as the client decodes it, compressed or
/// not. A secret inside a payload is encoded with the payload, at any
/// alignment, so a grep of the wire alone would not see it there.
fn ipc_plain(stream: &[u8]) -> Vec<u8> {
    use base64::Engine as _;
    fn payloads(value: &Value, out: &mut Vec<u8>) {
        match value {
            Value::Object(map) => {
                if let (Some(Value::String(media)), Some(Value::String(bytes))) =
                    (map.get("media_type"), map.get("bytes"))
                {
                    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .decode(bytes.trim_end_matches('='))
                        .expect("a payload's bytes are base64url");
                    if let Ok(info) = interweave_human_chat_protocol::parse_media_type(media) {
                        let text = interweave_human_chat_protocol::decode_envelope_bytes(
                            &raw,
                            info.encoding,
                        )
                        .expect("a HumanChatV2 payload decodes");
                        out.extend_from_slice(text.as_bytes());
                    }
                    out.extend_from_slice(&raw);
                }
                map.values().for_each(|v| payloads(v, out));
            }
            Value::Array(list) => list.iter().for_each(|v| payloads(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    let mut rest = stream;
    while rest.len() >= 4 {
        let len = u32::from_be_bytes(rest[..4].try_into().expect("four bytes")) as usize;
        let Some(body) = rest.get(4..4 + len) else {
            break;
        };
        out.extend_from_slice(body);
        if let Ok(value) = serde_json::from_slice::<Value>(body) {
            payloads(&value, &mut out);
        }
        rest = &rest[4 + len..];
    }
    out
}

fn holds(haystack: &[u8], forms: &[Vec<u8>]) -> bool {
    forms
        .iter()
        .any(|f| haystack.windows(f.len()).any(|w| w == f.as_slice()))
}

/// `card set --photo` with a real image file, run once end to end: the
/// photo prepared, shared, proposed with a photo, accepted as the card's;
/// and across that run the identity's private key is in no byte the
/// client sent over IPC, in no log of either daemon and in nothing the
/// client printed (human-client-ui.md section 13; plan section 29's
/// exit gate). The tap's control: it read the share's `direct.send` and,
/// decoded from its payload, the card's name.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_photo_set_from_a_file_reaches_the_contact_and_the_key_stays_in_its_file() {
    let (a, mut a_daemon, a_peer, b, mut b_daemon, b_peer) = two_daemons().await;
    let to_b = format!("{}/human", b_peer.as_str());
    let to_a = format!("{}/human", a_peer.as_str());
    let scratch = tempfile::tempdir().expect("scratch");
    let picture = picture_file(scratch.path());
    let picture = picture.to_str().expect("a UTF-8 path");
    let tap = Tap::start(&a);
    let mut printed = Vec::new();
    let mut keep = |out: &Output| {
        printed.extend_from_slice(&out.stdout);
        printed.extend_from_slice(&out.stderr);
    };

    let out = tap.human(&a, &["card", "set", "Ada", "Lovelace", "--photo", picture]);
    keep(&out);
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let out = tap.human(&a, &["card", "show", "--json"]);
    keep(&out);
    assert_eq!(lines(&out)[0]["has_photo"], true, "{}", said(&out));

    let listener = Listener::start(&b);
    listener.until("the session ready", |l| {
        l["event"] == "session" && l["session"] == "ready"
    });
    let deadline = Instant::now() + PATIENCE;
    loop {
        let out = tap.human(&a, &["--wait", "3000", "card", "share", &to_b, "--json"]);
        keep(&out);
        if code(&out) == Some(0) {
            break;
        }
        assert_eq!(code(&out), Some(6), "{}", said(&out));
        let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
        keep(&tap.human(&a, &["cancel", &id]));
        assert!(Instant::now() < deadline, "never accepted: {}", said(&out));
    }
    let heard = listener.until("the proposal", |l| l["event"] == "proposal");
    assert_eq!(heard["has_photo"], true, "the card carries the photo");
    listener.stop();

    let out = human(&b, &["proposals", "--json"], "");
    let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    let out = human(
        &b,
        &["proposals", "read", &to_a, "--accept", &id, "--json"],
        "",
    );
    assert_eq!(lines(&out)[0]["status"], "accepted", "{}", said(&out));
    let out = human(&b, &["contacts", "--json"], "");
    let contact = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    let out = human(&b, &["contacts", "show", &contact, "--json"], "");
    assert_eq!(
        lines(&out)[0]["photo_from"],
        "card",
        "the accepted photo is the card's: {}",
        said(&out)
    );

    let traffic: Vec<u8> = tap.seen().iter().flat_map(|s| ipc_plain(s)).collect();
    tap.stop();
    a_daemon.terminate().await;
    b_daemon.terminate().await;
    let forms = key_forms(&a);
    assert!(
        holds(&traffic, &[b"direct.send".to_vec()]) && holds(&traffic, &[b"Lovelace".to_vec()]),
        "the control: the tap carried the share and its card was read back ({} bytes)",
        traffic.len()
    );
    assert!(!holds(&traffic, &forms), "the key went over IPC");
    assert!(!holds(&printed, &forms), "the client printed the key");
    for (who, log) in [("A", a_daemon.log()), ("B", b_daemon.log())] {
        assert!(
            !holds(log.as_bytes(), &forms),
            "daemon {who} logged the key"
        );
    }
}

/// Each `HumanChatV2` card in what one direction of an IPC connection
/// carried, as its text: the payloads [`ipc_plain`] decodes, kept apart.
fn ipc_cards(stream: &[u8]) -> Vec<String> {
    use base64::Engine as _;
    fn cards(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                if let (Some(Value::String(media)), Some(Value::String(bytes))) =
                    (map.get("media_type"), map.get("bytes"))
                    && let Ok(info) = interweave_human_chat_protocol::parse_media_type(media)
                {
                    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .decode(bytes.trim_end_matches('='))
                        .expect("a payload's bytes are base64url");
                    let text =
                        interweave_human_chat_protocol::decode_envelope_bytes(&raw, info.encoding)
                            .expect("a HumanChatV2 payload decodes");
                    if interweave_human_chat_protocol::is_card(&text) {
                        out.push(text);
                    }
                }
                map.values().for_each(|v| cards(v, out));
            }
            Value::Array(list) => list.iter().for_each(|v| cards(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    let mut rest = stream;
    while rest.len() >= 4 {
        let len = u32::from_be_bytes(rest[..4].try_into().expect("four bytes")) as usize;
        let Some(body) = rest.get(4..4 + len) else {
            break;
        };
        if let Ok(value) = serde_json::from_slice::<Value>(body) {
            cards(&value, &mut out);
        }
        rest = &rest[4 + len..];
    }
    out
}

/// Plan section 29's exit gate (b): A's card shared into two conversations
/// with B, at B's `human` and at B's `claude` -- two identities of one
/// node, which the fake cannot hold apart (app-core's `cards.rs`) -- is
/// A's `human` identity's card each time, read back where it arrived:
/// from that route, verifying on it and on no other identity of A's node,
/// with A's own card's name and key. And every card A's client handed its
/// daemon over IPC verifies as that identity's and no other. The tap's
/// control: it read both shares. A's other identity, `claude`, holds no
/// key here, so "no other" is shown by the signature, which binds the
/// route it was made for (two identities each with a key are section 30's).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_card_shared_to_two_conversations_is_the_conversations_identitys_each_time() {
    use interweave_human_chat_protocol::{CardRoute, Classified, classify};
    use interweave_human_store::identity_key::{self, FilesystemCustody};
    use interweave_local_client_api::{
        DataCapability, DataSessionBinding as _, DataSessionPort as _, SessionEvent, SessionRequest,
    };
    use interweave_transport_api::EndpointId;

    let (a, mut a_daemon, a_peer, b, mut b_daemon, b_peer) = two_daemons().await;
    let (human_id, claude_id) = (
        EndpointId::parse("human").expect("valid"),
        EndpointId::parse("claude").expect("valid"),
    );
    let out = human(&a, &["card", "set", "Ada", "Lovelace"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    let key = identity_key::open(&a.paths, &human_id, &FilesystemCustody)
        .expect("opens")
        .expect("made with the card");

    // B's two identities, each held by a session of the kind its entry
    // admits, so what arrives at each is read as the daemon handed it over.
    let binding = b.binding();
    let open = |kind: &str, endpoint: &EndpointId| {
        binding.open(
            SessionRequest::new(
                kind,
                Some(endpoint.clone()),
                [DataCapability::Commands, DataCapability::Events],
            )
            .expect("bounds"),
        )
    };
    let at_human = open("human-client", &human_id).await.expect("B's human");
    let at_claude = open("claude-channel", &claude_id)
        .await
        .expect("B's claude");

    let tap = Tap::start(&a);
    let mut arrived = Vec::new();
    for (to, session) in [(&human_id, &at_human), (&claude_id, &at_claude)] {
        let conversation = format!("{}/{}", b_peer.as_str(), to.as_str());
        let deadline = Instant::now() + PATIENCE;
        loop {
            let out = tap.human(
                &a,
                &["--wait", "3000", "card", "share", &conversation, "--json"],
            );
            if code(&out) == Some(0) {
                break;
            }
            assert_eq!(code(&out), Some(6), "{}", said(&out));
            let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
            assert_eq!(code(&tap.human(&a, &["cancel", &id])), Some(0));
            assert!(Instant::now() < deadline, "never accepted: {}", said(&out));
        }
        let deadline = Instant::now() + PATIENCE;
        let direct = loop {
            let got = session.events(usize::MAX).await.expect("B's events");
            if let Some(d) = got.into_iter().find_map(|e| match e {
                SessionEvent::Direct(d) => Some(d),
                _ => None,
            }) {
                break d;
            }
            assert!(Instant::now() < deadline, "{conversation}: nothing arrived");
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        assert_eq!(&direct.destination_endpoint, to, "{conversation}");
        arrived.push((conversation, direct));
    }

    let as_identity = |text: &str, endpoint: &EndpointId| {
        classify(
            text,
            CardRoute {
                source_peer: &a_peer,
                source_endpoint: endpoint,
            },
        )
    };
    for (conversation, direct) in &arrived {
        assert_eq!(direct.source_peer, a_peer, "{conversation}");
        assert_eq!(
            direct.source_endpoint, human_id,
            "{conversation}: from the identity A's client holds"
        );
        let media = direct.payload.media_type().expect("a media type");
        let info =
            interweave_human_chat_protocol::parse_media_type(media.as_str()).expect("HumanChatV2");
        let text = interweave_human_chat_protocol::decode_envelope_bytes(
            direct.payload.bytes(),
            info.encoding,
        )
        .expect("decodes");
        let Classified::Card(card) = as_identity(&text, &human_id) else {
            panic!("{conversation}: not A's human card: {text}");
        };
        assert_eq!(card.name, "Ada", "{conversation}");
        assert_eq!(card.surname.as_deref(), Some("Lovelace"), "{conversation}");
        assert_eq!(
            card.identity_key,
            key.public_key(),
            "{conversation}: its key"
        );
        assert!(
            matches!(as_identity(&text, &claude_id), Classified::MalformedCard(_)),
            "{conversation}: it would not pass as A's other identity's"
        );
    }

    let sent: Vec<String> = tap.seen().iter().flat_map(|s| ipc_cards(s)).collect();
    tap.stop();
    assert!(
        sent.len() >= 2,
        "the control: the tap read both shares, {sent:?}"
    );
    for text in &sent {
        assert!(
            matches!(as_identity(text, &human_id), Classified::Card(_)),
            "a card A's client sent is not its human identity's: {text}"
        );
        let envelope: Value = serde_json::from_str(text).expect("JSON");
        assert!(
            envelope
                .get("from_endpoint")
                .is_none_or(|e| e == human_id.as_str()),
            "a card names another identity: {text}"
        );
    }
    a_daemon.terminate().await;
    b_daemon.terminate().await;
}

/// The control-plane carrier (architect-cto's REQUEST of 2026-10-27, the
/// owner's priority), through the binary and two daemons: `listen --json
/// --input --consume` sends a stdin line over its own lease and answers it
/// with its ref (G1), prints a message with the local endpoint that got it
/// (G6) and leaves it read, no unread row in the store (G3); `send
/// --no-pending` that is never accepted is cancelled and leaves nothing
/// pending (G4); and an endpoint the profile does not offer is refused
/// (G2).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_control_plane_carrier_sends_consumes_and_names_its_endpoint() {
    use std::io::Write as _;
    let (a, mut a_daemon, a_peer, b, mut b_daemon, b_peer) = two_daemons().await;
    let to_a = format!("{}/human", a_peer.as_str());
    let to_b = format!("{}/human", b_peer.as_str());

    let (listener, mut input) = Listener::with(&b, &["listen", "--json", "--input", "--consume"]);
    listener.until("the session ready", |l| {
        l["event"] == "session" && l["session"] == "ready"
    });
    // A takes what B sends only while a client of A holds its lease.
    let a_listener = Listener::start(&a);
    a_listener.until("A's session ready", |l| {
        l["event"] == "session" && l["session"] == "ready"
    });
    writeln!(input, "not json").expect("written");
    let refused = listener.until("the refusal", |l| l["event"] == "input-refused");
    assert_eq!(refused["ref"], Value::Null);
    let deadline = Instant::now() + PATIENCE;
    loop {
        writeln!(
            input,
            r#"{{"send":{{"conversation":"{to_a}","text":"over the lease","ref":"r1"}}}}"#
        )
        .expect("written");
        input.flush().expect("flushed");
        let answer = listener.until("the send's answer", |l| {
            l["event"] == "outbound" && l["ref"] == "r1"
        });
        let id = answer["id"].as_str().expect("an id").to_owned();
        let last = listener.until("its last status", |l| {
            l["event"] == "outbound"
                && l["id"] == id.as_str()
                && matches!(l["status"].as_str(), Some("accepted" | "needs-attention"))
        });
        assert_eq!(
            last["ref"], "r1",
            "every outbound event of its row names its ref"
        );
        if last["status"] == "accepted" {
            break;
        }
        assert!(Instant::now() < deadline, "never accepted: {last}");
    }
    a_listener.until("B's line at A", |l| {
        l["event"] == "message" && l["text"] == "over the lease"
    });
    a_listener.stop();

    send_accepted(&a, &to_b, "consumed by the carrier");
    let message = listener.until("the message", |l| {
        l["event"] == "message" && l["text"] == "consumed by the carrier"
    });
    assert_eq!(message["local_endpoint"], "human", "G6");
    // Read as printed, before listen looks at the stop: no wait here.
    drop(input);
    listener.stop();
    let out = human(&b, &["store", "health", "--json"], "");
    assert_eq!(code(&out), Some(0), "{}", said(&out));
    assert_eq!(lines(&out)[0]["unread"], 0, "consumed: {}", said(&out));

    // G4: with B's daemon gone, an ephemeral send to B is never accepted:
    // it is cancelled when its wait ends.
    b_daemon.terminate().await;
    let out = human(
        &a,
        &[
            "--wait",
            "1500",
            "send",
            &to_b,
            "ephemeral",
            "--no-pending",
            "--json",
        ],
        "",
    );
    assert_eq!(code(&out), Some(1), "{}", said(&out));
    assert_eq!(lines(&out)[0]["status"], "cancelled");
    assert_eq!(lines(&out)[0]["pending"], false);
    let out = human(&a, &["store", "health", "--json"], "");
    assert_eq!(
        lines(&out)[0]["pending"],
        0,
        "nothing left to send: {}",
        said(&out)
    );

    // G2: an endpoint the profile does not offer is refused, none made --
    // and refused before the store or a lease is asked for: with another
    // client holding both, the refusal is the endpoint's (exit 1), not the
    // held store's (exit 4). A lease the refused run took would end with
    // it, so only that order shows it took none (#477 F11).
    let holder = Listener::start(&a);
    holder.until("the holder's session ready", |l| {
        l["event"] == "session" && l["session"] == "ready"
    });
    let out = human(&a, &["listen", "--endpoint", "not-offered"], "");
    assert_eq!(
        code(&out),
        Some(1),
        "the endpoint's refusal: {}",
        said(&out)
    );
    assert!(
        said(&out).contains("not one a human client may use"),
        "{}",
        said(&out)
    );
    holder.stop();
    let listed = a.transportctl(&["endpoints", "list", "--json"], "");
    let listed: Value = serde_json::from_slice(&listed.stdout).expect("the list");
    assert!(
        listed["endpoints"]
            .as_array()
            .expect("rows")
            .iter()
            .all(|row| row["id"] != "not-offered"),
        "none made: {listed}"
    );

    a_daemon.terminate().await;
}

/// G2 through the binary: on a profile with a second human endpoint,
/// `listen --endpoint` leases that one and no other, and its events name
/// it. Mutation: `--endpoint` parsed and the profile's first endpoint
/// leased all the same.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn listen_with_endpoint_leases_the_one_named() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let a = Home::new("human-desktop");
    a.write_key();
    // A peer the profile names, never started.
    let other = Home::new("human-desktop").write_key();
    let claude = "    - { id: claude,";
    let config = example(
        "human-desktop.yaml",
        &other,
        &format!("/ip4/{ip}/tcp/{}", free_port(ip)),
        None,
    );
    assert!(config.contains(claude), "the example's claude row");
    a.write_config(&config.replacen(
        claude,
        &format!(
            "    - {{ id: control, enabled: true, advertise: true, allowed_client_kinds: [human-client], inbound: inherit_profile_trust, outbound: inherit_profile_trust }}\n{claude}"
        ),
        1,
    ));
    let mut a_daemon = a.start(&[]);
    a_daemon.serving(&a).await;
    runtime_up(&mut a_daemon).await;

    let listener = Listener::with(&a, &["listen", "--json", "--endpoint", "control"]).0;
    listener.until("the session ready", |l| {
        l["event"] == "session" && l["session"] == "ready"
    });
    let listed = a.transportctl(&["endpoints", "list", "--json"], "");
    let listed: Value = serde_json::from_slice(&listed.stdout).expect("the list");
    let leased: Vec<&Value> = listed["endpoints"]
        .as_array()
        .expect("rows")
        .iter()
        .filter(|row| !row["lease"].is_null())
        .map(|row| &row["id"])
        .collect();
    assert_eq!(leased, [&Value::from("control")], "{listed}");
    listener.stop();
    a_daemon.terminate().await;
}
