// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A store an earlier build wrote, through every migration to this
//! build's: the Stage 19 upgrade tree's (`test-data/upgrade/d3a193e9`), at
//! schema 7, never one this build made and changed back. Its messages are
//! of a conversation no accepted card names, so schema v14 discards them
//! and counts each (STATE.md §Migrations (c), (e)); the texts that build
//! encoded, naming its sender's label in `from_endpoint`, are still text
//! when this build classifies them as stored rows (j68 P2-1, schema v14).

#![allow(clippy::expect_used, clippy::panic)]

use interweave_human_store::{
    AppMessageId, DiscardedAtV14, HumanStore, InboundKind, InboundOrigin, NewInbound, StoreOptions,
};
use interweave_transport_api::{MediaType, TransportIdentity};

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

/// The tree's unread and kept messages, as that build stored them: the
/// source peer, the media type and the payload.
fn tree_messages(path: &std::path::Path) -> Vec<(String, String, Vec<u8>)> {
    let conn = rusqlite::Connection::open(path).expect("the tree's store");
    let mut out = Vec::new();
    for table in ["unread_inbound", "kept_inbound"] {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT source_peer, media_type, payload FROM {table} ORDER BY row_id"
            ))
            .expect("prepare");
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .expect("rows");
        out.extend(rows.map(|r| r.expect("a row")));
    }
    out
}

/// The tree's messages reach schema v14 and are discarded and counted;
/// their bytes, committed again as rows this build holds, are classified
/// text by this build. Mutations: the classifier answering `other` for a
/// message; the stored read refusing a label `from_endpoint`.
#[test]
fn an_older_builds_messages_are_counted_at_v14_and_their_texts_stay_text() {
    let (_dir, path) = older_store();
    let messages = tree_messages(&path);
    assert_eq!(
        messages.len(),
        2,
        "control: the tree's unread and kept rows"
    );
    assert!(
        messages.iter().all(|(_, _, payload)| {
            std::str::from_utf8(payload).is_ok_and(|t| t.contains(r#""from_endpoint":"human""#))
        }),
        "control: each names its sender's label"
    );

    let mut store = HumanStore::open(
        &path,
        &interweave_test_support::endpoint_keys::endpoint_id("human"),
        StoreOptions::default(),
    )
    .expect("migrates");
    assert_eq!(
        store.discarded_at_v14().expect("read"),
        Some(DiscardedAtV14 {
            conversations: 1,
            unread: 1,
            kept: 1,
            pending_never_attempted: 1,
            pending_may_have_reached: 0,
            contacts: 0,
            contacts_combined: 0,
            card_proposals: 0,
        })
    );
    assert!(store.unread_inbound().expect("unread").is_empty());
    assert!(store.kept_inbound().expect("kept").is_empty());

    let sender = interweave_test_support::endpoint_keys::endpoint_id("sender");
    for (n, (peer, media_type, payload)) in messages.into_iter().enumerate() {
        store
            .commit_unread_inbound(&NewInbound {
                app_message_id: AppMessageId::parse(format!("{:032x}", n + 1)).expect("an id"),
                origin: InboundOrigin {
                    peer: TransportIdentity::parse(peer).expect("the tree's peer"),
                    endpoint: Some(sender.clone()),
                    channel: None,
                },
                media_type: Some(MediaType::parse(media_type).expect("the tree's media type")),
                payload,
                received_at: 1,
                kind: InboundKind::Other,
            })
            .expect("the tree's bytes, held again");
    }
    store
        .reclassify_inbound("this build", interweave_human_app_core::stored_kind)
        .expect("classified");
    let kinds: Vec<_> = store
        .unread_inbound()
        .expect("unread")
        .iter()
        .map(|r| r.kind)
        .collect();
    assert_eq!(kinds, [InboundKind::Text, InboundKind::Text]);
}
