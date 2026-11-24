// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

//! The upgrade matrix's "previous build" axis (testing.md §Compatibility
//! fixtures, A 2026-10-08): each production build in
//! `tools/ci/previous-builds.txt`, built from its own commit as a second
//! binary, against HEAD's own binaries.
//!
//! From the Stage 22 release every build in the list is across a major
//! break: it speaks IPC 2.0 and `/interweave/direct/2.0.0` alone, HEAD
//! speaks IPC 3.0 and `/interweave/direct/3.0.0` (ADR-0067 decision 7).
//! Each row therefore holds a REFUSAL, and sits beside a control that
//! shows the harness serves where the break is not in the way, so a broken
//! harness cannot pass as a refusal.
//!
//! - **The entry set.** The built entries under `INTERWEAVE_PREVIOUS_BUILDS`
//!   are exactly the list's labels, each with both binaries. An entry added
//!   without its build fails here, and so does a stale build.
//! - **HEAD's client against the previous daemon.** HEAD's IPC client, a
//!   data session and an admin port alike, is answered
//!   `close{VersionIncompatible}` (LOCAL-IPC.md §Version negotiation and
//!   phases): nothing is served and no lease is taken. Controls: the same
//!   client is served by HEAD's own daemon, and the previous
//!   `transportctl` is served by the previous daemon.
//! - **The previous client against HEAD's daemon.** The old
//!   `transportctl`, that build's only IPC client, is refused by HEAD's
//!   `transport-daemon` for `status` and for `endpoints list` alike: exit
//!   1, `refused: VersionIncompatible` on stderr, nothing on stdout.
//!   Controls: HEAD's client is served by that same daemon, and the old
//!   `transportctl` is served by the old daemon.
//! - **Direct across the break.** HEAD's daemon sending direct to the
//!   previous daemon, which offers `/interweave/direct/2.0.0` alone, fails
//!   `ProtocolUnsupported` (DIRECT.md §Protocol family; the contract's
//!   `UnsupportedProtocols` at negotiation) and nothing arrives. Control:
//!   two HEAD daemons in the same wiring exchange a direct message. The
//!   direction in which the old daemon dials HEAD is not driven here (the
//!   old build's data sessions are IPC 2.0, which HEAD's client cannot
//!   open): `upgrade_matrix_direct.rs` holds it with a raw 2.0.0 peer.
//! - **Broadcast is not a row.** The contract names none across a major
//!   break of the direct protocol or of IPC (testing.md: the envelope's
//!   version byte is a major, no minor exists), so nothing is asserted.
//!
//! Every IPC await and every child process is bounded by `PATIENCE`. A
//! daemon that accepts a connection and never answers fails the row with
//! that daemon's log; it does not run out the CI job's clock.
//!
//! These rows once held the opposite: HEAD's client served by the previous
//! daemon, the old `transportctl` served by HEAD's, direct and broadcast
//! exchanged both ways. That was true only while HEAD still spoke IPC 2.0
//! and direct 2.0.0.
//!
//! Every row is `#[ignore]`: a plain `cargo test` has no second binary.
//! CI builds the list and HEAD's `transport-daemon`, then runs this file
//! with `--ignored`. Run that way, `builds()` panics when the variable is
//! unset, and the entry-set row fails when an entry is missing or extra
//! (devex-tooling's interface, 01a11c9e).

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::future::Future;
use std::os::unix::fs::DirBuilderExt as _;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use interweave_ipc_client::{IpcBinding, SocketPaths as ClientSockets};
use interweave_local_client_api::{
    AdminBinding as _, AdminCapability, AdminPort as _, DataCapability, DataSessionBinding as _,
    DataSessionPort, SessionEvent, SessionRequest,
};
use interweave_local_client_conformance_tests::{PATIENCE, receive};
use interweave_profile_config::endpoint_key::{self, Custody};
use interweave_profile_config::{ProfilePaths, XdgRoots};
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::endpoint_keys;
use interweave_test_support::net::free_port;
use interweave_transport_api::{
    DirectDestination, MessageId, Payload, TransportError, TransportIdentity,
};

const VARIABLE: &str = "INTERWEAVE_PREVIOUS_BUILDS";

/// What a daemon of either build prints when its listen port is taken,
/// measured on HEAD and on 45ba3928 alike: `transport-daemon: the runtime:
/// substrate: transport: `. Every refusal starts `transport-daemon:`, so
/// only this longer text tells a taken port from any other early exit.
const PORT_TAKEN: &str = "the runtime: substrate: transport:";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("tests/<pkg> is two levels below the root")
        .to_path_buf()
}

/// `tools/ci/previous-builds.txt`: `<label> <sha>` per line. Anything from
/// a `#` to the line's end is a comment, as `build_previous_builds.sh`
/// reads it, and blank lines are skipped.
fn listed() -> Vec<(String, String)> {
    let path = root().join("tools/ci/previous-builds.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let entries: Vec<(String, String)> = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or_default().trim())
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut words = l.split_whitespace();
            let label = words.next().expect("a label").to_owned();
            let sha = words
                .next()
                .unwrap_or_else(|| panic!("{label}: no sha"))
                .to_owned();
            assert!(words.next().is_none(), "{l}: more than a label and a sha");
            (label, sha)
        })
        .collect();
    assert!(!entries.is_empty(), "{} lists no build", path.display());
    entries
}

/// The directory CI built into; panics when the variable is unset.
fn builds() -> PathBuf {
    PathBuf::from(
        std::env::var_os(VARIABLE)
            .unwrap_or_else(|| panic!("{VARIABLE} is unset: these rows need the previous builds")),
    )
}

/// HEAD's `transport-daemon`, beside this test's own build:
/// `target/<profile>/deps/<test>` -> `target/<profile>/transport-daemon`.
fn head_daemon() -> PathBuf {
    let exe = std::env::current_exe().expect("this test's path");
    let bin = exe
        .parent()
        .and_then(Path::parent)
        .expect("a target directory")
        .join("transport-daemon");
    assert!(
        bin.exists(),
        "{} is missing: build it first (cargo build -p interweave-transport-daemon)",
        bin.display()
    );
    bin
}

/// One listed build's two binaries.
struct Entry {
    label: String,
    daemon: PathBuf,
    transportctl: PathBuf,
}

fn entries() -> Vec<Entry> {
    let dir = builds();
    listed()
        .into_iter()
        .map(|(label, _)| Entry {
            daemon: dir.join(&label).join("transport-daemon"),
            transportctl: dir.join(&label).join("transportctl"),
            label,
        })
        .collect()
}

#[test]
#[ignore = "needs INTERWEAVE_PREVIOUS_BUILDS"]
fn the_built_entries_are_exactly_the_list() {
    let dir = builds();
    let mut built: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    built.sort();
    let mut want: Vec<String> = listed().into_iter().map(|(l, _)| l).collect();
    want.sort();
    assert_eq!(
        built, want,
        "the built entries and tools/ci/previous-builds.txt differ"
    );
    for entry in entries() {
        for bin in [&entry.daemon, &entry.transportctl] {
            use std::os::unix::fs::PermissionsExt as _;
            let meta = std::fs::metadata(bin)
                .unwrap_or_else(|e| panic!("{}: {}: {e}", entry.label, bin.display()));
            assert!(
                meta.permissions().mode() & 0o111 != 0,
                "{}: not executable",
                bin.display()
            );
        }
    }
}

fn private_dir(path: &Path) {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .expect("a private directory");
}

/// One XDG tree for one profile. Each build resolves its paths in it by
/// its own rules; HEAD's are what this side writes and connects to.
struct Home {
    _root: tempfile::TempDir,
    roots: XdgRoots,
    profile: String,
}

impl Home {
    fn new(profile: &str) -> Self {
        let root = tempfile::tempdir().expect("tempdir");
        let at = |n: &str| root.path().join(n);
        let roots = XdgRoots {
            config_home: at("config"),
            data_home: at("data"),
            state_home: at("state"),
            cache_home: at("cache"),
            runtime_dir: Some(at("run")),
        };
        private_dir(&at("run"));
        Self {
            _root: root,
            roots,
            profile: profile.to_owned(),
        }
    }

    fn paths(&self) -> ProfilePaths {
        ProfilePaths::resolve(&self.profile, &self.roots).expect("paths")
    }

    fn command(&self, bin: &Path) -> Command {
        let mut c = Command::new(bin);
        c.args(["--profile", &self.profile])
            .env_clear()
            .env("XDG_CONFIG_HOME", &self.roots.config_home)
            .env("XDG_DATA_HOME", &self.roots.data_home)
            .env("XDG_STATE_HOME", &self.roots.state_home)
            .env("XDG_CACHE_HOME", &self.roots.cache_home)
            .env(
                "XDG_RUNTIME_DIR",
                self.roots.runtime_dir.as_ref().expect("run"),
            );
        c
    }

    fn write_config(&self, yaml: &str) {
        let file = self.paths().config_file();
        private_dir(file.parent().expect("a config directory"));
        std::fs::write(file, yaml).expect("the profile written");
    }

    fn write_key(&self, identity: &ProfileIdentity) {
        let file = self.paths().identity_file();
        private_dir(file.parent().expect("an identity directory"));
        identity.save(&file).expect("the key saved");
    }

    /// HEAD's `human` endpoint key (TEST-ONLY, `endpoint_keys`) where an
    /// entry naming `<id>.key` has it: `<state_dir>/endpoint-keys/`. The
    /// previous build has no such directory and is never given one.
    fn write_endpoint_key(&self) {
        let (key, id) = endpoint_keys::endpoint_keys("human");
        let header = endpoint_key::header_bytes(
            Custody::FilesystemOnly,
            id.as_str().as_bytes(),
            &key.verifying_key().to_bytes(),
        )
        .expect("a header");
        let line = endpoint_key::encode_record(&header, &key.to_bytes());
        let file = endpoint_key::key_path(&self.paths(), &id);
        private_dir(file.parent().expect("a key directory"));
        std::fs::write(&file, format!("{}\n", line.as_str())).expect("the key written");
        std::fs::set_permissions(
            &file,
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o600),
        )
        .expect("chmod");
    }

    fn client(&self) -> IpcBinding {
        let p = self.paths();
        IpcBinding::new(
            ClientSockets {
                data: p.data_socket().expect("data"),
                admin: p.admin_socket().expect("admin"),
            },
            "matrix-admin",
        )
    }
}

/// How a started daemon is known to serve.
#[derive(Clone, Copy)]
enum Ready<'a> {
    /// HEAD's IPC client reads an admin status: HEAD's own daemon.
    Head,
    /// This `transportctl` reads `status`: a previous daemon, which HEAD's
    /// client cannot talk to, answered by its own build's client.
    Previous(&'a Path),
}

/// A daemon process of either build; killed on drop, its log kept for
/// every failure message.
struct Daemon {
    child: Child,
    log: PathBuf,
    which: String,
}

impl Daemon {
    async fn start(home: &Home, bin: &Path, which: &str, extra: &[&str], ready: Ready<'_>) -> Self {
        Self::try_start(home, bin, which, extra, ready)
            .await
            .unwrap_or_else(|log| panic!("{which} exited before serving:\n{log}"))
    }

    /// As [`Self::start`], but a daemon that exits before serving is an
    /// `Err` carrying its log, for a caller that can retry the cause.
    async fn try_start(
        home: &Home,
        bin: &Path,
        which: &str,
        extra: &[&str],
        ready: Ready<'_>,
    ) -> Result<Self, String> {
        let log = home.roots.state_home.with_file_name(format!("{which}.log"));
        let child = home
            .command(bin)
            .args(extra)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(&log).expect("a log"))
            .spawn()
            .unwrap_or_else(|e| panic!("{which}: {}: {e}", bin.display()));
        let mut daemon = Self {
            child,
            log,
            which: which.to_owned(),
        };
        // Serving once its admin socket answers a status: a socket file is
        // not enough, a killed daemon leaves one behind. Each attempt is
        // bounded, so a daemon that accepts and never answers fails here.
        let deadline = tokio::time::Instant::now() + PATIENCE;
        loop {
            let served = match ready {
                Ready::Head => {
                    let attempt = tokio::time::timeout(Duration::from_secs(2), async {
                        let admin = home
                            .client()
                            .admin([AdminCapability::Status].into())
                            .await
                            .ok()?;
                        admin.status().await.ok()
                    });
                    matches!(attempt.await, Ok(Some(_)))
                }
                Ready::Previous(bin) => ctl(home, bin, &["status"], &daemon).status.success(),
            };
            if served {
                return Ok(daemon);
            }
            if let Ok(Some(status)) = daemon.child.try_wait() {
                return Err(format!("({status}) {}", daemon.log()));
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "{which} never served:\n{}",
                daemon.log()
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    fn log(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }

    /// `fut`, or a panic naming `what` and carrying this daemon's log.
    /// Boxed: the IPC client's futures are large, and a caller holding
    /// several on its stack would be larger still.
    fn within<'a, T: 'a>(
        &'a self,
        what: &'a str,
        fut: impl Future<Output = T> + 'a,
    ) -> Pin<Box<dyn Future<Output = T> + 'a>> {
        Box::pin(async move {
            tokio::time::timeout(PATIENCE, fut)
                .await
                .unwrap_or_else(|_| {
                    panic!(
                        "{}: {what} took more than {PATIENCE:?}:\n{}",
                        self.which,
                        self.log()
                    )
                })
        })
    }

    /// The daemon's own `PeerId`, from HEAD's client: HEAD's daemons only.
    fn peer<'a>(&'a self, home: &'a Home) -> Pin<Box<dyn Future<Output = TransportIdentity> + 'a>> {
        self.within("the admin status", async move {
            home.client()
                .admin([AdminCapability::Status].into())
                .await
                .expect("an admin port")
                .status()
                .await
                .expect("a status")
                .peer
        })
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Run `command` to completion within `PATIENCE`, killing it if it hangs.
fn run_bounded(mut command: Command, what: &str, context: &dyn Fn() -> String) -> Output {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("{what}: {e}"));
    let deadline = std::time::Instant::now() + PATIENCE;
    loop {
        if child.try_wait().expect("a status").is_some() {
            return child.wait_with_output().expect("its output");
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let out = child.wait_with_output().expect("its output");
            panic!(
                "{what} took more than {PATIENCE:?}:\n{}\n--- the daemon it talked to ---\n{}",
                String::from_utf8_lossy(&out.stderr),
                context()
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// `bin --profile <home's> <args>` (a `transportctl`) against `daemon`,
/// bounded.
fn ctl(home: &Home, bin: &Path, args: &[&str], daemon: &Daemon) -> Output {
    let mut command = home.command(bin);
    command.args(args);
    run_bounded(
        command,
        &format!("{}: transportctl {args:?}", bin.display()),
        &|| daemon.log(),
    )
}

/// The control of a refusal row: `bin`'s `transportctl` is SERVED by
/// `daemon` of its own build, in `home` -- `status` names a peer,
/// `endpoints list` names the `human` entry.
fn assert_ctl_served(home: &Home, bin: &Path, daemon: &Daemon, who: &str) {
    for (args, names) in [
        (&["status"][..], "12D3KooW"),
        (&["endpoints", "list"][..], "human"),
    ] {
        let out = ctl(home, bin, args, daemon);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{who}: control transportctl {args:?}: {}\n{stdout}\n{}\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr),
            daemon.log()
        );
        assert!(
            stdout.contains(names),
            "{who}: control transportctl {args:?} does not name {names}: {stdout}"
        );
    }
}

/// A peer id as the previous build's `transportctl status` prints it.
fn peer_of_status(out: &Output) -> String {
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let at = text.find("12D3KooW").expect("a peer id in the status");
    text[at..]
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect()
}

/// A profile the daemon of `head`'s build reads. HEAD's names its endpoint
/// by the key file of its derived id and an alias (ADR-0067); the previous
/// build's keeps its format, a label `id`, which its `deny_unknown_fields`
/// holds it to and HEAD refuses (`EndpointIdNotDerived`). `profile.name`
/// must equal `--profile`.
fn profile_yaml(
    head: bool,
    name: &str,
    trusted: &[&str],
    listen: &str,
    statics: &[String],
) -> String {
    let providers = if statics.is_empty() {
        "[]".to_owned()
    } else {
        format!(
            "\n    - type: static-bootstrap\n      enabled: true\n      priority: 5\n      config:\n        peers: [{}]",
            statics
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let entry = if head {
        format!(
            "key: {}.key\n      alias: human",
            endpoint_keys::endpoint_id("human").as_str()
        )
    } else {
        "id: human".to_owned()
    };
    let allowed = trusted
        .iter()
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "schema_version: 2
profile: {{ name: {name} }}
transport:
  listen: {{ addresses: [\"{listen}\"] }}
trust:
  policy: static-allowlist
  allowed_peers: [{allowed}]
endpoints:
  default_direct_endpoint: human
  entries:
    - {entry}
      enabled: true
      advertise: false
channels:
  desired: []
discovery:
  providers: {providers}
"
    )
}

/// A HEAD profile in a fresh home, its identity and endpoint key written.
fn head_home(
    name: &str,
    identity: &ProfileIdentity,
    trusted: &[&str],
    listen: &str,
    statics: &[String],
) -> Home {
    let home = Home::new(name);
    home.write_key(identity);
    home.write_endpoint_key();
    home.write_config(&profile_yaml(true, name, trusted, listen, statics));
    home
}

/// A previous-build profile in a fresh home: label ids, no key written
/// (`--create-identity` makes the node's).
fn old_home(trusted: &[&str], listen: &str, statics: &[String]) -> Home {
    let home = Home::new("old");
    home.write_config(&profile_yaml(false, "old", trusted, listen, statics));
    home
}

fn human_session() -> SessionRequest {
    SessionRequest::new(
        "human-client",
        Some(endpoint_keys::endpoint_id("human")),
        [DataCapability::Commands, DataCapability::Events],
    )
    .expect("in bounds")
}

fn stranger() -> String {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer")
        .as_str()
        .to_owned()
}

/// Row: HEAD's IPC client is refused by each previous daemon. The previous
/// daemon answers HEAD's major-3 hello `close{VersionIncompatible}`, on the
/// data socket and the admin socket alike, so no session opens, no lease
/// is taken and no admin port is minted. Controls in the same row: HEAD's
/// client is served by HEAD's own daemon on a HEAD profile, and the
/// previous `transportctl` is served by the previous daemon (so it is up
/// and answering, before and after the refusals, and the refusal is the
/// break).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs INTERWEAVE_PREVIOUS_BUILDS"]
async fn heads_client_is_refused_version_incompatible_by_each_previous_daemon() {
    let head_bin = head_daemon();
    for entry in entries() {
        let identity = ProfileIdentity::generate();
        let control_home = head_home(
            "head",
            &identity,
            &[&stranger()],
            "/ip4/127.0.0.1/tcp/0",
            &[],
        );
        let control =
            Daemon::start(&control_home, &head_bin, "HEAD's daemon", &[], Ready::Head).await;
        let session = control
            .within(
                "a leased session",
                control_home.client().open(human_session()),
            )
            .await
            .unwrap_or_else(|e| panic!("control: HEAD's client on HEAD: {e:?}\n{}", control.log()));
        control
            .within("a close", session.close())
            .await
            .expect("closes");

        let home = old_home(&[&stranger()], "/ip4/127.0.0.1/tcp/0", &[]);
        let old = Daemon::start(
            &home,
            &entry.daemon,
            &entry.label,
            &["--create-identity"],
            Ready::Previous(&entry.transportctl),
        )
        .await;
        assert_ctl_served(&home, &entry.transportctl, &old, &entry.label);

        let Err(data) = old
            .within("a leased session", home.client().open(human_session()))
            .await
        else {
            panic!(
                "{}: HEAD's data session was served by the previous daemon:\n{}",
                entry.label,
                old.log()
            );
        };
        assert_eq!(
            data,
            TransportError::VersionIncompatible,
            "{}: the data socket\n{}",
            entry.label,
            old.log()
        );
        let Err(admin) = old
            .within(
                "an admin port",
                home.client().admin([AdminCapability::Status].into()),
            )
            .await
        else {
            panic!(
                "{}: HEAD's admin port was served by the previous daemon:\n{}",
                entry.label,
                old.log()
            );
        };
        assert_eq!(
            admin,
            TransportError::VersionIncompatible,
            "{}: the admin socket\n{}",
            entry.label,
            old.log()
        );

        // The refusals left the previous daemon whole and its own build's
        // client still served: nothing half-open, nothing taken.
        assert_ctl_served(&home, &entry.transportctl, &old, &entry.label);
    }
}

/// Row: each previous `transportctl` is refused by HEAD's daemon, in one
/// XDG tree. HEAD's daemon answers its major-2 hello
/// `close{VersionIncompatible}`; the old client prints `refused:
/// VersionIncompatible` on stderr, exits 1 and writes nothing to stdout,
/// for `status` and `endpoints list` alike. Controls in the same row: HEAD's
/// client is served by that same daemon (so it is up, on a profile whose
/// endpoint is derived), and the old `transportctl` is served by the old
/// daemon.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs INTERWEAVE_PREVIOUS_BUILDS"]
async fn each_previous_transportctl_is_refused_version_incompatible_by_heads_daemon() {
    let head_bin = head_daemon();
    for entry in entries() {
        let previous = old_home(&[&stranger()], "/ip4/127.0.0.1/tcp/0", &[]);
        let old = Daemon::start(
            &previous,
            &entry.daemon,
            &entry.label,
            &["--create-identity"],
            Ready::Previous(&entry.transportctl),
        )
        .await;
        assert_ctl_served(&previous, &entry.transportctl, &old, &entry.label);

        let identity = ProfileIdentity::generate();
        let peer = identity.transport_identity().expect("peer");
        let home = head_home(
            "head",
            &identity,
            &[&stranger()],
            "/ip4/127.0.0.1/tcp/0",
            &[],
        );
        let head = Daemon::start(&home, &head_bin, "HEAD's daemon", &[], Ready::Head).await;
        assert_eq!(head.peer(&home).await, peer, "HEAD's daemon names itself");

        for args in [&["status"][..], &["endpoints", "list"][..]] {
            let out = ctl(&home, &entry.transportctl, args, &head);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert_eq!(
                out.status.code(),
                Some(1),
                "{}: transportctl {args:?} was not refused: {}\n{stdout}\n{stderr}\n{}",
                entry.label,
                out.status,
                head.log()
            );
            assert!(
                stderr.contains("refused: VersionIncompatible"),
                "{}: transportctl {args:?}: {stderr}",
                entry.label
            );
            assert!(
                stdout.is_empty(),
                "{}: transportctl {args:?} was served something: {stdout}",
                entry.label
            );
        }

        // The refusals left HEAD's daemon serving its own client.
        assert_eq!(head.peer(&home).await, peer, "HEAD's daemon still serves");
    }
}

/// HEAD daemon A, HEAD daemon B and one previous daemon. A trusts B and
/// the previous daemon and holds a static route to each; B and the
/// previous daemon trust A and route to A. A's route to the previous
/// daemon is what the refusal row sends over, B is its control.
struct Rig {
    a_home: Home,
    a: Daemon,
    b_home: Home,
    b: Daemon,
    old_peer: String,
    a_peer: TransportIdentity,
    b_peer: TransportIdentity,
    // Held for their Drop: the previous daemon and its home.
    _old: (Home, Daemon),
}

/// Bring the three up on fresh ports. `Err` carries the log of the daemon
/// that exited before serving, for a retry when its port was taken.
async fn bring_up(ip: std::net::Ipv4Addr, entry: &Entry, head_bin: &Path) -> Result<Rig, String> {
    // Ports are picked, released and then bound by the daemons: the status
    // carries no listen address, in either build, so the other side's
    // profile must name it before the daemon starts. Distinct ports:
    // `free_port` never answers one twice
    // (`net::tests::free_port_never_answers_the_same_port_twice`).
    let (a_port, b_port, old_port) = (free_port(ip), free_port(ip), free_port(ip));
    let listen = |p: u16| format!("/ip4/{ip}/tcp/{p}");
    let a_identity = ProfileIdentity::generate();
    let a_peer = a_identity.transport_identity().expect("peer");
    let b_identity = ProfileIdentity::generate();
    let b_peer = b_identity.transport_identity().expect("peer");
    let route_a = [format!("{}/p2p/{}", listen(a_port), a_peer.as_str())];

    let previous = old_home(&[a_peer.as_str()], &listen(old_port), &route_a);
    let old = Daemon::try_start(
        &previous,
        &entry.daemon,
        &entry.label,
        &["--create-identity"],
        Ready::Previous(&entry.transportctl),
    )
    .await?;
    let old_peer = peer_of_status(&ctl(&previous, &entry.transportctl, &["status"], &old));

    let b_home = head_home(
        "head-b",
        &b_identity,
        &[a_peer.as_str()],
        &listen(b_port),
        &route_a,
    );
    let b = Daemon::try_start(&b_home, head_bin, "HEAD's daemon B", &[], Ready::Head).await?;

    let a_home = head_home(
        "head-a",
        &a_identity,
        &[b_peer.as_str(), &old_peer],
        &listen(a_port),
        &[
            format!("{}/p2p/{}", listen(b_port), b_peer.as_str()),
            format!("{}/p2p/{old_peer}", listen(old_port)),
        ],
    );
    let a = Daemon::try_start(&a_home, head_bin, "HEAD's daemon A", &[], Ready::Head).await?;
    Ok(Rig {
        a_home,
        a,
        b_home,
        b,
        old_peer,
        a_peer,
        b_peer,
        _old: (previous, old),
    })
}

/// Row: direct across the break. A HEAD daemon sending direct to the
/// previous daemon, which speaks `/interweave/direct/2.0.0` alone, is
/// refused at negotiation: the send fails `ProtocolUnsupported` and is
/// never accepted. Control in the same row, over the same wiring and the
/// same kind of profile: HEAD's two daemons exchange a direct message both
/// ways, so a harness that could not connect or deliver would fail there
/// rather than pass as a refusal.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs INTERWEAVE_PREVIOUS_BUILDS"]
async fn heads_direct_is_refused_by_each_previous_daemon_beside_head_to_head_exchange() {
    let head_bin = head_daemon();
    let ip = interweave_test_support::net::require_private_interface_v4();
    for entry in entries() {
        // A port taken in between picking and binding makes a daemon exit
        // before serving with a transport error ([`PORT_TAKEN`]); retried
        // on fresh ports. Any other early exit, or a third taken port in a
        // row, fails the row.
        let mut attempt = 0;
        let rig = loop {
            attempt += 1;
            match bring_up(ip, &entry, &head_bin).await {
                Ok(rig) => break rig,
                Err(log) if attempt < 3 && log.contains(PORT_TAKEN) => {}
                Err(log) => panic!("{}: a daemon exited before serving:\n{log}", entry.label),
            }
        };
        let Rig {
            a_home,
            a,
            b_home,
            b,
            old_peer,
            a_peer,
            b_peer,
            ..
        } = &rig;
        let human = endpoint_keys::endpoint_id("human");
        let payload = |s: &str| Payload::at_ceiling(None, s.as_bytes().to_vec()).expect("fits");
        let at_a = a
            .within("a leased session", a_home.client().open(human_session()))
            .await
            .expect("leases");
        let at_b = b
            .within("a leased session", b_home.client().open(human_session()))
            .await
            .expect("leases");
        let logs = || {
            format!(
                "{}\n--- A ---\n{}\n--- B ---\n{}",
                entry.label,
                a.log(),
                b.log()
            )
        };

        // Control: A to B and B to A, A's send retried until the two are
        // connected.
        let deadline = tokio::time::Instant::now() + PATIENCE;
        loop {
            let sent = a
                .within(
                    "A's direct send",
                    at_a.send_direct(
                        DirectDestination {
                            peer: b_peer.clone(),
                            endpoint: Some(human.clone()),
                        },
                        MessageId::from_bytes([1; 16]),
                        payload("a to b"),
                    ),
                )
                .await;
            match sent {
                Ok(endpoint) => {
                    assert_eq!(endpoint, human, "{}", entry.label);
                    break;
                }
                Err(e) => assert!(
                    tokio::time::Instant::now() < deadline,
                    "control: A's send to HEAD never accepted: {e:?}\n{}",
                    logs()
                ),
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        let got = b.within("a receive", receive(&at_b, PATIENCE)).await;
        assert!(
            got.iter()
                .any(|e| matches!(e, SessionEvent::Direct(m) if m.payload.bytes() == b"a to b")),
            "control: B's session got: {got:?}\n{}",
            logs()
        );
        b.within(
            "B's direct send",
            at_b.send_direct(
                DirectDestination {
                    peer: a_peer.clone(),
                    endpoint: Some(human.clone()),
                },
                MessageId::from_bytes([2; 16]),
                payload("b to a"),
            ),
        )
        .await
        .unwrap_or_else(|e| panic!("control: B's send to A: {e:?}\n{}", logs()));
        let got = a.within("a receive", receive(&at_a, PATIENCE)).await;
        assert!(
            got.iter()
                .any(|e| matches!(e, SessionEvent::Direct(m) if m.payload.bytes() == b"b to a")),
            "control: A's session got: {got:?}\n{}",
            logs()
        );

        // The refusal: the previous daemon's peer, on the same wiring. The
        // connection forms on its own schedule, so an error that is not the
        // contract's is retried until the deadline; an accepted send never.
        let old_peer = TransportIdentity::parse(old_peer).expect("the previous peer id");
        let deadline = tokio::time::Instant::now() + PATIENCE;
        let mut last;
        loop {
            let sent = a
                .within(
                    "A's direct send to the previous daemon",
                    at_a.send_direct(
                        DirectDestination {
                            peer: old_peer.clone(),
                            endpoint: Some(human.clone()),
                        },
                        MessageId::from_bytes([3; 16]),
                        payload("a to old"),
                    ),
                )
                .await;
            match sent {
                Ok(endpoint) => panic!(
                    "{}: a direct send to a 2.0.0-only daemon was accepted ({endpoint:?})\n{}",
                    entry.label,
                    logs()
                ),
                Err(TransportError::ProtocolUnsupported) => break,
                Err(e) => last = e,
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "{}: never refused ProtocolUnsupported, last {last:?}\n{}",
                entry.label,
                logs()
            );
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        a.within("a close", at_a.close()).await.expect("closes");
        b.within("a close", at_b.close()).await.expect("closes");
    }
}
