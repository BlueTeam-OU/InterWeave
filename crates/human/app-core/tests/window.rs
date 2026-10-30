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
    ViewEvent, WindowEvent, WindowInput,
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

/// A surface that draws nothing: it counts its renders and hands over the
/// window reports a test queues, as a view would.
#[derive(Default)]
struct Blind {
    renders: usize,
    window: Vec<WindowEvent>,
}

impl Surface for Blind {
    fn render(&mut self, _model: &UiModel) {
        self.renders += 1;
    }
    fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
        Vec::new()
    }
    fn take_window(&mut self) -> Vec<WindowEvent> {
        std::mem::take(&mut self.window)
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

    let mut model = ModelSide::new(Blind::default(), NoOpener);
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
    assert_eq!(
        model.model().rows_not_shown(),
        1,
        "the count reaches the model"
    );

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
        let mut model = ModelSide::new(Blind::default(), NoOpener);
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

/// A contact's photo is read when asked, as it stood then: the contacts an
/// Accept hands back say the contact has one without its bytes -- no
/// contact's photo reaches the model but through its ask, under its cap
/// (#501 review F3) -- the ask brings it decoded, and an ask made under an
/// older time brings none, the photo it named being gone. Mutations: the
/// photo given whatever its time; a contact write answered with every
/// contact's photo decoded.
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
    assert!(
        !root
            .updates
            .iter()
            .any(|u| matches!(u, Update::ContactsListed(_))),
        "no contact list with photos: {:?}",
        root.updates
    );
    let contact = root
        .updates
        .iter()
        .rev()
        .find_map(|u| match u {
            Update::ContactDirectory(Ok(contacts)) => contacts.first().cloned(),
            _ => None,
        })
        .expect("the contacts the Accept hands back");
    assert!(
        contact.has_photo && contact.view.photo.is_none(),
        "no bytes"
    );
    let id = contact.view.contact_id.clone();
    assert_eq!(root.model().contact_photo(&id), PhotoState::Initials);

    // Try again's read gives the same: the contacts, no bytes.
    root.updates.clear();
    root.execute(Command::Window(WindowCommand::ReadContacts))
        .await;
    assert!(
        root.updates.iter().any(|u| matches!(
            u,
            Update::ContactDirectory(Ok(c)) if c.len() == 1 && c[0].view.photo.is_none()
        )),
        "{:?}",
        root.updates
    );

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

/// A card the drain hands over to a windowed start counts once in its
/// conversation's badge and is its newest row, as a card: the facade
/// side's `ReceivedCard` reaches the model as an arrival, not as a stored
/// card the counts already hold (#498 review F1, carried to this batch).
/// Mutation: the model side giving a drained card to `card_received`.
#[tokio::test]
async fn a_card_the_drain_hands_over_counts_once_in_its_badge() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let mut alice = facade(
        &a,
        HumanStore::open_in_memory(StoreOptions::default()).expect("store"),
    );
    alice.set_identity_key(IdentityKey::generate(human()).expect("a key"));
    let mut bob = Root::start(facade(
        &b,
        HumanStore::open_in_memory(StoreOptions::default()).expect("store"),
    ));
    for now in 0..3 {
        alice.turn(now).await;
        for update in bob.side.turn(now).await {
            bob.model.apply(update);
        }
    }
    let updates = alice.execute(Command::ShareCard(direct(b.peer())), 3).await;
    assert!(
        updates
            .iter()
            .any(|u| matches!(u, Update::CardQueued { .. })),
        "{updates:?}"
    );
    for now in 4..8 {
        alice.turn(now).await;
        for update in bob.side.turn(now).await {
            bob.updates.push(update.clone());
            bob.model.apply(update);
        }
    }
    assert!(
        bob.updates
            .iter()
            .any(|u| matches!(u, Update::ReceivedCard(_))),
        "control: the card was drained, {:?}",
        bob.updates
    );
    let listed = bob
        .model()
        .conversations()
        .into_iter()
        .find(|c| c.key == direct(a.peer()))
        .expect("the conversation is listed");
    assert_eq!(listed.unread, 1);
    assert!(listed.last.card);
}

/// What the view reports of the window reaches the model through the
/// turn, and the page it reaches is read: an opening reported by the
/// surface brings the conversation's first page with no call past the
/// surface. Mutation: the turn not draining the surface's reports.
#[tokio::test]
async fn the_turn_takes_the_views_window_reports_and_reads_the_page() {
    let from = peer();
    let mut store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
    for n in 1..=10 {
        let mut row = message(n, &from, u64::from(n));
        row.kind = InboundKind::Text;
        store.commit_unread_inbound(&row).expect("committed");
    }
    let (a, _b) = FakeNetwork::pair(node(), node());
    let mut root = Root::start(facade(&a, store));
    let key = direct(&from);
    root.model
        .surface_mut()
        .window
        .push(WindowEvent::Input(WindowInput::Opened(key.clone())));
    root.turn().await;
    assert_eq!(root.model().messages(&key).len(), 10);
}

/// A report that changes what is drawn and asks the facade nothing --
/// closing the open conversation lets its rows go and takes its marks
/// away -- is drawn by the turn that takes it, since no update would bring
/// another; the person's place, which a scroll reports on every move,
/// draws nothing. Mutations: a changed report not drawn; every report
/// drawn.
#[tokio::test]
async fn a_report_that_changes_what_is_drawn_draws_and_a_place_never_does() {
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
    assert!(!root.model().window_marks(&key).is_empty(), "the top line");
    let renders = |root: &mut Root| root.model.surface_mut().renders;

    let before = renders(&mut root);
    root.model
        .surface_mut()
        .window
        .push(WindowEvent::Input(WindowInput::Place {
            conversation: key.clone(),
            focused: None,
            in_view: Vec::new(),
        }));
    assert!(root.model.turn().is_empty());
    assert_eq!(renders(&mut root), before + 1, "the turn's own render only");

    let before = renders(&mut root);
    root.model
        .surface_mut()
        .window
        .push(WindowEvent::Input(WindowInput::Closed(key.clone())));
    assert!(root.model.turn().is_empty(), "closing asks nothing");
    assert!(root.model().window_marks(&key).is_empty());
    assert_eq!(
        renders(&mut root),
        before + 2,
        "drawn again by the same turn"
    );
}

/// TEST BUILDS ONLY (`test-hooks`): a store of `count` unread text rows
/// from `from`.
fn texts(from: &TransportIdentity, count: u32) -> HumanStore {
    let mut store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
    for n in 1..=count {
        let mut row = message(n, from, u64::from(n));
        row.kind = InboundKind::Text;
        store.commit_unread_inbound(&row).expect("committed");
    }
    store
}

fn top_failed(root: &Root, key: &ConversationKey) -> bool {
    root.model()
        .window_marks(key)
        .iter()
        .any(|m| m.kind == MarkKind::Failed(Edge::Top))
}

/// A page read the hooks fail is its command's failure: the top shows the
/// failure row and holds no row; with the fault lifted, Try again reads
/// the page. Mutation: the hook not consulted -- the page read.
#[tokio::test]
async fn a_failed_page_read_is_its_commands_failure_and_try_again_reads_it() {
    use interweave_human_app_core::test_hooks::{Fault, StoreFaults};
    let from = peer();
    let faults = std::sync::Arc::new(StoreFaults::default());
    faults.set_pages(Fault::Fail);
    let (a, _b) = FakeNetwork::pair(node(), node());
    let mut root = Root::start(facade(&a, texts(&from, 10)).with_store_faults(faults.clone()));
    let key = direct(&from);
    root.model.window(WindowInput::Opened(key.clone()));
    root.turn().await;
    assert!(root.model().messages(&key).is_empty(), "nothing read");
    assert!(top_failed(&root, &key), "the failure row");
    faults.set_pages(Fault::Pass);
    root.model.window(WindowInput::TryAgain {
        conversation: key.clone(),
        edge: Edge::Top,
    });
    root.turn().await;
    assert_eq!(root.model().messages(&key).len(), 10, "Try again reads it");
}

/// A held page read waits for the test and then reads: what lets a
/// headless test see the progress row and the pressed row while a read
/// is on its way. Mutation: a hold that does not wait.
#[tokio::test]
async fn a_held_page_read_returns_once_released() {
    use interweave_human_app_core::test_hooks::{Fault, StoreFaults};
    let from = peer();
    let faults = std::sync::Arc::new(StoreFaults::default());
    faults.set_pages(Fault::Hold);
    let (a, _b) = FakeNetwork::pair(node(), node());
    let mut root = Root::start(facade(&a, texts(&from, 10)).with_store_faults(faults.clone()));
    let key = direct(&from);
    root.model.window(WindowInput::Opened(key.clone()));
    let release = {
        let faults = faults.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));
            faults.set_pages(Fault::Pass);
        })
    };
    let began = std::time::Instant::now();
    root.turn().await;
    let waited = began.elapsed();
    release.join().expect("released");
    assert!(
        waited >= std::time::Duration::from_millis(200),
        "it waited: {waited:?}"
    );
    assert_eq!(root.model().messages(&key).len(), 10, "then read");
}

/// The contacts failing at start are the start's contacts error: the
/// model knows them unread. Mutation: the hook not consulted at start.
#[tokio::test]
async fn contacts_failed_at_start_are_the_starts_contacts_error() {
    use interweave_human_app_core::test_hooks::{Fault, StoreFaults};
    let faults = std::sync::Arc::new(StoreFaults::default());
    faults.set_contacts(Fault::Fail);
    let (a, _b) = FakeNetwork::pair(node(), node());
    let root = Root::start(facade(&a, texts(&peer(), 1)).with_store_faults(faults));
    assert_eq!(
        root.model().contacts_state(),
        interweave_human_ui_model::ContactsState::Unread
    );
}

/// A photo the hooks slow takes at least its delay, and one they make
/// unreadable arrives as none, the contact's initials staying. Mutations:
/// the delay not taken; the photo read as it is.
#[tokio::test]
async fn a_slowed_or_unreadable_photo_is_read_as_the_hooks_say() {
    use interweave_human_app_core::test_hooks::StoreFaults;
    let from = peer();
    let key = IdentityKey::generate(human()).expect("a key");
    let photo = interweave_human_app_core::card_photo::prepare_photo(&png()).expect("prepared");
    let mut store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
    store
        .commit_unread_inbound(&card(1, &key, &from, 1, Some(&photo.bytes)))
        .expect("committed");
    let faults = std::sync::Arc::new(StoreFaults::default());
    let (a, _b) = FakeNetwork::pair(node(), node());
    let mut root = Root::start(facade(&a, store).with_store_faults(faults.clone()));
    root.model.window(WindowInput::Opened(direct(&from)));
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
    let contact = root
        .updates
        .iter()
        .rev()
        .find_map(|u| match u {
            Update::ContactDirectory(Ok(contacts)) => contacts.first().cloned(),
            _ => None,
        })
        .expect("the contacts the Accept hands back");
    let ask = PhotoAsk {
        contact_id: contact.view.contact_id.clone(),
        photo_at: contact.view.photo_at,
    };
    faults.set_photo_delay(std::time::Duration::from_millis(150));
    let began = std::time::Instant::now();
    root.updates.clear();
    root.execute(Command::Window(WindowCommand::ReadPhoto(ask.clone())))
        .await;
    assert!(
        began.elapsed() >= std::time::Duration::from_millis(150),
        "slowed"
    );
    assert!(
        root.updates
            .iter()
            .any(|u| matches!(u, Update::ContactPhoto { photo: Some(_), .. })),
        "then read"
    );
    faults.set_photo_delay(std::time::Duration::ZERO);
    faults.set_unreadable_photo(&ask.contact_id);
    root.updates.clear();
    root.execute(Command::Window(WindowCommand::ReadPhoto(ask)))
        .await;
    assert!(
        root.updates
            .iter()
            .any(|u| matches!(u, Update::ContactPhoto { photo: None, .. })),
        "unreadable: none"
    );
}
