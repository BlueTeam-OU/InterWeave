// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The bootstrap-peers overlay on disk (ADR-0062, ADR-0028 A 2026-10-21
//! (ii)): the two moves and their refusals, the normalisation at load,
//! and every reason a present overlay stops the start -- each beside the
//! control that the same file, made right, loads.

#![allow(clippy::expect_used, clippy::panic)]

use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use interweave_profile_config::TrustBoundary;
use interweave_profile_config::bootstrap_overlay::{
    BOOTSTRAP_OVERLAY_FILE, BootstrapOverlay, BootstrapOverlayError, MoveRefusal,
};

const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

/// A distinct valid entry for each `port`.
fn entry(port: u16) -> String {
    format!("/ip4/192.168.1.20/tcp/{port}/p2p/{PEER}")
}

fn state() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::Builder::new()
        .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700))
        .tempdir()
        .expect("a private temporary directory");
    let path = dir.path().join(BOOTSTRAP_OVERLAY_FILE);
    (dir, path)
}

fn put(path: &Path, text: &str) {
    std::fs::write(path, text).expect("written");
    chmod(path, 0o600);
}

fn chmod(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

fn load(path: &Path, authored: &[String]) -> Result<BootstrapOverlay, BootstrapOverlayError> {
    BootstrapOverlay::load_within(path, authored, &TrustBoundary::root())
}

fn file(peers: &[String]) -> String {
    serde_json::json!({ "peers": peers }).to_string()
}

#[test]
fn an_absent_overlay_is_empty_and_is_not_created() {
    let (_dir, path) = state();
    assert!(load(&path, &[entry(1)]).expect("loads").peers().is_empty());
    assert!(!path.exists(), "nothing created");
}

#[test]
fn an_add_round_trips_owner_only_and_composes_after_the_authored() {
    let (_dir, path) = state();
    let authored = vec![entry(1)];
    let added = BootstrapOverlay::default()
        .add(&authored, &entry(2))
        .expect("admitted")
        .expect("a change");
    added
        .write_within(&path, &TrustBoundary::root())
        .expect("written");
    assert_eq!(
        std::fs::metadata(&path).expect("meta").mode() & 0o777,
        0o600
    );
    let back = load(&path, &authored).expect("loads");
    assert_eq!(back.peers(), [entry(2)]);
    assert_eq!(back.composed(&authored), vec![entry(1), entry(2)]);
}

#[test]
fn a_duplicate_of_an_authored_entry_leaves_at_load_and_the_file_is_rewritten() {
    let (_dir, path) = state();
    put(&path, &file(&[entry(1), entry(2), entry(2)]));
    let overlay = load(&path, &[entry(1)]).expect("loads");
    assert_eq!(overlay.peers(), [entry(2)]);
    let on_disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("json");
    assert_eq!(on_disk["peers"], serde_json::json!([entry(2)]));
}

#[test]
fn a_present_overlay_that_cannot_be_trusted_stops_the_load_by_name() {
    let (_dir, path) = state();
    // The control: the same file, made right, loads.
    put(&path, &file(&[entry(2)]));
    load(&path, &[]).expect("the control loads");

    put(&path, "{ not json");
    assert!(matches!(
        load(&path, &[]),
        Err(BootstrapOverlayError::Parse(_))
    ));

    put(&path, r#"{"peers": [], "trusted": true}"#);
    assert!(matches!(
        load(&path, &[]),
        Err(BootstrapOverlayError::Parse(_))
    ));

    put(
        &path,
        &file(&[entry(2), "/ip4/192.168.1.20/tcp/1".to_owned()]),
    );
    let err = load(&path, &[]).expect_err("an entry outside the grammar");
    assert!(
        matches!(err, BootstrapOverlayError::InvalidEntry { index: 1, .. }),
        "{err:?}"
    );
    assert!(err.to_string().contains("entry 1"), "{err}");

    put(&path, &file(&[entry(2)]));
    chmod(&path, 0o644);
    assert!(matches!(
        load(&path, &[]),
        Err(BootstrapOverlayError::NotPrivate { .. })
    ));
}

#[test]
fn a_composed_count_past_the_bound_stops_the_load_and_the_bound_itself_loads() {
    let (_dir, path) = state();
    let authored: Vec<String> = (1..=32).map(entry).collect();
    let added: Vec<String> = (101..=132).map(entry).collect();
    put(&path, &file(&added));
    assert_eq!(load(&path, &authored).expect("64 loads").peers().len(), 32);

    let more: Vec<String> = (101..=133).map(entry).collect();
    put(&path, &file(&more));
    let err = load(&path, &authored).expect_err("65 refused");
    assert!(
        matches!(err, BootstrapOverlayError::ComposedTooLarge { got: 65 }),
        "{err:?}"
    );
    assert!(err.to_string().contains("bootstrap overlay"), "{err}");
}

#[test]
fn an_add_is_refused_by_name_or_changes_nothing_when_present() {
    let authored = vec![entry(1)];
    let overlay = BootstrapOverlay::default()
        .add(&authored, &entry(2))
        .expect("admitted")
        .expect("a change");
    assert!(matches!(
        overlay.add(&authored, "not an address"),
        Err(MoveRefusal::Grammar(_))
    ));
    assert_eq!(overlay.add(&authored, &entry(1)), Ok(None), "authored");
    assert_eq!(overlay.add(&authored, &entry(2)), Ok(None), "added");

    let full: Vec<String> = (1..=63).map(entry).collect();
    let at_bound = BootstrapOverlay::default()
        .add(&full, &entry(500))
        .expect("the 64th admitted")
        .expect("a change");
    assert_eq!(at_bound.add(&full, &entry(501)), Err(MoveRefusal::Bound));
}

#[test]
fn a_removal_of_an_authored_entry_is_refused_and_of_an_absent_one_changes_nothing() {
    let authored = vec![entry(1)];
    let overlay = BootstrapOverlay::default()
        .add(&authored, &entry(2))
        .expect("admitted")
        .expect("a change");
    assert_eq!(
        overlay.remove(&authored, &entry(1)),
        Err(MoveRefusal::Authored)
    );
    assert_eq!(overlay.remove(&authored, &entry(3)), Ok(None));
    let removed = overlay
        .remove(&authored, &entry(2))
        .expect("removable")
        .expect("a change");
    assert!(removed.peers().is_empty());
}

/// AN ENTRY THE CONFIGURATION REFUSES IS REFUSED HERE, by name, before
/// anything is written (#367 review F3): a DNS name the grammar takes
/// whose address half passes `MAX_ADDRESS_BYTES`, refused at the add and
/// at the load. The control: the same shape within the bound is taken.
#[test]
fn an_entry_the_configuration_refuses_is_refused_here() {
    let name = |labels: usize| {
        let mut parts = vec!["a".repeat(63); labels];
        parts.push("b".repeat(61));
        parts.join(".")
    };
    let long = format!("/dns4/{}/tcp/4001/p2p/{PEER}", name(3));
    let fits = format!("/dns4/{}/tcp/4001/p2p/{PEER}", name(1));
    assert!(
        interweave_profile_config::split_peer_multiaddr(&long).is_ok(),
        "the grammar takes it"
    );

    assert!(matches!(
        BootstrapOverlay::default().add(&[], &long),
        Err(MoveRefusal::Grammar(_))
    ));
    assert!(
        BootstrapOverlay::default()
            .add(&[], &fits)
            .expect("the control")
            .is_some()
    );

    let (_dir, path) = state();
    put(&path, &file(&[fits.clone(), long]));
    let err = load(&path, &[]).expect_err("an oversize entry on disk");
    assert!(
        matches!(err, BootstrapOverlayError::InvalidEntry { index: 1, .. }),
        "{err:?}"
    );
    put(&path, &file(&[fits]));
    load(&path, &[]).expect("the control loads");
}
