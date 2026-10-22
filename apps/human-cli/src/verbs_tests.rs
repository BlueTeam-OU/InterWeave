// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Each verb at the model level: the session over `app-core`'s two sides,
//! against the in-memory pair of nodes the root's own tests use, with the
//! store a real file where what the next run sees is the claim. What these
//! do not show -- the IPC binding, a real daemon, the process's exit code
//! -- is `tests/desktop-e2e/tests/human_cli.rs`'s.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::time::{Duration, Instant};

use interweave_human_app_core::FacadeSide;
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::ConversationKey;
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportError, TransportIdentity};
use serde_json::Value;

use crate::Failure;
use crate::cli::Verb;
use crate::session::Session;
use crate::verbs::{self, Run};

const WALL: u64 = 1_792_652_603_000;

fn endpoint() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: ProfileIdentity::generate()
            .transport_identity()
            .expect("peer id"),
        endpoints: vec![FakeEndpoint::open(endpoint(), false)],
        default_endpoint: Some(endpoint()),
        queue_bound: 16,
    }
}

fn private_dir() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt as _;
    let mut builder = tempfile::Builder::new();
    builder.permissions(std::fs::Permissions::from_mode(0o700));
    builder.tempdir().expect("tempdir")
}

fn store(dir: &Path) -> HumanStore {
    HumanStore::open(&dir.join("human.sqlite3"), StoreOptions::default()).expect("store")
}

fn session(node: &FakeNode, store: HumanStore) -> Session<FakeNode, FakeNode> {
    let client = TransportClient::new(
        node.clone(),
        node.clone(),
        store,
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![endpoint()],
            channels: vec![],
            max_payload_bytes: 49_152,
        },
        Box::new(|| WALL),
        0,
    )
    .expect("facade");
    Session::new(FacadeSide::new(client, || WALL), || Some(true)).expect("session")
}

fn to(peer: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: peer.clone(),
        endpoint: Some(endpoint()),
    }
}

/// Run `verb` with a short wait; what it printed, and how it ended.
async fn run(
    s: &mut Session<FakeNode, FakeNode>,
    verb: Verb,
    json: bool,
) -> (String, Result<(), Failure>) {
    let mut out = String::new();
    let r = Run {
        profile: "test",
        json,
        until: Instant::now() + Duration::from_millis(1_500),
    };
    let result = verbs::run(s, &verb, &r, &mut out).await;
    (out, result)
}

fn lines(out: &str) -> Vec<Value> {
    out.lines()
        .map(|l| serde_json::from_str(l).expect("a JSON line"))
        .collect()
}

fn send(key: &ConversationKey, text: &str) -> Verb {
    Verb::Send {
        conversation: key.clone(),
        text: Some(text.to_owned()),
    }
}

#[tokio::test]
async fn a_message_is_sent_listed_read_and_kept_and_the_next_run_sees_the_kept_one() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    // Bob holds the session, so the message is his endpoint's to take.
    let (_, opened) = run(&mut bob, Verb::Status, false).await;
    opened.expect("status");

    let (out, sent) = run(&mut alice, send(&to(b.peer()), "hello bob"), true).await;
    sent.expect("sent");
    let sent = lines(&out);
    assert_eq!(sent.len(), 1, "{out}");
    assert_eq!(sent[0]["status"], "accepted");
    assert!(sent[0]["id"].as_str().expect("id").starts_with('p'));

    let (out, listed) = run(&mut bob, Verb::Conversations, true).await;
    listed.expect("conversations");
    let conversations = lines(&out);
    assert_eq!(conversations.len(), 1, "{out}");
    assert_eq!(conversations[0]["unread"], 1);
    let from_alice =
        crate::cli::conversation(conversations[0]["conversation"].as_str().expect("key"))
            .expect("the printed key parses back");

    let (out, shown) = run(&mut bob, Verb::Messages(from_alice.clone()), true).await;
    shown.expect("messages");
    let shown = lines(&out);
    assert_eq!(shown[0]["status"], "unread", "messages marks nothing read");
    assert_eq!(shown[0]["text"], "hello bob");
    let id = shown[0]["id"].as_str().expect("id").to_owned();

    let (out, read) = run(
        &mut bob,
        Verb::Read {
            conversation: from_alice.clone(),
            keep: vec![id],
        },
        true,
    )
    .await;
    read.expect("read");
    let read = lines(&out);
    assert_eq!(read[0]["status"], "kept", "{out}");

    bob.close().await;
    let mut next = session(&b, store(dir_b.path()));
    let (out, again) = run(&mut next, Verb::Messages(from_alice), true).await;
    again.expect("messages");
    let again = lines(&out);
    assert_eq!(again.len(), 1, "{out}");
    assert_eq!(again[0]["status"], "kept");
    assert!(again[0]["id"].as_str().expect("id").starts_with('k'));
}

#[tokio::test]
async fn a_message_read_and_not_kept_is_gone_from_the_next_run() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    run(&mut alice, send(&to(b.peer()), "read once"), false)
        .await
        .1
        .expect("sent");
    let (out, _) = run(&mut bob, Verb::Conversations, true).await;
    let key = crate::cli::conversation(lines(&out)[0]["conversation"].as_str().expect("key"))
        .expect("parses");
    let (out, read) = run(
        &mut bob,
        Verb::Read {
            conversation: key.clone(),
            keep: vec![],
        },
        true,
    )
    .await;
    read.expect("read");
    assert_eq!(lines(&out)[0]["status"], "read");
    bob.close().await;
    let mut next = session(&b, store(dir_b.path()));
    let (out, _) = run(&mut next, Verb::Messages(key), true).await;
    assert!(
        out.is_empty(),
        "RETENTION.md: read and not kept is gone: {out}"
    );
}

#[tokio::test]
async fn a_keep_naming_nothing_shown_refuses_before_anything_is_marked_read() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    run(&mut alice, send(&to(b.peer()), "still unread"), false)
        .await
        .1
        .expect("sent");
    let (out, _) = run(&mut bob, Verb::Conversations, true).await;
    let key = crate::cli::conversation(lines(&out)[0]["conversation"].as_str().expect("key"))
        .expect("parses");
    let (_, read) = run(
        &mut bob,
        Verb::Read {
            conversation: key.clone(),
            keep: vec!["u999".to_owned()],
        },
        false,
    )
    .await;
    assert!(matches!(read, Err(Failure::Refused(_))), "{read:?}");
    let (out, _) = run(&mut bob, Verb::Messages(key), true).await;
    assert_eq!(lines(&out)[0]["status"], "unread");
}

#[tokio::test]
async fn a_send_that_needs_attention_prints_its_id_and_retry_and_cancel_take_it() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");

    a.inject_send(TransportError::UnauthorizedPeer);
    let (out, sent) = run(&mut alice, send(&to(b.peer()), "first"), false).await;
    assert!(matches!(sent, Err(Failure::Refused(_))), "{sent:?}");
    let (id, status) = out.trim().split_once(' ').expect("id and status");
    assert_eq!(status, "needs-attention");
    let (out, retried) = run(&mut alice, Verb::Retry(id.to_owned()), false).await;
    retried.expect("retried");
    assert_eq!(out, format!("{id} accepted\n"));

    a.inject_send(TransportError::UnauthorizedPeer);
    let (out, _) = run(&mut alice, send(&to(b.peer()), "second"), false).await;
    let id = out.split_whitespace().next().expect("id").to_owned();
    let (out, cancelled) = run(&mut alice, Verb::Cancel(id.clone()), false).await;
    cancelled.expect("cancelled");
    assert_eq!(out, format!("{id} cancelled\n"));
    let (_, again) = run(&mut alice, Verb::Cancel(id), false).await;
    assert!(
        matches!(again, Err(Failure::Refused(_))),
        "only once: {again:?}"
    );
}

#[tokio::test]
async fn unkeep_releases_a_kept_message_for_this_run_only() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    run(&mut alice, send(&to(b.peer()), "keep me"), false)
        .await
        .1
        .expect("sent");
    let (out, _) = run(&mut bob, Verb::Conversations, true).await;
    let key = crate::cli::conversation(lines(&out)[0]["conversation"].as_str().expect("key"))
        .expect("parses");
    let (out, _) = run(&mut bob, Verb::Messages(key.clone()), true).await;
    let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    run(
        &mut bob,
        Verb::Read {
            conversation: key.clone(),
            keep: vec![id],
        },
        false,
    )
    .await
    .1
    .expect("kept");
    bob.close().await;

    let mut next = session(&b, store(dir_b.path()));
    let (out, _) = run(&mut next, Verb::Messages(key.clone()), true).await;
    let kept = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    let (out, unkept) = run(&mut next, Verb::Unkeep(kept.clone()), false).await;
    unkept.expect("unkept");
    assert_eq!(out, format!("{kept} unkept\n"));
    next.close().await;
    let mut last = session(&b, store(dir_b.path()));
    let (out, _) = run(&mut last, Verb::Messages(key), true).await;
    assert!(out.is_empty(), "unkept is read and not kept: {out}");
}

#[tokio::test]
async fn trust_is_listed_revoked_and_allowed_again_through_the_settings() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let dir = private_dir();
    let mut alice = session(&a, store(dir.path()));
    let (out, listed) = run(&mut alice, Verb::TrustList, true).await;
    listed.expect("listed");
    let rows = lines(&out);
    assert_eq!(rows.len(), 1, "{out}");
    assert_eq!(rows[0]["peer"], b.peer().as_str());
    assert_eq!(rows[0]["origin"], "configured");

    let (out, revoked) = run(&mut alice, Verb::SetTrust(b.peer().clone(), false), false).await;
    revoked.expect("revoked");
    assert_eq!(out, format!("revoked {}\n", b.peer().as_str()));
    let (out, _) = run(&mut alice, Verb::TrustList, true).await;
    assert!(out.is_empty(), "{out}");

    let (out, allowed) = run(&mut alice, Verb::SetTrust(b.peer().clone(), true), false).await;
    allowed.expect("allowed");
    assert_eq!(out, format!("allowed {}\n", b.peer().as_str()));
    let (out, again) = run(&mut alice, Verb::SetTrust(b.peer().clone(), true), false).await;
    again.expect("idempotent");
    assert_eq!(out, format!("allowed {} (already)\n", b.peer().as_str()));

    let (_, own) = run(&mut alice, Verb::SetTrust(a.peer().clone(), true), false).await;
    assert!(matches!(own, Err(Failure::Refused(_))), "{own:?}");
}

#[tokio::test]
async fn status_says_the_session_and_the_endpoint_it_holds() {
    let (a, _b) = FakeNetwork::pair(node(), node());
    let dir = private_dir();
    let mut alice = session(&a, store(dir.path()));
    let (out, status) = run(&mut alice, Verb::Status, true).await;
    status.expect("status");
    let status = &lines(&out)[0];
    assert_eq!(status["session"], "ready");
    assert_eq!(status["endpoint"], "human");
    assert_eq!(status["unread"], 0);
}

#[tokio::test]
async fn with_the_daemon_gone_a_send_is_unreachable_and_a_read_shows_the_store() {
    let (a, _b) = FakeNetwork::pair(node(), node());
    a.stop();
    let dir = private_dir();
    let client = TransportClient::new(
        a.clone(),
        a.clone(),
        store(dir.path()),
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![endpoint()],
            channels: vec![],
            max_payload_bytes: 49_152,
        },
        Box::new(|| WALL),
        0,
    )
    .expect("facade");
    let mut alice = Session::new(FacadeSide::new(client, || WALL), || Some(false)).expect("s");
    let peer = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer");
    let (out, sent) = run(&mut alice, send(&to(&peer), "nobody"), false).await;
    assert!(matches!(sent, Err(Failure::Unreachable(_))), "{sent:?}");
    assert!(out.is_empty());
    let (out, status) = run(&mut alice, Verb::Status, true).await;
    status.expect("status says it, and succeeds");
    assert_eq!(lines(&out)[0]["session"], "no-daemon");
    let (out, listed) = run(&mut alice, Verb::Conversations, false).await;
    listed.expect("the store is shown without a daemon");
    assert!(out.is_empty());
}

#[tokio::test]
async fn listen_prints_a_message_as_it_arrives_and_leaves_it_unread() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    run(&mut alice, send(&to(b.peer()), "while listening"), false)
        .await
        .1
        .expect("sent");
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        true,
        &mut out,
        tokio::time::sleep(Duration::from_millis(500)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    let events = lines(&out);
    let message = events
        .iter()
        .find(|e| e["event"] == "message")
        .unwrap_or_else(|| panic!("a message event: {out}"));
    assert_eq!(message["text"], "while listening");
    assert_eq!(message["status"], "unread");
    assert!(message["conversation"].is_string());
}

#[tokio::test]
async fn what_a_step_reported_is_held_for_that_step_only() {
    use interweave_human_app_core::Update;
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    run(&mut alice, send(&to(b.peer()), "once"), false)
        .await
        .1
        .expect("sent");
    // A verb that never takes the updates: what it received stays out of
    // them past the step that reported it.
    let (_, listed) = run(&mut bob, Verb::Conversations, false).await;
    listed.expect("conversations");
    bob.step().await;
    assert!(
        !bob.take_updates()
            .iter()
            .any(|u| matches!(u, Update::Received(_))),
        "the message received steps ago is not held"
    );
}

#[tokio::test]
async fn a_keep_naming_an_outbound_message_refuses_before_anything_is_read() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    run(&mut alice, send(&to(b.peer()), "to bob"), false)
        .await
        .1
        .expect("sent");
    let (out, _) = run(&mut bob, Verb::Conversations, true).await;
    let key = crate::cli::conversation(lines(&out)[0]["conversation"].as_str().expect("key"))
        .expect("parses");
    // Bob's own message in the same conversation: an id that is shown
    // there and cannot be kept.
    run(&mut bob, send(&key, "from bob"), false)
        .await
        .1
        .expect("sent back");
    let (out, _) = run(&mut bob, Verb::Messages(key.clone()), true).await;
    let own = lines(&out)
        .into_iter()
        .find(|m| m["direction"] == "out")
        .expect("bob's own message");
    let (out, read) = run(
        &mut bob,
        Verb::Read {
            conversation: key.clone(),
            keep: vec![own["id"].as_str().expect("id").to_owned()],
        },
        false,
    )
    .await;
    assert!(matches!(read, Err(Failure::Refused(_))), "{read:?}");
    assert!(out.is_empty(), "nothing was read, so nothing is printed");
    let (out, _) = run(&mut bob, Verb::Messages(key), true).await;
    assert!(
        lines(&out).iter().any(|m| m["status"] == "unread"),
        "still unread: {out}"
    );
}

#[tokio::test]
async fn a_keep_that_fails_after_the_read_still_prints_what_was_read() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    for text in ["first", "second"] {
        run(&mut alice, send(&to(b.peer()), text), false)
            .await
            .1
            .expect("sent");
    }
    let (out, _) = run(&mut bob, Verb::Conversations, true).await;
    let key = crate::cli::conversation(lines(&out)[0]["conversation"].as_str().expect("key"))
        .expect("parses");
    let (out, _) = run(&mut bob, Verb::Messages(key.clone()), true).await;
    let ids: Vec<String> = lines(&out)
        .iter()
        .map(|m| m["id"].as_str().expect("id").to_owned())
        .collect();
    // The second row cannot be read back now: marking it read fails after
    // the first was read, which this run alone then holds.
    let row: i64 = ids[1][1..].parse().expect("a row");
    let conn = rusqlite::Connection::open(dir_b.path().join("human.sqlite3")).expect("open");
    conn.execute(
        "UPDATE unread_inbound SET source_peer = 'not-a-peer' WHERE row_id = ?1",
        [row],
    )
    .expect("corrupt");
    drop(conn);
    let (out, read) = run(
        &mut bob,
        Verb::Read {
            conversation: key,
            keep: vec![ids[0].clone()],
        },
        true,
    )
    .await;
    assert!(read.is_err(), "{read:?}");
    let shown = lines(&out);
    assert_eq!(shown.len(), 2, "both are printed: {out}");
    assert_eq!(shown[0]["status"], "read", "{out}");
    assert_eq!(shown[0]["text"], "first");
}
