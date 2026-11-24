// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `--endpoint <alias|id>` over the real binary (TOOL-SURFACE.md, the
//! Stage 22 paragraph and §Status visibility): the daemon is a script on
//! the real data socket, byte-faithful to LOCAL-IPC's frames, and the
//! bridge is the shipped binary started as Claude Code starts it.
//!
//! What each row pins is what the DAEMON sees (the hello carries the id,
//! never the alias; a refused start opens no connection at all) and what
//! the HOST sees (`status`, the log's file name and its lines), because
//! those are the only two places the resolution is observable.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use interweave_ipc_protocol::{
    DecodedFrame, FrameError, IPC_MAJOR, IPC_MAX_MINOR, TRANSPORT_CONTRACT_VERSION, decode_frame,
    encode_frame,
};
use interweave_profile_config::endpoint_key::{self, Custody};
use interweave_profile_config::{ProfilePaths, XdgRoots, create_private_dir};
use interweave_test_support::endpoint_keys::endpoint_id;
use serde_json::{Value, json};

/// Generous against a loaded host; every answer here is immediate.
const PATIENCE: Duration = Duration::from_secs(10);
const PROFILE: &str = "alias";
const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
/// TEST-ONLY: a stand-in for a sealed key's secret. It must reach no
/// stream the bridge writes (row 13).
const MARKER: &str = "TEST-ONLY-SEALED-MARKER-7f3a";

/// What the scripted daemon answers a hello with.
#[derive(Clone, Copy)]
enum Answer {
    /// A `hello_response` leasing the endpoint the hello named.
    Lease,
    /// A `close` with this code (an unknown endpoint's).
    Close(&'static str),
    /// What a daemon of major 2 answers a hello of a major it does not
    /// speak: `close{VersionIncompatible}` listing 2.13 alone.
    MajorTwo,
}

/// The daemon: the data socket's listener on a thread of its own,
/// counting every connection and keeping every hello it read.
struct Daemon {
    accepted: Arc<AtomicUsize>,
    hellos: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Daemon {
    fn start(socket: &Path, answer: Answer) -> Self {
        let dir = socket.parent().expect("a directory");
        std::fs::create_dir_all(dir).expect("the socket directory");
        for d in [dir.parent().expect("run"), dir] {
            std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700)).expect("0700");
        }
        let listener = UnixListener::bind(socket).expect("binds");
        listener.set_nonblocking(true).expect("nonblocking");
        let accepted = Arc::new(AtomicUsize::new(0));
        let hellos = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (accepted, hellos, stop) = (accepted.clone(), hellos.clone(), stop.clone());
            std::thread::spawn(move || {
                // Connections are kept until the daemon ends: a lease
                // lives as long as its connection.
                let mut held: Vec<UnixStream> = Vec::new();
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            accepted.fetch_add(1, Ordering::SeqCst);
                            stream.set_nonblocking(false).expect("blocking");
                            stream
                                .set_read_timeout(Some(PATIENCE))
                                .expect("read timeout");
                            if let Some(stream) = hello_and_answer(stream, answer, &hellos) {
                                let reader = stream.try_clone().expect("a second handle");
                                std::thread::spawn(move || answer_requests(reader));
                                held.push(stream);
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(e) => panic!("accept: {e}"),
                    }
                }
            })
        };
        Self {
            accepted,
            hellos,
            stop,
            thread: Some(thread),
        }
    }

    fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    /// The first hello read, waited for within [`PATIENCE`].
    fn first_hello(&self) -> Value {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if let Some(hello) = self.hellos.lock().expect("lock").first() {
                return hello.clone();
            }
            assert!(Instant::now() < deadline, "the bridge sent no hello");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn hello_and_answer(
    mut stream: UnixStream,
    answer: Answer,
    hellos: &Mutex<Vec<Value>>,
) -> Option<UnixStream> {
    let mut buf = Vec::new();
    let body = loop {
        match decode_frame(&buf) {
            Ok(DecodedFrame { body, .. }) => break body,
            Err(FrameError::Incomplete { .. }) => {}
            Err(e) => panic!("a bad frame: {e:?}"),
        }
        let mut chunk = [0_u8; 4096];
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let hello: Value = serde_json::from_str(&body).expect("the hello is JSON");
    assert_eq!(hello["type"], "hello", "{hello}");
    let claimed = hello["endpoint"]["id"].as_str().map(str::to_owned);
    hellos.lock().expect("lock").push(hello);
    let reply = match answer {
        Answer::Lease => json!({
            "type": "hello_response",
            "ipc_version": {"major": IPC_MAJOR, "minor": IPC_MAX_MINOR},
            "transport_contract_version": TRANSPORT_CONTRACT_VERSION,
            "peer": PEER,
            "endpoint": claimed.expect("a claim"),
            "endpoint_lease_epoch": "AAAAAAAAAAAAAAAAAAAAAQ",
            "event_queue": 16,
            "granted_capabilities": ["events", "commands"]
        }),
        Answer::Close(code) => json!({"type": "close", "code": code}),
        Answer::MajorTwo => json!({
            "type": "close",
            "code": "VersionIncompatible",
            "supported": [{"major": 2, "minor": 13}]
        }),
    };
    stream
        .write_all(&encode_frame(&reply.to_string()).expect("a frame"))
        .ok()?;
    Some(stream)
}

/// Answers each request on a leased connection until it closes. On 3.0
/// every `status` reads `session.losses`, and the client sets no
/// deadline on a call, so a daemon that never answered would leave
/// `status` waiting; nothing here counts losses, so the counts are
/// zero. Any other method is an error: these rows send none.
fn answer_requests(mut stream: UnixStream) {
    let mut buf = Vec::new();
    loop {
        while let Ok(DecodedFrame { body, consumed }) = decode_frame(&buf) {
            buf.drain(..consumed);
            let frame: Value = serde_json::from_str(&body).expect("a frame is JSON");
            if frame["type"] != "request" {
                continue;
            }
            assert_eq!(frame["method"], "session.losses", "{frame}");
            let reply = json!({"type": "response", "id": frame["id"], "ok": true,
                               "result": {"queue_overflow": 0, "ingress_refused": 0}});
            if stream
                .write_all(&encode_frame(&reply.to_string()).expect("a frame"))
                .is_err()
            {
                return;
            }
        }
        let mut chunk = [0_u8; 4096];
        match stream.read(&mut chunk) {
            Ok(n) if n > 0 => buf.extend_from_slice(&chunk[..n]),
            // The accept loop's read timeout is shared by this handle; a
            // quiet bridge is not a closed one.
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Ok(_) | Err(_) => return,
        }
    }
}

/// The real binary under test, its stdio piped; killed and reaped however
/// the test ends.
struct Bridge {
    child: Child,
    lines: mpsc::Receiver<String>,
    stderr: Arc<Mutex<Vec<u8>>>,
}

impl Bridge {
    fn start(scratch: &Path, args: &[&str]) -> Self {
        let run = scratch.join("run");
        let mut child = Command::new(env!("CARGO_BIN_EXE_claude-channel"))
            .args(["--profile", PROFILE])
            .args(args)
            .args(["--delivery", "push", "--log-level", "debug"])
            .env("XDG_RUNTIME_DIR", &run)
            .env("XDG_STATE_HOME", scratch.join("state"))
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_DATA_HOME", scratch.join("data"))
            .env("XDG_CACHE_HOME", scratch.join("cache"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawns");
        let stdout = child.stdout.take().expect("stdout");
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let mut pipe = child.stderr.take().expect("stderr");
        let sink = stderr.clone();
        std::thread::spawn(move || {
            let mut chunk = [0_u8; 4096];
            while let Ok(n) = pipe.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                sink.lock().expect("lock").extend_from_slice(&chunk[..n]);
            }
        });
        Self {
            child,
            lines,
            stderr,
        }
    }

    fn ask(&mut self, id: u64, method: &str, params: &Value) -> Value {
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let stdin = self.child.stdin.as_mut().expect("stdin held");
        writeln!(stdin, "{request}").expect("written");
        stdin.flush().expect("flushed");
        let line = self.lines.recv_timeout(PATIENCE).expect("answered");
        serde_json::from_str(&line).expect("json")
    }

    fn initialize(&mut self) {
        self.ask(
            1,
            "initialize",
            &json!({"protocolVersion": "2025-11-25", "capabilities": {}}),
        );
    }

    fn status(&mut self) -> Value {
        let answer = self.ask(2, "tools/call", &json!({"name": "status"}));
        let text = answer["result"]["content"][0]["text"]
            .as_str()
            .expect("text");
        serde_json::from_str(text).expect("status is json")
    }

    /// The status once `ready` holds of it, polled within [`PATIENCE`].
    fn status_when(&mut self, ready: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let status = self.status();
            if ready(&status) {
                return status;
            }
            assert!(Instant::now() < deadline, "status never ready: {status}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Everything the binary wrote to stderr so far.
    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.stderr.lock().expect("lock")).into_owned()
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A scratch XDG tree for one test, and where the profile's parts are.
struct Scratch {
    dir: tempfile::TempDir,
    paths: ProfilePaths,
}

impl Scratch {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path();
        let roots = XdgRoots {
            config_home: base.join("config"),
            data_home: base.join("data"),
            state_home: base.join("state"),
            cache_home: base.join("cache"),
            runtime_dir: None,
        };
        let paths = ProfilePaths::resolve_offline(PROFILE, &roots).expect("paths");
        Self { dir, paths }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn socket(&self) -> PathBuf {
        self.root()
            .join("run/interweave")
            .join(format!("{PROFILE}.sock"))
    }

    fn daemon(&self, answer: Answer) -> Daemon {
        Daemon::start(&self.socket(), answer)
    }

    fn bridge(&self, endpoint: &str) -> Bridge {
        Bridge::start(self.root(), &["--endpoint", endpoint])
    }

    /// A `config.yaml` of these `(name, alias)` entries, written as the
    /// loader's own tests write one, with mode `mode`.
    fn config(&self, entries: &[(&str, Option<&str>)], mode: u32) {
        let items = entries
            .iter()
            .map(|(name, alias)| {
                let key = endpoint_key::key_file_name(&endpoint_id(name));
                let alias = alias.map_or_else(String::new, |a| format!("      alias: {a}\n"));
                format!("    - key: {key}\n{alias}      enabled: true\n      advertise: false\n")
            })
            .collect::<Vec<_>>()
            .concat();
        self.config_text(
            &format!(
                "schema_version: 2\nprofile:\n  name: {PROFILE}\ntrust:\n  policy: static-allowlist\n  allowed_peers: []\nendpoints:\n  entries:\n{items}"
            ),
            mode,
        );
    }

    fn config_text(&self, text: &str, mode: u32) {
        create_private_dir(self.paths.config_dir()).expect("config dir");
        std::fs::write(self.paths.config_file(), text).expect("write");
        std::fs::set_permissions(
            self.paths.config_file(),
            std::fs::Permissions::from_mode(mode),
        )
        .expect("chmod");
    }

    fn keys_dir(&self) -> PathBuf {
        self.paths.state_dir().join(endpoint_key::ENDPOINT_KEYS_DIR)
    }

    /// A key file for `name`'s id holding `text`, owner-only in an
    /// owner-only directory, as the loader reads one.
    fn key_file(&self, name: &str, text: &str) {
        create_private_dir(&self.keys_dir()).expect("keys dir");
        let path = endpoint_key::key_path(&self.paths, &endpoint_id(name));
        std::fs::write(&path, text).expect("write key");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("0600");
    }

    /// The sealed-key stand-in whose bytes no stream may carry.
    fn marker_key(&self, name: &str) {
        self.key_file(name, &format!("# header line\n{MARKER}\n"));
    }

    fn log_file(&self, name: &str) -> PathBuf {
        self.paths
            .state_dir()
            .join("claude-channel")
            .join(format!("{name}.log"))
    }
}

fn claude_id() -> String {
    endpoint_id("claude").as_str().to_owned()
}

/// Row 13's check over every stream the bridge writes: the marker is in
/// none of them, and the alias is in no log LINE (it may be in the file's
/// name, which is not a line).
fn assert_no_leak(bridge: &mut Bridge, log: &Path, statuses: &[&Value], alias: &str) {
    let log_text = std::fs::read_to_string(log).expect("the log file");
    let stderr = bridge.stderr();
    assert!(
        !log_text.is_empty() && !stderr.is_empty(),
        "both streams were live"
    );
    for (stream, text) in [("the log", &log_text), ("stderr", &stderr)] {
        assert!(!text.contains(MARKER), "{stream} carries the key's bytes");
        // The program's own name, `claude-channel`, holds the alias
        // "claude" in every line's prefix; it is not an endpoint's.
        assert!(
            !text.replace("claude-channel", "").contains(alias),
            "{stream} carries the alias in a line: {text}"
        );
    }
    for status in statuses {
        assert!(
            !status.to_string().contains(MARKER),
            "a status carries the key's bytes"
        );
    }
}

/// The hello's claim: the one id, never the alias.
fn assert_hello_names(daemon: &Daemon, id: &str, alias: Option<&str>) {
    let hello = daemon.first_hello();
    assert_eq!(hello["endpoint"]["id"], id, "{hello}");
    if let Some(alias) = alias {
        // Beside the client's kind, `claude-channel`, which holds the
        // alias "claude" and is no endpoint's.
        let mut rest = hello.clone();
        rest.as_object_mut().expect("an object").remove("client");
        assert!(
            !rest.to_string().contains(alias),
            "the alias reached the daemon: {hello}"
        );
    }
}

fn assert_refused_class(bridge: &mut Bridge, class: &str, absent: &[&str]) {
    bridge.initialize();
    let status = bridge.status();
    assert_eq!(status["endpoint_lease_state"], "bridge refused", "{status}");
    assert_eq!(status["refused"]["class"], class, "{status}");
    let reason = status["refused"]["reason"].to_string();
    for word in absent {
        assert!(!reason.contains(word), "the reason names {word}: {reason}");
    }
}

/// The lease taken: the session's peer shows in `status`.
fn leased(status: &Value) -> bool {
    !status["local_peer_id"].is_null()
}

/// Row 1 and row 13: an alias in `config.yaml` starts the bridge on the
/// entry's id.
#[test]
fn an_alias_starts_the_bridge_on_the_entrys_id() {
    let scratch = Scratch::new();
    scratch.config(
        &[("human", Some("human")), ("claude", Some("claude"))],
        0o644,
    );
    scratch.marker_key("claude");
    let daemon = scratch.daemon(Answer::Lease);
    let mut bridge = scratch.bridge("claude");
    bridge.initialize();
    let status = bridge.status_when(leased);
    assert_hello_names(&daemon, &claude_id(), Some("claude"));
    assert_eq!(status["local_endpoint"], claude_id(), "{status}");
    assert_eq!(status["local_endpoint_alias"], "claude", "{status}");
    let log = scratch.log_file("claude");
    assert_eq!(
        status["log"]["file"],
        log.to_str().expect("utf8"),
        "the log is named for the alias: {status}"
    );
    assert!(log.is_file(), "claude.log exists");
    assert_no_leak(&mut bridge, &log, &[&status], "claude");
}

/// Row 3 and row 13: an alias no entry carries refuses the start before
/// any socket is opened.
#[test]
fn an_unknown_alias_is_refused_and_the_daemon_sees_nothing() {
    let scratch = Scratch::new();
    scratch.config(&[("claude", Some("claude"))], 0o644);
    scratch.marker_key("claude");
    let daemon = scratch.daemon(Answer::Lease);
    let mut bridge = scratch.bridge("nobody");
    assert_refused_class(&mut bridge, "endpoint_alias", &["nobody"]);
    let status = bridge.status();
    // The bridge is serving its refusal: give a stray connection its
    // time to arrive before counting none.
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(daemon.accepted(), 0, "a refused start opens no connection");
    assert_no_leak(
        &mut bridge,
        &scratch.log_file("nobody"),
        &[&status],
        "nobody",
    );
}

/// Row 4: an alias needs the configuration; each way it does not load
/// refuses the start, and the daemon sees nothing.
#[test]
fn an_alias_with_a_configuration_that_does_not_load_is_refused() {
    type Prepare = fn(&Scratch);
    let cases: [(&str, Prepare); 3] = [
        ("config.yaml missing", |_| {}),
        ("config.yaml malformed", |s| {
            s.config_text("schema_version: [: not yaml\n  - {", 0o644);
        }),
        // Other-writable: `judge_document`'s refusal that no account's
        // private group can excuse (a group-writable file is accepted
        // when the group is the owner's own).
        ("config.yaml other-writable", |s| {
            s.config(&[("claude", Some("claude"))], 0o666);
        }),
    ];
    for (name, prepare) in cases {
        let scratch = Scratch::new();
        prepare(&scratch);
        let daemon = scratch.daemon(Answer::Lease);
        let mut bridge = scratch.bridge("claude");
        bridge.initialize();
        let status = bridge.status();
        assert_eq!(
            status["refused"]["class"], "endpoint_alias",
            "{name}: {status}"
        );
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(daemon.accepted(), 0, "{name}: no connection");
    }
}

/// Row 5 and row 13: an id needs no configuration.
#[test]
fn an_id_starts_without_a_configuration_and_reports_no_alias() {
    let scratch = Scratch::new();
    scratch.marker_key("claude");
    let daemon = scratch.daemon(Answer::Lease);
    let id = claude_id();
    let mut bridge = scratch.bridge(&id);
    bridge.initialize();
    let status = bridge.status_when(leased);
    assert_hello_names(&daemon, &id, None);
    assert!(
        status.get("local_endpoint_alias").is_none(),
        "no alias key at all: {status}"
    );
    assert_eq!(status["local_endpoint"], id.as_str(), "{status}");
    assert_no_leak(&mut bridge, &scratch.log_file(&id), &[&status], "claude");
}

/// Row 6: an id whose entry has an alias reports it.
#[test]
fn an_id_whose_entry_has_an_alias_reports_the_alias() {
    let scratch = Scratch::new();
    scratch.config(&[("claude", Some("claude"))], 0o644);
    let daemon = scratch.daemon(Answer::Lease);
    let id = claude_id();
    let mut bridge = scratch.bridge(&id);
    bridge.initialize();
    let status = bridge.status_when(leased);
    assert_hello_names(&daemon, &id, None);
    assert_eq!(status["local_endpoint_alias"], "claude", "{status}");
    assert!(
        scratch.log_file(&id).is_file(),
        "an id start's log is named for the id"
    );
}

/// Row 7: an id no entry holds starts, sends its hello, and shows the
/// daemon's refusal as every endpoint refusal is shown.
#[test]
fn an_id_in_no_entry_starts_and_meets_the_daemons_refusal() {
    let scratch = Scratch::new();
    scratch.config(&[("claude", Some("claude"))], 0o644);
    let daemon = scratch.daemon(Answer::Close("EndpointUnknown"));
    let stranger = endpoint_id("stranger").as_str().to_owned();
    let mut bridge = scratch.bridge(&stranger);
    bridge.initialize();
    let status = bridge.status_when(|s| !s["last_connect_error"].is_null());
    assert_hello_names(&daemon, &stranger, None);
    assert_eq!(status["last_connect_error"], "EndpointUnknown", "{status}");
    assert_eq!(
        status["endpoint_lease_state"], "endpoint refused",
        "{status}"
    );
    assert!(
        status.get("refused").is_none(),
        "not a refused start: {status}"
    );
    assert!(status.get("local_endpoint_alias").is_none(), "{status}");
}

/// Row 8: an uppercase alias is no alias and no id: a usage refusal.
#[test]
fn an_uppercase_endpoint_is_a_usage_refusal() {
    let scratch = Scratch::new();
    scratch.config(&[("claude", Some("claude"))], 0o644);
    let daemon = scratch.daemon(Answer::Lease);
    let mut bridge = scratch.bridge("Claude");
    assert_refused_class(&mut bridge, "usage", &[]);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(daemon.accepted(), 0, "a usage refusal opens no connection");
}

/// Row 11: the alias and the id name distinct log files in one profile.
#[test]
fn the_alias_and_the_id_write_distinct_log_files() {
    let scratch = Scratch::new();
    scratch.config(&[("claude", Some("claude"))], 0o644);
    let daemon = scratch.daemon(Answer::Lease);
    let id = claude_id();
    for endpoint in ["claude", id.as_str()] {
        let mut bridge = scratch.bridge(endpoint);
        bridge.initialize();
        bridge.status_when(leased);
        // The first is dropped, killed and reaped, before the second
        // starts: one lease at a time, as a daemon would hold it.
    }
    assert!(daemon.accepted() >= 2);
    let (by_alias, by_id) = (scratch.log_file("claude"), scratch.log_file(&id));
    assert_ne!(by_alias, by_id);
    for file in [by_alias, by_id] {
        assert!(
            std::fs::read_to_string(&file)
                .expect("the log file")
                .contains("a session opened, the lease held"),
            "{} holds its own start",
            file.display()
        );
    }
}

/// Restores a directory's mode however the test ends, so the tempdir can
/// be removed.
struct Restore(PathBuf);

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
    }
}

/// Row 10: a key directory the account cannot read costs the start
/// nothing.
#[test]
fn an_unreadable_key_directory_does_not_stop_an_alias_start() {
    let scratch = Scratch::new();
    scratch.config(&[("claude", Some("claude"))], 0o644);
    create_private_dir(&scratch.keys_dir()).expect("keys dir");
    let _restore = Restore(scratch.keys_dir());
    std::fs::set_permissions(scratch.keys_dir(), std::fs::Permissions::from_mode(0o000))
        .expect("chmod 000");
    let daemon = scratch.daemon(Answer::Lease);
    let mut bridge = scratch.bridge("claude");
    bridge.initialize();
    let status = bridge.status_when(leased);
    assert_hello_names(&daemon, &claude_id(), Some("claude"));
    assert_eq!(status["local_endpoint_alias"], "claude", "{status}");
}

/// Row 14: another entry's key file whose header does not derive its id
/// must not stop the start of this entry. A start resolves an alias from
/// the configuration alone and has no use for a key header.
#[test]
fn another_entrys_mismatched_key_header_does_not_stop_an_alias_start() {
    let scratch = Scratch::new();
    scratch.config(
        &[("claude", Some("claude")), ("other", Some("other"))],
        0o644,
    );
    // A record whose header names `other`'s id beside a public key that
    // does not derive it: `EndpointKeyMismatch`, not an unreadable file.
    let header = endpoint_key::header_bytes(
        Custody::FilesystemOnly,
        endpoint_id("other").as_str().as_bytes(),
        &[7_u8; 32],
    )
    .expect("a header");
    let line = endpoint_key::encode_record(&header, &[9_u8; 32]);
    scratch.key_file("other", &format!("{}\n", line.as_str()));
    let daemon = scratch.daemon(Answer::Lease);
    let mut bridge = scratch.bridge("claude");
    bridge.initialize();
    let status = bridge.status_when(|s| leased(s) || !s["refused"].is_null());
    assert!(status.get("refused").is_none(), "refused: {status}");
    assert_hello_names(&daemon, &claude_id(), Some("claude"));
}

/// Row 15: the bridge reads the endpoint overlay for an alias and never
/// writes it. An overlay that normalising changes -- `load_within` would
/// rewrite it -- is left byte for byte, with its inode and mtime, by a
/// start that resolved through it. The control is the same file made
/// other-readable: the start is refused, so the overlay was read.
#[test]
fn an_alias_start_reads_the_overlay_and_writes_nothing() {
    let overlay_text = format!(r#"{{"enabled":{{"{}":true}}}}"#, claude_id());
    let write_overlay = |scratch: &Scratch, mode: u32| -> PathBuf {
        create_private_dir(scratch.paths.state_dir()).expect("state dir");
        let path = scratch.paths.state_dir().join("endpoint-overlay.json");
        std::fs::write(&path, &overlay_text).expect("write overlay");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("chmod");
        path
    };

    let scratch = Scratch::new();
    scratch.config(&[("claude", Some("claude"))], 0o644);
    let path = write_overlay(&scratch, 0o600);
    let before = std::fs::metadata(&path).expect("stat");
    let daemon = scratch.daemon(Answer::Lease);
    let mut bridge = scratch.bridge("claude");
    bridge.initialize();
    let status = bridge.status_when(leased);
    assert_hello_names(&daemon, &claude_id(), Some("claude"));
    assert_eq!(status["local_endpoint_alias"], "claude", "{status}");
    let after = std::fs::metadata(&path).expect("stat");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), overlay_text);
    assert_eq!(
        (
            before.ino(),
            before.mode(),
            before.mtime(),
            before.mtime_nsec()
        ),
        (after.ino(), after.mode(), after.mtime(), after.mtime_nsec())
    );

    let scratch = Scratch::new();
    scratch.config(&[("claude", Some("claude"))], 0o644);
    write_overlay(&scratch, 0o644);
    let daemon = scratch.daemon(Answer::Lease);
    let mut bridge = scratch.bridge("claude");
    bridge.initialize();
    let status = bridge.status();
    assert_eq!(status["refused"]["class"], "endpoint_alias", "{status}");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(daemon.accepted(), 0, "no connection");
}

/// Row 12: a daemon still on major 2. The hello offers major 3; the
/// daemon answers `close{VersionIncompatible}` listing 2.13, as a 2.x
/// server answers a major it does not speak. The bridge reports the
/// daemon unavailable with `VersionIncompatible` as the last connect
/// error, keeps its id and alias, and goes on reconnecting, so a daemon
/// upgraded while it runs is met (packaging/claude-plugin/INSTALL.md,
/// "Upgrading from a release before Stage 22's").
#[test]
fn a_daemon_on_major_two_is_reported_by_version_and_retried() {
    let scratch = Scratch::new();
    scratch.config(&[("claude", Some("claude"))], 0o644);
    let daemon = scratch.daemon(Answer::MajorTwo);
    let mut bridge = scratch.bridge("claude");
    bridge.initialize();
    let status = bridge.status_when(|s| !s["last_connect_error"].is_null());
    assert_eq!(
        status["last_connect_error"], "VersionIncompatible",
        "{status}"
    );
    assert_eq!(
        status["endpoint_lease_state"], "daemon unavailable",
        "{status}"
    );
    assert_eq!(status["local_endpoint"], claude_id(), "{status}");
    assert_eq!(status["local_endpoint_alias"], "claude", "{status}");
    assert!(
        status.get("refused").is_none(),
        "not a refused start: {status}"
    );
    let hello = daemon.first_hello();
    assert_eq!(hello["ipc_version"]["major"], IPC_MAJOR, "{hello}");
    let deadline = Instant::now() + PATIENCE;
    while daemon.accepted() < 2 {
        assert!(Instant::now() < deadline, "the bridge never reconnected");
        std::thread::sleep(Duration::from_millis(20));
    }
}
