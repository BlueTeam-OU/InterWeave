// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What `transportctl endpoint init --alias` builds on: a key minted
//! under a name no daemon reads (`endpoint_key::mint_filesystem_at`,
//! `pending_key_path`) and published under its id by a rename that never
//! replaces (`persist::rename_private_no_replace_within`).

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(target_os = "linux")]

use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

use interweave_profile_config::endpoint_key::{
    KeyHeader, key_path, mint_filesystem, mint_filesystem_at, pending_key_path,
};
use interweave_profile_config::persist::rename_private_no_replace_within;
use interweave_profile_config::{EndpointAlias, PersistError, ProfilePaths, TrustBoundary};

fn app() -> (tempfile::TempDir, ProfilePaths) {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let app = root.path().join("app");
    std::fs::create_dir(&app).expect("mkdir");
    std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o700)).expect("chmod");
    let paths =
        ProfilePaths::resolve_embedded("work", TrustBoundary::new(&app).expect("a boundary"))
            .expect("paths");
    (root, paths)
}

fn alias() -> EndpointAlias {
    EndpointAlias::parse("claude").expect("an alias")
}

/// A key minted into the pending name is owner-only, derives the id it
/// returns, and has no id-named file; publishing it moves it to
/// `<id>.key` byte for byte and leaves no pending file. A second publish
/// over a taken name is refused and replaces nothing.
#[test]
fn a_pending_key_is_published_under_its_id_and_never_over_another() {
    let (_root, paths) = app();
    let pending = pending_key_path(&paths, &alias());
    let id = mint_filesystem_at(&paths, |_| pending.clone()).expect("minted");
    assert!(pending.exists(), "minted where asked");
    assert!(!key_path(&paths, &id).exists(), "under no id's name yet");
    assert_eq!(
        std::fs::metadata(&pending).expect("meta").mode() & 0o777,
        0o600
    );
    let line = std::fs::read_to_string(&pending).expect("read");
    assert_eq!(KeyHeader::from_line(&line).expect("header").endpoint(), &id);

    // Minting into a name that is taken is refused, the file untouched.
    let taken = mint_filesystem_at(&paths, |_| pending.clone());
    assert!(
        matches!(
            taken,
            Err(interweave_profile_config::endpoint_key::MintError::Persist(
                PersistError::AlreadyExists
            ))
        ),
        "{taken:?}"
    );
    assert_eq!(std::fs::read_to_string(&pending).expect("read"), line);

    let target = key_path(&paths, &id);
    rename_private_no_replace_within(&pending, &target, paths.boundary()).expect("published");
    assert!(!pending.exists(), "the pending name is gone");
    assert_eq!(std::fs::read_to_string(&target).expect("read"), line);

    // The control for the refusal: a taken target is kept.
    let other = mint_filesystem_at(&paths, |_| pending.clone()).expect("another");
    assert_ne!(other, id);
    let err =
        rename_private_no_replace_within(&pending, &target, paths.boundary()).expect_err("taken");
    assert!(matches!(err, PersistError::AlreadyExists), "{err:?}");
    assert_eq!(
        std::fs::read_to_string(&target).expect("read"),
        line,
        "not replaced"
    );
    assert!(pending.exists(), "the pending file is kept for a rerun");
}

/// The ordinary mint is unchanged: it writes `<id>.key` and no pending
/// file.
#[test]
fn the_ordinary_mint_writes_the_id_named_file() {
    let (_root, paths) = app();
    let id = mint_filesystem(&paths).expect("minted");
    assert!(key_path(&paths, &id).exists());
    assert!(!pending_key_path(&paths, &alias()).exists());
}
