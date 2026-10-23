// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `transportctl --version` prints the workspace version with the
//! commit it was built from as build metadata (ADR-0065 decision 6) and
//! exits 0; beside any other argument it is a usage error and starts
//! nothing.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

/// The root manifest's `[workspace.package] version`, read from the file
/// rather than from `CARGO_PKG_VERSION`, which is where the binary takes
/// its own: a test of that would agree with the binary whatever the
/// workspace says.
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

/// HEAD's first twelve hex digits, or `unknown` outside a checkout, as
/// `tools/stamp/commit_stamp.rs` stamps it.
fn head() -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git runs");
    if out.status.success() {
        String::from_utf8(out.stdout).expect("utf-8").trim()[..12].to_owned()
    } else {
        "unknown".to_owned()
    }
}

fn transportctl() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_transportctl"));
    // No XDG tree: a start that got past the parser would stop at
    // resolving one, never at a host's real profile.
    command.env_clear();
    command
}

#[test]
fn version_is_the_workspace_version_and_the_commit() {
    let out = transportctl().arg("--version").output().expect("runs");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("transportctl {}+{}\n", workspace_version(), head())
    );
    assert!(out.stderr.is_empty(), "nothing on stderr");
}

#[test]
fn version_beside_another_argument_is_a_usage_error() {
    for args in [
        &["--version", "--profile", "a", "status"][..],
        &["--profile", "a", "status", "--version"][..],
    ] {
        let out = transportctl().args(args).output().expect("runs");
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}: no version line");
    }
}
