// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `claude-channel --version` prints the workspace version with the commit
//! it was built from as build metadata (ADR-0065), and exits; with any other
//! argument beside it, the flag is not a version request.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

fn workspace_version() -> String {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml"))
            .expect("the workspace manifest");
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
        } else if in_package && let Some(value) = line.strip_prefix("version = ") {
            return value.trim_matches('"').to_owned();
        }
    }
    panic!("no [workspace.package] version");
}

#[test]
fn version_is_the_workspace_version_and_the_commit() {
    let head = Command::new("git")
        .arg("-C")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git runs");
    let commit = if head.status.success() {
        String::from_utf8(head.stdout).expect("utf-8").trim()[..12].to_owned()
    } else {
        "unknown".to_owned()
    };

    let out = Command::new(env!("CARGO_BIN_EXE_claude-channel"))
        .arg("--version")
        .output()
        .expect("the bridge runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("claude-channel {}+{commit}\n", workspace_version())
    );
    assert!(out.stderr.is_empty(), "nothing on stderr");
}

#[test]
fn version_beside_another_argument_is_not_a_version_request() {
    let out = Command::new(env!("CARGO_BIN_EXE_claude-channel"))
        .args(["--version", "--profile"])
        .env_remove("XDG_RUNTIME_DIR")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("the bridge runs");
    assert!(
        !String::from_utf8_lossy(&out.stdout).starts_with("claude-channel "),
        "no version line: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("usage: claude-channel"),
        "the start is refused as a usage error: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
