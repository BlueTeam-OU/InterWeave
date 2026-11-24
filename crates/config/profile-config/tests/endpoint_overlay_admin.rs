// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The endpoint set's administrative moves on the overlay (IPC 3.0,
//! `LOCAL-IPC.md` §The endpoint set's methods): `add`, `remove`,
//! `set_advertise` and `set_alias`. Each refusal beside the control that
//! the same move minus the one fault is made; each made move is written
//! and loaded back through the loader the daemon takes.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use interweave_profile_config::endpoint_overlay::{
    AddRefusal, Added, AliasRefusal, ENDPOINT_OVERLAY_FILE, EndpointOverlay, EndpointOverlayError,
    RemoveRefusal, Removed, Renamed,
};
use interweave_profile_config::{
    ClientKind, EndpointsConfig, MAX_ENDPOINTS, ProfilePaths, TrustBoundary, XdgRoots, endpoint_key,
};
use interweave_transport_api::EndpointId;
use serde_json::{Value, json};

fn id(s: &str) -> EndpointId {
    interweave_test_support::endpoint_keys::endpoint_id(s)
}

fn key_of(name: &str) -> String {
    endpoint_key::key_file_name(&id(name))
}

/// Authored `a` (default, advertised) and `b`, aliased by name.
fn config() -> EndpointsConfig {
    serde_json::from_value(json!({
        "entries": [
            {"key": key_of("a"), "alias": "a", "enabled": true, "advertise": true},
            {"key": key_of("b"), "alias": "b", "enabled": true, "advertise": false},
        ],
        "default_direct_endpoint": "a",
    }))
    .expect("an endpoints block")
}

struct Profile {
    _dir: tempfile::TempDir,
    paths: ProfilePaths,
    overlay: PathBuf,
}

fn profile() -> Profile {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("a temporary directory");
    let roots = XdgRoots {
        config_home: dir.path().join("config"),
        data_home: dir.path().join("data"),
        state_home: dir.path().join("state"),
        cache_home: dir.path().join("cache"),
        runtime_dir: None,
    };
    let paths = ProfilePaths::resolve_offline("work", &roots).expect("paths");
    let overlay = paths.state_dir().join(ENDPOINT_OVERLAY_FILE);
    Profile {
        _dir: dir,
        paths,
        overlay,
    }
}

fn kinds(names: &[&str]) -> Vec<ClientKind> {
    names
        .iter()
        .map(|n| ClientKind::parse(*n).expect("a kind"))
        .collect()
}

/// Add a freshly minted key under `alias`, expecting `Added`.
fn add_minted(
    p: &Profile,
    overlay: &EndpointOverlay,
    alias: &str,
) -> (EndpointOverlay, EndpointId) {
    let minted = endpoint_key::mint_filesystem(&p.paths).expect("minted");
    match overlay.add(
        &config(),
        &p.paths,
        &endpoint_key::key_file_name(&minted),
        alias,
        kinds(&["human"]),
    ) {
        Added::Added { overlay, endpoint } => {
            assert_eq!(endpoint, minted);
            (overlay, endpoint)
        }
        other => panic!("{other:?}"),
    }
}

fn refusal(added: &Added) -> Option<AddRefusal> {
    match added {
        Added::Refused { reason, .. } => Some(*reason),
        _ => None,
    }
}

fn file_json(p: &Profile) -> Value {
    serde_json::from_str(&std::fs::read_to_string(&p.overlay).expect("read")).expect("json")
}

/// The add writes the entry into `added` with no trust member of its own,
/// advertise false and enabled, and its id into both set members with
/// `[]`; the default stays; the file loads back to the same overlay and
/// composes a leasable, unadvertised entry.
#[test]
fn an_add_writes_the_entry_and_its_two_empty_sets() {
    let p = profile();
    let (overlay, endpoint) = add_minted(&p, &EndpointOverlay::default(), "work");
    overlay
        .write_within(&p.overlay, &TrustBoundary::root())
        .expect("written");
    let file = file_json(&p);
    let entry = &file["added"][0];
    assert_eq!(entry["key"], json!(endpoint_key::key_file_name(&endpoint)));
    assert_eq!(entry["alias"], json!("work"));
    assert_eq!(entry["enabled"], json!(true));
    assert_eq!(entry["advertise"], json!(false));
    assert!(entry.get("inbound").is_none() && entry.get("outbound").is_none());
    assert_eq!(file["inbound"][endpoint.as_str()], json!([]));
    assert_eq!(file["outbound"][endpoint.as_str()], json!([]));
    assert!(file.get("default").is_none(), "the default is left: {file}");
    let (mut back, changes) =
        EndpointOverlay::load_within(&p.overlay, &config(), &TrustBoundary::root()).expect("loads");
    assert!(changes.is_empty(), "{changes:?}");
    assert!(back.read_added_keys(&p.paths).is_empty());
    assert_eq!(back, overlay);
    let composed = back.apply(&config());
    assert_eq!(composed.default_direct_endpoint, Some(id("a")));
    assert!(
        back.own_sets(&endpoint)
            .is_some_and(|(i, o)| i.is_empty() && o.is_empty())
    );
}

/// Every refusal by name, in the contract's order: each case carries the
/// fault being named AND every later one, so a check moved later answers
/// another name. The control is the same add, every fault cleared.
#[test]
fn the_add_refusals_are_checked_in_order() {
    let p = profile();
    let (base, _) = add_minted(&p, &EndpointOverlay::default(), "taken");
    let minted = endpoint_key::mint_filesystem(&p.paths).expect("minted");
    let key = endpoint_key::key_file_name(&minted);
    let add = |overlay: &EndpointOverlay, key: &str, alias: &str| {
        overlay.add(&config(), &p.paths, key, alias, kinds(&["human"]))
    };
    // id_not_derived beats alias_invalid.
    assert_eq!(
        refusal(&add(&base, "not-a-key", "Bad Alias")),
        Some(AddRefusal::IdNotDerived)
    );
    assert_eq!(
        refusal(&add(&base, minted.as_str(), "ok")),
        Some(AddRefusal::IdNotDerived),
        "a bare id is not a key file's name"
    );
    // alias_invalid beats key_missing: the unminted id has no file.
    assert_eq!(
        refusal(&add(&base, &key_of("nofile"), "Bad Alias")),
        Some(AddRefusal::AliasInvalid)
    );
    // key_missing beats exists and alias_taken: `a` is authored and
    // `taken` is held, and `a`'s key was never minted.
    assert_eq!(
        refusal(&add(&base, &key_of("a"), "taken")),
        Some(AddRefusal::KeyMissing)
    );
    // key_mismatch: a file under one id whose header derives another.
    let other = endpoint_key::mint_filesystem(&p.paths).expect("minted");
    let keys = p.paths.state_dir().join("endpoint-keys");
    std::fs::copy(
        keys.join(endpoint_key::key_file_name(&other)),
        keys.join(key_of("forged")),
    )
    .expect("copied");
    assert_eq!(
        refusal(&add(&base, &key_of("forged"), "taken")),
        Some(AddRefusal::KeyMismatch)
    );
    // exists beats alias_taken: the authored id, given a key file.
    let authored = profile_with_authored_key(&p);
    assert_eq!(
        refusal(&add(&base, &authored, "taken")),
        Some(AddRefusal::Exists)
    );
    // alias_taken against an added entry and against an authored one.
    assert_eq!(
        refusal(&add(&base, &key, "taken")),
        Some(AddRefusal::AliasTaken)
    );
    assert_eq!(
        refusal(&add(&base, &key, "b")),
        Some(AddRefusal::AliasTaken)
    );
    // The control: the same key under a free alias is added.
    assert!(matches!(add(&base, &key, "free"), Added::Added { .. }));
}

/// The TEST-ONLY key `a` -- the authored entry's id -- sealed at
/// filesystem custody under its own name, so the add reaches `exists`.
fn profile_with_authored_key(p: &Profile) -> String {
    let (signing, endpoint) = interweave_test_support::endpoint_keys::endpoint_keys("a");
    let public = signing.verifying_key().to_bytes();
    let header = endpoint_key::header_bytes(
        endpoint_key::Custody::FilesystemOnly,
        endpoint.as_str().as_bytes(),
        &public,
    )
    .expect("a header");
    let line = endpoint_key::encode_record(&header, &signing.to_bytes());
    interweave_profile_config::create_private_exclusive_within(
        &endpoint_key::key_path(&p.paths, &endpoint),
        line.as_bytes(),
        p.paths.boundary(),
    )
    .expect("written");
    endpoint_key::key_file_name(&endpoint)
}

/// `unchanged`: the same id, alias and kinds (in any order) again; another
/// alias or other kinds is `exists`.
#[test]
fn a_retry_is_unchanged_and_a_different_one_exists() {
    let p = profile();
    let minted = endpoint_key::mint_filesystem(&p.paths).expect("minted");
    let key = endpoint_key::key_file_name(&minted);
    let Added::Added { overlay, .. } =
        EndpointOverlay::default().add(&config(), &p.paths, &key, "work", kinds(&["human", "cli"]))
    else {
        panic!("added")
    };
    assert_eq!(
        overlay.add(&config(), &p.paths, &key, "work", kinds(&["cli", "human"])),
        Added::Unchanged(minted.clone())
    );
    assert_eq!(
        refusal(&overlay.add(&config(), &p.paths, &key, "home", kinds(&["human", "cli"]))),
        Some(AddRefusal::Exists)
    );
    assert_eq!(
        refusal(&overlay.add(&config(), &p.paths, &key, "work", kinds(&["human"]))),
        Some(AddRefusal::Exists)
    );
    // A retry whose key file has gone is key_missing, not unchanged.
    std::fs::remove_file(endpoint_key::key_path(&p.paths, &minted)).expect("removed");
    assert_eq!(
        refusal(&overlay.add(&config(), &p.paths, &key, "work", kinds(&["human", "cli"]))),
        Some(AddRefusal::KeyMissing)
    );
}

/// Sixty-four endpoints, authored and added together, is the bound: the
/// 64th is added, the 65th refused `bound`.
#[test]
fn the_sixty_fifth_endpoint_is_refused_bound() {
    let p = profile();
    let mut overlay = EndpointOverlay::default();
    for i in 0..(MAX_ENDPOINTS - config().entries.len()) {
        overlay = add_minted(&p, &overlay, &format!("e{i}")).0;
    }
    assert_eq!(
        overlay.added().count() + config().entries.len(),
        MAX_ENDPOINTS
    );
    let minted = endpoint_key::mint_filesystem(&p.paths).expect("minted");
    assert_eq!(
        refusal(&overlay.add(
            &config(),
            &p.paths,
            &endpoint_key::key_file_name(&minted),
            "last",
            kinds(&[]),
        )),
        Some(AddRefusal::Bound)
    );
}

/// The remove drops the added entry and every member keyed by its id in
/// one overlay value, and keeps every other endpoint's; `authored` before
/// `default`; an unknown id is `Unknown`.
#[test]
fn a_remove_drops_every_member_keyed_by_the_id() {
    let p = profile();
    let (overlay, gone) = add_minted(&p, &EndpointOverlay::default(), "gone");
    let (overlay, kept) = add_minted(&p, &overlay, "kept");
    let overlay = overlay
        .set_enabled(&config(), &gone, false)
        .expect("set")
        .expect("moved");
    let overlay = overlay
        .set_advertise(&config(), &gone, true)
        .expect("set")
        .expect("moved");
    let Removed::Removed(next) = overlay.remove(&config(), &gone).expect("removed") else {
        panic!("removed")
    };
    next.write_within(&p.overlay, &TrustBoundary::root())
        .expect("written");
    let text = std::fs::read_to_string(&p.overlay).expect("read");
    assert!(!text.contains(gone.as_str()), "{text}");
    assert!(
        text.contains(kept.as_str()),
        "the other entry stays: {text}"
    );
    assert!(next.own_sets(&kept).is_some());
    assert_eq!(
        overlay.remove(&config(), &id("a")).expect("answered"),
        Removed::Refused(RemoveRefusal::Authored)
    );
    let defaulted = overlay
        .set_default(&config(), Some(&kept))
        .expect("set")
        .expect("moved");
    assert_eq!(
        defaulted.remove(&config(), &kept).expect("answered"),
        Removed::Refused(RemoveRefusal::Default)
    );
    assert!(matches!(
        overlay.remove(&config(), &kept).expect("answered"),
        Removed::Removed(_)
    ));
    assert!(matches!(
        overlay.remove(&config(), &id("nowhere")),
        Err(EndpointOverlayError::Unknown)
    ));
}

/// `set_advertise`: an authored endpoint's delta kept when it differs and
/// dropped when it equals the authored value; an added one's member set;
/// a value in effect changes nothing.
#[test]
fn set_advertise_writes_the_delta_or_the_member() {
    let p = profile();
    let base = EndpointOverlay::default();
    // `b` is authored unadvertised.
    let on = base
        .set_advertise(&config(), &id("b"), true)
        .expect("set")
        .expect("moved");
    assert!(
        on.apply(&config())
            .entries
            .iter()
            .any(|e| e.id == id("b") && e.advertise)
    );
    on.write_within(&p.overlay, &TrustBoundary::root())
        .expect("written");
    assert_eq!(file_json(&p)["advertise"][id("b").as_str()], json!(true));
    let off = on
        .set_advertise(&config(), &id("b"), false)
        .expect("set")
        .expect("moved");
    assert_eq!(off, base, "a delta equal to the authored value is not kept");
    assert_eq!(
        base.set_advertise(&config(), &id("b"), false).expect("set"),
        None
    );
    let (added, e) = add_minted(&p, &base, "work");
    let advertised = added
        .set_advertise(&config(), &e, true)
        .expect("set")
        .expect("moved");
    advertised
        .write_within(&p.overlay, &TrustBoundary::root())
        .expect("written");
    let file = file_json(&p);
    assert_eq!(file["added"][0]["advertise"], json!(true));
    assert!(file.get("advertise").is_none(), "{file}");
    assert!(matches!(
        base.set_advertise(&config(), &id("nowhere"), true),
        Err(EndpointOverlayError::Unknown)
    ));
}

/// `set_alias`: renamed, unchanged, and the refusals in order -- authored
/// before an invalid alias, invalid before taken -- with `alias_taken`
/// against an authored and an added entry; unknown is `Unknown`.
#[test]
fn set_alias_renames_an_added_entry_only() {
    let p = profile();
    let (overlay, e) = add_minted(&p, &EndpointOverlay::default(), "work");
    let (overlay, _) = add_minted(&p, &overlay, "home");
    let reason = |r: Result<Renamed, EndpointOverlayError>| match r {
        Ok(Renamed::Refused { reason, .. }) => Some(reason),
        _ => None,
    };
    assert_eq!(
        reason(overlay.set_alias(&config(), &id("a"), "Bad Alias")),
        Some(AliasRefusal::Authored)
    );
    assert_eq!(
        reason(overlay.set_alias(&config(), &e, "Bad Alias")),
        Some(AliasRefusal::AliasInvalid)
    );
    assert_eq!(
        reason(overlay.set_alias(&config(), &e, "home")),
        Some(AliasRefusal::AliasTaken)
    );
    assert_eq!(
        reason(overlay.set_alias(&config(), &e, "b")),
        Some(AliasRefusal::AliasTaken)
    );
    assert_eq!(
        overlay.set_alias(&config(), &e, "work").expect("answered"),
        Renamed::Unchanged
    );
    let Renamed::Renamed(next) = overlay
        .set_alias(&config(), &e, "office")
        .expect("answered")
    else {
        panic!("renamed")
    };
    assert!(next.added().any(|entry| {
        entry.id == e
            && entry
                .alias
                .as_ref()
                .map(interweave_profile_config::EndpointAlias::as_str)
                == Some("office")
    }));
    assert!(matches!(
        overlay.set_alias(&config(), &id("nowhere"), "x"),
        Err(EndpointOverlayError::Unknown)
    ));
}
