// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

//! One network namespace whose loopback holds the addresses a test
//! names, inside an unprivileged user namespace ([`on_addresses`]): for
//! a test of an address class this host has no interface in. Std only,
//! so a suite reaches it without the `e2e` feature's harness;
//! `e2e::netns` is the heavier topology of several namespaces, NATs
//! and a relay.

/// Set in the inner copy [`on_addresses`] runs.
const ON_ADDRESSES_ENV: &str = "IW_ON_ADDRESSES";

/// How long the inner copy may run before it is killed.
const ON_ADDRESSES_DEADLINE: std::time::Duration = std::time::Duration::from_secs(300);

/// Run the calling test where this host holds exactly `ips` beside
/// loopback: a fresh network namespace inside an unprivileged user
/// namespace, `ips` added to its loopback interface, for a test whose
/// address class the host has no interface in -- the shared address
/// space (RFC 6598) on a host with none, say. `true` in the inner copy,
/// which runs the body; `false` in the outer one, which has asserted that
/// the inner one passed. Lighter than `e2e::netns::in_second_network`:
/// an in-process runtime judges no profile directory's ancestors, so it
/// needs no root of its own, and one namespace with no route out is the
/// whole network.
///
/// # Panics
/// In the outer copy, when the namespace cannot be made (no unprivileged
/// user namespace, no util-linux `unshare`/`setpriv`, no iproute2) or
/// the inner copy did not pass -- never a skip: a test that cannot ask
/// its question has not answered it.
#[must_use]
pub fn on_addresses(test: &str, ips: &[std::net::Ipv4Addr]) -> bool {
    if std::env::var_os(ON_ADDRESSES_ENV).is_some() {
        return true;
    }
    let exe = std::env::current_exe().expect("the test binary's path");
    let adds = ips.iter().fold(String::new(), |mut adds, ip| {
        use std::fmt::Write as _;
        let _ = write!(adds, "ip addr add {ip}/32 dev lo && ");
        adds
    });
    let script = format!(
        "set -e; ip link set lo up && {adds}export {ON_ADDRESSES_ENV}=1; \
         exec \"$IW_TEST_EXE\" --exact \"$IW_TEST_NAME\" --include-ignored --nocapture --test-threads 1"
    );
    // The inner copy dies with this test however it ends: the kernel
    // kills `unshare` when the spawning thread -- this one, which waits
    // below -- goes, and the pid namespace's `--kill-child` takes the
    // copy with it; the deadline kills `unshare` on a hang.
    let child = std::process::Command::new("setpriv")
        .args([
            "--pdeathsig",
            "KILL",
            "unshare",
            "-rn",
            "--pid",
            "--fork",
            "--kill-child",
            "sh",
            "-c",
            &script,
        ])
        .env("IW_TEST_EXE", &exe)
        .env("IW_TEST_NAME", test)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| {
            panic!(
                "cannot run `setpriv --pdeathsig KILL unshare -rn` ({e}): this test needs \
                 util-linux's setpriv and unshare, an unprivileged user namespace and iproute2; \
                 it fails rather than skips where they are unavailable"
            )
        });
    let pid = child.id();
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(child.wait_with_output());
    });
    let Ok(output) = finished.recv_timeout(ON_ADDRESSES_DEADLINE) else {
        let _ = std::process::Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status();
        panic!("the test on {ips:?} ran past {ON_ADDRESSES_DEADLINE:?}: killed");
    };
    let output = output.expect("the inner copy's output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "the test on {ips:?} did not pass ({}). If the namespace could not be made, this host \
         cannot ask the question.\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        output.status
    );
    false
}
