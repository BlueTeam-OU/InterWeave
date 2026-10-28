// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The windowed start (design/2026-10-27-start-window.md sections 3 and
//! 11) over a real store and the fake network: what the facade side reads
//! at a start, counted in rows, and what the model is given.
//!
//! What this does NOT prove: a memory figure, a real daemon, a real
//! window. A row count is what a test can hold; the time a start takes on
//! the phone is the device run's.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use interweave_human_app_core::{
    Command, FacadeSide, Failure, ModelSide, Opener, Surface, Update, WindowCommand,
};
use interweave_human_chat_protocol::{CardDraft, CardRoute, build_card};
use interweave_human_store::identity_key::IdentityKey;
use interweave_human_store::{
    AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, StoreOptions,
};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{
    ConversationKey, Edge, Intent, MarkKind, PhotoAsk, PhotoState, READ_PROPOSAL_CAP, UiModel,
    ViewEvent, WindowInput,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, MediaType, TransportIdentity};

const WALL: u64 = 1_786_600_000_000;

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: peer(),
        endpoints: vec![FakeEndpoint::open(human(), false)],
        default_endpoint: Some(human()),
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
            endpoints: vec![human()],
            channels: vec![],
            max_payload_bytes: 49_152,
        },
        Box::new(|| WALL),
        0,
    )
    .expect("facade");
    FacadeSide::new(client, || WALL)
}

/// Message `n` from `from`, a text envelope as the facade commits one. Its
/// kind is left `Other`, as a store no client has classified holds it.
fn message(n: u32, from: &TransportIdentity, at: u64) -> NewInbound {
    let id = format!("{n:032x}");
    NewInbound {
        app_message_id: AppMessageId::parse(id.clone()).expect("an id"),
        origin: InboundOrigin {
            peer: from.clone(),
            endpoint: Some(human()),
            channel: None,
        },
        media_type: Some(
            MediaType::parse("application/vnd.interweave-human-chat+json;v=2").expect("media"),
        ),
        payload: format!(r#"{{"v":2,"kind":"text","app_message_id":"{id}","text":"message {n}"}}"#)
            .into_bytes(),
        received_at: at,
        kind: InboundKind::Other,
    }
}

struct Blind;

impl Surface for Blind {
    fn render(&mut self, _model: &UiModel) {}
    fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
        Vec::new()
    }
}

struct NoOpener;

impl Opener for NoOpener {
    fn open(&mut self, _destination: &str) {}
    fn ask_network_access(&mut self) {}
    fn authenticate(&mut self) {}
}

const CONVERSATIONS: u32 = 3;
const PER_CONVERSATION: u32 = 300;

/// A start reads one row per conversation, its newest, and none of a
/// backlog: three conversations of 300 unread rows are started from three
/// rows, every badge the store's count, and the model holds no row of
/// them. The first start pays the kinds' classification once, every row;
/// a whole listing of the same store reads every row, the control that
/// shows the count is live. Mutation: the start summarising by listing
/// every row.
#[test]
fn a_start_reads_a_row_per_conversation_never_the_backlog() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let peers: Vec<TransportIdentity> = (0..CONVERSATIONS).map(|_| peer()).collect();
    {
        let mut store = HumanStore::open(&path, StoreOptions::default()).expect("store");
        let mut n = 0;
        for (i, from) in peers.iter().enumerate() {
            for at in 0..PER_CONVERSATION {
                n += 1;
                let at = u64::from(at) * 10 + u64::try_from(i).expect("small");
                store
                    .commit_unread_inbound(&message(n, from, at))
                    .expect("committed");
            }
        }
        let mut hidden = message(n + 1, &peers[0], 1);
        hidden.payload = b"no envelope".to_vec();
        store
            .commit_unread_inbound(&hidden)
            .expect("a row not shown");
    }
    let total = u64::from(CONVERSATIONS * PER_CONVERSATION) + 1;
    let (a, _b) = FakeNetwork::pair(node(), node());

    let mut first = facade(
        &a,
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
    );
    first.start().expect("started");
    assert!(
        first.rows_read() >= total,
        "the first start classifies every row: {}",
        first.rows_read()
    );
    drop(first);

    let mut side = facade(
        &a,
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
    );
    let start = side.start().expect("started");
    assert_eq!(
        side.rows_read(),
        u64::from(CONVERSATIONS),
        "one row per conversation"
    );
    assert_eq!(start.summaries.len(), peers.len());
    assert_eq!(start.undecodable_unread, 1);
    assert!(start.contacts.is_ok());

    let mut model = ModelSide::new(Blind, NoOpener);
    model.apply(Update::Started(Box::new(start)));
    assert!(model.model().windowed());
    assert!(model.model().is_empty(), "no row of a conversation is held");
    for from in &peers {
        let key = ConversationKey::Direct {
            peer: from.clone(),
            endpoint: Some(human()),
        };
        let listed = model
            .model()
            .conversations()
            .into_iter()
            .find(|c| c.key == key)
            .expect("listed");
        assert_eq!(
            listed.unread,
            usize::try_from(PER_CONVERSATION).expect("small")
        );
        assert!(!listed.last.preview.shown().is_empty());
    }
    assert_eq!(model.hidden_rows(), 1);

    let before = side.rows_read();
    side.listing().expect("listed");
    assert!(
        side.rows_read() - before >= total,
        "a whole listing reads every row"
    );
}

const MEDIA: &str = "application/vnd.interweave-human-chat+json;v=2";

/// Card `n`, signed by `key` for the route `from` sends it on, with a
/// photo when one is given (its bytes as a share sends them).
fn card(
    n: u32,
    key: &IdentityKey,
    from: &TransportIdentity,
    at: u64,
    photo: Option<&[u8]>,
) -> NewInbound {
    let id = format!("{n:032x}");
    let text = build_card(
        &id,
        CardDraft {
            name: "Ada",
            surname: None,
            photo: photo.map(|p| ("image/jpeg", p)),
        },
        key.public_key(),
        CardRoute {
            source_peer: from,
            source_endpoint: &human(),
        },
        Some(at),
        |message| key.sign(message),
    )
    .expect("a card");
    NewInbound {
        app_message_id: AppMessageId::parse(id).expect("an id"),
        origin: InboundOrigin {
            peer: from.clone(),
            endpoint: Some(human()),
            channel: None,
        },
        media_type: Some(MediaType::parse(MEDIA).expect("media")),
        payload: text.into_bytes(),
        received_at: at,
        kind: InboundKind::Card,
    }
}

/// A model side started windowed over `side`, and every update kept.
struct Root {
    side: FacadeSide<FakeNode, FakeNode>,
    model: ModelSide<Blind, NoOpener>,
    updates: Vec<Update>,
}

impl Root {
    fn start(mut side: FacadeSide<FakeNode, FakeNode>) -> Self {
        let mut model = ModelSide::new(Blind, NoOpener);
        model.apply(Update::Started(Box::new(side.start().expect("started"))));
        Self {
            side,
            model,
            updates: Vec::new(),
        }
    }

    /// The model side's turn, and every command it asks carried out.
    async fn turn(&mut self) {
        for command in self.model.turn() {
            self.execute(command).await;
        }
    }

    async fn execute(&mut self, command: Command) {
        for update in self.side.execute(command, 1).await {
            self.updates.push(update.clone());
            self.model.apply(update);
        }
    }

    fn model(&self) -> &UiModel {
        self.model.model()
    }
}

fn direct(from: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: from.clone(),
        endpoint: Some(human()),
    }
}

/// Opened, a conversation's rows come a page at a time from the store
/// through the facade, the newest first, until none remain: a backlog of
/// 100 is two pages, every row held once, and the edge then asks nothing.
/// Mutation: the ask's bound not handed to the store, so every page
/// repeats the first.
#[tokio::test]
async fn an_opened_conversation_pages_its_rows_in_from_the_store() {
    let from = peer();
    let mut store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
    for n in 1..=100 {
        let mut row = message(n, &from, u64::from(n));
        row.kind = InboundKind::Text;
        store.commit_unread_inbound(&row).expect("committed");
    }
    let (a, _b) = FakeNetwork::pair(node(), node());
    let mut root = Root::start(facade(&a, store));
    let key = direct(&from);
    root.model.window(WindowInput::Opened(key.clone()));
    root.turn().await;
    assert_eq!(root.model().messages(&key).len(), 64, "one page");
    for _ in 0..2 {
        root.model.window(WindowInput::Reached {
            conversation: key.clone(),
            edge: Edge::Top,
        });
        root.turn().await;
        root.model.window(WindowInput::Left {
            conversation: key.clone(),
            edge: Edge::Top,
        });
    }
    let held = root.model().messages(&key);
    assert_eq!(held.len(), 100, "every row, once");
    let pages = root
        .updates
        .iter()
        .filter(|u| matches!(u, Update::Page(_)))
        .count();
    assert_eq!(pages, 2, "the exhausted edge asks nothing more");
}

/// Paging a conversation of more unread cards than the read proposals the
/// session holds (`READ_PROPOSAL_CAP`) stops at that bound, with the limit
/// row, while each shown card is read: no read proposal is evicted and the
/// facade still holds the copy of the first, which a Decline needs
/// (architect-cto's ruling on j68 B2). Mutation: the page's room taken
/// from the session's items alone, ignoring the proposals'.
#[tokio::test]
async fn paging_past_the_read_proposal_bound_evicts_none_and_loses_no_copy() {
    let from = peer();
    let key = IdentityKey::generate(human()).expect("a key");
    let mut store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
    let total = u32::try_from(READ_PROPOSAL_CAP).expect("small") + 44;
    for n in 1..=total {
        store
            .commit_unread_inbound(&card(n, &key, &from, u64::from(n), None))
            .expect("committed");
    }
    let (a, _b) = FakeNetwork::pair(node(), node());
    let mut root = Root::start(facade(&a, store));
    let conversation = direct(&from);
    root.model.window(WindowInput::Opened(conversation.clone()));
    let mut first_read = None;
    for _ in 0..8 {
        root.turn().await;
        // What a view showing them raises: each shown card read.
        for intent in root.model().proposals_viewed(&conversation, true) {
            if let Intent::MarkRead(row) = intent {
                first_read.get_or_insert(row);
                root.execute(Command::MarkRead(row)).await;
            }
        }
        root.model.window(WindowInput::Reached {
            conversation: conversation.clone(),
            edge: Edge::Top,
        });
        root.turn().await;
        root.model.window(WindowInput::Left {
            conversation: conversation.clone(),
            edge: Edge::Top,
        });
    }
    assert_eq!(
        root.model().proposals(&conversation).len(),
        READ_PROPOSAL_CAP,
        "every read proposal held, none past the bound"
    );
    assert!(
        root.model()
            .window_marks(&conversation)
            .iter()
            .any(|m| m.kind == MarkKind::AtLimit(Edge::Top)),
        "the limit row: {:?}",
        root.model().window_marks(&conversation)
    );
    let first = first_read.expect("cards were read");
    root.updates.clear();
    root.execute(Command::DeclineCard(first)).await;
    assert!(
        !root.updates.iter().any(|u| matches!(
            u,
            Update::Failed {
                why: Failure::CopyGone,
                ..
            }
        )),
        "the first card's copy is held: {:?}",
        root.updates
    );
}

/// A contact's photo is read when asked, as it stood then: the directory
/// read says the contact has one without its bytes, the ask brings it
/// decoded, and an ask made under an older time brings none, the photo it
/// named being gone. Mutation: the photo given whatever its time.
#[tokio::test]
async fn a_photo_is_read_when_asked_and_only_as_it_stood() {
    let from = peer();
    let key = IdentityKey::generate(human()).expect("a key");
    let png = png();
    let photo = interweave_human_app_core::card_photo::prepare_photo(&png).expect("prepared");
    let mut store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
    store
        .commit_unread_inbound(&card(1, &key, &from, 1, Some(&photo.bytes)))
        .expect("committed");
    let (a, _b) = FakeNetwork::pair(node(), node());
    let mut root = Root::start(facade(&a, store));
    let conversation = direct(&from);
    root.model.window(WindowInput::Opened(conversation.clone()));
    root.turn().await;
    let proposal = root
        .updates
        .iter()
        .find_map(|u| match u {
            Update::Page(rows) => rows.proposals.first().cloned(),
            _ => None,
        })
        .expect("the card's proposal");
    root.execute(Command::AcceptCard {
        row: proposal.row,
        shown: Box::new(proposal.context.clone()),
    })
    .await;
    root.execute(Command::Window(WindowCommand::ReadContacts))
        .await;
    let contact = root
        .updates
        .iter()
        .rev()
        .find_map(|u| match u {
            Update::ContactDirectory(Ok(contacts)) => contacts.first().cloned(),
            _ => None,
        })
        .expect("the contact, read again");
    assert!(
        contact.has_photo && contact.view.photo.is_none(),
        "no bytes"
    );
    let id = contact.view.contact_id.clone();
    assert_eq!(root.model().contact_photo(&id), PhotoState::Initials);

    root.model.photo_shown(&id);
    root.turn().await;
    assert!(
        matches!(root.model().contact_photo(&id), PhotoState::Shown(_)),
        "{:?}",
        root.model().contact_photo(&id)
    );

    root.updates.clear();
    root.execute(Command::Window(WindowCommand::ReadPhoto(PhotoAsk {
        contact_id: id.clone(),
        photo_at: contact.view.photo_at.map(|at| at - 1),
    })))
    .await;
    assert!(
        root.updates
            .iter()
            .any(|u| matches!(u, Update::ContactPhoto { photo: None, .. })),
        "{:?}",
        root.updates
    );
}

/// A small PNG.
fn png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(8, 8, image::Rgba([200, 40, 40, 255]));
    let mut out = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .expect("a PNG");
    out
}
