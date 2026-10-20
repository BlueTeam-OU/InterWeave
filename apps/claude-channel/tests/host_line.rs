// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A host line is bounded before it is parsed (`host_input`): the real
//! binary, its stdin and stdout pipes, fed a line far past the ceiling
//! with no newline for most of it. Its peak resident memory stays near
//! where it was, the line is answered with nothing, and the next line is
//! read and answered; a line that is not UTF-8 is dropped the same way
//! instead of ending the bridge.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used)]

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use interweave_claude_channel::host_input::MAX_HOST_LINE_BYTES;
use serde_json::Value;

/// Generous against a loaded host; an answer to `ping` is immediate.
const PROMPT: Duration = Duration::from_secs(10);

/// Far past the ceiling: a reader that kept the line would grow by this.
const FLOOD_BYTES: usize = 64 << 20;

/// What the peak may grow by: the ceiling, the pipe's and the reader's
/// buffers, and the allocator's slack, well under the flood.
const GROWTH_BOUND_BYTES: u64 = 16 << 20;

fn bridge(scratch: &std::path::Path) -> Child {
    let run = scratch.join("run");
    std::fs::create_dir_all(&run).expect("run dir");
    Command::new(env!("CARGO_BIN_EXE_claude-channel"))
        .args([
            "--profile",
            "host-line",
            "--endpoint",
            "claude",
            "--delivery",
            "push",
        ])
        .env("XDG_RUNTIME_DIR", &run)
        .env("XDG_STATE_HOME", scratch.join("state"))
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_CACHE_HOME", scratch.join("cache"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawns")
}

/// The process's peak resident set, from `/proc/<pid>/status`.
fn peak_rss_bytes(pid: u32) -> u64 {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).expect("status");
    let kib: u64 = status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .expect("VmHWM");
    kib * 1024
}

fn ping(stdin: &mut ChildStdin, id: u64) {
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":{id},"method":"ping"}}"#).expect("writes");
    stdin.flush().expect("flushes");
}

/// The next answer on stdout carrying an id; every line before it is a
/// notification, or the test fails on the id it carries.
fn next_answer(answers: &mpsc::Receiver<String>) -> Value {
    loop {
        let line = answers.recv_timeout(PROMPT).expect("an answer in time");
        let message: Value = serde_json::from_str(&line).expect("a JSON line");
        if message.get("id").is_some() {
            return message;
        }
    }
}

/// The running bridge, its answers read beside it and its stderr
/// drained beside it: a full stderr pipe would stall it.
struct Running {
    child: Child,
    stdin: ChildStdin,
    answers: mpsc::Receiver<String>,
    log: std::thread::JoinHandle<String>,
    _scratch: tempfile::TempDir,
}

impl Running {
    fn start() -> Self {
        let scratch = tempfile::tempdir().expect("scratch");
        let mut child = bridge(scratch.path());
        let stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        let mut stderr = child.stderr.take().expect("stderr");
        let log = std::thread::spawn(move || {
            let mut log = String::new();
            stderr.read_to_string(&mut log).expect("stderr");
            log
        });
        let (tx, answers) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut running = Self {
            child,
            stdin,
            answers,
            log,
            _scratch: scratch,
        };
        ping(&mut running.stdin, 1);
        assert_eq!(next_answer(&running.answers)["id"], 1, "the bridge answers");
        running
    }

    /// Its stdin closed: it ends cleanly, and its stderr is returned.
    fn end(self) -> String {
        let Self {
            mut child,
            stdin,
            log,
            ..
        } = self;
        drop(stdin);
        let status = child.wait().expect("waits");
        assert!(status.success(), "{status}");
        log.join().expect("stderr read")
    }
}

#[test]
fn a_line_past_the_ceiling_is_dropped_at_bounded_memory_and_the_next_is_answered() {
    let mut bridge = Running::start();
    let before = peak_rss_bytes(bridge.child.id());

    // A request whose id comes after its params, sent in pipe-sized
    // writes with the newline only at the end.
    bridge
        .stdin
        .write_all(br#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"send","arguments":{"content":""#)
        .expect("writes");
    let chunk = vec![b'a'; 64 << 10];
    for _ in 0..FLOOD_BYTES / chunk.len() {
        bridge.stdin.write_all(&chunk).expect("writes");
    }
    bridge.stdin.write_all(b"\"}},\"id\":7}\n").expect("writes");
    ping(&mut bridge.stdin, 2);
    let answer = next_answer(&bridge.answers);
    let growth = peak_rss_bytes(bridge.child.id()).saturating_sub(before);
    eprintln!("peak grew by {growth} bytes over a {FLOOD_BYTES}-byte line");
    assert!(
        growth < GROWTH_BOUND_BYTES,
        "the peak grew by {growth} bytes over a {FLOOD_BYTES}-byte line; the ceiling is {MAX_HOST_LINE_BYTES}"
    );
    assert_eq!(
        answer["id"], 2,
        "nothing answered the dropped line: {answer}"
    );
    assert!(answer.get("result").is_some(), "{answer}");

    let log = bridge.end();
    assert!(
        log.contains(&format!(
            "a host line over {MAX_HOST_LINE_BYTES} bytes was dropped"
        )),
        "{log}"
    );
    assert!(!log.contains("aaaa"), "no content reaches stderr");
}

#[test]
fn a_line_that_is_not_utf8_is_dropped_and_ends_nothing() {
    let mut bridge = Running::start();
    bridge
        .stdin
        .write_all(b"\xff\xfe not utf-8\n")
        .expect("writes");
    ping(&mut bridge.stdin, 2);
    assert_eq!(next_answer(&bridge.answers)["id"], 2);
    bridge.end();
}
