// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The store at rest (ADR-0060 decision 6 with A 2026-10-21; plan §29
//! step 3 and exit gate (b)): encrypted under its key, the
//! plaintext-to-encrypted rewrite killed after each step and resumed with
//! no row lost, the open's order for a file that is not plaintext, and
//! Discard and recreate with its two scopes.
//!
//! A process is killed with the `crash-points` feature this crate's tests
//! enable: the test binary runs itself as a child that ends after the
//! named step, as a kill would, with no destructor run.
//!
//! What a byte search shows: in an encrypted file it finds no marker,
//! which on its own proves nothing; every case that searches also runs the
//! control (the seeded plaintext holds the marker).

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;
#[path = "common/keystore.rs"]
mod keystore;

use std::path::{Path, PathBuf};

use interweave_human_store::at_rest::{
    self, AtRestRecovery, CRASH_STATUS, CRASH_VARIABLE, DiscardScope, KeyPresence, RewriteStep,
    StoreFiles,
};
use interweave_human_store::key_custody::{Custody, FilesystemCustody, KeyCipher};
use interweave_human_store::store_key::{self, StoreKey};
use interweave_human_store::{
    AcceptedCard, AppMessageId, HumanStore, InboundOrigin, NewInbound, STORE_FILE, StoreError,
    StoreOptions,
};
use interweave_profile_config::TrustBoundary;
use interweave_profile_config::paths::ProfilePaths;
use interweave_profile_config::profile_format;
use interweave_transport_api::{EndpointId, TransportIdentity};
use keystore::SoftKeystore;

const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
/// TEST-ONLY markers, long and unique: a message kept, one unread, and a
/// contact's name.
const KEPT: &[u8] = b"ATREST-KEPT-MESSAGE-CONTENT-5b1d9e3a7c2f4e6a8d0b";
const UNREAD: &[u8] = b"ATREST-UNREAD-MESSAGE-CONTENT-8f2a6c4e0b9d1f3a5c7e";
const NAME: &str = "ATREST-CONTACT-NAME-3c7e1a9f5d2b8e4a6c0f";
/// Rows the seed writes: 20 unread and 5 kept messages, and one contact.
const UNREAD_ROWS: u32 = 20;
const KEPT_ROWS: u32 = 5;

/// The child's environment: where its profile is.
const CHILD_DIR: &str = "AT_REST_CHILD_DIR";

fn inbound(id: u32, payload: &[u8]) -> NewInbound {
    NewInbound {
        app_message_id: AppMessageId::parse(format!("{id:032x}")).expect("canonical id"),
        origin: InboundOrigin {
            peer: TransportIdentity::parse(PEER).expect("canonical peer"),
            endpoint: None,
            channel: None,
        },
        media_type: None,
        payload: payload.to_vec(),
        received_at: 2_000 + u64::from(id),
    }
}

fn paths_at(dir: &Path) -> ProfilePaths {
    let boundary = TrustBoundary::new(dir).expect("a boundary");
    ProfilePaths::resolve_embedded("default", boundary).expect("paths")
}

fn profile() -> (tempfile::TempDir, ProfilePaths) {
    let dir = common::private_tempdir();
    let paths = paths_at(dir.path());
    // The profile's directories as the clients make them, owner-only
    // whatever the umask: the store refuses a wider one.
    std::os::unix::fs::DirBuilderExt::mode(std::fs::DirBuilder::new().recursive(true), 0o700)
        .create(paths.human_dir())
        .expect("the human directory");
    (dir, paths)
}

fn db(paths: &ProfilePaths) -> PathBuf {
    paths.human_dir().join(STORE_FILE)
}

/// Write the seed into `store`.
fn fill(store: &mut HumanStore) {
    for id in 0..UNREAD_ROWS {
        store
            .commit_unread_inbound(&inbound(id, UNREAD))
            .expect("unread commits");
    }
    for id in 100..100 + KEPT_ROWS {
        let row = store
            .commit_unread_inbound(&inbound(id, KEPT))
            .expect("commits");
        let read = store.mark_read(row, 3_000).expect("read");
        store.keep(&read, 3_001).expect("kept");
    }
    let p = TransportIdentity::parse(PEER).unwrap();
    let human = EndpointId::parse("human").unwrap();
    store
        .accept_card(
            &AcceptedCard {
                peer: &p,
                endpoint: &human,
                name: NAME,
                surname: None,
                identity_key: [7; 32],
                photo: None,
            },
            10,
        )
        .expect("a contact");
}

/// A plaintext store as a build before ADR-0060 left it, closed cleanly.
fn plaintext_store(paths: &ProfilePaths) {
    let mut store = HumanStore::open(&db(paths), StoreOptions::default()).expect("plaintext");
    fill(&mut store);
    drop(store);
    for needle in [KEPT, NAME.as_bytes(), PEER.as_bytes()] {
        assert!(
            holds(&paths.human_dir(), needle),
            "the control: the seeded plaintext holds {} in clear",
            String::from_utf8_lossy(needle)
        );
    }
}

/// Every row the seed wrote is in `store`.
fn assert_all_rows(store: &HumanStore) {
    assert_eq!(
        store.unread_inbound().expect("unread").len(),
        UNREAD_ROWS as usize
    );
    assert_eq!(
        store.kept_inbound().expect("kept").len(),
        KEPT_ROWS as usize
    );
    let contacts = store.contacts().expect("contacts");
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].display_name, NAME);
}

/// Whether any file under `dir` holds `needle`.
fn holds(dir: &Path, needle: &[u8]) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|e| {
            std::fs::read(e.path())
                .is_ok_and(|bytes| bytes.windows(needle.len()).any(|w| w == needle))
        })
    })
}

/// No file under the human directory holds any marker or SQLite's
/// plaintext header.
fn assert_no_plaintext(paths: &ProfilePaths) {
    let dir = paths.human_dir();
    for needle in [
        KEPT,
        UNREAD,
        NAME.as_bytes(),
        // The route: the PeerId the contact's card came from.
        PEER.as_bytes(),
        b"SQLite format 3\0".as_slice(),
    ] {
        assert!(
            !holds(&dir, needle),
            "{} in clear under {}",
            String::from_utf8_lossy(needle),
            dir.display()
        );
    }
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Every file under `dir` with its bytes.
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    names_in(dir)
        .into_iter()
        .map(|n| {
            let bytes = std::fs::read(dir.join(&n)).unwrap_or_default();
            (n, bytes)
        })
        .collect()
}

// ----- the child process a crash point ends -----

/// Runs only as the child [`crash_after`] starts: opens the profile, which
/// rewrites it, and is ended by the crash point.
#[test]
fn child_opens_the_profile() {
    let Some(dir) = std::env::var_os(CHILD_DIR) else {
        return;
    };
    let paths = paths_at(Path::new(&dir));
    let _ = HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default());
    panic!("the crash point was not reached");
}

/// Run the rewrite of `paths`' profile in a child ended after `step`.
fn crash_after(dir: &Path, step: RewriteStep) {
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["child_opens_the_profile", "--exact", "--test-threads=1"])
        .env(CHILD_DIR, dir)
        .env(CRASH_VARIABLE, format!("{step:?}"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the child runs");
    assert_eq!(status.code(), Some(CRASH_STATUS), "ended at {step:?}");
}

const STEPS: [RewriteStep; 8] = [
    RewriteStep::MarkerRaised,
    RewriteStep::KeyWritten,
    RewriteStep::Exported,
    RewriteStep::Marked,
    RewriteStep::ScrubDeleted,
    RewriteStep::Scrubbed,
    RewriteStep::Unlinked,
    RewriteStep::Renamed,
];

// ----- a new store and the rewrite -----

#[test]
fn a_new_store_is_encrypted_from_its_first_page_under_a_new_key() {
    let (_dir, paths) = profile();
    let mut store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    fill(&mut store);
    drop(store);
    assert!(store_key::exists(&paths).unwrap());
    assert_no_plaintext(&paths);
    let store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    assert_all_rows(&store);
}

#[test]
fn a_plaintext_store_is_rewritten_encrypted_at_its_first_open_with_every_row() {
    let (_dir, paths) = profile();
    plaintext_store(&paths);
    let store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    assert_all_rows(&store);
    drop(store);
    assert_no_plaintext(&paths);
    assert_eq!(names_in(&paths.human_dir()), [STORE_FILE]);
    // The marker was raised, so a build from before it refuses the profile
    // by name rather than meeting the encrypted file.
    assert_eq!(
        profile_format::read_within(&profile_format::path_for(&paths), paths.boundary()).unwrap(),
        profile_format::PROFILE_FORMAT
    );
}

#[test]
fn the_rewrite_killed_after_each_step_loses_no_row_at_the_next_open() {
    for step in STEPS {
        let (dir, paths) = profile();
        plaintext_store(&paths);
        crash_after(dir.path(), step);
        let store = HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default())
            .unwrap_or_else(|e| panic!("after {step:?}: {e}"));
        assert_all_rows(&store);
        drop(store);
        assert_eq!(names_in(&paths.human_dir()), [STORE_FILE], "after {step:?}");
        assert_no_plaintext(&paths);
    }
}

#[test]
fn a_crash_leaves_the_files_by_name_as_decision_6_lists_them() {
    let expect = [
        (RewriteStep::MarkerRaised, StoreFiles::Plaintext),
        (RewriteStep::KeyWritten, StoreFiles::Plaintext),
        (RewriteStep::Exported, StoreFiles::ExportUnfinished),
        (RewriteStep::Marked, StoreFiles::ScrubUnfinished),
        (RewriteStep::ScrubDeleted, StoreFiles::ScrubUnfinished),
        (RewriteStep::Scrubbed, StoreFiles::ScrubUnfinished),
        (RewriteStep::Unlinked, StoreFiles::RenameUnfinished),
        (RewriteStep::Renamed, StoreFiles::Encrypted),
    ];
    for (step, files) in expect {
        let (dir, paths) = profile();
        plaintext_store(&paths);
        crash_after(dir.path(), step);
        assert_eq!(
            at_rest::inspect(&paths, &FilesystemCustody).unwrap().files,
            files,
            "after {step:?}"
        );
    }
}

#[test]
fn a_store_whose_every_session_ended_before_a_checkpoint_is_rewritten_too() {
    let (_dir, paths) = profile();
    let mut store = HumanStore::open(&db(&paths), StoreOptions::default()).unwrap();
    fill(&mut store);
    // Killed: no close, no checkpoint; the content is in the log alone.
    std::mem::forget(store);
    assert_eq!(
        at_rest::inspect(&paths, &FilesystemCustody).unwrap().files,
        StoreFiles::Plaintext
    );
    let store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    assert_all_rows(&store);
    drop(store);
    assert_no_plaintext(&paths);
}

#[test]
fn no_plaintext_after_an_unclean_close_of_the_encrypted_store() {
    let (_dir, paths) = profile();
    let mut store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    fill(&mut store);
    std::mem::forget(store);
    assert!(
        std::fs::metadata(paths.human_dir().join(format!("{STORE_FILE}-wal")))
            .is_ok_and(|m| m.len() > 0),
        "the setup: the content is in the log"
    );
    assert_no_plaintext(&paths);
    let store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    assert_all_rows(&store);
}

// ----- a file that is not plaintext: the open's order -----

#[test]
fn a_store_whose_key_is_missing_opens_nothing_and_reaches_recovery_mode() {
    let (_dir, paths) = profile();
    let mut store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    fill(&mut store);
    drop(store);
    std::fs::remove_file(store_key::key_path(&paths)).unwrap();
    let before = snapshot(&paths.human_dir());
    let err =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap_err();
    assert!(
        matches!(err, StoreError::AtRest(AtRestRecovery::KeyMissing)),
        "{err:?}"
    );
    assert!(err.needs_recovery());
    assert_eq!(snapshot(&paths.human_dir()), before, "nothing written");
    assert!(
        !store_key::exists(&paths).unwrap(),
        "no key made in its place"
    );
}

#[test]
fn a_key_that_does_not_open_the_store_reaches_recovery_mode() {
    let (_dir, paths) = profile();
    drop(HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap());
    store_key::remove(&paths, &FilesystemCustody).unwrap();
    store_key::create(&paths, &FilesystemCustody).unwrap();
    let before = snapshot(&paths.human_dir());
    let err =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap_err();
    assert!(err.needs_recovery(), "{err:?}");
    assert_eq!(snapshot(&paths.human_dir()), before);
}

#[test]
fn an_invalidated_key_is_its_own_condition_not_recovery_mode() {
    let (_dir, paths) = profile();
    let keystore = SoftKeystore::default();
    drop(HumanStore::open_profile(&paths, &keystore, StoreOptions::default()).unwrap());
    keystore.invalidate(&store_key::alias("default"));
    let err = HumanStore::open_profile(&paths, &keystore, StoreOptions::default()).unwrap_err();
    assert!(matches!(err, StoreError::KeyInvalidated), "{err:?}");
    assert!(!err.needs_recovery());
    assert_eq!(
        at_rest::inspect(&paths, &keystore).unwrap().key,
        KeyPresence::Invalidated
    );
}

#[test]
fn a_plaintext_store_with_a_damaged_header_reaches_recovery_mode_not_discard() {
    let (_dir, paths) = profile();
    plaintext_store(&paths);
    let path = db(&paths);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[..16].copy_from_slice(&[0xA5; 16]);
    std::fs::write(&path, &bytes).unwrap();
    let before = snapshot(&paths.human_dir());
    let err =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap_err();
    assert!(err.needs_recovery(), "{err:?}");
    assert_eq!(
        snapshot(&paths.human_dir()),
        before,
        "nothing removed or rewritten"
    );
}

#[test]
fn names_decision_6_does_not_list_reach_recovery_mode() {
    let (_dir, paths) = profile();
    drop(HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap());
    // A complete copy beside a store that is not plaintext.
    std::fs::copy(db(&paths), paths.human_dir().join(at_rest::ENCRYPTED)).unwrap();
    let err =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap_err();
    assert!(
        matches!(err, StoreError::AtRest(AtRestRecovery::UnlistedFiles(_))),
        "{err:?}"
    );
}

#[test]
fn a_copy_that_does_not_open_leaves_the_plaintext_beside_it_unscrubbed() {
    let (dir, paths) = profile();
    plaintext_store(&paths);
    crash_after(dir.path(), RewriteStep::Marked);
    // The key that sealed the copy is lost and another is in its place.
    store_key::remove(&paths, &FilesystemCustody).unwrap();
    store_key::create(&paths, &FilesystemCustody).unwrap();
    let err =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap_err();
    assert!(
        matches!(err, StoreError::AtRest(AtRestRecovery::CopyUnopenable)),
        "{err:?}"
    );
    assert!(holds(&paths.human_dir(), KEPT), "the plaintext is intact");
}

// ----- Discard and recreate -----

#[test]
fn a_discard_removes_everything_and_the_key_and_the_next_open_recreates_an_empty_store() {
    let (_dir, paths) = profile();
    let mut store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    fill(&mut store);
    drop(store);
    std::fs::remove_file(store_key::key_path(&paths)).unwrap();
    let plan = at_rest::plan_discard(&paths, &FilesystemCustody).unwrap();
    assert_eq!(plan.scope(), DiscardScope::Everything);
    assert!(plan.removes_key());
    assert!(plan.files().contains(&db(&paths)));
    at_rest::discard(&paths, &FilesystemCustody, plan.confirm()).unwrap();
    assert!(names_in(&paths.human_dir()).is_empty());
    let store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    assert!(store.unread_inbound().unwrap().is_empty());
    assert!(store.contacts().unwrap().is_empty());
    assert!(store_key::exists(&paths).unwrap(), "a new key");
}

#[test]
fn a_discard_leaves_every_identity_key_and_the_profiles_other_state() {
    let (_dir, paths) = profile();
    drop(HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap());
    let human = EndpointId::parse("human").unwrap();
    interweave_human_store::identity_key::create(&paths, &human, &FilesystemCustody).unwrap();
    let identity = std::fs::read(interweave_human_store::identity_key::key_path(
        &paths, &human,
    ))
    .unwrap();
    let marker = std::fs::read(profile_format::path_for(&paths)).unwrap();
    let plan = at_rest::plan_discard(&paths, &FilesystemCustody).unwrap();
    at_rest::discard(&paths, &FilesystemCustody, plan.confirm()).unwrap();
    assert_eq!(
        std::fs::read(interweave_human_store::identity_key::key_path(
            &paths, &human
        ))
        .unwrap(),
        identity
    );
    assert_eq!(
        std::fs::read(profile_format::path_for(&paths)).unwrap(),
        marker
    );
}

#[test]
fn a_discard_during_a_stalled_rewrite_removes_the_plaintext_and_keeps_the_key_and_the_copy() {
    let (dir, paths) = profile();
    plaintext_store(&paths);
    crash_after(dir.path(), RewriteStep::Marked);
    let plan = at_rest::plan_discard(&paths, &FilesystemCustody).unwrap();
    assert_eq!(plan.scope(), DiscardScope::UnfinishedPlaintext);
    assert!(!plan.removes_key());
    assert!(
        !plan
            .files()
            .contains(&paths.human_dir().join(at_rest::ENCRYPTED))
    );
    at_rest::discard(&paths, &FilesystemCustody, plan.confirm()).unwrap();
    assert_eq!(names_in(&paths.human_dir()), [at_rest::ENCRYPTED]);
    assert!(store_key::exists(&paths).unwrap());
    // The next open promotes the complete copy: nothing the person had is
    // lost.
    let store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    assert_all_rows(&store);
}

/// The four ways decision 6 names a copy that cannot be opened: each makes
/// the one confirmed discard remove the plaintext, the copy and the key.
#[test]
fn a_discard_with_the_copy_unopenable_removes_plaintext_copy_and_key_in_one_step() {
    type Spoil = fn(&ProfilePaths, &SoftKeystore);
    let cases: [(&str, Spoil); 4] = [
        ("no key", |p, k| {
            store_key::remove(p, k).unwrap();
        }),
        ("an invalidated key", |_, k| {
            k.invalidate(&store_key::alias("default"));
        }),
        ("a key that does not open it", |p, k| {
            store_key::remove(p, k).unwrap();
            store_key::create(p, k).unwrap();
        }),
        ("a damaged copy", |p, _| {
            let copy = p.human_dir().join(at_rest::ENCRYPTED);
            let mut bytes = std::fs::read(&copy).unwrap();
            let middle = bytes.len() / 2;
            bytes[middle..middle + 64].fill(0);
            bytes[..64].fill(0);
            std::fs::write(&copy, bytes).unwrap();
        }),
    ];
    for (what, spoil) in cases {
        let (dir, paths) = profile();
        // The stalled rewrite under the soft Keystore: the child runs on
        // the desktop's custody, so the copy is re-sealed here first.
        plaintext_store(&paths);
        let keystore = SoftKeystore::default();
        crash_after(dir.path(), RewriteStep::Marked);
        let key = store_key::open(&paths, &FilesystemCustody)
            .unwrap()
            .unwrap();
        reseal(&paths, &key, &keystore);
        assert_eq!(
            at_rest::plan_discard(&paths, &keystore).unwrap().scope(),
            DiscardScope::UnfinishedPlaintext,
            "the control, {what}: the copy opens before it is spoiled"
        );
        spoil(&paths, &keystore);
        let plan = at_rest::plan_discard(&paths, &keystore).unwrap();
        assert_eq!(plan.scope(), DiscardScope::Everything, "{what}");
        assert!(plan.removes_key(), "{what}");
        at_rest::discard(&paths, &keystore, plan.confirm()).unwrap();
        assert!(names_in(&paths.human_dir()).is_empty(), "{what}");
        assert!(!store_key::exists(&paths).unwrap(), "{what}");
    }
}

/// Write `key` into `paths`' key file sealed by `cipher` instead.
fn reseal(paths: &ProfilePaths, key: &StoreKey, cipher: &dyn KeyCipher) {
    let line = key.to_record("default", cipher).unwrap();
    std::fs::remove_file(store_key::key_path(paths)).unwrap();
    interweave_profile_config::persist::create_private_exclusive_within(
        &store_key::key_path(paths),
        line.as_bytes(),
        paths.boundary(),
    )
    .unwrap();
}

#[test]
fn a_confirmed_discard_whose_store_changed_since_removes_nothing() {
    let (dir, paths) = profile();
    plaintext_store(&paths);
    crash_after(dir.path(), RewriteStep::Marked);
    // Confirmed while the copy opened; then the key is lost.
    let plan = at_rest::plan_discard(&paths, &FilesystemCustody).unwrap();
    store_key::remove(&paths, &FilesystemCustody).unwrap();
    let before = snapshot(&paths.human_dir());
    assert!(matches!(
        at_rest::discard(&paths, &FilesystemCustody, plan.confirm()),
        Err(StoreError::DiscardChanged)
    ));
    assert_eq!(snapshot(&paths.human_dir()), before);
}

#[test]
fn the_health_report_names_the_protection_the_key_and_the_files() {
    let (_dir, paths) = profile();
    let report = at_rest::inspect(&paths, &FilesystemCustody).unwrap();
    assert_eq!(report.protection, Custody::FilesystemOnly);
    assert_eq!(report.key, KeyPresence::Absent);
    assert_eq!(report.files, StoreFiles::None);
    let keystore = SoftKeystore::default();
    drop(HumanStore::open_profile(&paths, &keystore, StoreOptions::default()).unwrap());
    let report = at_rest::inspect(&paths, &keystore).unwrap();
    assert_eq!(report.protection, Custody::Keystore);
    assert_eq!(report.key, KeyPresence::Present);
    assert_eq!(report.files, StoreFiles::Encrypted);
    assert!(
        !format!("{report:?}").contains("x'"),
        "no key literal in a report"
    );
}

#[test]
fn a_plaintext_store_from_a_newer_build_is_refused_with_every_file_as_it_was() {
    let (_dir, paths) = profile();
    plaintext_store(&paths);
    {
        let conn = rusqlite::Connection::open(db(&paths)).unwrap();
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .unwrap();
        conn.pragma_update(
            None,
            "user_version",
            interweave_human_store::SCHEMA_VERSION + 1,
        )
        .unwrap();
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .unwrap();
    }
    let state = paths.state_dir();
    let before = (snapshot(&paths.human_dir()), snapshot(state));
    let err =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap_err();
    assert!(err.needs_recovery(), "{err:?}");
    assert_eq!(
        (snapshot(&paths.human_dir()), snapshot(state)),
        before,
        "no key, no marker, no copy: nothing written"
    );
}

/// A store an earlier build wrote: the Stage 19 upgrade tree's, at
/// schema 7 (`test-data/upgrade/d3a193e9/tree.json`), one pending, one
/// unread and one kept message.
fn older_store(paths: &ProfilePaths) {
    use std::os::unix::fs::PermissionsExt as _;
    let from = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../../test-data/upgrade/d3a193e9/state/interweave/profiles/human-desktop/human/human.sqlite",
    );
    let to = db(paths);
    std::fs::copy(&from, &to).expect("the upgrade tree's store");
    std::fs::set_permissions(&to, std::fs::Permissions::from_mode(0o600)).unwrap();
    let version: i64 = rusqlite::Connection::open(&to)
        .unwrap()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert!(
        version < interweave_human_store::SCHEMA_VERSION,
        "the control: the tree's store is older than this build ({version})"
    );
}

/// The rewrite of a store an older build wrote, killed after each step
/// that leaves a complete copy beside the plaintext: the open migrates
/// the plaintext before it exports it, so the copy is at this build's
/// schema, opens under its key, and the next open promotes it with every
/// row, and a discard planned meanwhile removes
/// the unfinished plaintext alone (ADR-0060 decision 6; review F1 of #377).
#[test]
fn an_older_stores_rewrite_killed_beside_its_copy_loses_no_row_and_keeps_the_copy() {
    for step in [
        RewriteStep::Marked,
        RewriteStep::ScrubDeleted,
        RewriteStep::Scrubbed,
    ] {
        let (dir, paths) = profile();
        older_store(&paths);
        crash_after(dir.path(), step);
        assert_eq!(
            at_rest::plan_discard(&paths, &FilesystemCustody)
                .unwrap()
                .scope(),
            DiscardScope::UnfinishedPlaintext,
            "after {step:?}: the complete copy opens and is kept"
        );
        let store = HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default())
            .unwrap_or_else(|e| panic!("after {step:?}: {e}"));
        assert_eq!(store.pending_outbound().unwrap().len(), 1, "after {step:?}");
        assert_eq!(store.unread_inbound().unwrap().len(), 1, "after {step:?}");
        assert_eq!(store.kept_inbound().unwrap().len(), 1, "after {step:?}");
        drop(store);
        assert_eq!(names_in(&paths.human_dir()), [STORE_FILE], "after {step:?}");
    }
}

/// A stalled rewrite an EARLIER build left: its complete copy at that
/// build's schema (the v7 tree's store exported under the key, as that
/// build's step (a) would have), the plaintext beside it. The copy opens
/// under its key, so it is kept by a planned discard, and the open
/// promotes and migrates it with every row: the copy is judged by whether
/// it opens, never by this build's shape (#377 review F1).
#[test]
fn a_copy_an_earlier_build_left_at_its_schema_is_kept_and_migrated() {
    use base64::Engine as _;
    let (_dir, paths) = profile();
    older_store(&paths);
    store_key::create(&paths, &FilesystemCustody).expect("a key");
    let line = std::fs::read_to_string(store_key::key_path(&paths)).unwrap();
    let record = base64::engine::general_purpose::STANDARD
        .decode(line.trim_end())
        .unwrap();
    let hex: String = record[6..].iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    });
    let copy = paths.human_dir().join(at_rest::ENCRYPTED);
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&copy)
            .unwrap();
        let plain = rusqlite::Connection::open(db(&paths)).unwrap();
        plain
            .execute_batch(&format!(
                "ATTACH DATABASE '{}' AS c KEY \"x'{hex}'\";
                 SELECT sqlcipher_export('c');
                 PRAGMA c.user_version = 7;
                 DETACH DATABASE c;",
                copy.display()
            ))
            .unwrap();
    }
    assert_eq!(
        at_rest::inspect(&paths, &FilesystemCustody).unwrap().files,
        StoreFiles::ScrubUnfinished,
        "the setup: a complete copy beside the plaintext"
    );
    assert_eq!(
        at_rest::plan_discard(&paths, &FilesystemCustody)
            .unwrap()
            .scope(),
        DiscardScope::UnfinishedPlaintext
    );
    let store =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap();
    assert_eq!(store.pending_outbound().unwrap().len(), 1);
    assert_eq!(store.unread_inbound().unwrap().len(), 1);
    assert_eq!(store.kept_inbound().unwrap().len(), 1);
    drop(store);
    assert_eq!(names_in(&paths.human_dir()), [STORE_FILE]);
    assert_no_plaintext_header(&paths);
}

fn assert_no_plaintext_header(paths: &ProfilePaths) {
    assert!(!holds(&paths.human_dir(), b"SQLite format 3\0"));
}

/// A newer build's plaintext store whose version is only in its
/// write-ahead log -- the header still older -- is refused before the
/// marker, a key or a copy is written: the plaintext's own migration
/// reads the log and refuses, ahead of the rewrite (#377 review, risk 1).
#[test]
fn a_newer_plaintext_known_only_from_its_log_is_refused_before_any_write() {
    let (_dir, paths) = profile();
    std::mem::forget(HumanStore::open(&db(&paths), StoreOptions::default()).unwrap());
    let raw = rusqlite::Connection::open(db(&paths)).unwrap();
    raw.pragma_update(
        None,
        "user_version",
        interweave_human_store::SCHEMA_VERSION + 1,
    )
    .unwrap();
    // Killed: the version stays in the log, the header older.
    std::mem::forget(raw);
    assert_eq!(
        at_rest::inspect(&paths, &FilesystemCustody).unwrap().files,
        StoreFiles::Plaintext,
        "the setup: a plaintext store"
    );
    let marker = profile_format::path_for(&paths);
    let err =
        HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default()).unwrap_err();
    assert!(err.needs_recovery(), "{err:?}");
    assert!(!marker.exists(), "no marker raised");
    assert!(!store_key::exists(&paths).unwrap(), "no key made");
    let names = names_in(&paths.human_dir());
    assert!(
        !names.iter().any(|n| n.contains(".encrypt")),
        "no copy written: {names:?}"
    );
}
