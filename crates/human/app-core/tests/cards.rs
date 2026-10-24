// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Contact cards through the root, both sides and the loop between them,
//! over two fake nodes (plan §29 step 2 and its exit gate (b);
//! `human-client-ui.md` §13): Share sends the conversation's identity's
//! card and nothing else; the card arrives as a proposal; Accept writes
//! the contact and names the conversation; Decline leaves nothing; an
//! unread card is a proposal again after a restart; a changed key is a
//! key change; a local edit stands; and the identity's private key is in
//! no file of the store.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::collections::VecDeque;

use base64::Engine as _;
use interweave_human_app_core::{Command, FacadeSide, Failure, ModelSide, Opener, Surface, Update};
use interweave_human_store::identity_key::{self, FilesystemCustody, IdentityKey};
use interweave_human_store::{
    AppMessageId, HumanStore, InboundOrigin, NewInbound, OwnCard, StoreOptions,
};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{
    AcceptLine, ConversationKey, Intent, KeyView, PhotoEdit, ProposalItem, Table, UiModel, UiText,
    ViewEvent,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_config::TrustBoundary;
use interweave_profile_config::paths::ProfilePaths;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};

const WALL: u64 = 1_786_600_000_000;

fn human() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: ProfileIdentity::generate()
            .transport_identity()
            .expect("peer id"),
        endpoints: vec![FakeEndpoint::open(human(), false)],
        default_endpoint: Some(human()),
        queue_bound: 16,
    }
}

/// One profile on disk: its paths and its directory.
struct Profile {
    _dir: tempfile::TempDir,
    paths: ProfilePaths,
}

fn profile() -> Profile {
    let dir = common::private_tempdir();
    let paths = ProfilePaths::resolve_embedded(
        "default",
        TrustBoundary::new(dir.path()).expect("boundary"),
    )
    .expect("paths");
    Profile { _dir: dir, paths }
}

fn facade(
    node: &FakeNode,
    profile: &Profile,
    key: Option<IdentityKey>,
) -> FacadeSide<FakeNode, FakeNode> {
    let store =
        HumanStore::open_profile(&profile.paths, &FilesystemCustody, StoreOptions::default())
            .expect("store");
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
    let mut side = FacadeSide::new(client, || WALL);
    if let Some(key) = key {
        side.set_identity_key(key);
    }
    side
}

fn key_of(profile: &Profile) -> IdentityKey {
    identity_key::open(&profile.paths, &human(), &FilesystemCustody)
        .expect("opens")
        .unwrap_or_else(|| {
            identity_key::create(&profile.paths, &human(), &FilesystemCustody).expect("created")
        })
}

#[derive(Default)]
struct Scripted {
    batches: VecDeque<Vec<ViewEvent>>,
}

impl Surface for Scripted {
    fn render(&mut self, _model: &UiModel) {}

    fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
        self.batches.pop_front().unwrap_or_default()
    }
}

#[derive(Default)]
struct NoOpener;

impl Opener for NoOpener {
    fn open(&mut self, _destination: &str) {}
    fn ask_network_access(&mut self) {}
    fn authenticate(&mut self) {}
}

struct Root {
    facade: FacadeSide<FakeNode, FakeNode>,
    model: ModelSide<Scripted, NoOpener>,
    updates: Vec<Update>,
}

impl Root {
    fn new(facade: FacadeSide<FakeNode, FakeNode>) -> Self {
        let mut root = Self {
            facade,
            model: ModelSide::new(Scripted::default(), NoOpener),
            updates: Vec::new(),
        };
        let listing = root.facade.listing().expect("listing");
        root.model.apply(Update::Listed(listing));
        root
    }

    async fn pump(&mut self, now: u64) {
        self.facade_turn(now).await;
        let commands = self.model.turn();
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

    fn will(&mut self, intent: Intent) {
        self.model
            .surface_mut()
            .batches
            .push_back(vec![ViewEvent::Intent(intent)]);
    }

    fn model(&self) -> &UiModel {
        self.model.model()
    }

    fn failed(&self) -> Vec<(Command, Failure)> {
        self.updates
            .iter()
            .filter_map(|u| match u {
                Update::Failed { command, why } => Some((command.clone(), *why)),
                _ => None,
            })
            .collect()
    }
}

fn direct(to: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: to.clone(),
        endpoint: Some(human()),
    }
}

/// Alice names her card, shares it with Bob, and Bob's loop runs.
async fn shared(alice: &mut Root, bob: &mut Root, b: &FakeNode) {
    alice.will(Intent::SetOwnCard {
        name: "Ada".to_owned(),
        surname: Some("Lovelace".to_owned()),
        photo: PhotoEdit::Keep,
    });
    alice.pump(1).await;
    alice.will(
        alice
            .model()
            .share_card(&direct(b.peer()))
            .expect("offered"),
    );
    alice.pump(2).await;
    alice.pump(3).await;
    bob.pump(4).await;
}

#[tokio::test]
async fn a_valid_card_accepted_creates_the_contact_and_names_the_conversation() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, Some(key_of(&pb))));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    assert!(alice.failed().is_empty(), "{:?}", alice.failed());

    let from_alice = direct(a.peer());
    let proposals = bob.model().proposals(&from_alice);
    assert_eq!(proposals.len(), 1, "a proposal, {:?}", bob.updates);
    let proposal = &proposals[0];
    assert_eq!(proposal.name, "Ada Lovelace");
    assert_eq!(&proposal.peer, a.peer(), "beside the full PeerId");
    assert_eq!(proposal.accept, AcceptLine::Creates);
    assert_eq!(proposal.key, KeyView::New);
    assert_eq!(proposal.fingerprint, key_of(&pa).fingerprint().to_string());
    assert!(
        bob.model().contacts().is_empty(),
        "nothing written before Accept"
    );

    bob.will(Intent::AcceptCard(proposal.row));
    bob.pump(5).await;
    assert!(bob.failed().is_empty(), "{:?}", bob.failed());
    assert!(
        bob.model().proposals(&from_alice).is_empty(),
        "the proposal is gone"
    );
    let contacts = bob.model().contacts();
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].name, "Ada Lovelace");
    assert!(contacts[0].name_from_card);
    assert_eq!(
        contacts[0].routes[0].fingerprint.as_deref(),
        Some(key_of(&pa).fingerprint().to_string().as_str()),
        "the contact page shows the key's fingerprint beside the PeerId"
    );
    assert_eq!(
        bob.model().title(&from_alice),
        "Ada Lovelace",
        "the conversation takes the name"
    );
    let store = HumanStore::open_profile(&pb.paths, &FilesystemCustody, StoreOptions::default())
        .expect("reopen");
    assert!(
        store.unread_inbound().expect("read").is_empty(),
        "the card has no Keep"
    );
}

#[tokio::test]
async fn a_declined_card_leaves_nothing_in_the_store() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let row = bob.model().proposals(&direct(a.peer()))[0].row;
    bob.will(Intent::DeclineCard(row));
    bob.pump(5).await;
    assert!(bob.model().proposals(&direct(a.peer())).is_empty());
    let store = HumanStore::open_profile(&pb.paths, &FilesystemCustody, StoreOptions::default())
        .expect("reopen");
    assert!(store.contacts().expect("read").is_empty(), "no contact");
    assert!(
        store.unread_inbound().expect("read").is_empty(),
        "no unread copy"
    );
    assert!(
        store.kept_inbound().expect("read").is_empty(),
        "no kept copy"
    );
}

#[tokio::test]
async fn an_unread_card_is_a_proposal_again_after_a_restart() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    assert_eq!(bob.model().proposals(&direct(a.peer())).len(), 1);
    Box::pin(bob.facade.close()).await;
    drop(bob);
    let bob = Root::new(facade(&b, &pb, None));
    let again = bob.model().proposals(&direct(a.peer()));
    assert_eq!(again.len(), 1, "listed again from the store");
    assert_eq!(again[0].name, "Ada Lovelace");
}

#[tokio::test]
async fn a_shown_card_left_undecided_is_gone_after_a_restart() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let from_alice = direct(a.peer());
    let shown = bob.model().proposals_viewed(&from_alice, true);
    assert_eq!(shown.len(), 1, "showing the proposal reads it");
    assert!(
        bob.model().proposals_viewed(&from_alice, false).is_empty(),
        "an unfocused window shows nothing"
    );
    bob.model
        .surface_mut()
        .batches
        .push_back(shown.into_iter().map(ViewEvent::Intent).collect());
    bob.pump(5).await;
    assert!(bob.failed().is_empty(), "{:?}", bob.failed());
    assert_eq!(
        bob.model().proposals(&from_alice).len(),
        1,
        "still offered this session"
    );
    assert!(
        bob.model().proposals_viewed(&from_alice, true).is_empty(),
        "read once"
    );
    Box::pin(bob.facade.close()).await;
    drop(bob);
    let store = HumanStore::open_profile(&pb.paths, &FilesystemCustody, StoreOptions::default())
        .expect("reopen");
    assert!(
        store.unread_inbound().expect("read").is_empty(),
        "no durable copy once shown"
    );
    drop(store);
    let bob = Root::new(facade(&b, &pb, None));
    assert!(
        bob.model().proposals(&from_alice).is_empty(),
        "not offered again"
    );
}

#[tokio::test]
async fn a_shown_card_is_still_accepted_from_the_sessions_copy() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let from_alice = direct(a.peer());
    let shown = bob.model().proposals_viewed(&from_alice, true);
    bob.model
        .surface_mut()
        .batches
        .push_back(shown.into_iter().map(ViewEvent::Intent).collect());
    bob.pump(5).await;
    let row = bob.model().proposals(&from_alice)[0].row;
    bob.will(Intent::AcceptCard(row));
    bob.pump(6).await;
    assert!(bob.failed().is_empty(), "{:?}", bob.failed());
    assert!(bob.model().proposals(&from_alice).is_empty());
    assert_eq!(bob.model().contacts().len(), 1, "accepted");
    assert_eq!(bob.model().title(&from_alice), "Ada Lovelace");
}

#[tokio::test]
async fn two_cards_on_one_route_before_any_accept_never_take_the_second_key_unseen() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    let first_key = key_of(&pa).fingerprint().to_string();
    alice.will(Intent::ShareCard(direct(b.peer())));
    alice.pump(1).await;
    alice.pump(2).await;
    // A new key on Alice's side, and a second card, before Bob acts.
    identity_key::remove(&pa.paths, &human(), &FilesystemCustody).expect("removed");
    let second = key_of(&pa);
    let second_key = second.fingerprint().to_string();
    alice.facade.set_identity_key(second);
    alice.will(Intent::ShareCard(direct(b.peer())));
    alice.pump(3).await;
    alice.pump(4).await;
    bob.pump(5).await;
    let from_alice = direct(a.peer());
    let both = bob.model().proposals(&from_alice);
    assert_eq!(both.len(), 2, "{:?}", bob.updates);
    assert!(
        both.iter().all(|p| p.key == KeyView::New),
        "both shown against an empty route"
    );
    let (one, two) = if both[0].fingerprint == first_key {
        (both[0].row, both[1].row)
    } else {
        (both[1].row, both[0].row)
    };
    // Both accepted before the facade has run: the second goes as it was
    // shown, new, which the first's Accept made untrue; it is refused and
    // given again.
    bob.will(Intent::AcceptCard(one));
    bob.will(Intent::AcceptCard(two));
    bob.pump(7).await;
    assert!(
        bob.failed()
            .iter()
            .any(|(_, why)| *why == Failure::ProposalChanged),
        "{:?}",
        bob.failed()
    );
    assert_eq!(
        bob.model().contacts()[0].routes[0].fingerprint.as_deref(),
        Some(first_key.as_str()),
        "nothing written: the route keeps the first key"
    );
    let again = bob.model().proposals(&from_alice);
    assert_eq!(again.len(), 1);
    assert_eq!(
        again[0].key,
        KeyView::Changed {
            recorded: first_key.clone()
        },
        "given again as the key change it now is"
    );
    // Given again as judged, never as a card received anew, so a row the
    // model has let go is not brought back. Mutation: the refusal handing
    // it over as ReceivedCard.
    assert!(
        bob.updates
            .iter()
            .any(|u| matches!(u, Update::ProposalJudged(p) if p.row == two)),
        "{:?}",
        bob.updates
    );
    assert_eq!(
        bob.updates
            .iter()
            .filter(|u| matches!(u, Update::ReceivedCard(p) if p.row == two))
            .count(),
        1,
        "received once, when it arrived"
    );
    // Accepted as shown now, it writes: the copy was put back.
    bob.will(Intent::AcceptCard(two));
    bob.pump(8).await;
    assert_eq!(
        bob.model().contacts()[0].routes[0].fingerprint.as_deref(),
        Some(second_key.as_str())
    );
    assert!(bob.model().proposals(&from_alice).is_empty());
}

#[tokio::test]
async fn a_keep_of_a_shown_cards_copy_is_refused_and_the_card_stays_decidable() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    // A message too, for an item a Keep can name.
    alice.will(Intent::Send {
        key: direct(b.peer()),
        draft: "hello".to_owned(),
    });
    alice.pump(1).await;
    shared(&mut alice, &mut bob, &b).await;
    let from_alice = direct(a.peer());
    let item = bob.model().messages(&from_alice)[0].key;
    let row = bob.model().proposals(&from_alice)[0].row;
    for intent in bob.model().proposals_viewed(&from_alice, true) {
        let Intent::MarkRead(r) = intent else {
            panic!("{intent:?}")
        };
        bob.facade.execute(Command::MarkRead(r), 5).await;
    }
    let updates = bob
        .facade
        .execute(
            Command::Keep {
                item,
                from: (Table::Unread, row),
            },
            6,
        )
        .await;
    assert!(
        updates.iter().any(|u| matches!(
            u,
            Update::Failed {
                why: Failure::Refused,
                ..
            }
        )),
        "{updates:?}"
    );
    let shown = bob
        .model()
        .proposal_context(row)
        .expect("still offered")
        .clone();
    let updates = bob
        .facade
        .execute(
            Command::AcceptCard {
                row,
                shown: Box::new(shown),
            },
            7,
        )
        .await;
    assert!(
        updates
            .iter()
            .any(|u| matches!(u, Update::CardResolved { row: r, .. } if *r == row)),
        "the copy is still there for Accept: {updates:?}"
    );
    Box::pin(bob.facade.close()).await;
    drop(bob);
    let store = HumanStore::open_profile(&pb.paths, &FilesystemCustody, StoreOptions::default())
        .expect("reopen");
    assert!(
        store.kept_inbound().expect("read").is_empty(),
        "no card kept: nothing else was asked to be"
    );
}

/// A JPEG of `width` × `height`, written straight into the identity's
/// own card: a sender that skipped the re-encode, as a remote one may.
fn own_photo(profile: &Profile, width: u32, height: u32) {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::new(width, height))
        .write_to(&mut bytes, image::ImageFormat::Jpeg)
        .expect("encoded");
    let mut store =
        HumanStore::open_profile(&profile.paths, &FilesystemCustody, StoreOptions::default())
            .expect("store");
    store
        .set_own_card(
            &OwnCard {
                endpoint: human(),
                name: "Ada".to_owned(),
                surname: None,
                photo: Some(bytes.into_inner()),
            },
            WALL,
        )
        .expect("set");
}

#[tokio::test]
async fn accept_keeps_a_photo_only_when_it_decodes_within_256_by_256() {
    for (width, kept) in [(257, None), (256, Some(true))] {
        let (a, b) = FakeNetwork::pair(node(), node());
        let (pa, pb) = (profile(), profile());
        own_photo(&pa, width, 8);
        let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
        let mut bob = Root::new(facade(&b, &pb, None));
        alice.pump(0).await;
        bob.pump(0).await;
        alice.will(Intent::ShareCard(direct(b.peer())));
        alice.pump(1).await;
        alice.pump(2).await;
        bob.pump(3).await;
        let proposal = bob.model().proposals(&direct(a.peer()))[0].clone();
        assert!(proposal.has_photo, "the card carried one, {width} wide");
        bob.will(Intent::AcceptCard(proposal.row));
        bob.pump(4).await;
        assert!(bob.failed().is_empty(), "{:?}", bob.failed());
        let contact = &bob.model().contacts()[0];
        assert_eq!(contact.name, "Ada", "the card is kept");
        assert_eq!(contact.photo_from_card, kept, "{width} wide");
    }
}

#[tokio::test]
async fn share_without_a_key_sends_nothing() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let pa = profile();
    let mut alice = Root::new(facade(&a, &pa, None));
    alice.pump(0).await;
    alice.will(Intent::ShareCard(direct(b.peer())));
    alice.pump(1).await;
    assert_eq!(
        alice.failed(),
        vec![(Command::ShareCard(direct(b.peer())), Failure::NoIdentityKey)]
    );
    let store = HumanStore::open_profile(&pa.paths, &FilesystemCustody, StoreOptions::default())
        .expect("reopen");
    assert!(store.pending_outbound().expect("read").is_empty());
}

#[tokio::test]
async fn the_card_shared_to_two_conversations_is_the_identitys_each_time() {
    // The fake joins two nodes at a time, so the identity is held by two
    // of its roots, one per pair, with one key: what this shows is that
    // each card carries the identity's key and own card, not that one node
    // serves two conversations (the desktop e2e's).
    let (a1, b) = FakeNetwork::pair(node(), node());
    let (a2, c) = FakeNetwork::pair(node(), node());
    let (pa1, pa2, pb, pc) = (profile(), profile(), profile(), profile());
    let fingerprint = key_of(&pa1).fingerprint().to_string();
    let mut to_bob = Root::new(facade(&a1, &pa1, Some(key_of(&pa1))));
    let mut to_carol = Root::new(facade(&a2, &pa2, Some(key_of(&pa1))));
    let mut bob = Root::new(facade(&b, &pb, None));
    let mut carol = Root::new(facade(&c, &pc, None));
    for root in [&mut to_bob, &mut to_carol, &mut bob, &mut carol] {
        root.pump(0).await;
    }
    shared(&mut to_bob, &mut bob, &b).await;
    shared(&mut to_carol, &mut carol, &c).await;
    for (who, root, from) in [("bob", &bob, &a1), ("carol", &carol, &a2)] {
        let got = root.model().proposals(&direct(from.peer()));
        assert_eq!(got.len(), 1, "{who}");
        assert_eq!(got[0].name, "Ada Lovelace", "{who}");
        assert_eq!(
            got[0].fingerprint, fingerprint,
            "{who}: the conversation's identity's key"
        );
    }
}

#[tokio::test]
async fn a_changed_key_on_a_known_route_is_a_key_change_and_a_local_edit_stands() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let row = bob.model().proposals(&direct(a.peer()))[0].row;
    bob.will(Intent::AcceptCard(row));
    bob.pump(5).await;
    let contact_id = bob.model().contacts()[0].contact_id.clone();
    bob.will(Intent::RenameContact {
        contact_id: contact_id.clone(),
        name: "Countess".to_owned(),
    });
    bob.pump(6).await;
    let old = key_of(&pa).fingerprint().to_string();

    // Alice's identity gets a new key (a reinstall, say) and shares again.
    identity_key::remove(&pa.paths, &human(), &FilesystemCustody).expect("removed");
    let new_key = key_of(&pa);
    let new = new_key.fingerprint().to_string();
    alice.facade.set_identity_key(new_key);
    alice.will(Intent::ShareCard(direct(b.peer())));
    alice.pump(7).await;
    alice.pump(8).await;
    bob.pump(9).await;
    let proposal = bob.model().proposals(&direct(a.peer()))[0].clone();
    assert_eq!(
        proposal.key,
        KeyView::Changed { recorded: old },
        "shown as a key change"
    );
    assert_eq!(proposal.fingerprint, new);
    assert_eq!(proposal.accept, AcceptLine::Updates("Countess".to_owned()));
    // Accepted only by the person: until then the route keeps the old key.
    assert_ne!(
        bob.model().contacts()[0].routes[0].fingerprint.as_deref(),
        Some(new.as_str())
    );
    bob.will(Intent::AcceptCard(proposal.row));
    bob.pump(10).await;
    let contact = &bob.model().contacts()[0];
    assert_eq!(
        contact.routes[0].fingerprint.as_deref(),
        Some(new.as_str()),
        "the accepted key"
    );
    assert_eq!(contact.name, "Countess", "the local edit stands");
    assert!(
        !contact.name_from_card,
        "and the page says it is the person's"
    );
}

#[tokio::test]
async fn the_identity_key_never_enters_the_store() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, Some(key_of(&pb))));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let row = bob.model().proposals(&direct(a.peer()))[0].row;
    bob.will(Intent::AcceptCard(row));
    bob.pump(5).await;
    for root in [&mut alice, &mut bob] {
        Box::pin(root.facade.close()).await;
    }
    drop((alice, bob));

    for p in [&pa, &pb] {
        // The record's last 32 bytes are the secret at the filesystem floor.
        let line = std::fs::read_to_string(identity_key::key_path(&p.paths, &human()))
            .expect("the key file");
        let record = base64::engine::general_purpose::STANDARD
            .decode(line.trim())
            .expect("base64");
        let secret = &record[record.len() - 32..];
        let forms: [Vec<u8>; 3] = [
            secret.to_vec(),
            base64::engine::general_purpose::STANDARD
                .encode(secret)
                .into_bytes(),
            secret
                .iter()
                .fold(String::new(), |mut hex, b| {
                    use std::fmt::Write as _;
                    let _ = write!(hex, "{b:02x}");
                    hex
                })
                .into_bytes(),
        ];
        let mut scanned = Vec::new();
        for entry in std::fs::read_dir(p.paths.human_dir()).expect("the store's directory") {
            let path = entry.expect("entry").path();
            scanned.push(
                path.file_name()
                    .expect("a name")
                    .to_string_lossy()
                    .into_owned(),
            );
            let bytes = std::fs::read(&path).expect("read");
            for form in &forms {
                assert!(
                    !bytes.windows(form.len()).any(|w| w == form.as_slice()),
                    "{} holds the identity's private key",
                    path.display()
                );
            }
        }
        assert!(
            scanned
                .iter()
                .any(|n| n == interweave_human_store::STORE_FILE),
            "the control: the store's own file was read, {scanned:?}"
        );
    }
}

#[test]
fn the_model_holds_as_many_read_proposals_as_the_facade_holds_copies() {
    assert_eq!(
        interweave_human_ui_model::READ_PROPOSAL_CAP,
        interweave_human_app_core::READ_COPY_CAP
    );
}

/// A Keep is refused for a card by its kind, verified or not: a stored
/// card that would no longer verify on its route is not keepable either.
#[tokio::test]
async fn a_keep_of_a_card_that_no_longer_verifies_is_refused_by_its_kind() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let pb = profile();
    let row = {
        let mut store =
            HumanStore::open_profile(&pb.paths, &FilesystemCustody, StoreOptions::default())
                .expect("store");
        store
            .commit_unread_inbound(&NewInbound {
                app_message_id: AppMessageId::parse("0000000000000000000000000000000c".to_owned())
                    .expect("id"),
                origin: InboundOrigin {
                    peer: a.peer().clone(),
                    endpoint: Some(human()),
                    channel: None,
                },
                media_type: Some(
                    interweave_transport_api::MediaType::parse(
                        "application/vnd.interweave-human-chat+json;v=2",
                    )
                    .expect("media type"),
                ),
                // A card by its kind, with no key and no signature: one
                // `classify` gives as malformed.
                payload: br#"{"v":2,"kind":"contact_card","app_message_id":"0000000000000000000000000000000c","name":"Ada"}"#
                    .to_vec(),
                received_at: 1,
            })
            .expect("committed")
    };
    let mut bob = facade(&b, &pb, None);
    let read = bob.execute(Command::MarkRead(row), 1).await;
    assert!(
        read.iter()
            .any(|u| matches!(u, Update::Read(r) if *r == row)),
        "{read:?}"
    );
    let mut alice = Root::new(facade(&a, &profile(), None));
    let mut bob = Root::new(bob);
    alice.pump(0).await;
    bob.pump(0).await;
    alice.will(Intent::Send {
        key: direct(b.peer()),
        draft: "an item to name".to_owned(),
    });
    alice.pump(1).await;
    alice.pump(2).await;
    bob.pump(3).await;
    let item = bob.model().messages(&direct(a.peer()))[0].key;
    let updates = bob
        .facade
        .execute(
            Command::Keep {
                item,
                from: (Table::Unread, row),
            },
            3,
        )
        .await;
    assert!(
        updates.iter().any(|u| matches!(
            u,
            Update::Failed {
                why: Failure::Refused,
                ..
            }
        )),
        "{updates:?}"
    );
    Box::pin(bob.facade.close()).await;
    drop(bob);
    let store = HumanStore::open_profile(&pb.paths, &FilesystemCustody, StoreOptions::default())
        .expect("reopen");
    assert!(
        store.kept_inbound().expect("read").is_empty(),
        "nothing kept"
    );
}

#[tokio::test]
async fn a_deleted_contact_is_gone_and_the_next_card_from_its_route_is_a_new_contact() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let row = bob.model().proposals(&direct(a.peer()))[0].row;
    bob.will(Intent::AcceptCard(row));
    bob.pump(5).await;
    let contact_id = bob.model().contacts()[0].contact_id.clone();
    let delete = bob.model().delete_contact(&contact_id).expect("offered");
    bob.will(delete);
    bob.pump(6).await;
    assert!(bob.model().contacts().is_empty(), "deleted");
    assert!(bob.model().delete_contact(&contact_id).is_none());

    shared(&mut alice, &mut bob, &b).await;
    let proposal = bob.model().proposals(&direct(a.peer()))[0].clone();
    assert_eq!(proposal.accept, AcceptLine::Creates, "a new contact");
    assert_eq!(proposal.key, KeyView::New, "no key survived the delete");
}

/// A PNG of `w` × `h`, as a person's file might hold one.
fn png(w: u32, h: u32) -> Vec<u8> {
    let image = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([
            u8::try_from(x % 256).unwrap_or(0),
            u8::try_from(y % 256).unwrap_or(0),
            90,
        ])
    });
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .expect("a PNG");
    out
}

/// What the screens are given of a photo: the own card's, decoded, for
/// its edit; a proposal's, nothing but that there is one; an accepted
/// contact's, decoded within the bound, with where it came from and
/// when, and when the name came too (plan section 29's exit gate: the
/// contact page says where each value came from).
#[tokio::test]
async fn a_photo_reaches_the_screens_decoded_only_once_it_is_accepted_with_its_time() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, Some(key_of(&pb))));
    alice.pump(0).await;
    bob.pump(0).await;
    alice.will(Intent::SetOwnCard {
        name: "Ada".to_owned(),
        surname: None,
        photo: PhotoEdit::Chosen(png(400, 300)),
    });
    alice.pump(1).await;
    assert!(alice.failed().is_empty(), "{:?}", alice.failed());
    let own = alice.model().identity().expect("the identity").clone();
    let photo = own.photo.expect("the own card's photo, decoded");
    assert!(
        photo.width <= 256 && photo.height <= 256,
        "the card's bound"
    );
    assert_eq!(photo.rgba.len(), (photo.width * photo.height * 4) as usize);
    assert_eq!((own.given.as_str(), own.surname.as_deref()), ("Ada", None));
    alice.will(
        alice
            .model()
            .share_card(&direct(b.peer()))
            .expect("offered"),
    );
    alice.pump(2).await;
    alice.pump(3).await;
    bob.pump(4).await;

    let from_alice = direct(a.peer());
    let proposal = bob.model().proposals(&from_alice)[0].clone();
    assert!(proposal.has_photo, "the proposal says there is one");
    assert!(
        bob.model().contacts().is_empty(),
        "and nothing is decoded for it"
    );

    bob.will(Intent::AcceptCard(proposal.row));
    bob.pump(5).await;
    let contact = bob.model().contacts()[0].clone();
    let photo = contact.photo.expect("the accepted photo, decoded");
    assert!(photo.width <= 256 && photo.height <= 256);
    assert_eq!(contact.photo_from_card, Some(true));
    assert_eq!(contact.photo_at, Some(WALL), "when it came");
    assert_eq!(contact.name_at, Some(WALL), "when the name came");
}

/// Alice sets her card to `name` and `surname` with `photo` and shares it
/// with Bob again, from `t`; the proposal Bob is shown.
async fn shared_again(
    alice: &mut Root,
    bob: &mut Root,
    (a, b): (&FakeNode, &FakeNode),
    card: (&str, Option<&str>, PhotoEdit),
    t: u64,
) -> ProposalItem {
    alice.will(Intent::SetOwnCard {
        name: card.0.to_owned(),
        surname: card.1.map(str::to_owned),
        photo: card.2,
    });
    alice.pump(t).await;
    alice.will(Intent::ShareCard(direct(b.peer())));
    alice.pump(t + 1).await;
    alice.pump(t + 2).await;
    bob.pump(t + 3).await;
    let shown = bob.model().proposals(&direct(a.peer()));
    assert_eq!(shown.len(), 1, "one proposal at a time here");
    shown[0].clone()
}

// Case S (design/2026-10-21-contact-cards.md section 5.4.1): a card is
// Close alone when Accept would write nothing -- the key recorded and each
// value the person's or already the card's -- and Update whenever a value
// that is not the person's would change. Mutations: `changes_nothing`
// ignoring the person's name (the "Countess" case is Update); ignoring the
// photo (the photo case is Close). Ignoring the key is caught by
// `a_changed_key_on_a_known_route_is_a_key_change_and_a_local_edit_stands`,
// whose card matches the person's name and holds no photo.
#[tokio::test]
async fn a_card_that_changes_nothing_offers_close_alone_and_one_that_changes_a_card_value_updates()
{
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let first = bob.model().proposals(&direct(a.peer()))[0].clone();
    assert_eq!(first.accept, AcceptLine::Creates);
    bob.will(Intent::AcceptCard(first.row));
    bob.pump(5).await;
    let contact_id = bob.model().contacts()[0].contact_id.clone();

    // The same card again: nothing to write, so Close (Decline) alone.
    let same = shared_again(
        &mut alice,
        &mut bob,
        (&a, &b),
        ("Ada", Some("Lovelace"), PhotoEdit::Keep),
        10,
    )
    .await;
    assert_eq!(same.accept, AcceptLine::Matches("Ada Lovelace".to_owned()));
    assert_eq!(same.accept_text(), UiText::Close);
    assert_eq!(
        bob.model().card_actions(same.row),
        vec![Intent::DeclineCard(same.row)]
    );
    bob.will(Intent::DeclineCard(same.row));
    bob.pump(15).await;
    assert!(
        bob.model().proposals(&direct(a.peer())).is_empty(),
        "closed"
    );

    // A changed name the card gave: Update.
    let renamed = shared_again(
        &mut alice,
        &mut bob,
        (&a, &b),
        ("Ada", None, PhotoEdit::Keep),
        20,
    )
    .await;
    assert_eq!(
        renamed.accept,
        AcceptLine::Updates("Ada Lovelace".to_owned())
    );
    assert_eq!(
        bob.model().card_actions(renamed.row),
        vec![
            Intent::AcceptCard(renamed.row),
            Intent::DeclineCard(renamed.row)
        ]
    );
    bob.will(Intent::DeclineCard(renamed.row));
    bob.pump(25).await;

    // The person names the contact: a card's other name changes nothing.
    bob.will(Intent::RenameContact {
        contact_id,
        name: "Countess".to_owned(),
    });
    bob.pump(30).await;
    let kept = shared_again(
        &mut alice,
        &mut bob,
        (&a, &b),
        ("Ada", Some("Byron"), PhotoEdit::Keep),
        40,
    )
    .await;
    assert_eq!(kept.accept, AcceptLine::Matches("Countess".to_owned()));
    bob.will(Intent::DeclineCard(kept.row));
    bob.pump(45).await;

    // A photo the contact does not hold, and no person's photo: Update.
    let photo = shared_again(
        &mut alice,
        &mut bob,
        (&a, &b),
        ("Ada", Some("Byron"), PhotoEdit::Chosen(png(64, 64))),
        50,
    )
    .await;
    assert_eq!(photo.accept, AcceptLine::Updates("Countess".to_owned()));
}

// A held proposal is judged again when the person changes a contact: a
// card that matched a contact (case S, Close alone) offers Accept once
// that contact is deleted, since Accept would now make one -- whether the
// proposal was still unshown or shown and undecided. Mutation: the facade
// not judging held proposals again after a delete (Close alone stays).
#[tokio::test]
async fn a_matching_card_offers_accept_again_once_its_contact_is_deleted() {
    for shown_first in [false, true] {
        let (a, b) = FakeNetwork::pair(node(), node());
        let (pa, pb) = (profile(), profile());
        let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
        let mut bob = Root::new(facade(&b, &pb, None));
        alice.pump(0).await;
        bob.pump(0).await;
        shared(&mut alice, &mut bob, &b).await;
        let from_alice = direct(a.peer());
        let first = bob.model().proposals(&from_alice)[0].row;
        bob.will(Intent::AcceptCard(first));
        bob.pump(5).await;
        let contact_id = bob.model().contacts()[0].contact_id.clone();
        let same = shared_again(
            &mut alice,
            &mut bob,
            (&a, &b),
            ("Ada", Some("Lovelace"), PhotoEdit::Keep),
            10,
        )
        .await;
        assert_eq!(same.accept, AcceptLine::Matches("Ada Lovelace".to_owned()));
        if shown_first {
            let viewed = bob.model().proposals_viewed(&from_alice, true);
            assert!(!viewed.is_empty(), "shown, so read");
            bob.model
                .surface_mut()
                .batches
                .push_back(viewed.into_iter().map(ViewEvent::Intent).collect());
            bob.pump(15).await;
        }
        bob.will(Intent::DeleteContact { contact_id });
        bob.pump(20).await;
        assert!(bob.failed().is_empty(), "{:?}", bob.failed());
        let again = bob.model().proposals(&from_alice);
        assert_eq!(again.len(), 1, "shown first: {shown_first}");
        assert_eq!(
            again[0].accept,
            AcceptLine::Creates,
            "shown first: {shown_first}"
        );
        assert_eq!(
            bob.model().card_actions(again[0].row),
            vec![
                Intent::AcceptCard(again[0].row),
                Intent::DeclineCard(again[0].row)
            ]
        );
        bob.will(Intent::AcceptCard(again[0].row));
        bob.pump(25).await;
        assert!(bob.failed().is_empty(), "{:?}", bob.failed());
        assert_eq!(bob.model().contacts().len(), 1, "accepted as a new contact");
    }
}

// The same after an Accept: of two cards on one route, the older matches
// its contact (case S) until the newer, carrying another name, is
// accepted; then the older would write its name back, so it offers
// Accept. Mutation: the facade not judging held proposals again after an
// Accept.
#[tokio::test]
async fn a_matching_card_offers_accept_once_another_card_on_its_route_is_accepted() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let from_alice = direct(a.peer());
    let first = bob.model().proposals(&from_alice)[0].row;
    bob.will(Intent::AcceptCard(first));
    bob.pump(5).await;
    let older = shared_again(
        &mut alice,
        &mut bob,
        (&a, &b),
        ("Ada", Some("Lovelace"), PhotoEdit::Keep),
        10,
    )
    .await;
    assert_eq!(older.accept, AcceptLine::Matches("Ada Lovelace".to_owned()));
    // A second card on the route, with another surname.
    alice.will(Intent::SetOwnCard {
        name: "Ada".to_owned(),
        surname: Some("Byron".to_owned()),
        photo: PhotoEdit::Keep,
    });
    alice.pump(20).await;
    alice.will(Intent::ShareCard(direct(b.peer())));
    alice.pump(21).await;
    alice.pump(22).await;
    bob.pump(23).await;
    let both = bob.model().proposals(&from_alice);
    assert_eq!(both.len(), 2);
    let newer = both
        .iter()
        .find(|p| p.row != older.row)
        .expect("the newer")
        .clone();
    assert_eq!(newer.accept, AcceptLine::Updates("Ada Lovelace".to_owned()));
    bob.will(Intent::AcceptCard(newer.row));
    bob.pump(25).await;
    assert!(bob.failed().is_empty(), "{:?}", bob.failed());
    let left = bob.model().proposals(&from_alice);
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].row, older.row);
    assert_eq!(left[0].accept, AcceptLine::Updates("Ada Byron".to_owned()));
    assert_eq!(
        bob.model().card_actions(older.row),
        vec![
            Intent::AcceptCard(older.row),
            Intent::DeclineCard(older.row)
        ]
    );
}

impl Root {
    fn view(&mut self, event: ViewEvent) {
        self.model.surface_mut().batches.push_back(vec![event]);
    }
}

/// Contact-cards design 5.4.2 through the loop: Accept saves the contact,
/// the snackbar's Undo deletes it with its route, and the proposal comes
/// back read and decidable from the session's copy -- an Accept after the
/// Undo saves it again.
#[tokio::test]
async fn undo_of_an_accept_deletes_the_contact_and_the_proposal_returns_to_accept_again() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let from_alice = direct(a.peer());
    let row = bob.model().proposals(&from_alice)[0].row;
    bob.will(Intent::AcceptCard(row));
    bob.pump(5).await;
    assert_eq!(bob.model().contacts().len(), 1);
    assert_eq!(bob.model().card_answers(&from_alice).len(), 1);
    let snack = bob.model().snackbar().expect("Contact saved");
    bob.view(ViewEvent::Undo(snack.id));
    bob.pump(6).await;
    assert!(bob.failed().is_empty(), "{:?}", bob.failed());
    assert!(bob.model().contacts().is_empty(), "the contact is gone");
    assert!(bob.model().card_answers(&from_alice).is_empty());
    assert_eq!(bob.model().proposals(&from_alice).len(), 1, "back");
    let store = HumanStore::open_profile(&pb.paths, &FilesystemCustody, StoreOptions::default())
        .expect("reopen");
    assert!(store.contacts().expect("read").is_empty(), "nothing stored");
    assert!(
        store.unread_inbound().expect("read").is_empty(),
        "and no durable copy came back: the proposal is read"
    );
    drop(store);

    bob.will(Intent::AcceptCard(row));
    bob.pump(7).await;
    assert!(bob.failed().is_empty(), "{:?}", bob.failed());
    assert_eq!(bob.model().contacts().len(), 1, "saved again");
}

/// 5.4.3: Undo of a Decline brings the proposal back; once a snackbar
/// closes, the facade side lets the card go, and an Undo then finds
/// nothing to restore.
#[tokio::test]
async fn undo_of_a_decline_returns_the_proposal_and_a_closed_snackbar_lets_the_card_go() {
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let from_alice = direct(a.peer());
    let row = bob.model().proposals(&from_alice)[0].row;
    bob.will(Intent::DeclineCard(row));
    bob.pump(5).await;
    assert!(bob.model().proposals(&from_alice).is_empty());
    let snack = bob.model().snackbar().expect("Card declined");
    bob.view(ViewEvent::Undo(snack.id));
    bob.pump(6).await;
    assert_eq!(bob.model().proposals(&from_alice).len(), 1, "back");

    bob.will(Intent::DeclineCard(row));
    bob.pump(7).await;
    let snack = bob.model().snackbar().expect("Card declined");
    bob.view(ViewEvent::SnackbarClosed(snack.id));
    bob.pump(8).await;
    let updates = bob.facade.execute(Command::UndoCard(row), 9).await;
    assert!(
        updates.iter().any(|u| matches!(
            u,
            Update::Failed {
                why: Failure::NoSuchRow,
                ..
            }
        )),
        "the card was let go: {updates:?}"
    );
}

/// A facade whose identity key comes from its file through `cipher`, as
/// every root composes it (design/2026-10-24-identity-key.md).
fn facade_with_source(
    node: &FakeNode,
    profile: &Profile,
    cipher: Box<dyn identity_key::KeyCipher + Send>,
) -> FacadeSide<FakeNode, FakeNode> {
    let mut side = facade(node, profile, None);
    side.set_key_source(interweave_human_app_core::KeySource::new(
        profile.paths.clone(),
        human(),
        cipher,
    ));
    side
}

fn own_key(root: &Root) -> interweave_human_ui_model::OwnKey {
    root.model().identity().expect("the identity").key.clone()
}

/// Design sections 3, 4.1 and 5, acceptance items 2, 5, 6 and 7 at the
/// facade: a damaged key file leaves the identity keyless and named so,
/// its card still saves, Share is refused, and a confirmed new key gives a
/// new fingerprint and lists the conversation the card was shared in --
/// whose counterpart then sees the next card as a key change.
#[tokio::test]
async fn a_damaged_key_is_named_the_card_still_saves_and_a_new_key_lists_where_to_share_again() {
    use interweave_human_ui_model::{KeyProblem, KeyStep, OwnKey};
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade_with_source(&a, &pa, Box::new(FilesystemCustody)));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    let OwnKey::Ready { fingerprint: old } = own_key(&alice) else {
        panic!("a first key: {:?}", own_key(&alice))
    };
    shared(&mut alice, &mut bob, &b).await;
    let from_alice = direct(a.peer());
    let row = bob.model().proposals(&from_alice)[0].row;
    bob.will(Intent::AcceptCard(row));
    bob.pump(5).await;
    drop(alice);

    // Alice's key file is damaged; she starts again.
    let file = identity_key::key_path(&pa.paths, &human());
    std::fs::write(&file, "not a record\n").expect("damage");
    let mut alice = Root::new(facade_with_source(&a, &pa, Box::new(FilesystemCustody)));
    alice.pump(6).await;
    assert_eq!(own_key(&alice), OwnKey::Unusable(KeyProblem::Damaged));
    alice.will(Intent::SetOwnCard {
        name: "Ada".to_owned(),
        surname: Some("King".to_owned()),
        photo: PhotoEdit::Keep,
    });
    alice.pump(7).await;
    assert!(
        alice.failed().is_empty(),
        "Save needs no key: {:?}",
        alice.failed()
    );
    assert_eq!(alice.model().identity().expect("shown").name, "Ada King");
    let to_bob = direct(b.peer());
    alice.will(alice.model().share_card(&to_bob).expect("offered"));
    alice.pump(8).await;
    assert!(
        alice
            .failed()
            .iter()
            .any(|(_, why)| *why == Failure::NoIdentityKey),
        "nothing is signed without a key"
    );

    alice.will(Intent::AskNewKey);
    alice.pump(9).await;
    assert_eq!(
        alice.model().key_step(),
        Some(&KeyStep::Confirming { pending: false })
    );
    alice.will(Intent::MakeNewKey);
    alice.pump(10).await;
    let OwnKey::Ready { fingerprint: new } = own_key(&alice) else {
        panic!("made: {:?}", own_key(&alice))
    };
    assert_ne!(new, old, "a new key");
    assert_eq!(
        alice.model().key_step(),
        Some(&KeyStep::Made {
            share_again: vec![to_bob.clone()]
        }),
        "the conversation the card went to, and no other"
    );

    // Bob's client shows the next card as a key change.
    alice.will(alice.model().share_card(&to_bob).expect("offered"));
    alice.pump(11).await;
    alice.pump(12).await;
    bob.pump(13).await;
    let proposals = bob.model().proposals(&from_alice);
    assert!(
        proposals
            .iter()
            .any(|p| matches!(&p.key, KeyView::Changed { recorded } if *recorded == old)),
        "a key change, the old fingerprint named: {proposals:?}"
    );
}

/// Acceptance items 9 and 11 at the facade: a key file gone after a
/// share is missing and nothing makes one by itself; a new key that
/// cannot be stored after the old file went says the saved key is gone.
#[tokio::test]
async fn a_missing_key_after_a_share_is_named_and_a_new_key_not_stored_says_the_old_one_went() {
    use interweave_human_ui_model::{KeyProblem, KeyStep, OwnKey};
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade_with_source(&a, &pa, Box::new(FilesystemCustody)));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    drop(alice);
    let file = identity_key::key_path(&pa.paths, &human());
    std::fs::remove_file(&file).expect("lost");
    let mut alice = Root::new(facade_with_source(&a, &pa, Box::new(NoSeal)));
    alice.pump(5).await;
    assert_eq!(own_key(&alice), OwnKey::Unusable(KeyProblem::Missing));
    assert!(!file.exists(), "no key made by itself");

    std::fs::write(&file, "not a record\n").expect("a file to go");
    std::fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o600))
        .expect("chmod");
    alice.will(Intent::AskNewKey);
    alice.pump(6).await;
    alice.will(Intent::MakeNewKey);
    alice.pump(7).await;
    assert_eq!(
        alice.model().key_step(),
        Some(&KeyStep::Failed { gone: true })
    );
    assert_eq!(own_key(&alice), OwnKey::Unusable(KeyProblem::Missing));
    assert!(!file.exists(), "the old file went first");
}

/// A cipher at the desktop's custody that cannot seal: a new key cannot
/// be stored under it.
struct NoSeal;

impl identity_key::KeyCipher for NoSeal {
    fn custody(&self) -> identity_key::Custody {
        identity_key::Custody::FilesystemOnly
    }

    fn seal(
        &self,
        _: &str,
        _: &[u8],
        _: &[u8; 32],
    ) -> Result<Vec<u8>, identity_key::CipherFailure> {
        Err(identity_key::CipherFailure::Platform("no seal".to_owned()))
    }

    fn open(
        &self,
        alias: &str,
        aad: &[u8],
        sealed: &[u8],
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, identity_key::CipherFailure> {
        FilesystemCustody.open(alias, aad, sealed)
    }

    fn retire(&self, _: &str) -> Result<(), identity_key::CipherFailure> {
        Ok(())
    }
}

/// #423 F1 through the loop: a Make the facade refuses before it starts
/// ends the step as not made, never one the page waits on.
#[tokio::test]
async fn a_make_the_facade_refuses_ends_the_step_not_made() {
    use interweave_human_ui_model::{IdentityView, KeyProblem, KeyStep, OwnKey};
    let (a, _b) = FakeNetwork::pair(node(), node());
    let pa = profile();
    let mut alice = Root::new(facade(&a, &pa, None));
    alice.model.apply(Update::IdentityShown(IdentityView {
        endpoint: human(),
        name: "Ada".to_owned(),
        given: "Ada".to_owned(),
        surname: None,
        has_photo: false,
        photo: None,
        key: OwnKey::Unusable(KeyProblem::Damaged),
        peer: None,
    }));
    alice.will(Intent::AskNewKey);
    alice.pump(1).await;
    assert_eq!(
        alice.model().key_step(),
        Some(&KeyStep::Confirming { pending: false })
    );
    alice.will(Intent::MakeNewKey);
    alice.pump(2).await;
    assert_eq!(
        alice.model().key_step(),
        Some(&KeyStep::Failed { gone: false }),
        "refused: not made, Make a new key offered again"
    );
}

/// Contact-cards design 5.4.1 cases U and E, through the loop: a later
/// card with a new name proposes that name for the contact; once the
/// person names the contact themselves, the same card says that name stays
/// and proposes none.
#[tokio::test]
async fn a_later_card_names_the_values_it_changes_and_the_edit_it_keeps() {
    use interweave_human_ui_model::ValueChanges;
    let (a, b) = FakeNetwork::pair(node(), node());
    let (pa, pb) = (profile(), profile());
    let mut alice = Root::new(facade(&a, &pa, Some(key_of(&pa))));
    let mut bob = Root::new(facade(&b, &pb, None));
    alice.pump(0).await;
    bob.pump(0).await;
    shared(&mut alice, &mut bob, &b).await;
    let from_alice = direct(a.peer());
    let first = bob.model().proposals(&from_alice)[0].row;
    bob.will(Intent::AcceptCard(first));
    bob.pump(5).await;
    let contact_id = bob.model().contacts()[0].contact_id.clone();

    let share_again = |alice: &mut Root, name: &str| {
        alice.will(Intent::SetOwnCard {
            name: name.to_owned(),
            surname: None,
            photo: PhotoEdit::Keep,
        });
    };
    share_again(&mut alice, "Ada King");
    alice.pump(6).await;
    alice.will(
        alice
            .model()
            .share_card(&direct(b.peer()))
            .expect("offered"),
    );
    alice.pump(7).await;
    alice.pump(8).await;
    bob.pump(9).await;
    let proposal = bob.model().proposals(&from_alice)[0].clone();
    assert_eq!(
        proposal.values,
        ValueChanges {
            new_name: Some("Ada King".to_owned()),
            ..ValueChanges::default()
        },
        "case U: the new name named"
    );
    bob.will(Intent::DeclineCard(proposal.row));
    bob.pump(10).await;

    bob.will(Intent::RenameContact {
        contact_id,
        name: "Countess".to_owned(),
    });
    bob.pump(11).await;
    alice.will(
        alice
            .model()
            .share_card(&direct(b.peer()))
            .expect("offered"),
    );
    alice.pump(12).await;
    alice.pump(13).await;
    bob.pump(14).await;
    let proposal = bob
        .model()
        .proposals(&from_alice)
        .last()
        .expect("a proposal")
        .clone();
    assert_eq!(
        proposal.values,
        ValueChanges {
            kept_name: true,
            ..ValueChanges::default()
        },
        "case E: the person's name stays, no new name proposed"
    );
}
