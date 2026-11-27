// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A store older than schema v10 reaches v14 with `other` as the kind of
//! every unread row, since v10 added the column with that default and left
//! the client to classify (STATE.md's v10 paragraph): v14's (e) discards
//! an unread row by its kind `card`, so it does not discard such a store's
//! unread cards. One whose route recorded a key is rebound to the id that
//! key derives (b), and stays an unread card signed over the label. Such a
//! row is no proposal and cannot be accepted: its signature does not
//! verify on the route the row now names, so the client classifies it as
//! any card that does not verify, a row this build cannot show, and Accept
//! of its row writes no contact.
//!
//! Every key here is TEST-ONLY (`interweave_test_support::endpoint_keys`).

#![allow(clippy::expect_used, clippy::panic)]

#[path = "../../store/tests/common/mod.rs"]
mod store_common;

use ed25519_dalek::Signer as _;
use interweave_human_app_core::{Command, FacadeSide, Failure, Update};
use interweave_human_chat_protocol::{CardDraft, CardRoute, build_card};
use interweave_human_store::{
    AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, StoreOptions,
};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{KeyView, ProposalContext};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::endpoint_keys::{endpoint_id, signing_key};
use interweave_transport_api::{MediaType, TransportIdentity};

const MEDIA_TYPE: &str = "application/vnd.interweave-human-chat+json;v=2";
const LABEL: &str = "ada-label";

fn node() -> FakeConfig {
    let id = endpoint_id("human");
    FakeConfig {
        peer: ProfileIdentity::generate()
            .transport_identity()
            .expect("peer id"),
        endpoints: vec![FakeEndpoint::open(id.clone(), false)],
        default_endpoint: Some(id),
        queue_bound: 16,
    }
}

fn id(n: u32) -> String {
    format!("{n:032x}")
}

/// Ada's card as `peer` sends it, signed by her key. `over_label` signs it
/// as the previous release did, over its sender's label in the route's
/// place; otherwise over the id her key derives, as this release does. The
/// canonical bytes end with the route's endpoint as `u16be(len) || bytes`
/// (chat-protocol's `canonical_card_bytes`), so the label's message is the
/// derived one with that last field replaced.
fn ada_card(n: u32, peer: &TransportIdentity, over_label: bool) -> Vec<u8> {
    let ada = endpoint_id("ada");
    let key = signing_key("ada");
    build_card(
        &id(n),
        CardDraft {
            name: "Ada",
            surname: None,
            photo: None,
        },
        key.verifying_key().to_bytes(),
        CardRoute {
            source_peer: peer,
            source_endpoint: &ada,
        },
        None,
        |message| {
            let tail = ada.as_str().len() + 2;
            let mut field = u16::try_from(ada.as_str().len())
                .expect("short")
                .to_be_bytes()
                .to_vec();
            field.extend_from_slice(ada.as_str().as_bytes());
            assert!(message.ends_with(&field), "control: the endpoint is last");
            if !over_label {
                return key.sign(message).to_bytes();
            }
            let mut label = message[..message.len() - tail].to_vec();
            label.extend_from_slice(&u16::try_from(LABEL.len()).expect("short").to_be_bytes());
            label.extend_from_slice(LABEL.as_bytes());
            key.sign(&label).to_bytes()
        },
    )
    .expect("a card")
    .into_bytes()
}

/// A file this build made, faked back to v9 (the store's own undo of
/// v15 to v10), holding Ada's contact, her route under her label with her
/// key recorded, and her card unread, signed over that label.
fn v9_store(path: &std::path::Path, peer: &TransportIdentity) {
    drop(HumanStore::open(path, &endpoint_id("human"), StoreOptions::default()).expect("a store"));
    let conn = rusqlite::Connection::open(path).expect("a second connection");
    conn.execute_batch(&format!(
        "{}PRAGMA user_version = 9;",
        store_common::UNDO_V10
    ))
    .expect("back to v9");
    let has_kind: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('unread_inbound') WHERE name = 'kind'",
            [],
            |r| r.get(0),
        )
        .expect("columns");
    assert_eq!(has_kind, 0, "control: a v9 row has no kind");
    conn.execute_batch(
        "INSERT INTO contacts (contact_id, display_name, created_at, updated_at)
         VALUES ('c1', 'Ada', 1, 1);",
    )
    .expect("a contact");
    conn.execute(
        "INSERT INTO contact_routes (contact_id, peer_id, endpoint_id, identity_key)
         VALUES ('c1', ?1, ?2, ?3)",
        rusqlite::params![
            peer.as_str(),
            LABEL,
            signing_key("ada").verifying_key().to_bytes().to_vec()
        ],
    )
    .expect("a keyed route");
    conn.execute(
        "INSERT INTO unread_inbound
             (app_message_id, source_peer, source_endpoint, media_type, payload, received_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        rusqlite::params![
            id(1),
            peer.as_str(),
            LABEL,
            MEDIA_TYPE,
            ada_card(1, peer, true)
        ],
    )
    .expect("her card, unread");
}

fn facade(node: &FakeNode, store: HumanStore) -> FacadeSide<FakeNode, FakeNode> {
    let client = TransportClient::new(
        node.clone(),
        node.clone(),
        store,
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![endpoint_id("human")],
            channels: vec![],
            max_payload_bytes: 49_152,
        },
        Box::new(|| 1),
        0,
    )
    .expect("facade");
    FacadeSide::new(client, || 1)
}

fn kind(path: &std::path::Path, n: u32) -> String {
    rusqlite::Connection::open(path)
        .expect("read back")
        .query_row(
            "SELECT kind FROM unread_inbound WHERE app_message_id = ?1",
            [id(n)],
            |r| r.get(0),
        )
        .expect("the row")
}

/// What no Accept was shown: any context will do, since a row that holds
/// no card this build verifies is refused before its context is judged.
fn some_context() -> ProposalContext {
    ProposalContext {
        updates: None,
        shares_node: vec![],
        same_name: vec![],
        key_matches: vec![],
        key: KeyView::New,
        fingerprint: String::new(),
        changes_nothing: false,
        values: interweave_human_ui_model::ValueChanges::default(),
    }
}

fn failure(updates: &[Update]) -> Option<Failure> {
    updates.iter().find_map(|u| match u {
        Update::Failed { why, .. } => Some(*why),
        _ => None,
    })
}

/// The pre-v10 card survives v14 under the derived id, is classified as
/// a card that does not verify -- a row this build cannot show, counted
/// as one -- and its Accept is refused with nothing written. The control:
/// the same card signed over the derived id, held beside it, is a card and
/// passes the verification its Accept starts with. Mutations: `kind_of`
/// taking any card-shaped row for a card, verified or not; the row's card
/// signed over the derived id instead of the label.
#[tokio::test]
async fn a_pre_v10_unread_card_signed_over_a_label_cannot_be_accepted() {
    let dir = store_common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let (a, b) = FakeNetwork::pair(node(), node());
    let peer = a.peer().clone();
    v9_store(&path, &peer);

    let mut store =
        HumanStore::open(&path, &endpoint_id("human"), StoreOptions::default()).expect("migrates");
    let discarded = store.discarded_at_v14().expect("read");
    assert_eq!(
        discarded.map(|d| d.card_proposals),
        None,
        "nothing went: (e) did not see the card by its kind"
    );
    let unread = store.unread_inbound().expect("unread");
    assert_eq!(unread.len(), 1);
    let label_row = unread[0].row_id;
    let ada = endpoint_id("ada");
    let rebound: Option<String> = rusqlite::Connection::open(&path)
        .expect("read back")
        .query_row(
            "SELECT source_endpoint FROM unread_inbound WHERE app_message_id = ?1",
            [id(1)],
            |r| r.get(0),
        )
        .expect("the row");
    assert_eq!(rebound.as_deref(), Some(ada.as_str()), "rebound by (b)");
    assert_eq!(kind(&path, 1), "other", "v10's default, carried");
    // The control, as this release receives it.
    let control_row = store
        .commit_unread_inbound(&NewInbound {
            app_message_id: AppMessageId::parse(id(2)).expect("an id"),
            origin: InboundOrigin {
                peer: peer.clone(),
                endpoint: Some(ada.clone()),
                channel: None,
            },
            media_type: Some(MediaType::parse(MEDIA_TYPE).expect("media type")),
            payload: ada_card(2, &peer, false),
            received_at: 2,
            kind: InboundKind::Other,
        })
        .expect("the control's card");

    let mut facade = facade(&b, store);
    let start = facade.start().expect("started");
    assert_eq!(
        kind(&path, 1),
        "other",
        "classified: no card this build verifies"
    );
    assert_eq!(kind(&path, 2), "card", "control: classified a card");
    assert_eq!(
        start.undecodable_unread, 1,
        "shown as any card that does not verify: a row this build cannot show"
    );

    let refused = facade
        .execute(
            Command::AcceptCard {
                row: label_row,
                shown: Box::new(some_context()),
            },
            3,
        )
        .await;
    assert_eq!(failure(&refused), Some(Failure::NotACard), "{refused:?}");
    let control = facade
        .execute(
            Command::AcceptCard {
                row: control_row,
                shown: Box::new(some_context()),
            },
            4,
        )
        .await;
    assert_eq!(
        failure(&control),
        Some(Failure::ProposalChanged),
        "control: a verifying card reaches the judgement of its context: {control:?}"
    );
    Box::pin(facade.close()).await;
    drop(facade);

    let store =
        HumanStore::open(&path, &endpoint_id("human"), StoreOptions::default()).expect("reopen");
    let contacts = store.contacts().expect("contacts");
    assert_eq!(contacts.len(), 1, "no contact written");
    assert_eq!(contacts[0].display_name, "Ada");
    let routes = &contacts[0].routes;
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].endpoint, ada, "the route as v14 rebound it");
}
