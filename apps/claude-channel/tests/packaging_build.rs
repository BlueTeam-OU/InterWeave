// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `packaging/claude-plugin/build.sh`, run on a copy of the package source
//! in a scratch git repository, with this crate's own binary as the bridge.
//! The marketplace it assembles holds exactly the files git tracks there
//! plus `interweave/bin/claude-channel`; a file git does not track in the
//! source, ignored or not, fails the build and creates nothing; and an
//! out-dir inside the repository, or under a parent that does not exist,
//! is refused before anything is written.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used)]

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PACKAGE: &str = "packaging/claude-plugin";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn git(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "core.hooksPath=/dev/null"])
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("git runs")
}

fn lines(out: &Output) -> BTreeSet<String> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// A scratch repository holding the package source as this checkout
/// tracks it, at the same path, so `build.sh` finds its repository root
/// two levels up as it does here.
struct Scratch {
    _dir: tempfile::TempDir,
    repo: PathBuf,
    out_root: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("scratch repository");
        let repo = dir.path().join("repo");
        let source = workspace();
        let listed = git(&source, &["ls-files", "--", PACKAGE]);
        assert!(
            listed.status.success(),
            "the checkout lists its package files"
        );
        for rel in lines(&listed) {
            let dest = repo.join(&rel);
            fs::create_dir_all(dest.parent().expect("a parent")).expect("mkdir");
            fs::copy(source.join(&rel), &dest).expect("copy a tracked file");
        }
        assert!(git(&repo, &["init", "-q"]).status.success(), "git init");
        assert!(
            git(&repo, &["add", "--", PACKAGE]).status.success(),
            "git add"
        );
        // Committed, so a clean tree reads as clean in `git status`.
        let commit = git(
            &repo,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "-m",
                "package source",
            ],
        );
        assert!(
            commit.status.success(),
            "git commit: {}",
            String::from_utf8_lossy(&commit.stderr)
        );
        Self {
            _dir: dir,
            repo,
            out_root: tempfile::tempdir().expect("out root"),
        }
    }

    fn package(&self) -> PathBuf {
        self.repo.join(PACKAGE)
    }

    fn build(&self, out: &Path) -> Output {
        Command::new("bash")
            .arg(self.package().join("build.sh"))
            .arg(out)
            .arg(env!("CARGO_BIN_EXE_claude-channel"))
            .output()
            .expect("build.sh runs")
    }

    /// What git tracks under the package source, relative to it, as the
    /// marketplace should hold it: the plugin and the marketplace manifest.
    fn tracked(&self) -> BTreeSet<String> {
        let out = git(
            &self.package(),
            &[
                "ls-files",
                "--",
                ".claude-plugin/marketplace.json",
                "interweave",
            ],
        );
        lines(&out)
    }
}

fn files_under(root: &Path) -> BTreeSet<String> {
    fn walk(root: &Path, dir: &Path, acc: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(root, &path, acc);
            } else {
                acc.insert(
                    path.strip_prefix(root)
                        .expect("under root")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    let mut acc = BTreeSet::new();
    walk(root, root, &mut acc);
    acc
}

#[test]
fn the_marketplace_is_the_tracked_files_and_the_binary() {
    let s = Scratch::new();
    let out = s.out_root.path().join("marketplace");
    let run = s.build(&out);
    assert!(
        run.status.success(),
        "build.sh: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let mut expected = s.tracked();
    assert!(
        expected.contains("interweave/.mcp.json"),
        "the source tracks the plugin: {expected:?}"
    );
    expected.insert("interweave/bin/claude-channel".to_owned());
    assert_eq!(files_under(&out), expected);

    let mode = fs::metadata(out.join("interweave/bin/claude-channel"))
        .expect("binary")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);
}

#[test]
fn an_untracked_file_in_the_source_fails_the_build() {
    let s = Scratch::new();
    let bin = s.package().join("interweave/bin");
    fs::create_dir_all(&bin).expect("mkdir bin");
    fs::write(bin.join("claude-channel"), b"stale").expect("a stale binary");

    let out = s.out_root.path().join("marketplace");
    let run = s.build(&out);
    assert!(!run.status.success(), "a stray file must fail the build");
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("interweave/bin/claude-channel"),
        "it is named"
    );
    assert!(!out.exists(), "nothing is assembled");
}

#[test]
fn an_ignored_file_in_the_source_fails_the_build_too() {
    let s = Scratch::new();
    fs::write(
        s.repo.join(".gitignore"),
        format!("{PACKAGE}/interweave/bin/\n"),
    )
    .expect("ignore bin/");
    let bin = s.package().join("interweave/bin");
    fs::create_dir_all(&bin).expect("mkdir bin");
    fs::write(bin.join("left-over"), b"x").expect("an ignored file");
    let status = git(&s.repo, &["status", "--porcelain", "--", PACKAGE]);
    assert!(
        lines(&status).is_empty(),
        "the control: git status shows nothing: {:?}",
        lines(&status)
    );

    let out = s.out_root.path().join("marketplace");
    let run = s.build(&out);
    assert!(!run.status.success(), "an ignored file must fail the build");
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("interweave/bin/left-over"),
        "it is named"
    );
    assert!(!out.exists(), "nothing is assembled");
}

#[test]
fn an_out_dir_in_the_repository_is_refused_before_anything_is_written() {
    let s = Scratch::new();
    for out in [
        s.repo.join("dist/marketplace"),
        s.repo.join("marketplace"),
        s.package().join("out"),
    ] {
        let run = s.build(&out);
        assert!(!run.status.success(), "{} must be refused", out.display());
        assert!(!out.exists(), "{} was created", out.display());
    }
    assert!(
        !s.repo.join("dist").exists(),
        "a missing parent is not created"
    );
    let status = git(&s.repo, &["status", "--porcelain", "--untracked-files=all"]);
    assert!(
        lines(&status).is_empty(),
        "the tree is untouched: {:?}",
        lines(&status)
    );
}
