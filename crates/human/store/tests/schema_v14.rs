// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Schema v14 (STATE.md's v14 paragraph and §Migrations, A 2026-11-19):
//! ADR-0067's clean break reaches the store. Every row takes the first
//! identity's derived id; a keyed route is rebound to the id its key
//! derives and its conversation follows; a pair no keyed route names is
//! discarded; two routes deriving one id are merged; every pending row and
//! every unread card goes; what was discarded is counted in eight counts,
//! the row written only when one is not zero and kept until the person's
//! acknowledgement; one pair under two contacts with two keys fails the
//! migration. And from v14
//! every read, bound and key is one identity's.
//!
//! Every key here is TEST-ONLY (`interweave_test_support::endpoint_keys`).

#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

mod common;

use interweave_human_store::{
    AppMessageId, DiscardedAtV14, HumanStore, InboundKind, InboundOrigin, NewInbound, NewOutbound,
    OutboundDestination, READ_PAIR_CAP, SCHEMA_VERSION, StoreError, StoreOptions,
};
use interweave_test_support::endpoint_keys::{endpoint_id, signing_key};
use interweave_transport_api::{DirectDestination, EndpointId, MessageId, TransportIdentity};

/// The first identity: the id the client hands to the open.
fn me() -> EndpointId {
    endpoint_id("me")
}

/// Another identity of the same node.
fn other() -> EndpointId {
    endpoint_id("other")
}

/// A remote node's `PeerId`: the derived text of a test-only key is a
/// well-formed Ed25519 `PeerId`.
fn node(name: &str) -> String {
    endpoint_id(&format!("node-{name}")).as_str().to_owned()
}

/// The public key `name`'s card would carry.
fn key(name: &str) -> Vec<u8> {
    signing_key(name).verifying_key().to_bytes().to_vec()
}

fn id(n: u32) -> String {
    format!("{n:032x}")
}

/// A file this build made, faked back to v13 with no rows.
fn v13_store() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    drop(HumanStore::open(&path, &me(), StoreOptions::default()).expect("a store"));
    let conn = rusqlite::Connection::open(&path).expect("a second connection");
    conn.execute_batch(common::UNDO_V14).expect("back to v13");
    (dir, path)
}

fn version(path: &std::path::Path) -> i64 {
    rusqlite::Connection::open(path)
        .expect("a connection")
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .expect("the version")
}

struct V13<'a>(&'a rusqlite::Connection);

impl V13<'_> {
    fn contact(&self, contact_id: &str, name: &str, created_at: i64) {
        self.0
            .execute(
                "INSERT INTO contacts (contact_id, display_name, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?3)",
                rusqlite::params![contact_id, name, created_at],
            )
            .expect("a contact");
    }

    fn route(&self, contact_id: &str, peer: &str, endpoint: &str, key: Option<&[u8]>) {
        self.0
            .execute(
                "INSERT INTO contact_routes (contact_id, peer_id, endpoint_id, identity_key)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![contact_id, peer, endpoint, key],
            )
            .expect("a route");
    }

    fn indexed(&self, peer: &str, endpoint: &str, card_shared_at: Option<i64>) {
        self.0
            .execute(
                "INSERT INTO conversation_index
                     (conversation_id, peer_id, endpoint_id, card_shared_at)
                 VALUES ('direct/' || ?1 || '/' || ?2, ?1, ?2, ?3)",
                rusqlite::params![peer, endpoint, card_shared_at],
            )
            .expect("an index row");
    }

    fn unread(&self, n: u32, peer: &str, endpoint: Option<&str>, channel: Option<&str>) {
        self.unread_at(n, peer, endpoint, channel, i64::from(n), "text");
    }

    fn unread_at(
        &self,
        n: u32,
        peer: &str,
        endpoint: Option<&str>,
        channel: Option<&str>,
        received_at: i64,
        kind: &str,
    ) {
        self.0
            .execute(
                "INSERT INTO unread_inbound
                     (app_message_id, source_peer, source_endpoint, channel_id, payload,
                      received_at, kind)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    id(n),
                    peer,
                    endpoint,
                    channel,
                    payload(n, endpoint),
                    received_at,
                    kind
                ],
            )
            .expect("an unread row");
    }

    fn kept(&self, n: u32, peer: &str, endpoint: &str, received_at: i64) {
        self.kept_in(n, peer, Some(endpoint), None, received_at);
    }

    fn kept_in(
        &self,
        n: u32,
        peer: &str,
        endpoint: Option<&str>,
        channel: Option<&str>,
        received_at: i64,
    ) {
        self.0
            .execute(
                "INSERT INTO kept_inbound
                     (app_message_id, source_peer, source_endpoint, channel_id, payload,
                      received_at, read_at, kept_at, kind)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?6, 'text')",
                rusqlite::params![
                    id(n),
                    peer,
                    endpoint,
                    channel,
                    payload(n, endpoint),
                    received_at
                ],
            )
            .expect("a kept row");
    }

    fn pair(&self, n: u32, peer: &str, endpoint: Option<&str>, channel: Option<&str>, at: i64) {
        self.0
            .execute(
                "INSERT INTO read_pairs (app_message_id, source_peer, source_endpoint, channel_id, at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id(n), peer, endpoint, channel, at],
            )
            .expect("a read pair");
    }

    fn pending(
        &self,
        n: u32,
        peer: &str,
        endpoint: Option<&str>,
        channel: Option<&str>,
        tries: i64,
    ) {
        self.0
            .execute(
                "INSERT INTO pending_outbound
                     (app_message_id, transport_message_id, destination_peer,
                      destination_endpoint, channel_id, payload, created_at, attempts)
                 VALUES (?1, randomblob(16), ?2, ?3, ?4, x'01', 1, ?5)",
                rusqlite::params![id(n), peer, endpoint, channel, tries],
            )
            .expect("a pending row");
    }
}

/// A text the previous release encoded: its sender's label in
/// `from_endpoint`, as a stored row from before v14 carries it.
fn payload(n: u32, endpoint: Option<&str>) -> Vec<u8> {
    format!(
        r#"{{"v":2,"kind":"text","app_message_id":"{}","text":"m{n}","from_endpoint":"{}"}}"#,
        id(n),
        endpoint.unwrap_or("human")
    )
    .into_bytes()
}

/// `(app_message_id, source_peer, source_endpoint, channel_id)` of every
/// row of `table` under `local`, ordered.
fn rows(
    conn: &rusqlite::Connection,
    table: &str,
    local: &str,
) -> Vec<(String, String, Option<String>, Option<String>)> {
    conn.prepare(&format!(
        "SELECT app_message_id, source_peer, source_endpoint, channel_id FROM {table}
          WHERE local_endpoint_id = ?1 ORDER BY app_message_id, source_endpoint"
    ))
    .expect("prepare")
    .query_map([local], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    })
    .expect("rows")
    .collect::<Result<_, _>>()
    .expect("read")
}

fn total(conn: &rusqlite::Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .expect("a count")
}

/// Every case of §Migrations' (a)-(e) on one store, and the counts row
/// exactly. Mutations, each failing here: (b)'s rebinding left out of the
/// unread copy; (c) not deleting a keyless pair's kept rows; (d)'s route
/// merge keeping the later contact; (d)'s unread-meets-kept rule dropped;
/// (d)'s read-pair merge keeping the later; (e)'s card discard dropped; the
/// own card left under its label; the counts row's `kept` taken after the
/// merge (so counting (d)'s second copy); the card proposals deleted after
/// (c), so that (c)'s card is counted as an unread message; (d)'s combined
/// contact counted among (c)'s contacts.
#[test]
fn a_v13_store_takes_its_first_identity_and_keeps_only_what_a_key_names() {
    let (_dir, path) = v13_store();
    let (p, q, r) = (node("p"), node("q"), node("r"));
    let (alice, carol, bob) = (
        endpoint_id("alice"),
        endpoint_id("carol"),
        endpoint_id("bob"),
    );
    {
        let conn = rusqlite::Connection::open(&path).expect("v13");
        let v13 = V13(&conn);
        // (b) a keyed route, (c) a keyless one, (d) two routes of Q whose
        // keys are carol's under two contacts, and a contact holding a
        // keyed route and a keyless one.
        v13.contact("c1", "Alice", 10);
        v13.route("c1", &p, "alice-label", Some(key("alice").as_slice()));
        v13.contact("c2", "Typed", 20);
        v13.route("c2", &p, "typed-label", None);
        v13.contact("c3", "Carol A", 30);
        v13.route("c3", &q, "carol-a", Some(key("carol").as_slice()));
        v13.contact("c4", "Carol B", 40);
        v13.route("c4", &q, "carol-b", Some(key("carol").as_slice()));
        v13.contact("c5", "Mixed", 50);
        v13.route("c5", &r, "bob-label", Some(key("bob").as_slice()));
        v13.route("c5", &p, "old-label", None);

        v13.indexed(&p, "alice-label", Some(100));
        v13.indexed(&p, "typed-label", None);
        v13.indexed(&q, "carol-a", Some(200));
        v13.indexed(&q, "carol-b", Some(300));
        v13.indexed(&r, "bob-label", None);
        v13.indexed(&p, "stranger", None);
        v13.indexed(&p, "old-label", None);

        v13.unread(1, &p, Some("alice-label"), None);
        v13.unread_at(2, &p, Some("alice-label"), None, 2, "card");
        v13.unread(3, &p, Some("typed-label"), None);
        v13.unread(4, &p, Some("stranger"), None);
        // A card of a pair (c) discards is a card proposal, not a message.
        v13.unread_at(6, &p, Some("stranger"), None, 6, "card");
        // Two unread copies meet: the earlier stays, from either route.
        v13.unread_at(5, &q, Some("carol-a"), None, 10, "text");
        v13.unread_at(5, &q, Some("carol-b"), None, 20, "text");
        v13.unread_at(7, &q, Some("carol-b"), None, 5, "text");
        v13.unread_at(7, &q, Some("carol-a"), None, 15, "text");
        // An unread copy meets the other route's kept row, and its read pair.
        v13.unread(9, &q, Some("carol-a"), None);
        v13.kept(9, &q, "carol-b", 9);
        v13.unread(10, &q, Some("carol-b"), None);
        v13.pair(10, &q, Some("carol-a"), None, 5);
        v13.unread(11, &p, None, Some("room"));
        // An unread copy beside its own route's kept row is no meeting.
        v13.unread(12, &q, Some("carol-a"), None);
        v13.kept(12, &q, "carol-a", 12);

        v13.kept(13, &p, "alice-label", 13);
        v13.kept(14, &p, "typed-label", 14);
        v13.kept(15, &q, "carol-a", 50);
        v13.kept(15, &q, "carol-b", 40);

        v13.pair(16, &q, Some("carol-a"), None, 7);
        v13.pair(16, &q, Some("carol-b"), None, 3);
        v13.pair(17, &p, Some("stranger"), None, 1);
        v13.pair(18, &p, None, Some("room"), 1);

        v13.pending(20, &p, Some("alice-label"), None, 0);
        v13.pending(21, &p, Some("stranger"), None, 2);
        v13.pending(22, &p, None, Some("room"), 0);
        v13.pending(23, &q, Some("carol-b"), None, 1);

        conn.execute(
            "INSERT INTO own_cards (endpoint_id, name, created_at, updated_at)
             VALUES ('human', 'Me', 5, 5)",
            [],
        )
        .expect("the label's own card");
    }

    let store = HumanStore::open(&path, &me(), StoreOptions::default()).expect("migrates");
    assert_eq!(store.identity(), &me());
    assert_eq!(
        store.discarded_at_v14().expect("the counts"),
        Some(DiscardedAtV14 {
            conversations: 3,
            unread: 2,
            kept: 1,
            pending_never_attempted: 2,
            pending_may_have_reached: 2,
            contacts: 1,
            contacts_combined: 1,
            card_proposals: 2,
        }),
        "typed-label, stranger and old-label; unread 3 and 4 (the later 5 and 7, 9 and 10 \
         counted nowhere); kept 14 (the later 15 nowhere); c2 with its last route; c4 \
         combined into c3; the cards 2 and 6"
    );
    // What stays reads back through the store as this identity's.
    let unread = store.unread_inbound().expect("unread");
    assert_eq!(unread.len(), 5);
    let mine = unread
        .iter()
        .find(|row| row.app_message_id.as_str() == id(1))
        .expect("Alice's text stays");
    assert_eq!(mine.origin.endpoint.as_ref(), Some(&alice));
    assert_eq!(
        mine.payload,
        payload(1, Some("alice-label")),
        "bytes untouched"
    );
    assert_eq!(store.kept_inbound().expect("kept").len(), 4);
    assert!(store.pending_outbound().expect("pending").is_empty());
    let contacts = store.contacts().expect("contacts");
    let names: Vec<&str> = contacts.iter().map(|c| c.display_name.as_str()).collect();
    assert_eq!(names, ["Alice", "Carol A", "Mixed"]);
    assert_eq!(
        store
            .card_shared_at(&TransportIdentity::parse(&q).expect("Q"), &carol)
            .expect("read"),
        Some(300),
        "the merged conversation carries the later share"
    );
    assert_eq!(
        store.own_card(&me()).expect("read").map(|c| c.name),
        Some("Me".to_owned())
    );
    drop(store);

    let conn = rusqlite::Connection::open(&path).expect("read back");
    let me = me();
    let m = me.as_str();
    let ep = |e: &EndpointId| Some(e.as_str().to_owned());
    assert_eq!(
        rows(&conn, "unread_inbound", m),
        vec![
            (id(1), p.clone(), ep(&alice), None),
            (id(5), q.clone(), ep(&carol), None),
            (id(7), q.clone(), ep(&carol), None),
            (id(11), p.clone(), None, Some("room".to_owned())),
            (id(12), q.clone(), ep(&carol), None),
        ]
    );
    let received: Vec<(String, i64)> = conn
        .prepare("SELECT app_message_id, received_at FROM unread_inbound WHERE app_message_id IN (?1, ?2) ORDER BY app_message_id")
        .expect("prepare")
        .query_map([id(5), id(7)], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("rows")
        .collect::<Result<_, _>>()
        .expect("read");
    assert_eq!(
        received,
        [(id(5), 10), (id(7), 5)],
        "the earlier copy of each"
    );
    assert_eq!(
        rows(&conn, "kept_inbound", m),
        vec![
            (id(9), q.clone(), ep(&carol), None),
            (id(12), q.clone(), ep(&carol), None),
            (id(13), p.clone(), ep(&alice), None),
            (id(15), q.clone(), ep(&carol), None),
        ]
    );
    let kept15: i64 = conn
        .query_row(
            "SELECT received_at FROM kept_inbound WHERE app_message_id = ?1",
            [id(15)],
            |r| r.get(0),
        )
        .expect("read");
    assert_eq!(kept15, 40, "the earlier kept copy");
    assert_eq!(
        rows(&conn, "read_pairs", m),
        vec![
            (id(10), q.clone(), ep(&carol), None),
            (id(16), q.clone(), ep(&carol), None),
            (id(18), p.clone(), None, Some("room".to_owned())),
        ]
    );
    let pair16: i64 = conn
        .query_row(
            "SELECT at FROM read_pairs WHERE app_message_id = ?1",
            [id(16)],
            |r| r.get(0),
        )
        .expect("read");
    assert_eq!(pair16, 3, "the earlier pair");
    let routes: Vec<(String, String, String, String)> = conn
        .prepare("SELECT contact_id, local_endpoint_id, peer_id, endpoint_id FROM contact_routes ORDER BY contact_id")
        .expect("prepare")
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .expect("rows")
        .collect::<Result<_, _>>()
        .expect("read");
    assert_eq!(
        routes,
        [
            (
                "c1".to_owned(),
                m.to_owned(),
                p.clone(),
                alice.as_str().to_owned()
            ),
            (
                "c3".to_owned(),
                m.to_owned(),
                q.clone(),
                carol.as_str().to_owned()
            ),
            (
                "c5".to_owned(),
                m.to_owned(),
                r.clone(),
                bob.as_str().to_owned()
            ),
        ],
        "the route under the contact created first stays"
    );
    let index: Vec<(String, String, Option<i64>)> = conn
        .prepare("SELECT conversation_id, local_endpoint_id, card_shared_at FROM conversation_index ORDER BY conversation_id")
        .expect("prepare")
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("rows")
        .collect::<Result<_, _>>()
        .expect("read");
    let mut expected = vec![
        (
            format!("direct/{p}/{}", alice.as_str()),
            m.to_owned(),
            Some(100),
        ),
        (
            format!("direct/{q}/{}", carol.as_str()),
            m.to_owned(),
            Some(300),
        ),
        (format!("direct/{r}/{}", bob.as_str()), m.to_owned(), None),
    ];
    expected.sort();
    assert_eq!(index, expected);
    for table in [
        "contacts",
        "unread_inbound",
        "kept_inbound",
        "read_pairs",
        "contact_routes",
        "conversation_index",
    ] {
        let foreign: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE local_endpoint_id <> ?1"),
                [m],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(foreign, 0, "{table}: every row is the first identity's");
    }
    let own: Vec<String> = conn
        .prepare("SELECT endpoint_id FROM own_cards")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("rows")
        .collect::<Result<_, _>>()
        .expect("read");
    assert_eq!(own, [m.to_owned()], "rekeyed from the label");
    assert_eq!(total(&conn, "pending_outbound"), 0);
    assert_eq!(
        total(&conn, "sqlite_master WHERE name = 'v14_route_map'"),
        0
    );
}

/// The counts row is kept until the person's acknowledgement: read at
/// every open until the client deletes it, and not written again by a
/// later open; a store made at v14 has none. Mutation: the row deleted
/// when it is first read.
#[test]
fn the_counts_row_stays_until_it_is_deleted() {
    let (_dir, path) = v13_store();
    {
        let conn = rusqlite::Connection::open(&path).expect("v13");
        V13(&conn).pending(1, &node("p"), None, Some("room"), 0);
    }
    let counted = Some(DiscardedAtV14 {
        pending_never_attempted: 1,
        ..DiscardedAtV14::default()
    });
    let store = HumanStore::open(&path, &me(), StoreOptions::default()).expect("migrates");
    assert_eq!(store.discarded_at_v14().expect("read"), counted);
    drop(store);
    let mut store = HumanStore::open(&path, &me(), StoreOptions::default()).expect("reopens");
    assert_eq!(
        store.discarded_at_v14().expect("read"),
        counted,
        "a second open still holds it: nothing was acknowledged"
    );
    store.forget_discarded_at_v14().expect("deleted");
    store
        .forget_discarded_at_v14()
        .expect("deleted again is no error");
    assert_eq!(store.discarded_at_v14().expect("read"), None);
    drop(store);
    let store = HumanStore::open(&path, &me(), StoreOptions::default()).expect("reopens");
    assert_eq!(store.discarded_at_v14().expect("read"), None);

    let fresh = HumanStore::open_in_memory(&me(), StoreOptions::default()).expect("fresh");
    assert_eq!(fresh.discarded_at_v14().expect("read"), None);
}

/// A migration that discarded nothing writes no row: it exists to be
/// acknowledged (A 2026-11-22). The control: a store whose migration
/// kept everything, a keyed route with its unread and kept texts, a
/// channel row and a read pair. Mutation: the row written whatever the
/// counts.
#[test]
fn an_upgrade_that_discarded_nothing_writes_no_counts_row() {
    let (_dir, path) = v13_store();
    let p = node("p");
    {
        let conn = rusqlite::Connection::open(&path).expect("v13");
        let v13 = V13(&conn);
        v13.contact("c1", "Alice", 10);
        v13.route("c1", &p, "alice-label", Some(key("alice").as_slice()));
        v13.indexed(&p, "alice-label", Some(1));
        v13.unread(1, &p, Some("alice-label"), None);
        v13.kept(2, &p, "alice-label", 2);
        v13.unread(3, &p, None, Some("room"));
        v13.pair(4, &p, Some("alice-label"), None, 4);
    }
    let store = HumanStore::open(&path, &me(), StoreOptions::default()).expect("migrates");
    assert_eq!(store.unread_inbound().expect("unread").len(), 2, "control");
    assert_eq!(store.kept_inbound().expect("kept").len(), 1, "control");
    assert_eq!(store.discarded_at_v14().expect("read"), None);
    drop(store);
    assert_eq!(
        total(
            &rusqlite::Connection::open(&path).expect("read back"),
            "settings WHERE key = 'v14_discarded'"
        ),
        0
    );
}

/// The counts are by the row (STATE.md §Migrations, A 2026-11-22 (ii)): a
/// pending row and an unread card of a conversation (c) discards are each
/// counted once, in its pending part and in card proposals, and the card
/// is no unread message. Mutations: (c) taking its pair's pending rows
/// before the pending parts are counted; the card proposals deleted after
/// (c).
#[test]
fn a_discarded_conversations_pending_row_and_card_are_counted_by_the_row() {
    let (_dir, path) = v13_store();
    let p = node("p");
    {
        let conn = rusqlite::Connection::open(&path).expect("v13");
        let v13 = V13(&conn);
        v13.indexed(&p, "stranger", None);
        v13.pending(1, &p, Some("stranger"), None, 0);
        v13.pending(2, &p, Some("stranger"), None, 3);
        v13.unread_at(3, &p, Some("stranger"), None, 3, "card");
    }
    let store = HumanStore::open(&path, &me(), StoreOptions::default()).expect("migrates");
    assert_eq!(
        store.discarded_at_v14().expect("read"),
        Some(DiscardedAtV14 {
            conversations: 1,
            pending_never_attempted: 1,
            pending_may_have_reached: 1,
            card_proposals: 1,
            ..DiscardedAtV14::default()
        })
    );
    assert!(store.unread_inbound().expect("unread").is_empty());
    assert!(store.pending_outbound().expect("pending").is_empty());
}

/// Which of the three tables a channel's row is seeded in.
#[derive(Clone, Copy)]
enum Held {
    Unread,
    Kept,
    Pair,
}

impl Held {
    fn table(self) -> &'static str {
        match self {
            Self::Unread => "unread_inbound",
            Self::Kept => "kept_inbound",
            Self::Pair => "read_pairs",
        }
    }

    /// Message `n` from `peer` under `endpoint` on `channel`, at `at`.
    fn seed(self, v13: &V13<'_>, n: u32, peer: &str, endpoint: Option<&str>, at: i64) {
        let channel = Some("room");
        match self {
            Self::Unread => v13.unread_at(n, peer, endpoint, channel, at, "text"),
            Self::Kept => v13.kept_in(n, peer, endpoint, channel, at),
            Self::Pair => v13.pair(n, peer, endpoint, channel, at),
        }
    }
}

/// A channel's row in `held` follows a keyed route naming its publisher's
/// `{peer, label}` to the id the key derives, and where no keyed route
/// names it the row stays with its label NULL; nothing discarded, nothing
/// counted (STATE.md v14 (b)-(c), A 2026-11-23). The store reads every row
/// back: a label left in place read Corrupt (p2p-network-dev-01's finding).
fn a_channel_row_follows_a_keyed_route_or_loses_its_label(held: Held) {
    let (_dir, path) = v13_store();
    let p = node("p");
    {
        let conn = rusqlite::Connection::open(&path).expect("v13");
        let v13 = V13(&conn);
        v13.contact("c1", "Alice", 10);
        v13.route("c1", &p, "alice-label", Some(key("alice").as_slice()));
        held.seed(&v13, 1, &p, Some("alice-label"), 1);
        held.seed(&v13, 2, &p, Some("stranger"), 2);
        held.seed(&v13, 3, &p, None, 3);
    }
    let store = HumanStore::open(&path, &me(), StoreOptions::default()).expect("migrates");
    assert_eq!(
        store.discarded_at_v14().expect("read"),
        None,
        "counted nowhere"
    );
    let read = match held {
        Held::Unread => Some(store.unread_inbound().expect("every row reads")),
        Held::Kept => Some(store.kept_inbound().expect("every row reads")),
        Held::Pair => None,
    };
    if let Some(read) = read {
        let endpoints: Vec<_> = read.iter().map(|row| row.origin.endpoint.clone()).collect();
        assert_eq!(endpoints, vec![Some(endpoint_id("alice")), None, None]);
    }
    drop(store);
    let conn = rusqlite::Connection::open(&path).expect("read back");
    let alice = endpoint_id("alice").as_str().to_owned();
    let room = Some("room".to_owned());
    assert_eq!(
        rows(&conn, held.table(), me().as_str()),
        vec![
            (id(1), p.clone(), Some(alice), room.clone()),
            (id(2), p.clone(), None, room.clone()),
            (id(3), p, None, room),
        ],
        "rebound, NULLed, and the NULL it already held"
    );
}

/// Mutations: the unread copy's rebinding kept to direct rows; its NULL
/// left out (the label read back Corrupt).
#[test]
fn a_channel_unread_row_follows_a_keyed_route_or_loses_its_label() {
    a_channel_row_follows_a_keyed_route_or_loses_its_label(Held::Unread);
}

/// Mutations: as for the unread copy, on the kept copy.
#[test]
fn a_channel_kept_row_follows_a_keyed_route_or_loses_its_label() {
    a_channel_row_follows_a_keyed_route_or_loses_its_label(Held::Kept);
}

/// Mutations: as for the unread copy, on the read pairs' copy.
#[test]
fn a_channel_read_pair_follows_a_keyed_route_or_loses_its_label() {
    a_channel_row_follows_a_keyed_route_or_loses_its_label(Held::Pair);
}

/// One publisher's one message on one channel held under two labels, or a
/// label and none, is brought to one key by the NULL, and under two labels
/// whose routes' keys derive one id by the rebinding: within each table the
/// one received (or read) earlier stays and the other is discarded,
/// counted nowhere (STATE.md v14 (d), A 2026-11-23). Mutation: the
/// within-table meeting kept to direct rows, which fails the copy on the
/// table's key.
#[test]
fn a_channel_message_held_under_two_labels_is_one_row_the_earlier() {
    for held in [Held::Unread, Held::Kept, Held::Pair] {
        let (_dir, path) = v13_store();
        let (p, q) = (node("p"), node("q"));
        {
            let conn = rusqlite::Connection::open(&path).expect("v13");
            let v13 = V13(&conn);
            v13.contact("c3", "Carol A", 30);
            v13.route("c3", &q, "carol-a", Some(key("carol").as_slice()));
            v13.contact("c4", "Carol B", 40);
            v13.route("c4", &q, "carol-b", Some(key("carol").as_slice()));
            // NULL meets NULL: "y" earliest stays.
            held.seed(&v13, 1, &p, Some("x"), 20);
            held.seed(&v13, 1, &p, Some("y"), 10);
            held.seed(&v13, 1, &p, None, 30);
            // The rebinding meets itself: carol-b earlier stays.
            held.seed(&v13, 2, &q, Some("carol-a"), 20);
            held.seed(&v13, 2, &q, Some("carol-b"), 10);
            // A rebound row and a NULLed one keep two keys.
            held.seed(&v13, 3, &q, Some("carol-a"), 1);
            held.seed(&v13, 3, &q, Some("someone"), 2);
        }
        let store = HumanStore::open(&path, &me(), StoreOptions::default()).expect("migrates");
        assert_eq!(
            store.discarded_at_v14().expect("read"),
            Some(DiscardedAtV14 {
                contacts_combined: 1,
                ..DiscardedAtV14::default()
            }),
            "c4 combined into c3; no meeting counted"
        );
        drop(store);
        let conn = rusqlite::Connection::open(&path).expect("read back");
        let carol = Some(endpoint_id("carol").as_str().to_owned());
        let room = Some("room".to_owned());
        assert_eq!(
            rows(&conn, held.table(), me().as_str()),
            vec![
                (id(1), p.clone(), None, room.clone()),
                (id(2), q.clone(), carol.clone(), room.clone()),
                (id(3), q.clone(), None, room.clone()),
                (id(3), q.clone(), carol, room.clone()),
            ],
        );
        let at = if matches!(held, Held::Pair) {
            "at"
        } else {
            "received_at"
        };
        let stayed: Vec<i64> = conn
            .prepare(&format!(
                "SELECT {at} FROM {} WHERE app_message_id IN (?1, ?2) ORDER BY app_message_id",
                held.table()
            ))
            .expect("prepare")
            .query_map([id(1), id(2)], |r| r.get(0))
            .expect("rows")
            .collect::<Result<_, _>>()
            .expect("read");
        assert_eq!(stayed, vec![10, 10], "the earlier of each meeting");
    }
}

/// A channel's unread copy that the NULL brings to the key of a kept row
/// or a read pair of the same message is discarded in their favour, while
/// an unread copy beside a kept row under its own label, a state v13
/// allows, is no meeting and stays; none of it counted (STATE.md v14 (d),
/// A 2026-11-23). Mutation: the cross-table meeting kept to direct rows.
#[test]
fn a_channel_unread_copy_meeting_a_kept_row_or_read_pair_goes() {
    let (_dir, path) = v13_store();
    let p = node("p");
    {
        let conn = rusqlite::Connection::open(&path).expect("v13");
        let v13 = V13(&conn);
        v13.unread_at(1, &p, Some("x"), Some("room"), 1, "text");
        v13.kept_in(1, &p, Some("y"), Some("room"), 5);
        v13.unread_at(2, &p, Some("x"), Some("room"), 1, "text");
        v13.pair(2, &p, None, Some("room"), 5);
        v13.unread_at(3, &p, Some("x"), Some("room"), 1, "text");
        v13.kept_in(3, &p, Some("x"), Some("room"), 5);
    }
    let store = HumanStore::open(&path, &me(), StoreOptions::default()).expect("migrates");
    assert_eq!(
        store.discarded_at_v14().expect("read"),
        None,
        "counted nowhere"
    );
    let unread: Vec<String> = store
        .unread_inbound()
        .expect("unread")
        .iter()
        .map(|row| row.app_message_id.as_str().to_owned())
        .collect();
    assert_eq!(unread, vec![id(3)], "1 met a kept row, 2 a read pair");
    assert_eq!(store.kept_inbound().expect("kept").len(), 2);
    drop(store);
    let conn = rusqlite::Connection::open(&path).expect("read back");
    assert_eq!(total(&conn, "read_pairs"), 1);
}

/// One `{peer, label}` pair under two contacts with two different keys,
/// which no writer of the store makes, fails the migration rather than
/// being settled either way: the transaction rolls back, the store stays
/// at v13 with every row it had, and the open is refused as any failed
/// migration is (STATE.md §Migrations (d), A 2026-11-22). Mutation:
/// `route_map` keeping the first key's id instead of refusing.
#[test]
fn one_pair_under_two_contacts_with_two_keys_fails_the_migration() {
    let (_dir, path) = v13_store();
    let p = node("p");
    {
        let conn = rusqlite::Connection::open(&path).expect("v13");
        let v13 = V13(&conn);
        v13.contact("c1", "Alice", 10);
        v13.route("c1", &p, "shared-label", Some(key("alice").as_slice()));
        v13.contact("c2", "Bob", 20);
        v13.route("c2", &p, "shared-label", Some(key("bob").as_slice()));
        v13.indexed(&p, "shared-label", None);
        v13.unread(1, &p, Some("shared-label"), None);
        v13.pending(2, &p, Some("shared-label"), None, 0);
    }
    match HumanStore::open(&path, &me(), StoreOptions::default()) {
        Err(error @ StoreError::Migration(_)) => {
            assert!(error.needs_recovery(), "{error}");
            assert!(error.to_string().contains("two identity keys"), "{error}");
        }
        other => panic!("one pair under two keys must fail the migration: {other:?}"),
    }
    assert_eq!(version(&path), 13);
    let conn = rusqlite::Connection::open(&path).expect("read back");
    for (table, rows) in [
        ("contacts", 2),
        ("contact_routes", 2),
        ("conversation_index", 1),
        ("unread_inbound", 1),
        ("pending_outbound", 1),
        ("settings WHERE key = 'v14_discarded'", 0),
        ("sqlite_master WHERE name = 'v14_route_map'", 0),
    ] {
        assert_eq!(total(&conn, table), rows, "{table}");
    }
    let labels: Vec<String> = conn
        .prepare("SELECT endpoint_id FROM contact_routes ORDER BY contact_id")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("rows")
        .collect::<Result<_, _>>()
        .expect("read");
    assert_eq!(labels, ["shared-label", "shared-label"], "rows untouched");
}

/// A row naming a parent the store does not hold fails the migration, which
/// ran with foreign keys off, before its commit: the store stays at v13
/// with every row it had (STATE.md §Migrations; architect-cto's T2).
/// Mutation: the foreign-key check before the commit left out.
#[test]
fn an_orphan_route_fails_the_migration_and_leaves_v13() {
    let (_dir, path) = v13_store();
    {
        let conn = rusqlite::Connection::open(&path).expect("v13");
        conn.execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("off");
        V13(&conn).route(
            "gone",
            &node("p"),
            "alice-label",
            Some(key("alice").as_slice()),
        );
        V13(&conn).unread(1, &node("p"), Some("alice-label"), None);
    }
    match HumanStore::open(&path, &me(), StoreOptions::default()) {
        Err(error @ StoreError::Migration(_)) => {
            assert!(error.needs_recovery(), "{error}");
            assert!(error.to_string().contains("parent row"), "{error}");
        }
        other => panic!("an orphan route must fail the migration: {other:?}"),
    }
    assert_eq!(version(&path), 13);
    let conn = rusqlite::Connection::open(&path).expect("read back");
    assert_eq!(total(&conn, "contact_routes"), 1);
    assert_eq!(total(&conn, "unread_inbound"), 1);
    assert_eq!(
        total(&conn, "sqlite_master WHERE name = 'v14_route_map'"),
        0
    );
}

/// The shape guard holds v14: a file still in v13's shape that claims
/// v14 is refused, and so is a v14 file whose kind index lost the
/// identity it is led by. Mutation: `unread_inbound`'s expected shape
/// without `local_endpoint_id`.
#[test]
fn the_shape_guard_refuses_a_v13_shape_and_an_index_without_its_identity() {
    let (_dir, path) = v13_store();
    rusqlite::Connection::open(&path)
        .expect("v13")
        .execute_batch("PRAGMA user_version = 14;")
        .expect("claims v14");
    match HumanStore::open(&path, &me(), StoreOptions::default()) {
        Err(StoreError::Migration(why)) => assert!(why.contains("pending_outbound"), "{why}"),
        other => panic!("a v13 shape is not v14: {other:?}"),
    }

    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    drop(HumanStore::open(&path, &me(), StoreOptions::default()).expect("v14"));
    rusqlite::Connection::open(&path)
        .expect("v14")
        .execute_batch(
            "DROP INDEX unread_inbound_by_kind;
             CREATE INDEX unread_inbound_by_kind
                 ON unread_inbound (kind, source_peer, source_endpoint, channel_id, app_message_id);",
        )
        .expect("v13's index under v14's name");
    match HumanStore::open(&path, &me(), StoreOptions::default()) {
        Err(StoreError::Migration(why)) => assert!(why.contains("unread_inbound"), "{why}"),
        other => panic!("an index without its identity is refused: {other:?}"),
    }
}

/// The routes' foreign key is ONE key over two columns: split into two
/// single-column keys it references no unique parent, and the guard
/// refuses it, whichever of the two is declared first (SQLite numbers a
/// table's keys in its own order, and a guard reading the rows in that
/// order must still tell two keys from one). Mutation:
/// `actual_foreign_keys` joining every row of a table into one key
/// whatever its id.
#[test]
fn a_composite_route_key_split_into_two_is_refused() {
    let by_contact = "FOREIGN KEY (contact_id) REFERENCES contacts (contact_id) ON DELETE CASCADE";
    let by_identity = "FOREIGN KEY (local_endpoint_id)
                     REFERENCES contacts (local_endpoint_id) ON DELETE CASCADE";
    for (first, second) in [(by_contact, by_identity), (by_identity, by_contact)] {
        let dir = common::private_tempdir();
        let path = dir.path().join("state").join("human.sqlite3");
        drop(HumanStore::open(&path, &me(), StoreOptions::default()).expect("v14"));
        rusqlite::Connection::open(&path)
            .expect("v14")
            .execute_batch(&format!(
                "PRAGMA foreign_keys = OFF;
                 CREATE TABLE contact_routes_split (
                     contact_id         TEXT NOT NULL,
                     local_endpoint_id  TEXT NOT NULL,
                     peer_id            TEXT NOT NULL,
                     endpoint_id        TEXT NOT NULL,
                     device_label       TEXT,
                     verification_note  TEXT,
                     last_seen          INTEGER,
                     identity_key       BLOB,
                     UNIQUE(local_endpoint_id, peer_id, endpoint_id),
                     UNIQUE(contact_id, peer_id),
                     {first},
                     {second}
                 );
                 DROP TABLE contact_routes;
                 ALTER TABLE contact_routes_split RENAME TO contact_routes;
                 CREATE INDEX contact_routes_by_route ON contact_routes (peer_id, endpoint_id);
                 CREATE INDEX contact_routes_by_key ON contact_routes (identity_key);"
            ))
            .expect("split");
        match HumanStore::open(&path, &me(), StoreOptions::default()) {
            Err(StoreError::Migration(why)) => assert!(why.contains("foreign keys"), "{why}"),
            other => panic!("two keys are not the composite one ({first}): {other:?}"),
        }
    }
}

fn inbound(n: u32, peer: &TransportIdentity, endpoint: &EndpointId) -> NewInbound {
    NewInbound {
        app_message_id: AppMessageId::parse(id(n)).expect("an id"),
        origin: InboundOrigin {
            peer: peer.clone(),
            endpoint: Some(endpoint.clone()),
            channel: None,
        },
        media_type: None,
        payload: vec![1],
        received_at: u64::from(n),
        kind: InboundKind::Text,
    }
}

/// One message that reached two identities is two rows, each identity
/// reads and counts its own, and reading it under one suppresses no copy
/// under the other (schema v14: the identity leads the inbound key and the
/// read pairs'). Mutations: the read-pair check of the commit without its
/// identity term; the unread walk without its identity term.
#[test]
fn a_message_reaching_two_identities_is_two_rows() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let peer = TransportIdentity::parse(node("p")).expect("a PeerId");
    let alice = endpoint_id("alice");

    let mut first = HumanStore::open(&path, &me(), StoreOptions::default()).expect("me");
    let row = first
        .commit_unread_inbound(&inbound(1, &peer, &alice))
        .expect("mine");
    first.mark_read(row, 2).expect("read under me");
    assert!(
        first
            .commit_unread_inbound(&inbound(1, &peer, &alice))
            .expect_err("a copy read under me")
            .is_duplicate()
    );
    drop(first);

    let mut second = HumanStore::open(&path, &other(), StoreOptions::default()).expect("other");
    second
        .commit_unread_inbound(&inbound(1, &peer, &alice))
        .expect("the other identity's own copy");
    assert_eq!(second.unread_inbound().expect("unread").len(), 1);
    assert_eq!(second.unread_counts().expect("counts").len(), 1);
    assert!(second.holds_endpoint(&peer, &alice).expect("indexed"));
    drop(second);

    let first = HumanStore::open(&path, &me(), StoreOptions::default()).expect("me again");
    assert!(
        first.unread_inbound().expect("unread").is_empty(),
        "the other's row is not mine"
    );
    assert!(first.unread_counts().expect("counts").is_empty());
}

/// The read pairs are bounded per identity: at [`READ_PAIR_CAP`] one
/// identity's newest read evicts its own oldest pair and never another's.
/// Mutation: the eviction ordering every identity's pairs together.
#[test]
fn one_identitys_reads_never_evict_anothers_pairs() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let peer = TransportIdentity::parse(node("p")).expect("a PeerId");
    let alice = endpoint_id("alice");

    // The other identity's one pair is the oldest in the table.
    let mut second = HumanStore::open(&path, &other(), StoreOptions::default()).expect("other");
    let row = second
        .commit_unread_inbound(&inbound(1, &peer, &alice))
        .expect("theirs");
    second.mark_read(row, 1).expect("read under the other");
    drop(second);
    // Mine fill my bound, planted in one transaction.
    {
        // Derived once: a key's derivation per row is most of a debug
        // build's minute.
        let mine = me();
        let mut conn = rusqlite::Connection::open(&path).expect("plant");
        let tx = conn.transaction().expect("tx");
        for n in 0..READ_PAIR_CAP {
            tx.execute(
                "INSERT INTO read_pairs
                     (local_endpoint_id, app_message_id, source_peer, source_endpoint, at)
                 VALUES (?1, ?2, ?3, ?4, 2)",
                rusqlite::params![
                    mine.as_str(),
                    format!("{:032x}", 0xe000_u128 + n as u128),
                    peer.as_str(),
                    alice.as_str()
                ],
            )
            .expect("a pair");
        }
        tx.commit().expect("planted");
    }
    let mut first = HumanStore::open(&path, &me(), StoreOptions::default()).expect("me");
    let row = first
        .commit_unread_inbound(&inbound(2, &peer, &alice))
        .expect("mine");
    first.mark_read(row, 3).expect("read under me");
    drop(first);

    let conn = rusqlite::Connection::open(&path).expect("read back");
    let count = |local: &EndpointId| -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM read_pairs WHERE local_endpoint_id = ?1",
            [local.as_str()],
            |r| r.get(0),
        )
        .expect("a count")
    };
    assert_eq!(count(&me()), i64::try_from(READ_PAIR_CAP).expect("fits"));
    assert_eq!(count(&other()), 1, "the other identity's pair stays");
    let evicted: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM read_pairs WHERE app_message_id = ?1",
            [format!("{:032x}", 0xe000_u128)],
            |r| r.get(0),
        )
        .expect("a count");
    assert_eq!(evicted, 0, "my oldest went");
}

/// The per-peer bound of 64 endpoints is counted per identity: another
/// identity's conversations and routes with the same peer count for it
/// alone (STATE.md v14). Mutation: the routes' half of the count without
/// its identity term.
#[test]
fn the_endpoint_bound_is_counted_per_identity() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let peer = TransportIdentity::parse(node("p")).expect("a PeerId");

    let mut second = HumanStore::open(&path, &other(), StoreOptions::default()).expect("other");
    for n in 0..64 {
        second
            .commit_unread_inbound(&inbound(n, &peer, &endpoint_id(&format!("e{n}"))))
            .expect("one of the other's 64");
    }
    second
        .accept_card(
            &interweave_human_store::AcceptedCard {
                peer: &peer,
                endpoint: &endpoint_id("card"),
                name: "Ada",
                surname: None,
                identity_key: signing_key("ada").verifying_key().to_bytes(),
                photo: None,
            },
            1,
        )
        .expect("a route of the other's");
    assert_eq!(second.remote_endpoints(&peer).expect("count"), 65);
    drop(second);

    let mut first = HumanStore::open(&path, &me(), StoreOptions::default()).expect("me");
    assert_eq!(
        first.remote_endpoints(&peer).expect("count"),
        0,
        "none of them mine"
    );
    assert!(
        !first
            .holds_endpoint(&peer, &endpoint_id("card"))
            .expect("not mine")
    );
    first
        .commit_unread_inbound(&inbound(100, &peer, &endpoint_id("e0")))
        .expect("mine");
    assert_eq!(first.remote_endpoints(&peer).expect("count"), 1);
    assert!(
        first.contacts().expect("contacts").is_empty(),
        "the other's contact is not mine"
    );
}

/// A store created at this build holds no free page: the migrations build
/// every earlier shape before v14 replaces six tables, and the open
/// compacts the new file once, so a quota counts content and not the
/// replaced tables. Mutation: the compaction of a new store left out.
#[test]
fn a_new_store_keeps_no_page_the_migrations_replaced() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    drop(HumanStore::open(&path, &me(), StoreOptions::default()).expect("created"));
    let free: i64 = rusqlite::Connection::open(&path)
        .expect("read back")
        .query_row("PRAGMA freelist_count", [], |r| r.get(0))
        .expect("the free pages");
    assert_eq!(free, 0);
}

/// The version this build writes.
#[test]
fn this_build_writes_v14() {
    assert_eq!(SCHEMA_VERSION, 14);
}

fn outbound(n: u32, peer: &TransportIdentity, endpoint: &EndpointId) -> NewOutbound {
    NewOutbound {
        app_message_id: AppMessageId::parse(id(n)).expect("an id"),
        transport_message_id: MessageId::parse_hex(&format!("{:032x}", u128::from(n) + (1 << 100)))
            .expect("an id"),
        destination: OutboundDestination::Direct(DirectDestination {
            peer: peer.clone(),
            endpoint: Some(endpoint.clone()),
        }),
        media_type: None,
        payload: vec![1],
        created_at: u64::from(n),
    }
}

/// A handle opened as the first identity and re-scoped with
/// `speaking_as` speaks as the other: it lists none of the first's rows,
/// and what it writes the first does not see; the open's own argument,
/// the migration's target, is untouched, so the first reopened finds its
/// rows as they were. Mutation: `speaking_as` returning the handle without
/// its new identity.
#[test]
fn a_handle_speaking_as_another_identity_reads_and_writes_its_rows_alone() {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let peer = TransportIdentity::parse(node("p")).expect("a PeerId");
    let alice = endpoint_id("alice");

    let mut first = HumanStore::open(&path, &me(), StoreOptions::default()).expect("me");
    first
        .commit_unread_inbound(&inbound(1, &peer, &alice))
        .expect("mine");
    first
        .commit_pending_outbound(&outbound(2, &peer, &alice))
        .expect("my send");
    drop(first);

    let mut second = HumanStore::open(&path, &me(), StoreOptions::default())
        .expect("opened as the first")
        .speaking_as(other());
    assert_eq!(second.identity(), &other());
    assert!(
        second.unread_inbound().expect("unread").is_empty(),
        "the first's unread row is not the other's"
    );
    assert!(
        second.pending_outbound().expect("pending").is_empty(),
        "the first's send is not the other's"
    );
    second
        .commit_unread_inbound(&inbound(3, &peer, &alice))
        .expect("the other's");
    second
        .commit_pending_outbound(&outbound(4, &peer, &alice))
        .expect("the other's send");
    assert_eq!(second.unread_inbound().expect("unread").len(), 1);
    assert_eq!(second.pending_outbound().expect("pending").len(), 1);
    drop(second);

    let first = HumanStore::open(&path, &me(), StoreOptions::default()).expect("me again");
    let unread = first.unread_inbound().expect("unread");
    assert_eq!(unread.len(), 1, "mine alone, as they were");
    assert_eq!(unread[0].app_message_id.as_str(), id(1));
    let pending = first.pending_outbound().expect("pending");
    assert_eq!(pending.len(), 1, "my send alone");
    assert_eq!(pending[0].app_message_id.as_str(), id(2));
}
