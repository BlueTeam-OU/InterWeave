// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The profile-format marker (ADR-0028 A 2026-10-20) over a real
//! provisioned profile: absent reads as 0, a raise writes this build's
//! format owner-only and never lowers it, a marker above this build's
//! refuses `ProfileConfig::load` -- naming the file and both numbers,
//! before `config.yaml` is read, with nothing written -- and a marker
//! that is not private, a link or not its one shape is refused, never
//! read as 0.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(target_os = "linux")]

use std::collections::BTreeMap;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use interweave_profile_config::profile_format::{
    MAX_PROFILE_FORMAT_BYTES, PROFILE_FORMAT, PROFILE_FORMAT_FILE, ProfileFormatError,
    check_within, path_for, raise, raise_within, read_within, require_supported,
};
use interweave_profile_config::provision::provision_embedded;
use interweave_profile_config::{
    LoadError, ProfileConfig, ProfileLock, ProfilePaths, TrustBoundary, persist,
};

/// A provisioned embedded profile under an owner-only app data directory.
fn profile() -> (tempfile::TempDir, ProfilePaths) {
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
    provision_embedded(&paths).expect("provisioned");
    (root, paths)
}

/// Put `text` where the marker lives, mode `mode`, as something other
/// than the marker's own writer would.
fn plant(paths: &ProfilePaths, text: &[u8], mode: u32) {
    let path = path_for(paths);
    persist::create_private_dir_within(paths.state_dir(), paths.boundary()).expect("state dir");
    persist::write_private_atomic_within(&path, text, paths.boundary()).expect("planted");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

fn marker(n: u64) -> Vec<u8> {
    format!("{{\"profile_format\":{n}}}").into_bytes()
}

/// Every file and directory under `root` with its bytes, mode and
/// modification time: what "nothing written" is checked against.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, (Vec<u8>, u32, i64, i64)> {
    let mut seen = BTreeMap::new();
    let mut stack = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("readdir") {
            let path = entry.expect("entry").path();
            let meta = std::fs::symlink_metadata(&path).expect("metadata");
            let bytes = if meta.is_file() {
                std::fs::read(&path).expect("read")
            } else {
                Vec::new()
            };
            if meta.is_dir() {
                stack.push(path.clone());
            }
            seen.insert(path, (bytes, meta.mode(), meta.mtime(), meta.mtime_nsec()));
        }
    }
    seen
}

#[test]
fn an_absent_marker_reads_as_zero_and_every_build_loads() {
    let (_root, paths) = profile();
    let path = path_for(&paths);
    assert_eq!(
        path.file_name().and_then(|n| n.to_str()),
        Some(PROFILE_FORMAT_FILE)
    );
    assert!(
        path.starts_with(paths.state_dir()),
        "in the state directory"
    );
    assert!(
        !paths.state_dir().exists(),
        "provisioning made no state dir"
    );

    assert_eq!(read_within(&path, paths.boundary()).expect("reads"), 0);
    assert_eq!(require_supported(&paths).expect("supported"), 0);
    ProfileConfig::load(&paths).expect("an unmarked profile loads");
    assert!(!paths.state_dir().exists(), "and the check created nothing");

    // With the state directory present and no marker in it: still 0.
    persist::create_private_dir_within(paths.state_dir(), paths.boundary()).expect("state dir");
    assert_eq!(read_within(&path, paths.boundary()).expect("reads"), 0);
    assert!(!path.exists(), "a read writes nothing");
}

#[test]
fn a_raise_writes_this_builds_format_owner_only_and_only_when_below() {
    let (_root, paths) = profile();
    let _lock = ProfileLock::acquire(&paths, std::time::Duration::ZERO).expect("the lock");
    let path = path_for(&paths);

    raise(&paths).expect("raised from absent");
    let meta = std::fs::symlink_metadata(&path).expect("written");
    assert!(meta.file_type().is_file(), "a regular file");
    assert_eq!(meta.mode() & 0o777, 0o600, "owner-only");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(&path).expect("read"))
            .expect("json"),
        serde_json::json!({ "profile_format": PROFILE_FORMAT }),
        "exactly the one shape"
    );
    assert_eq!(
        read_within(&path, paths.boundary()).expect("reads"),
        PROFILE_FORMAT
    );
    ProfileConfig::load(&paths).expect("this build loads what it marked");

    // At its own format a raise leaves the file alone: same inode, same
    // bytes, no rename.
    let before = snapshot(paths.state_dir());
    let inode = meta.ino();
    raise(&paths).expect("already raised");
    assert_eq!(snapshot(paths.state_dir()), before, "nothing rewritten");
    assert_eq!(
        std::fs::symlink_metadata(&path).expect("present").ino(),
        inode
    );

    // A build whose format is above the marker's raises it to its own.
    raise_within(&path, paths.boundary(), PROFILE_FORMAT + 2).expect("raised");
    assert_eq!(
        read_within(&path, paths.boundary()).expect("reads"),
        PROFILE_FORMAT + 2
    );
    assert_eq!(
        std::fs::symlink_metadata(&path).expect("present").mode() & 0o777,
        0o600
    );
}

#[test]
fn a_raise_never_lowers_the_marker() {
    let (_root, paths) = profile();
    let path = path_for(&paths);
    plant(&paths, &marker(u64::from(PROFILE_FORMAT) + 1), 0o600);
    let before = snapshot(paths.state_dir());

    let raised = raise(&paths);
    match raised {
        Err(ProfileFormatError::Newer {
            path: p,
            found,
            own,
        }) => {
            assert_eq!(p, path);
            assert_eq!(found, PROFILE_FORMAT + 1);
            assert_eq!(own, PROFILE_FORMAT);
        }
        other => panic!("a lower build's raise is refused: {other:?}"),
    }
    assert_eq!(snapshot(paths.state_dir()), before, "and writes nothing");
    assert_eq!(
        read_within(&path, paths.boundary()).expect("reads"),
        PROFILE_FORMAT + 1,
        "never lowered"
    );

    // THE CONTROL: the same marker is accepted by a build at its format,
    // and raised by one above it.
    assert_eq!(
        check_within(&path, paths.boundary(), PROFILE_FORMAT + 1).expect("its own"),
        PROFILE_FORMAT + 1
    );
    raise_within(&path, paths.boundary(), PROFILE_FORMAT + 2).expect("raised");
    assert_eq!(
        read_within(&path, paths.boundary()).expect("reads"),
        PROFILE_FORMAT + 2
    );
}

#[test]
fn a_marker_above_this_build_refuses_the_load_and_writes_nothing() {
    let (root, paths) = profile();
    let path = path_for(&paths);

    // THE CONTROL: a marker at this build's format loads.
    plant(&paths, &marker(u64::from(PROFILE_FORMAT)), 0o600);
    ProfileConfig::load(&paths).expect("this build's format loads");

    plant(&paths, &marker(u64::from(PROFILE_FORMAT) + 1), 0o600);
    let before = snapshot(root.path());
    let loaded = ProfileConfig::load(&paths);
    let Err(LoadError::ProfileFormat(ProfileFormatError::Newer {
        path: p,
        found,
        own,
    })) = &loaded
    else {
        panic!("a newer build's profile is refused: {loaded:?}");
    };
    assert_eq!(p, &path);
    assert_eq!((*found, *own), (PROFILE_FORMAT + 1, PROFILE_FORMAT));
    let said = loaded
        .as_ref()
        .err()
        .map(ToString::to_string)
        .unwrap_or_default();
    for part in [
        path.display().to_string(),
        format!("format {}", PROFILE_FORMAT + 1),
        format!("this build's {PROFILE_FORMAT}"),
        "a newer build".to_owned(),
    ] {
        assert!(said.contains(&part), "names {part:?}: {said}");
    }
    assert_eq!(snapshot(root.path()), before, "nothing written");

    // Judged BEFORE config.yaml: with the document gone the refusal is
    // still the format's, not a read failure -- so an identity init on a
    // profile with no configuration yet is refused too.
    std::fs::remove_file(paths.config_file()).expect("rm config.yaml");
    assert!(matches!(
        ProfileConfig::load(&paths),
        Err(LoadError::ProfileFormat(ProfileFormatError::Newer { .. }))
    ));
    // The control: without the marker the same profile is a read failure.
    std::fs::remove_file(&path).expect("rm marker");
    assert!(matches!(
        ProfileConfig::load(&paths),
        Err(LoadError::Read(_))
    ));
}

#[test]
fn a_marker_that_is_not_its_one_shape_is_refused_never_read_as_zero() {
    let (_root, paths) = profile();
    let path = path_for(&paths);
    // THE CONTROL: the one shape, planted by hand, reads -- 0 included.
    plant(&paths, &marker(0), 0o600);
    assert_eq!(read_within(&path, paths.boundary()).expect("reads"), 0);
    plant(&paths, b"{\n  \"profile_format\": 1\n}", 0o600);
    assert_eq!(read_within(&path, paths.boundary()).expect("reads"), 1);

    for text in [
        &b"{}"[..],
        br#"{"profile_format":"1"}"#,
        br#"{"profile_format":-1}"#,
        br#"{"profile_format":1.5}"#,
        br#"{"profile_format":4294967296}"#,
        br#"{"profile_format":null}"#,
        br#"{"profile_format":1,"build":"x"}"#,
        br"not json",
        br"",
        // A derived struct also deserialises from a sequence.
        br"[1]",
        // A key named twice is not the one shape.
        br#"{"profile_format":1,"profile_format":1}"#,
    ] {
        plant(&paths, text, 0o600);
        let read = read_within(&path, paths.boundary());
        assert!(
            matches!(read, Err(ProfileFormatError::Parse(_))),
            "{}: {read:?}",
            String::from_utf8_lossy(text)
        );
        assert!(
            matches!(
                ProfileConfig::load(&paths),
                Err(LoadError::ProfileFormat(ProfileFormatError::Parse(_)))
            ),
            "the load refuses it: {}",
            String::from_utf8_lossy(text)
        );
        assert!(
            raise(&paths).is_err(),
            "and a raise does not overwrite it: {}",
            String::from_utf8_lossy(text)
        );
        assert_eq!(std::fs::read(&path).expect("read"), text, "left as found");
    }

    let padded = format!(
        "{{\"profile_format\":1}}{}",
        " ".repeat(usize::try_from(MAX_PROFILE_FORMAT_BYTES).expect("small"))
    );
    plant(&paths, padded.as_bytes(), 0o600);
    let read = read_within(&path, paths.boundary());
    assert!(
        matches!(read, Err(ProfileFormatError::TooLarge)),
        "{read:?}"
    );
}

#[test]
fn a_marker_that_is_not_private_or_is_a_link_is_refused() {
    let (root, paths) = profile();
    let path = path_for(&paths);
    // THE CONTROL: owner-only, read.
    plant(&paths, &marker(1), 0o600);
    assert_eq!(read_within(&path, paths.boundary()).expect("reads"), 1);

    for mode in [0o640, 0o604, 0o620, 0o602, 0o644] {
        plant(&paths, &marker(1), mode);
        let read = read_within(&path, paths.boundary());
        assert!(
            matches!(read, Err(ProfileFormatError::NotPrivate { .. })),
            "{mode:04o}: {read:?}"
        );
        assert!(
            matches!(
                ProfileConfig::load(&paths),
                Err(LoadError::ProfileFormat(
                    ProfileFormatError::NotPrivate { .. }
                ))
            ),
            "{mode:04o}: the load refuses it"
        );
        assert!(raise(&paths).is_err(), "{mode:04o}: and a raise");
        assert_eq!(
            std::fs::symlink_metadata(&path).expect("present").mode() & 0o777,
            mode,
            "left as found"
        );
    }

    // A link to an owner-only marker naming this build's format: refused,
    // not followed.
    std::fs::remove_file(&path).expect("rm");
    let target = root.path().join("elsewhere.json");
    std::fs::write(&target, marker(1)).expect("target");
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    std::os::unix::fs::symlink(&target, &path).expect("link");
    let read = read_within(&path, paths.boundary());
    assert!(
        matches!(read, Err(ProfileFormatError::NotPrivate { .. })),
        "{read:?}"
    );
    assert!(matches!(
        ProfileConfig::load(&paths),
        Err(LoadError::ProfileFormat(
            ProfileFormatError::NotPrivate { .. }
        ))
    ));
    assert!(raise(&paths).is_err(), "a raise does not replace the link");
    assert!(
        std::fs::symlink_metadata(&path)
            .expect("present")
            .file_type()
            .is_symlink(),
        "left as found"
    );

    // A directory in its place.
    std::fs::remove_file(&path).expect("rm link");
    std::fs::create_dir(&path).expect("mkdir");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).expect("chmod");
    let read = read_within(&path, paths.boundary());
    assert!(
        matches!(read, Err(ProfileFormatError::NotPrivate { .. })),
        "{read:?}"
    );
}

/// A state directory other accounts can read is the profile lock's to
/// refuse, in the class it has always had, and not a marker error:
/// absent, the load passes and the lock refuses the directory; a newer
/// build's marker in it still refuses the load, and an untrusted marker
/// is refused as the file's; the raise, which writes, judges the
/// directory and refuses, writing nothing.
#[test]
fn a_marker_in_a_state_directory_others_can_read_is_still_judged() {
    let (_root, paths) = profile();
    let path = path_for(&paths);
    persist::create_private_dir_within(paths.state_dir(), paths.boundary()).expect("state dir");
    std::fs::set_permissions(paths.state_dir(), std::fs::Permissions::from_mode(0o755))
        .expect("chmod");

    // No marker: the load passes, and the lock refuses the directory as
    // it did before the marker existed.
    ProfileConfig::load(&paths).expect("no marker: the directory is not the load's to judge");
    let locked = ProfileLock::acquire(&paths, std::time::Duration::ZERO);
    assert!(
        matches!(
            locked,
            Err(interweave_profile_config::PersistError::DirectoryNotPrivate { .. })
        ),
        "the lock refuses it: {locked:?}"
    );

    // A newer build's marker: refused wherever it lies.
    std::fs::write(&path, marker(u64::from(PROFILE_FORMAT) + 1)).expect("planted");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    assert!(matches!(
        ProfileConfig::load(&paths),
        Err(LoadError::ProfileFormat(ProfileFormatError::Newer { .. }))
    ));

    // An untrusted marker: refused as the file's, naming the file's mode.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("chmod");
    match ProfileConfig::load(&paths) {
        Err(LoadError::ProfileFormat(ProfileFormatError::NotPrivate { detail })) => {
            assert!(detail.contains("mode is 0644"), "{detail}");
        }
        other => panic!("refused as the file's: {other:?}"),
    }

    // The control for the load: this build's marker passes.
    std::fs::write(&path, marker(u64::from(PROFILE_FORMAT))).expect("planted");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    ProfileConfig::load(&paths).expect("this build's format loads");

    // The raise judges the directory and writes nothing.
    std::fs::write(&path, marker(0)).expect("planted");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    let raised = raise(&paths);
    assert!(
        matches!(raised, Err(ProfileFormatError::NotPrivate { .. })),
        "{raised:?}"
    );
    assert_eq!(
        std::fs::read(&path).expect("read"),
        marker(0),
        "left as found"
    );
}

/// A state directory this uid cannot search, or a file where it should
/// be, is not the marker's to report: the marker check reads no marker,
/// the load refuses it by its older rule and the lock refuses it. THE
/// CONTROL beside it: a marker this uid cannot open, in a directory it
/// can, is the file's error, never read as absent.
#[test]
fn a_state_directory_the_load_cannot_search_is_left_to_the_lock() {
    let (_root, paths) = profile();
    let path = path_for(&paths);

    // A regular file where the state directory should be: the open fails
    // (ENOTDIR) and the directory is refused, so the marker check reads
    // no marker -- the load refuses it by its own older rule (the human
    // directory under it cannot be resolved), never as a marker error,
    // and the lock refuses it too.
    persist::create_private_dir_within(
        paths.state_dir().parent().expect("a parent"),
        paths.boundary(),
    )
    .expect("the state directory's parent");
    std::fs::write(paths.state_dir(), b"").expect("a file in its place");
    let loaded = ProfileConfig::load(&paths);
    assert!(
        matches!(loaded, Err(LoadError::KeyFileUnresolved { .. })),
        "refused by the older rule, not the marker's: {loaded:?}"
    );
    assert!(
        ProfileLock::acquire(&paths, std::time::Duration::ZERO).is_err(),
        "the lock refuses it"
    );
    std::fs::remove_file(paths.state_dir()).expect("rm");

    // A state directory this uid cannot search: the same. (Skipped where
    // the mode refuses nothing, as for root.)
    persist::create_private_dir_within(paths.state_dir(), paths.boundary()).expect("state dir");
    std::fs::write(&path, marker(u64::from(PROFILE_FORMAT) + 1)).expect("planted");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    std::fs::set_permissions(paths.state_dir(), std::fs::Permissions::from_mode(0o000))
        .expect("chmod");
    let blocked = std::fs::metadata(&path).is_err();
    let loaded = ProfileConfig::load(&paths);
    let locked = ProfileLock::acquire(&paths, std::time::Duration::ZERO).map(drop);
    std::fs::set_permissions(paths.state_dir(), std::fs::Permissions::from_mode(0o700))
        .expect("chmod");
    if blocked {
        // Not the marker's error though a newer marker lies there: the
        // older rule refuses it as above, and so does the lock.
        assert!(
            matches!(loaded, Err(LoadError::KeyFileUnresolved { .. })),
            "refused by the older rule, not the marker's: {loaded:?}"
        );
        assert!(locked.is_err(), "the lock refuses it");
    } else {
        eprintln!("the mode blocked nothing here (root?): not reached");
    }

    // THE CONTROL: a marker of this uid's it cannot open, in a private
    // directory, is refused as the file's, never read as absent.
    std::fs::write(&path, marker(u64::from(PROFILE_FORMAT))).expect("planted");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    let unreadable = std::fs::read(&path).is_err();
    let loaded = ProfileConfig::load(&paths);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    if unreadable {
        assert!(
            matches!(
                loaded,
                Err(LoadError::ProfileFormat(ProfileFormatError::Read(_)))
            ),
            "the file's error: {loaded:?}"
        );
    }
    ProfileConfig::load(&paths).expect("readable again, it loads");
}
