// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The contacts without their photos, the photos on demand, and the
//! contacts unread (design/2026-10-27-start-window.md section 11).

use interweave_human_chat_protocol::Card;
use interweave_human_core::RowId;
use interweave_transport_api::{EndpointId, TransportIdentity};

use super::{DirectoryContact, PHOTO_CAP, PhotoAsk, PhotoState};
use crate::model::cards::{ContactView, KeyView, Photo, Proposal, ProposalContext, ValueChanges};
use crate::model::{ConversationKey, Intent, UiModel};

const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

fn contact(id: &str, photo_at: Option<u64>) -> DirectoryContact {
    DirectoryContact {
        view: ContactView {
            contact_id: id.to_owned(),
            name: format!("Contact {id}"),
            name_from_card: true,
            name_at: None,
            photo_from_card: photo_at.map(|_| true),
            photo_at,
            photo: None,
            routes: Vec::new(),
        },
        has_photo: photo_at.is_some(),
    }
}

fn photo() -> Photo {
    Photo {
        width: 1,
        height: 1,
        rgba: vec![0; 4],
    }
}

fn proposal(row: i64) -> Proposal {
    Proposal {
        row: RowId::from_stored(row),
        peer: TransportIdentity::parse(P).expect("a PeerId"),
        endpoint: EndpointId::parse("human").expect("an endpoint"),
        card: Card {
            app_message_id: "0".repeat(32),
            name: "Ada".to_owned(),
            surname: None,
            photo: None,
            identity_key: [1; 32],
            sent_at_ms: None,
            from_endpoint: None,
        },
        received_at: 1,
        context: ProposalContext {
            updates: None,
            shares_node: Vec::new(),
            same_name: Vec::new(),
            key_matches: Vec::new(),
            key: KeyView::New,
            fingerprint: "0123456789abcdef".to_owned(),
            changes_nothing: false,
            values: ValueChanges::default(),
        },
    }
}

fn conversation() -> ConversationKey {
    ConversationKey::Direct {
        peer: TransportIdentity::parse(P).expect("a PeerId"),
        endpoint: Some(EndpointId::parse("human").expect("an endpoint")),
    }
}

/// While the contacts are unread no proposal offers Accept -- not by its
/// button, its menu, its swipe nor `offers` -- it shows no note of a
/// comparison never made, and naming by hand is withheld; Decline stays.
/// Read again, Accept comes back. Mutation: the actions not judged by the
/// contacts' state.
#[test]
fn while_the_contacts_are_unread_no_proposal_offers_accept() {
    let mut model = UiModel::new();
    model.contacts_directory(Err(()));
    model.card_received(proposal(3));
    let row = RowId::from_stored(3);
    assert_eq!(model.card_actions(row), [Intent::DeclineCard(row)]);
    assert!(!model.offers(&Intent::AcceptCard(row)));
    let item = &model.proposals(&conversation())[0];
    assert!(item.contacts_unread);
    assert!(!item.swipe().accept);
    assert!(item.swipe().decline);
    assert!(!model.naming_offered());

    model.contacts_directory(Ok(Vec::new()));
    assert!(model.offers(&Intent::AcceptCard(row)));
    assert!(!model.proposals(&conversation())[0].contacts_unread);
}

/// Try again reads every contact once: a second press while the read is
/// on its way asks nothing. Mutation: a read asked whatever the state.
#[test]
fn try_again_reads_once_while_one_is_on_its_way() {
    let mut model = UiModel::new();
    model.contacts_directory(Err(()));
    assert!(model.contacts_try_again());
    assert!(model.take_contacts_ask());
    assert!(!model.contacts_try_again(), "already reading");
    assert!(!model.take_contacts_ask());
}

/// A contact's photo is asked when its avatar shows, once, and its
/// initials stand until the photo arrives; one that cannot be read keeps
/// the initials. Mutation: a photo asked each time it shows.
#[test]
fn a_photo_is_asked_when_shown_and_the_initials_stand_until_it_arrives() {
    let mut model = UiModel::new();
    model.contacts_directory(Ok(vec![contact("a", Some(5)), contact("b", None)]));
    assert_eq!(model.contact_photo("a"), PhotoState::Initials);
    assert_eq!(
        model.contact_photo("b"),
        PhotoState::None,
        "no photo at all"
    );
    model.photo_shown("a");
    model.photo_shown("a");
    assert_eq!(
        model.take_photo_asks(),
        [PhotoAsk {
            contact_id: "a".to_owned(),
            photo_at: Some(5)
        }]
    );
    model.photo_arrived("a", Some(5), Some(photo()));
    assert!(matches!(model.contact_photo("a"), PhotoState::Shown(_)));
    model.photo_shown("b");
    assert!(model.take_photo_asks().is_empty(), "nothing to read for b");
}

/// A photo the contact no longer has -- removed, or replaced, so its time
/// changed -- goes at once. Mutation: the cache kept by contact alone.
#[test]
fn a_photo_the_contact_no_longer_has_goes_at_once() {
    let mut model = UiModel::new();
    model.contacts_directory(Ok(vec![contact("a", Some(5))]));
    model.photo_shown("a");
    let _ = model.take_photo_asks();
    model.photo_arrived("a", Some(5), Some(photo()));
    model.contacts_directory(Ok(vec![contact("a", Some(6))]));
    assert_eq!(
        model.contact_photo("a"),
        PhotoState::Initials,
        "the new one not read yet"
    );
    model.contacts_directory(Ok(vec![contact("a", None)]));
    assert_eq!(model.contact_photo("a"), PhotoState::None);
}

/// Past the photo cap the photo evicted is never one in view. Mutation:
/// the oldest evicted whether in view or not.
#[test]
fn a_photo_in_view_is_never_evicted_by_the_cap() {
    let mut model = UiModel::new();
    let ids: Vec<String> = (0..=PHOTO_CAP).map(|n| format!("c{n}")).collect();
    model.contacts_directory(Ok(ids.iter().map(|id| contact(id, Some(1))).collect()));
    for id in &ids {
        model.photo_shown(id);
        model.photo_arrived(id, Some(1), Some(photo()));
    }
    model.photo_hidden(&ids[1]);
    model.photo_arrived(&ids[1], Some(1), Some(photo()));
    assert!(
        ids.iter()
            .filter(|id| *id != &ids[1])
            .all(|id| matches!(model.contact_photo(id), PhotoState::Shown(_))),
        "every photo in view stays"
    );
}

/// While the contacts are unread no contact is known: a failed read
/// leaves none from before, so no title uses a contact's name. Mutation:
/// the contacts kept on a failed read.
#[test]
fn a_failed_read_leaves_no_contact_known() {
    let mut model = UiModel::new();
    model.contacts_directory(Ok(vec![contact("a", None)]));
    assert!(model.contact("a").is_some());
    model.contacts_directory(Err(()));
    assert!(model.contact("a").is_none());
}
