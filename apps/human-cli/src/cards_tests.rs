// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Batch 2's verbs at the model level: the own card, sharing it, a
//! proposal shown and decided, and the contacts, against the in-memory
//! node pair over file stores, each side holding its identity's key. What
//! these do not show -- the IPC binding and a real daemon -- is
//! `tests/desktop-e2e/tests/human_cli.rs`'s.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::time::{Duration, Instant};

use interweave_human_app_core::FacadeSide;
use interweave_human_store::identity_key::IdentityKey;
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::ConversationKey;
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_config::{ProfilePaths, TrustBoundary};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use serde_json::Value;

use crate::Failure;
use crate::cli::{PhotoArg, Verb};
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

/// A side: its node, and its store directory, under which its identity's
/// key file lives: every run of it opens the key, as every start of the
/// client does, by app-core's rule.
struct Side {
    node: FakeNode,
    dir: tempfile::TempDir,
}

impl Side {
    fn new(node: FakeNode) -> Self {
        Self {
            node,
            dir: private_dir(),
        }
    }

    fn key(&self) -> IdentityKey {
        let paths = ProfilePaths::resolve_embedded(
            "default",
            TrustBoundary::new(self.dir.path()).expect("a boundary"),
        )
        .expect("paths");
        interweave_human_app_core::identity_key(&paths, &[endpoint()], &store(self.dir.path()))
            .expect("opened or made")
            .expect("a key")
    }

    fn session(&self) -> Session<FakeNode, FakeNode> {
        let client = TransportClient::new(
            self.node.clone(),
            self.node.clone(),
            store(self.dir.path()),
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
        let mut facade = FacadeSide::new(client, || WALL);
        facade.set_identity_key(self.key());
        Session::new(facade, || Some(true)).expect("session")
    }

    fn route(&self) -> ConversationKey {
        to(self.node.peer())
    }
}

fn to(peer: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: peer.clone(),
        endpoint: Some(endpoint()),
    }
}

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
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{l:?}: {e}")))
        .collect()
}

fn set(name: &str, surname: Option<&str>) -> Verb {
    Verb::CardSet {
        name: name.to_owned(),
        surname: surname.map(str::to_owned),
        photo: PhotoArg::Keep,
    }
}

/// Alice names her card and shares it with Bob, whose client holds the
/// session: Bob then holds one proposal from her.
async fn shared() -> (Side, Side, Session<FakeNode, FakeNode>) {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (alice, bob) = (Side::new(a), Side::new(b));
    let mut b_run = bob.session();
    run(&mut b_run, Verb::Status, false)
        .await
        .1
        .expect("status");
    let mut a_run = alice.session();
    run(&mut a_run, set("Ada", Some("Lovelace")), false)
        .await
        .1
        .expect("set");
    let (out, shared) = run(&mut a_run, Verb::CardShare(bob.route()), true).await;
    shared.expect("shared");
    assert_eq!(lines(&out)[0]["status"], "accepted", "{out}");
    a_run.close().await;
    (alice, bob, b_run)
}

#[tokio::test]
async fn the_own_card_is_set_and_shown_with_its_fingerprint() {
    let (a, _b) = FakeNetwork::pair(node(), node());
    let alice = Side::new(a);
    let mut s = alice.session();
    let (_, done) = run(&mut s, set("Ada", Some("Lovelace")), false).await;
    done.expect("set");
    let (out, shown) = run(&mut s, Verb::CardShow, true).await;
    shown.expect("shown");
    let card = &lines(&out)[0];
    assert_eq!(card["name"], "Ada Lovelace");
    assert_eq!(card["endpoint"], "human");
    assert_eq!(card["has_photo"], false);
    let fingerprint = card["fingerprint"].as_str().expect("a fingerprint");
    assert_eq!(fingerprint.len(), 16);
    assert!(fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn a_shared_card_is_a_proposal_that_accept_makes_a_contact() {
    let (alice, _bob, mut b_run) = shared().await;
    let (out, listed) = run(&mut b_run, Verb::Proposals, true).await;
    listed.expect("proposals");
    let held = lines(&out);
    assert_eq!(held.len(), 1, "{out}");
    assert_eq!(held[0]["status"], "unread", "listing reads nothing");
    assert_eq!(held[0]["name"], "Ada Lovelace");
    assert_eq!(held[0]["peer"], alice.node.peer().as_str());
    assert_eq!(held[0]["accept"]["creates"], true);
    let id = held[0]["id"].as_str().expect("id").to_owned();

    let (out, read) = run(
        &mut b_run,
        Verb::ProposalsRead {
            conversation: alice.route(),
            accept: vec![id],
            decline: vec![],
        },
        true,
    )
    .await;
    read.expect("accepted");
    assert_eq!(lines(&out)[0]["status"], "accepted", "{out}");

    let (out, _) = run(&mut b_run, Verb::Contacts, true).await;
    let contacts = lines(&out);
    assert_eq!(contacts.len(), 1, "{out}");
    assert_eq!(contacts[0]["name"], "Ada Lovelace");
    assert_eq!(contacts[0]["name_from"], "card");
    let id = contacts[0]["id"].as_str().expect("id").to_owned();
    let (out, shown) = run(&mut b_run, Verb::ContactShow(id), true).await;
    shown.expect("shown");
    let route = &lines(&out)[0]["routes"][0];
    assert_eq!(route["peer"], alice.node.peer().as_str());
    assert_eq!(route["endpoint"], "human");
    assert_eq!(route["fingerprint"], held[0]["fingerprint"]);
}

#[tokio::test]
async fn a_proposal_shown_and_left_undecided_is_gone_from_the_next_run() {
    let (alice, bob, mut b_run) = shared().await;
    let (out, read) = run(
        &mut b_run,
        Verb::ProposalsRead {
            conversation: alice.route(),
            accept: vec![],
            decline: vec![],
        },
        true,
    )
    .await;
    read.expect("read");
    assert_eq!(lines(&out)[0]["status"], "read");
    b_run.close().await;
    let mut next = bob.session();
    let (out, _) = run(&mut next, Verb::Proposals, true).await;
    assert!(out.is_empty(), "RETENTION.md: a card shown is read: {out}");
    let (out, _) = run(&mut next, Verb::Contacts, true).await;
    assert!(out.is_empty(), "and nothing was written: {out}");
}

#[tokio::test]
async fn decline_writes_nothing() {
    let (alice, _bob, mut b_run) = shared().await;
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    let (out, declined) = run(
        &mut b_run,
        Verb::ProposalsRead {
            conversation: alice.route(),
            accept: vec![],
            decline: vec![id],
        },
        true,
    )
    .await;
    declined.expect("declined");
    assert_eq!(lines(&out)[0]["status"], "declined");
    let (out, _) = run(&mut b_run, Verb::Contacts, true).await;
    assert!(out.is_empty(), "{out}");
}

#[tokio::test]
async fn a_decision_naming_no_proposal_refuses_before_anything_is_read() {
    let (alice, _bob, mut b_run) = shared().await;
    for (accept, decline) in [
        (vec!["u999".to_owned()], vec![]),
        (vec![], vec!["p1".to_owned()]),
    ] {
        let (out, refused) = run(
            &mut b_run,
            Verb::ProposalsRead {
                conversation: alice.route(),
                accept,
                decline,
            },
            false,
        )
        .await;
        assert!(matches!(refused, Err(Failure::Refused(_))), "{refused:?}");
        assert!(out.is_empty());
    }
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    assert_eq!(lines(&out)[0]["status"], "unread", "still unread");
    let (_, both) = run(
        &mut b_run,
        Verb::ProposalsRead {
            conversation: alice.route(),
            accept: vec![id.clone()],
            decline: vec![id],
        },
        false,
    )
    .await;
    assert!(matches!(both, Err(Failure::Refused(_))), "{both:?}");
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    assert_eq!(lines(&out)[0]["status"], "unread", "still unread");
}

#[tokio::test]
async fn a_decision_that_fails_after_the_read_still_prints_the_proposals() {
    let (alice, bob, mut b_run) = shared().await;
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    // The card's row cannot be read back now: showing it fails after the
    // proposals were listed, and they are printed all the same.
    let row: i64 = id[1..].parse().expect("a row");
    let conn = rusqlite::Connection::open(bob.dir.path().join("human.sqlite3")).expect("open");
    conn.execute(
        "UPDATE unread_inbound SET source_peer = 'not-a-peer' WHERE row_id = ?1",
        [row],
    )
    .expect("corrupt");
    drop(conn);
    let (out, read) = run(
        &mut b_run,
        Verb::ProposalsRead {
            conversation: alice.route(),
            accept: vec![id],
            decline: vec![],
        },
        true,
    )
    .await;
    assert!(read.is_err(), "{read:?}");
    let shown = lines(&out);
    assert_eq!(shown.len(), 1, "printed: {out}");
    assert_eq!(shown[0]["name"], "Ada Lovelace");
}

#[tokio::test]
async fn a_contact_is_renamed_its_photo_removed_and_deleted_by_the_person() {
    let (alice, _bob, mut b_run) = shared().await;
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    run(
        &mut b_run,
        Verb::ProposalsRead {
            conversation: alice.route(),
            accept: vec![id],
            decline: vec![],
        },
        false,
    )
    .await
    .1
    .expect("accepted");
    let (out, _) = run(&mut b_run, Verb::Contacts, true).await;
    let contact = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    let (_, renamed) = run(
        &mut b_run,
        Verb::ContactRename(contact.clone(), "Countess".to_owned()),
        false,
    )
    .await;
    renamed.expect("renamed");
    let (_, removed) = run(
        &mut b_run,
        Verb::ContactPhoto(contact.clone(), PhotoArg::Remove),
        false,
    )
    .await;
    removed.expect("removed");
    let (out, _) = run(&mut b_run, Verb::ContactShow(contact.clone()), true).await;
    let shown = &lines(&out)[0];
    assert_eq!(shown["name"], "Countess");
    assert_eq!(
        shown["name_from"], "own",
        "the person's name, which no card replaces"
    );
    let (_, unknown) = run(
        &mut b_run,
        Verb::ContactRename("no-such-contact".to_owned(), "X".to_owned()),
        false,
    )
    .await;
    assert!(matches!(unknown, Err(Failure::Refused(_))), "{unknown:?}");
    let (out, deleted) = run(&mut b_run, Verb::ContactDelete(contact.clone()), false).await;
    deleted.expect("deleted");
    assert_eq!(out, format!("{contact} deleted\n"));
    let (out, _) = run(&mut b_run, Verb::Contacts, true).await;
    assert!(out.is_empty(), "{out}");
    let (_, again) = run(&mut b_run, Verb::ContactDelete(contact), false).await;
    assert!(matches!(again, Err(Failure::Refused(_))), "{again:?}");
}

#[tokio::test]
async fn a_card_is_shared_only_on_a_route_naming_its_endpoint() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let alice = Side::new(a);
    let mut s = alice.session();
    for key in [
        ConversationKey::Direct {
            peer: b.peer().clone(),
            endpoint: None,
        },
        ConversationKey::Channel(
            interweave_transport_api::ChannelId::parse("general").expect("a channel"),
        ),
    ] {
        let (out, refused) = run(&mut s, Verb::CardShare(key), false).await;
        assert!(matches!(refused, Err(Failure::Refused(_))), "{refused:?}");
        assert!(out.is_empty());
    }
}

#[test]
fn a_photo_file_past_the_bound_is_refused_unread() {
    let dir = private_dir();
    let big = dir.path().join("big.jpg");
    let file = std::fs::File::create(&big).expect("create");
    file.set_len(crate::cards::MAX_PHOTO_FILE + 1)
        .expect("sparse");
    assert!(matches!(
        crate::cards::photo(&PhotoArg::File(big)),
        Err(Failure::Refused(_))
    ));
    let fits = dir.path().join("fits.jpg");
    std::fs::write(&fits, b"not checked here").expect("write");
    assert!(matches!(
        crate::cards::photo(&PhotoArg::File(fits)),
        Ok(interweave_human_ui_model::PhotoEdit::Chosen(bytes)) if bytes == b"not checked here"
    ));
}

#[tokio::test]
async fn listen_prints_a_card_as_a_proposal_and_leaves_it_unread() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (alice, bob) = (Side::new(a), Side::new(b));
    let mut b_run = bob.session();
    run(&mut b_run, Verb::Status, false)
        .await
        .1
        .expect("status");
    let mut a_run = alice.session();
    run(&mut a_run, set("Ada", None), false)
        .await
        .1
        .expect("set");
    run(&mut a_run, Verb::CardShare(bob.route()), false)
        .await
        .1
        .expect("shared");
    let mut out = Vec::new();
    verbs::listen(
        &mut b_run,
        true,
        &mut out,
        tokio::time::sleep(Duration::from_millis(500)),
    )
    .await
    .expect("listened");
    let out = String::from_utf8(out).expect("utf-8");
    let card = lines(&out)
        .into_iter()
        .find(|e| e["event"] == "proposal")
        .unwrap_or_else(|| panic!("a proposal event: {out}"));
    assert_eq!(card["name"], "Ada");
    assert_eq!(card["status"], "unread");
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    assert_eq!(lines(&out)[0]["status"], "unread", "still unread: {out}");
}

#[tokio::test]
async fn an_accept_made_before_another_fails_is_printed_accepted() {
    let (alice, bob, mut b_run) = shared().await;
    // A second card on the same route: shown as creating a contact too,
    // which the first accept makes false, so the second is refused as
    // changed after the first was made.
    let mut a_run = alice.session();
    run(&mut a_run, Verb::CardShare(bob.route()), false)
        .await
        .1
        .expect("shared again");
    a_run.close().await;
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    let held = lines(&out);
    assert_eq!(held.len(), 2, "{out}");
    let ids: Vec<String> = held
        .iter()
        .map(|p| p["id"].as_str().expect("id").to_owned())
        .collect();
    let (out, read) = run(
        &mut b_run,
        Verb::ProposalsRead {
            conversation: alice.route(),
            accept: ids.clone(),
            decline: vec![],
        },
        true,
    )
    .await;
    assert!(matches!(read, Err(Failure::Refused(_))), "{read:?}");
    let shown = lines(&out);
    assert_eq!(shown[0]["status"], "accepted", "the one made: {out}");
    assert_eq!(shown[1]["status"], "read", "the one refused: {out}");
    let (out, _) = run(&mut b_run, Verb::Contacts, true).await;
    assert_eq!(lines(&out).len(), 1, "the first accept stands: {out}");
}

#[tokio::test]
async fn proposals_are_read_on_a_route_naming_its_endpoint() {
    let (alice, _bob, mut b_run) = shared().await;
    let (out, refused) = run(
        &mut b_run,
        Verb::ProposalsRead {
            conversation: ConversationKey::Direct {
                peer: alice.node.peer().clone(),
                endpoint: None,
            },
            accept: vec![],
            decline: vec![],
        },
        false,
    )
    .await;
    assert!(matches!(refused, Err(Failure::Refused(_))), "{refused:?}");
    assert!(out.is_empty());
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    assert_eq!(lines(&out)[0]["status"], "unread");
}

// Case S: a card that changes nothing of its contact says `matches`, and
// an `--accept` naming it is refused before anything is read, so it stays
// unread; `--decline` closes it. Mutations: `matches` printed as
// `updates`; the refusal removed (the accept reaches the model, which
// offers Decline alone, and is refused after the read).
#[tokio::test]
async fn a_card_that_changes_nothing_says_it_matches_and_is_only_declined() {
    let (alice, bob, mut b_run) = shared().await;
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    let id = lines(&out)[0]["id"].as_str().expect("id").to_owned();
    let accept = |id: String| Verb::ProposalsRead {
        conversation: alice.route(),
        accept: vec![id],
        decline: vec![],
    };
    run(&mut b_run, accept(id), true).await.1.expect("accepted");
    // The same card again.
    let mut a_run = alice.session();
    let (_, shared) = run(&mut a_run, Verb::CardShare(bob.route()), true).await;
    shared.expect("shared again");
    a_run.close().await;
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    let held = lines(&out);
    assert_eq!(held.len(), 1, "{out}");
    assert_eq!(held[0]["accept"]["matches"], "Ada Lovelace", "{out}");
    let id = held[0]["id"].as_str().expect("id").to_owned();
    let (out, refused) = run(&mut b_run, accept(id.clone()), true).await;
    assert!(matches!(refused, Err(Failure::Refused(_))), "{refused:?}");
    assert!(out.is_empty(), "nothing read or printed: {out}");
    let (out, _) = run(&mut b_run, Verb::Proposals, true).await;
    assert_eq!(lines(&out)[0]["status"], "unread", "{out}");
    let (out, closed) = run(
        &mut b_run,
        Verb::ProposalsRead {
            conversation: alice.route(),
            accept: vec![],
            decline: vec![id],
        },
        true,
    )
    .await;
    closed.expect("declined");
    assert_eq!(lines(&out)[0]["status"], "declined");
}
