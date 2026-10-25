// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Contact cards, drawn (plan section 29 step 4;
//! `design/2026-10-21-contact-cards.md`; `human-client-ui.md` section 13):
//! a proposal in its conversation, the Contacts list, a contact's page,
//! the own card and the share sheet, driven as a screen reader drives
//! them, by each control's accessible label and default action, with the
//! model given what the facade hands over. What the window shows and asks
//! is checked here; what the store and the facade do with an answer is
//! app-core's (`tests/cards.rs` there), and no platform's accessibility
//! tool reads this tree here.

#![allow(clippy::expect_used, clippy::panic)]

use i_slint_backend_testing::ElementHandle;
use interweave_human_chat_protocol::{Card, CardPhoto, CardRoute, Classified, classify};
use interweave_human_core::RowId;
use interweave_human_ui_model::{
    CardOutcome, ContactRef, ContactView, ConversationKey, IdentityView, Intent, KeyView, Photo,
    PhotoEdit, Proposal, ProposalContext, RouteView, RuntimeHost, UiModel, UiText, fill, isolated,
    placeholder_en,
};
use interweave_human_ui_slint::{View, ViewEvent};
use interweave_transport_api::{ChannelId, EndpointId, TransportIdentity};
use slint::{ComponentHandle as _, Model as _};

const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
const Q: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";
const R: &str = "12D3KooWAkJJFJbqnb5JovQcRRUpVTqX2o8v2P3tg9wHSpnZmYjp";
/// 2026-10-24 12:00 UTC.
const NOW: u64 = 1_792_843_200_000;

fn peer(s: &str) -> TransportIdentity {
    TransportIdentity::parse(s).expect("a PeerId")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("an endpoint")
}

fn direct(p: &str) -> ConversationKey {
    ConversationKey::Direct {
        peer: peer(p),
        endpoint: Some(human()),
    }
}

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

fn view() -> View {
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Daemon).expect("a window");
    view.set_clock(|| NOW, |_| 0);
    view.window().show().expect("shown");
    view
}

/// The root's loop: take, apply, take again, until empty.
fn intents(view: &mut View, model: &mut UiModel) -> Vec<Intent> {
    let mut out = Vec::new();
    loop {
        let events = view.take_events(model);
        if events.is_empty() {
            view.render(model);
            return out;
        }
        for event in events {
            match event {
                ViewEvent::Intent(intent) => {
                    if let Intent::MarkRead(row) = &intent {
                        model.read(*row);
                    }
                    out.push(intent);
                }
                ViewEvent::DraftChanged { key, draft } => model.draft_changed(key, draft),
                ViewEvent::Trust(input) => out.extend(model.trust_settings_mut().input(input)),
                ViewEvent::Peers(input) => out.extend(model.peers_input(input)),
                ViewEvent::Undo(id) => out.extend(model.undo(id)),
                ViewEvent::SnackbarClosed(id) => model.snackbar_closed(id),
                ViewEvent::IdentifierCopied(key) => model.identifier_copied(key),
            }
        }
        view.render(model);
    }
}

fn announcement(e: &ElementHandle) -> bool {
    e.id()
        .is_some_and(|id| id.ends_with("announce-a") || id.ends_with("announce-b"))
}

fn labelled(view: &View, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view.window(), label)
        .filter(|e| !announcement(e))
        .collect()
}

fn shows(view: &View, label: &str) -> bool {
    !labelled(view, label).is_empty()
}

fn press(view: &View, label: &str) {
    let found = labelled(view, label);
    assert!(!found.is_empty(), "a control labelled {label:?}");
    found[0].invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
}

/// Let a view's one-shot timers run: a dialog's first focus. Two turns,
/// since a row created on the first turn starts its own timer then.
fn settle() {
    for _ in 0..2 {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(5));
        slint::platform::update_timers_and_animations();
    }
}

/// Every element the window draws as an image.
fn images(view: &View) -> usize {
    ElementHandle::find_by_element_type_name(view.window(), "Image")
        .filter(|e| {
            let size = e.size();
            size.width > 0.0 && size.height > 0.0
        })
        .count()
}

fn row(n: i64) -> RowId {
    RowId::from_stored(n)
}

fn context() -> ProposalContext {
    ProposalContext {
        updates: None,
        shares_node: Vec::new(),
        same_name: Vec::new(),
        key_matches: Vec::new(),
        key: KeyView::New,
        fingerprint: "0123456789abcdef".to_owned(),
        changes_nothing: false,
        values: interweave_human_ui_model::ValueChanges::default(),
    }
}

fn card(name: &str, photo: bool) -> Card {
    Card {
        app_message_id: "0".repeat(32),
        name: name.to_owned(),
        surname: None,
        photo: photo.then(|| CardPhoto {
            media_type: "image/png",
            bytes: vec![0x89, b'P', b'N', b'G'],
        }),
        identity_key: [1; 32],
        sent_at_ms: None,
        from_endpoint: None,
    }
}

fn proposal(n: i64, p: &str, card: Card, context: ProposalContext) -> Proposal {
    Proposal {
        row: row(n),
        peer: peer(p),
        endpoint: human(),
        card,
        received_at: NOW,
        context,
    }
}

fn photo() -> Photo {
    Photo {
        width: 4,
        height: 4,
        rgba: vec![200; 64],
    }
}

fn contact(id: &str, name: &str, p: &str) -> ContactView {
    ContactView {
        contact_id: id.to_owned(),
        name: name.to_owned(),
        name_from_card: true,
        name_at: Some(NOW),
        photo_from_card: None,
        photo_at: None,
        photo: None,
        routes: vec![RouteView {
            peer: peer(p),
            endpoint: human(),
            fingerprint: Some("0123456789abcdef".to_owned()),
        }],
    }
}

/// The conversation with `p` shown, the window focused.
fn shown(view: &mut View, model: &mut UiModel, p: &str) -> Vec<Intent> {
    view.set_window_focused(true);
    view.select(direct(p));
    intents(view, model)
}

/// Section 13 bullets 1 and 3: before Accept the claim is never without
/// the full `PeerId` beside it, the row says a photo is there and draws
/// none -- no image anywhere in the window -- and showing the
/// conversation reads it. The photo is drawn once the card is accepted and
/// the contact holds it, and not before. Mutation: an image drawn for the
/// proposal's photo line, or the full `PeerId` swapped for the short one.
#[test]
fn a_proposal_names_the_claim_beside_the_full_peer_id_and_draws_no_image_before_accept() {
    let mut view = view();
    let mut model = UiModel::new();
    model.card_received(proposal(1, P, card("Ada", true), context()));
    let raised = shown(&mut view, &mut model, P);
    assert!(
        raised.contains(&Intent::MarkRead(row(1))),
        "shown is read: {raised:?}"
    );
    let heading = fill(text(UiText::CardProposal), &[("name", &isolated("Ada"))]);
    assert!(shows(&view, &heading), "the claim, as a claim");
    assert!(shows(&view, P), "the full PeerId beside it");
    assert!(shows(&view, text(UiText::CardIncludesPhoto)));
    assert_eq!(images(&view), 0, "no image before Accept");

    press(&view, text(UiText::CardAccept));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::AcceptCard(row(1))]
    );
    // What the facade hands over after Accept: the proposal gone, the
    // contact with its photo decoded.
    model.card_resolved(
        row(1),
        CardOutcome::Accepted {
            contact_id: "c1".to_owned(),
            created: true,
            photo_dropped: false,
            photo_removed: false,
        },
    );
    let mut ada = contact("c1", "Ada", P);
    ada.photo = Some(photo());
    ada.photo_from_card = Some(true);
    ada.photo_at = Some(NOW);
    model.contacts_listed(vec![ada]);
    view.render(&model);
    assert!(!shows(&view, &heading), "the proposal is gone");
    assert!(
        shows(
            &view,
            &fill(text(UiText::CardAccepted), &[("name", &isolated("Ada"))])
        ),
        "its confirmation row stands in its place (design 5.4.2)"
    );
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::ContactsTitle));
    let _ = intents(&mut view, &mut model);
    assert!(images(&view) >= 1, "the accepted photo is drawn");
}

/// Section 13 bullets 2 and 4: Accept's label and context line say what it
/// will do before it runs -- Update contact naming the contact it updates,
/// a same-name contact on another `PeerId` named -- and the row offers
/// Accept, Decline and Copy `PeerId`, no Keep; Decline answers its own row.
/// Mutation: the context line dropped, or a Keep offered.
#[test]
fn accepts_label_and_context_say_what_it_will_do_and_decline_answers_its_row() {
    let mut view = view();
    let mut model = UiModel::new();
    model.card_received(proposal(
        1,
        P,
        card("Ada", false),
        ProposalContext {
            updates: Some(ContactRef {
                contact_id: "c1".to_owned(),
                name: "Countess".to_owned(),
            }),
            same_name: vec![ContactRef {
                contact_id: "c2".to_owned(),
                name: "Ada".to_owned(),
            }],
            ..context()
        },
    ));
    let _ = shown(&mut view, &mut model, P);
    assert!(shows(&view, text(UiText::UpdateContact)));
    assert!(!shows(&view, text(UiText::CardAccept)));
    assert!(shows(
        &view,
        &fill(
            text(UiText::CardUpdates),
            &[("name", &isolated("Countess"))]
        )
    ));
    assert!(shows(
        &view,
        &fill(text(UiText::CardSameName), &[("contact", &isolated("Ada"))])
    ));
    assert!(shows(&view, text(UiText::CopyPeerId)));
    assert!(shows(&view, &received_noon()), "when the card came");
    assert!(!shows(&view, text(UiText::Keep)), "a proposal has no Keep");
    press(&view, text(UiText::CardDecline));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::DeclineCard(row(1))]
    );
}

/// What the row says of when its card came, at the tests' clock (NOW is
/// 2026-10-24 12:00 UTC, the offset zero).
fn received_noon() -> String {
    fill(
        text(UiText::CardReceivedAt),
        &[("date", "Oct 24"), ("time", "12:00")],
    )
}

// Case S (design/2026-10-21-contact-cards.md section 5.4.1): a card that
// changes nothing says it matches the contact, offers Close alone in
// Accept's place, says nothing of being removed at close, and Close
// answers as Decline. The row says when the card came, as every proposal
// does, since it is drawn after the messages. Mutations: Decline drawn
// beside Close; Close raising Accept; the received line not drawn.
#[test]
fn a_card_that_changes_nothing_offers_close_alone_and_says_when_it_came() {
    let mut view = view();
    let mut model = UiModel::new();
    model.card_received(proposal(
        1,
        P,
        card("Ada", false),
        ProposalContext {
            updates: Some(ContactRef {
                contact_id: "c1".to_owned(),
                name: "Countess".to_owned(),
            }),
            key: KeyView::Same,
            changes_nothing: true,
            values: interweave_human_ui_model::ValueChanges::default(),
            ..context()
        },
    ));
    let _ = shown(&mut view, &mut model, P);
    assert!(shows(
        &view,
        &fill(
            text(UiText::CardSame),
            &[("contact", &isolated("Countess"))]
        )
    ));
    assert!(shows(&view, text(UiText::Close)));
    assert!(
        !shows(&view, text(UiText::CardDecline)),
        "no Decline in case S"
    );
    assert!(!shows(&view, text(UiText::UpdateContact)));
    assert!(!shows(&view, text(UiText::CardGoesAtClose)));
    assert!(shows(&view, &received_noon()));
    press(&view, text(UiText::Close));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::DeclineCard(row(1))]
    );
}

/// Every element of the window, in the tree's order: the order keyboard
/// focus and a screen reader follow.
fn in_order(view: &View) -> Vec<ElementHandle> {
    i_slint_backend_testing::ElementQuery::from_root(view.window())
        .match_descendants()
        .find_all()
}

/// The element labelled `label`'s place in the tree's reading order.
fn reading_index(view: &View, label: &str) -> usize {
    let order: Vec<String> = in_order(view)
        .iter()
        .filter_map(|e| e.accessible_label().map(|l| l.to_string()))
        .collect();
    order
        .iter()
        .position(|l| l == label)
        .unwrap_or_else(|| panic!("{label:?} is not in the tree: {order:?}"))
}

/// Section 13 bullets 24 and 26: a key change is said in plain words
/// before Accept, the old fingerprint then the new, read before Accept in
/// the tree's order (the order keyboard focus and a screen reader follow),
/// and Accept is not where focus starts. Mutation: the key change drawn
/// after the buttons, or the fingerprints in the other order.
#[test]
fn a_key_change_is_read_before_accept_with_both_fingerprints_and_accept_has_no_focus() {
    let mut view = view();
    let mut model = UiModel::new();
    model.card_received(proposal(
        1,
        P,
        card("Ada", false),
        ProposalContext {
            key: KeyView::Changed {
                recorded: "fedcba9876543210".to_owned(),
            },
            ..context()
        },
    ));
    let _ = shown(&mut view, &mut model, P);
    let change = fill(
        text(UiText::CardKeyChange),
        &[("old", "fedcba9876543210"), ("new", "0123456789abcdef")],
    );
    assert!(shows(&view, &change), "the key change, old then new");
    assert!(
        reading_index(&view, &change) < reading_index(&view, text(UiText::CardAccept)),
        "read before Accept"
    );
    assert!(
        reading_index(&view, &change) < reading_index(&view, text(UiText::CardDecline)),
        "and before Decline"
    );
    assert!(
        !view.window().get_focus_id().starts_with("proposal-accept"),
        "Accept is not the initial focus"
    );
    // Reached by its control, as a keyboard reaches it.
    press(&view, text(UiText::CardAccept));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::AcceptCard(row(1))]
    );
}

fn open_contact(view: &mut View, model: &mut UiModel, name: &str) {
    press(view, text(UiText::AppOptions));
    press(view, text(UiText::ContactsTitle));
    let _ = intents(view, model);
    press(view, &isolated(name));
    let _ = intents(view, model);
    settle();
}

/// Plan section 29's exit gate and section 13 bullet 8: the contact page
/// says where each value came from and when -- a card's name and photo,
/// or the person's -- and a later card's Accept, as the facade lists the
/// contact after it, leaves the person's name on the page as theirs.
/// Mutation: the sources swapped, or the date left out.
#[test]
fn the_contact_page_says_where_each_value_came_from_and_a_local_edit_stands() {
    let mut view = view();
    let mut model = UiModel::new();
    let mut ada = contact("c1", "Ada", P);
    ada.photo = Some(photo());
    ada.photo_from_card = Some(true);
    ada.photo_at = Some(NOW);
    model.contacts_listed(vec![ada.clone()]);
    view.render(&model);
    open_contact(&mut view, &mut model, "Ada");
    // `{date}` is the calendar date, never the list's "12:00".
    assert_eq!(
        view.window().get_contact_name_source(),
        fill(text(UiText::ContactNameFromCard), &[("date", "Oct 24")])
    );
    assert!(
        view.window()
            .get_contact_photo_source()
            .starts_with("Photo from their card, ")
    );

    // The person renames it; the facade answers with the contact listed
    // again, the name theirs.
    view.window().set_contact_name_draft("Countess".into());
    press(&view, text(UiText::Save));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::RenameContact {
            contact_id: "c1".to_owned(),
            name: "Countess".to_owned(),
        }]
    );
    let mut edited = ada.clone();
    edited.name = "Countess".to_owned();
    edited.name_from_card = false;
    model.contacts_listed(vec![edited.clone()]);
    view.render(&model);
    assert!(
        view.window()
            .get_contact_name_source()
            .starts_with("Name you entered, ")
    );
    // A later card from the route arrives and is accepted: the facade
    // lists the contact with the person's name kept (app-core's
    // `a_changed_key_on_a_known_route_is_a_key_change_and_a_local_edit_stands`).
    model.card_received(proposal(2, P, card("Ada again", false), context()));
    model.card_resolved(
        row(2),
        CardOutcome::Accepted {
            contact_id: "c1".to_owned(),
            created: false,
            photo_dropped: false,
            photo_removed: false,
        },
    );
    model.contacts_listed(vec![edited]);
    view.render(&model);
    assert_eq!(view.window().get_contact_name_draft(), "Countess");
    assert_eq!(view.window().get_contact_title(), isolated("Countess"));
    assert!(
        view.window()
            .get_contact_name_source()
            .starts_with("Name you entered, ")
    );
}

/// Section 13 bullet 10: every received name is drawn inside an isolate
/// wherever it sits in other text -- the proposal's claim and notes, the
/// Contacts list, a contact's page and the conversation's title.
/// Mutation: the isolate dropped from one of them.
#[test]
fn every_received_name_is_drawn_isolated() {
    let rtl = "\u{5d0}\u{5d3}\u{5d4}";
    let mut view = view();
    let mut model = UiModel::new();
    model.card_received(proposal(
        1,
        Q,
        card(rtl, false),
        ProposalContext {
            same_name: vec![ContactRef {
                contact_id: "c1".to_owned(),
                name: rtl.to_owned(),
            }],
            ..context()
        },
    ));
    model.contacts_listed(vec![contact("c1", rtl, P)]);
    let _ = shown(&mut view, &mut model, Q);
    assert!(shows(
        &view,
        &fill(text(UiText::CardProposal), &[("name", &isolated(rtl))])
    ));
    assert!(shows(
        &view,
        &fill(text(UiText::CardSameName), &[("contact", &isolated(rtl))])
    ));
    view.select(direct(P));
    let _ = intents(&mut view, &mut model);
    assert_eq!(view.window().get_header_title(), isolated(rtl), "the title");
    open_contact(&mut view, &mut model, rtl);
    assert_eq!(view.window().get_contact_title(), isolated(rtl));
    // A message from the contact puts its conversation in the list, whose
    // row draws the title isolated too. Mutation: the row drawing `title`.
    model.received(interweave_human_client_api::Received {
        row: row(2),
        origin: interweave_human_client_api::Origin::Direct {
            peer: peer(P),
            endpoint: human(),
        },
        envelope: interweave_human_chat_protocol::HumanChatV2 {
            v: 2,
            kind: interweave_human_chat_protocol::MessageKind::Text,
            app_message_id: "1".repeat(32),
            text: "hello".to_owned(),
            reply_to: None,
            sent_at_ms: None,
            from_endpoint: None,
        },
        received_at: NOW,
    });
    let _ = intents(&mut view, &mut model);
    let rows = view.window().get_conversations();
    assert!(
        (0..rows.row_count()).any(|i| rows.row_data(i).is_some_and(|r| r.title == isolated(rtl))),
        "the list row's title"
    );
}

/// The fixture's test-only key, as the card parser reads it from the
/// fixture's valid vector, and the fingerprint the vector states.
fn fixture_key() -> ([u8; 32], String) {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/human-chat-v2/human-chat-v2-contact-card.json"
    ))
    .expect("the fixture");
    let vector = &fixture["vectors"][0];
    let route_peer = peer(vector["received_on"]["peer_id"].as_str().expect("peer"));
    let route_endpoint = EndpointId::parse(
        vector["received_on"]["source_endpoint"]
            .as_str()
            .expect("e"),
    )
    .expect("endpoint");
    let envelope = serde_json::to_string(&vector["envelope"]).expect("json");
    let Classified::Card(card) = classify(
        &envelope,
        CardRoute {
            source_peer: &route_peer,
            source_endpoint: &route_endpoint,
        },
    ) else {
        panic!("the fixture's first vector is a valid card");
    };
    let fingerprint = vector["signer"]["fingerprint_hex"]
        .as_str()
        .expect("fingerprint")
        .to_owned();
    (card.identity_key, fingerprint)
}

/// Section 13 bullets 27 and 28: the identity page and the contact page
/// show the fingerprint ADR-0063 decision 1 defines beside the `PeerId` --
/// for the fixture's test key, the vector's -- and a typed route shows no
/// fingerprint and nothing shaped like one. Mutation: the fingerprint
/// label drawn for a route without one.
#[test]
fn the_fingerprint_shows_beside_the_peer_id_and_a_typed_route_shows_none() {
    let (key, vector) = fixture_key();
    let fingerprint = interweave_human_store::identity_key::Fingerprint::of(&key).to_string();
    assert_eq!(
        fingerprint, vector,
        "the client's fingerprint is the vector's"
    );
    let mut view = view();
    let mut model = UiModel::new();
    model.identity_shown(IdentityView {
        endpoint: human(),
        name: "Ada".to_owned(),
        given: "Ada".to_owned(),
        surname: None,
        has_photo: false,
        photo: None,
        key: interweave_human_ui_model::OwnKey::Ready {
            fingerprint: fingerprint.clone(),
        },
        peer: Some(peer(P)),
    });
    let mut carded = contact("c1", "Ada", Q);
    carded.routes[0].fingerprint = Some(fingerprint.clone());
    let mut typed = contact("c2", "Bob", P);
    typed.routes[0].fingerprint = None;
    typed.name_from_card = false;
    model.contacts_listed(vec![carded, typed]);
    view.render(&model);

    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::OwnCardTitle));
    let _ = intents(&mut view, &mut model);
    assert!(shows(&view, &fingerprint), "the identity page");
    assert!(shows(&view, P), "beside the PeerId");
    assert!(
        reading_index(&view, text(UiText::Fingerprint)) < reading_index(&view, P),
        "the fingerprint, then the PeerId"
    );

    press(&view, text(UiText::BackToConversations));
    let _ = intents(&mut view, &mut model);
    open_contact(&mut view, &mut model, "Ada");
    assert!(shows(&view, &fingerprint), "the contact page");
    assert!(shows(&view, Q));

    press(&view, text(UiText::BackToConversations));
    let _ = intents(&mut view, &mut model);
    open_contact(&mut view, &mut model, "Bob");
    assert!(shows(&view, P));
    assert!(
        !shows(&view, text(UiText::Fingerprint)),
        "a typed route: no fingerprint label"
    );
    let hexlike = |l: &str| l.len() == 16 && l.chars().all(|c| c.is_ascii_hexdigit());
    let found: Vec<String> = in_order(&view)
        .iter()
        .filter_map(|e| e.accessible_label().map(|l| l.to_string()))
        .filter(|l| hexlike(l))
        .collect();
    assert!(found.is_empty(), "nothing shaped like one: {found:?}");
}

/// Section 13 bullet 6: Share is offered in a direct conversation's menu
/// and never in a channel's; the sheet shows what goes and who gets it,
/// focus starts on Cancel, Cancel sends nothing, and Share sends this
/// conversation's card. Mutation: Share offered in a channel, or the
/// sheet's Share sent on open.
#[test]
fn share_shows_what_goes_first_and_sends_only_on_share_in_a_direct_conversation() {
    let mut view = view();
    let mut model = UiModel::new();
    model.identity_shown(IdentityView {
        endpoint: human(),
        name: "Ada Lovelace".to_owned(),
        given: "Ada".to_owned(),
        surname: Some("Lovelace".to_owned()),
        has_photo: false,
        photo: None,
        key: interweave_human_ui_model::OwnKey::Ready {
            fingerprint: "0123456789abcdef".to_owned(),
        },
        peer: Some(peer(Q)),
    });
    let _ = shown(&mut view, &mut model, P);
    press(&view, text(UiText::MoreOptions));
    press(&view, text(UiText::ShareCard));
    assert!(
        intents(&mut view, &mut model).is_empty(),
        "opening the sheet sends nothing"
    );
    let title = model.drawn_title(&direct(P));
    assert!(shows(
        &view,
        &fill(text(UiText::ShareCardTitle), &[("title", &title)])
    ));
    assert!(shows(&view, &isolated("Ada Lovelace")), "what goes");
    settle();
    assert_eq!(view.window().get_focus_id(), "share-cancel");
    press(&view, text(UiText::Cancel));
    assert!(
        intents(&mut view, &mut model).is_empty(),
        "Cancel sends nothing"
    );

    press(&view, text(UiText::MoreOptions));
    press(&view, text(UiText::ShareCard));
    assert!(
        intents(&mut view, &mut model).is_empty(),
        "opening the sheet sends nothing"
    );
    press(&view, text(UiText::ShareCardAction));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::ShareCard(direct(P))]
    );
    // The facade says it went; Details says when, as a date.
    model.card_shared(direct(P), NOW);
    view.render(&model);
    assert_eq!(
        view.window().get_details_card(),
        fill(text(UiText::DetailsCardShared), &[("date", "Oct 24")])
    );

    let room = ConversationKey::Channel(ChannelId::parse("room").expect("a channel"));
    view.select(room);
    let _ = intents(&mut view, &mut model);
    press(&view, text(UiText::MoreOptions));
    assert!(!shows(&view, text(UiText::ShareCard)), "never in a channel");
}

/// The own card: a name refused in place with the reason, focus on its
/// field and nothing sent; a valid edit saved with the photo kept; the
/// photo removed keeping the names.
/// Mutation: an empty name sent to the facade.
#[test]
fn the_own_card_refuses_an_empty_name_in_place_and_saves_a_valid_one() {
    let mut view = view();
    let mut model = UiModel::new();
    model.identity_shown(IdentityView {
        endpoint: human(),
        name: "Ada".to_owned(),
        given: "Ada".to_owned(),
        surname: None,
        has_photo: true,
        photo: Some(photo()),
        key: interweave_human_ui_model::OwnKey::Ready {
            fingerprint: "0123456789abcdef".to_owned(),
        },
        peer: Some(peer(Q)),
    });
    view.render(&model);
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::OwnCardTitle));
    let _ = intents(&mut view, &mut model);
    assert_eq!(view.window().get_own_given(), "Ada");
    assert!(images(&view) >= 1, "the own photo, as it is sent");

    view.window().set_own_given("   ".into());
    press(&view, text(UiText::Save));
    assert!(intents(&mut view, &mut model).is_empty(), "nothing sent");
    assert_eq!(
        view.window().get_own_given_error(),
        text(UiText::CardNameNeeded)
    );

    // Remove photo is an unsaved change (design 5.2 as #443 proposes it):
    // nothing written until Save. The model's side of it, as model_side
    // applies it.
    press(&view, text(UiText::RemovePhoto));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::RemoveOwnPhoto],
        "nothing saved yet"
    );
    model.own_photo_removed();

    view.window().set_own_given("Ada".into());
    view.window().set_own_surname("Lovelace".into());
    press(&view, text(UiText::Save));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::SetOwnCard {
            name: "Ada".to_owned(),
            surname: Some("Lovelace".to_owned()),
            photo: PhotoEdit::Remove,
        }],
        "the names and the removal, written together"
    );
}

/// Delete contact asks first, with focus on Cancel, deletes only on its
/// confirmation, and the page goes back to the list.
/// Mutation: the delete sent on the first press.
#[test]
fn delete_contact_asks_first_and_deletes_only_on_its_confirmation() {
    let mut view = view();
    let mut model = UiModel::new();
    model.contacts_listed(vec![contact("c1", "Ada", P)]);
    view.render(&model);
    open_contact(&mut view, &mut model, "Ada");
    press(&view, text(UiText::DeleteContact));
    assert!(intents(&mut view, &mut model).is_empty(), "asked, not done");
    let confirm = fill(
        text(UiText::ContactDeleteConfirm),
        &[("name", &isolated("Ada"))],
    );
    assert!(shows(&view, &confirm));
    settle();
    assert_eq!(view.window().get_focus_id(), "contact-delete-cancel");
    press(&view, text(UiText::Cancel));
    assert!(intents(&mut view, &mut model).is_empty());
    settle();
    assert_eq!(
        view.window().get_focus_id(),
        "contact-delete",
        "focus back on Delete"
    );

    press(&view, text(UiText::DeleteContact));
    let _ = intents(&mut view, &mut model);
    let buttons = labelled(&view, text(UiText::DeleteContact));
    assert_eq!(buttons.len(), 1, "the confirmation's own Delete");
    buttons[0].invoke_accessible_default_action();
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::DeleteContact {
            contact_id: "c1".to_owned()
        }]
    );
    assert_eq!(
        view.window().get_cards_page(),
        interweave_human_ui_slint::CardsPage::Contacts
    );
}

/// Contacts with none says how one is made; every row is a control
/// labelled by the name.
#[test]
fn contacts_with_none_say_how_one_is_made_and_each_row_opens_its_contact() {
    let mut view = view();
    let mut model = UiModel::new();
    view.render(&model);
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::ContactsTitle));
    let _ = intents(&mut view, &mut model);
    assert!(shows(&view, text(UiText::ContactsEmpty)));
    model.contacts_listed(vec![contact("c2", "Bob", Q), contact("c1", "Ada", P)]);
    view.render(&model);
    assert!(!shows(&view, text(UiText::ContactsEmpty)));
    let rows = view.window().get_contact_rows();
    let names: Vec<String> = rows.iter().map(|r| r.name.to_string()).collect();
    assert_eq!(names, vec![isolated("Ada"), isolated("Bob")], "by name");
    press(&view, &isolated("Bob"));
    let _ = intents(&mut view, &mut model);
    assert_eq!(view.window().get_contact_title(), isolated("Bob"));
}

/// Section 13 bullet 5: a contact's page shows each conversation's
/// `PeerId` and names this identity's other contacts on the same device,
/// with the note that they may be one person and are never combined; a
/// contact on another device is not named. Mutation: the same-device
/// lines left undrawn.
#[test]
fn a_contact_page_names_the_other_contacts_on_its_device_and_no_other() {
    let mut view = view();
    let mut model = UiModel::new();
    let mut work = contact("c2", "Ada at work", P);
    work.routes[0].endpoint = EndpointId::parse("work").expect("an endpoint");
    model.contacts_listed(vec![contact("c1", "Ada", P), work, contact("c3", "Bob", Q)]);
    view.render(&model);
    open_contact(&mut view, &mut model, "Ada");
    assert!(shows(&view, P), "the conversation's PeerId");
    assert!(shows(
        &view,
        &fill(
            text(UiText::ContactSharesDevice),
            &[("name", &isolated("Ada at work"))]
        )
    ));
    assert!(shows(&view, text(UiText::ContactSharesDeviceNote)));
    assert!(!shows(
        &view,
        &fill(
            text(UiText::ContactSharesDevice),
            &[("name", &isolated("Bob"))]
        )
    ));

    press(&view, text(UiText::BackToConversations));
    let _ = intents(&mut view, &mut model);
    open_contact(&mut view, &mut model, "Bob");
    assert!(
        !shows(&view, text(UiText::ContactSharesDeviceNote)),
        "alone on its device: no note"
    );
}

/// Contact-cards design 5.4.2 and 5.4.3, drawn: Decline takes the row
/// away with "Card declined" and its Undo; the Undo raises the card's
/// Undo and the proposal is drawn again, focus on its Accept.
/// Accept leaves its confirmation row, which takes focus, and its own
/// snackbar replaces the last one.
#[test]
fn a_cards_answer_shows_its_snackbar_and_undo_draws_the_proposal_again() {
    let mut view = view();
    let mut model = UiModel::new();
    model.card_received(proposal(1, P, card("Ada", false), context()));
    let _ = shown(&mut view, &mut model, P);
    let heading = fill(text(UiText::CardProposal), &[("name", &isolated("Ada"))]);
    press(&view, text(UiText::CardDecline));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::DeclineCard(row(1))]
    );
    model.card_resolved(row(1), CardOutcome::Declined);
    view.render(&model);
    assert!(!shows(&view, &heading), "the row is gone");
    assert!(shows(&view, text(UiText::ConfirmCardDeclined)));

    press(&view, text(UiText::Undo));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::UndoCard(row(1))]
    );
    model.card_restored(proposal(1, P, card("Ada", false), context()));
    view.render(&model);
    settle();
    assert!(shows(&view, &heading), "the proposal is back");
    assert_eq!(
        view.window().get_focus_target().as_str(),
        "proposal-accept:0",
        "focus on its Accept (design section 9's focus table)"
    );

    press(&view, text(UiText::CardAccept));
    let _ = intents(&mut view, &mut model);
    model.card_resolved(
        row(1),
        CardOutcome::Accepted {
            contact_id: "c1".to_owned(),
            created: true,
            photo_dropped: true,
            photo_removed: false,
        },
    );
    model.contacts_listed(vec![contact("c1", "Ada", P)]);
    view.render(&model);
    settle();
    assert!(shows(&view, text(UiText::ConfirmContactSaved)));
    assert!(shows(&view, text(UiText::CardPhotoNotUsed)));
    assert_eq!(view.window().get_focus_target().as_str(), "answer:0");
}

/// Step the view's monotonic clock and its timers by `by`.
fn step(clock: &std::rc::Rc<std::cell::Cell<std::time::Instant>>, by: std::time::Duration) {
    clock.set(clock.get() + by);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(150));
    slint::platform::update_timers_and_animations();
}

/// Contact-cards design 5.4.2 and section 8 as #416 settles them: after
/// Accept the confirmation row carries `CardUndoHint` while the snackbar
/// is open, and the snackbar's timeout waits while focus is between the
/// row and its Undo; once focus leaves that span the full time runs, and
/// two minutes after the Accept it closes whatever focus does.
/// Mutation: the single-shot timeout kept for Accept, or the hint dropped.
#[test]
fn an_accept_snackbar_waits_while_focus_is_in_its_span() {
    let clock = std::rc::Rc::new(std::cell::Cell::new(std::time::Instant::now()));
    let mut view = view();
    let c = std::rc::Rc::clone(&clock);
    view.set_monotonic(move || c.get());
    let mut model = UiModel::new();
    let accept = |view: &mut View, model: &mut UiModel, n: i64| {
        model.card_received(proposal(n, P, card("Ada", false), context()));
        let _ = shown(view, model, P);
        press(view, text(UiText::CardAccept));
        let _ = intents(view, model);
        model.card_resolved(
            row(n),
            CardOutcome::Accepted {
                contact_id: "c1".to_owned(),
                created: true,
                photo_dropped: false,
                photo_removed: false,
            },
        );
        model.contacts_listed(vec![contact("c1", "Ada", P)]);
        view.render(model);
        settle();
    };
    accept(&mut view, &mut model, 1);
    let saved = fill(text(UiText::CardAccepted), &[("name", &isolated("Ada"))]);
    let row_line = labelled(&view, &saved);
    assert_eq!(
        row_line[0].accessible_description().as_deref(),
        Some(text(UiText::CardUndoHint)),
        "the hint says where Undo is"
    );
    view.window().set_focus_id("answer:0".into());
    step(&clock, std::time::Duration::from_secs(10));
    let _ = intents(&mut view, &mut model);
    assert!(
        model.snackbar().is_some(),
        "waits while focus is on the row"
    );
    view.window().set_focus_id("composer".into());
    step(&clock, std::time::Duration::from_secs(10));
    let _ = intents(&mut view, &mut model);
    assert!(model.snackbar().is_some(), "and on the composer");

    view.window().set_focus_id("heading".into());
    step(&clock, std::time::Duration::from_millis(100));
    step(&clock, std::time::Duration::from_secs(7));
    let _ = intents(&mut view, &mut model);
    assert!(
        model.snackbar().is_none(),
        "the full time ran outside the span"
    );
    view.render(&model);
    assert!(
        !view.accept_clock_running(),
        "its poll stops with it (#414 F5)"
    );
    assert!(
        labelled(&view, &saved)[0]
            .accessible_description()
            .is_none_or(|d| d.is_empty()),
        "the hint goes with the snackbar"
    );

    accept(&mut view, &mut model, 2);
    view.window().set_focus_id("undo".into());
    step(&clock, std::time::Duration::from_secs(121));
    let _ = intents(&mut view, &mut model);
    assert!(model.snackbar().is_none(), "closed by two minutes");
}

fn keyless(problem: interweave_human_ui_model::KeyProblem) -> IdentityView {
    IdentityView {
        endpoint: human(),
        name: "Ada".to_owned(),
        given: "Ada".to_owned(),
        surname: None,
        has_photo: false,
        photo: None,
        key: interweave_human_ui_model::OwnKey::Unusable(problem),
        peer: Some(peer(Q)),
    }
}

/// Press the last control labelled `label`: a dialog's, over a page's
/// control of the same name.
fn press_last(view: &View, label: &str) {
    let found = labelled(view, label);
    found
        .last()
        .expect("a control")
        .invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
}

/// design/2026-10-24-identity-key.md sections 4.1, 5 and 8, acceptance
/// items 1, 6 and 12 (keyboard), drawn: the key block in the fingerprint's
/// place says what happened and what it stops; Make a new key opens the
/// confirmation with focus on Cancel, Cancel returns focus to Make a new
/// key, a confirmed one shows the step and then the new fingerprint with
/// where to share the card again, focus following each.
#[test]
fn a_key_that_cannot_be_used_is_said_and_a_new_key_is_confirmed_then_made() {
    use interweave_human_ui_model::{KeyProblem, OwnKey};
    let mut view = view();
    // Tall enough for the whole page: the search finds what is on screen.
    view.window()
        .window()
        .set_size(slint::PhysicalSize::new(900, 1400));
    let mut model = UiModel::new();
    model.identity_shown(keyless(KeyProblem::Damaged));
    view.render(&model);
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::OwnCardTitle));
    let _ = intents(&mut view, &mut model);
    for line in [
        UiText::KeyStateTitle,
        UiText::KeyDamaged,
        UiText::KeyCannotShare,
        UiText::KeyMakeNew,
    ] {
        assert!(shows(&view, text(line)), "{line:?}");
    }
    assert!(
        !shows(&view, text(UiText::Fingerprint)),
        "no fingerprint row"
    );
    assert!(
        !shows(&view, text(UiText::TryAgain)),
        "damage: no Try again"
    );

    press(&view, text(UiText::KeyMakeNew));
    assert_eq!(intents(&mut view, &mut model), vec![Intent::AskNewKey]);
    assert_eq!(
        model.ask_new_key(),
        Some(Intent::AskNewKey),
        "as the root does"
    );
    model.new_key_asked(true);
    view.render(&model);
    settle();
    assert!(shows(
        &view,
        &fill(text(UiText::KeyNewTitle), &[("identity", &isolated("Ada"))])
    ));
    for line in [UiText::KeyNewBody, UiText::KeyNewOld, UiText::KeyNewPending] {
        assert!(shows(&view, text(line)), "{line:?}");
    }
    assert_eq!(
        view.window().get_focus_id(),
        "new-key-cancel",
        "Cancel first"
    );

    press_last(&view, text(UiText::Cancel));
    assert_eq!(intents(&mut view, &mut model), vec![Intent::CloseKeyStep]);
    model.key_step_closed();
    view.render(&model);
    settle();
    assert!(!view.window().get_new_key_open());
    assert_eq!(view.window().get_focus_target(), "own-key-new");

    press(&view, text(UiText::KeyMakeNew));
    let _ = intents(&mut view, &mut model);
    let _ = model.ask_new_key();
    model.new_key_asked(false);
    view.render(&model);
    settle();
    assert!(
        !shows(&view, text(UiText::KeyNewPending)),
        "no card on its way"
    );
    press_last(&view, text(UiText::KeyNewConfirm));
    assert_eq!(intents(&mut view, &mut model), vec![Intent::MakeNewKey]);
    assert_eq!(model.confirm_new_key(), Some(Intent::MakeNewKey));
    view.render(&model);
    settle();
    assert!(shows(&view, text(UiText::KeyMaking)));
    assert_eq!(view.window().get_focus_target(), "own-key-making");

    model.key_made(
        Some(IdentityView {
            key: OwnKey::Ready {
                fingerprint: "fedcba9876543210".to_owned(),
            },
            ..keyless(KeyProblem::Damaged)
        }),
        "fedcba9876543210".to_owned(),
        vec![direct(P)],
    );
    view.render(&model);
    settle();
    assert!(shows(&view, "fedcba9876543210"), "the new fingerprint");
    assert!(shows(&view, text(UiText::KeyMade)));
    assert!(shows(&view, text(UiText::KeyShareAgainTitle)));
    assert_eq!(view.window().get_focus_target(), "own-key-fingerprint");
    assert!(!shows(&view, text(UiText::KeyStateTitle)), "the block went");
}

/// Sections 4.3 and 8, acceptance items 4 and 18, drawn: the share sheet
/// while the key folder is not safe says why with the folder's location,
/// that the card cannot be shared, and offers Open your card -- focused --
/// in Share's place.
#[test]
fn the_share_sheet_says_the_key_cannot_be_used_and_opens_the_card_page() {
    use interweave_human_ui_model::KeyProblem;
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::PhysicalSize::new(900, 1400));
    let mut model = UiModel::new();
    model.identity_shown(keyless(KeyProblem::DirUnsafe {
        folder: "/home/ada/.local/state/interweave/default/identity-keys".to_owned(),
    }));
    let _ = shown(&mut view, &mut model, P);
    press(&view, text(UiText::MoreOptions));
    press(&view, text(UiText::ShareCard));
    let _ = intents(&mut view, &mut model);
    settle();
    for line in [
        UiText::KeyDirUnsafe,
        UiText::KeyCannotShareHere,
        UiText::KeyOpenCard,
    ] {
        assert!(shows(&view, text(line)), "{line:?}");
    }
    assert!(
        !shows(&view, text(UiText::ShareCardAction)),
        "no Share, never shown disabled"
    );
    assert_eq!(view.window().get_focus_target(), "share-open-card");
    press(&view, text(UiText::KeyOpenCard));
    let _ = intents(&mut view, &mut model);
    assert!(shows(&view, text(UiText::KeyStateTitle)), "the card page");
    assert!(!shows(&view, text(UiText::KeyMakeNew)), "no new key there");
    assert!(shows(&view, text(UiText::TryAgain)));
}

/// The bounds of the one element labelled `label`.
fn bounds(view: &View, label: &str) -> (f32, f32, f32, f32) {
    let found = labelled(view, label);
    assert_eq!(found.len(), 1, "one {label:?}");
    let (at, size) = (found[0].absolute_position(), found[0].size());
    (at.x, at.y, at.x + size.width, at.y + size.height)
}

/// A proposal's row holds what it says: its heading and its Accept lie
/// inside the card's fill, so the card is drawn behind them and the list
/// reaches its end. Its children were all conditional once, which pass no
/// size up, and the card was drawn 0 px tall with its text spilling below
/// it; then, its size implicit, measured before its lines wrapped, and a
/// phone drew Accept under it. Mutations: the card's body layout taken
/// away; its two height bindings taken away.
#[test]
fn a_proposal_row_is_as_tall_as_what_it_says() {
    let mut view = view();
    // Phone-wide, where its lines wrap most.
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    let mut model = UiModel::new();
    model.card_received(proposal(1, P, card("Ada", false), context()));
    let _ = shown(&mut view, &mut model, P);
    let cards: Vec<ElementHandle> =
        ElementHandle::find_by_element_type_name(view.window(), "ProposalCard").collect();
    assert_eq!(cards.len(), 1, "one card");
    let (at, size) = (cards[0].absolute_position(), cards[0].size());
    let card = (at.x, at.y, at.x + size.width, at.y + size.height);
    let heading = fill(text(UiText::CardProposal), &[("name", &isolated("Ada"))]);
    for label in [heading.as_str(), text(UiText::CardAccept)] {
        let inner = bounds(&view, label);
        assert!(
            inner.1 >= card.1 && inner.3 <= card.3,
            "{label:?} at {inner:?} inside the card at {card:?}"
        );
    }
}

/// Its case D card as a phone draws it, with its longest Accept label,
/// "Accept as a new contact": the window `width` wide, text at `font`
/// px when given (Android's 200 % is 28). Every edge is checked against
/// the window and the card, and the answers against each other.
fn case_d_at(width: f32, font: Option<f32>) {
    let mut view = view();
    if let Some(px) = font {
        i_slint_core::window::WindowInner::from_pub(view.window().window())
            .context()
            .set_platform_default_font_size(Some(i_slint_core::lengths::LogicalLength::new(px)));
    }
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(width, 2400.0));
    let mut model = UiModel::new();
    model.card_received(proposal(
        1,
        P,
        card("Ada", false),
        ProposalContext {
            same_name: vec![ContactRef {
                contact_id: "c2".to_owned(),
                name: "Ada".to_owned(),
            }],
            ..context()
        },
    ));
    let _ = shown(&mut view, &mut model, P);
    assert!(shows(&view, text(UiText::AcceptAsNew)), "case D");
    let cards: Vec<ElementHandle> =
        ElementHandle::find_by_element_type_name(view.window(), "ProposalCard").collect();
    let (at, size) = (cards[0].absolute_position(), cards[0].size());
    let card = (at.x, at.y, at.x + size.width, at.y + size.height);
    assert!(
        card.0 >= 0.0 && card.2 <= width,
        "the card at {card:?}, inside {width} px"
    );
    let decline = bounds(&view, text(UiText::CardDecline));
    let accept = bounds(&view, text(UiText::AcceptAsNew));
    for (label, b) in [("Decline", decline), ("Accept", accept)] {
        assert!(
            b.0 >= card.0 && b.2 <= card.2 && b.1 >= card.1 && b.3 <= card.3,
            "{label} at {b:?} inside the card at {card:?}"
        );
    }
    assert!(decline.3 <= accept.1, "Decline on a line above Accept");
    assert!(
        (decline.0 - accept.0).abs() < 0.5 && (decline.2 - accept.2).abs() < 0.5,
        "each the column's full width: {decline:?} {accept:?}"
    );
}

/// A proposal's answers are one per line, each the full width of the
/// card's column, Decline first: a case D card fits a phone with its
/// longest Accept (ux-designer's answer of 2026-10-25 to design section
/// 8, provisional until the contact-cards revision). Copy `PeerId` sat on
/// the action line once and pushed it 23 px past its row. Mutations:
/// Copy `PeerId` back on the action line; the answers on one line.
#[test]
fn a_proposal_fits_a_phone_with_its_longest_accept() {
    case_d_at(390.0, None);
}

/// The same at 320 px with 200 % text, where "Accept as a new contact"
/// no longer fits one line and wraps inside its button. Mutation:
/// Accept's label not wrapping (`wraps` off), which sets the card wider
/// than the window.
#[test]
fn a_proposal_fits_a_narrow_window_with_large_text() {
    case_d_at(320.0, Some(28.0));
}

/// Contact-cards design section 6.3 on the card pages: Delete contact's
/// snackbar shows on the Contacts list with its Undo, which raises the
/// Undo and, once the contact is back, sends focus to its row; leaving
/// the card pages closes it. Remove photo's shows on the contact's page,
/// and its Undo sends focus back to Remove photo. Mutations: leaving not
/// closing it; the restored focus not mapped, to the row or to Remove
/// photo.
#[test]
fn a_contacts_delete_and_photo_removal_show_their_undo_on_the_card_pages() {
    use interweave_human_ui_model::ContactChange;
    let mut view = view();
    let mut model = UiModel::new();
    let ada = ContactView {
        photo: Some(photo()),
        photo_from_card: Some(true),
        photo_at: Some(NOW),
        ..contact("c1", "Ada", P)
    };
    model.contacts_listed(vec![ada.clone()]);
    view.render(&model);
    open_contact(&mut view, &mut model, "Ada");

    // As the facade answers a Delete: the list, then the change.
    model.contacts_listed(Vec::new());
    model.contact_changed("c1".to_owned(), ContactChange::Deleted, 1);
    let _ = intents(&mut view, &mut model);
    assert!(shows(&view, text(UiText::ConfirmContactDeleted)));
    press(&view, text(UiText::Undo));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::UndoContact {
            contact_id: "c1".to_owned(),
            change: ContactChange::Deleted,
            hold: 1,
        }]
    );
    model.contacts_listed(vec![ada.clone()]);
    model.contact_restored("c1", ContactChange::Deleted);
    let _ = intents(&mut view, &mut model);
    settle();
    // Where focus is, not only where it was asked for: a row taking no
    // focus request once left it on the back arrow.
    assert_eq!(view.window().get_focus_id().as_str(), "contact:0");

    // Deleted again, then the card pages left: the snackbar closes.
    model.contacts_listed(Vec::new());
    model.contact_changed("c1".to_owned(), ContactChange::Deleted, 2);
    let _ = intents(&mut view, &mut model);
    assert!(shows(&view, text(UiText::ConfirmContactDeleted)));
    press(&view, text(UiText::BackToConversations));
    let _ = intents(&mut view, &mut model);
    let _ = intents(&mut view, &mut model);
    assert!(model.snackbar().is_none(), "closed by leaving");
    assert_eq!(model.take_released().contacts, vec![2]);
    assert!(!shows(&view, text(UiText::ConfirmContactDeleted)));

    // Remove photo, on the contact's page.
    model.contacts_listed(vec![ada.clone()]);
    view.render(&model);
    open_contact(&mut view, &mut model, "Ada");
    press(&view, text(UiText::RemovePhoto));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::SetContactPhoto {
            contact_id: "c1".to_owned(),
            photo: PhotoEdit::Remove,
        }]
    );
    model.contacts_listed(vec![ContactView {
        photo: None,
        photo_from_card: Some(false),
        ..ada.clone()
    }]);
    model.contact_changed("c1".to_owned(), ContactChange::PhotoRemoved, 3);
    let _ = intents(&mut view, &mut model);
    assert!(shows(&view, text(UiText::ConfirmPhotoRemoved)));
    press(&view, text(UiText::Undo));
    let _ = intents(&mut view, &mut model);
    model.contacts_listed(vec![ada]);
    model.contact_restored("c1", ContactChange::PhotoRemoved);
    let _ = intents(&mut view, &mut model);
    settle();
    assert_eq!(
        view.window().get_focus_id().as_str(),
        "contact-remove-photo"
    );
}

/// #433 F6, contact-cards design 5.4.1 cases U and E as the row draws
/// them: the values Accept changes and the person's edits it keeps, each
/// a line read right after the context line. Mutation: the value lines
/// not drawn.
#[test]
fn a_proposal_reads_the_values_accept_changes_after_its_context_line() {
    let mut view = view();
    let mut model = UiModel::new();
    let countess = ContactRef {
        contact_id: "c1".to_owned(),
        name: "Countess".to_owned(),
    };
    model.card_received(proposal(
        1,
        P,
        card("Ada", true),
        ProposalContext {
            updates: Some(countess.clone()),
            values: interweave_human_ui_model::ValueChanges {
                new_name: Some("Ada King".to_owned()),
                photo: interweave_human_ui_model::PhotoChange::New,
                ..interweave_human_ui_model::ValueChanges::default()
            },
            ..context()
        },
    ));
    let _ = shown(&mut view, &mut model, P);
    let context_line = fill(
        text(UiText::CardUpdates),
        &[("name", &isolated("Countess"))],
    );
    let new_name = fill(
        text(UiText::CardUpdatesName),
        &[("name", &isolated("Ada King"))],
    );
    let at = reading_index(&view, &context_line);
    assert_eq!(reading_index(&view, &new_name), at + 1, "case U, the name");
    assert_eq!(
        reading_index(&view, text(UiText::CardUpdatesPhoto)),
        at + 2,
        "and the photo"
    );

    model.card_received(proposal(
        2,
        Q,
        card("Ada", true),
        ProposalContext {
            updates: Some(countess),
            values: interweave_human_ui_model::ValueChanges {
                kept_name: true,
                photo: interweave_human_ui_model::PhotoChange::Kept,
                ..interweave_human_ui_model::ValueChanges::default()
            },
            ..context()
        },
    ));
    let _ = shown(&mut view, &mut model, Q);
    let kept = fill(
        text(UiText::CardKeepsEdit),
        &[("name", &isolated("Countess"))],
    );
    let at = reading_index(&view, &context_line);
    assert_eq!(reading_index(&view, &kept), at + 1, "case E, the name");
    assert_eq!(
        reading_index(&view, text(UiText::CardKeepsPhoto)),
        at + 2,
        "and the photo"
    );

    // Case E with a card carrying no photo: the edit stays, and the
    // contact's photo, from a card, goes (#435).
    model.card_received(proposal(
        3,
        R,
        card("Ada", false),
        ProposalContext {
            updates: Some(ContactRef {
                contact_id: "c3".to_owned(),
                name: "Countess".to_owned(),
            }),
            values: interweave_human_ui_model::ValueChanges {
                kept_name: true,
                photo: interweave_human_ui_model::PhotoChange::Removed,
                ..interweave_human_ui_model::ValueChanges::default()
            },
            ..context()
        },
    ));
    let _ = shown(&mut view, &mut model, R);
    let at = reading_index(&view, &context_line);
    assert_eq!(reading_index(&view, &kept), at + 1, "case E, the name");
    assert_eq!(
        reading_index(&view, text(UiText::CardUpdatesPhotoRemoved)),
        at + 2,
        "and the photo that goes"
    );
    assert!(!shows(&view, text(UiText::CardUpdatesPhoto)));
}

/// Contact-cards design 5.4.2: an Accept whose card photo was unusable
/// says so on the confirmation row, and says the previous photo went too
/// when the Accept removed one. Mutation: the note ignoring
/// `photo_removed`.
#[test]
fn an_unusable_photo_note_says_when_the_previous_photo_went() {
    let mut view = view();
    let mut model = UiModel::new();
    for (n, from, removed) in [(1, P, false), (2, Q, true)] {
        model.card_received(proposal(n, from, card("Ada", true), context()));
        let _ = shown(&mut view, &mut model, from);
        press(&view, text(UiText::CardAccept));
        let _ = intents(&mut view, &mut model);
        model.card_resolved(
            row(n),
            CardOutcome::Accepted {
                contact_id: format!("c{n}"),
                created: false,
                photo_dropped: true,
                photo_removed: removed,
            },
        );
        model.contacts_listed(vec![contact(&format!("c{n}"), "Ada", from)]);
        view.render(&model);
        settle();
        let (said, not) = if removed {
            (UiText::CardPhotoNotUsedRemoved, UiText::CardPhotoNotUsed)
        } else {
            (UiText::CardPhotoNotUsed, UiText::CardPhotoNotUsedRemoved)
        };
        assert!(shows(&view, text(said)), "removed {removed}");
        assert!(!shows(&view, text(not)), "removed {removed}");
    }
}

/// One key pressed and released.
fn key(view: &View, key: slint::platform::Key) {
    let text: slint::SharedString = key.into();
    let window = view.window().window();
    window.dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
    window.dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
}

/// Contact-cards design 5.4.1 and section 9: a row showing `CardSameName`
/// links the contact it names, "Open your contact {name}", in focus order
/// after Copy `PeerId` and before Decline; the link opens that contact's
/// page, and Back returns to the link in its conversation. A note that is
/// no same-name warning has no link. Mutations: the link dropped from the
/// note; Close not returning to it.
#[test]
fn a_same_name_warning_links_its_contact_and_back_returns_to_the_link() {
    use slint::platform::Key;
    let mut view = view();
    let mut model = UiModel::new();
    model.contacts_listed(vec![contact("c1", "Ada", Q), contact("c2", "Bea", R)]);
    model.card_received(proposal(
        1,
        P,
        card("Ada", false),
        ProposalContext {
            same_name: vec![ContactRef {
                contact_id: "c1".to_owned(),
                name: "Ada".to_owned(),
            }],
            shares_node: vec![ContactRef {
                contact_id: "c2".to_owned(),
                name: "Bea".to_owned(),
            }],
            ..context()
        },
    ));
    let _ = shown(&mut view, &mut model, P);
    let link = fill(text(UiText::ShowContact), &[("name", &isolated("Ada"))]);
    assert_eq!(labelled(&view, &link).len(), 1, "the same-name note's");
    let other = fill(text(UiText::ShowContact), &[("name", &isolated("Bea"))]);
    assert!(!shows(&view, &other), "a shares-a-device note has none");

    let window = view.window();
    window.set_focus_target("proposal-copy:0".into());
    window.set_focus_seq(window.get_focus_seq() + 1);
    settle();
    assert_eq!(window.get_focus_id(), "proposal-copy:0");
    let order: Vec<String> = (0..3)
        .map(|_| {
            key(&view, Key::Tab);
            view.window().get_focus_id().to_string()
        })
        .collect();
    assert_eq!(
        order,
        [
            "proposal-contact:0:0",
            "proposal-decline:0",
            "proposal-accept:0"
        ],
        "Copy PeerId, the link, Decline, Accept"
    );

    key(&view, Key::Backtab);
    key(&view, Key::Backtab);
    assert_eq!(view.window().get_focus_id(), "proposal-contact:0:0");
    key(&view, Key::Return);
    let _ = intents(&mut view, &mut model);
    settle();
    assert_eq!(
        view.window().get_cards_page(),
        interweave_human_ui_slint::CardsPage::Contact
    );
    assert_eq!(view.window().get_contact_title(), isolated("Ada"));
    assert_eq!(view.window().get_focus_id(), "close-cards");

    key(&view, Key::Return);
    let _ = intents(&mut view, &mut model);
    settle();
    assert_eq!(
        view.window().get_cards_page(),
        interweave_human_ui_slint::CardsPage::None
    );
    assert!(shows(&view, &link), "the conversation, shown again");
    assert_eq!(
        view.window().get_focus_id(),
        "proposal-contact:0:0",
        "Back returns to the link"
    );
}

/// Contact-cards design section 9's focus table, "contact deleted": the
/// Contacts list's first row once the list is drawn without the deleted
/// contact, else the page's back arrow, the empty text being no focus
/// stop. Mutation: the focus left on the back arrow after the delete.
#[test]
fn a_deleted_contact_leaves_focus_on_the_lists_first_row() {
    let mut view = view();
    let mut model = UiModel::new();
    model.contacts_listed(vec![contact("c1", "Ada", P), contact("c2", "Bob", Q)]);
    view.render(&model);
    let delete = |view: &mut View, model: &mut UiModel, name: &str, id: &str| {
        open_contact(view, model, name);
        press(view, text(UiText::DeleteContact));
        let _ = intents(view, model);
        let buttons = labelled(view, text(UiText::DeleteContact));
        buttons[0].invoke_accessible_default_action();
        assert_eq!(
            intents(view, model),
            vec![Intent::DeleteContact {
                contact_id: id.to_owned()
            }]
        );
        settle();
    };

    delete(&mut view, &mut model, "Ada", "c1");
    assert_eq!(view.window().get_focus_id(), "close-cards", "until it goes");
    model.contacts_listed(vec![contact("c2", "Bob", Q)]);
    view.render(&model);
    settle();
    assert_eq!(view.window().get_focus_id(), "contact:0", "Bob's row");

    delete(&mut view, &mut model, "Bob", "c2");
    model.contacts_listed(Vec::new());
    view.render(&model);
    settle();
    assert_eq!(view.window().get_focus_id(), "close-cards", "none left");
}

/// #440 F1: a card page left some other way than its Back -- another
/// conversation chosen -- forgets the way back it was opened with, so a
/// later Back from a page the app options opened is that page's own: the
/// list shown, no old proposal link focused, no old share sheet opened.
/// Mutations: `forget_returns` not called when a card page opens from
/// the app options, or from a proposal's link; the share sheet's way
/// back set before the open, which forgets it.
#[test]
fn a_card_page_left_another_way_forgets_its_way_back() {
    use interweave_human_ui_model::KeyProblem;
    use slint::platform::Key;
    let mut view = view();
    let mut model = UiModel::new();
    model.identity_shown(keyless(KeyProblem::DirUnsafe {
        folder: "/home/ada/.local/state/interweave/default/identity-keys".to_owned(),
    }));
    model.contacts_listed(vec![contact("c1", "Ada", Q)]);
    model.card_received(proposal(
        1,
        P,
        card("Ada", false),
        ProposalContext {
            same_name: vec![ContactRef {
                contact_id: "c1".to_owned(),
                name: "Ada".to_owned(),
            }],
            ..context()
        },
    ));
    let back_from_contacts = |view: &mut View, model: &mut UiModel| {
        let _ = shown(view, model, P);
        press(view, text(UiText::AppOptions));
        press(view, text(UiText::ContactsTitle));
        let _ = intents(view, model);
        settle();
        assert_eq!(view.window().get_focus_id(), "close-cards");
        key(view, Key::Return);
        let _ = intents(view, model);
        settle();
    };

    // The proposal's link, then another conversation.
    let _ = shown(&mut view, &mut model, P);
    let link = fill(text(UiText::ShowContact), &[("name", &isolated("Ada"))]);
    press(&view, &link);
    let _ = intents(&mut view, &mut model);
    let _ = shown(&mut view, &mut model, R);
    back_from_contacts(&mut view, &mut model);
    assert!(view.window().get_list_shown(), "Back is the page's own");
    assert_ne!(view.window().get_focus_id(), "proposal-contact:0:0");

    // The share sheet's Open your card: its Back is to the sheet.
    let open_card_from_share = |view: &mut View, model: &mut UiModel| {
        let _ = shown(view, model, P);
        press(view, text(UiText::MoreOptions));
        press(view, text(UiText::ShareCard));
        let _ = intents(view, model);
        settle();
        press(view, text(UiText::KeyOpenCard));
        let _ = intents(view, model);
        settle();
    };
    open_card_from_share(&mut view, &mut model);
    assert_eq!(view.window().get_focus_id(), "close-cards");
    key(&view, Key::Return);
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_share_open(), "control: back to the sheet");
    view.window().set_share_open(false);

    // Then another conversation chosen instead.
    open_card_from_share(&mut view, &mut model);
    let _ = shown(&mut view, &mut model, R);
    back_from_contacts(&mut view, &mut model);
    assert!(!view.window().get_share_open(), "no old share sheet");

    // The sheet's way back left the same way, then a proposal's link:
    // its Back is the link's alone (round 2).
    open_card_from_share(&mut view, &mut model);
    let _ = shown(&mut view, &mut model, R);
    let _ = shown(&mut view, &mut model, P);
    press(&view, &link);
    let _ = intents(&mut view, &mut model);
    settle();
    key(&view, Key::Return);
    let _ = intents(&mut view, &mut model);
    settle();
    assert!(!view.window().get_share_open(), "no old share sheet");
    assert_eq!(view.window().get_focus_id(), "proposal-contact:0:0");
}

/// What the window's announcement slots say now.
fn announced(view: &View) -> Vec<String> {
    ["AppWindow::announce-a", "AppWindow::announce-b"]
        .iter()
        .flat_map(|id| ElementHandle::find_by_element_id(view.window(), id))
        .filter_map(|e| e.accessible_label().map(|l| l.to_string()))
        .filter(|l| !l.is_empty())
        .collect()
}

/// Contact-cards design 5.2, with the focus and speech #443 proposes:
/// Add a photo opens the platform's picker, and what it picks goes to the
/// facade side to prepare; a cancel changes nothing. While it is prepared
/// the circle reads `CardPhotoPreparing`; once ready it shows the photo
/// and reads `CardPhotoLabel`, Change photo and Remove photo are offered,
/// focus stays on the button and `PhotoAdded` is announced; an unusable
/// one shows and announces `PhotoUnusable`; closing the page lets an
/// unsaved choice go. Mutations: the picked bytes not raised; the circle's
/// label left as it was; no announcement when the photo is ready; Close
/// keeping the choice.
#[test]
fn the_editor_chooses_a_photo_through_the_platforms_picker() {
    let mut view = view();
    let asked = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = std::rc::Rc::clone(&asked);
    view.photo_picker_hook(move || counted.set(counted.get() + 1));
    let mut model = UiModel::new();
    model.identity_shown(IdentityView {
        endpoint: human(),
        name: "Ada".to_owned(),
        given: "Ada".to_owned(),
        surname: None,
        has_photo: false,
        photo: None,
        key: interweave_human_ui_model::OwnKey::Ready {
            fingerprint: "0123456789abcdef".to_owned(),
        },
        peer: Some(peer(Q)),
    });
    view.render(&model);
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::OwnCardTitle));
    let _ = intents(&mut view, &mut model);
    settle();
    assert!(
        !shows(&view, text(UiText::CardPhotoElsewhere)),
        "a picker here"
    );
    assert!(
        shows(&view, text(UiText::CardPhotoNone)),
        "the circle, no photo"
    );
    assert!(!shows(&view, text(UiText::RemovePhoto)));

    press(&view, text(UiText::AddPhoto));
    let _ = intents(&mut view, &mut model);
    assert_eq!(asked.get(), 1, "the platform's picker");
    view.photo_picked(None);
    assert!(
        intents(&mut view, &mut model).is_empty(),
        "a cancel: nothing"
    );

    view.photo_picked(Some(vec![1, 2, 3]));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::ChooseOwnPhoto(vec![1, 2, 3])]
    );
    settle();
    assert_eq!(view.window().get_focus_id(), "own-choose-photo");
    let first = model.own_photo_chosen();
    view.render(&model);
    settle();
    assert!(shows(&view, text(UiText::CardPhotoPreparing)));
    assert!(view.window().get_own_photo_preparing());
    assert!(announced(&view).is_empty(), "the start is not announced");

    model.own_photo_prepared(first, Some(photo()));
    view.render(&model);
    settle();
    assert!(shows(&view, text(UiText::CardPhotoLabel)));
    assert!(shows(&view, text(UiText::ChangePhoto)));
    assert!(shows(&view, text(UiText::RemovePhoto)));
    assert!(images(&view) >= 1, "the prepared photo, drawn");
    assert_eq!(announced(&view), vec![text(UiText::PhotoAdded).to_owned()]);
    assert_eq!(view.window().get_focus_id(), "own-choose-photo");

    press(&view, text(UiText::ChangePhoto));
    view.photo_picked(Some(vec![9]));
    let _ = intents(&mut view, &mut model);
    let second = model.own_photo_chosen();
    view.render(&model);
    model.own_photo_prepared(second, None);
    view.render(&model);
    settle();
    assert!(shows(&view, text(UiText::PhotoUnusable)));
    assert_eq!(
        announced(&view),
        vec![text(UiText::PhotoUnusable).to_owned()]
    );

    view.window().invoke_close_cards();
    let raised = intents(&mut view, &mut model);
    assert!(
        raised.contains(&Intent::ForgetOwnPhoto),
        "the unsaved choice goes: {raised:?}"
    );
}

/// Contact-cards design 5.4 and 5.4.1 as #443 left them: case S says
/// `CardSame` only when nothing is kept by a local choice, the kept line
/// standing in its place otherwise; `CardIncludesPhoto` shows only when
/// Accept would write the card's photo. Mutations: `CardSame` drawn beside
/// a kept line; the photo line drawn whenever the card carries one.
#[test]
fn a_matching_card_says_it_matches_only_when_nothing_is_kept() {
    use interweave_human_ui_model::{PhotoChange, ValueChanges};
    let mut view = view();
    let mut model = UiModel::new();
    let ada = ContactRef {
        contact_id: "c1".to_owned(),
        name: "Ada".to_owned(),
    };
    let same = fill(text(UiText::CardSame), &[("contact", &isolated("Ada"))]);
    let judged = |photo: PhotoChange, matches: bool| ProposalContext {
        updates: Some(ada.clone()),
        changes_nothing: matches,
        values: ValueChanges {
            photo,
            ..ValueChanges::default()
        },
        ..context()
    };

    model.card_received(proposal(
        1,
        P,
        card("Ada", true),
        judged(PhotoChange::KeptNone, true),
    ));
    let _ = shown(&mut view, &mut model, P);
    assert!(shows(&view, text(UiText::CardKeepsNoPhoto)));
    assert!(!shows(&view, &same), "the kept line in its place");
    assert!(
        !shows(&view, text(UiText::CardIncludesPhoto)),
        "the photo is not used"
    );

    model.card_received(proposal(
        2,
        Q,
        card("Ada", false),
        judged(PhotoChange::Unchanged, true),
    ));
    let _ = shown(&mut view, &mut model, Q);
    assert!(shows(&view, &same), "nothing kept: it matches");

    model.card_received(proposal(
        3,
        R,
        card("Ada", true),
        judged(PhotoChange::New, false),
    ));
    let _ = shown(&mut view, &mut model, R);
    assert!(
        shows(&view, text(UiText::CardIncludesPhoto)),
        "control: written"
    );
}

/// #449 F1: the editor's circle draws the card's saved photo when it
/// opens with nothing chosen, and the new one after a Save that changed
/// it, not a blank or an earlier one. Mutation: the circle's image set
/// only when the photo step changes.
#[test]
fn the_editor_draws_the_saved_photo_and_follows_a_new_one() {
    let mut view = view();
    view.photo_picker_hook(|| {});
    let mut model = UiModel::new();
    let identity = |photo: Option<Photo>| IdentityView {
        endpoint: human(),
        name: "Ada".to_owned(),
        given: "Ada".to_owned(),
        surname: None,
        has_photo: photo.is_some(),
        photo,
        key: interweave_human_ui_model::OwnKey::Ready {
            fingerprint: "0123456789abcdef".to_owned(),
        },
        peer: Some(peer(Q)),
    };
    model.identity_shown(identity(Some(photo())));
    view.render(&model);
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::OwnCardTitle));
    let _ = intents(&mut view, &mut model);
    settle();
    let drawn = |view: &View| {
        let image = view.window().get_own_edit_photo();
        let size = image.size();
        (size.width, size.height)
    };
    assert!(view.window().get_own_edit_has_photo());
    assert_eq!(drawn(&view), (4, 4), "the saved photo, drawn");

    let other = Photo {
        width: 8,
        height: 8,
        rgba: vec![90; 256],
    };
    model.identity_shown(identity(Some(other)));
    view.render(&model);
    assert_eq!(drawn(&view), (8, 8), "the card's new photo");
}

/// #449 F2, design 5.2: while a choice is prepared the buttons and circle
/// stay as they were before it -- Change photo and Remove photo over a
/// photo prepared earlier, Add a photo and no Remove photo after a
/// removal -- and an unusable choice leaves them so. Mutation: the circle
/// following the saved card while a choice is prepared.
#[test]
fn while_a_photo_is_prepared_the_editor_stays_as_it_was_before() {
    let mut view = view();
    view.photo_picker_hook(|| {});
    let mut model = UiModel::new();
    model.identity_shown(IdentityView {
        endpoint: human(),
        name: "Ada".to_owned(),
        given: "Ada".to_owned(),
        surname: None,
        has_photo: false,
        photo: None,
        key: interweave_human_ui_model::OwnKey::Ready {
            fingerprint: "0123456789abcdef".to_owned(),
        },
        peer: Some(peer(Q)),
    });
    view.render(&model);
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::OwnCardTitle));
    let _ = intents(&mut view, &mut model);
    let first = model.own_photo_chosen();
    model.own_photo_prepared(first, Some(photo()));
    view.render(&model);

    let _second = model.own_photo_chosen();
    view.render(&model);
    assert!(
        shows(&view, text(UiText::ChangePhoto)),
        "as before the choice"
    );
    assert!(shows(&view, text(UiText::RemovePhoto)));
    assert!(view.window().get_own_edit_has_photo(), "the earlier photo");
    let third = model.own_photo_chosen();
    model.own_photo_prepared(third, None);
    view.render(&model);
    assert!(shows(&view, text(UiText::PhotoUnusable)));
    assert!(shows(&view, text(UiText::ChangePhoto)), "still as before");

    model.own_photo_removed();
    let _fourth = model.own_photo_chosen();
    view.render(&model);
    assert!(
        shows(&view, text(UiText::AddPhoto)),
        "the removal stays shown"
    );
    assert!(!shows(&view, text(UiText::RemovePhoto)));
    assert!(!view.window().get_own_edit_has_photo());
}
