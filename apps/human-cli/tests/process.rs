// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The binary as a script runs it, with no daemon: `--help` names every
//! verb and exit code, a usage error is exit 2, and a profile that cannot
//! be resolved is exit 1 before anything is opened. What a verb does
//! against a daemon is `tests/desktop-e2e/tests/human_cli.rs`'s. (Being
//! here also makes cargo build the binary for that suite.)

#![cfg(unix)]
#![allow(clippy::expect_used)]

use interweave_test_support::endpoint_keys::endpoint_id;
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
        // No command since ADR-0067 decision 5, its old flag with it.
        &[
            "--profile",
            "p",
            "identity",
            "new-key",
            "--confirm",
            "key-change",
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
                &format!(
                    "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN/{}",
                    endpoint_id("human").as_str()
                ),
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

/// A profile `p` under private roots in `xdg`, two human entries: `human`
/// and `spare`, by key. The daemon's data socket is returned listening, and
/// counts what connects to it.
fn profile_with_a_listening_socket(
    xdg: &std::path::Path,
) -> (
    interweave_profile_config::ProfilePaths,
    std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    use interweave_profile_config::{ProfilePaths, XdgRoots};
    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
    for sub in ["config", "data", "state", "cache", "run"] {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(xdg.join(sub))
            .expect("an xdg dir");
    }
    let roots = XdgRoots {
        config_home: xdg.join("config"),
        data_home: xdg.join("data"),
        state_home: xdg.join("state"),
        cache_home: xdg.join("cache"),
        runtime_dir: Some(xdg.join("run")),
    };
    let paths = ProfilePaths::resolve("p", &roots).expect("paths");
    let file = paths.config_file();
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(file.parent().expect("a parent"))
        .expect("the config dir");
    let entry = |alias: &str| {
        format!(
            "    - {{ key: {}.key, alias: {alias}, enabled: true, advertise: false, \
             allowed_client_kinds: [human-client], inbound: inherit_profile_trust, \
             outbound: inherit_profile_trust }}\n",
            endpoint_id(alias).as_str()
        )
    };
    let text = format!(
        "schema_version: 2
runtime: {{ deployment: daemon-ipc }}
profile: {{ name: p }}
transport:
  backend: libp2p
  listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/0\"] }}
identity: {{ algorithm: ed25519 }}
trust: {{ policy: static-allowlist, allowed_peers: [] }}
endpoints:
  registration_policy: configured-only
  default_direct_endpoint: human
  entries:
{}{}ipc: {{ enabled: true, socket_layout: split-data-admin }}
channels: {{ desired: [] }}
",
        entry("human"),
        entry("spare")
    );
    std::fs::write(&file, text).expect("config");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).expect("a mode");

    let socket = paths.data_socket().expect("socket");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(socket.parent().expect("a parent"))
        .expect("the run dir");
    let listener = std::os::unix::net::UnixListener::bind(socket).expect("bind");
    let connections = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = connections.clone();
    // Closes each connection at once: the client gets no hello answer and
    // ends, which is all the control needs.
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            drop(stream);
        }
    });
    (paths, connections)
}

/// A data verb run as an id a pending-add record names is refused with
/// exit 1 before it connects to the data socket (ADR-0061 decision 7); the
/// same run as the same id with no record connects (the control).
/// Mutation: `refuse_pending_add` not called.
#[test]
fn a_run_naming_an_id_a_pending_add_record_names_is_refused_before_any_socket() {
    use interweave_human_store::key_custody::FilesystemCustody;
    use interweave_human_store::{HumanStore, StoreOptions};
    use std::sync::atomic::Ordering::SeqCst;

    for (record, refused) in [(true, true), (false, false)] {
        let xdg = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(
            xdg.path(),
            std::os::unix::fs::PermissionsExt::from_mode(0o700),
        )
        .expect("private");
        let (paths, connections) = profile_with_a_listening_socket(xdg.path());
        if record {
            let mut store = HumanStore::open_profile(
                &paths,
                &FilesystemCustody,
                &endpoint_id("human"),
                StoreOptions::default(),
            )
            .expect("the store opens");
            store
                .add_pending_add(&endpoint_id("spare"))
                .expect("the record");
        }
        let out = human(
            &[
                "--profile",
                "p",
                "send",
                &format!(
                    "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN/{}",
                    endpoint_id("human").as_str()
                ),
                "hello",
                "--endpoint",
                "spare",
            ],
            xdg.path(),
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        // The listener counts on its own thread: the run has ended, so what
        // it accepted is settled once it has had a moment to see the close.
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert_eq!(connections.load(SeqCst) == 0, refused, "{record}: {stderr}");
        assert_eq!(
            stderr.contains("is still being added"),
            refused,
            "{record}: {stderr}"
        );
        if refused {
            assert_eq!(out.status.code(), Some(1), "{stderr}");
            assert!(stderr.contains(endpoint_id("spare").as_str()), "{stderr}");
        }
    }
}
