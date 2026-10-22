// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The build a release case runs: an installed package, or the
//! workspace's own build until one exists (plan section 22, Stage 19's
//! Validate rows).
//!
//! THE SEAM a package keeps: `INTERWEAVE_PACKAGE_BIN` names one directory
//! holding the three desktop executables under their own names --
//! `transport-daemon`, `transportctl` and `human-desktop`
//! (`human-client-packaging.md` section 2) -- as `packaging/linux`
//! installs them into `~/.local/bin`. Each is started with nothing but
//! `--profile <name>` and the XDG variables, so whatever the package adds
//! around them (a unit, a launcher) is not what these cases exercise.
//! Unset, the three are the workspace's build beside this test, as every
//! other case here runs them.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort as _};
use interweave_transport_api::TransportIdentity;

use crate::common::{Home, PATIENCE, workspace_binary};

/// The variable naming an installed package's executables.
pub(crate) const PACKAGE_BIN: &str = "INTERWEAVE_PACKAGE_BIN";

/// One of the three executables, from the package when one is named.
pub(crate) fn installed(name: &str, package: &str) -> PathBuf {
    match std::env::var_os(PACKAGE_BIN) {
        Some(dir) => {
            let bin = Path::new(&dir).join(name);
            // A package without the executable fails by name, never falls
            // back to the workspace: that would test a build the package
            // does not ship.
            assert!(
                bin.is_file(),
                "{PACKAGE_BIN} names {}, which holds no {name}",
                Path::new(&dir).display()
            );
            bin
        }
        None => workspace_binary(name, package),
    }
}

/// `transport-daemon`, as the package ships it.
pub(crate) fn daemon_binary() -> PathBuf {
    installed("transport-daemon", "interweave-transport-daemon")
}

/// `transportctl`, as the package ships it.
pub(crate) fn transportctl_binary() -> PathBuf {
    installed("transportctl", "interweave-transportctl")
}

/// `human-desktop`, as the package ships it.
pub(crate) fn client_binary() -> PathBuf {
    installed("human-desktop", "interweave-human-desktop")
}

/// `command` in `home`'s XDG tree and nothing else of this environment.
pub(crate) fn in_home(command: &mut Command, home: &Home) {
    let env = |p: &Path| p.as_os_str().to_owned();
    command
        .args(["--profile", home.paths.profile()])
        .env_clear()
        .env("XDG_CONFIG_HOME", env(&home.roots.config_home))
        .env("XDG_DATA_HOME", env(&home.roots.data_home))
        .env("XDG_STATE_HOME", env(&home.roots.state_home))
        .env("XDG_CACHE_HOME", env(&home.roots.cache_home))
        .env(
            "XDG_RUNTIME_DIR",
            env(home.roots.runtime_dir.as_deref().expect("a runtime dir")),
        );
}

/// The package's `transportctl` in `home`, `input` on stdin.
pub(crate) fn transportctl(home: &Home, args: &[&str], input: &str) -> Output {
    transportctl_of(&transportctl_binary(), home, args, input)
}

/// The `transportctl` at `bin` -- another build's -- in `home`, `input`
/// on stdin.
pub(crate) fn transportctl_of(bin: &Path, home: &Home, args: &[&str], input: &str) -> Output {
    use std::io::Write as _;
    let mut command = Command::new(bin);
    in_home(&mut command, home);
    let mut child = command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("transportctl runs");
    // A refusal before the read closes stdin unread.
    let _ = child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes());
    child.wait_with_output().expect("transportctl ends")
}

/// [`transportctl`] that must succeed; its stdout.
pub(crate) fn transportctl_ok(home: &Home, args: &[&str], input: &str) -> String {
    let out = transportctl(home, args, input);
    assert!(
        out.status.success(),
        "transportctl {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8")
}

/// A process of the package, killed and reaped when dropped.
pub(crate) struct Running {
    pub(crate) child: Child,
    log: PathBuf,
}

impl Running {
    /// What it wrote to stderr so far.
    pub(crate) fn log(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }

    /// Its exit, which must come within `PATIENCE`.
    pub(crate) fn exit(&mut self) -> ExitStatus {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if let Some(status) = self.child.try_wait().expect("a status") {
                return status;
            }
            assert!(Instant::now() < deadline, "it exits: {}", self.log());
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// SIGTERM, then its exit.
    pub(crate) fn terminate(&mut self) -> ExitStatus {
        crate::harness::signal(self.child.id(), "TERM");
        self.exit()
    }

    /// Until both of `home`'s sockets accept a connection, or the daemon
    /// exits, which fails the case with its log.
    pub(crate) async fn serving(&mut self, home: &Home) {
        let deadline = tokio::time::Instant::now() + PATIENCE;
        let accepted = |socket: PathBuf| std::os::unix::net::UnixStream::connect(socket).is_ok();
        while !(accepted(home.data_socket()) && accepted(home.admin_socket())) {
            if let Some(status) = self.child.try_wait().expect("a status") {
                panic!(
                    "the daemon exited ({status}) instead of serving:\n{}",
                    self.log()
                );
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "not serving in time:\n{}",
                self.log()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The package's daemon for `home`'s profile, its stderr to a file
/// OUTSIDE the XDG tree, so a byte-for-byte comparison of the tree sees
/// only what the daemon itself wrote there.
pub(crate) fn start_daemon(home: &Home) -> Running {
    start_daemon_of(&daemon_binary(), home)
}

/// [`start_daemon`] with the `transport-daemon` at `bin`, another
/// build's.
pub(crate) fn start_daemon_of(bin: &Path, home: &Home) -> Running {
    // One log per start, as the client's in harness.rs: a clock-named
    // file could be reused within a tick and truncated under a daemon
    // still writing it.
    static STARTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = STARTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let log = home.root.path().join(format!("daemon-of-{n}.log"));
    let stderr = std::fs::File::create_new(&log).expect("a log file of its own");
    let mut command = Command::new(bin);
    in_home(&mut command, home);
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr)
        .spawn()
        .expect("the daemon starts");
    Running { child, log }
}

/// The profile's identity, as its daemon's status reads it.
pub(crate) async fn status_peer(home: &Home) -> TransportIdentity {
    let binding = home.binding();
    let port = binding
        .admin([AdminCapability::Status].into())
        .await
        .expect("an admin port");
    port.status().await.expect("status").peer
}
