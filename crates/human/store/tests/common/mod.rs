// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//
// What every test binary of this crate shares.
#![allow(dead_code, clippy::expect_used)]

use std::path::Path;

/// A temporary directory made owner-only AT CREATION, whatever the umask.
///
/// `tempfile` creates with the process umask, so under umask 002 with a
/// shared primary group a bare `tempfile::tempdir()` is group-writable,
/// and the store's ancestor rule (ADR-0028 A 2026-10-08) refuses it: the
/// test fails at its setup, not at what it tests. Under umask 022 a bare
/// tempdir is 0755, which the rule accepts, so nothing else would notice
/// the mode going missing: the helper checks its own result, and every
/// test that uses it fails if it is not 0700.
pub(crate) fn private_tempdir() -> tempfile::TempDir {
    private_tempdir_in(std::env::temp_dir())
}

/// [`private_tempdir`] under `dir`.
pub(crate) fn private_tempdir_in(dir: impl AsRef<Path>) -> tempfile::TempDir {
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    let temp = builder.tempdir_in(dir).expect("tempdir");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(temp.path())
            .expect("stat")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "a private tempdir is owner-only");
    }
    temp
}

/// What v12 added (the same-name rule's shape keys: STATE.md's v12
/// paragraph), undone on a file this build made: the indexes before the
/// columns they cover, and the keys' version row back to version 1's. A
/// macro, so [`UNDO_V10`] begins with the same text.
macro_rules! undo_v12 {
    () => {
        "DROP INDEX contacts_by_shape_key;
     DROP INDEX contacts_by_compat_shape_key;
     ALTER TABLE contacts DROP COLUMN shape_key;
     ALTER TABLE contacts DROP COLUMN compat_shape_key;
     UPDATE settings SET value = '1' WHERE key = 'name_key_version';
     "
    };
}

/// [`undo_v12!`] as a string.
pub(crate) const UNDO_V12: &str = undo_v12!();

/// What v10 added (the start window: STATE.md's v10 paragraph), undone on
/// a file this build made, after v12's, whose index and columns sit on
/// v10's table: a test that fakes an earlier schema runs it first, the
/// indexes before the columns they cover. v11 rebuilt an index v10 drops
/// here, so it has no undo of its own.
pub(crate) const UNDO_V10: &str = concat!(
    undo_v12!(),
    "DROP INDEX contacts_by_name_key;
     DROP INDEX contact_routes_by_route;
     DROP INDEX contact_routes_by_key;
     DROP INDEX unread_inbound_by_conversation;
     DROP INDEX unread_inbound_by_channel;
     DROP INDEX kept_inbound_by_conversation;
     DROP INDEX kept_inbound_by_channel;
     DROP INDEX unread_inbound_by_kind;
     DROP INDEX kept_inbound_by_kind;
     ALTER TABLE contacts DROP COLUMN name_key;
     ALTER TABLE unread_inbound DROP COLUMN kind;
     ALTER TABLE kept_inbound DROP COLUMN kind;
     DELETE FROM settings WHERE key = 'name_key_version';
     "
);
