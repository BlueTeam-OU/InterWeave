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
//! checkout, the commit is `unknown`. `tools/ci/test_commit_stamp.sh` is
//! this script's test.

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
    // loose ref and packed-refs stay for a clone with no reflog -- and
    // there a packed branch has no loose ref yet, so the nearest directory
    // above where it will be written is watched instead: cargo scans a
    // directory for changes, and the commit's new file is one (reflogs
    // off and refs packed, measured; #386 review). Any other path that
    // does not exist is left out, since cargo reruns every build for one.
    // A reftable repository has none of those but HEAD, a stub no move
    // touches: every ref and reflog update writes a table into a reftable/
    // directory -- the shared refs' in the common directory, a linked
    // worktree's HEAD in its own, which `--git-path` alone names -- so
    // both directories are watched where they exist (#386 review).
    let mut watched = vec![
        ("logs/HEAD".to_owned(), false),
        ("HEAD".to_owned(), false),
        ("packed-refs".to_owned(), false),
    ];
    if let Some(branch) = git(dir, &["symbolic-ref", "-q", "HEAD"]) {
        watched.push((branch, true));
    }
    let mut paths = Vec::new();
    for (name, or_its_directory) in watched {
        if let Some(path) = git(
            dir,
            &["rev-parse", "--path-format=absolute", "--git-path", &name],
        ) {
            paths.push((PathBuf::from(path), or_its_directory));
        }
    }
    for which in ["--git-dir", "--git-common-dir"] {
        if let Some(path) = git(dir, &["rev-parse", "--path-format=absolute", which]) {
            paths.push((Path::new(&path).join("reftable"), false));
        }
    }
    for (path, or_its_directory) in &paths {
        let path = path.as_path();
        let watch = if *or_its_directory {
            path.ancestors().find(|p| p.exists())
        } else {
            path.exists().then_some(path)
        };
        if let Some(watch) = watch {
            println!("cargo:rerun-if-changed={}", watch.display());
        }
    }
    println!(
        "cargo:rustc-env=INTERWEAVE_COMMIT={}",
        commit.as_deref().unwrap_or("unknown")
    );
}
