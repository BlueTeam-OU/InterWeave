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

/// What v14 changed (every row's local identity: STATE.md's v14
/// paragraph), undone on a PLAINTEXT file this build made: each table that
/// gained `local_endpoint_id` rebuilt in v13's shape, its rows copied
/// without it and its indexes as v13 left them, the AUTOINCREMENT
/// high-water marks kept, and the discard counts row removed. Foreign keys
/// off, as v14 runs, so dropping the contacts deletes no route. A macro,
/// so every earlier undo begins with the same text. What v14 DISCARDED is
/// not brought back: a file faked at v13 from this build holds only what
/// this build wrote.
macro_rules! undo_v14 {
    () => {
        "PRAGMA foreign_keys = OFF;
     CREATE TEMP TABLE undo_v14_seq AS SELECT name, seq FROM sqlite_sequence;
     DELETE FROM settings WHERE key = 'v14_discarded';
     CREATE TABLE pending_outbound_v13 (
         row_id                INTEGER PRIMARY KEY AUTOINCREMENT,
         app_message_id        TEXT    NOT NULL UNIQUE,
         transport_message_id  BLOB    NOT NULL UNIQUE
                               CHECK (length(transport_message_id) = 16),
         destination_peer      TEXT    NOT NULL,
         destination_endpoint  TEXT,
         channel_id            TEXT,
         media_type            TEXT,
         payload               BLOB    NOT NULL,
         created_at            INTEGER NOT NULL,
         last_attempt_at       INTEGER,
         attempts              INTEGER NOT NULL DEFAULT 0
     );
     INSERT INTO pending_outbound_v13
         SELECT row_id, app_message_id, transport_message_id, destination_peer,
                destination_endpoint, channel_id, media_type, payload, created_at,
                last_attempt_at, attempts FROM pending_outbound;
     DROP TABLE pending_outbound;
     ALTER TABLE pending_outbound_v13 RENAME TO pending_outbound;
     CREATE TABLE unread_inbound_v13 (
         row_id          INTEGER PRIMARY KEY AUTOINCREMENT,
         app_message_id  TEXT    NOT NULL,
         source_peer     TEXT    NOT NULL,
         source_endpoint TEXT,
         channel_id      TEXT,
         media_type      TEXT,
         payload         BLOB    NOT NULL,
         received_at     INTEGER NOT NULL,
         source_endpoint_key TEXT GENERATED ALWAYS AS (IFNULL(source_endpoint, '')) VIRTUAL,
         channel_key         TEXT GENERATED ALWAYS AS (IFNULL(channel_id, '')) VIRTUAL,
         kind            TEXT    NOT NULL DEFAULT 'other',
         UNIQUE(source_peer, source_endpoint_key, channel_key, app_message_id)
     );
     INSERT INTO unread_inbound_v13
         (row_id, app_message_id, source_peer, source_endpoint, channel_id, media_type,
          payload, received_at, kind)
         SELECT row_id, app_message_id, source_peer, source_endpoint, channel_id, media_type,
                payload, received_at, kind FROM unread_inbound;
     DROP TABLE unread_inbound;
     ALTER TABLE unread_inbound_v13 RENAME TO unread_inbound;
     CREATE TABLE kept_inbound_v13 (
         row_id          INTEGER PRIMARY KEY AUTOINCREMENT,
         app_message_id  TEXT    NOT NULL,
         source_peer     TEXT    NOT NULL,
         source_endpoint TEXT,
         channel_id      TEXT,
         media_type      TEXT,
         payload         BLOB    NOT NULL,
         received_at     INTEGER NOT NULL,
         read_at         INTEGER NOT NULL,
         kept_at         INTEGER NOT NULL,
         source_endpoint_key TEXT GENERATED ALWAYS AS (IFNULL(source_endpoint, '')) VIRTUAL,
         channel_key         TEXT GENERATED ALWAYS AS (IFNULL(channel_id, '')) VIRTUAL,
         kind            TEXT    NOT NULL DEFAULT 'other',
         UNIQUE(source_peer, source_endpoint_key, channel_key, app_message_id)
     );
     INSERT INTO kept_inbound_v13
         (row_id, app_message_id, source_peer, source_endpoint, channel_id, media_type,
          payload, received_at, read_at, kept_at, kind)
         SELECT row_id, app_message_id, source_peer, source_endpoint, channel_id, media_type,
                payload, received_at, read_at, kept_at, kind FROM kept_inbound;
     DROP TABLE kept_inbound;
     ALTER TABLE kept_inbound_v13 RENAME TO kept_inbound;
     CREATE TABLE read_pairs_v13 (
         pair_id         INTEGER PRIMARY KEY AUTOINCREMENT,
         app_message_id  TEXT    NOT NULL,
         source_peer     TEXT    NOT NULL,
         source_endpoint TEXT,
         channel_id      TEXT,
         at              INTEGER NOT NULL,
         source_endpoint_key TEXT GENERATED ALWAYS AS (IFNULL(source_endpoint, '')) VIRTUAL,
         channel_key         TEXT GENERATED ALWAYS AS (IFNULL(channel_id, '')) VIRTUAL,
         UNIQUE(source_peer, source_endpoint_key, channel_key, app_message_id)
     );
     INSERT INTO read_pairs_v13
         (pair_id, app_message_id, source_peer, source_endpoint, channel_id, at)
         SELECT pair_id, app_message_id, source_peer, source_endpoint, channel_id, at
           FROM read_pairs;
     DROP TABLE read_pairs;
     ALTER TABLE read_pairs_v13 RENAME TO read_pairs;
     CREATE TABLE contacts_v13 (
         contact_id    TEXT    NOT NULL PRIMARY KEY,
         display_name  TEXT    NOT NULL,
         notes         TEXT,
         created_at    INTEGER NOT NULL,
         updated_at    INTEGER NOT NULL,
         name_source   TEXT,
         name_at       INTEGER,
         photo_source  TEXT,
         photo_at      INTEGER,
         photo         BLOB,
         name_key      TEXT NOT NULL DEFAULT '',
         shape_key     TEXT NOT NULL DEFAULT '',
         compat_shape_key TEXT NOT NULL DEFAULT ''
     );
     INSERT INTO contacts_v13
         SELECT contact_id, display_name, notes, created_at, updated_at, name_source, name_at,
                photo_source, photo_at, photo, name_key, shape_key, compat_shape_key
           FROM contacts;
     CREATE TABLE contact_routes_v13 (
         contact_id         TEXT NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
         peer_id            TEXT NOT NULL,
         endpoint_id        TEXT NOT NULL,
         device_label       TEXT,
         verification_note  TEXT,
         last_seen          INTEGER,
         identity_key       BLOB,
         UNIQUE(contact_id, peer_id, endpoint_id),
         UNIQUE(contact_id, peer_id)
     );
     INSERT INTO contact_routes_v13
         SELECT contact_id, peer_id, endpoint_id, device_label, verification_note, last_seen,
                identity_key FROM contact_routes;
     DROP TABLE contact_routes;
     DROP TABLE contacts;
     ALTER TABLE contacts_v13 RENAME TO contacts;
     ALTER TABLE contact_routes_v13 RENAME TO contact_routes;
     CREATE TABLE conversation_index_v13 (
         conversation_id  TEXT    NOT NULL PRIMARY KEY,
         peer_id          TEXT    NOT NULL,
         endpoint_id      TEXT,
         channel_id       TEXT,
         title            TEXT,
         last_activity    INTEGER,
         card_shared_at   INTEGER
     );
     INSERT INTO conversation_index_v13
         SELECT conversation_id, peer_id, endpoint_id, channel_id, title, last_activity,
                card_shared_at FROM conversation_index;
     DROP TABLE conversation_index;
     ALTER TABLE conversation_index_v13 RENAME TO conversation_index;
     CREATE INDEX unread_inbound_by_received ON unread_inbound (received_at, row_id);
     CREATE INDEX kept_inbound_by_received ON kept_inbound (received_at, row_id);
     CREATE INDEX contacts_by_name_key ON contacts (name_key);
     CREATE INDEX contact_routes_by_route ON contact_routes (peer_id, endpoint_id);
     CREATE INDEX contact_routes_by_key ON contact_routes (identity_key);
     CREATE INDEX unread_inbound_by_conversation
         ON unread_inbound (source_peer, source_endpoint_key, received_at, row_id);
     CREATE INDEX unread_inbound_by_channel ON unread_inbound (channel_key, received_at, row_id);
     CREATE INDEX kept_inbound_by_conversation
         ON kept_inbound (source_peer, source_endpoint_key, received_at, row_id);
     CREATE INDEX kept_inbound_by_channel ON kept_inbound (channel_key, received_at, row_id);
     CREATE INDEX unread_inbound_by_kind
         ON unread_inbound (kind, source_peer, source_endpoint, channel_id, app_message_id);
     CREATE INDEX kept_inbound_by_kind
         ON kept_inbound (kind, source_peer, source_endpoint, channel_id);
     CREATE INDEX contacts_by_shape_key ON contacts (shape_key);
     CREATE INDEX contacts_by_compat_shape_key ON contacts (compat_shape_key);
     CREATE INDEX unread_inbound_by_message
         ON unread_inbound (app_message_id, channel_id, source_peer, source_endpoint, kind);
     CREATE INDEX kept_inbound_by_message
         ON kept_inbound (app_message_id, channel_id, source_peer, source_endpoint, kind);
     DELETE FROM sqlite_sequence;
     INSERT INTO sqlite_sequence (name, seq) SELECT name, seq FROM undo_v14_seq;
     DROP TABLE undo_v14_seq;
     PRAGMA user_version = 13;
     "
    };
}

/// [`undo_v14!`] as a string: a file this build made, back at v13.
pub(crate) const UNDO_V14: &str = undo_v14!();

/// What v13 added (a reply's target by its id: STATE.md's v13
/// paragraph), undone on a file this build made, after v14's. A macro, so
/// every earlier undo begins with the same text: a file faked at v12 or
/// before that kept v13's indexes would refuse the migration that builds
/// them.
macro_rules! undo_v13 {
    () => {
        concat!(
            undo_v14!(),
            "DROP INDEX unread_inbound_by_message;
     DROP INDEX kept_inbound_by_message;
     "
        )
    };
}

/// [`undo_v13!`] as a string.
pub(crate) const UNDO_V13: &str = undo_v13!();

/// What v12 added (the same-name rule's shape keys: STATE.md's v12
/// paragraph), undone on a file this build made, after v13's: the indexes
/// before the columns they cover, and the keys' version row back to
/// version 1's. A macro, so [`UNDO_V10`] begins with the same text.
macro_rules! undo_v12 {
    () => {
        concat!(
            undo_v13!(),
            "DROP INDEX contacts_by_shape_key;
     DROP INDEX contacts_by_compat_shape_key;
     ALTER TABLE contacts DROP COLUMN shape_key;
     ALTER TABLE contacts DROP COLUMN compat_shape_key;
     UPDATE settings SET value = '1' WHERE key = 'name_key_version';
     "
        )
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
