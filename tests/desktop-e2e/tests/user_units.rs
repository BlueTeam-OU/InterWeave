// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Linux package under a systemd USER manager (plan §22, Validate:
//! service/autostart lifecycle, uninstall semantics): `install.sh` and
//! `uninstall.sh` as shipped, and the two unit templates they install,
//! run by the account's own manager.
//!
//! Every case is `#[ignore]`: a plain `cargo test` may have no user
//! manager, and CI has none yet (the row's `limit` until its job exists).
//! Run them with `--ignored --test-threads 1` as an account whose manager
//! is up (`systemctl --user show-environment` answers) and that runs no
//! `interweave-*@` instance of its own -- uninstall stops every one, so a
//! case refuses to start beside one.
//!
//! The install goes into a scratch `HOME` and `XDG_CONFIG_HOME`, which the
//! account's manager does not search, so each case LINKS the installed
//! template into the manager's runtime unit directory (`link --runtime`,
//! gone at logout) and gives its instance a runtime drop-in naming the
//! installed binary and the scratch XDG tree, since the template's `%h` is
//! the real home. Everything else -- `Restart=`, `RestartSec=`, the exit
//! statuses, `BindsTo=`, `WantedBy=` -- is the shipped template's.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use interweave_local_client_api::{
    AdminBinding as _, AdminCapability, AdminPort as _, DataSessionBinding as _,
    DataSessionPort as _,
};
use interweave_transport_api::TransportIdentity;

mod common;

use common::{Home, PATIENCE, lease_request, stranger, workspace_binary};

/// One case at a time: the template's runtime link is one name for the
/// whole account, and uninstall stops every instance.
static MANAGER: Mutex<()> = Mutex::new(());

fn profile(name: &str, trusted: &TransportIdentity) -> String {
    format!(
        "schema_version: 2
profile: {{ name: {name} }}
trust: {{ policy: static-allowlist, allowed_peers: [\"{}\"] }}
endpoints:
  default_direct_endpoint: human
  entries:
    - {{ id: human, enabled: true, advertise: false }}
channels: {{ desired: [] }}
discovery: {{ providers: [] }}
transport: {{ listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/0\"] }} }}
",
        trusted.as_str()
    )
}

fn packaging() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/linux")
}

/// The directory holding the workspace's three executables, each checked.
fn bin_dir() -> PathBuf {
    let daemon = workspace_binary("transport-daemon", "interweave-transport-daemon");
    workspace_binary("transportctl", "interweave-transportctl");
    workspace_binary("human-desktop", "interweave-human-desktop");
    daemon.parent().expect("a target directory").to_path_buf()
}

fn systemctl(args: &[&str]) -> Output {
    Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .expect("systemctl runs")
}

fn systemctl_ok(args: &[&str]) {
    let out = systemctl(args);
    assert!(
        out.status.success(),
        "systemctl --user {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// One property of `unit`, as `systemctl show` prints it.
fn show(unit: &str, property: &str) -> String {
    let out = systemctl(&["show", "--value", "-p", property, unit]);
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// The manager's runtime unit directory: where `link --runtime` and
/// `enable --runtime` write, and where the drop-ins go.
fn runtime_units() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").expect("the session's XDG_RUNTIME_DIR");
    Path::new(&dir).join("systemd/user")
}

/// A key's value in the shipped daemon template, the one place it is set.
fn template_seconds(key: &str) -> Duration {
    let text = std::fs::read_to_string(packaging().join("systemd/interweave-daemon@.service"))
        .expect("the daemon template");
    let value = text
        .lines()
        .find_map(|line| line.strip_prefix(key))
        .unwrap_or_else(|| panic!("{key} in the template"));
    Duration::from_secs(value.trim().parse().expect("whole seconds"))
}

/// One case's world: a scratch home, the package installed into it, and
/// the account's manager told where to find it. Dropped, it stops its
/// instances and removes every runtime file it made, whatever failed.
struct Installed {
    home: Home,
    /// The scratch `HOME` install.sh writes `.local/bin` under.
    user_home: PathBuf,
    daemon: String,
    client: String,
    _manager: std::sync::MutexGuard<'static, ()>,
}

impl Installed {
    fn new(peer_trusted: &TransportIdentity) -> Self {
        let manager = MANAGER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(
            systemctl(&["show-environment"]).status.success(),
            "no systemd user manager answers: run these cases as an account whose \
             manager is up"
        );
        let foreign = String::from_utf8_lossy(
            &systemctl(&[
                "list-units",
                "--all",
                "--plain",
                "--no-legend",
                "interweave-*@*.service",
            ])
            .stdout,
        )
        .into_owned();
        assert!(
            foreign.trim().is_empty(),
            "this account already runs InterWeave units, which uninstall.sh would stop: \
             {foreign}"
        );
        let name = format!("e2e-units-{}", std::process::id());
        let home = Home::new(&name);
        home.write_config(&profile(&name, peer_trusted));
        let user_home = home.root.path().join("home");
        common::private_dir(&user_home);
        let this = Self {
            home,
            user_home,
            daemon: format!("interweave-daemon@{name}.service"),
            client: format!("interweave-client@{name}.service"),
            _manager: manager,
        };
        this.install();
        this
    }

    fn installed_units(&self) -> PathBuf {
        self.home.roots.config_home.join("systemd/user")
    }

    fn installed_bin(&self) -> PathBuf {
        self.user_home.join(".local/bin")
    }

    /// Run a shipped script in the scratch home, the session's manager
    /// still reachable through the real `XDG_RUNTIME_DIR`.
    fn script(&self, script: &str, args: &[&Path]) -> Output {
        let out = Command::new(packaging().join(script))
            .args(args)
            .env("HOME", &self.user_home)
            .env("XDG_CONFIG_HOME", &self.home.roots.config_home)
            .output()
            .expect("the script runs");
        assert!(
            out.status.success(),
            "{script}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }

    /// install.sh, then the runtime link and drop-ins that stand in for
    /// the manager searching the scratch config directory.
    fn install(&self) {
        let bins = bin_dir();
        self.script("install.sh", &[&bins]);
        let units = runtime_units();
        for template in ["interweave-daemon@.service", "interweave-client@.service"] {
            let installed = self.installed_units().join(template);
            // A previous install's link, left by uninstall.sh, which knows
            // nothing of it.
            let _ = std::fs::remove_file(units.join(template));
            systemctl_ok(&["link", "--runtime", installed.to_str().expect("utf-8")]);
        }
        let env = |p: &Path| p.to_str().expect("utf-8").to_owned();
        let roots = &self.home.roots;
        let daemon = format!(
            "[Service]\nExecStart=\nExecStart={} --profile %i\n\
             Environment=XDG_CONFIG_HOME={} XDG_DATA_HOME={} XDG_STATE_HOME={} \
             XDG_CACHE_HOME={} XDG_RUNTIME_DIR={}\n",
            env(&self.installed_bin().join("transport-daemon")),
            env(&roots.config_home),
            env(&roots.data_home),
            env(&roots.state_home),
            env(&roots.cache_home),
            env(roots.runtime_dir.as_deref().expect("a runtime dir")),
        );
        // InterWeave ships no automated client; a sleeper is the
        // deployment's program here, which is all BindsTo= needs.
        let client = "[Service]\nExecStart=\nExecStart=/usr/bin/sleep infinity\n";
        for (unit, conf) in [(&self.daemon, daemon.as_str()), (&self.client, client)] {
            let dir = units.join(format!("{unit}.d"));
            std::fs::create_dir_all(&dir).expect("a drop-in directory");
            std::fs::write(dir.join("e2e.conf"), conf).expect("a drop-in");
        }
        systemctl_ok(&["daemon-reload"]);
    }

    /// Until the daemon's two sockets accept and its admin port answers
    /// with a status: the `PeerId` it serves.
    async fn serving(&self) -> TransportIdentity {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if let Ok(port) = self
                .home
                .binding()
                .admin([AdminCapability::Status].into())
                .await
                && let Ok(status) = port.status().await
            {
                return status.peer;
            }
            assert!(
                Instant::now() < deadline,
                "{} not serving: {}",
                self.daemon,
                self.journal()
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn lease_epoch(&self) -> String {
        let session = self
            .home
            .binding()
            .open(lease_request())
            .await
            .expect("leases");
        let epoch = session
            .session()
            .endpoint_lease()
            .expect("a lease")
            .epoch
            .clone();
        format!("{epoch:?}")
    }

    /// The manager's own lines about the daemon instance, oldest first.
    fn manager_lines(&self) -> Vec<String> {
        let out = Command::new("journalctl")
            .args(["--user", "--no-pager", "-o", "cat", "-u", &self.daemon])
            .output()
            .expect("journalctl runs");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|line| line.starts_with(&format!("{}:", self.daemon)))
            .map(str::to_owned)
            .collect()
    }

    /// Once the manager has logged its `n`th scheduled restart: how many
    /// runs it has logged as failed. `Result=` cannot say it -- systemd
    /// resets it when the restarted run starts -- so the journal, where
    /// each stopped run leaves its own line, is the oracle.
    async fn failures_by_restart(&self, n: u32) -> usize {
        let scheduled = format!("restart counter is at {n}.");
        let deadline = Instant::now() + PATIENCE;
        loop {
            let lines = self.manager_lines();
            if lines.iter().any(|line| line.ends_with(&scheduled)) {
                return lines
                    .iter()
                    .filter(|line| line.contains("Failed with result"))
                    .count();
            }
            assert!(
                Instant::now() < deadline,
                "no restart {n} in the journal: {lines:#?}"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn journal(&self) -> String {
        let out = Command::new("journalctl")
            .args(["--user", "--no-pager", "-n", "40", "-u", &self.daemon])
            .output();
        out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default()
    }

    /// Until `unit`'s main process is a new one, active: its PID.
    async fn restarted(&self, unit: &str, old_pid: &str) -> String {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let pid = show(unit, "MainPID");
            if pid != "0" && pid != old_pid && show(unit, "ActiveState") == "active" {
                return pid;
            }
            assert!(
                Instant::now() < deadline,
                "{unit} not restarted: {}",
                self.journal()
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn sockets_gone(&self) -> bool {
        !self.home.data_socket().exists() && !self.home.admin_socket().exists()
    }
}

impl Drop for Installed {
    fn drop(&mut self) {
        for unit in [&self.client, &self.daemon] {
            let _ = systemctl(&["disable", "--runtime", unit]);
            let _ = systemctl(&["stop", unit]);
        }
        let units = runtime_units();
        for unit in [&self.client, &self.daemon] {
            let _ = std::fs::remove_dir_all(units.join(format!("{unit}.d")));
        }
        for template in ["interweave-daemon@.service", "interweave-client@.service"] {
            let _ = std::fs::remove_file(units.join(template));
        }
        let _ = systemctl(&["daemon-reload"]);
        let _ = systemctl(&["reset-failed", "interweave-*@*.service"]);
    }
}

/// The service lifecycle under the account's manager, from an install
/// that enables nothing: started, the daemon serves; SIGHUP stops it
/// cleanly and the unit starts it again no sooner than `RestartSec=`,
/// the stop counted a success, with a fresh epoch and the same `PeerId`
/// -- and the control, SIGKILL, restarts it too but is a failure; a stop
/// is not restarted and leaves no socket; enabling it is what makes it
/// start with the manager, and makes the enabled client start with it
/// and stop with it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a systemd user manager"]
async fn the_installed_daemon_unit_starts_restarts_on_sighup_and_stops() {
    let installed = Installed::new(&stranger());
    let key = installed.home.write_key();
    let daemon = installed.daemon.as_str();

    // Installed, enabled nowhere: autostart is the person's act.
    for exe in ["transport-daemon", "transportctl", "human-desktop"] {
        assert!(installed.installed_bin().join(exe).is_file(), "{exe}");
    }
    assert_eq!(
        show(daemon, "UnitFileState"),
        "disabled",
        "install enables nothing"
    );
    assert_eq!(show(daemon, "ActiveState"), "inactive");

    systemctl_ok(&["start", daemon]);
    assert_eq!(installed.serving().await, key, "the profile's own PeerId");
    let first_epoch = installed.lease_epoch().await;
    let first_pid = show(daemon, "MainPID");

    // SIGHUP: the orderly stop, counted a success, restarted after RestartSec=.
    let hup = Instant::now();
    systemctl_ok(&["kill", "--kill-whom=main", "--signal=SIGHUP", daemon]);
    let second_pid = installed.restarted(daemon, &first_pid).await;
    let took = hup.elapsed();
    let restart_sec = template_seconds("RestartSec=");
    assert!(
        took >= restart_sec,
        "restarted after RestartSec= ({restart_sec:?}), not at once: {took:?}"
    );
    assert_eq!(show(daemon, "NRestarts"), "1");
    assert_eq!(
        installed.failures_by_restart(1).await,
        0,
        "a SIGHUP stop is logged as a success: {:#?}",
        installed.manager_lines()
    );
    assert_eq!(installed.serving().await, key, "the same PeerId");
    assert_ne!(
        installed.lease_epoch().await,
        first_epoch,
        "no lease survives SIGHUP"
    );

    // The control: SIGKILL restarts it too, and is a failure.
    systemctl_ok(&["kill", "--kill-whom=main", "--signal=SIGKILL", daemon]);
    installed.restarted(daemon, &second_pid).await;
    assert_eq!(show(daemon, "NRestarts"), "2");
    assert_eq!(
        installed.failures_by_restart(2).await,
        1,
        "the control, SIGKILL, is logged as a failure: {:#?}",
        installed.manager_lines()
    );
    installed.serving().await;

    // A stop is final: exit 0, no restart, no socket left.
    systemctl_ok(&["stop", daemon]);
    assert_eq!(show(daemon, "ActiveState"), "inactive");
    assert_eq!(show(daemon, "ExecMainStatus"), "0");
    assert!(installed.sockets_gone(), "both sockets unlinked");
    tokio::time::sleep(restart_sec + Duration::from_secs(1)).await;
    assert_eq!(
        show(daemon, "ActiveState"),
        "inactive",
        "a stop is not restarted"
    );

    // Enabled: wanted by the manager's default target, and the enabled
    // client comes and goes with its daemon.
    systemctl_ok(&["enable", "--runtime", daemon]);
    systemctl_ok(&["enable", "--runtime", &installed.client]);
    assert_eq!(show(daemon, "UnitFileState"), "enabled-runtime");
    assert!(
        runtime_units()
            .join("default.target.wants")
            .join(daemon)
            .is_symlink(),
        "the daemon starts with the manager"
    );
    systemctl_ok(&["start", daemon]);
    installed.serving().await;
    installed.restarted(&installed.client, "0").await;
    systemctl_ok(&["stop", daemon]);
    assert_eq!(
        show(&installed.client, "ActiveState"),
        "inactive",
        "the client stops with its daemon"
    );
}

/// Uninstall (plan §22, ruled 2026-10-20): every instance stopped and
/// disabled, the units and executables gone, nothing loaded, no daemon,
/// no socket -- and every profile's configuration, key and state kept, so
/// a reinstall serves the same `PeerId`. The control for "nothing
/// running" is the same instance serving, and enabled, just before.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a systemd user manager"]
async fn uninstall_stops_every_instance_removes_the_package_and_keeps_the_profile() {
    let installed = Installed::new(&stranger());
    let key = installed.home.write_key();
    let daemon = installed.daemon.clone();
    systemctl_ok(&["enable", "--runtime", &daemon]);
    systemctl_ok(&["enable", "--runtime", &installed.client]);
    systemctl_ok(&["start", &daemon]);
    assert_eq!(installed.serving().await, key, "serving before");
    installed.restarted(&installed.client, "0").await;
    let pid = show(&daemon, "MainPID");
    assert_ne!(pid, "0", "the control: a daemon runs");
    let kept: Vec<(PathBuf, Vec<u8>)> = [
        installed.home.paths.config_file(),
        installed.home.paths.identity_file(),
    ]
    .into_iter()
    .map(|path| {
        let bytes = std::fs::read(&path).expect("a profile file");
        (path, bytes)
    })
    .collect();

    installed.script("uninstall.sh", &[]);

    for unit in [&daemon, &installed.client] {
        assert_eq!(show(unit, "ActiveState"), "inactive", "{unit} stopped");
        assert_eq!(show(unit, "LoadState"), "not-found", "{unit} not loaded");
    }
    assert!(
        !Path::new("/proc").join(&pid).exists(),
        "the daemon process {pid} is gone"
    );
    assert!(installed.sockets_gone(), "no socket in the runtime dir");
    assert!(
        !runtime_units()
            .join("default.target.wants")
            .join(&daemon)
            .exists(),
        "disabled"
    );
    for exe in ["transport-daemon", "transportctl", "human-desktop"] {
        assert!(
            !installed.installed_bin().join(exe).exists(),
            "{exe} removed"
        );
    }
    for template in ["interweave-daemon@.service", "interweave-client@.service"] {
        assert!(
            !installed.installed_units().join(template).exists(),
            "{template} removed"
        );
    }
    for (path, bytes) in &kept {
        assert_eq!(
            &std::fs::read(path).expect("kept"),
            bytes,
            "{} kept as it was",
            path.display()
        );
    }

    // Reinstalled, the kept key is the same PeerId.
    installed.install();
    systemctl_ok(&["start", &daemon]);
    assert_eq!(
        installed.serving().await,
        key,
        "the same PeerId after a reinstall"
    );
}
