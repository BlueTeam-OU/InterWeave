// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Contacts built from accepted cards (plan §29 step 2 and exit gate
//! (b); ADR-0059 decisions 3-4, ADR-0061 decision 4, ADR-0063 decision
//! 4): the store's writers keep the rules no constraint can.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use interweave_human_store::{
    AcceptedCard, HumanStore, KeyStanding, OwnCard, StoreOptions, ValueSource,
};
use interweave_transport_api::{EndpointId, TransportIdentity};

const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
const Q: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";

fn peer(s: &str) -> TransportIdentity {
    TransportIdentity::parse(s).expect("a PeerId")
}

fn endpoint(s: &str) -> EndpointId {
    EndpointId::parse(s).expect("an endpoint")
}

fn store() -> (tempfile::TempDir, HumanStore) {
    let dir = common::private_tempdir();
    let store = HumanStore::open(
        &dir.path().join("state").join("human.sqlite3"),
        StoreOptions::default(),
    )
    .expect("opens");
    (dir, store)
}

fn card<'a>(
    peer: &'a TransportIdentity,
    endpoint: &'a EndpointId,
    name: &'a str,
    key: u8,
) -> AcceptedCard<'a> {
    AcceptedCard {
        peer,
        endpoint,
        name,
        surname: None,
        identity_key: [key; 32],
        photo: None,
    }
}

#[test]
fn a_valid_card_accepted_creates_the_contact_with_its_route_and_key() {
    let (_dir, mut store) = store();
    let (p, human) = (peer(P), endpoint("human"));
    let accepted = store
        .accept_card(
            &AcceptedCard {
                surname: Some("Lovelace"),
                ..card(&p, &human, "Ada", 1)
            },
            10,
        )
        .expect("accepted");
    assert!(accepted.created);
    let contact = store
        .contact(&accepted.contact_id)
        .expect("read")
        .expect("there");
    assert_eq!(
        contact.display_name, "Ada Lovelace",
        "the name, a space, the surname"
    );
    assert_eq!(contact.name_source, ValueSource::Card);
    assert_eq!(contact.name_at, Some(10));
    assert_eq!(contact.routes.len(), 1);
    assert_eq!(
        contact.routes[0].identity_key,
        Some([1; 32]),
        "the first card records the key"
    );
    assert_eq!(
        store
            .contact_for_route(&p, &human)
            .expect("read")
            .map(|c| c.contact_id),
        Some(accepted.contact_id)
    );
}

#[test]
fn a_second_card_from_the_same_route_updates_it_and_overwrites_no_local_edit() {
    let (_dir, mut store) = store();
    let (p, human) = (peer(P), endpoint("human"));
    let first = store
        .accept_card(&card(&p, &human, "Ada", 1), 10)
        .expect("first");
    let second = store
        .accept_card(&card(&p, &human, "Ada L.", 1), 20)
        .expect("second");
    assert!(!second.created);
    assert_eq!(second.contact_id, first.contact_id);
    assert_eq!(second.key, KeyStanding::Same);
    assert_eq!(
        store
            .contact(&first.contact_id)
            .expect("read")
            .expect("there")
            .display_name,
        "Ada L."
    );

    // The person edits the name; a later card leaves it.
    assert!(
        store
            .set_contact_name(&first.contact_id, "Countess", 30)
            .expect("edited")
    );
    let third = store
        .accept_card(&card(&p, &human, "Ada again", 1), 40)
        .expect("third");
    assert!(third.name_kept);
    let contact = store
        .contact(&first.contact_id)
        .expect("read")
        .expect("there");
    assert_eq!(
        contact.display_name, "Countess",
        "a local edit is never overwritten by a card"
    );
    assert_eq!(contact.name_source, ValueSource::Person);
    assert_eq!(
        contact.name_at,
        Some(30),
        "and the contact page can say when"
    );
    assert_eq!(
        store.contacts().expect("read").len(),
        1,
        "one contact still"
    );
}

#[test]
fn two_same_name_cards_from_two_peer_ids_are_two_contacts_and_each_names_the_other() {
    let (_dir, mut store) = store();
    let (p, q, human) = (peer(P), peer(Q), endpoint("human"));
    store
        .accept_card(&card(&p, &human, "Ada", 1), 10)
        .expect("from P");
    let from_q = card(&q, &human, "Ada", 2);
    let standing = store.card_standing(&from_q).expect("standing");
    assert!(
        standing.updates.is_none(),
        "Q's route is held by no contact"
    );
    assert_eq!(
        standing.same_name.len(),
        1,
        "the same-name note names P's contact"
    );
    assert_eq!(standing.same_name[0].routes[0].peer, p);
    let second = store.accept_card(&from_q, 20).expect("from Q");
    assert!(second.created, "a second contact, never a merge");
    assert_eq!(store.contacts().expect("read").len(), 2);
}

#[test]
fn a_card_on_another_route_of_a_known_peer_id_makes_its_own_contact_and_names_the_one_sharing_its_node()
 {
    let (_dir, mut store) = store();
    let (p, human, work) = (peer(P), endpoint("human"), endpoint("work"));
    let first = store
        .accept_card(&card(&p, &human, "Ada", 1), 10)
        .expect("human");
    let at_work = card(&p, &work, "Ada at work", 3);
    let standing = store.card_standing(&at_work).expect("standing");
    assert!(standing.updates.is_none());
    assert_eq!(standing.shares_node.len(), 1);
    assert_eq!(standing.shares_node[0].contact_id, first.contact_id);
    assert!(
        standing.same_name.is_empty(),
        "a node-sharing contact is not a same-name one"
    );
    let second = store.accept_card(&at_work, 20).expect("work");
    assert!(second.created);
    assert_ne!(second.contact_id, first.contact_id);
    for contact in store.contacts().expect("read") {
        assert_eq!(
            contact.routes.len(),
            1,
            "a contact never holds two routes of one PeerId"
        );
    }
}

#[test]
fn a_changed_key_is_a_key_change_and_accepting_it_replaces_the_key_on_that_route_alone() {
    let (dir, mut store) = store();
    let (p, q, human) = (peer(P), peer(Q), endpoint("human"));
    let first = store
        .accept_card(&card(&p, &human, "Ada", 1), 10)
        .expect("first");
    // The contact's second route, grouped by hand (ADR-0043), with its own key.
    let second = store
        .accept_card(&card(&q, &human, "Ada phone", 2), 10)
        .expect("Q");
    {
        let conn = rusqlite::Connection::open(dir.path().join("state").join("human.sqlite3"))
            .expect("raw");
        conn.execute(
            "UPDATE contact_routes SET contact_id = ?1 WHERE contact_id = ?2",
            [&first.contact_id, &second.contact_id],
        )
        .expect("moved");
        conn.execute(
            "DELETE FROM contacts WHERE contact_id = ?1",
            [&second.contact_id],
        )
        .expect("emptied");
    }
    let changed = card(&p, &human, "Ada", 9);
    let standing = store.card_standing(&changed).expect("standing");
    assert_eq!(standing.key, KeyStanding::Changed { recorded: [1; 32] });
    let accepted = store
        .accept_card(&changed, 20)
        .expect("accepted by the person");
    assert_eq!(accepted.key, KeyStanding::Changed { recorded: [1; 32] });
    let contact = store
        .contact(&first.contact_id)
        .expect("read")
        .expect("there");
    let key_of = |pid: &TransportIdentity| {
        contact
            .routes
            .iter()
            .find(|r| &r.peer == pid)
            .and_then(|r| r.identity_key)
    };
    assert_eq!(
        key_of(&p),
        Some([9; 32]),
        "the route's key is the accepted one"
    );
    assert_eq!(
        key_of(&q),
        Some([2; 32]),
        "the contact's other route keeps its own"
    );
}

#[test]
fn a_card_whose_key_a_contact_holds_on_another_peer_id_says_so_and_groups_nothing() {
    let (_dir, mut store) = store();
    let (p, q, human) = (peer(P), peer(Q), endpoint("human"));
    let first = store
        .accept_card(&card(&p, &human, "Ada", 5), 10)
        .expect("P");
    let standing = store
        .card_standing(&card(&q, &human, "Ada", 5))
        .expect("standing");
    assert_eq!(standing.key_matches.len(), 1);
    assert_eq!(standing.key_matches[0].contact_id, first.contact_id);
    assert_eq!(
        standing.key,
        KeyStanding::Unrecorded,
        "a new route, whose key nothing records"
    );
    let second = store
        .accept_card(&card(&q, &human, "Ada", 5), 20)
        .expect("Q");
    assert!(second.created, "the matching key groups nothing on its own");
}

#[test]
fn an_identitys_own_card_and_when_a_conversation_shared_it() {
    let (_dir, mut store) = store();
    let human = endpoint("human");
    let made = store.ensure_own_card(&human, 1).expect("made");
    assert_eq!(
        made.name, "human",
        "the route label until the person writes a name"
    );
    let edited = OwnCard {
        name: "Grace".to_owned(),
        surname: Some("Hopper".to_owned()),
        photo: Some(vec![0xFF, 0xD8]),
        ..made
    };
    store.set_own_card(&edited, 2).expect("edited");
    assert_eq!(store.own_card(&human).expect("read"), Some(edited.clone()));
    assert_eq!(
        store.ensure_own_card(&human, 3).expect("kept"),
        edited,
        "never replaced once made"
    );

    let p = peer(P);
    assert_eq!(store.card_shared_at(&p, &human).expect("read"), None);
    store.mark_card_shared(&p, &human, 50).expect("shared");
    store
        .mark_card_shared(&p, &human, 60)
        .expect("shared again");
    assert_eq!(store.card_shared_at(&p, &human).expect("read"), Some(60));
}

#[test]
fn an_accepted_photo_is_kept_as_bytes_and_a_photo_the_person_set_is_never_replaced() {
    let (_dir, mut store) = store();
    let (p, human) = (peer(P), endpoint("human"));
    let with = |photo: &'static [u8]| AcceptedCard {
        photo: Some(photo),
        ..card(&p, &human, "Ada", 1)
    };
    let first = store.accept_card(&with(b"jpeg one"), 10).expect("first");
    let contact = store
        .contact(&first.contact_id)
        .expect("read")
        .expect("there");
    assert_eq!(contact.photo.as_deref(), Some(b"jpeg one".as_slice()));
    assert_eq!(contact.photo_source, Some(ValueSource::Card));
    store.accept_card(&with(b"jpeg two"), 20).expect("second");
    let contact = store
        .contact(&first.contact_id)
        .expect("read")
        .expect("there");
    assert_eq!(
        contact.photo.as_deref(),
        Some(b"jpeg two".as_slice()),
        "a card's photo follows the card"
    );
    assert!(
        store
            .set_contact_photo(&first.contact_id, Some(b"mine"), 30)
            .expect("set")
    );
    let third = store.accept_card(&with(b"jpeg three"), 40).expect("third");
    assert!(third.photo_kept);
    let contact = store
        .contact(&first.contact_id)
        .expect("read")
        .expect("there");
    assert_eq!(
        contact.photo.as_deref(),
        Some(b"mine".as_slice()),
        "the person's photo stands"
    );
    assert_eq!(contact.photo_source, Some(ValueSource::Person));
}
