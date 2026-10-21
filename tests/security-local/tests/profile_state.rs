// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! S10's profile-state half (plan section 21; threat-model.md's rows on
//! a swapped ancestor or link, and on the durable trust and endpoint
//! overlays tampered with or deleted), through the shipped daemon.
//!
//! `profile-config` holds each rule one function at a time
//! (`src/persist.rs`, `tests/trust_overlay.rs`, `tests/endpoint_overlay.rs`)
//! and the composition holds the overlays in-process
//! (`crates/transport/composition/tests/{trust,endpoint}_overlay.rs`).
//! What is new here is the daemon binary itself: a start refused with
//! status 1 and the cause named in its log, nothing created or rewritten,
//! no socket served -- beside the control, the same home starting once the
//! cause is put right -- and a set over the admin socket whose write fails
//! answered failed with nothing changed.
//!
//! Every case needs the modes to bind, so none runs as root, which the
//! filesystem exempts (`not_root`).

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::Command;

use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort};
use interweave_profile_config::ProfilePaths;
use interweave_test_support::e2e::{Daemon, Home, stranger};
use interweave_transport_api::{EndpointId, TransportError, TransportIdentity};

/// One profile, two endpoints -- `human` the default, `agent` beside it,
/// so `agent` can be disabled -- trusting `peer`, on loopback.
fn profile(peer: &TransportIdentity) -> String {
    format!(
        "schema_version: 2
profile: {{ name: s10 }}
trust: {{ policy: static-allowlist, allowed_peers: [\"{}\"] }}
endpoints:
  default_direct_endpoint: human
  entries:
    - {{ id: human, enabled: true, advertise: false }}
    - {{ id: agent, enabled: true, advertise: false }}
channels: {{ desired: [] }}
discovery: {{ providers: [] }}
transport: {{ listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/0\"] }} }}
",
        peer.as_str()
    )
}

fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path).expect("metadata").mode() & 0o7777
}

fn chmod(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

fn mkdir(path: &Path, mode: u32) {
    std::fs::create_dir_all(path).expect("mkdir");
    chmod(path, mode);
}

/// The modes bind this process: root would pass every refusal here.
fn not_root() {
    assert_ne!(
        interweave_profile_config::effective_uid().expect("this process's uid"),
        0,
        "run as root, the filesystem ignores the modes these cases rest on"
    );
}

/// A home with its key and the profile trusting `peer`.
fn home(peer: &TransportIdentity) -> Home {
    let home = Home::new("s10");
    home.write_key();
    home.write_config(&profile(peer));
    home
}

/// The daemon is refused at start: status 1, every one of `named` in its
/// log, and neither socket served.
async fn refused(home: &Home, named: &[&str]) -> String {
    let mut daemon = home.start(&[]);
    let status = daemon.exit().await;
    let log = daemon.log();
    assert_eq!(status.code(), Some(1), "{log}");
    for name in named {
        assert!(log.contains(name), "{name:?} not in:\n{log}");
    }
    assert!(
        !home.data_socket().exists() && !home.admin_socket().exists(),
        "a refused start serves no socket"
    );
    log
}

/// The daemon serving, and an admin port on it holding the trust and
/// endpoint capabilities.
async fn up(home: &Home) -> (Daemon, impl AdminPort) {
    let mut daemon = home.start(&[]);
    daemon.serving(home).await;
    let admin = home
        .binding()
        .admin([AdminCapability::Trust, AdminCapability::Endpoints].into())
        .await
        .expect("an admin port");
    (daemon, admin)
}

/// The port dropped and the daemon stopped cleanly.
async fn down(mut daemon: Daemon, admin: impl AdminPort) {
    drop(admin);
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}

async fn trusted(admin: &impl AdminPort) -> Vec<TransportIdentity> {
    admin
        .trust()
        .await
        .expect("the trust view")
        .peers()
        .cloned()
        .collect()
}

/// `(id, enabled)` of every configured endpoint, in id order.
async fn endpoints(admin: &impl AdminPort) -> Vec<(String, bool)> {
    admin
        .leases()
        .await
        .expect("the endpoint rows")
        .into_iter()
        .map(|row| (row.endpoint.as_str().to_owned(), row.enabled))
        .collect()
}

/// The trusted peers of a fresh start, the daemon stopped after.
async fn trusted_after_start(home: &Home) -> Vec<TransportIdentity> {
    let (daemon, admin) = up(home).await;
    let out = trusted(&admin).await;
    down(daemon, admin).await;
    out
}

/// The endpoint rows of a fresh start, the daemon stopped after.
async fn endpoints_after_start(home: &Home) -> Vec<(String, bool)> {
    let (daemon, admin) = up(home).await;
    let out = endpoints(&admin).await;
    down(daemon, admin).await;
    out
}

/// The daemon starts and stops: the control that a home is good.
async fn starts(home: &Home) {
    let (daemon, admin) = up(home).await;
    down(daemon, admin).await;
}

fn agent() -> EndpointId {
    EndpointId::parse("agent").expect("an endpoint id")
}

// ---------------------------------------------------------------- TM-49

/// TM-49, the write bits: an ancestor of the profile's state directory
/// that another account could rename things in -- other-writable, not
/// sticky -- stops the daemon naming the ancestor and its mode, and
/// nothing is created under it. The control: the same home, the bit
/// taken off, starts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_other_writable_ancestor_of_the_state_directory_stops_the_daemon() {
    not_root();
    let home = home(&stranger());
    let state = home.roots.state_home.clone();
    mkdir(&state, 0o777);
    let log = refused(
        &home,
        &[
            &state.display().to_string(),
            "mode 0777: other-writable and not sticky",
        ],
    )
    .await;
    assert_eq!(
        std::fs::read_dir(&state).expect("readable").count(),
        0,
        "nothing created under the refused ancestor:\n{log}"
    );

    chmod(&state, 0o700);
    starts(&home).await;
}

/// TM-49, the link: the state directory reached through a symbolic link
/// that sits in an other-writable directory -- where another account
/// could swap the link -- stops the daemon naming that directory. The
/// control: the same link in a directory only its owner writes is
/// followed, and the daemon starts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_link_on_the_path_in_an_other_writable_directory_stops_the_daemon() {
    not_root();
    let mut home = home(&stranger());
    let base = home.root.path().to_path_buf();
    let open = base.join("open");
    let real = base.join("real-state");
    mkdir(&open, 0o777);
    mkdir(&real, 0o700);
    let link = open.join("state");
    std::os::unix::fs::symlink(&real, &link).expect("the link");
    relocate_state(&mut home, link);
    let log = refused(
        &home,
        &[
            &open.display().to_string(),
            "mode 0777: other-writable and not sticky",
        ],
    )
    .await;
    assert_eq!(
        std::fs::read_dir(&real).expect("readable").count(),
        0,
        "nothing created through the refused link:\n{log}"
    );

    chmod(&open, 0o700);
    starts(&home).await;
}

/// `XDG_STATE_HOME` moved to `to`, the profile's paths resolved again.
fn relocate_state(home: &mut Home, to: PathBuf) {
    home.roots.state_home = to;
    home.paths = ProfilePaths::resolve(home.paths.profile(), &home.roots).expect("paths");
}

/// The helper-user variable of decided (4): the account reached with
/// `sudo -n -u`.
const FOREIGN_USER: &str = "INTERWEAVE_FOREIGN_UID_USER";

/// `args` run as `user` through `sudo -n -u`; a refused switch fails the
/// test by name, never a green row for a case that did not run.
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

/// TM-49, the owner, with a REAL second uid: the state directory reached
/// through a link another account made, in a directory it owns, stops
/// the daemon naming that account's uid -- whatever the modes, which are
/// that account's to change. The control is the link case above: our
/// own link in our own directory is followed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs INTERWEAVE_FOREIGN_UID_USER, an account sudo -n -u reaches"]
async fn a_link_another_account_owns_on_the_path_stops_the_daemon() {
    not_root();
    let user = std::env::var(FOREIGN_USER).unwrap_or_else(|_| {
        panic!("{FOREIGN_USER} is unset: this test needs a second account that sudo -n -u reaches")
    });
    let own_uid = interweave_profile_config::effective_uid().expect("this process's uid");

    let mut home = home(&stranger());
    let real = home.root.path().join("real-state");
    mkdir(&real, 0o700);
    // The other account's directory, in the sticky /tmp it can write.
    let theirs = PathBuf::from(as_user(&user, &["mktemp", "-d", "/tmp/iw-tm49-XXXXXXXX"]));
    let cleanup = Cleanup {
        user: user.clone(),
        dir: theirs.clone(),
    };
    as_user(&user, &["chmod", "0755", theirs.to_str().expect("utf-8")]);
    // The owner as this process sees it, which is what the daemon judges
    // -- not `id -u` inside the switch, which a user namespace renames.
    let foreign_uid = std::fs::metadata(&theirs).expect("their directory").uid();
    assert_ne!(foreign_uid, own_uid, "sudo -u {user} switched the uid");
    let link = theirs.join("state");
    as_user(
        &user,
        &[
            "ln",
            "-s",
            real.to_str().expect("utf-8"),
            link.to_str().expect("utf-8"),
        ],
    );
    relocate_state(&mut home, link);
    refused(
        &home,
        &[
            &format!("owned by uid {foreign_uid}"),
            &format!("owned by neither root nor uid {own_uid}"),
        ],
    )
    .await;
    assert_eq!(
        std::fs::read_dir(&real).expect("readable").count(),
        0,
        "nothing created through the other account's link"
    );
    drop(cleanup);
}

/// Removes the other account's directory however the test ends.
struct Cleanup {
    user: String,
    dir: PathBuf,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = Command::new("sudo")
            .args(["-n", "-u", &self.user, "--", "rm", "-rf"])
            .arg(&self.dir)
            .status();
    }
}

// ---------------------------------------------------------------- TM-50

/// TM-50: a revoke kept in `<state>/trust-overlay.json` holds across a
/// restart (the control that the overlay is in force); the file
/// tampered with -- garbage, readable by others, the peer in both lists --
/// stops the start naming it, each time left as it was found, and
/// the overlay put back starts with the revoke in force. Deleted, the
/// revoked peer is allowed again: by design (threat-model.md's residual,
/// the backup table's "never delete it to reset"), pinned so a change
/// to it is seen.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_tampered_trust_overlay_stops_the_daemon_and_a_deleted_one_re_allows() {
    not_root();
    let peer = stranger();
    let home = home(&peer);
    let overlay = home.paths.state_dir().join("trust-overlay.json");
    let (daemon, admin) = up(&home).await;
    assert_eq!(trusted(&admin).await, vec![peer.clone()]);
    admin
        .set_trust(peer.clone(), false)
        .await
        .expect("the revoke");
    down(daemon, admin).await;
    assert_eq!(mode(&overlay), 0o600, "written owner-only");
    let kept = std::fs::read(&overlay).expect("the overlay");
    let still = trusted_after_start(&home).await;
    assert!(
        still.is_empty(),
        "the revoke held across the restart: {still:?}"
    );

    let both = format!(
        r#"{{"added":["{p}"],"revoked":["{p}"]}}"#,
        p = peer.as_str()
    );
    for (what, bytes, file_mode) in [
        ("garbage", b"{ not json".to_vec(), 0o600),
        ("readable by others", kept.clone(), 0o644),
        ("a peer in both lists", both.into_bytes(), 0o600),
    ] {
        std::fs::write(&overlay, &bytes).expect("tampered");
        chmod(&overlay, file_mode);
        let log = refused(&home, &["the trust overlay"]).await;
        assert_eq!(
            (std::fs::read(&overlay).expect("left"), mode(&overlay)),
            (bytes, file_mode),
            "{what}: the file left as found:\n{log}"
        );
    }

    std::fs::write(&overlay, &kept).expect("put back");
    chmod(&overlay, 0o600);
    let back = trusted_after_start(&home).await;
    assert!(back.is_empty(), "the put-back overlay in force: {back:?}");

    std::fs::remove_file(&overlay).expect("deleted");
    let deleted = trusted_after_start(&home).await;
    assert_eq!(
        deleted,
        vec![peer],
        "deleted, the revoke is gone, by design"
    );
}

// ---------------------------------------------------------------- TM-51

/// TM-51: an endpoint disabled into `<state>/endpoint-overlay.json`
/// stays disabled across a restart (the control); the file tampered
/// with -- garbage, readable by others, an entry the profile's
/// validation refuses once composed (enabling an endpoint past
/// `directory.max_advertised`) -- stops the start naming it, left as
/// found, and the overlay put back starts with the disable in force.
/// Deleted, the endpoint is enabled again: by design, pinned.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_tampered_endpoint_overlay_stops_the_daemon_and_a_deleted_one_re_enables() {
    not_root();
    let home = home(&stranger());
    let overlay = home.paths.state_dir().join("endpoint-overlay.json");
    let (daemon, admin) = up(&home).await;
    admin
        .set_endpoint_enabled(agent(), false)
        .await
        .expect("the disable");
    down(daemon, admin).await;
    assert_eq!(mode(&overlay), 0o600, "written owner-only");
    let kept = std::fs::read(&overlay).expect("the overlay");
    let disabled = vec![("agent".to_owned(), false), ("human".to_owned(), true)];
    assert_eq!(
        endpoints_after_start(&home).await,
        disabled,
        "the disable held across the restart"
    );

    // One advertised endpoint at most, `agent` disabled: config.yaml
    // validates alone, and an overlay enabling `agent` composes into a
    // profile that does not.
    let bounded = profile(&stranger())
        .replace(
            "  default_direct_endpoint: human\n",
            "  default_direct_endpoint: human\n  directory: { max_advertised: 1 }\n",
        )
        .replace(
            "id: human, enabled: true, advertise: false",
            "id: human, enabled: true, advertise: true",
        )
        .replace(
            "id: agent, enabled: true, advertise: false",
            "id: agent, enabled: false, advertise: true",
        );
    for (what, bytes, file_mode, config) in [
        ("garbage", b"{ not json".to_vec(), 0o600, None),
        ("readable by others", kept.clone(), 0o644, None),
        (
            "an enable past the advertised bound",
            br#"{"enabled":{"agent":true}}"#.to_vec(),
            0o600,
            Some(bounded.as_str()),
        ),
    ] {
        if let Some(config) = config {
            home.write_config(config);
        }
        std::fs::write(&overlay, &bytes).expect("tampered");
        chmod(&overlay, file_mode);
        let log = refused(&home, &["the endpoint overlay"]).await;
        assert_eq!(
            (std::fs::read(&overlay).expect("left"), mode(&overlay)),
            (bytes, file_mode),
            "{what}: the file left as found:\n{log}"
        );
    }

    home.write_config(&profile(&stranger()));
    std::fs::write(&overlay, &kept).expect("put back");
    chmod(&overlay, 0o600);
    assert_eq!(
        endpoints_after_start(&home).await,
        disabled,
        "the put-back overlay in force"
    );

    std::fs::remove_file(&overlay).expect("deleted");
    assert_eq!(
        endpoints_after_start(&home).await,
        vec![("agent".to_owned(), true), ("human".to_owned(), true)],
        "deleted, the disable is gone, by design"
    );
}

// ---------------------------------------------------------- write fault

/// S10, profile state under a write fault (beside the store's
/// `durable_write_fault.rs`): with the state directory unwritable, a
/// revoke and a disable over the admin socket are each answered failed,
/// nothing is written, and the running policy is unchanged -- never a
/// change answered and published that the next start would not have.
/// The control: the directory writable again, the same two sets are
/// answered and survive a restart.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_set_whose_overlay_cannot_be_written_is_answered_failed_and_changes_nothing() {
    not_root();
    let peer = stranger();
    let home = home(&peer);
    let state = home.paths.state_dir().to_path_buf();
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    chmod(&state, 0o500);
    let admin = home
        .binding()
        .admin([AdminCapability::Trust, AdminCapability::Endpoints].into())
        .await
        .expect("an admin port");
    let revoke = admin.set_trust(peer.clone(), false).await;
    let disable = admin.set_endpoint_enabled(agent(), false).await;
    chmod(&state, 0o700);
    assert_eq!(revoke, Err(TransportError::Internal), "{}", daemon.log());
    assert!(
        matches!(disable, Err(TransportError::Internal)),
        "{disable:?}\n{}",
        daemon.log()
    );
    for file in ["trust-overlay.json", "endpoint-overlay.json"] {
        assert!(!state.join(file).exists(), "{file} not written");
    }
    assert_eq!(
        trusted(&admin).await,
        vec![peer.clone()],
        "the peer still trusted"
    );
    assert_eq!(
        endpoints(&admin).await,
        vec![("agent".to_owned(), true), ("human".to_owned(), true)],
        "the endpoint still enabled"
    );

    admin
        .set_trust(peer, false)
        .await
        .expect("the control revoke");
    admin
        .set_endpoint_enabled(agent(), false)
        .await
        .expect("the control disable");
    drop(admin);
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
    let (daemon, admin) = up(&home).await;
    let (trust, rows) = (trusted(&admin).await, endpoints(&admin).await);
    down(daemon, admin).await;
    assert!(trust.is_empty(), "the control revoke survived: {trust:?}");
    assert_eq!(
        rows[0],
        ("agent".to_owned(), false),
        "the control disable survived"
    );
}
