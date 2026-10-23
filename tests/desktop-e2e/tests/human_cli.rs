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
        let mut child = command(home, &["listen", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("listen runs");
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
        Self { child, lines }
    }

    /// The first line `matches` takes, within `PATIENCE`.
    fn until(&self, what: &str, matches: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) if matches(&line) => return line,
                Ok(_) => {}
                Err(e) => panic!("listen never printed {what}: {e}"),
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
