// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Two threads' first stores in one process both open (j263): the
//! engine's start-up is finished before any connection is keyed. The
//! race is once per process, so the measurement is fresh processes.
//! Without `crate::engine`'s start 5 of 1500 of them failed one of their
//! eight opens with "An error occurred with PRAGMA key or rekey"; with it
//! none did. Ignored by default: 1500 processes take two to eleven
//! minutes, by the host's load.
//! `cargo test -p interweave-human-store --test first_open_race -- --ignored`

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::os::unix::fs::PermissionsExt as _;
use std::sync::{Arc, Barrier};

use interweave_human_store::{HumanStore, StoreOptions, key_custody::FilesystemCustody};
use interweave_profile_config::{ProfilePaths, XdgRoots};

const THREADS: usize = 8;

fn fresh_paths() -> (tempfile::TempDir, ProfilePaths) {
    let outer = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in("/tmp")
        .expect("tempdir under /tmp");
    let roots = XdgRoots {
        config_home: outer.path().join("config"),
        data_home: outer.path().join("data"),
        state_home: outer.path().join("state"),
        cache_home: outer.path().join("cache"),
        runtime_dir: None,
    };
    let paths = ProfilePaths::resolve_offline("default", &roots).expect("desktop paths");
    (outer, paths)
}

/// The child: `THREADS` threads released together, each one's open the
/// process's first use of the engine. Runs only under the parent.
#[test]
fn child_first_opens() {
    if std::env::var_os("FIRST_OPEN_CHILD").is_none() {
        return;
    }
    let barrier = Arc::new(Barrier::new(THREADS));
    let failed: Vec<String> = (0..THREADS)
        .map(|_| {
            let barrier = barrier.clone();
            let (dir, paths) = fresh_paths();
            std::thread::spawn(move || {
                barrier.wait();
                let opened =
                    HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default());
                drop(dir);
                opened.err().map(|e| format!("{e:?}"))
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .filter_map(|h| h.join().expect("join"))
        .collect();
    assert!(failed.is_empty(), "first opens failed: {failed:#?}");
}

#[test]
#[ignore = "1500 fresh processes, minutes"]
fn the_first_opens_of_two_threads_both_open() {
    let runs: usize = std::env::var("FIRST_OPEN_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1500);
    let exe = std::env::current_exe().expect("this test binary");
    let mut failed = Vec::new();
    for run in 0..runs {
        let out = std::process::Command::new(&exe)
            .args(["--exact", "child_first_opens", "--test-threads=1"])
            .env("FIRST_OPEN_CHILD", "1")
            .output()
            .expect("run the child");
        if !out.status.success() {
            failed.push((run, String::from_utf8_lossy(&out.stdout).into_owned()));
        }
    }
    assert!(
        failed.is_empty(),
        "{} of {runs} processes failed a first open; the first: {:?}",
        failed.len(),
        failed.first()
    );
}
