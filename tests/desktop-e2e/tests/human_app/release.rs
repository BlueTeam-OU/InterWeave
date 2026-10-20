// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Stage 19's Validate rows a person's install depends on (plan section
//! 22): fresh install, upgrade, rollback, profile/config migration,
//! `PeerId` preservation and the recovery drill, each run by the package's
//! own executables (`package.rs` names the seam).
//!
//! The previous build is two things (architect-cto's rulings on j30's
//! Q1, 2026-10-20). A TREE, on every run: `test-data/upgrade/<commit>/`
//! is the XDG tree
//! the build of that commit left after a person's ordinary use -- a key,
//! the shipped desktop profile, a trust and an endpoint overlay written
//! by its daemon, a store holding all three retention sets -- with
//! `tree.json` saying what it holds and the README how it was made
//! ([`write_the_previous_build_tree`]). Upgrade is this build reading that
//! tree; rollback is this build meeting a NEWER format of each artefact
//! and refusing it with nothing written and the `PeerId` kept, which is
//! what an older build owes a tree a newer one wrote (ADR-0044's fail
//! safe; `human-client-packaging.md` section 7). And the real older
//! BINARIES of the previous-build axis (`tools/ci/previous-builds.txt`),
//! daemon and `transportctl`, run old -> new -> old over that tree, with
//! what the old build does on it RECORDED
//! ([`a_rollback_to_each_previous_build_keeps_the_peer_id_and_is_recorded`]).
//!
//! What this does NOT prove, by name: an older CLIENT on this build's
//! tree, a `limit` while no older `human-desktop` is on the axis; that a
//! build before the profile-format marker refuses a newer tree -- it
//! cannot, and the axis row records what it does instead; a tree whose
//! formats are older than this build's own (the committed tree is the
//! first, and has today's formats -- the store's migrations from older
//! versions are proved in isolation by `crates/human/store`'s cases); a
//! package's installer, unit or launcher, which these cases bypass by
//! starting each executable bare.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use interweave_human_chat_protocol::encode_outbound;
use interweave_human_store::{
    AppMessageId, HumanStore, InboundOrigin, NewInbound, NewOutbound, OutboundDestination,
    SCHEMA_VERSION, StoreOptions,
};
use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort as _};
use interweave_profile_config::profile_format::{self, PROFILE_FORMAT};
use interweave_transport_api::{
    DirectDestination, MAX_PAYLOAD_BYTES, MediaType, MessageId, TransportIdentity,
};

use interweave_human_ui_model::{UiText, placeholder_en};

use crate::a11y::Bus;
use crate::common::{Home, PATIENCE, example, human, stranger};
use crate::display;
use crate::harness::{self as app, human_lease, until_lease};
use crate::package::{self, Running};
use crate::world::{app_store, envelope, wall_ms};

/// The profile every tree here holds: the shipped desktop example's.
const PROFILE: &str = "human-desktop";

/// The tree the previous build left, under the repository root.
const PREVIOUS_TREE: &str = "test-data/upgrade/d3a193e9";

/// IDENTITY-RECOVERY.md's golden vector (`fixtures/identity/`), the
/// tree's key: TEST-ONLY, the all-zero entropy, public, never a key.
const GOLDEN_WORDS: &str = "abandon abandon abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon abandon abandon art";
const GOLDEN_PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

/// The XDG roots a tree carries; the runtime directory is the session's
/// and is never part of one.
const ROOTS: [&str; 4] = ["config", "data", "state", "cache"];

/// What a tree holds, as `tree.json` records it.
#[derive(Debug)]
struct Manifest {
    /// The commit whose build wrote the tree.
    made_by: String,
    /// The store's schema version that build wrote.
    store_schema: i64,
    /// The profile's `PeerId`.
    peer: String,
    /// `config.yaml`'s allowlist.
    configured: Vec<String>,
    /// Allowed by `transportctl trust allow`, in the trust overlay.
    added: String,
    /// Revoked by `transportctl trust revoke`, in the trust overlay.
    revoked: String,
    /// Disabled by `transportctl endpoints disable`, in the endpoint
    /// overlay.
    disabled_endpoint: String,
    /// The pending outbound row's application id.
    pending: String,
    /// The unread inbound row's.
    unread: String,
    /// The receiver-kept inbound row's.
    kept: String,
}

impl Manifest {
    fn read(tree: &Path) -> Self {
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(tree.join("tree.json")).expect("tree.json"))
                .expect("a manifest");
        let text = |key: &str| {
            value[key]
                .as_str()
                .unwrap_or_else(|| panic!("tree.json's {key}"))
                .to_owned()
        };
        Self {
            made_by: text("made_by"),
            store_schema: value["store_schema"]
                .as_i64()
                .expect("tree.json's store_schema"),
            peer: text("peer"),
            configured: value["configured"]
                .as_array()
                .expect("tree.json's configured")
                .iter()
                .map(|p| p.as_str().expect("a peer").to_owned())
                .collect(),
            added: text("added"),
            revoked: text("revoked"),
            disabled_endpoint: text("disabled_endpoint"),
            pending: text("pending"),
            unread: text("unread"),
            kept: text("kept"),
        }
    }

    fn write(&self, path: &Path) {
        let value = serde_json::json!({
            "made_by": self.made_by,
            "store_schema": self.store_schema,
            "peer": self.peer,
            "configured": self.configured,
            "added": self.added,
            "revoked": self.revoked,
            "disabled_endpoint": self.disabled_endpoint,
            "pending": self.pending,
            "unread": self.unread,
            "kept": self.kept,
        });
        let mut text = serde_json::to_string_pretty(&value).expect("json");
        text.push('\n');
        std::fs::write(path, text).expect("tree.json");
    }
}

fn previous_tree() -> PathBuf {
    interweave_test_support::repo_root().join(PREVIOUS_TREE)
}

/// `tree` installed as a fresh home: its files copied in, and the modes
/// the build that wrote them gave them put back, which git does not keep
/// (directories 0700, `config.yaml` 0644, every other file 0600).
fn install(tree: &Path) -> Home {
    let home = Home::new(PROFILE);
    for root in ROOTS {
        let from = tree.join(root);
        if from.exists() {
            copy_private(&from, &home.root.path().join(root));
        }
    }
    home
}

fn copy_private(from: &Path, to: &Path) {
    crate::common::private_dir(to);
    for entry in std::fs::read_dir(from).expect("a tree directory") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a type").is_dir() {
            copy_private(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copied");
            let mode = if entry.file_name() == "config.yaml" {
                0o644
            } else {
                0o600
            };
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(mode))
                .expect("chmod");
        }
    }
}

/// Every file of `home`'s XDG tree and its bytes, the runtime directory
/// aside. The two lock files are left out: each is created empty to be
/// locked, holds nothing, and a refused start may leave one.
fn snapshot(home: &Home) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(dir: &Path, base: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let entry = entry.expect("an entry");
            let path = entry.path();
            if entry.file_type().expect("a type").is_dir() {
                walk(&path, base, out);
            } else if !matches!(
                entry.file_name().to_str(),
                Some("profile.lock" | "human-desktop.lock")
            ) {
                let bytes = std::fs::read(&path).expect("readable");
                out.insert(path.strip_prefix(base).expect("inside").to_owned(), bytes);
            }
        }
    }
    let mut out = BTreeMap::new();
    for root in ROOTS {
        walk(&home.root.path().join(root), home.root.path(), &mut out);
    }
    out
}

/// Which files differ between two snapshots, by name.
fn changed(before: &BTreeMap<PathBuf, Vec<u8>>, after: &BTreeMap<PathBuf, Vec<u8>>) -> Vec<String> {
    let mut names: Vec<_> = before.keys().chain(after.keys()).collect();
    names.sort();
    names.dedup();
    names
        .into_iter()
        .filter(|name| before.get(*name) != after.get(*name))
        .map(|name| name.display().to_string())
        .collect()
}

/// The store's `user_version`, read-only.
fn store_version(home: &Home) -> i64 {
    let conn = rusqlite::Connection::open_with_flags(
        app_store(home),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("the store opens read-only");
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .expect("a version")
}

/// The application ids of `table`'s rows, read-only.
fn ids(home: &Home, table: &str) -> Vec<String> {
    let conn = rusqlite::Connection::open_with_flags(
        app_store(home),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("the store opens read-only");
    conn.busy_timeout(Duration::from_secs(5))
        .expect("a timeout");
    let mut stmt = conn
        .prepare(&format!(
            "SELECT app_message_id FROM {table} ORDER BY app_message_id"
        ))
        .expect("a select");
    stmt.query_map([], |r| r.get(0))
        .expect("rows")
        .map(|r| r.expect("a row"))
        .collect()
}

/// Every WAL frame folded into the store file and the WAL emptied, as a
/// clean close leaves it: what the tree commits, and what a newer
/// header must be written into to be seen without the WAL.
fn checkpoint(path: &Path) {
    let conn = rusqlite::Connection::open(path).expect("the store opens");
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("checkpointed");
}

/// The daemon of `home`, from the package, ANSWERING: a socket accepts
/// a connection once it is bound, before the daemon raises the
/// profile-format marker and starts serving, so a snapshot taken on
/// accept alone races the marker's write.
async fn serving(home: &Home) -> Running {
    let mut daemon = package::start_daemon(home);
    daemon.serving(home).await;
    package::status_peer(home).await;
    daemon
}

/// What the daemon's trust policy allows, as strings.
async fn allowed(home: &Home) -> Vec<String> {
    let binding = home.binding();
    let port = binding
        .admin([AdminCapability::Trust].into())
        .await
        .expect("an admin port");
    let view = port.trust().await.expect("the trust view");
    view.peers().map(|p| p.as_str().to_owned()).collect()
}

/// Whether `endpoint` is enabled, as the daemon reports it.
async fn endpoint_enabled(home: &Home, endpoint: &str) -> bool {
    let binding = home.binding();
    let port = binding
        .admin([AdminCapability::Endpoints].into())
        .await
        .expect("an admin port");
    port.leases()
        .await
        .expect("the endpoints")
        .into_iter()
        .find(|row| row.endpoint.as_str() == endpoint)
        .unwrap_or_else(|| panic!("{endpoint} is configured"))
        .enabled
}

/// Fresh install: an empty tree, the package's `transportctl identity
/// init`, the shipped desktop profile, then the daemon and the window.
/// The `PeerId` `init` printed is the one the daemon serves, before and
/// after a restart, and the window's first run creates the store at this
/// build's schema.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_fresh_install_keeps_the_peer_id_init_printed() {
    let home = Home::new(PROFILE);
    let printed = package::transportctl_ok(&home, &["identity", "init"], "");
    let printed = printed.trim().to_owned();
    TransportIdentity::parse(&printed).expect("init prints a PeerId");
    home.write_config(&example(
        "human-desktop.yaml",
        &stranger(),
        "/ip4/127.0.0.1/tcp/0",
        None,
    ));
    let key = std::fs::read(home.paths.identity_file()).expect("the key");

    let mut daemon = serving(&home).await;
    assert_eq!(package::status_peer(&home).await.as_str(), printed);
    let mut client = app::start(&home);
    until_lease(&home.binding(), true, &client).await;
    assert!(client.terminate().success(), "{}", client.log());
    assert_eq!(store_version(&home), SCHEMA_VERSION);
    assert!(daemon.terminate().success(), "{}", daemon.log());

    let mut again = serving(&home).await;
    assert_eq!(
        package::status_peer(&home).await.as_str(),
        printed,
        "a restart serves the same PeerId"
    );
    assert!(again.terminate().success(), "{}", again.log());
    assert_eq!(
        std::fs::read(home.paths.identity_file()).expect("the key"),
        key,
        "the key file is never rewritten"
    );
}

/// Upgrade: this build over the previous build's tree reads every
/// artefact it kept -- the key (same `PeerId`), the profile, the trust
/// overlay (the added peer allowed, the revoked one not), the endpoint
/// overlay (the disabled endpoint still disabled), the store with all
/// three retention sets, migrated to this build's schema -- and rewrites
/// none of the key, the profile or the overlays.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_upgrade_reads_everything_the_previous_build_kept() {
    let tree = previous_tree();
    let manifest = Manifest::read(&tree);
    assert_eq!(
        manifest.peer, GOLDEN_PEER,
        "the tree's key is the golden vector"
    );
    // An upgrade reads an older or equal store, never a newer one: that is
    // the rollback case below.
    assert!(
        manifest.store_schema <= SCHEMA_VERSION,
        "the tree from {} is at store schema {}, newer than this build's {SCHEMA_VERSION}",
        manifest.made_by,
        manifest.store_schema
    );
    let home = install(&tree);
    let before = snapshot(&home);

    let mut daemon = serving(&home).await;
    assert_eq!(package::status_peer(&home).await.as_str(), manifest.peer);
    let allowed = allowed(&home).await;
    assert!(allowed.contains(&manifest.added), "{allowed:?}");
    assert!(!allowed.contains(&manifest.revoked), "{allowed:?}");
    for peer in manifest
        .configured
        .iter()
        .filter(|p| **p != manifest.revoked)
    {
        assert!(allowed.contains(peer), "{peer} in {allowed:?}");
    }
    assert!(!endpoint_enabled(&home, &manifest.disabled_endpoint).await);
    assert!(endpoint_enabled(&home, human().as_str()).await);

    let mut client = app::start(&home);
    until_lease(&home.binding(), true, &client).await;
    assert!(client.terminate().success(), "{}", client.log());
    assert_eq!(store_version(&home), SCHEMA_VERSION);
    assert_eq!(
        ids(&home, "pending_outbound"),
        std::slice::from_ref(&manifest.pending)
    );
    assert_eq!(
        ids(&home, "unread_inbound"),
        std::slice::from_ref(&manifest.unread)
    );
    assert_eq!(
        ids(&home, "kept_inbound"),
        std::slice::from_ref(&manifest.kept)
    );
    assert!(daemon.terminate().success(), "{}", daemon.log());

    let after = snapshot(&home);
    let rewritten: Vec<_> = changed(&before, &after)
        .into_iter()
        .filter(|name| !name.contains("/human/") && !name.ends_with("/peers.json"))
        .collect();
    // The tree predates the profile-format marker (ADR-0028 A
    // 2026-10-20), so an upgrade is the marker's first write: this
    // build's format, and nothing else beside the store and the cache.
    let marker = profile_format::path_for(&home.paths);
    let marker_name = marker
        .strip_prefix(home.root.path())
        .expect("inside")
        .display()
        .to_string();
    assert_eq!(
        profile_format::read_file(&marker).expect("the marker"),
        PROFILE_FORMAT,
        "the upgrade raised the marker to this build's format"
    );
    let rewritten: Vec<_> = rewritten
        .into_iter()
        .filter(|name| *name != marker_name)
        .collect();
    assert!(
        rewritten.is_empty(),
        "the key, the profile and the overlays are read, never rewritten: {rewritten:?}"
    );
}

/// Rollback, the client's half: a store whose header says a newer build
/// wrote it stops the window before it takes the endpoint, says why, and
/// leaves every file as it was; the daemon's `PeerId` is untouched.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_store_from_a_newer_build_stops_the_client_and_is_left_as_it_was() {
    let home = install(&previous_tree());
    let store = app_store(&home);
    checkpoint(&store);
    rusqlite::Connection::open(&store)
        .expect("the store opens")
        .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
        .expect("a newer header");
    checkpoint(&store);
    let mut daemon = serving(&home).await;
    let before = snapshot(&home);

    let _focus = display::exclusive().await;
    let bus = Bus::connect().await;
    let mut client = app::start(&home);
    // Shown, not only said: the refusal screen names the store.
    let screen = bus.window_of(client.child.id(), || client.log()).await;
    screen
        .until(
            "why",
            |e| e.name == placeholder_en::text(UiText::RefusalStoreRecovery),
            || client.log(),
        )
        .await;
    let close = screen
        .until(
            "Close",
            |e| e.role == "button" && e.name == placeholder_en::text(UiText::Close),
            || client.log(),
        )
        .await;
    assert!(human_lease(&home.binding()).await.is_none());
    screen.activate(&close, "click").await;
    let status = client.exit_within(PATIENCE);
    // `EX_DATAERR`: the file needs recovery (run.rs).
    assert_eq!(status.code(), Some(65), "{}", client.log());
    assert!(
        client.log().contains("cannot be read by this version"),
        "{}",
        client.log()
    );
    assert!(human_lease(&home.binding()).await.is_none());
    assert_eq!(package::status_peer(&home).await.as_str(), GOLDEN_PEER);
    assert!(daemon.terminate().success(), "{}", daemon.log());
    let after = snapshot(&home);
    assert_eq!(
        changed(&before, &after),
        Vec::<String>::new(),
        "nothing written"
    );
}

/// Rollback, the daemon's half: each of the three artefacts a newer
/// build could write in a format this build does not know -- the trust
/// overlay, the endpoint overlay, `config.yaml` -- stops the daemon
/// before it serves, writes nothing, and once the artefact is put back
/// the same `PeerId` is served.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_overlay_or_profile_from_a_newer_build_stops_the_daemon_and_is_left_as_it_was() {
    let home = install(&previous_tree());
    let newer_json = |path: &Path| {
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).expect("the overlay")).expect("json");
        value
            .as_object_mut()
            .expect("an object")
            .insert("written_by_a_newer_build".to_owned(), true.into());
        serde_json::to_vec(&value).expect("json")
    };
    // (what, its file, its newer form, what the daemon's refusal names)
    let cases: [(&str, PathBuf, &NewerForm, &str); 3] = [
        (
            "trust overlay",
            home.paths.state_dir().join("trust-overlay.json"),
            &newer_json,
            "trust overlay",
        ),
        (
            "endpoint overlay",
            home.paths.state_dir().join("endpoint-overlay.json"),
            &newer_json,
            "endpoint overlay",
        ),
        (
            "profile",
            home.paths.config_file(),
            &|path: &Path| {
                let text = std::fs::read_to_string(path).expect("the profile");
                assert!(text.contains("schema_version: 2\n"));
                text.replacen("schema_version: 2\n", "schema_version: 3\n", 1)
                    .into_bytes()
            },
            "schema_version",
        ),
    ];
    for (what, path, newer, named) in cases {
        let original = std::fs::read(&path).unwrap_or_else(|_| panic!("the {what}"));
        let mode = std::fs::metadata(&path).expect("metadata").permissions();
        std::fs::write(&path, newer(&path)).expect("written");
        std::fs::set_permissions(&path, mode).expect("chmod");
        let before = snapshot(&home);

        let mut daemon = package::start_daemon(&home);
        let status = daemon.exit();
        assert!(
            !status.success(),
            "a newer {what} stops the daemon: {}",
            daemon.log()
        );
        // The refusal is the artefact's, not any start-up failure: the log
        // names it, and once it is put back the same tree serves (below).
        assert!(
            daemon.log().contains(named),
            "the refusal names {named}: {}",
            daemon.log()
        );
        assert!(
            !home.data_socket().exists()
                || std::os::unix::net::UnixStream::connect(home.data_socket()).is_err(),
            "nothing served"
        );
        assert_eq!(
            changed(&before, &snapshot(&home)),
            Vec::<String>::new(),
            "nothing written after a newer {what}"
        );

        std::fs::write(&path, &original).expect("put back");
        let mut daemon = serving(&home).await;
        assert_eq!(package::status_peer(&home).await.as_str(), GOLDEN_PEER);
        assert!(daemon.terminate().success(), "{}", daemon.log());
    }
}

/// An artefact's bytes rewritten in a format a newer build would write.
type NewerForm = dyn Fn(&Path) -> Vec<u8>;

/// The recovery drill on an upgraded install: a backup, the key lost,
/// the daemon refusing to start rather than mint a new identity, and the
/// backup restoring the same `PeerId` with the trust the overlay kept.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_recovery_drill_restores_the_same_peer_id_and_never_mints_one() {
    let tree = previous_tree();
    let manifest = Manifest::read(&tree);
    let home = install(&tree);
    let record = home.root.path().join("record.json");
    package::transportctl_ok(
        &home,
        &[
            "identity",
            "backup",
            "--to-file",
            record.to_str().expect("utf-8"),
        ],
        "",
    );

    std::fs::remove_file(home.paths.identity_file()).expect("the key lost");
    let mut daemon = package::start_daemon(&home);
    assert!(
        !daemon.exit().success(),
        "no key, no start: {}",
        daemon.log()
    );
    assert!(
        !home.paths.identity_file().exists(),
        "a missing key is never replaced by a new one"
    );

    let restored = package::transportctl(
        &home,
        &[
            "identity",
            "restore",
            "--new",
            "--expected-peer-id",
            &manifest.peer,
        ],
        &std::fs::read_to_string(&record).expect("the record"),
    );
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let mut daemon = serving(&home).await;
    assert_eq!(package::status_peer(&home).await.as_str(), manifest.peer);
    assert!(allowed(&home).await.contains(&manifest.added));
    assert!(daemon.terminate().success(), "{}", daemon.log());
}

/// `home`'s profile-format marker set to `n`, owner-only, as a build of
/// that format would leave it.
fn plant_marker(home: &Home, n: u32) {
    let path = profile_format::path_for(&home.paths);
    std::fs::write(&path, format!("{{\"profile_format\":{n}}}")).expect("planted");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
}

/// Rollback past the marker (ADR-0028 A 2026-10-20): a profile a newer
/// build marked stops every program of this one -- the window,
/// `transportctl` and the daemon -- before any of them writes, each
/// naming the marker's file; once it is put back to this build's format,
/// the same `PeerId` is served and the window takes the endpoint.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_profile_a_newer_build_marked_stops_every_program_and_is_left_as_it_was() {
    let home = install(&previous_tree());
    let mut daemon = serving(&home).await;
    assert!(daemon.terminate().success(), "{}", daemon.log());
    plant_marker(&home, PROFILE_FORMAT + 1);
    let before = snapshot(&home);

    let _focus = display::exclusive().await;
    let bus = Bus::connect().await;
    let mut client = app::start(&home);
    // The release page's "it shows which file stopped it": the screen
    // says why and names the marker, whole.
    let screen = bus.window_of(client.child.id(), || client.log()).await;
    screen
        .until(
            "why",
            |e| e.name == placeholder_en::text(UiText::RefusalProfileNewer),
            || client.log(),
        )
        .await;
    let marker = profile_format::path_for(&home.paths).display().to_string();
    let path = screen
        .until(
            "the marker's path",
            |e| e.name == placeholder_en::text(UiText::RefusalPath),
            || client.log(),
        )
        .await;
    assert_eq!(
        screen.text_of(&path).await,
        marker,
        "the file that stopped it, whole"
    );
    let close = screen
        .until(
            "Close",
            |e| e.role == "button" && e.name == placeholder_en::text(UiText::Close),
            || client.log(),
        )
        .await;
    screen.activate(&close, "click").await;
    let status = client.exit_within(PATIENCE);
    // `EX_CONFIG`, as before the screen (run.rs).
    assert_eq!(
        status.code(),
        Some(78),
        "the window refuses: {}",
        client.log()
    );
    assert!(
        client.log().contains(profile_format::PROFILE_FORMAT_FILE),
        "the stderr line says which file stopped it: {}",
        client.log()
    );
    let ctl = package::transportctl(&home, &["identity", "show"], "");
    let said = String::from_utf8_lossy(&ctl.stderr);
    assert!(!ctl.status.success(), "transportctl refuses: {said}");
    assert!(said.contains(profile_format::PROFILE_FORMAT_FILE), "{said}");
    let mut daemon = package::start_daemon(&home);
    assert!(
        !daemon.exit().success(),
        "the daemon refuses: {}",
        daemon.log()
    );
    assert!(
        daemon.log().contains(profile_format::PROFILE_FORMAT_FILE),
        "{}",
        daemon.log()
    );
    assert_eq!(
        changed(&before, &snapshot(&home)),
        Vec::<String>::new(),
        "nothing written by a refused program"
    );

    plant_marker(&home, PROFILE_FORMAT);
    let mut daemon = serving(&home).await;
    assert_eq!(package::status_peer(&home).await.as_str(), GOLDEN_PEER);
    let mut client = app::start(&home);
    until_lease(&home.binding(), true, &client).await;
    assert!(client.terminate().success(), "{}", client.log());
    assert!(daemon.terminate().success(), "{}", daemon.log());
}

/// The previous-build axis (`tools/ci/previous-builds.txt`, built by
/// `tools/ci/build_previous_builds.sh` into the directory
/// `INTERWEAVE_PREVIOUS_BUILDS` names): each entry's `transport-daemon`
/// and `transportctl`, as `<dir>/<label>/`. Unset, the rows that need it
/// fail rather than skip.
fn axis() -> Vec<(String, PathBuf)> {
    let dir = PathBuf::from(
        std::env::var_os("INTERWEAVE_PREVIOUS_BUILDS").unwrap_or_else(|| {
            panic!("INTERWEAVE_PREVIOUS_BUILDS is unset: this row needs the previous builds")
        }),
    );
    let list = interweave_test_support::repo_root().join("tools/ci/previous-builds.txt");
    let entries: Vec<_> = std::fs::read_to_string(&list)
        .expect("the previous-build list")
        .lines()
        .map(|l| l.split('#').next().unwrap_or_default().trim())
        .filter(|l| !l.is_empty())
        .map(|l| {
            let label = l.split_whitespace().next().expect("a label").to_owned();
            let at = dir.join(&label);
            for bin in ["transport-daemon", "transportctl"] {
                assert!(
                    at.join(bin).is_file(),
                    "{label} has no {bin} in {}",
                    dir.display()
                );
            }
            (label, at)
        })
        .collect();
    assert!(!entries.is_empty(), "the axis has an entry");
    entries
}

/// Rollback with real binaries (architect-cto's re-ruling on j30's Q1,
/// 2026-10-20): for each build on the previous-build axis, old -> new ->
/// old -> new, the `PeerId` the same at every step.
///
/// - old -> new: the old `transportctl` restores the golden key into a
///   fresh profile and the old daemon runs on it; this build's daemon
///   then serves the same `PeerId` from what the old one left.
/// - new -> old, RECORDED: on the tree this build leaves, the old daemon
///   starts, serves the same `PeerId` and writes nothing -- and ignores
///   the endpoint overlay, which postdates it: the endpoint this build
///   disabled reads enabled under it. Every build before the profile-
///   format marker is unprotected this way (ADR-0028's amendment, the
///   ruling's B); when an entry from after the marker joins the axis,
///   it must refuse the newer tree instead, and this row changes with
///   it.
/// - old -> new again: the trip through the old build lost nothing this
///   build reads: the overlays are still in effect.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs INTERWEAVE_PREVIOUS_BUILDS; CI runs it beside the upgrade matrix"]
async fn a_rollback_to_each_previous_build_keeps_the_peer_id_and_is_recorded() {
    let manifest = Manifest::read(&previous_tree());
    for (label, old) in axis() {
        let (old_daemon, old_ctl) = (old.join("transport-daemon"), old.join("transportctl"));

        // old -> new, on a profile the old build made.
        let fresh = Home::new(PROFILE);
        std::fs::create_dir_all(fresh.paths.config_file().parent().expect("a directory"))
            .expect("a config directory");
        fresh.write_config(
            &std::fs::read_to_string(
                previous_tree().join("config/interweave/profiles/human-desktop/config.yaml"),
            )
            .expect("the tree's profile"),
        );
        let restored = package::transportctl_of(
            &old_ctl,
            &fresh,
            &[
                "identity",
                "restore",
                "--new",
                "--expected-peer-id",
                GOLDEN_PEER,
            ],
            &format!("{GOLDEN_WORDS}\n"),
        );
        assert!(
            restored.status.success(),
            "{label}'s restore: {}",
            String::from_utf8_lossy(&restored.stderr)
        );
        let mut daemon = package::start_daemon_of(&old_daemon, &fresh);
        daemon.serving(&fresh).await;
        assert_eq!(
            package::status_peer(&fresh).await.as_str(),
            GOLDEN_PEER,
            "{label}"
        );
        assert!(daemon.terminate().success(), "{label}: {}", daemon.log());
        let mut daemon = serving(&fresh).await;
        assert_eq!(
            package::status_peer(&fresh).await.as_str(),
            GOLDEN_PEER,
            "this build serves the PeerId {label} left"
        );
        assert!(daemon.terminate().success(), "{}", daemon.log());

        // new -> old, on the tree this build leaves: run here first, so
        // the old build meets the profile-format marker this build wrote.
        let home = install(&previous_tree());
        let mut daemon = serving(&home).await;
        assert!(daemon.terminate().success(), "{}", daemon.log());
        let marker = profile_format::path_for(&home.paths);
        assert_eq!(
            profile_format::read_file(&marker).expect("marked"),
            PROFILE_FORMAT
        );
        let before = snapshot(&home);
        let mut daemon = package::start_daemon_of(&old_daemon, &home);
        daemon.serving(&home).await;
        assert_eq!(
            package::status_peer(&home).await.as_str(),
            GOLDEN_PEER,
            "{label}"
        );
        assert!(
            endpoint_enabled(&home, &manifest.disabled_endpoint).await,
            "RECORDED: {label} predates the endpoint overlay and the profile-format \
             marker, so it starts on a marked tree and does not read the overlay; if it \
             now reads {} disabled, the axis gained a build that does, and this row \
             changes with it",
            manifest.disabled_endpoint
        );
        assert!(daemon.terminate().success(), "{label}: {}", daemon.log());
        assert_eq!(
            changed(&before, &snapshot(&home)),
            Vec::<String>::new(),
            "{label} wrote nothing into this build's tree"
        );

        // old -> new again: nothing this build reads was lost.
        let mut daemon = serving(&home).await;
        assert_eq!(package::status_peer(&home).await.as_str(), GOLDEN_PEER);
        assert!(!endpoint_enabled(&home, &manifest.disabled_endpoint).await);
        let allowed = allowed(&home).await;
        assert!(allowed.contains(&manifest.added), "{allowed:?}");
        assert!(!allowed.contains(&manifest.revoked), "{allowed:?}");
        assert!(daemon.terminate().success(), "{}", daemon.log());
    }
}

/// Writes the previous build's tree: run with the build of the commit it
/// is named for, by `test-data/upgrade/make-tree.sh`, never by CI. Each
/// artefact is written the way that build writes it in use: the key by
/// its `transportctl identity restore` from the golden phrase, the
/// overlays by its daemon answering its `transportctl`, the store by its
/// store crate, which is the code its window writes with.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "writes test-data/upgrade/<commit>/; make-tree.sh runs it"]
async fn write_the_previous_build_tree() {
    let out = PathBuf::from(
        std::env::var_os("INTERWEAVE_UPGRADE_TREE_OUT").expect("INTERWEAVE_UPGRADE_TREE_OUT"),
    );
    let made_by =
        std::env::var("INTERWEAVE_UPGRADE_TREE_COMMIT").expect("INTERWEAVE_UPGRADE_TREE_COMMIT");
    assert!(!out.exists(), "{} is new", out.display());
    let home = Home::new(PROFILE);

    let (kept_peer, revoked, added) = (stranger(), stranger(), stranger());
    let config = example(
        "human-desktop.yaml",
        &kept_peer,
        "/ip4/127.0.0.1/tcp/0",
        None,
    );
    let one = format!("allowed_peers: [\"{}\"]", kept_peer.as_str());
    assert!(config.contains(&one), "the example allows one peer");
    home.write_config(&config.replacen(
        &one,
        &format!(
            "allowed_peers: [\"{}\", \"{}\"]",
            kept_peer.as_str(),
            revoked.as_str()
        ),
        1,
    ));
    // After the profile: a restore reads where the profile keeps its key.
    package::transportctl_ok(
        &home,
        &[
            "identity",
            "restore",
            "--new",
            "--expected-peer-id",
            GOLDEN_PEER,
        ],
        &format!("{GOLDEN_WORDS}\n"),
    );

    let mut daemon = serving(&home).await;
    package::transportctl_ok(&home, &["trust", "allow", added.as_str()], "");
    package::transportctl_ok(&home, &["trust", "revoke", revoked.as_str()], "");
    package::transportctl_ok(&home, &["endpoints", "disable", "claude"], "");
    assert!(daemon.terminate().success(), "{}", daemon.log());

    let (pending, unread, kept) = (
        envelope(0x51, "pending"),
        envelope(0x52, "unread"),
        envelope(0x53, "kept"),
    );
    let path = app_store(&home);
    crate::common::private_dir(path.parent().expect("a store directory"));
    {
        let mut store = HumanStore::open(&path, StoreOptions::default()).expect("a store");
        let encoded = encode_outbound(&pending, MAX_PAYLOAD_BYTES).expect("it fits");
        store
            .commit_pending_outbound(&NewOutbound {
                app_message_id: AppMessageId::parse(pending.app_message_id.clone()).expect("an id"),
                transport_message_id: MessageId::from_bytes(0x51_u128.to_be_bytes()),
                destination: OutboundDestination::Direct(DirectDestination {
                    peer: kept_peer.clone(),
                    endpoint: Some(human()),
                }),
                media_type: Some(MediaType::parse(encoded.media_type).expect("a media type")),
                payload: encoded.bytes,
                created_at: wall_ms(),
            })
            .expect("pending committed");
        let inbound = |e: &interweave_human_chat_protocol::HumanChatV2| {
            let encoded = encode_outbound(e, MAX_PAYLOAD_BYTES).expect("it fits");
            NewInbound {
                app_message_id: AppMessageId::parse(e.app_message_id.clone()).expect("an id"),
                origin: InboundOrigin {
                    peer: kept_peer.clone(),
                    endpoint: Some(human()),
                    channel: None,
                },
                media_type: Some(MediaType::parse(encoded.media_type).expect("a media type")),
                payload: encoded.bytes,
                received_at: wall_ms(),
            }
        };
        store
            .commit_unread_inbound(&inbound(&unread))
            .expect("unread committed");
        let row = store
            .commit_unread_inbound(&inbound(&kept))
            .expect("committed");
        let held = store.mark_read(row, wall_ms()).expect("read");
        store.keep(&held, wall_ms()).expect("kept");
    }
    checkpoint(&path);

    for root in ROOTS {
        let from = home.root.path().join(root);
        if from.exists() {
            copy_tree(&from, &out.join(root));
        }
    }
    let manifest = Manifest {
        made_by,
        store_schema: store_version(&home),
        peer: GOLDEN_PEER.to_owned(),
        configured: vec![kept_peer.as_str().to_owned(), revoked.as_str().to_owned()],
        added: added.as_str().to_owned(),
        revoked: revoked.as_str().to_owned(),
        disabled_endpoint: "claude".to_owned(),
        pending: pending.app_message_id,
        unread: unread.app_message_id,
        kept: kept.app_message_id,
    };
    manifest.write(&out.join("tree.json"));
}

/// `from` copied under `to`, leaving out what is never part of a tree:
/// the lock files, and a WAL or shared-memory file a checkpoint emptied.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a directory");
    for entry in std::fs::read_dir(from).expect("a directory") {
        let entry = entry.expect("an entry");
        let name = entry.file_name();
        let name = name.to_str().expect("utf-8");
        let target = to.join(name);
        if entry.file_type().expect("a type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else if !(Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lock"))
            || name.ends_with("-shm")
            || name.ends_with("-wal"))
        {
            std::fs::copy(entry.path(), &target).expect("copied");
        }
    }
}
