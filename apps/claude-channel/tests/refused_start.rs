// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A start that cannot work is served, not exited (`src/refused.rs`): the
//! real binary, started as Claude Code starts it, with its stdin a pipe
//! held open. Each bad start answers `initialize` with no
//! `claude/channel` capability, reports its class from `status`, is still
//! running after its answers, and ends on the first SIGINT. The control:
//! a start that can work declares the capability.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used)]

use std::io::{BufRead as _, BufReader, Write as _};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::Value;

/// Generous against a loaded host; every answer here is immediate.
const PROMPT: Duration = Duration::from_secs(10);

struct Bridge {
    child: Child,
    lines: mpsc::Receiver<String>,
}

impl Bridge {
    fn start(args: &[&str], env: &[(&str, &std::path::Path)], unset: &[&str]) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_claude-channel"));
        command.args(args);
        for (key, value) in env {
            command.env(key, value);
        }
        for key in unset {
            command.env_remove(key);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
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
        Self { child, lines }
    }

    fn ask(&mut self, id: u64, method: &str, params: &Value) -> Value {
        let request =
            serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let stdin = self.child.stdin.as_mut().expect("stdin held");
        writeln!(stdin, "{request}").expect("written");
        stdin.flush().expect("flushed");
        let line = self.lines.recv_timeout(PROMPT).expect("answered");
        serde_json::from_str(&line).expect("json")
    }

    fn status(&mut self) -> Value {
        let answer = self.ask(2, "tools/call", &serde_json::json!({"name": "status"}));
        let text = answer["result"]["content"][0]["text"]
            .as_str()
            .expect("text");
        serde_json::from_str(text).expect("status is json")
    }

    fn running(&mut self) -> bool {
        self.child.try_wait().expect("waitable").is_none()
    }

    fn ends_on_sigint(mut self) -> bool {
        let killed = Command::new("kill")
            .args(["-INT", &self.child.id().to_string()])
            .status()
            .expect("kill runs");
        assert!(killed.success());
        let deadline = Instant::now() + PROMPT;
        while Instant::now() < deadline {
            if self.child.try_wait().expect("waitable").is_some() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        false
    }
}

fn initialize(bridge: &mut Bridge) -> Value {
    bridge.ask(
        1,
        "initialize",
        &serde_json::json!({"protocolVersion": "2025-11-25", "capabilities": {}}),
    )
}

fn scratch_env(dir: &std::path::Path) -> Vec<(&'static str, std::path::PathBuf)> {
    let run = dir.join("run");
    std::fs::create_dir_all(&run).expect("run dir");
    vec![
        ("XDG_RUNTIME_DIR", run),
        ("XDG_STATE_HOME", dir.join("state")),
        ("XDG_CONFIG_HOME", dir.join("config")),
        ("XDG_DATA_HOME", dir.join("data")),
        ("XDG_CACHE_HOME", dir.join("cache")),
    ]
}

fn assert_refused(args: &[&str], unset: &[&str], class: &str) {
    let scratch = tempfile::tempdir().expect("tempdir");
    let env = scratch_env(scratch.path());
    let env: Vec<(&str, &std::path::Path)> = env.iter().map(|(k, v)| (*k, v.as_path())).collect();
    let mut bridge = Bridge::start(args, &env, unset);
    let init = initialize(&mut bridge);
    let result = &init["result"];
    assert!(
        result["capabilities"].get("experimental").is_none(),
        "{class}: a refused start declares no claude/channel: {result}"
    );
    assert!(
        result["instructions"]
            .as_str()
            .expect("instructions")
            .contains("did not start"),
        "{class}: the instructions say why"
    );
    let status = bridge.status();
    assert_eq!(
        status["endpoint_lease_state"], "bridge refused",
        "{class}: {status}"
    );
    assert_eq!(status["refused"]["class"], class, "{status}");
    let send = bridge.ask(
        3,
        "tools/call",
        &serde_json::json!({"name": "send", "arguments": {}}),
    );
    assert_eq!(
        send["result"]["isError"], true,
        "{class}: a tool that would act is an error"
    );
    assert!(bridge.running(), "{class}: still serving after its answers");
    assert!(
        bridge.ends_on_sigint(),
        "{class}: the first SIGINT ends it, stdin held open"
    );
}

const GOOD: [&str; 6] = [
    "--profile",
    "refused",
    "--endpoint",
    "claude",
    "--delivery",
    "push",
];

#[test]
fn an_unset_runtime_dir_is_served_as_a_refusal() {
    assert_refused(&GOOD, &["XDG_RUNTIME_DIR"], "runtime_dir_unset");
}

#[test]
fn a_profile_name_that_is_not_valid_is_served_as_a_refusal() {
    assert_refused(
        &[
            "--profile",
            "../escape",
            "--endpoint",
            "claude",
            "--delivery",
            "push",
        ],
        &[],
        "profile_name",
    );
}

#[test]
fn a_usage_error_is_served_as_a_refusal() {
    assert_refused(
        &["--profile", "refused", "--endpoint", "claude"],
        &[],
        "usage",
    );
}

#[test]
fn the_control_a_start_that_can_work_declares_the_channel() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let env = scratch_env(scratch.path());
    let env: Vec<(&str, &std::path::Path)> = env.iter().map(|(k, v)| (*k, v.as_path())).collect();
    let mut bridge = Bridge::start(&GOOD, &env, &[]);
    let init = initialize(&mut bridge);
    assert!(
        init["result"]["capabilities"]["experimental"]
            .get("claude/channel")
            .is_some(),
        "a good start, the daemon away, still declares the channel: {init}"
    );
    assert_ne!(bridge.status()["endpoint_lease_state"], "bridge refused");
    assert!(bridge.ends_on_sigint());
}
