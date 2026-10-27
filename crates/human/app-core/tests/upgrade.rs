// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A store an earlier build wrote, through schema v10 (j68 P2-1): its rows
//! are migrated as `other` with no classification version, and this
//! build's classification of the stored rows makes its messages text, so
//! the badge counts them after an upgrade. The store is the Stage 19
//! upgrade tree's (`test-data/upgrade/d3a193e9`), at schema 7, never one
//! this build made and changed back.

#![allow(clippy::expect_used, clippy::panic)]

use interweave_human_store::{HumanStore, InboundKind, StoreOptions};

/// The tree's store, copied to a private directory of its own.
fn older_store() -> (tempfile::TempDir, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().expect("a directory");
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).expect("private");
    let from = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../../test-data/upgrade/d3a193e9/state/interweave/profiles/human-desktop/human/human.sqlite",
    );
    let to = dir.path().join("human.sqlite3");
    std::fs::copy(&from, &to).expect("the upgrade tree's store");
    std::fs::set_permissions(&to, std::fs::Permissions::from_mode(0o600)).expect("private");
    (dir, to)
}

/// The tree's unread and kept messages come through the v10 migration as
/// `other`, unclassified, and this build's classification makes both
/// text. Mutation: the classifier answering `other` for a message.
#[test]
fn an_older_builds_messages_are_text_once_this_build_classifies_them() {
    let (_dir, path) = older_store();
    let mut store = HumanStore::open(&path, StoreOptions::default()).expect("migrates");
    assert_eq!(store.inbound_kind_version().expect("version"), None);
    let unread = store.unread_inbound().expect("unread");
    let kept = store.kept_inbound().expect("kept");
    assert_eq!(
        (unread.len(), kept.len()),
        (1, 1),
        "control: the tree's rows"
    );
    assert!(
        unread
            .iter()
            .chain(&kept)
            .all(|r| r.kind == InboundKind::Other),
        "unclassified after the migration"
    );

    store
        .reclassify_inbound("this build", interweave_human_app_core::stored_kind)
        .expect("classified");
    let kinds: Vec<_> = store
        .unread_inbound()
        .expect("unread")
        .iter()
        .chain(&store.kept_inbound().expect("kept"))
        .map(|r| r.kind)
        .collect();
    assert_eq!(kinds, [InboundKind::Text, InboundKind::Text]);
}
