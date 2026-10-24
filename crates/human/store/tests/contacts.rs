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

#[test]
fn a_card_replayed_under_the_key_its_route_changed_away_from_is_a_key_change_back() {
    // TM-55: the route's key went from 1 to 9 on the person's Accept; a
    // card signed under 1 again is a key change, never an update.
    let (_dir, mut store) = store();
    let (p, human) = (peer(P), endpoint("human"));
    store
        .accept_card(&card(&p, &human, "Ada", 1), 10)
        .expect("first");
    store
        .accept_card(&card(&p, &human, "Ada", 9), 20)
        .expect("changed by the person");
    let replayed = card(&p, &human, "Ada", 1);
    assert_eq!(
        store.card_standing(&replayed).expect("standing").key,
        KeyStanding::Changed { recorded: [9; 32] }
    );
    assert_eq!(
        store
            .card_standing(&card(&p, &human, "Ada", 9))
            .expect("standing")
            .key,
        KeyStanding::Same,
        "the control: the recorded key is the same"
    );
}

#[test]
fn a_deleted_contact_takes_its_routes_and_keys_and_a_later_card_starts_afresh() {
    let (_dir, mut store) = store();
    let (p, human) = (peer(P), endpoint("human"));
    let first = store
        .accept_card(&card(&p, &human, "Ada", 1), 10)
        .expect("accepted");
    let other = store
        .accept_card(&card(&peer(Q), &human, "Grace", 2), 11)
        .expect("accepted");
    assert!(store.delete_contact(&first.contact_id).expect("deleted"));
    assert!(store.contact(&first.contact_id).expect("read").is_none());
    assert!(
        store.contact_for_route(&p, &human).expect("read").is_none(),
        "its route went with it"
    );
    assert!(
        store.contact(&other.contact_id).expect("read").is_some(),
        "and nothing else"
    );
    assert!(
        !store.delete_contact(&first.contact_id).expect("asked"),
        "a second delete finds nothing"
    );
    let again = store
        .accept_card(&card(&p, &human, "Ada", 3), 20)
        .expect("accepted again");
    assert!(again.created, "a new contact");
    assert_eq!(
        again.key,
        KeyStanding::Unrecorded,
        "no key survives the delete"
    );
}

#[test]
fn a_deleted_contact_leaves_both_files_before_the_call_returns() {
    const GONE: &str = "Zebulon-deleted-marker";
    const STAYS: &str = "Quentin-kept-marker";
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let holds = |p: &std::path::Path| {
        std::fs::read(p).is_ok_and(|bytes| bytes.windows(GONE.len()).any(|w| w == GONE.as_bytes()))
    };
    let wal = path.with_extension("sqlite3-wal");
    let mut store = HumanStore::open(&path, StoreOptions::default()).expect("opens");
    let (p, human) = (peer(P), endpoint("human"));
    let gone = store
        .accept_card(&card(&p, &human, GONE, 1), 10)
        .expect("accepted");
    store
        .accept_card(&card(&peer(Q), &human, STAYS, 2), 11)
        .expect("accepted");
    drop(store);
    assert!(
        holds(&path),
        "the setup: the name reached the database file"
    );
    let mut store = HumanStore::open(&path, StoreOptions::default()).expect("reopens");
    assert!(store.delete_contact(&gone.contact_id).expect("deleted"));
    // Still open: the bound is the delete itself.
    assert!(
        !holds(&path) && !holds(&wal),
        "the deleted name left both files"
    );
    let control = [&path, &wal].iter().any(|f| {
        std::fs::read(f)
            .is_ok_and(|bytes| bytes.windows(STAYS.len()).any(|w| w == STAYS.as_bytes()))
    });
    assert!(control, "the control: the search sees a kept contact");
    drop(store);
}

/// When the photo came, as the contact page says it beside where from:
/// the card's Accept, then the person's removal, which a later card's
/// photo leaves alone.
#[test]
fn a_photo_records_when_it_was_set_and_a_later_card_keeps_the_persons_time() {
    let (_dir, mut store) = store();
    let (p, human) = (peer(P), endpoint("human"));
    let first = store
        .accept_card(
            &AcceptedCard {
                photo: Some(b"jpeg"),
                ..card(&p, &human, "Ada", 1)
            },
            10,
        )
        .expect("accepted");
    let read = |store: &HumanStore| {
        store
            .contact(&first.contact_id)
            .expect("read")
            .expect("there")
    };
    let contact = read(&store);
    assert_eq!(contact.photo_source, Some(ValueSource::Card));
    assert_eq!(contact.photo_at, Some(10));

    assert!(
        store
            .set_contact_photo(&first.contact_id, None, 30)
            .expect("removed")
    );
    store
        .accept_card(
            &AcceptedCard {
                photo: Some(b"jpeg2"),
                ..card(&p, &human, "Ada", 1)
            },
            40,
        )
        .expect("a later card");
    let contact = read(&store);
    assert_eq!(contact.photo_source, Some(ValueSource::Person));
    assert_eq!(contact.photo_at, Some(30), "the person's removal, and when");
    assert_eq!(contact.photo, None);
}

/// Undo after an Accept that created the contact (contact-cards design
/// section 5.4.2): the contact and its route are gone, so the same card
/// is a first card again, and no other contact is touched.
#[test]
fn undo_of_an_accept_that_created_deletes_the_contact_and_its_route() {
    let (_dir, mut store) = store();
    let (p, human) = (peer(P), endpoint("human"));
    let other = store
        .accept_card(&card(&peer(Q), &human, "Grace", 2), 5)
        .expect("accepted");
    let accepted = store
        .accept_card(&card(&p, &human, "Ada", 1), 10)
        .expect("accepted");
    assert_eq!(accepted.undo.contact_id(), accepted.contact_id);
    assert!(store.undo_accept(&accepted.undo).expect("undone"));
    assert!(store.contact(&accepted.contact_id).expect("read").is_none());
    assert!(store.contact_for_route(&p, &human).expect("read").is_none());
    assert_eq!(
        store
            .card_standing(&card(&p, &human, "Ada", 1))
            .expect("standing")
            .key,
        KeyStanding::Unrecorded,
        "no key stays behind"
    );
    assert!(
        store.contact(&other.contact_id).expect("read").is_some(),
        "the control: another contact stays"
    );
}

/// Undo after an Accept that updated a contact: the name, the photo, the
/// times and the route's key are what they were before the Accept, a key
/// change included; a contact deleted since is not made again.
#[test]
fn undo_of_an_accept_that_updated_writes_back_the_values_and_the_key_it_had() {
    let (_dir, mut store) = store();
    let (p, human) = (peer(P), endpoint("human"));
    let photo: &[u8] = b"photo one";
    let first = store
        .accept_card(
            &AcceptedCard {
                photo: Some(photo),
                ..card(&p, &human, "Ada", 1)
            },
            10,
        )
        .expect("first");
    let before = store
        .contact(&first.contact_id)
        .expect("read")
        .expect("there");
    let changed = store
        .accept_card(
            &AcceptedCard {
                photo: Some(b"photo two"),
                ..card(&p, &human, "Ada L.", 9)
            },
            20,
        )
        .expect("a key change, accepted");
    assert_eq!(changed.key, KeyStanding::Changed { recorded: [1; 32] });
    assert!(store.undo_accept(&changed.undo).expect("undone"));
    let after = store
        .contact(&first.contact_id)
        .expect("read")
        .expect("there");
    assert_eq!(after, before, "every value as it was, the key included");
    assert_eq!(after.routes[0].identity_key, Some([1; 32]));

    assert!(store.delete_contact(&first.contact_id).expect("deleted"));
    assert!(
        !store.undo_accept(&changed.undo).expect("asked"),
        "nothing to undo on"
    );
    assert!(
        store.contacts().expect("read").is_empty(),
        "nothing made again"
    );
}
