// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Stamps the commit the bridge is built from, for `claude-channel
//! --version`: the workspace version with the commit as Semantic Versioning
//! build metadata (ADR-0065), `0.1.0+<commit>`.
//!
//! The id is the first twelve hex digits, never git's abbreviation, which
//! grows with the repository's object count: a release's shallow clone and
//! a full one would stamp the same commit differently. Built from no
//! checkout, the commit is `unknown`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let text = text.trim();
    (out.status.success() && !text.is_empty()).then(|| text.to_owned())
}

/// `name` inside the git directory, as an absolute path.
fn git_path(dir: &Path, name: &str) -> Option<PathBuf> {
    git(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-path", name],
    )
    .map(PathBuf::from)
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rerun-if-changed=build.rs");
    // The full id cut to twelve: `--short=12` is a minimum git lengthens
    // when twelve digits are ambiguous among the objects a clone holds.
    let commit = git(dir, &["rev-parse", "HEAD"]).map(|id| id.chars().take(12).collect::<String>());
    // Rebuilt when HEAD moves. A commit on a branch git has packed writes
    // a loose ref that did not exist when this ran, and touches neither
    // HEAD nor packed-refs (measured, InterWeave #398 F1). HEAD's reflog
    // catches that when there is one; with reflogs off it does not exist,
    // so the nearest existing directory above the absent loose ref is
    // watched, which cargo scans, and the new ref is a change in it
    // (measured with core.logAllRefUpdates=false, #398). Any other path
    // that does not exist is left out, since cargo reruns every build for
    // one. A reftable repository has none of these but a stub HEAD: every
    // ref and reflog update writes a table under reftable/, the shared
    // refs in the common directory's and a linked worktree's HEAD in its
    // own, so both directories are watched where they exist (measured
    // with --ref-format=reftable, #398).
    for flag in ["--git-dir", "--git-common-dir"] {
        if let Some(dir) = git(dir, &["rev-parse", "--path-format=absolute", flag])
            .map(|d| PathBuf::from(d).join("reftable"))
            .filter(|p| p.exists())
        {
            println!("cargo:rerun-if-changed={}", dir.display());
        }
    }
    for name in ["logs/HEAD", "HEAD", "packed-refs"] {
        if let Some(path) = git_path(dir, name).filter(|p| p.exists()) {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    if let Some(branch) = git(dir, &["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git_path(dir, &branch)
        && let Some(watched) = path.ancestors().find(|p| p.exists())
    {
        println!("cargo:rerun-if-changed={}", watched.display());
    }
    println!(
        "cargo:rustc-env=INTERWEAVE_COMMIT={}",
        commit.as_deref().unwrap_or("unknown")
    );
}
