// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The binary as a script runs it, with no daemon: `--help` names every
//! verb and exit code, a usage error is exit 2, and a profile that cannot
//! be resolved is exit 1 before anything is opened. What a verb does
//! against a daemon is `tests/desktop-e2e/tests/human_cli.rs`'s. (Being
//! here also makes cargo build the binary for that suite.)

#![cfg(unix)]
#![allow(clippy::expect_used)]

use std::process::{Command, Output};

fn human(args: &[&str], xdg: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_interweave-human"))
        .args(args)
        .env_clear()
        .env("XDG_CONFIG_HOME", xdg.join("config"))
        .env("XDG_DATA_HOME", xdg.join("data"))
        .env("XDG_STATE_HOME", xdg.join("state"))
        .env("XDG_CACHE_HOME", xdg.join("cache"))
        .env("XDG_RUNTIME_DIR", xdg.join("run"))
        .output()
        .expect("the binary runs")
}

#[test]
fn help_names_every_verb_and_every_exit_code() {
    let xdg = tempfile::tempdir().expect("tempdir");
    let out = human(&["--help"], xdg.path());
    assert_eq!(out.status.code(), Some(0));
    let help = String::from_utf8(out.stdout).expect("utf-8");
    for verb in [
        "status",
        "conversations",
        "messages",
        "read",
        "send",
        "cancel|retry|unkeep",
        "listen",
        "trust list",
        "trust allow|revoke",
        "identities list",
        "identities enable|disable",
        "identities default",
        "store health",
        "store discard",
        "card show",
        "card set",
        "card share",
        "proposals read",
        "contacts show",
        "contacts rename",
        "contacts photo",
        "contacts delete",
        "peers add|remove",
        "peers show",
        "identity show",
        "identity new-key",
        "--accept-key-change",
        "contacts route delete",
        "peers add -|--file",
    ] {
        assert!(help.contains(verb), "{verb} in --help");
    }
    for code in 0..=6 {
        assert!(
            help.contains(&format!("\n  {code}  ")),
            "exit {code} in --help"
        );
    }
}

#[test]
fn a_usage_error_is_exit_2_and_prints_the_usage() {
    let xdg = tempfile::tempdir().expect("tempdir");
    let out = human(&["--profile", "p", "launch"], xdg.path());
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage:"));
}

#[test]
fn batch_fours_flags_out_of_place_are_exit_2() {
    let xdg = tempfile::tempdir().expect("tempdir");
    for args in [
        &[
            "--profile",
            "p",
            "identity",
            "new-key",
            "--confirm",
            "everything",
        ][..],
        &[
            "--profile",
            "p",
            "store",
            "discard",
            "--confirm",
            "key-change",
        ][..],
        &["--profile", "p", "read", "#c", "--accept-key-change", "u1"][..],
        &["--profile", "p", "peers", "remove", "--file", "f"][..],
        &["--profile", "p", "contacts", "route", "delete", "c1", "#c"][..],
    ] {
        let out = human(args, xdg.path());
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
    }
}

#[test]
fn a_profile_with_no_configuration_is_refused_with_exit_1() {
    let xdg = tempfile::tempdir().expect("tempdir");
    let out = human(&["--profile", "absent", "status"], xdg.path());
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(out.stdout.is_empty());
    assert!(
        !xdg.path().join("state").exists(),
        "nothing is created for a profile that does not resolve"
    );
}

/// `send ... -` reads stdin to its bound and refuses a text one byte past
/// it with exit 1, naming the bound, before the profile is resolved or
/// anything opened; at the bound the text is taken, and the absent profile
/// is what refuses it (the control). Mutation: the bound not checked.
#[test]
fn a_message_on_stdin_past_its_bound_is_refused_before_anything_opens() {
    use std::io::Write as _;
    // `MAX_SEND_STDIN_BYTES`, 256 KiB, as the README states it.
    const BOUND: usize = 256 * 1024;
    for (size, refused_for_size) in [(BOUND + 1, true), (BOUND, false)] {
        let xdg = tempfile::tempdir().expect("tempdir");
        let mut child = Command::new(env!("CARGO_BIN_EXE_interweave-human"))
            .args([
                "--profile",
                "absent",
                "send",
                "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN/human",
                "-",
            ])
            .env_clear()
            .env("XDG_CONFIG_HOME", xdg.path().join("config"))
            .env("XDG_DATA_HOME", xdg.path().join("data"))
            .env("XDG_STATE_HOME", xdg.path().join("state"))
            .env("XDG_CACHE_HOME", xdg.path().join("cache"))
            .env("XDG_RUNTIME_DIR", xdg.path().join("run"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("the binary runs");
        let mut stdin = child.stdin.take().expect("stdin");
        // Written whole or until the binary stops reading: a refusal at
        // the bound closes the pipe under the rest.
        let _ = stdin.write_all(&vec![b'a'; size]);
        drop(stdin);
        let out = child.wait_with_output().expect("it ends");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{size}: {stderr}");
        assert_eq!(
            stderr.contains("larger than 256 KiB"),
            refused_for_size,
            "{size}: {stderr}"
        );
        assert!(
            !xdg.path().join("state").exists(),
            "nothing is created for {size}"
        );
    }
}
