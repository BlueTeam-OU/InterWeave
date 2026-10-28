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
use serde_json::{Value, json};

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
        pending: true,
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
        verbs::ListenMode {
            filter: crate::cli::ListenFilter::default(),
            json: true,
            consume: false,
            input: None,
        },
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
    assert_eq!(
        message["local_endpoint"], "human",
        "G6: the endpoint it reached"
    );
}

/// G1: `listen --input` sends each JSON line over the lease it holds and
/// answers each with its `ref`: an outbound event for a send, an
/// input-refused one for a line it cannot read, and it goes on after
/// both and after EOF. Mutations: a refused line ending listen; the
/// answer losing its ref.
#[tokio::test]
async fn listen_input_sends_each_line_and_answers_with_its_ref() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut alice, Verb::Status, false)
        .await
        .1
        .expect("status");
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    let to_alice = format!("{}/human", a.peer().as_str());
    let (lines_in, input) = tokio::sync::mpsc::channel(verbs::INPUT_QUEUE);
    // As stdin hands them over: a line that is not UTF-8 among them, and
    // two sends, answered in the order read.
    let mut stdin = Vec::new();
    stdin.extend_from_slice(b"not json\n");
    stdin.extend_from_slice(br#"{"send":{"conversation":"x","ref":"bad"}}"#);
    stdin.extend_from_slice(b"\n\xff\xfe{\"send\"}\r\n");
    stdin.extend_from_slice(
        format!(
            "{{\"send\":{{\"conversation\":\"{to_alice}\",\"text\":\"x\",\"ref\":\"ep\",\"endpoint\":\"other\"}}}}\n"
        )
        .as_bytes(),
    );
    for (text, reference) in [("over the lease", 7), ("and after it", 8)] {
        stdin.extend_from_slice(
            format!(
                "{{\"send\":{{\"conversation\":\"{to_alice}\",\"text\":\"{text}\",\"ref\":{reference}}}}}\n"
            )
            .as_bytes(),
        );
    }
    // On a thread of its own, as main runs it: it waits while the channel
    // is full.
    std::thread::spawn(move || verbs::read_input(stdin.as_slice(), &lines_in));
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter::default(),
            json: true,
            consume: false,
            input: Some(input),
        },
        &mut out,
        tokio::time::sleep(Duration::from_millis(1_500)),
    )
    .await
    .expect("listened past EOF");
    let out = String::from_utf8(out).expect("utf-8");
    let events = lines(&out);
    let refused: Vec<&Value> = events
        .iter()
        .filter(|e| e["event"] == "input-refused")
        .collect();
    let refs: Vec<&Value> = refused.iter().map(|e| &e["ref"]).collect();
    assert_eq!(
        refs,
        [&Value::Null, &json!("bad"), &Value::Null, &json!("ep")],
        "{out}"
    );
    assert_eq!(refused[2]["reason"], "not UTF-8 text", "{out}");
    let answered: Vec<&Value> = events
        .iter()
        .filter(|e| e["event"] == "outbound" && e["ref"].is_number())
        .map(|e| &e["ref"])
        .collect();
    assert_eq!(answered.first(), Some(&&json!(7)), "{out}");
    assert!(
        answered.contains(&&json!(8)),
        "the second send, after: {out}"
    );
    let sent = events
        .iter()
        .find(|e| e["event"] == "outbound" && e["ref"] == 7)
        .unwrap_or_else(|| panic!("an outbound answer for ref 7: {out}"));
    assert!(
        sent["id"].as_str().is_some_and(|id| id.starts_with('p')),
        "{out}"
    );
    let of_its_row: Vec<&Value> = events
        .iter()
        .filter(|e| e["event"] == "outbound" && e["id"] == sent["id"])
        .collect();
    assert!(
        of_its_row.iter().all(|e| e["ref"] == 7),
        "every outbound event of its row names its ref: {out}"
    );
    let (out, _) = run(&mut alice, Verb::Messages(to(b.peer())), true).await;
    for text in ["over the lease", "and after it"] {
        assert!(
            lines(&out).iter().any(|m| m["text"] == text),
            "it reached alice: {out}"
        );
    }
}

/// G5: an outbound event names the send problem its status carries
/// beside the status, `null` when there is none, so a carrier tells a
/// refusal from a route that may come back. Mutation: the problem left
/// out of the event.
#[tokio::test]
async fn an_input_sends_outbound_event_names_its_problem() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut alice, Verb::Status, false)
        .await
        .1
        .expect("status");
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    let to_alice = format!("{}/human", a.peer().as_str());
    b.inject_send(TransportError::UnauthorizedPeer);
    let (lines_in, input) = tokio::sync::mpsc::channel(verbs::INPUT_QUEUE);
    lines_in
        .try_send(Ok(format!(
            r#"{{"send":{{"conversation":"{to_alice}","text":"held","ref":"r1"}}}}"#
        )))
        .expect("queued");
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter::default(),
            json: true,
            consume: false,
            input: Some(input),
        },
        &mut out,
        tokio::time::sleep(Duration::from_millis(1_000)),
    )
    .await
    .expect("listened");
    drop(lines_in);
    let out = String::from_utf8(out).expect("utf-8");
    let outbound: Vec<Value> = lines(&out)
        .into_iter()
        .filter(|e| e["event"] == "outbound")
        .collect();
    assert!(
        outbound.iter().all(|e| e.get("problem").is_some()),
        "every outbound event has the field: {out}"
    );
    assert!(
        outbound.iter().any(|e| e["ref"] == "r1"
            && e["status"] == "needs-attention"
            && e["problem"] == "peer-untrusted"),
        "{out}"
    );
}

/// A line's `ref` is capped at [`verbs::MAX_INPUT_REF_BYTES`] serialised:
/// one at the cap is sent and echoed; one byte over is refused
/// input-refused with `ref` null, the send not made. Mutation: the cap
/// taken as exclusive.
#[tokio::test]
async fn a_ref_past_its_cap_is_refused_and_one_at_it_is_echoed() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut alice, Verb::Status, false)
        .await
        .1
        .expect("status");
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    let to_alice = format!("{}/human", a.peer().as_str());
    // A JSON string serialises with its two quotes.
    let at_cap = "a".repeat(verbs::MAX_INPUT_REF_BYTES - 2);
    let over = "b".repeat(verbs::MAX_INPUT_REF_BYTES - 1);
    let (lines_in, input) = tokio::sync::mpsc::channel(verbs::INPUT_QUEUE);
    for (text, reference) in [("over", &over), ("at the cap", &at_cap)] {
        let line = json!({"send": {"conversation": to_alice, "text": text, "ref": reference}});
        lines_in.try_send(Ok(line.to_string())).expect("queued");
    }
    drop(lines_in);
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter::default(),
            json: true,
            consume: false,
            input: Some(input),
        },
        &mut out,
        tokio::time::sleep(Duration::from_millis(1_000)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    let events = lines(&out);
    let refused: Vec<&Value> = events
        .iter()
        .filter(|e| e["event"] == "input-refused")
        .collect();
    assert_eq!(refused.len(), 1, "{out}");
    assert_eq!(refused[0]["ref"], Value::Null, "{out}");
    assert!(
        events
            .iter()
            .any(|e| e["event"] == "outbound" && e["ref"] == Value::String(at_cap.clone())),
        "{out}"
    );
    assert!(!out.contains(&over), "the long ref is echoed nowhere");
}

/// G3: `listen --consume` marks each message read once its line is
/// written, so the store holds it unread no more. Mutation: consume
/// marking nothing.
#[tokio::test]
async fn listen_consume_marks_each_printed_message_read() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    run(&mut alice, send(&to(b.peer()), "consumed"), false)
        .await
        .1
        .expect("sent");
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter::default(),
            json: true,
            consume: true,
            input: None,
        },
        &mut out,
        tokio::time::sleep(Duration::from_millis(800)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    assert!(lines(&out).iter().any(|e| e["text"] == "consumed"), "{out}");
    bob.close().await;
    let unread = store(dir_b.path()).unread_inbound().expect("read");
    assert!(unread.is_empty(), "read once printed: {unread:?}");
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

/// G4: `send --no-pending` that is not accepted by the end of `--wait` is
/// cancelled, said so with `"pending": false`, and leaves no pending row
/// for a later run to send; without it the send stays pending (exit 6).
/// Mutation: --no-pending leaving the send pending.
#[tokio::test]
async fn a_send_with_no_pending_is_cancelled_when_its_wait_ends() {
    let (a, _b) = FakeNetwork::pair(node(), node());
    let dir = private_dir();
    let mut alice = session(&a, store(dir.path()));
    run(&mut alice, Verb::Status, false)
        .await
        .1
        .expect("status");
    // A peer with no address: the send is retried, never accepted.
    let nobody = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer");
    let verb = Verb::Send {
        conversation: to(&nobody),
        text: Some("ephemeral".to_owned()),
        pending: false,
    };
    let (out, sent) = run(&mut alice, verb, true).await;
    assert!(matches!(sent, Err(Failure::Refused(_))), "{sent:?}");
    let line = &lines(&out)[0];
    assert_eq!(line["status"], "cancelled", "{out}");
    assert_eq!(line["pending"], false);
    alice.close().await;
    let pending = store(dir.path()).pending_outbound().expect("read");
    assert!(pending.is_empty(), "never sent by a later run: {pending:?}");
}

/// G4 for a send held for attention: the next run's facade attempts every
/// pending row again, so `--no-pending` cancels it too, rather than
/// leaving it for a later run to send once the peer is trusted.
/// Mutation: the needs-attention arm returning before the cancel.
#[tokio::test]
async fn a_send_with_no_pending_that_needs_attention_is_cancelled() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    a.inject_send(TransportError::UnauthorizedPeer);
    let verb = Verb::Send {
        conversation: to(b.peer()),
        text: Some("held".to_owned()),
        pending: false,
    };
    let (out, sent) = run(&mut alice, verb, true).await;
    assert!(matches!(sent, Err(Failure::Refused(_))), "{sent:?}");
    let line = &lines(&out)[0];
    assert_eq!(line["status"], "cancelled", "{out}");
    assert_eq!(line["pending"], false);
    alice.close().await;
    let pending = store(dir_a.path()).pending_outbound().expect("read");
    assert!(pending.is_empty(), "never sent by a later run: {pending:?}");
}

/// Bob's store holding `texts` unread from alice, committed by a run that
/// reads nothing, and bob's session opened afresh over it.
async fn unread_at_open(
    texts: &[&str],
) -> (
    FakeNode,
    tempfile::TempDir,
    Session<FakeNode, FakeNode>,
    tempfile::TempDir,
) {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    for text in texts {
        run(&mut alice, send(&to(b.peer()), text), false)
            .await
            .1
            .expect("sent");
    }
    run(&mut bob, Verb::Conversations, true)
        .await
        .1
        .expect("listed");
    bob.close().await;
    alice.close().await;
    assert_eq!(
        store(dir_b.path()).unread_inbound().expect("read").len(),
        texts.len()
    );
    let bob = session(&b, store(dir_b.path()));
    (b, dir_b, bob, dir_a)
}

/// G3: `--consume` drains what the store held unread when it opened --
/// printed as it would have been, `local_endpoint` `null` since the store
/// keeps no record of the endpoint that received it -- and the store holds
/// it unread no more. Mutation: no drain at open.
#[tokio::test]
async fn listen_consume_drains_what_the_store_held_unread_at_open() {
    let (_b, dir_b, mut bob, _dir_a) = Box::pin(unread_at_open(&["held", "and this"])).await;
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter::default(),
            json: true,
            consume: true,
            input: None,
        },
        &mut out,
        tokio::time::sleep(Duration::from_millis(500)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    let messages: Vec<Value> = lines(&out)
        .into_iter()
        .filter(|e| e["event"] == "message")
        .collect();
    let texts: Vec<&Value> = messages.iter().map(|m| &m["text"]).collect();
    assert_eq!(texts.len(), 2, "each once: {out}");
    assert!(texts.contains(&&json!("held")) && texts.contains(&&json!("and this")));
    assert!(
        messages.iter().all(|m| m["local_endpoint"].is_null()),
        "{out}"
    );
    bob.close().await;
    let unread = store(dir_b.path()).unread_inbound().expect("read");
    assert!(unread.is_empty(), "drained: {unread:?}");
}

/// What `put` is handed, failing every write once `messages` message
/// lines are written with a broken pipe, and stopping `listen` once a
/// `message` line is written.
struct Tap {
    out: Vec<u8>,
    messages: Option<usize>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
}

impl std::io::Write for Tap {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let message = buf.windows(17).any(|w| w == br#""event":"message""#);
        if let Some(left) = self.messages.as_mut()
            && message
        {
            if *left == 0 {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            *left -= 1;
        }
        self.out.extend_from_slice(buf);
        if message && let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A message whose line was written is read before `listen` ends, even
/// when the stop comes the moment the line is out. Mutation: the read
/// left for a later step.
#[tokio::test]
async fn a_printed_message_is_read_before_listen_ends() {
    let (_b, dir_b, mut bob, _dir_a) = Box::pin(unread_at_open(&["last words"])).await;
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let mut tap = Tap {
        out: Vec::new(),
        messages: None,
        stop: Some(stop),
    };
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter::default(),
            json: true,
            consume: true,
            input: None,
        },
        &mut tap,
        // The bound only keeps a broken listen from hanging the test.
        async {
            let _ = tokio::time::timeout(Duration::from_secs(5), stopped).await;
        },
    )
    .await
    .expect("listened");
    let out = String::from_utf8(tap.out).expect("utf-8");
    assert!(out.contains("last words"), "{out}");
    bob.close().await;
    let unread = store(dir_b.path()).unread_inbound().expect("read");
    assert!(unread.is_empty(), "read as written: {unread:?}");
}

/// A reader that goes away mid-batch: the line written is read, the one
/// not written stays unread, and `listen` ends as a stop does. Mutation:
/// the reads of a batch dropped when the pipe breaks.
#[tokio::test]
async fn a_line_not_written_leaves_its_message_unread() {
    let (_b, dir_b, mut bob, _dir_a) = Box::pin(unread_at_open(&["one", "two"])).await;
    let mut tap = Tap {
        out: Vec::new(),
        messages: Some(1),
        stop: None,
    };
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter::default(),
            json: true,
            consume: true,
            input: None,
        },
        &mut tap,
        // Ended by the reader going away; the bound only keeps a broken
        // listen from hanging the test.
        tokio::time::sleep(Duration::from_secs(5)),
    )
    .await
    .expect("the reader gone ends it");
    let out = String::from_utf8(tap.out).expect("utf-8");
    let written = lines(&out)
        .into_iter()
        .filter(|e| e["event"] == "message")
        .count();
    assert_eq!(written, 1, "{out}");
    bob.close().await;
    let unread = store(dir_b.path()).unread_inbound().expect("read");
    assert_eq!(unread.len(), 1, "the one not written: {unread:?}");
}

/// Every read `--consume` asks that the store cannot carry out is said
/// as a `failed` event naming its row, never dropped: a drain's reads run
/// in one step. Mutations: listen leaving the model side's problems
/// untaken; only the first of a step's said.
#[tokio::test]
async fn every_read_the_store_refuses_is_said_by_its_row() {
    let (_b, dir_b, mut bob, _dir_a) = Box::pin(unread_at_open(&["gone", "and gone"])).await;
    // Gone from the store after the session listed them: each read finds
    // no row.
    let conn = rusqlite::Connection::open(dir_b.path().join("human.sqlite3")).expect("open");
    conn.execute("DELETE FROM unread_inbound", [])
        .expect("deleted");
    drop(conn);
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter::default(),
            json: true,
            consume: true,
            input: None,
        },
        &mut out,
        tokio::time::sleep(Duration::from_millis(500)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    let events = lines(&out);
    let printed: Vec<&Value> = events
        .iter()
        .filter(|e| e["event"] == "message")
        .map(|e| &e["id"])
        .collect();
    let mut failed: Vec<&Value> = events
        .iter()
        .filter(|e| e["event"] == "failed" && e["reason"].as_str().is_some_and(|r| !r.is_empty()))
        .map(|e| &e["id"])
        .collect();
    failed.sort_by_key(|id| id.as_str().map(str::to_owned));
    let mut printed = printed;
    printed.sort_by_key(|id| id.as_str().map(str::to_owned));
    assert_eq!(printed.len(), 2, "{out}");
    assert_eq!(failed, printed, "each read refused, by its row: {out}");
}

/// A send the facade refuses before any row is made answers its own line
/// `input-refused` with the facade's reason, and the next line is sent.
/// Mutation: the refusal left unanswered, which stalls the input.
#[tokio::test]
async fn an_input_send_refused_before_a_row_is_answered_and_input_goes_on() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut alice, Verb::Status, false)
        .await
        .1
        .expect("status");
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    let to_alice = format!("{}/human", a.peer().as_str());
    let (lines_in, input) = tokio::sync::mpsc::channel(verbs::INPUT_QUEUE);
    // Past the payload bound however it is encoded: text that does not
    // compress.
    let mut seed: u64 = 1;
    let symbols = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let big: String = (0..120_000)
        .map(|_| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            char::from(symbols[usize::try_from(seed >> 58).unwrap_or(0)])
        })
        .collect();
    for (text, reference) in [(big, "big"), ("after".to_owned(), "after")] {
        let line = json!({"send": {"conversation": to_alice, "text": text, "ref": reference}});
        lines_in.try_send(Ok(line.to_string())).expect("queued");
    }
    drop(lines_in);
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter::default(),
            json: true,
            consume: false,
            input: Some(input),
        },
        &mut out,
        tokio::time::sleep(Duration::from_millis(1_000)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    let events = lines(&out);
    assert!(
        events.iter().any(|e| e["event"] == "input-refused"
            && e["ref"] == "big"
            && e["reason"] == "too-large"),
        "{out}"
    );
    assert!(
        events
            .iter()
            .any(|e| e["event"] == "outbound" && e["ref"] == "after"),
        "{out}"
    );
}

/// Every line `reader` gives under `ceiling`, read as `listen --input`
/// reads stdin: on a thread of its own, into the bounded channel.
fn read_all(
    reader: impl std::io::BufRead + Send + 'static,
    ceiling: usize,
) -> Vec<verbs::InputLine> {
    let (lines_in, mut lines) = tokio::sync::mpsc::channel(verbs::INPUT_QUEUE);
    let reading = std::thread::spawn(move || verbs::read_input_under(reader, &lines_in, ceiling));
    let mut read = Vec::new();
    while let Some(line) = lines.blocking_recv() {
        read.push(line);
    }
    reading.join().expect("the reader");
    read
}

/// A line past the ceiling, checked as its bytes arrive, is refused and
/// drained through its newline, and the next line is read; a line at the
/// ceiling is taken, a carriage return before the newline counting toward
/// it, and a last line without a newline too. Read three bytes at a time,
/// so a line spans many reads. Mutations: the ceiling taken as exclusive;
/// the drain ending before the newline.
#[test]
fn a_line_past_the_ceiling_is_refused_and_the_next_is_read() {
    let input = b"12345678\n123456789abc\n1234567\r\nnext\ntail".to_vec();
    let read = read_all(
        std::io::BufReader::with_capacity(3, std::io::Cursor::new(input)),
        8,
    );
    assert_eq!(
        read,
        [
            Ok("12345678".to_owned()),
            Err("longer than 8 bytes".to_owned()),
            Ok("1234567".to_owned()),
            Ok("next".to_owned()),
            Ok("tail".to_owned()),
        ]
    );
}

/// The program's own ceiling: a line of exactly
/// [`verbs::MAX_INPUT_LINE_BYTES`] is taken and one byte more is refused.
#[test]
fn a_line_at_the_input_ceiling_is_taken_and_one_byte_more_is_not() {
    let mut input = vec![b'a'; verbs::MAX_INPUT_LINE_BYTES];
    input.push(b'\n');
    input.extend(vec![b'b'; verbs::MAX_INPUT_LINE_BYTES + 1]);
    input.push(b'\n');
    let read = read_all(
        std::io::BufReader::new(std::io::Cursor::new(input)),
        verbs::MAX_INPUT_LINE_BYTES,
    );
    assert_eq!(read.len(), 2);
    assert_eq!(
        read[0].as_ref().map(String::len),
        Ok(verbs::MAX_INPUT_LINE_BYTES)
    );
    assert!(read[1].is_err(), "one byte past it");
}

/// A reader ahead of listen waits once [`verbs::INPUT_QUEUE`] lines are
/// not yet taken, rather than reading on into memory, and every line is
/// still delivered once they are taken. Mutation: the send not waiting
/// (a full channel dropping the line).
#[test]
fn a_reader_ahead_of_listen_waits_rather_than_growing() {
    let input: Vec<u8> = (0..100)
        .flat_map(|n| format!("line {n}\n").into_bytes())
        .collect();
    let (lines_in, mut lines) = tokio::sync::mpsc::channel(verbs::INPUT_QUEUE);
    let reading = std::thread::spawn(move || {
        verbs::read_input(std::io::Cursor::new(input), &lines_in);
    });
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(lines.len(), verbs::INPUT_QUEUE, "the channel is full");
    assert!(!reading.is_finished(), "and the reader waits on it");
    let mut taken = 0;
    while let Some(line) = lines.blocking_recv() {
        assert_eq!(line, Ok(format!("line {taken}")));
        taken += 1;
    }
    assert_eq!(taken, 100, "none dropped");
    reading.join().expect("the reader");
}

/// The line ceiling is checked as the bytes arrive: a 256 MiB line far
/// past it is drained while the process's peak memory grows by less than
/// 16 MiB, and the next line is read (#495 review F5, F7). Read from the
/// kernel's high-water mark, which a line held before it is judged raises
/// by what is held. What it holds is that much: a part of the line under
/// 16 MiB held is not seen. Mutation: a `read_until` of the whole line,
/// then the length check.
#[cfg(target_os = "linux")]
#[test]
fn an_over_long_line_is_refused_without_being_held() {
    use std::io::Read as _;
    fn peak_kib() -> u64 {
        std::fs::read_to_string("/proc/self/status")
            .expect("status")
            .lines()
            .find_map(|l| l.strip_prefix("VmHWM:"))
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok())
            .expect("VmHWM")
    }
    const LINE: u64 = 256 << 20;
    let before = peak_kib();
    let input = std::io::repeat(b'a')
        .take(LINE)
        .chain(std::io::Cursor::new(b"\nnext\n".to_vec()));
    let read = read_all(std::io::BufReader::new(input), 8);
    let grown = peak_kib().saturating_sub(before);
    assert_eq!(
        read,
        [Err("longer than 8 bytes".to_owned()), Ok("next".to_owned())]
    );
    assert!(
        grown < 16 << 10,
        "peak memory grew {grown} KiB for a {} KiB line",
        LINE >> 10
    );
}

/// G7: a filtered `listen --consume` consumes only what it prints. A
/// message arriving in another conversation than `--conversation` names
/// is not printed and stays unread; one in the conversation named is
/// printed and read. Mutation: the filter ignored for a message that
/// arrives.
#[tokio::test]
async fn a_filtered_listen_consumes_only_what_it_prints() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    run(
        &mut alice,
        send(&to(b.peer()), "not for this filter"),
        false,
    )
    .await
    .1
    .expect("sent");
    let elsewhere = to(&node().peer);
    let listen = |filter: crate::cli::ListenFilter| verbs::ListenMode {
        filter,
        json: true,
        consume: true,
        input: None,
    };
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        listen(crate::cli::ListenFilter {
            events: None,
            conversation: Some(elsewhere),
        }),
        &mut out,
        tokio::time::sleep(Duration::from_millis(800)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    assert!(
        !lines(&out).iter().any(|e| e["event"] == "message"),
        "{out}"
    );
    assert_eq!(
        store(dir_b.path()).unread_inbound().expect("read").len(),
        1,
        "not printed, not read"
    );

    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        listen(crate::cli::ListenFilter {
            events: Some(vec!["message"]),
            conversation: Some(ConversationKey::Direct {
                peer: a.peer().clone(),
                endpoint: None,
            }),
        }),
        &mut out,
        tokio::time::sleep(Duration::from_millis(800)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    let events = lines(&out);
    assert!(events.iter().all(|e| e["event"] == "message"), "{out}");
    assert!(
        events.iter().any(|e| e["text"] == "not for this filter"),
        "{out}"
    );
    bob.close().await;
    assert!(
        store(dir_b.path())
            .unread_inbound()
            .expect("read")
            .is_empty()
    );
}

/// G7: `--events` drops the events it does not name, and what the store
/// held unread at open is drained only if `message` is named: here it is
/// not, so nothing is printed of it and it stays unread. Mutation: the
/// drain at open filtered by nothing.
#[tokio::test]
async fn a_listen_not_naming_messages_drains_none_of_them() {
    let (_b, dir_b, mut bob, _dir_a) = Box::pin(unread_at_open(&["held"])).await;
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter {
                events: Some(vec!["session"]),
                conversation: None,
            },
            json: true,
            consume: true,
            input: None,
        },
        &mut out,
        tokio::time::sleep(Duration::from_millis(500)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    let events = lines(&out);
    assert!(!events.is_empty(), "the session event: {out}");
    assert!(events.iter().all(|e| e["event"] == "session"), "{out}");
    bob.close().await;
    assert_eq!(store(dir_b.path()).unread_inbound().expect("read").len(), 1);
}

/// G7: an `--input` line is answered whatever the filter -- its first
/// `outbound` line and each later status of the row it made, all with its
/// `ref` -- though the filter names no `outbound` event (#508 review F1).
/// Mutation: a later outbound event of an input's row filtered like any.
#[tokio::test]
async fn an_input_lines_answers_print_whatever_the_filter() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (dir_a, dir_b) = (private_dir(), private_dir());
    let mut alice = session(&a, store(dir_a.path()));
    let mut bob = session(&b, store(dir_b.path()));
    run(&mut alice, Verb::Status, false)
        .await
        .1
        .expect("status");
    run(&mut bob, Verb::Status, false).await.1.expect("status");
    let to_alice = format!("{}/human", a.peer().as_str());
    let (lines_in, input) = tokio::sync::mpsc::channel(verbs::INPUT_QUEUE);
    let stdin = format!(
        "{{\"send\":{{\"conversation\":\"{to_alice}\",\"text\":\"filtered\",\"ref\":9}}}}\n"
    )
    .into_bytes();
    std::thread::spawn(move || verbs::read_input(stdin.as_slice(), &lines_in));
    let mut out = Vec::new();
    verbs::listen(
        &mut bob,
        verbs::ListenMode {
            filter: crate::cli::ListenFilter {
                events: Some(vec!["session"]),
                conversation: None,
            },
            json: true,
            consume: false,
            input: Some(input),
        },
        &mut out,
        tokio::time::sleep(Duration::from_millis(1_500)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    let answers: Vec<Value> = lines(&out)
        .into_iter()
        .filter(|e| e["event"] == "outbound")
        .collect();
    assert!(answers.iter().all(|e| e["ref"] == 9), "{out}");
    // One from the send's own answer, the rest the row's later status
    // events: the send answers once, so a second is a later one.
    assert!(answers.len() >= 2, "the answer and a later status: {out}");
}
