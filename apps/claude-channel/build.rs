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

use std::path::Path;
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

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rerun-if-changed=build.rs");
    // The full id cut to twelve: `--short=12` is a minimum git lengthens
    // when twelve digits are ambiguous among the objects a clone holds.
    let commit = git(dir, &["rev-parse", "HEAD"]).map(|id| id.chars().take(12).collect::<String>());
    // Rebuilt when HEAD moves. HEAD's reflog is appended on every move,
    // which is what catches a commit on a branch git has packed: the commit
    // writes a loose ref that did not exist when this ran, and touches
    // neither HEAD nor packed-refs (measured, InterWeave #398 F1). HEAD,
    // the branch's loose ref and packed-refs stay for a clone with no
    // reflog. A path that does not exist is left out, since cargo reruns
    // every build for one.
    let mut watched = vec![
        "logs/HEAD".to_owned(),
        "HEAD".to_owned(),
        "packed-refs".to_owned(),
    ];
    if let Some(branch) = git(dir, &["symbolic-ref", "-q", "HEAD"]) {
        watched.push(branch);
    }
    for name in watched {
        if let Some(path) = git(
            dir,
            &["rev-parse", "--path-format=absolute", "--git-path", &name],
        ) && Path::new(&path).exists()
        {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    println!(
        "cargo:rustc-env=INTERWEAVE_COMMIT={}",
        commit.as_deref().unwrap_or("unknown")
    );
}
