// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! S9: the shipped daemon's two sockets attacked from the same host
//! (plan section 21; threat-model.md's "Local IPC attack" and "Admin
//! confused deputy" rows). The in-process server's refusals are
//! `tests/ipc-v2`'s; what is new here is the daemon binary itself -- its
//! composition root, its sockets as it binds them, under the process
//! state it inherits -- and a real second uid.
//!
//! What this does NOT prove is decided (3)'s residual: a process of the
//! SAME uid opens the admin socket as `transportctl` does, and nothing
//! here or anywhere refuses it. `client.kind` is not authentication.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use interweave_ipc_protocol::{DecodedFrame, Frame, FrameError, decode_frame, encode_frame};
use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort as _};
use interweave_test_support::e2e::{Home, PATIENCE, stranger};
use interweave_transport_api::{TransportError, TransportIdentity};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::UnixStream;

/// A profile of one endpoint, `human`, trusting `trusted`, on loopback.
fn profile(trusted: &TransportIdentity) -> String {
    format!(
        "schema_version: 2
profile: {{ name: s9 }}
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

fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path).expect("metadata").mode() & 0o777
}

fn chmod(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

/// The directory both sockets sit in.
fn run_dir(home: &Home) -> PathBuf {
    home.data_socket()
        .parent()
        .expect("a run directory")
        .to_path_buf()
}

/// Frames on one raw connection, the hello written by the test.
struct Raw {
    stream: UnixStream,
    buf: Vec<u8>,
}

impl Raw {
    async fn connect(socket: &Path) -> Self {
        Self {
            stream: UnixStream::connect(socket).await.expect("connects"),
            buf: Vec::new(),
        }
    }

    async fn send(&mut self, body: &str) {
        self.stream
            .write_all(&encode_frame(body).expect("a frame"))
            .await
            .expect("sent");
    }

    /// The next frame other than a keepalive probe, or `None` once the
    /// daemon has closed the connection.
    async fn next(&mut self) -> Option<Frame> {
        tokio::time::timeout(PATIENCE, async {
            loop {
                match decode_frame(&self.buf) {
                    Ok(DecodedFrame { body, consumed }) => {
                        let frame = Frame::parse(&body).expect("a frame");
                        self.buf.drain(..consumed);
                        if !matches!(frame, Frame::Ping(_)) {
                            return Some(frame);
                        }
                        continue;
                    }
                    Err(FrameError::Incomplete { .. }) => {}
                    Err(e) => panic!("a bad frame: {e:?}"),
                }
                let mut chunk = [0_u8; 4096];
                match self.stream.read(&mut chunk).await {
                    Ok(0) | Err(_) => return None,
                    Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                }
            }
        })
        .await
        .expect("answered in time")
    }

    /// The error code of the response to request `id`; the view and
    /// events pushed meanwhile are passed over.
    async fn refusal(&mut self, id: &str) -> TransportError {
        let mut frame = self.next().await;
        while matches!(frame, Some(Frame::ServerState(_) | Frame::Event(_))) {
            frame = self.next().await;
        }
        match frame {
            Some(Frame::Response(response)) => {
                assert_eq!(response.id.as_str(), id, "{response:?}");
                let shown = format!("{response:?}");
                response
                    .error
                    .unwrap_or_else(|| panic!("{id} was granted: {shown}"))
                    .code
            }
            other => panic!("a response to {id}, got {other:?}"),
        }
    }
}

/// A hello claiming to be `transportctl`, asking for `capabilities`.
fn transportctl_hello(capabilities: &str) -> String {
    format!(
        r#"{{"type":"hello","ipc_version":{{"major":2,"minor":2}},
        "client":{{"kind":"transportctl"}},"requested_capabilities":[{capabilities}]}}"#
    )
}

/// `client.kind=transportctl` on the DATA socket buys nothing: a hello
/// asking for `admin.*` is closed `CapabilityDenied`; one asking for the
/// data capabilities is granted only those, and every `admin.*` request
/// on it -- a status read, a trust grant, a shutdown -- is refused
/// before dispatch: the trust list is unchanged, the daemon still serves,
/// and the admin socket counts the three refusals. The control is the
/// admin socket, where the same kind's status read is answered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_data_socket_hello_claiming_transportctl_gets_no_admin_authority() {
    let home = Home::new("s9");
    home.write_key();
    let trusted = stranger();
    home.write_config(&profile(&trusted));
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;

    let mut asking_admin = Raw::connect(&home.data_socket()).await;
    asking_admin
        .send(&transportctl_hello(
            r#""admin.status","admin.trust","admin.shutdown""#,
        ))
        .await;
    match asking_admin.next().await {
        Some(Frame::Close(close)) => assert_eq!(close.code, TransportError::CapabilityDenied),
        other => panic!("closed CapabilityDenied, got {other:?}"),
    }
    assert!(asking_admin.next().await.is_none(), "and the stream ends");

    let mut spoof = Raw::connect(&home.data_socket()).await;
    spoof
        .send(&transportctl_hello(r#""commands","events""#))
        .await;
    match spoof.next().await {
        Some(Frame::HelloResponse(response)) => {
            let granted: Vec<String> = response
                .granted_capabilities
                .iter()
                .map(|c| serde_json::to_value(c).expect("json").to_string())
                .collect();
            assert!(
                granted.iter().all(|c| !c.contains("admin")),
                "nothing admin granted on the data socket: {granted:?}"
            );
        }
        other => panic!("a hello_response, got {other:?}"),
    }
    let someone = stranger();
    for (id, request) in [
        (
            "status",
            r#"{"type":"request","id":"status","method":"admin.status"}"#.to_owned(),
        ),
        (
            "trust",
            format!(
                r#"{{"type":"request","id":"trust","method":"admin.trust.set","params":{{"peer":"{}","allowed":true}}}}"#,
                someone.as_str()
            ),
        ),
        (
            "shutdown",
            r#"{"type":"request","id":"shutdown","method":"admin.shutdown"}"#.to_owned(),
        ),
    ] {
        spoof.send(&request).await;
        assert_eq!(
            spoof.refusal(id).await,
            TransportError::CapabilityDenied,
            "{request}"
        );
    }

    let admin = home
        .binding()
        .admin([AdminCapability::Status, AdminCapability::Trust].into())
        .await
        .expect("the control: the admin socket answers the same kind");
    let status = admin.status_result().await.expect("status");
    assert_eq!(
        status.ipc.cross_domain_capability_denied_total, 3,
        "the three refusals counted"
    );
    let trust = admin.trust().await.expect("the trust list");
    assert_eq!(
        trust.peers().cloned().collect::<Vec<_>>(),
        vec![trusted],
        "the spoofed grant changed nothing"
    );
    assert!(
        daemon.child.try_wait().expect("a status").is_none(),
        "the spoofed shutdown stopped nothing:\n{}",
        daemon.log()
    );
    drop((spoof, admin));
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}

/// Under umask 000 nothing the daemon makes is open to group or other:
/// the run directory `0700`, both sockets `0600`, and every directory
/// and file it wrote under the profile's roots. The control is the
/// umask itself: a file the same shell creates before exec is `0666`, so
/// the daemon inherited it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn under_a_hostile_umask_the_daemon_makes_nothing_group_or_world_accessible() {
    let home = Home::new("s9");
    home.write_key();
    home.write_config(&profile(&stranger()));
    let probe = home.root.path().join("umask-probe");
    let mut daemon = home.start_after(&format!("umask 000\n: > '{}'", probe.display()), &[]);
    daemon.serving(&home).await;
    assert_eq!(mode(&probe), 0o666, "the control: the umask is live");

    assert_eq!(mode(&run_dir(&home)), 0o700, "the run directory");
    assert_eq!(mode(&home.data_socket()), 0o600, "the data socket");
    assert_eq!(mode(&home.admin_socket()), 0o600, "the admin socket");
    let config = home.paths.config_file();
    let mut open = Vec::new();
    let mut walked = 0;
    let mut stack = vec![home.root.path().to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("readable") {
            let path = entry.expect("an entry").path();
            let meta = std::fs::symlink_metadata(&path).expect("metadata");
            if meta.is_dir() {
                stack.push(path.clone());
            }
            // The test's own: the profile document (0644 by design), the
            // probe, and the daemon's log, which the harness opened at the
            // home's root as `daemon-<pid>-<n>.log` -- that name there and
            // nowhere else, so a log the daemon wrote is still judged.
            let prefix = format!("daemon-{}-", std::process::id());
            let harness_log = path.parent() == Some(home.root.path())
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| n.strip_prefix(&prefix))
                    .and_then(|n| n.strip_suffix(".log"))
                    .is_some_and(|n| n.parse::<u64>().is_ok());
            let ours = path == config || path == probe || harness_log;
            if !ours {
                walked += 1;
                if meta.mode() & 0o077 != 0 {
                    open.push(format!("{} {:o}", path.display(), meta.mode() & 0o777));
                }
            }
        }
    }
    assert!(walked > 6, "the walk saw the daemon's tree: {walked}");
    assert!(open.is_empty(), "group or world accessible: {open:?}");
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}

/// A run directory left open -- by a hand, or a umask before the daemon
/// -- is refused, not repaired: the start fails, nothing is bound, and
/// the mode is as it was. The previous case is its control.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_open_run_directory_is_refused_and_left_as_it_was() {
    let home = Home::new("s9");
    home.write_key();
    home.write_config(&profile(&stranger()));
    let dir = run_dir(&home);
    std::fs::create_dir(&dir).expect("the run directory made");
    chmod(&dir, 0o755);
    let mut daemon = home.start_after("umask 000", &[]);
    let status = daemon.exit().await;
    assert_eq!(status.code(), Some(1), "{}", daemon.log());
    assert_eq!(mode(&dir), 0o755, "not repaired");
    assert!(!home.data_socket().exists() && !home.admin_socket().exists());
    assert!(daemon.log().contains("mode 755"), "{}", daemon.log());
}

/// The helper-user variable of decided (4): the account the CI job
/// switches to with `sudo -n -u`.
const FOREIGN_USER: &str = "INTERWEAVE_FOREIGN_UID_USER";

/// What a client of another uid does: connect to `path`, send nothing,
/// and report how that ended -- or stat it, with `stat`.
const CLIENT: &str = r#"
import errno, os, socket, sys
mode, path = sys.argv[1], sys.argv[2]
if mode == "stat":
    try:
        os.stat(path); print("reachable")
    except OSError as e:
        print("unreachable", errno.errorcode.get(e.errno, e.errno))
    sys.exit(0)
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
try:
    s.connect(path)
except OSError as e:
    print("refused", errno.errorcode.get(e.errno, e.errno)); sys.exit(0)
s.settimeout(10)
try:
    data = s.recv(65536)
except socket.timeout:
    print("open"); sys.exit(0)
except OSError as e:
    print("reset", errno.errorcode.get(e.errno, e.errno)); sys.exit(0)
print("closed" if not data else "answered")
"#;

/// `args` run as `user` through `sudo -n -u`, its one line of output. A
/// refused switch fails the test by name: P2's rule, a test that did not
/// run is a red job and never a green row.
fn as_user(user: &str, args: &[&str]) -> String {
    let out = Command::new("sudo")
        .args(["-n", "-u", user, "--"])
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("sudo does not run: {e}"));
    assert!(
        out.status.success(),
        "sudo -n -u {user} was refused ({}): {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf-8")
        .trim()
        .to_owned()
}

fn as_foreign_client(user: &str, mode: &str, path: &Path) -> String {
    as_user(
        user,
        &["python3", "-c", CLIENT, mode, path.to_str().expect("utf-8")],
    )
}

/// The peer-credential refusals the daemon has counted.
async fn refused_total(home: &Home) -> u64 {
    home.binding()
        .admin([AdminCapability::Status].into())
        .await
        .expect("an admin port")
        .status_result()
        .await
        .expect("status")
        .ipc
        .peer_credential_refused_total
}

/// Decided (4): a REAL second uid, in two layers and in this order.
/// Under the modes the daemon makes (`LOCAL-IPC.md`: the run directory
/// `0700`, both sockets `0600`) the filesystem refuses it with `EACCES`
/// and the daemon never sees it -- its count stays 0. Then, the modes
/// widened by the owner one at a time (the directory alone still leaves
/// `EACCES`, from the sockets' own mode), its connection reaches the
/// daemon, which closes it before `hello` and counts it: the
/// peer-credential check, which the first layer alone never exercises.
///
/// The ancestors above the run directory are opened to traversal first,
/// so the first layer's refusal is the daemon's modes and not the
/// scratch directory's. `sudo -n -u` and `python3` must work for the
/// account the variable names; that the switch changed the uid is
/// checked, not assumed. The Stage 13 injected-uid test
/// (`crates/local/ipc-server/src/server.rs`,
/// `a_peer_of_another_uid_is_closed_before_hello_and_counted`) stays the
/// standing proof on a host without a second account.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs INTERWEAVE_FOREIGN_UID_USER, an account sudo -n -u reaches"]
async fn a_foreign_uid_is_refused_by_the_modes_and_then_by_the_peer_credential_check() {
    let user = std::env::var(FOREIGN_USER).unwrap_or_else(|_| {
        panic!("{FOREIGN_USER} is unset: this test needs a second account that sudo -n -u reaches")
    });
    let foreign_uid = as_user(&user, &["id", "-u"]);
    let own_uid = interweave_profile_config::effective_uid()
        .expect("this process's uid")
        .to_string();
    assert_ne!(foreign_uid, own_uid, "sudo -u {user} switched the uid");

    let home = Home::new("s9");
    home.write_key();
    home.write_config(&profile(&stranger()));
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    let run = run_dir(&home);
    let mut ancestor = run.parent();
    while let Some(dir) = ancestor {
        if !dir.starts_with(home.root.path()) {
            break;
        }
        chmod(dir, mode(dir) | 0o011);
        ancestor = dir.parent();
    }
    let reach = as_foreign_client(&user, "stat", run.parent().expect("a parent"));
    assert_eq!(
        reach, "reachable",
        "{user} must reach the run directory's parent (TMPDIR under a private home?)"
    );
    let sockets = [home.data_socket(), home.admin_socket()];

    // Layer 1: the modes the daemon made.
    assert_eq!(
        (mode(&run), mode(&sockets[0]), mode(&sockets[1])),
        (0o700, 0o600, 0o600)
    );
    for socket in &sockets {
        assert_eq!(
            as_foreign_client(&user, "connect", socket),
            "refused EACCES",
            "{}",
            socket.display()
        );
    }
    assert_eq!(refused_total(&home).await, 0, "the daemon never saw it");

    // The directory widened alone: the sockets' own mode still refuses.
    chmod(&run, 0o711);
    for socket in &sockets {
        assert_eq!(
            as_foreign_client(&user, "connect", socket),
            "refused EACCES",
            "{} at 0600",
            socket.display()
        );
    }
    assert_eq!(refused_total(&home).await, 0, "still never seen");

    // Layer 2: the sockets widened too, the connection reaches the daemon.
    for socket in &sockets {
        chmod(socket, 0o666);
        let ended = as_foreign_client(&user, "connect", socket);
        assert!(
            ended == "closed" || ended == "reset ECONNRESET",
            "{} closed without a word, got {ended}",
            socket.display()
        );
    }
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while refused_total(&home).await < 2 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "both refusals counted:\n{}",
            daemon.log()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(refused_total(&home).await, 2, "exactly the two");
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}
