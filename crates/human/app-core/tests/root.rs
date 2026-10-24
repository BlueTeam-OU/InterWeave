// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The headless root over the fake network: a real facade and a real
//! store, a scripted surface in place of a window. What the person does is
//! given as the `ViewEvent`s a view would hand over; what they see is read
//! from the model.
//!
//! What this does NOT prove: a real daemon, a real window, a real
//! platform. Those are the desktop end-to-end suite's.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use interweave_human_app_core::{
    CANCELLED_CAP, Command, FacadeSide, Failure, ModelSide, Opener, Problem, Surface, Update,
};
use interweave_human_store::{HumanStore, InboundOrigin, NewInbound, RowId, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{
    Announce, Confirmation, ConversationKey, Intent, LabelKey, RowAction, Table, TrustChange,
    TrustInput, TrustOutcome, UiModel, ViewEvent,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportError, TransportIdentity};

const WALL: u64 = 1_786_600_000_000;

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn endpoint(name: &str) -> EndpointId {
    EndpointId::parse(name).expect("valid")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: peer(),
        endpoints: vec![FakeEndpoint::open(endpoint("human"), false)],
        default_endpoint: Some(endpoint("human")),
        queue_bound: 16,
    }
}

fn facade(node: &FakeNode, store: HumanStore) -> FacadeSide<FakeNode, FakeNode> {
    let client = TransportClient::new(
        node.clone(),
        node.clone(),
        store,
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![endpoint("human")],
            channels: vec![],
            max_payload_bytes: 49_152,
        },
        Box::new(|| WALL),
        0,
    )
    .expect("facade");
    FacadeSide::new(client, || WALL)
}

fn memory() -> HumanStore {
    HumanStore::open_in_memory(StoreOptions::default()).expect("store")
}

/// A view whose person acts from a script: each `take_events` hands over
/// the next batch, and a render is counted.
#[derive(Default)]
struct Scripted {
    batches: VecDeque<Vec<ViewEvent>>,
    renders: usize,
    /// Whether each render was shown a trust change to confirm.
    pending_shown: Vec<bool>,
    /// Peers as the last render was shown it.
    peers_shown: Option<interweave_human_ui_model::Peers>,
}

impl Scripted {
    fn will(&mut self, events: Vec<ViewEvent>) {
        self.batches.push_back(events);
    }
}

impl Surface for Scripted {
    fn render(&mut self, model: &UiModel) {
        self.renders += 1;
        self.pending_shown
            .push(model.trust_settings().pending().is_some());
        self.peers_shown = Some(model.peers().clone());
    }

    fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
        self.batches.pop_front().unwrap_or_default()
    }
}

/// Records what it was asked to open; a network-access ask as
/// [`NETWORK_ASKED`].
#[derive(Clone, Default)]
struct Opened(Rc<RefCell<Vec<String>>>);

/// What [`Opened`] records for an ask for network access: no link has
/// this shape.
const NETWORK_ASKED: &str = "<network access>";

/// What [`Opened`] records for an ask for the person's authentication.
const AUTHENTICATION_ASKED: &str = "<authentication>";

impl Opener for Opened {
    fn open(&mut self, destination: &str) {
        self.0.borrow_mut().push(destination.to_owned());
    }

    fn ask_network_access(&mut self) {
        self.0.borrow_mut().push(NETWORK_ASKED.to_owned());
    }

    fn authenticate(&mut self) {
        self.0.borrow_mut().push(AUTHENTICATION_ASKED.to_owned());
    }
}

/// A root: both sides, and the loop between them.
struct Root {
    facade: FacadeSide<FakeNode, FakeNode>,
    model: ModelSide<Scripted, Opened>,
    commands: Vec<Command>,
    updates: Vec<Update>,
}

impl Root {
    fn new(facade: FacadeSide<FakeNode, FakeNode>) -> (Self, Opened) {
        let opened = Opened::default();
        let mut root = Self {
            facade,
            model: ModelSide::new(Scripted::default(), opened.clone()),
            commands: Vec::new(),
            updates: Vec::new(),
        };
        let listing = root.facade.listing().expect("listing");
        root.model.apply(Update::Listed(listing));
        (root, opened)
    }

    /// One pass of the loop: the facade's turn, the model side's turn,
    /// and every command it asked for -- then the facade's turn again,
    /// as both roots run one after the commands they carried out: a send,
    /// a retry or a restore is attempted there, not by its command.
    async fn pump(&mut self, now: u64) {
        self.facade_turn(now).await;
        let commands = self.model.turn();
        self.commands.extend(commands.iter().cloned());
        let any = !commands.is_empty();
        for command in commands {
            for update in self.facade.execute(command, now).await {
                self.updates.push(update.clone());
                self.model.apply(update);
            }
        }
        if any {
            self.facade_turn(now).await;
        }
    }

    async fn facade_turn(&mut self, now: u64) {
        for update in self.facade.turn(now).await {
            self.updates.push(update.clone());
            self.model.apply(update);
        }
    }

    /// Pump until the session is ready and the reads it owes at start
    /// -- the trust and address lists, for the no-reach notice -- are
    /// answered, then forget what was asked: a test of what a person's
    /// input asks starts from here.
    async fn settled(&mut self, now: u64) {
        for _ in 0..4 {
            self.pump(now).await;
        }
        assert!(
            self.commands.contains(&Command::ReadTrust)
                && self.commands.contains(&Command::ReadAddresses),
            "{:?}",
            self.commands
        );
        self.commands.clear();
        self.updates.clear();
    }

    fn will(&mut self, events: Vec<ViewEvent>) {
        self.model.surface_mut().will(events);
    }

    fn model(&self) -> &UiModel {
        self.model.model()
    }
}

fn direct(to: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: to.clone(),
        endpoint: None,
    }
}

fn send(key: &ConversationKey, text: &str) -> ViewEvent {
    ViewEvent::Intent(Intent::Send {
        key: key.clone(),
        draft: text.to_owned(),
    })
}

#[tokio::test]
async fn a_send_goes_out_and_reads_as_accepted_never_as_read() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, _) = Root::new(facade(&a, memory()));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;

    let to_bob = direct(b.peer());
    alice.will(vec![send(&to_bob, "hello bob")]);
    alice.pump(1).await;
    alice.pump(2).await;

    let items = alice.model().messages(&to_bob);
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].source, "hello bob");
    assert_eq!(items[0].label, LabelKey::AcceptedByRemoteTransport);

    bob.pump(3).await;
    let from_alice: Vec<String> = bob
        .model()
        .conversations()
        .iter()
        .flat_map(|c| bob.model().messages(&c.key))
        .map(|m| m.source)
        .collect();
    assert_eq!(from_alice, ["hello bob"]);
}

/// A sent message names the endpoint the facade leases, not the first it
/// was configured with: here the runtime has the first disabled, so the
/// open leased the second (ADR-0028 A 2026-10-11).
#[tokio::test]
async fn a_send_names_the_endpoint_leased_not_the_first_configured() {
    use interweave_local_client_api::{AdminBinding, AdminCapability, AdminPort};
    let mut config = node();
    config
        .endpoints
        .push(FakeEndpoint::open(endpoint("human-spare"), false));
    let (a, b) = FakeNetwork::pair(config, node());
    a.admin(std::collections::BTreeSet::from([
        AdminCapability::Endpoints,
    ]))
    .await
    .expect("admin")
    .set_endpoint_enabled(endpoint("human"), false)
    .await
    .expect("disabled");
    let client = TransportClient::new(
        a.clone(),
        a.clone(),
        memory(),
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![endpoint("human"), endpoint("human-spare")],
            channels: vec![],
            max_payload_bytes: 49_152,
        },
        Box::new(|| WALL),
        0,
    )
    .expect("facade");
    let (mut alice, _) = Root::new(FacadeSide::new(client, || WALL));
    alice.pump(0).await;
    alice.will(vec![send(&direct(b.peer()), "from the spare")]);
    alice.pump(1).await;
    let named: Vec<Option<EndpointId>> = alice
        .updates
        .iter()
        .filter_map(|u| match u {
            Update::Sent { envelope, .. } => Some(envelope.from_endpoint.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(named, [Some(endpoint("human-spare"))]);
}

#[tokio::test]
async fn a_second_send_while_one_is_in_flight_is_not_sent_twice() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, _) = Root::new(facade(&a, memory()));
    alice.pump(0).await;
    let to_bob = direct(b.peer());
    // Two presses of Send in one take, before either is answered.
    alice.will(vec![send(&to_bob, "once"), send(&to_bob, "once")]);
    alice.pump(1).await;
    let sends = alice
        .commands
        .iter()
        .filter(|c| matches!(c, Command::Send { .. }))
        .count();
    assert_eq!(sends, 1, "{:?}", alice.commands);
    // Answered, the same action can be taken again.
    alice.will(vec![send(&to_bob, "twice")]);
    alice.pump(2).await;
    let sends = alice
        .commands
        .iter()
        .filter(|c| matches!(c, Command::Send { .. }))
        .count();
    assert_eq!(sends, 2, "{:?}", alice.commands);
}

#[tokio::test]
async fn read_then_keep_then_unkeep_then_keep_again_reaches_the_store() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let (mut alice, _) = Root::new(facade(
        &a,
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
    ));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    bob.will(vec![send(&direct(a.peer()), "keep me")]);
    bob.pump(1).await;
    alice.pump(2).await;

    let from_bob = alice.model().conversations()[0].key.clone();
    let item = alice.model().messages(&from_bob)[0].key;
    let unread_row = match alice
        .model()
        .conversation_viewed(&from_bob, true)
        .as_slice()
    {
        [Intent::MarkRead(row)] => *row,
        other => panic!("one unread row: {other:?}"),
    };

    alice.will(vec![ViewEvent::Intent(Intent::MarkRead(unread_row))]);
    alice.pump(3).await;
    assert_eq!(
        alice.model().actions(item),
        [Intent::Keep {
            item,
            from: (Table::Unread, unread_row)
        }]
    );

    let keep = alice.model().actions(item)[0].clone();
    alice.will(vec![ViewEvent::Intent(keep)]);
    alice.pump(4).await;
    let kept_row = match alice.model().actions(item).as_slice() {
        [Intent::Unkeep(row)] => *row,
        other => panic!("kept: {other:?}"),
    };

    alice.will(vec![ViewEvent::Intent(Intent::Unkeep(kept_row))]);
    alice.pump(5).await;
    let keep_again = alice.model().actions(item);
    assert_eq!(
        keep_again,
        [Intent::Keep {
            item,
            from: (Table::Kept, kept_row)
        }],
        "Keep is offered again in the session, from the unkeep's copy"
    );
    alice.will(vec![ViewEvent::Intent(keep_again[0].clone())]);
    alice.pump(6).await;
    assert!(
        matches!(alice.model().actions(item).as_slice(), [Intent::Unkeep(_)]),
        "kept again"
    );
    assert!(
        !alice
            .updates
            .iter()
            .any(|u| matches!(u, Update::Failed { .. })),
        "{:?}",
        alice.updates
    );
    alice.facade.close().await;
    drop(alice);

    let store = HumanStore::open(&path, StoreOptions::default()).expect("reopen");
    let kept = store.kept_inbound().expect("kept");
    assert_eq!(kept.len(), 1, "durable after the second Keep");
    assert!(store.unread_inbound().expect("unread").is_empty());
}

#[tokio::test]
async fn a_keep_with_no_copy_held_fails_and_changes_nothing() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let (mut alice, _) = Root::new(facade(
        &a,
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
    ));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    bob.will(vec![send(&direct(a.peer()), "unread")]);
    bob.pump(1).await;
    alice.pump(2).await;
    let from_bob = alice.model().conversations()[0].key.clone();
    let item = alice.model().messages(&from_bob)[0].key;
    let unread_row = match alice
        .model()
        .conversation_viewed(&from_bob, true)
        .as_slice()
    {
        [Intent::MarkRead(row)] => *row,
        other => panic!("one unread row: {other:?}"),
    };
    // The state a dropped copy leaves: the model has the message read and
    // offers Keep, while the facade side holds no copy of it.
    alice.model.apply(Update::Read(unread_row));
    let keep = Command::Keep {
        item,
        from: (Table::Unread, unread_row),
    };
    assert_eq!(
        alice.model().actions(item),
        [Intent::Keep {
            item,
            from: (Table::Unread, unread_row)
        }],
        "the control: Keep is offered before the failure"
    );
    let updates = alice.facade.execute(keep.clone(), 3).await;
    assert_eq!(
        updates,
        [Update::Failed {
            command: keep.clone(),
            why: Failure::CopyGone
        }]
    );
    for update in updates {
        alice.model.apply(update);
    }
    assert!(
        alice.model().actions(item).is_empty(),
        "Keep is no longer offered: {:?}",
        alice.model().actions(item)
    );
    assert_eq!(
        alice.model.take_problems(),
        [Problem::Command {
            command: keep,
            why: Failure::CopyGone
        }],
        "and the root is told"
    );
    alice.facade.close().await;
    drop(alice);
    let store = HumanStore::open(&path, StoreOptions::default()).expect("reopen");
    assert!(
        store.kept_inbound().expect("kept").is_empty(),
        "nothing kept"
    );
    assert_eq!(
        store.unread_inbound().expect("unread").len(),
        1,
        "still unread"
    );
}

#[tokio::test]
async fn a_link_is_opened_only_when_allowlisted_and_never_reaches_the_facade() {
    let (a, _b) = FakeNetwork::pair(node(), node());
    let (mut alice, opened) = Root::new(facade(&a, memory()));
    alice.settled(0).await;
    alice.will(vec![
        ViewEvent::Intent(Intent::OpenLink("https://example.org/a".to_owned())),
        ViewEvent::Intent(Intent::OpenLink("javascript:alert(1)".to_owned())),
    ]);
    alice.pump(0).await;
    assert_eq!(*opened.0.borrow(), ["https://example.org/a"]);
    assert!(alice.commands.is_empty(), "{:?}", alice.commands);
}

/// The ask for network access is the platform's, as a link is: it reaches
/// the opener and never the facade, and while access is withheld a draft
/// is not sent.
#[tokio::test]
async fn an_ask_for_network_access_reaches_the_platform_and_never_the_facade() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, opened) = Root::new(facade(&a, memory()));
    alice.settled(0).await;
    let to_bob = direct(b.peer());
    alice.model.network_access(false);
    alice.will(vec![
        ViewEvent::DraftChanged {
            key: to_bob.clone(),
            draft: "typed".to_owned(),
        },
        ViewEvent::Intent(Intent::AllowNetwork),
    ]);
    alice.pump(1).await;
    assert_eq!(*opened.0.borrow(), [NETWORK_ASKED]);
    assert!(alice.commands.is_empty(), "{:?}", alice.commands);
    assert_eq!(
        alice.model().send_draft(&to_bob),
        None,
        "nothing to send while access is withheld"
    );
}

#[tokio::test]
async fn a_draft_edit_reaches_the_model_before_a_send_in_the_same_take() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, _) = Root::new(facade(&a, memory()));
    alice.pump(0).await;
    let to_bob = direct(b.peer());
    alice.will(vec![ViewEvent::DraftChanged {
        key: to_bob.clone(),
        draft: "typed".to_owned(),
    }]);
    alice.pump(1).await;
    assert_eq!(alice.model().composer(&to_bob).draft, "typed");
}

#[tokio::test]
async fn the_store_is_listed_at_start() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let (a, b) = FakeNetwork::pair(node(), node());
    // A message that stays pending: the remote is busy.
    a.inject_send(TransportError::Overloaded);
    let (mut alice, _) = Root::new(facade(
        &a,
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
    ));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    alice.will(vec![send(&direct(b.peer()), "still pending")]);
    alice.pump(1).await;
    bob.will(vec![send(&direct(a.peer()), "left unread")]);
    bob.pump(2).await;
    alice.pump(3).await;
    alice.facade.close().await;
    drop(alice);

    let (a2, _b2) = FakeNetwork::pair(node(), node());
    let (restarted, _) = Root::new(facade(
        &a2,
        HumanStore::open(&path, StoreOptions::default()).expect("reopen"),
    ));
    let mut texts: Vec<String> = restarted
        .model()
        .conversations()
        .iter()
        .flat_map(|c| restarted.model().messages(&c.key))
        .map(|m| m.source)
        .collect();
    texts.sort();
    assert_eq!(texts, ["left unread", "still pending"]);
}

/// A store failure behind a command changes nothing in the model and is
/// reported with its class: a read of a row that is not there, and an
/// unkeep of a kept row the build cannot decode.
#[tokio::test]
async fn a_store_failure_changes_nothing_in_the_model_and_says_why() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let (mut alice, _) = Root::new(facade(
        &a,
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
    ));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    bob.will(vec![send(&direct(a.peer()), "one")]);
    bob.pump(1).await;
    alice.pump(2).await;
    let from_bob = alice.model().conversations()[0].key.clone();
    let item = alice.model().messages(&from_bob)[0].key;

    let missing = Command::MarkRead(RowId::from_stored(999));
    let updates = alice.facade.execute(missing.clone(), 3).await;
    assert_eq!(
        updates,
        [Update::Failed {
            command: missing,
            why: Failure::NoSuchRow
        }],
        "no Read reaches the model"
    );

    // Read and keep it, then make the kept row undecodable.
    let unread_row = match alice
        .model()
        .conversation_viewed(&from_bob, true)
        .as_slice()
    {
        [Intent::MarkRead(row)] => *row,
        other => panic!("one unread row: {other:?}"),
    };
    alice.will(vec![ViewEvent::Intent(Intent::MarkRead(unread_row))]);
    alice.pump(4).await;
    let keep = alice.model().actions(item)[0].clone();
    alice.will(vec![ViewEvent::Intent(keep)]);
    alice.pump(5).await;
    let kept_row = match alice.model().actions(item).as_slice() {
        [Intent::Unkeep(row)] => *row,
        other => panic!("kept: {other:?}"),
    };
    let conn = rusqlite::Connection::open(&path).expect("open");
    conn.execute("UPDATE kept_inbound SET source_peer = 'not-a-peer'", [])
        .expect("corrupt");
    drop(conn);

    let unkeep = Command::Unkeep(kept_row);
    let updates = alice.facade.execute(unkeep.clone(), 6).await;
    assert_eq!(
        updates,
        [Update::Failed {
            command: unkeep,
            why: Failure::Corrupt
        }]
    );
    for update in updates {
        alice.model.apply(update);
    }
    assert_eq!(
        alice.model().actions(item),
        [Intent::Unkeep(kept_row)],
        "the model still shows it kept: no Unkept was applied"
    );
}

/// A stored row that cannot be decoded is held, never shown, and counted.
#[tokio::test]
async fn an_undecodable_stored_row_is_not_shown_and_is_counted() {
    let (a, _b) = FakeNetwork::pair(node(), node());
    let mut store = memory();
    let origin = InboundOrigin {
        peer: peer(),
        endpoint: Some(endpoint("human")),
        channel: None,
    };
    let media = interweave_transport_api::MediaType::parse(
        "application/vnd.interweave-human-chat+json;v=2",
    )
    .expect("media type");
    for (id, payload) in [
        ("0000000000000000000000000000000a", b"not json".to_vec()),
        (
            "0000000000000000000000000000000b",
            br#"{"v":2,"kind":"text","app_message_id":"0000000000000000000000000000000b","text":"shown"}"#
                .to_vec(),
        ),
    ] {
        store
            .commit_unread_inbound(&NewInbound {
                app_message_id: interweave_human_store::AppMessageId::parse(id.to_owned())
                    .expect("id"),
                origin: origin.clone(),
                media_type: Some(media.clone()),
                payload,
                received_at: 1,
            })
            .expect("committed");
    }
    let mut side = facade(&a, store);
    let listing = side.listing().expect("listing");
    assert_eq!(listing.undecodable_unread, 1);
    assert_eq!(listing.unread.len(), 1);
    let (root, _) = Root::new(side);
    assert_eq!(root.model.hidden_rows(), 1);
    let texts: Vec<String> = root
        .model()
        .conversations()
        .iter()
        .flat_map(|c| root.model().messages(&c.key))
        .map(|m| m.source)
        .collect();
    assert_eq!(texts, ["shown"]);
}

/// The model side records the press when it issues a send, so the
/// facade's answer clears an unedited composer and keeps one edited since.
#[tokio::test]
async fn a_sent_message_clears_the_composer_only_if_it_was_not_edited_since() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, _) = Root::new(facade(&a, memory()));
    alice.pump(0).await;
    let to_bob = direct(b.peer());
    alice.will(vec![
        ViewEvent::DraftChanged {
            key: to_bob.clone(),
            draft: "hi".to_owned(),
        },
        send(&to_bob, "hi"),
    ]);
    alice.pump(1).await;
    assert_eq!(
        alice.model().composer(&to_bob).draft,
        "",
        "unedited: cleared"
    );
}

fn trust(input: TrustInput) -> Vec<ViewEvent> {
    vec![ViewEvent::Trust(input)]
}

/// The trust settings through the root: opening reads the daemon's
/// allowlist, a proposal reaches nothing, and only a confirmation sends
/// the change -- which the daemon then holds, read back.
#[tokio::test]
async fn a_trust_change_reaches_the_daemon_only_once_confirmed() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&a, memory()));
    root.pump(0).await;

    root.will(trust(TrustInput::Opened));
    root.pump(1).await;
    let list = root.model().trust_settings().list().cloned().expect("read");
    assert_eq!(list.local_peer.as_ref(), Some(a.peer()));
    assert_eq!(
        list.peers().cloned().collect::<Vec<_>>(),
        vec![b.peer().clone()]
    );

    let stranger = peer();
    root.will(trust(TrustInput::EntryChanged(
        stranger.as_str().to_owned(),
    )));
    root.will(trust(TrustInput::ProposeAllow));
    root.pump(2).await;
    let set = |c: &Command| matches!(c, Command::SetTrust(_));
    assert!(!root.commands.iter().any(set), "a proposal sends nothing");
    assert!(root.model().trust_settings().pending().is_some());

    let change = TrustChange {
        peer: stranger.clone(),
        allowed: true,
    };
    root.will(trust(TrustInput::Confirm(change.clone())));
    root.pump(3).await;
    assert!(root.commands.contains(&Command::SetTrust(change.clone())));
    let settings = root.model().trust_settings();
    assert_eq!(settings.outcome().0, Some(&TrustOutcome::Changed(change)));
    assert!(
        settings.list().is_some_and(|l| l.allows(&stranger)),
        "the daemon holds it, read back"
    );

    root.will(trust(TrustInput::ProposeRevoke(b.peer().clone())));
    root.will(trust(TrustInput::Confirm(TrustChange {
        peer: b.peer().clone(),
        allowed: false,
    })));
    root.pump(4).await;
    assert!(
        root.model()
            .trust_settings()
            .list()
            .is_some_and(|l| !l.allows(b.peer())),
        "revoked, read back"
    );
}

/// A trust input that asks the facade nothing still changes what the
/// settings show, so the same turn renders it: a proposal is on screen
/// to confirm without waiting for an unrelated event.
#[tokio::test]
async fn a_trust_proposal_is_rendered_in_the_turn_that_took_it() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&a, memory()));
    root.will(trust(TrustInput::Opened));
    root.pump(0).await;
    root.pump(1).await;
    root.will(trust(TrustInput::ProposeRevoke(b.peer().clone())));
    root.model.turn();
    assert_eq!(
        root.model.surface().pending_shown.last(),
        Some(&true),
        "the turn's last render showed the proposal"
    );
}

/// A Hide asks the facade nothing, so no update brings another turn: the
/// turn that took it renders it, as it does a trust input (seen on the
/// phone: the row held slid out and no snackbar until the next turn).
#[tokio::test]
async fn a_hide_is_rendered_in_the_turn_that_took_it() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, _) = Root::new(facade(&a, memory()));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    bob.will(vec![send(&direct(a.peer()), "hide me")]);
    bob.pump(1).await;
    alice.pump(2).await;
    let from_bob = alice.model().conversations()[0].key.clone();
    let item = alice.model().messages(&from_bob)[0].key;
    let read = alice.model().conversation_viewed(&from_bob, true);
    alice.will(read.into_iter().map(ViewEvent::Intent).collect());
    alice.pump(3).await;

    alice.will(vec![ViewEvent::Intent(Intent::Hide(item))]);
    let before = alice.model.surface().renders;
    alice.model.turn();
    assert_eq!(
        alice.model.surface().renders,
        before + 2,
        "the turn's own render, and one more once the Hide was taken"
    );
    assert!(alice.model().snackbar().is_some());
}

/// A trust intent handed over bare -- by a surface that built one rather
/// than confirming the change shown -- reaches nothing: trust changes only
/// through the settings' inputs.
#[tokio::test]
async fn a_trust_intent_handed_over_bare_reaches_nothing() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&a, memory()));
    root.settled(0).await;
    root.will(vec![
        ViewEvent::Intent(Intent::ReadTrust),
        ViewEvent::Intent(Intent::SetTrust(TrustChange {
            peer: b.peer().clone(),
            allowed: false,
        })),
    ]);
    root.pump(0).await;
    assert!(
        !root
            .commands
            .iter()
            .any(|c| matches!(c, Command::ReadTrust | Command::SetTrust(_))),
        "{:?}",
        root.commands
    );
}

/// A change made whose list was not read back -- or one the daemon did not
/// confirm -- leaves the list shown possibly stale: the next turn reads it
/// again, unasked.
#[tokio::test]
async fn a_change_not_read_back_is_read_again_by_the_next_turn() {
    use interweave_human_client_api::{TrustProblem, TrustSetFailure};
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&a, memory()));
    root.will(trust(TrustInput::Opened));
    root.pump(0).await;
    let change = TrustChange {
        peer: b.peer().clone(),
        allowed: false,
    };
    root.will(trust(TrustInput::ProposeRevoke(b.peer().clone())));
    root.will(trust(TrustInput::Confirm(change.clone())));
    let commands = root.model.turn();
    assert_eq!(commands, vec![Command::SetTrust(change.clone())]);
    root.model.apply(Update::TrustSet {
        change,
        answer: Err(TrustSetFailure::MadeNotReadBack(TrustProblem::Unavailable)),
    });
    root.model.apply(Update::Done(commands[0].clone()));
    assert_eq!(root.model.turn(), vec![Command::ReadTrust], "read again");
}

/// The snackbar now up, which the scripted person answers by its id.
fn snack(root: &Root) -> u64 {
    root.model().snackbar().expect("a snackbar").id
}

#[tokio::test]
async fn an_undone_cancel_goes_out_as_the_same_message_once() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, _) = Root::new(facade(&a, memory()));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    // Not taken, so the row stays pending to be cancelled.
    b.inject_send(TransportError::Overloaded);
    let to_alice = direct(a.peer());
    bob.will(vec![send(&to_alice, "take it back")]);
    bob.pump(1).await;
    let item = bob.model().messages(&to_alice)[0].key;
    let Some(Intent::Cancel(row)) = bob.model().intent_for(item, RowAction::Cancel) else {
        panic!("Cancel is offered on a pending row");
    };

    bob.will(vec![ViewEvent::Intent(Intent::Cancel(row))]);
    bob.pump(2).await;
    assert!(bob.model().messages(&to_alice).is_empty(), "the row left");
    let id = snack(&bob);
    assert!(bob.model().snackbar().expect("up").undo);

    bob.will(vec![ViewEvent::Undo(id)]);
    bob.pump(3).await;
    let restored = match bob.updates.iter().rev().find_map(|u| match u {
        Update::Restored { cancelled, row } => Some((*cancelled, *row)),
        _ => None,
    }) {
        Some((cancelled, restored)) => {
            assert_eq!(cancelled, row);
            restored
        }
        None => panic!("restored: {:?}", bob.updates),
    };
    assert_ne!(restored, row);
    assert_eq!(
        bob.model()
            .messages(&to_alice)
            .iter()
            .map(|m| m.key)
            .collect::<Vec<_>>(),
        [item],
        "back in its place, under its key"
    );

    // The restore's attempt went out; Alice holds one message.
    alice.pump(4).await;
    let from_bob = alice.model().conversations()[0].key.clone();
    let shown: Vec<_> = alice
        .model()
        .messages(&from_bob)
        .into_iter()
        .map(|m| m.source)
        .collect();
    assert_eq!(shown, ["take it back"]);
    assert!(
        !bob.updates
            .iter()
            .any(|u| matches!(u, Update::Failed { .. })),
        "{:?}",
        bob.updates
    );
}

#[tokio::test]
async fn a_hidden_message_lets_go_of_its_copy_when_the_snackbar_closes() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, _) = Root::new(facade(&a, memory()));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    bob.will(vec![send(&direct(a.peer()), "hide me")]);
    bob.pump(1).await;
    alice.pump(2).await;
    let from_bob = alice.model().conversations()[0].key.clone();
    let item = alice.model().messages(&from_bob)[0].key;
    let read = alice.model().conversation_viewed(&from_bob, true);
    alice.will(read.into_iter().map(ViewEvent::Intent).collect());
    alice.pump(3).await;
    let Some(Intent::Keep { from, .. }) = alice.model().intent_for(item, RowAction::Keep) else {
        panic!("Keep offered after read");
    };

    alice.will(vec![ViewEvent::Intent(Intent::Hide(item))]);
    alice.pump(4).await;
    assert!(alice.model().messages(&from_bob).is_empty());
    assert!(
        !alice.commands.contains(&Command::ForgetCopy(from)),
        "held while its Undo is offered"
    );
    let id = snack(&alice);
    alice.will(vec![ViewEvent::SnackbarClosed(id)]);
    alice.pump(5).await;
    assert!(alice.commands.contains(&Command::ForgetCopy(from)));
    // A Keep raised now, from a stale view, is not issued: the message is
    // gone from the model.
    alice.will(vec![ViewEvent::Intent(Intent::Keep { item, from })]);
    alice.pump(6).await;
    assert!(
        !alice
            .commands
            .iter()
            .any(|c| matches!(c, Command::Keep { .. }))
    );
    // And the facade side holds no copy to keep any more.
    let updates = alice.facade.execute(Command::Keep { item, from }, 7).await;
    assert!(updates.iter().any(|u| matches!(
        u,
        Update::Failed {
            why: Failure::CopyGone,
            ..
        }
    )));
}

#[tokio::test]
async fn an_undone_unkeep_is_kept_in_the_store() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let (mut alice, _) = Root::new(facade(
        &a,
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
    ));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    bob.will(vec![send(&direct(a.peer()), "keep me")]);
    bob.pump(1).await;
    alice.pump(2).await;
    let from_bob = alice.model().conversations()[0].key.clone();
    let item = alice.model().messages(&from_bob)[0].key;
    let read = alice.model().conversation_viewed(&from_bob, true);
    alice.will(read.into_iter().map(ViewEvent::Intent).collect());
    alice.pump(3).await;
    let keep = alice
        .model()
        .intent_for(item, RowAction::Keep)
        .expect("offered");
    alice.will(vec![ViewEvent::Intent(keep)]);
    alice.pump(4).await;
    let unkeep = alice
        .model()
        .intent_for(item, RowAction::Unkeep)
        .expect("offered");
    alice.will(vec![ViewEvent::Intent(unkeep)]);
    alice.pump(5).await;
    let id = snack(&alice);
    alice.will(vec![ViewEvent::Undo(id)]);
    alice.pump(6).await;
    assert!(alice.model().snackbar().is_none());
    assert!(
        !alice
            .updates
            .iter()
            .any(|u| matches!(u, Update::Failed { .. })),
        "{:?}",
        alice.updates
    );
    alice.facade.close().await;
    drop(alice);

    let store = HumanStore::open(&path, StoreOptions::default()).expect("reopen");
    assert_eq!(store.kept_inbound().expect("kept").len(), 1, "kept again");
}

/// Pending rows to `to` that stay pending: every attempt is refused as
/// not taken.
async fn pending_rows(
    root: &mut Root,
    node: &FakeNode,
    to: &TransportIdentity,
    n: usize,
    now: u64,
) -> Vec<RowId> {
    let mut rows = Vec::new();
    for i in 0..n {
        node.inject_send(TransportError::Overloaded);
        root.will(vec![send(&direct(to), &format!("pending {i}"))]);
        root.pump(now).await;
        let row = root
            .updates
            .iter()
            .rev()
            .find_map(|u| match u {
                Update::Sent { row, .. } => Some(*row),
                _ => None,
            })
            .expect("sent");
        rows.push(row);
    }
    rows
}

/// A cancel can run before the Undo of the snackbar it replaces: the
/// offered Undo still finds its record (review F2 on #289).
#[tokio::test]
async fn an_undo_of_cancel_survives_a_cancel_queued_before_it() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut bob, _) = Root::new(facade(&b, memory()));
    bob.pump(0).await;
    let rows = pending_rows(&mut bob, &b, a.peer(), 2, 1).await;
    bob.will(vec![ViewEvent::Intent(Intent::Cancel(rows[0]))]);
    bob.pump(2).await;
    let first = snack(&bob);
    assert!(bob.model().snackbar().expect("up").undo);
    // In one take: cancel the second, then undo the first.
    bob.will(vec![
        ViewEvent::Intent(Intent::Cancel(rows[1])),
        ViewEvent::Undo(first),
    ]);
    bob.pump(3).await;
    assert!(
        bob.updates.iter().any(|u| matches!(
            u,
            Update::Restored { cancelled, .. } if *cancelled == rows[0]
        )),
        "{:?}",
        bob.updates
    );
}

/// Swipe spec section 11 item 19 (as #286 carries it): while a Cancel
/// waits for the facade, a newer action's snackbar stays with its Undo
/// when the Cancel's success comes, and the Cancel is only announced.
/// The facade is slowed by holding the first Cancel's command back.
#[tokio::test]
async fn a_cancels_late_success_leaves_a_newer_snackbar_and_its_undo() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut bob, _) = Root::new(facade(&b, memory()));
    bob.pump(0).await;
    let rows = pending_rows(&mut bob, &b, a.peer(), 2, 1).await;
    bob.will(vec![ViewEvent::Intent(Intent::Cancel(rows[0]))]);
    let held = bob.model.turn();
    assert_eq!(held, [Command::Cancel(rows[0])]);
    // The person acts again while it waits.
    bob.will(vec![ViewEvent::Intent(Intent::Cancel(rows[1]))]);
    bob.pump(2).await;
    let newer = bob.model().snackbar().expect("the second cancel's");
    assert!(newer.undo, "control: the newer snackbar offers Undo");
    for command in held {
        for update in bob.facade.execute(command, 3).await {
            bob.model.apply(update);
        }
    }
    assert_eq!(bob.model().snackbar(), Some(newer.clone()), "it stays");
    assert!(matches!(
        bob.model().outcome().and_then(|o| o.announce),
        Some(Announce::Confirmed(Confirmation::Cancelled { .. }))
    ));
    // Its Undo still restores the second.
    bob.will(vec![ViewEvent::Undo(newer.id)]);
    bob.pump(4).await;
    assert!(
        bob.updates.iter().any(|u| matches!(
            u,
            Update::Restored { cancelled, .. } if *cancelled == rows[1]
        )),
        "{:?}",
        bob.updates
    );
}

/// The control: once the newer snackbar has closed, the late Cancel shows
/// its own, with its Undo.
#[tokio::test]
async fn a_cancels_late_success_after_the_newer_snackbar_closed_shows_its_own() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut bob, _) = Root::new(facade(&b, memory()));
    bob.pump(0).await;
    let rows = pending_rows(&mut bob, &b, a.peer(), 2, 1).await;
    bob.will(vec![ViewEvent::Intent(Intent::Cancel(rows[0]))]);
    let held = bob.model.turn();
    bob.will(vec![ViewEvent::Intent(Intent::Cancel(rows[1]))]);
    bob.pump(2).await;
    let newer = snack(&bob);
    bob.will(vec![ViewEvent::SnackbarClosed(newer)]);
    bob.pump(3).await;
    for command in held {
        for update in bob.facade.execute(command, 4).await {
            bob.model.apply(update);
        }
    }
    let own = bob.model().snackbar().expect("the late cancel's own");
    assert!(matches!(own.confirms, Confirmation::Cancelled { .. }));
    assert!(own.undo);
}

/// The records held for an Undo are bounded: past the cap the oldest goes.
#[tokio::test]
async fn the_cancelled_records_held_are_bounded() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut bob, _) = Root::new(facade(&b, memory()));
    bob.pump(0).await;
    let rows = pending_rows(&mut bob, &b, a.peer(), CANCELLED_CAP + 1, 1).await;
    for row in &rows {
        let _ = bob.facade.execute(Command::Cancel(*row), 2).await;
    }
    let oldest = bob.facade.execute(Command::Restore(rows[0]), 3).await;
    assert!(
        matches!(
            oldest.last(),
            Some(Update::Failed {
                why: Failure::NoSuchRow,
                ..
            })
        ),
        "the oldest went: {oldest:?}"
    );
    let next = bob.facade.execute(Command::Restore(rows[1]), 3).await;
    assert!(
        next.iter().any(|u| matches!(u, Update::Restored { .. })),
        "the cap's worth stay: {next:?}"
    );
}

/// Keep, then Hide of the same message, then a Hide of another, in one
/// take: the kept message is still shown once its Keep is answered
/// (review F1 on #289, round 2).
#[tokio::test]
async fn a_keep_on_its_way_keeps_its_message_through_a_hide() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, _) = Root::new(facade(&a, memory()));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    bob.will(vec![send(&direct(a.peer()), "keep me")]);
    bob.pump(1).await;
    bob.will(vec![send(&direct(a.peer()), "and me")]);
    bob.pump(2).await;
    alice.pump(3).await;
    let from_bob = alice.model().conversations()[0].key.clone();
    let read = alice.model().conversation_viewed(&from_bob, true);
    alice.will(read.into_iter().map(ViewEvent::Intent).collect());
    alice.pump(4).await;
    let items: Vec<_> = alice
        .model()
        .messages(&from_bob)
        .iter()
        .map(|m| m.key)
        .collect();
    let keep = alice
        .model()
        .intent_for(items[0], RowAction::Keep)
        .expect("offered");
    alice.will(vec![
        ViewEvent::Intent(keep),
        ViewEvent::Intent(Intent::Hide(items[0])),
        ViewEvent::Intent(Intent::Hide(items[1])),
    ]);
    alice.pump(5).await;
    alice.pump(6).await;
    let shown: Vec<_> = alice
        .model()
        .messages(&from_bob)
        .iter()
        .map(|m| m.key)
        .collect();
    assert_eq!(shown, [items[0]], "kept, and shown");
    assert!(matches!(
        alice.model().actions(items[0]).as_slice(),
        [Intent::Unkeep(_)]
    ));
}

/// More cancels than the old cap of eight queued in one take before an
/// Undo: the Undo still finds its record (review F2 on #289, round 2).
#[tokio::test]
async fn an_undo_of_cancel_survives_many_cancels_queued_before_it() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut bob, _) = Root::new(facade(&b, memory()));
    bob.pump(0).await;
    let rows = pending_rows(&mut bob, &b, a.peer(), 10, 1).await;
    bob.will(vec![ViewEvent::Intent(Intent::Cancel(rows[0]))]);
    bob.pump(2).await;
    let first = snack(&bob);
    let mut take: Vec<ViewEvent> = rows[1..]
        .iter()
        .map(|row| ViewEvent::Intent(Intent::Cancel(*row)))
        .collect();
    take.push(ViewEvent::Undo(first));
    bob.will(take);
    bob.pump(3).await;
    assert!(
        bob.updates.iter().any(|u| matches!(
            u,
            Update::Restored { cancelled, .. } if *cancelled == rows[0]
        )),
        "{:?}",
        bob.updates
    );
}

/// However late the facade answers, the model side keeps no more cancels
/// in flight than the facade side can hold records for (review F2 on #289,
/// round 3).
#[tokio::test]
async fn cancels_in_flight_stay_within_what_can_be_held() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut bob, _) = Root::new(facade(&b, memory()));
    bob.pump(0).await;
    let rows = pending_rows(&mut bob, &b, a.peer(), CANCELLED_CAP + 2, 1).await;
    // Every cancel pressed, none answered: the facade is busy elsewhere.
    bob.will(
        rows.iter()
            .map(|row| ViewEvent::Intent(Intent::Cancel(*row)))
            .collect(),
    );
    let commands = bob.model.turn();
    let cancels = commands
        .iter()
        .filter(|c| matches!(c, Command::Cancel(_)))
        .count();
    assert_eq!(cancels, CANCELLED_CAP - 2);
    // The four past it are reported, never silent.
    let too_many = bob
        .model
        .take_problems()
        .into_iter()
        .filter(|p| {
            matches!(
                p,
                Problem::Command {
                    command: Command::Cancel(_),
                    why: Failure::TooMany
                }
            )
        })
        .count();
    assert_eq!(too_many, 4);
}

/// What closed snackbars no longer need is let go ahead of the next
/// turn's cancels (review of #289, round 4).
#[tokio::test]
async fn released_records_go_before_the_next_cancels() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut bob, _) = Root::new(facade(&b, memory()));
    bob.pump(0).await;
    let rows = pending_rows(&mut bob, &b, a.peer(), 4, 1).await;
    bob.will(vec![
        ViewEvent::Intent(Intent::Cancel(rows[0])),
        ViewEvent::Intent(Intent::Cancel(rows[1])),
        ViewEvent::Intent(Intent::Cancel(rows[2])),
    ]);
    bob.pump(2).await;
    bob.will(vec![ViewEvent::Intent(Intent::Cancel(rows[3]))]);
    let commands = bob.model.turn();
    let cancel = commands
        .iter()
        .position(|c| matches!(c, Command::Cancel(_)))
        .expect("the cancel");
    let forgets: Vec<_> = commands[..cancel]
        .iter()
        .filter(|c| matches!(c, Command::ForgetCancelled(_)))
        .collect();
    assert_eq!(
        forgets.len(),
        2,
        "both replaced snackbars' records first: {commands:?}"
    );
}

/// Dismiss, then Keep of the same message, then another Dismiss, in one
/// take: the Keep, judged by the view before the Dismiss was applied, is
/// not issued for a dismissed message, and nothing the store holds is
/// lost (review P-1 on #289, round 4).
#[tokio::test]
async fn a_keep_taken_after_its_dismiss_is_not_issued() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, _) = Root::new(facade(&a, memory()));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    bob.will(vec![send(&direct(a.peer()), "first")]);
    bob.pump(1).await;
    bob.will(vec![send(&direct(a.peer()), "second")]);
    bob.pump(2).await;
    alice.pump(3).await;
    let from_bob = alice.model().conversations()[0].key.clone();
    let read = alice.model().conversation_viewed(&from_bob, true);
    alice.will(read.into_iter().map(ViewEvent::Intent).collect());
    alice.pump(4).await;
    let items: Vec<_> = alice
        .model()
        .messages(&from_bob)
        .iter()
        .map(|m| m.key)
        .collect();
    let keep = alice
        .model()
        .intent_for(items[0], RowAction::Keep)
        .expect("offered");
    alice.commands.clear();
    alice.will(vec![
        ViewEvent::Intent(Intent::Hide(items[0])),
        ViewEvent::Intent(keep),
        ViewEvent::Intent(Intent::Hide(items[1])),
    ]);
    alice.pump(5).await;
    alice.pump(6).await;
    assert!(
        !alice
            .commands
            .iter()
            .any(|c| matches!(c, Command::Keep { .. })),
        "no Keep for a dismissed message: {:?}",
        alice.commands
    );
    // Reported, not silent.
    assert!(alice.model.take_problems().iter().any(|p| matches!(
        p,
        Problem::Command {
            command: Command::Keep { .. },
            why: Failure::NoSuchRow
        }
    )));
}

/// Keep of a message, then a Dismiss of it and a hide of its conversation,
/// in one take: the Keep is issued, the two hides are dropped without a
/// report, and the message stays shown -- the Keep's confirmation answers
/// them (the README's "Hide writes nothing" rule; review of #289, round 7).
#[tokio::test]
async fn hides_after_a_keep_in_one_take_leave_the_keep_and_report_nothing() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut alice, _) = Root::new(facade(&a, memory()));
    let (mut bob, _) = Root::new(facade(&b, memory()));
    alice.pump(0).await;
    bob.pump(0).await;
    bob.will(vec![send(&direct(a.peer()), "keep me")]);
    bob.pump(1).await;
    alice.pump(2).await;
    let from_bob = alice.model().conversations()[0].key.clone();
    let read = alice.model().conversation_viewed(&from_bob, true);
    alice.will(read.into_iter().map(ViewEvent::Intent).collect());
    alice.pump(3).await;
    let item = alice.model().messages(&from_bob)[0].key;
    let keep = alice
        .model()
        .intent_for(item, RowAction::Keep)
        .expect("offered");
    let _ = alice.model.take_problems();
    alice.commands.clear();
    alice.will(vec![
        ViewEvent::Intent(keep),
        ViewEvent::Intent(Intent::Hide(item)),
        ViewEvent::Intent(Intent::HideConversation(from_bob.clone())),
    ]);
    alice.pump(4).await;
    assert_eq!(
        alice
            .commands
            .iter()
            .filter(|c| matches!(c, Command::Keep { .. }))
            .count(),
        1
    );
    assert!(alice.model.take_problems().is_empty(), "no report");
    assert_eq!(
        alice
            .model()
            .messages(&from_bob)
            .iter()
            .map(|m| m.key)
            .collect::<Vec<_>>(),
        [item],
        "not hidden"
    );
}

/// The ask for the person's authentication is the platform's too: it
/// reaches the opener and never the facade, and while it is needed the
/// model says so, offline rather than unknown (#309 acceptance 10).
#[tokio::test]
async fn an_ask_for_authentication_reaches_the_platform_and_never_the_facade() {
    use interweave_human_client_api::Connectivity;
    use interweave_human_ui_model::SessionNotice;
    let (a, _) = FakeNetwork::pair(node(), node());
    let (mut alice, opened) = Root::new(facade(&a, memory()));
    alice.settled(0).await;
    alice.model.authentication_needed(true);
    assert_eq!(
        alice.model().session_notice(),
        Some(SessionNotice::AuthenticationNeeded)
    );
    assert_eq!(alice.model().connectivity(), Connectivity::Offline);
    alice.will(vec![ViewEvent::Intent(Intent::Authenticate)]);
    alice.pump(1).await;
    assert_eq!(*opened.0.borrow(), [AUTHENTICATION_ASKED]);
    assert!(alice.commands.is_empty(), "{:?}", alice.commands);
    alice.model.authentication_needed(false);
    assert_ne!(
        alice.model().session_notice(),
        Some(SessionNotice::AuthenticationNeeded)
    );
}

/// A session ready reads the trust and address lists once, for the
/// no-reach notice, and both are read in. (A runtime without the address
/// methods is the facade's own test: `a_runtime_without_the_methods_is_incompatible`.)
#[tokio::test]
async fn a_ready_session_reads_both_lists_once() {
    let (a, _) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&a, memory()));
    root.settled(0).await;
    for now in 1..4 {
        root.pump(now).await;
    }
    assert!(
        !root
            .commands
            .iter()
            .any(|c| matches!(c, Command::ReadTrust | Command::ReadAddresses)),
        "read once: {:?}",
        root.commands
    );
    assert!(root.model().trust_settings().list().is_some());
    assert!(root.model().peers().list().is_some());
    assert_eq!(root.model().peers().list_problem(), None);
}

/// An address intent handed over bare reaches nothing: the addresses
/// change only through Peers' inputs, after the client's own rules.
#[tokio::test]
async fn a_bare_address_intent_reaches_nothing() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&a, memory()));
    root.settled(0).await;
    let address = format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", b.peer().as_str());
    root.will(vec![
        ViewEvent::Intent(Intent::ReadAddresses),
        ViewEvent::Intent(Intent::AddAddresses(vec![address.clone()])),
        ViewEvent::Intent(Intent::RemoveAddress(address)),
        ViewEvent::Intent(Intent::ReadOwnAddresses),
    ]);
    root.pump(1).await;
    assert!(
        !root.commands.iter().any(|c| matches!(
            c,
            Command::ReadAddresses
                | Command::AddAddresses(_)
                | Command::RemoveAddress(_)
                | Command::ReadOwnAddresses
        )),
        "{:?}",
        root.commands
    );
}

/// Show my address reaches the facade as one read of this node's own
/// addresses, and what it answers is what Show draws.
#[tokio::test]
async fn show_reads_this_nodes_addresses_through_the_facade() {
    use interweave_human_ui_model::{PeersInput, ShowState};
    let (a, _) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&a, memory()));
    root.settled(0).await;
    root.will(vec![ViewEvent::Peers(PeersInput::OpenShow)]);
    root.pump(1).await;
    assert_eq!(root.commands, vec![Command::ReadOwnAddresses]);
    let want = format!("/ip4/192.168.77.10/tcp/4001/p2p/{}", a.peer().as_str());
    assert_eq!(
        root.model().peers().show(),
        Some(&ShowState::Shown {
            peer: a.peer().clone(),
            lines: vec![want]
        })
    );
}

/// Peers' add reaches the facade as one add of all its lines: one line
/// is the runtime's single add, which this fake answers as added;
/// several, typed or scanned, are one add, never split, which the
/// runtime's add of several answers; more than `TYPED_LINES` typed are
/// refused before anything is sent.
#[tokio::test]
async fn an_add_reaches_the_runtime_whole_and_says_what_it_answered() {
    use interweave_human_ui_model::{AddNote, PeersInput, TYPED_LINES};
    let (a, b) = FakeNetwork::pair(node(), node());
    let one = format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", b.peer().as_str());
    let two = format!("/ip4/192.168.1.21/tcp/4001/p2p/{}", b.peer().as_str());

    let (mut root, _) = Root::new(facade(&a, memory()));
    root.settled(0).await;
    root.will(vec![
        ViewEvent::Peers(PeersInput::OpenAdd),
        ViewEvent::Peers(PeersInput::FieldChanged(one.clone())),
        ViewEvent::Peers(PeersInput::Add),
    ]);
    root.pump(1).await;
    assert_eq!(
        root.commands,
        vec![Command::AddAddresses(vec![one.clone()])]
    );
    // The pair trust each other: added, no trust step, the page closed.
    assert!(
        !root.model().peers().add_open(),
        "{:?}",
        root.model().peers().add_page_note()
    );
    assert_eq!(
        root.model().peers().snack().0,
        Some(&interweave_human_ui_model::PeersSnack::Added)
    );

    let (c, _) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&c, memory()));
    root.settled(0).await;
    root.will(vec![
        ViewEvent::Peers(PeersInput::OpenAdd),
        ViewEvent::Peers(PeersInput::FieldChanged(format!("{one}\n{two}"))),
        ViewEvent::Peers(PeersInput::Add),
    ]);
    root.pump(1).await;
    assert_eq!(
        root.commands,
        vec![Command::AddAddresses(vec![one.clone(), two.clone()])],
        "two typed lines are one add"
    );
    let many: Vec<String> = (0..=TYPED_LINES)
        .map(|i| format!("/ip4/192.168.1.{i}/tcp/4001/p2p/{}", b.peer().as_str()))
        .collect();
    let (fifth, _) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&fifth, memory()));
    root.settled(0).await;
    root.will(vec![
        ViewEvent::Peers(PeersInput::OpenAdd),
        ViewEvent::Peers(PeersInput::FieldChanged(many.join("\n"))),
        ViewEvent::Peers(PeersInput::Add),
    ]);
    root.pump(1).await;
    assert!(root.commands.is_empty(), "one line too many sends nothing");
    assert_eq!(
        root.model().peers().add_page_note(),
        Some(&AddNote::TooManyLines {
            lines: TYPED_LINES + 1
        })
    );

    let (d, _) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&d, memory()));
    root.settled(0).await;
    root.will(vec![
        ViewEvent::Peers(PeersInput::OpenAdd),
        ViewEvent::Peers(PeersInput::Scanned(format!("{one}\n{two}"))),
    ]);
    root.pump(1).await;
    assert_eq!(root.commands, vec![Command::AddAddresses(vec![one, two])]);
    // Both lines added; their peer is not this node's to trust yet.
    assert_eq!(
        root.model().peers().add_result(),
        Some(&interweave_human_ui_model::AddResult::Added {
            lines: 2,
            peers: vec![b.peer().clone()]
        }),
        "{:?}",
        root.model().peers().add_page_note()
    );
}

/// The words' `{max}` is the configuration's bound, which the runtime
/// enforces: one value, held equal here.
#[test]
fn the_address_bound_in_the_words_is_the_configurations() {
    assert_eq!(
        interweave_human_ui_model::ADDRESS_BOUND,
        interweave_profile_config::MAX_STATIC_BOOTSTRAP_PEERS
    );
}

/// A step-two Trust's answer reaches Peers only at the turn's end, after
/// its render, and asks the facade nothing: the turn that takes it draws
/// it all the same -- a refused Trust's problem line in its block, and a
/// made one's decided page closing -- with no other wake.
#[tokio::test]
async fn a_step_two_trust_answer_is_drawn_by_the_turn_that_takes_it() {
    use interweave_human_client_api::{
        AddressAnswer, AddressList, AddressOrigin, AddressOutcome, PeerAddress, TrustList,
        TrustProblem, TrustRow, TrustSetFailure,
    };
    use interweave_human_ui_model::{BlockProblem, PeersInput};
    for made in [false, true] {
        // A peer the pair's trust lists do not hold, so its add has a block.
        let (a, _) = FakeNetwork::pair(node(), node());
        let stranger = ProfileIdentity::generate()
            .transport_identity()
            .expect("a peer");
        let line = format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", stranger.as_str());
        let (mut root, _) = Root::new(facade(&a, memory()));
        root.settled(0).await;
        root.will(vec![
            ViewEvent::Peers(PeersInput::OpenAdd),
            ViewEvent::Peers(PeersInput::FieldChanged(line.clone())),
            ViewEvent::Peers(PeersInput::Add),
        ]);
        let _ = root.model.turn();
        root.model.apply(Update::AddressesAdded {
            lines: vec![line.clone()],
            answer: Ok(AddressAnswer {
                outcome: AddressOutcome::Added {
                    peer: Some(stranger.clone()),
                },
                list: Ok(AddressList {
                    addresses: vec![PeerAddress {
                        address: line.clone(),
                        peer: Some(stranger.clone()),
                        origin: AddressOrigin::AddedHere,
                        dialled: true,
                    }],
                }),
            }),
        });
        root.will(vec![ViewEvent::Peers(PeersInput::Trust(stranger.clone()))]);
        let commands = root.model.turn();
        assert!(
            commands.iter().any(|c| matches!(c, Command::SetTrust(_))),
            "{commands:?}"
        );
        let change = TrustChange {
            peer: stranger.clone(),
            allowed: true,
        };
        root.model.apply(Update::TrustSet {
            change,
            answer: if made {
                Ok(TrustList {
                    local_peer: Some(a.peer().clone()),
                    allowed: vec![TrustRow::added_here(stranger.clone())],
                })
            } else {
                Err(TrustSetFailure::NotMade(TrustProblem::NotPermitted))
            },
        });
        let _ = root.model.turn();
        let shown = root.model.surface().peers_shown.clone().expect("rendered");
        if made {
            assert!(!shown.add_open(), "the decided page closed, drawn");
        } else {
            let trust = root.model().trust_settings();
            assert_eq!(
                shown.blocks(trust)[0].problem,
                Some(BlockProblem::Trust(TrustProblem::NotPermitted)),
                "the refusal drawn in its block"
            );
        }
    }
}

/// A session that becomes ready again -- a reconnection, the runtime
/// perhaps restarted with other lists -- reads both lists again, once,
/// though both were read before; and the facade names this node's
/// `PeerId` with it, for Peers' own row.
#[tokio::test]
async fn a_new_session_reads_both_lists_again() {
    use interweave_human_client_api::{ClientEvent, SessionState};
    let (a, _) = FakeNetwork::pair(node(), node());
    let (mut root, _) = Root::new(facade(&a, memory()));
    for now in 0..4 {
        root.pump(now).await;
    }
    assert!(
        root.updates
            .iter()
            .any(|u| matches!(u, Update::ThisDevice(p) if p == a.peer())),
        "the session's PeerId sent with Ready"
    );
    // Both lists read: the fake answers the address read as from an
    // older runtime, so it is given here as read and empty.
    root.model.apply(Update::AddressesRead(Ok(
        interweave_human_client_api::AddressList {
            addresses: Vec::new(),
        },
    )));
    assert!(root.model().trust_settings().list().is_some());
    assert!(root.model().peers().list().is_some());
    root.commands.clear();
    root.model.apply(Update::Client(ClientEvent::Session(
        SessionState::Reconnecting {
            attempt: 1,
            next_at: 0,
        },
    )));
    root.model
        .apply(Update::Client(ClientEvent::Session(SessionState::Ready {
            endpoint: None,
        })));
    let commands = root.model.turn();
    let count = |want: fn(&Command) -> bool| commands.iter().filter(|c| want(c)).count();
    assert_eq!(
        count(|c| matches!(c, Command::ReadTrust)),
        1,
        "{commands:?}"
    );
    assert_eq!(
        count(|c| matches!(c, Command::ReadAddresses)),
        1,
        "{commands:?}"
    );
}

/// An admin answer and its `Done` are separate updates, and each platform
/// may deliver them in separate turns: the answer frees the command, so a
/// retry the model offers once it is answered is sent, never dropped as
/// still on its way -- which would leave Peers busy, its add page with no
/// way out, for good.
#[tokio::test]
async fn an_admin_retry_answered_before_its_done_is_sent() {
    use interweave_human_client_api::{
        AddressAnswer, AddressChangeFailure, AddressList, AddressOrigin, AddressOutcome,
        AddressProblem, ClientEvent, PeerAddress, SessionState, TrustProblem, TrustSetFailure,
    };
    use interweave_human_ui_model::PeersInput;
    let (a, _) = FakeNetwork::pair(node(), node());
    let stranger = ProfileIdentity::generate()
        .transport_identity()
        .expect("a peer");
    let line = format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", stranger.as_str());
    let (mut root, _) = Root::new(facade(&a, memory()));
    root.settled(0).await;
    let issued = |commands: &[Command], want: &Command| commands.iter().any(|c| c == want);

    // Both reads, answered failed with no Done; a new session reads again.
    let again = |root: &mut Root| {
        root.model.apply(Update::Client(ClientEvent::Session(
            SessionState::Reconnecting {
                attempt: 1,
                next_at: 0,
            },
        )));
        root.model
            .apply(Update::Client(ClientEvent::Session(SessionState::Ready {
                endpoint: None,
            })));
    };
    again(&mut root);
    let commands = root.model.turn();
    assert!(issued(&commands, &Command::ReadTrust), "{commands:?}");
    assert!(issued(&commands, &Command::ReadAddresses), "{commands:?}");
    root.model
        .apply(Update::TrustRead(Err(TrustProblem::Unavailable)));
    root.model
        .apply(Update::AddressesRead(Err(AddressProblem::Unavailable)));
    again(&mut root);
    let commands = root.model.turn();
    assert!(issued(&commands, &Command::ReadTrust), "{commands:?}");
    assert!(issued(&commands, &Command::ReadAddresses), "{commands:?}");
    root.model.apply(Update::Done(Command::ReadTrust));
    root.model.apply(Update::Done(Command::ReadAddresses));

    // An add not made, and its retry.
    let add = Command::AddAddresses(vec![line.clone()]);
    root.will(vec![
        ViewEvent::Peers(PeersInput::OpenAdd),
        ViewEvent::Peers(PeersInput::FieldChanged(line.clone())),
        ViewEvent::Peers(PeersInput::Add),
    ]);
    assert!(issued(&root.model.turn(), &add));
    root.model.apply(Update::AddressesAdded {
        lines: vec![line.clone()],
        answer: Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable)),
    });
    root.will(vec![ViewEvent::Peers(PeersInput::Add)]);
    let commands = root.model.turn();
    assert!(issued(&commands, &add), "the retry sent: {commands:?}");
    root.model.apply(Update::Done(add.clone()));

    // Made, so the step's Trust is offered: refused, and pressed again.
    let listed = AddressList {
        addresses: vec![PeerAddress {
            address: line.clone(),
            peer: Some(stranger.clone()),
            origin: AddressOrigin::AddedHere,
            dialled: true,
        }],
    };
    root.model.apply(Update::AddressesAdded {
        lines: vec![line.clone()],
        answer: Ok(AddressAnswer {
            outcome: AddressOutcome::Added {
                peer: Some(stranger.clone()),
            },
            list: Ok(listed.clone()),
        }),
    });
    root.model.apply(Update::Done(add));
    let set = Command::SetTrust(TrustChange {
        peer: stranger.clone(),
        allowed: true,
    });
    root.will(vec![ViewEvent::Peers(PeersInput::Trust(stranger.clone()))]);
    assert!(issued(&root.model.turn(), &set));
    root.model.apply(Update::TrustSet {
        change: TrustChange {
            peer: stranger.clone(),
            allowed: true,
        },
        answer: Err(TrustSetFailure::NotMade(TrustProblem::Unavailable)),
    });
    // Peers takes a Trust answer at the turn's end: the block offers its
    // Trust again from the next.
    let _ = root.model.turn();
    root.will(vec![ViewEvent::Peers(PeersInput::Trust(stranger.clone()))]);
    let commands = root.model.turn();
    assert!(
        issued(&commands, &set),
        "the Trust again sent: {commands:?}"
    );
    root.model.apply(Update::Done(set));

    // A removal not made, and its Try again.
    let remove = Command::RemoveAddress(line.clone());
    root.will(vec![ViewEvent::Peers(PeersInput::Remove(line.clone()))]);
    assert!(issued(&root.model.turn(), &remove));
    root.model.apply(Update::AddressRemoved {
        address: line.clone(),
        answer: Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable)),
    });
    root.will(vec![ViewEvent::Peers(PeersInput::TryAgain)]);
    let commands = root.model.turn();
    assert!(
        issued(&commands, &remove),
        "the Try again sent: {commands:?}"
    );
    root.model.apply(Update::Done(remove));
}
