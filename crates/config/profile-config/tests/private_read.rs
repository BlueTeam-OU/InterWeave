// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The public private-file reader, as a crate outside profile-config
//! calls it for a state file of its own: an owner-only file in an
//! owner-only directory under the boundary is read, and every other
//! shape is refused rather than read as absent, each refusal beside the
//! read it differs from by one property. No refusal's text carries what
//! the file holds.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(target_os = "linux")]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use interweave_profile_config::{
    PrivateReadError, TrustBoundary, create_private_dir_within, read_private_within,
    write_private_atomic_within,
};

const KEY: &str = "c2VjcmV0LWtleS1tYXRlcmlhbC1mb3ItdGhlLXRlc3Q=\n";
const BOUND: u64 = 256;

/// An owner-only boundary with an owner-only `keys` directory in it, and
/// the path of a file there.
fn setup() -> (tempfile::TempDir, TrustBoundary, PathBuf) {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let boundary = TrustBoundary::new(root.path()).expect("a boundary");
    let dir = root.path().join("keys");
    create_private_dir_within(&dir, &boundary).expect("an owner-only directory");
    let path = dir.join("one.key");
    (root, boundary, path)
}

fn write(path: &Path, boundary: &TrustBoundary, text: &str) {
    write_private_atomic_within(path, text.as_bytes(), boundary).expect("written owner-only");
}

fn chmod(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

fn refused(path: &Path, boundary: &TrustBoundary) -> PrivateReadError {
    match read_private_within(path, boundary, BOUND) {
        Err(e) => {
            assert!(
                !e.to_string().contains(KEY.trim()),
                "the refusal's text carries no content: {e}"
            );
            e
        }
        Ok(read) => panic!("refused, but read {read:?}"),
    }
}

#[test]
fn an_owner_only_file_is_read_and_an_absent_one_is_none() {
    let (_root, boundary, path) = setup();
    assert_eq!(
        read_private_within(&path, &boundary, BOUND).expect("absent reads"),
        None
    );
    assert_eq!(
        read_private_within(
            &path.parent().expect("dir").join("none/x.key"),
            &boundary,
            BOUND
        )
        .expect("an absent directory reads"),
        None
    );
    write(&path, &boundary, KEY);
    assert_eq!(
        read_private_within(&path, &boundary, BOUND).expect("reads"),
        Some(KEY.to_owned())
    );
}

#[test]
fn a_file_others_can_read_is_refused() {
    let (_root, boundary, path) = setup();
    write(&path, &boundary, KEY);
    chmod(&path, 0o640);
    assert!(matches!(
        refused(&path, &boundary),
        PrivateReadError::NotPrivate { .. }
    ));
    chmod(&path, 0o600);
    assert!(
        read_private_within(&path, &boundary, BOUND).is_ok(),
        "the control reads"
    );
}

#[test]
fn a_directory_others_can_enter_is_refused() {
    let (_root, boundary, path) = setup();
    write(&path, &boundary, KEY);
    let dir = path.parent().expect("dir");
    chmod(dir, 0o755);
    assert!(matches!(
        refused(&path, &boundary),
        PrivateReadError::NotPrivate { .. }
    ));
    chmod(dir, 0o700);
    assert!(
        read_private_within(&path, &boundary, BOUND).is_ok(),
        "the control reads"
    );
}

#[test]
fn a_link_to_an_owner_only_file_is_refused() {
    let (_root, boundary, path) = setup();
    let real = path.with_file_name("real.key");
    write(&real, &boundary, KEY);
    std::os::unix::fs::symlink(&real, &path).expect("a link");
    assert!(matches!(
        refused(&path, &boundary),
        PrivateReadError::NotPrivate { .. }
    ));
    assert!(
        read_private_within(&real, &boundary, BOUND).is_ok(),
        "the control reads"
    );
}

#[test]
fn a_file_over_the_bound_is_refused() {
    let (_root, boundary, path) = setup();
    let over = KEY.repeat(usize::try_from(BOUND).expect("small") / KEY.len() + 1);
    write(&path, &boundary, &over);
    assert!(matches!(
        refused(&path, &boundary),
        PrivateReadError::TooLarge
    ));
    let at = &over[..usize::try_from(BOUND).expect("small")];
    write(&path, &boundary, at);
    assert_eq!(
        read_private_within(&path, &boundary, BOUND).expect("at the bound reads"),
        Some(at.to_owned())
    );
}
