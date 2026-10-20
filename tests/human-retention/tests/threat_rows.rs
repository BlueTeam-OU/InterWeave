// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The threat model's three retention rows (`threat-model.md`, rows TM-44,
//! TM-45 and TM-46 of `tests/SECURITY-MATRIX.md`; Stage 18's S10), each as
//! conformance of `RETENTION.md` over a real file:
//!
//! - TM-44: a remote sender cannot force persistence -- an envelope asking
//!   to be kept arrives over the facade, is unread, and is gone once read;
//! - TM-45: no shadow archive -- a trigger, a view or a search index that
//!   would hold released content is refused at open, and released content
//!   is in no table of the file;
//! - TM-46: a restored device replays no old send -- what a backup carries
//!   is all a restore has, and a client started on it sends nothing, while
//!   the original file restarted retries its pending send (the control).

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::path::{Path, PathBuf};

use interweave_human_chat_protocol::{HumanChatV2, MEDIA_TYPE_V2, MessageKind};
use interweave_human_store::{
    BackupCursor, HumanStore, NewInbound, PageLimits, StoreError, StoreOptions, StoredInbound,
};
use interweave_human_transport_client::{ClientConfig, Destination, TransportClient};
use interweave_local_client_api::{
    DataCapability, DataSessionBinding, DataSessionPort, SessionEvent, SessionRequest,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{
    DirectDestination, EndpointId, MediaType, MessageId, Payload, TransportError,
};

const LIMIT: usize = 49_152;
const WALL: u64 = 1_786_600_000_000;

fn human() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn agent() -> EndpointId {
    EndpointId::parse("agent").expect("valid")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: ProfileIdentity::generate()
            .transport_identity()
            .expect("peer id"),
        endpoints: vec![
            FakeEndpoint::open(human(), false),
            FakeEndpoint::open(agent(), true),
        ],
        default_endpoint: Some(human()),
        queue_bound: 16,
    }
}

fn config(endpoint: EndpointId) -> ClientConfig {
    ClientConfig {
        client_kind: "human-client".to_owned(),
        endpoints: vec![endpoint],
        channels: vec![],
        max_payload_bytes: LIMIT,
    }
}

fn store_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(name).join("human.sqlite3")
}

async fn session_on(node: &FakeNode, endpoint: EndpointId) -> impl DataSessionPort {
    node.open(
        SessionRequest::new(
            "holder",
            Some(endpoint),
            [DataCapability::Commands, DataCapability::Events],
        )
        .expect("bounds"),
    )
    .await
    .expect("holds the endpoint")
}

async fn directs_received(session: &impl DataSessionPort) -> usize {
    session
        .events(64)
        .await
        .expect("events")
        .iter()
        .filter(|e| matches!(e, SessionEvent::Direct(_)))
        .count()
}

/// Every value of every row of every table in the file, as bytes: what an
/// archive would have to be made of, wherever it was put.
fn every_stored_value(path: &Path) -> Vec<Vec<u8>> {
    let conn = rusqlite::Connection::open(path).expect("independent reader");
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .expect("tables");
        stmt.query_map([], |r| r.get(0))
            .expect("names")
            .map(|n| n.expect("name"))
            .collect()
    };
    let mut values = Vec::new();
    for table in tables {
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM \"{table}\""))
            .expect("select");
        let columns = stmt.column_count();
        let mut rows = stmt.query([]).expect("rows");
        while let Some(row) = rows.next().expect("row") {
            for i in 0..columns {
                match row.get_ref(i).expect("value") {
                    rusqlite::types::ValueRef::Text(t) | rusqlite::types::ValueRef::Blob(t) => {
                        values.push(t.to_vec());
                    }
                    _ => {}
                }
            }
        }
    }
    values
}

fn holds(values: &[Vec<u8>], needle: &[u8]) -> bool {
    values
        .iter()
        .any(|v| v.windows(needle.len()).any(|w| w == needle))
}

// -------------------------------------------------------------------
// TM-44
// -------------------------------------------------------------------

/// The envelope is open to unknown members by contract (the envelope
/// schema's description), so a sender CAN put `keep` in it; what it cannot
/// do is make that reach retention. Sent as raw bytes over the session,
/// not through the facade's `send`, because a hostile sender is not
/// bound to that client.
#[tokio::test]
async fn tm_44_an_envelope_asking_to_be_kept_arrives_unread_and_is_gone_once_read() {
    const BODY: &str = "keep me forever, receiver";
    let dir = common::private_tempdir();
    let path = store_path(dir.path(), "receiver");
    let (a, b) = FakeNetwork::pair(node(), node());
    let mut receiver = TransportClient::new(
        b.clone(),
        b.clone(),
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
        config(human()),
        Box::new(|| WALL),
        0,
    )
    .expect("client");
    receiver.tick(0).await;

    let hostile = format!(
        r#"{{"v":2,"kind":"text","app_message_id":"{}","text":"{BODY}","keep":true,"retain":"forever","persist":{{"until":"never"}},"kept_at":1}}"#,
        "4".repeat(32)
    );
    // The receiver's own decoder takes it: a refusal here would make the
    // rest of this case prove nothing about retention.
    HumanChatV2::parse(&hostile).expect("unknown members are ignored, not refused");
    let sender = session_on(&a, agent()).await;
    sender
        .send_direct(
            DirectDestination::to_default(b.peer().clone()),
            MessageId::from_bytes([7; 16]),
            Payload::new(
                Some(MediaType::parse(MEDIA_TYPE_V2).expect("media type")),
                hostile.into_bytes(),
                LIMIT,
            )
            .expect("fits"),
        )
        .await
        .expect("admitted to the receiver's queue");

    assert!(receiver.drain(1, 1).await.is_empty(), "the open-time state");
    let got = receiver.drain(1, 2).await;
    assert_eq!(got.len(), 1, "the message arrives");
    assert_eq!(got[0].envelope.text, BODY);
    let store = receiver.store_mut();
    assert_eq!(store.unread_inbound().expect("read").len(), 1, "as unread");
    assert!(
        store.kept_inbound().expect("read").is_empty(),
        "arriving made nothing kept"
    );

    // The person reads it and does not keep it.
    let read = store.mark_read(got[0].row, WALL + 1).expect("read");
    drop(read);
    receiver.close().await;
    drop(receiver);

    // Across a restart, the content is nowhere: not kept, not unread, and
    // in no other table of the file.
    let store = HumanStore::open(&path, StoreOptions::default()).expect("reopens");
    assert!(store.kept_inbound().expect("read").is_empty(), "not kept");
    assert!(
        store.unread_inbound().expect("read").is_empty(),
        "not unread"
    );
    drop(store);
    assert!(
        !holds(&every_stored_value(&path), BODY.as_bytes()),
        "the body survived its read somewhere in the file"
    );
}

// -------------------------------------------------------------------
// TM-45
// -------------------------------------------------------------------

/// The three shapes a shadow archive takes inside the store's own file: a
/// trigger copying a row on its way out into a table the shape guard
/// allows (`settings`, content-free metadata); a view, which is a content
/// surface of its own; and a full-text index, which keeps its own copy of
/// every word it indexed. Each is refused at open -- before any read or
/// release could feed it -- and the control shows that without one, a
/// released body is in no table at all.
#[test]
fn tm_45_a_trigger_view_or_search_index_shadowing_content_is_refused_at_open() {
    const BODY: &[u8] = b"a body that must leave with its read";
    let shadows = [
        (
            "trigger",
            "CREATE TRIGGER keep_a_copy AFTER DELETE ON unread_inbound BEGIN
                 INSERT INTO settings (key, value)
                 VALUES ('copy_' || old.row_id, CAST(old.payload AS TEXT));
             END;",
        ),
        (
            "view",
            "CREATE VIEW recent AS SELECT payload FROM unread_inbound
                 UNION ALL SELECT payload FROM kept_inbound;",
        ),
        (
            "search index",
            "CREATE VIRTUAL TABLE search USING fts5(body);
             INSERT INTO search (body) SELECT CAST(payload AS TEXT) FROM unread_inbound;",
        ),
    ];
    for (what, sql) in shadows {
        let dir = common::private_tempdir();
        let path = store_path(dir.path(), "state");
        {
            let mut store = HumanStore::open(&path, StoreOptions::default()).expect("opens");
            let mut new = interweave_human_retention_tests::unread_inbound();
            new.payload = BODY.to_vec();
            store.commit_unread_inbound(&new).expect("unread");
        }
        rusqlite::Connection::open(&path)
            .expect("raw")
            .execute_batch(sql)
            .expect("the shadow is added behind the store's back");

        let refused = HumanStore::open(&path, StoreOptions::default())
            .expect_err("a store carrying a shadow is not opened");
        assert!(
            matches!(&refused, StoreError::Migration(m) if m.contains("not part of this store's schema")),
            "{what}: {refused:?}"
        );
    }

    // THE CONTROL: no shadow, and the body leaves every table with its read.
    let dir = common::private_tempdir();
    let path = store_path(dir.path(), "state");
    let mut store = HumanStore::open(&path, StoreOptions::default()).expect("opens");
    let mut new = interweave_human_retention_tests::unread_inbound();
    new.payload = BODY.to_vec();
    let row = store.commit_unread_inbound(&new).expect("unread");
    assert!(
        holds(&every_stored_value(&path), BODY),
        "the scan sees an unread body, so its silence below means something"
    );
    drop(store.mark_read(row, WALL).expect("read"));
    drop(store);
    assert!(
        !holds(&every_stored_value(&path), BODY),
        "a read, unkept body is in no table"
    );
}

// -------------------------------------------------------------------
// TM-46
// -------------------------------------------------------------------

/// A backup is what `backup_eligible_page` walks, and a restore is what
/// can be made from it: unread rows committed again, kept rows committed,
/// read and kept again -- the store has no other way in.
fn backup_of(path: &Path) -> Vec<StoredInbound> {
    let store = HumanStore::open(path, StoreOptions::default()).expect("opens");
    let mut out = Vec::new();
    let mut after: Option<BackupCursor> = None;
    loop {
        let page = store
            .backup_eligible_page(after, PageLimits::default())
            .expect("page");
        out.extend(page.items);
        match page.next {
            Some(next) => after = Some(next),
            None => break,
        }
    }
    out
}

fn restore_into(path: &Path, backup: &[StoredInbound]) {
    let mut store = HumanStore::open(path, StoreOptions::default()).expect("a fresh device");
    for record in backup {
        let row = store
            .commit_unread_inbound(&NewInbound {
                app_message_id: record.app_message_id.clone(),
                origin: record.origin.clone(),
                media_type: record.media_type.clone(),
                payload: record.payload.clone(),
                received_at: record.received_at,
            })
            .expect("restored");
        if let (Some(read_at), Some(kept_at)) = (record.read_at, record.kept_at) {
            let held = store.mark_read(row, read_at).expect("read");
            store.keep(&held, kept_at).expect("kept again");
        }
    }
}

#[tokio::test]
async fn tm_46_a_device_restored_from_backup_sends_nothing_the_original_had_pending() {
    let dir = common::private_tempdir();
    let original = store_path(dir.path(), "original");

    // One network throughout: the pending send names `b`, and the control
    // below must be able to reach it.
    let (a, b) = FakeNetwork::pair(node(), node());

    // THE ORIGINAL: a send attempted once and still pending (its first
    // attempt met an unreachable peer), beside an unread and a kept
    // message, which a backup carries.
    {
        let mut store = HumanStore::open(&original, StoreOptions::default()).expect("store");
        let mut unread = interweave_human_retention_tests::unread_inbound();
        unread.payload = b"unread, in the backup".to_vec();
        store.commit_unread_inbound(&unread).expect("unread");
        let mut kept = interweave_human_retention_tests::unread_inbound();
        kept.app_message_id =
            interweave_human_store::AppMessageId::parse("5".repeat(32)).expect("canonical");
        kept.payload = b"kept, in the backup".to_vec();
        let row = store.commit_unread_inbound(&kept).expect("unread");
        let held = store.mark_read(row, WALL).expect("read");
        store.keep(&held, WALL + 1).expect("kept");

        let mut sender = TransportClient::new(
            a.clone(),
            a.clone(),
            store,
            config(agent()),
            Box::new(|| WALL),
            0,
        )
        .expect("client");
        sender.tick(0).await;
        a.inject_send(TransportError::PeerUnreachable);
        sender
            .send(
                Destination::Direct {
                    peer: b.peer().clone(),
                    endpoint: None,
                },
                &HumanChatV2 {
                    v: 2,
                    kind: MessageKind::Text,
                    app_message_id: "6".repeat(32),
                    text: "an old send".to_owned(),
                    reply_to: None,
                    sent_at_ms: None,
                    from_endpoint: None,
                },
                0,
            )
            .await
            .expect("committed");
        sender.tick(0).await;
        let pending = sender.store_mut().pending_outbound().expect("read");
        assert_eq!(
            pending.len(),
            1,
            "the failed attempt leaves the send pending"
        );
        // Attempted here, so the control run below is a retry of this
        // send, not its first attempt.
        assert_eq!(pending[0].attempts, 1, "attempted once");
        sender.close().await;
    }

    // THE BACKUP carries the two inbound messages and nothing of the send.
    let backup = backup_of(&original);
    assert_eq!(backup.len(), 2, "the unread and the kept message");
    assert!(
        backup
            .iter()
            .all(|r| !r.payload.windows(11).any(|w| w == b"an old send")),
        "no record of the backup carries the pending send"
    );

    // THE RESTORED DEVICE: started on the backup, it sends nothing.
    let restored = store_path(dir.path(), "restored");
    restore_into(&restored, &backup);
    let store = HumanStore::open(&restored, StoreOptions::default()).expect("restored store");
    assert!(store.pending_outbound().expect("read").is_empty());
    assert_eq!(store.unread_inbound().expect("read").len(), 1);
    assert_eq!(store.kept_inbound().expect("read").len(), 1);
    let sent = run_client_on(&a, &b, store).await;
    assert_eq!(sent, 0, "a restored device replays no old send");

    // THE CONTROL: the original file restarted retries its pending send,
    // so the count above is a live measurement and not a dead receiver.
    let store = HumanStore::open(&original, StoreOptions::default()).expect("original");
    let sent = run_client_on(&a, &b, store).await;
    assert_eq!(sent, 1, "the original retries what it had pending");
}

/// Start a sender's client on `store` at `a` while a session holds `b`'s
/// human endpoint; run it as a restart would; answer how many direct
/// messages reached that session.
async fn run_client_on(a: &FakeNode, b: &FakeNode, store: HumanStore) -> usize {
    let holder = session_on(b, human()).await;
    let mut sender = TransportClient::new(
        a.clone(),
        a.clone(),
        store,
        config(agent()),
        Box::new(|| WALL),
        0,
    )
    .expect("client");
    for now in [0, 1, 10_000, 60_000] {
        sender.tick(now).await;
    }
    sender.close().await;
    let reached = directs_received(&holder).await;
    holder.close().await.expect("closes");
    reached
}
