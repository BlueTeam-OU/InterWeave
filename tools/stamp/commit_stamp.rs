// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The build script of every program whose `--version` names the commit
//! it was built from (ADR-0065 decision 6): the workspace version with the
//! commit as Semantic Versioning build metadata, `0.1.0+<commit>`. A
//! package uses it with `build = "../../tools/stamp/commit_stamp.rs"` and
//! reads `env!("INTERWEAVE_COMMIT")`; one file, so two programs cannot
//! stamp one commit two ways.
//!
//! The id is the first twelve hex digits, never git's abbreviation, which
//! grows with the repository's object count: a release's shallow clone and
//! a full one would stamp the same commit differently. Built from no
//! checkout, the commit is `unknown`. Same rules as `claude-channel`'s
//! own build script (#398).

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
    // No rerun line for this file: an edit recompiles the script, and a
    // recompiled script runs again.
    // The full id cut to twelve: `--short=12` is a minimum git lengthens
    // when twelve digits are ambiguous among the objects a clone holds.
    let commit = git(dir, &["rev-parse", "HEAD"]).map(|id| id.chars().take(12).collect::<String>());
    // Rebuilt when HEAD moves. HEAD's reflog is appended on every move,
    // which is what catches a commit on a branch git has packed: the commit
    // writes a loose ref that did not exist when this ran, and touches
    // neither HEAD nor packed-refs (measured, #398 F1). HEAD, the branch's
    // loose ref and packed-refs stay for a clone with no reflog. A path
    // that does not exist is left out, since cargo reruns every build for
    // one.
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
