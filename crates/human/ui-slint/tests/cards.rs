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
            text(UiText::CardWillUpdate),
            &[("contact", &isolated("Countess"))]
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
        fingerprint: fingerprint.clone(),
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
        fingerprint: "0123456789abcdef".to_owned(),
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
        fingerprint: "0123456789abcdef".to_owned(),
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

    press(&view, text(UiText::RemovePhoto));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::SetOwnCard {
            name: "Ada".to_owned(),
            surname: None,
            photo: PhotoEdit::Remove,
        }],
        "the photo removed, the names as stored"
    );

    view.window().set_own_given("Ada".into());
    view.window().set_own_surname("Lovelace".into());
    press(&view, text(UiText::Save));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::SetOwnCard {
            name: "Ada".to_owned(),
            surname: Some("Lovelace".to_owned()),
            photo: PhotoEdit::Keep,
        }]
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
/// Undo and the proposal is drawn again, focus on its first control.
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
