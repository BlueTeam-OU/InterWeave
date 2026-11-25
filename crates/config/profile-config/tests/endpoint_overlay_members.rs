// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The endpoint overlay's four members past `enabled` and `default`
//! (ADR-0067 decision 6): `inbound` and `outbound` (an endpoint's own
//! trusted sets), `added` (entries added through the administrative path)
//! and `advertise` (a delta over an authored entry). Each refusal is
//! fed as a file on disk, beside the control that the same file minus the
//! one fault loads; the state is judged through the loader the daemon
//! takes, and the write API through the overlay it returns.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::ops::Range;
use std::path::{Path, PathBuf};

use interweave_profile_config::endpoint_overlay::{
    ADDED_TRUST, ENDPOINT_OVERLAY_FILE, EndpointOverlay, EndpointOverlayError,
    MAX_ENDPOINT_OVERLAY_BYTES, Normalised, OverlayMember, Seeded,
};
use interweave_profile_config::{
    ClientKind, EndpointConfig, EndpointKeyState, EndpointsConfig, KeyMissing, MAX_ENDPOINTS,
    ProfilePaths, ResolveEndpointError, TrustBoundary, XdgRoots, endpoint_key, resolve_endpoint,
};
use interweave_transport_api::{EndpointId, TransportIdentity};
use interweave_trust_api::{MAX_ENDPOINT_SET_ROWS, PeerTrustPolicy};
use serde_json::{Value, json};

/// The TEST-ONLY derived id named `s` (tests/support).
fn id(s: &str) -> EndpointId {
    interweave_test_support::endpoint_keys::endpoint_id(s)
}

/// The `i`th synthetic Ed25519-form peer: the canonical prefix, then
/// bytes that differ with `i`. Always the longest form a `PeerId` has, 52
/// bytes.
fn peer(i: u64) -> TransportIdentity {
    let mut raw = vec![0x00, 0x24, 0x08, 0x01, 0x12, 0x20];
    raw.extend_from_slice(&i.to_be_bytes());
    raw.extend_from_slice(&[0x5a; 24]);
    let text = bs58::encode(raw).into_string();
    assert_eq!(text.len(), 52, "{text}");
    TransportIdentity::parse(text).expect("a canonical peer")
}

fn peers_json(range: Range<u64>) -> Vec<String> {
    range.map(|i| peer(i).as_str().to_owned()).collect()
}

fn key_of(name: &str) -> String {
    endpoint_key::key_file_name(&id(name))
}

/// `a`, `b` enabled and `c` disabled, none advertised, each aliased by its
/// name; no default.
fn config() -> EndpointsConfig {
    config_of(&["a", "b", "c"])
}

fn config_of(names: &[&str]) -> EndpointsConfig {
    let entries: Vec<Value> = names
        .iter()
        .map(|n| {
            json!({
                "key": key_of(n), "alias": n, "enabled": *n != "c", "advertise": false,
            })
        })
        .collect();
    serde_json::from_value(json!({ "entries": entries })).expect("an endpoints block")
}

/// An added entry: a `config.yaml` entry's fields, enabled and not
/// advertised.
fn added(name: &str, alias: Option<&str>) -> Value {
    let mut v = json!({"key": key_of(name), "enabled": true, "advertise": false});
    if let Some(alias) = alias {
        v["alias"] = json!(alias);
    }
    v
}

/// `{<id>: []}` for each name: the own, empty set an added entry must
/// stand in, in each of `inbound` and `outbound`.
fn empty_sets(names: &[&str]) -> Value {
    Value::Object(
        names
            .iter()
            .map(|n| (id(n).as_str().to_owned(), json!([])))
            .collect(),
    )
}

/// `file` with an own, empty set for each of `names` in both members:
/// the added entries' sets, which a loaded file always holds.
fn with_empty_sets(mut file: Value, names: &[&str]) -> Value {
    file["inbound"] = empty_sets(names);
    file["outbound"] = empty_sets(names);
    file
}

fn state() -> (tempfile::TempDir, PathBuf) {
    let dir = private_tempdir();
    let path = dir.path().join(ENDPOINT_OVERLAY_FILE);
    (dir, path)
}

fn private_tempdir() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt as _;
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("a temporary directory")
}

/// Write `value` at `path` as the daemon would: owner-only.
fn put(path: &Path, value: &Value) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::write(path, value.to_string()).expect("written");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
}

fn load(
    path: &Path,
    config: &EndpointsConfig,
) -> Result<(EndpointOverlay, Vec<Normalised>), EndpointOverlayError> {
    EndpointOverlay::load_within(path, config, &TrustBoundary::root())
}

/// `value` as a file, loaded against `config`.
fn loads(
    value: &Value,
    config: &EndpointsConfig,
) -> Result<(EndpointOverlay, Vec<Normalised>), EndpointOverlayError> {
    let (_dir, path) = state();
    put(&path, value);
    load(&path, config)
}

fn json_of(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("json")
}

fn set_ids(a: u64) -> Value {
    json!({ id("a").as_str(): peers_json(0..a) })
}

// ---- inbound and outbound ------------------------------------------

/// An id in one member alone is refused, naming the member it stands in;
/// in both, with different lists, it loads. Own-and-empty is the same
/// file with `[]`.
#[test]
fn an_id_in_one_member_alone_is_refused() {
    let both = json!({"enabled": {}, "inbound": set_ids(2), "outbound": set_ids(1)});
    let (overlay, changes) = loads(&both, &config()).expect("both members: the control loads");
    assert!(changes.is_empty(), "{changes:?}");
    let (inbound, outbound) = overlay.own_sets(&id("a")).expect("a's sets");
    assert_eq!((inbound.len(), outbound.len()), (2, 1));

    for (member, only_in) in [
        ("inbound", OverlayMember::Inbound),
        ("outbound", OverlayMember::Outbound),
    ] {
        let one = json!({"enabled": {}, member: set_ids(1)});
        match loads(&one, &config()) {
            Err(EndpointOverlayError::SetInOneMember {
                endpoint,
                only_in: got,
            }) => {
                assert_eq!((endpoint, got), (id("a"), only_in), "{member}");
            }
            other => panic!("{member}: expected SetInOneMember, got {other:?}"),
        }
    }
    // The other id's set, in both, does not excuse the one in one.
    let mixed = json!({"enabled": {}, "inbound": {id("a").as_str(): [], id("b").as_str(): []},
                       "outbound": {id("a").as_str(): []}});
    assert!(matches!(
        loads(&mixed, &config()),
        Err(EndpointOverlayError::SetInOneMember { endpoint, only_in: OverlayMember::Inbound })
            if endpoint == id("b")
    ));
}

/// An id present with `[]` is an own and empty set and an id absent has
/// none, through load, a normalising rewrite and a write and read.
#[test]
fn own_and_empty_survives_load_normalise_and_write_distinct_from_absent() {
    let (_dir, path) = state();
    // `enabled.a: true` equals config.yaml, so the load rewrites the file.
    put(
        &path,
        &json!({"enabled": {id("a").as_str(): true},
                "inbound": {id("a").as_str(): []}, "outbound": {id("a").as_str(): []}}),
    );
    let (overlay, changes) = load(&path, &config()).expect("loads");
    assert_eq!(changes, vec![Normalised::EnabledEqualsConfig(id("a"))]);
    let (inbound, outbound) = overlay.own_sets(&id("a")).expect("own and empty");
    assert!(inbound.is_empty() && outbound.is_empty());
    assert!(overlay.own_sets(&id("b")).is_none(), "absent is no own set");
    // The rewritten file still says present-and-empty.
    let text = json_of(&path);
    assert_eq!(text["inbound"], json!({id("a").as_str(): []}));
    assert_eq!(text["outbound"], json!({id("a").as_str(): []}));

    // A write and a read back, a second time over.
    overlay
        .write_within(&path, &TrustBoundary::root())
        .expect("written");
    let (back, changes) = load(&path, &config()).expect("loads");
    assert!(changes.is_empty(), "{changes:?}");
    assert_eq!(back, overlay);
    assert!(back.own_sets(&id("a")).is_some() && back.own_sets(&id("b")).is_none());

    // And with no sets the members are not written at all.
    let none = EndpointOverlay::default();
    none.write_within(&path, &TrustBoundary::root())
        .expect("written");
    assert_eq!(json_of(&path), json!({"enabled": {}}));
}

/// A set for an endpoint nothing configures leaves BOTH members; for an
/// added id it stays.
#[test]
fn a_set_for_an_unconfigured_id_is_normalised_away_from_both_members() {
    let file = json!({
        "enabled": {},
        "added": [added("d", Some("d"))],
        "inbound": {id("a").as_str(): peers_json(0..1), id("d").as_str(): peers_json(1..3),
                    id("gone").as_str(): peers_json(0..1)},
        "outbound": {id("a").as_str(): [], id("d").as_str(): [], id("gone").as_str(): []},
    });
    let (_dir, path) = state();
    put(&path, &file);
    let (overlay, changes) = load(&path, &config()).expect("loads");
    assert_eq!(changes, vec![Normalised::UnknownSetDropped(id("gone"))]);
    assert!(overlay.own_sets(&id("gone")).is_none());
    assert_eq!(
        overlay.own_sets(&id("d")).expect("added id kept").0.len(),
        2
    );
    assert!(overlay.own_sets(&id("a")).is_some());
    let on_disk = json_of(&path);
    for member in ["inbound", "outbound"] {
        let keys: Vec<&String> = on_disk[member]
            .as_object()
            .expect("object")
            .keys()
            .collect();
        assert_eq!(keys.len(), 2, "{member}: {keys:?}");
        assert!(!keys.iter().any(|k| **k == id("gone").as_str()), "{member}");
    }
    // The control: the same file without the unknown id reports nothing.
    let (_overlay, changes) = loads(
        &json!({"enabled": {}, "added": [added("d", Some("d"))],
                "inbound": {id("d").as_str(): []}, "outbound": {id("d").as_str(): []}}),
        &config(),
    )
    .expect("loads");
    assert!(changes.is_empty(), "{changes:?}");
}

/// Each list at most `MAX_ALLOWED_PEERS`: a list of that many loads, one
/// more is refused naming the member.
#[test]
fn a_list_past_the_per_list_bound_is_refused() {
    let max = PeerTrustPolicy::MAX_ALLOWED_PEERS as u64;
    let file = |inbound: u64, outbound: u64| {
        json!({"enabled": {},
               "inbound": {id("a").as_str(): peers_json(0..inbound)},
               "outbound": {id("a").as_str(): peers_json(0..outbound)}})
    };
    let (overlay, _) = loads(&file(max, max), &config()).expect("at the bound: the control");
    assert_eq!(overlay.set_rows(), 2 * PeerTrustPolicy::MAX_ALLOWED_PEERS);
    for (inbound, outbound, member) in [
        (max + 1, 0, OverlayMember::Inbound),
        (0, max + 1, OverlayMember::Outbound),
    ] {
        match loads(&file(inbound, outbound), &config()) {
            Err(EndpointOverlayError::SetTooLarge {
                member: got,
                endpoint,
                got: n,
            }) => {
                assert_eq!((got, endpoint, n as u64), (member, id("a"), max + 1));
            }
            other => panic!("{member}: expected SetTooLarge, got {other:?}"),
        }
    }
}

/// All rows together, both members counted, at most 16384: exactly that
/// many loads, one more is refused although no list is past its own bound.
#[test]
fn the_row_bound_holds_at_exactly_its_limit() {
    let max = PeerTrustPolicy::MAX_ALLOWED_PEERS as u64;
    assert_eq!(MAX_ENDPOINT_SET_ROWS, 16_384);
    // a and b hold two full lists each: 4 * 4096 rows. `c` holds the
    // extra row in a list of its own, `[]` in the other.
    let file = |extra: u64| {
        json!({"enabled": {},
            "inbound": {id("a").as_str(): peers_json(0..max), id("b").as_str(): peers_json(0..max),
                        id("c").as_str(): peers_json(0..extra)},
            "outbound": {id("a").as_str(): peers_json(0..max), id("b").as_str(): peers_json(0..max),
                         id("c").as_str(): []}})
    };
    let (overlay, changes) = loads(&file(0), &config()).expect("exactly 16384: the control");
    assert!(changes.is_empty(), "{changes:?}");
    assert_eq!(overlay.set_rows(), MAX_ENDPOINT_SET_ROWS);
    assert!(matches!(
        loads(&file(1), &config()),
        Err(EndpointOverlayError::SetRowsExceeded { got: 16_385 })
    ));
}

/// A peer named twice in one list is refused, not collapsed; the same
/// peer in the two lists, or in two endpoints', is allowed.
#[test]
fn a_peer_named_twice_in_one_list_is_refused() {
    let p = peer(1);
    let file = |inbound: Vec<&str>| {
        json!({"enabled": {},
               "inbound": {id("a").as_str(): inbound, id("b").as_str(): [p.as_str()]},
               "outbound": {id("a").as_str(): [p.as_str()], id("b").as_str(): []}})
    };
    loads(&file(vec![p.as_str()]), &config()).expect("a peer in several lists: the control");
    match loads(&file(vec![p.as_str(), p.as_str()]), &config()) {
        Err(EndpointOverlayError::DuplicatePeer {
            member,
            endpoint,
            peer,
        }) => {
            assert_eq!(
                (member, endpoint, peer),
                (OverlayMember::Inbound, id("a"), p)
            );
        }
        other => panic!("expected DuplicatePeer, got {other:?}"),
    }
}

/// A peer that is not canonical, and an endpoint key that is not a
/// derived id, are parse refusals.
#[test]
fn a_set_naming_garbage_is_a_parse_refusal() {
    for (what, file) in [
        (
            "peer",
            json!({"enabled": {}, "inbound": {id("a").as_str(): ["nonsense"]},
                         "outbound": {id("a").as_str(): []}}),
        ),
        (
            "id",
            json!({"enabled": {}, "inbound": {"a": []}, "outbound": {"a": []}}),
        ),
    ] {
        assert!(
            matches!(loads(&file, &config()), Err(EndpointOverlayError::Parse(_))),
            "{what}"
        );
    }
}

// ---- added ---------------------------------------------------------

/// An added entry carrying `inbound` or `outbound` is refused, naming the
/// member; the same entry without loads and composes.
#[test]
fn an_added_entry_never_carries_trust() {
    let (overlay, changes) = loads(
        &with_empty_sets(
            json!({"enabled": {}, "added": [added("d", Some("d"))]}),
            &["d"],
        ),
        &config(),
    )
    .expect("the control loads");
    assert!(changes.is_empty(), "{changes:?}");
    assert_eq!(overlay.added().count(), 1);
    for (member, name) in [
        ("inbound", OverlayMember::Inbound),
        ("outbound", OverlayMember::Outbound),
    ] {
        let mut entry = added("d", Some("d"));
        entry[member] = json!("inherit_profile_trust");
        match loads(
            &with_empty_sets(json!({"enabled": {}, "added": [entry]}), &["d"]),
            &config(),
        ) {
            Err(EndpointOverlayError::AddedEntryCarriesTrust { member: got }) => {
                assert_eq!(got, name);
            }
            other => panic!("{member}: expected AddedEntryCarriesTrust, got {other:?}"),
        }
    }
}

/// An added entry whose id stands in neither `inbound` nor `outbound` is
/// refused at load, naming the id: an added endpoint's trust is its set
/// alone, `[]` at the add, and the file states it. The same file with
/// `[]` in both loads and `d` has an own, empty set.
#[test]
fn an_added_entry_without_a_set_is_refused_naming_the_id() {
    let file = json!({"enabled": {}, "added": [added("d", Some("d"))]});
    match loads(&file, &config()) {
        Err(e @ EndpointOverlayError::AddedEntryWithoutSet { .. }) => {
            let EndpointOverlayError::AddedEntryWithoutSet { endpoint } = &e else {
                unreachable!()
            };
            assert_eq!(endpoint, &id("d"));
            let text = e.to_string();
            assert!(text.contains(id("d").as_str()), "{text}");
            assert!(
                text.contains("`inbound`") && text.contains("`outbound`"),
                "{text}"
            );
        }
        other => panic!("expected AddedEntryWithoutSet, got {other:?}"),
    }
    // Another added entry's set is not this one's.
    assert!(matches!(
        loads(
            &with_empty_sets(
                json!({"enabled": {}, "added": [added("d", Some("d")), added("e", Some("e"))]}),
                &["e"],
            ),
            &config(),
        ),
        Err(EndpointOverlayError::AddedEntryWithoutSet { endpoint }) if endpoint == id("d")
    ));
    // An authored endpoint's absent id is still "no own set".
    loads(&json!({"enabled": {}}), &config()).expect("authored, no set: the control");

    let (overlay, changes) =
        loads(&with_empty_sets(file, &["d"]), &config()).expect("with [] in both: the control");
    assert!(changes.is_empty(), "{changes:?}");
    let (inbound, outbound) = overlay.own_sets(&id("d")).expect("an own set");
    assert_eq!((inbound.len(), outbound.len()), (0, 0));
}

/// An added entry is read as an authored one is: a retired `id` and a
/// key that is not `<EndpointId>.key` are refused.
#[test]
fn an_added_entry_is_judged_as_an_authored_one_is() {
    for (what, entry) in [
        (
            "retired id",
            json!({"id": id("d").as_str(), "enabled": true, "advertise": false}),
        ),
        ("no key", json!({"enabled": true, "advertise": false})),
        (
            "a key that is not an id",
            json!({"key": "d.key", "enabled": true, "advertise": false}),
        ),
        (
            "no enabled",
            json!({"key": key_of("d"), "advertise": false}),
        ),
    ] {
        assert!(
            matches!(
                loads(&json!({"enabled": {}, "added": [entry]}), &config()),
                Err(EndpointOverlayError::Parse(_))
            ),
            "{what}"
        );
    }
}

/// 64 endpoints, authored and added together, load; 65 are refused by
/// name, never truncated.
#[test]
fn authored_and_added_together_are_at_most_sixty_four() {
    let names: Vec<String> = (0..40).map(|i| format!("e{i}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let config = config_of(&refs);
    let file = |n: usize| {
        let entries: Vec<Value> = (0..n)
            .map(|i| added(&format!("x{i}"), Some(&format!("x{i}"))))
            .collect();
        let names: Vec<String> = (0..n).map(|i| format!("x{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        with_empty_sets(json!({"enabled": {}, "added": entries}), &names)
    };
    let (overlay, _) = loads(&file(MAX_ENDPOINTS - 40), &config).expect("64 together: the control");
    assert_eq!(overlay.apply(&config).entries.len(), MAX_ENDPOINTS);
    assert!(matches!(
        loads(&file(MAX_ENDPOINTS - 40 + 1), &config),
        Err(EndpointOverlayError::TooManyEndpoints { got: 65 })
    ));
}

/// An added id or alias that an authored or another added entry holds is
/// refused by name; each beside the entry that differs and loads.
#[test]
fn an_added_id_or_alias_that_collides_is_refused() {
    let ok = with_empty_sets(
        json!({"enabled": {}, "added": [added("d", Some("d")), added("e", Some("e"))]}),
        &["d", "e"],
    );
    loads(&ok, &config()).expect("distinct ids and aliases: the control");

    let id_of_authored = with_empty_sets(
        json!({"enabled": {}, "added": [added("a", Some("fresh"))]}),
        &["a"],
    );
    assert!(matches!(
        loads(&id_of_authored, &config()),
        Err(EndpointOverlayError::AddedIdCollides(i)) if i == id("a")
    ));
    let id_of_added = with_empty_sets(
        json!({"enabled": {}, "added": [added("d", Some("d")), added("d", Some("e"))]}),
        &["d"],
    );
    assert!(matches!(
        loads(&id_of_added, &config()),
        Err(EndpointOverlayError::AddedIdCollides(i)) if i == id("d")
    ));
    let alias_of_authored = with_empty_sets(
        json!({"enabled": {}, "added": [added("d", Some("a"))]}),
        &["d"],
    );
    assert!(matches!(
        loads(&alias_of_authored, &config()),
        Err(EndpointOverlayError::AddedAliasCollides(a)) if a.as_str() == "a"
    ));
    let alias_of_added = with_empty_sets(
        json!({"enabled": {}, "added": [added("d", Some("x")), added("e", Some("x"))]}),
        &["d", "e"],
    );
    assert!(matches!(
        loads(&alias_of_added, &config()),
        Err(EndpointOverlayError::AddedAliasCollides(a)) if a.as_str() == "x"
    ));
}

/// The added entries are composed after the authored ones, `enabled` and
/// `advertise` as written; the overlay's `enabled` delta applies to them.
#[test]
fn apply_composes_the_added_entries() {
    let mut d = added("d", Some("d"));
    d["advertise"] = json!(true);
    d["allowed_client_kinds"] = json!(["human"]);
    let (overlay, _) = loads(
        &with_empty_sets(json!({"enabled": {}, "added": [d]}), &["d"]),
        &config(),
    )
    .expect("loads");
    let composed = overlay.apply(&config());
    let ids: Vec<&EndpointId> = composed.entries.iter().map(|e| &e.id).collect();
    assert_eq!(ids, vec![&id("a"), &id("b"), &id("c"), &id("d")]);
    let entry: &EndpointConfig = &composed.entries[3];
    assert!(entry.enabled && entry.advertise);
    assert_eq!(
        entry.allowed_client_kinds,
        vec![ClientKind::parse("human").expect("kind")]
    );
    assert_eq!(entry.key_state, EndpointKeyState::NotRead);
    assert!(overlay.effective(&config()).enabled[&id("d")]);

    // `set_enabled` reaches an added endpoint and composes as a delta.
    let off = overlay
        .set_enabled(&config(), &id("d"), false)
        .expect("known")
        .expect("a change");
    assert!(!off.apply(&config()).entries[3].enabled);
    // An added endpoint can be the default, and it is judged enabled.
    let with_default = overlay
        .set_default(&config(), Some(&id("d")))
        .expect("known")
        .expect("a change");
    assert_eq!(
        with_default.apply(&config()).default_direct_endpoint,
        Some(id("d"))
    );
    assert!(matches!(
        off.set_default(&config(), Some(&id("d"))),
        Err(EndpointOverlayError::Disabled)
    ));
}

/// A loaded added entry's own filters admit nobody in either direction,
/// a peer of the operator's list included, so a runtime that holds it
/// without its own set (an overlay left ahead of it) fails closed:
/// `ADDED_TRUST`, never the inheritance the entry reader gives an absent
/// member. The control: an authored entry with no member beside it
/// admits that peer.
#[test]
fn a_loaded_added_entrys_filters_admit_nobody() {
    let (overlay, _) = loads(
        &with_empty_sets(
            json!({"enabled": {}, "added": [added("d", Some("d"))]}),
            &["d"],
        ),
        &config(),
    )
    .expect("loads");
    let composed = overlay.apply(&config());
    let operator = PeerTrustPolicy::new([peer(1)]).expect("bounded");
    let entry_of = |name: &str| {
        composed
            .entries
            .iter()
            .find(|e| e.id == id(name))
            .expect("composed")
    };
    let d = entry_of("d");
    assert_eq!(d.inbound, ADDED_TRUST);
    assert_eq!(d.outbound, ADDED_TRUST);
    for policy in [&d.inbound, &d.outbound] {
        assert!(!operator.decide_for_endpoint(&peer(1), policy).is_allowed());
    }
    let a = entry_of("a");
    assert!(
        operator
            .decide_for_endpoint(&peer(1), &a.inbound)
            .is_allowed(),
        "control: an authored entry with no member inherits the list"
    );
}

/// A written overlay reads back as itself with its added entries and
/// kinds, and carries no `inbound` or `outbound` on an added entry.
#[test]
fn an_added_entry_round_trips_without_trust_members() {
    let mut d = added("d", Some("d"));
    d["allowed_client_kinds"] = json!(["human", "claude"]);
    let (_dir, path) = state();
    put(
        &path,
        &with_empty_sets(json!({"enabled": {}, "added": [d.clone()]}), &["d"]),
    );
    let (overlay, _) = load(&path, &config()).expect("loads");
    overlay
        .write_within(&path, &TrustBoundary::root())
        .expect("written");
    assert_eq!(json_of(&path)["added"], json!([d]));
    let (back, changes) = load(&path, &config()).expect("loads");
    assert!(changes.is_empty(), "{changes:?}");
    assert_eq!(back, overlay);
}

/// The key state of an added entry is read as the loader reads an
/// authored entry's: a minted key is present, none is missing, and the
/// composed entry carries it.
#[test]
fn added_entries_get_the_loaders_key_states() {
    let dir = private_tempdir();
    let roots = XdgRoots {
        config_home: dir.path().join("config"),
        data_home: dir.path().join("data"),
        state_home: dir.path().join("state"),
        cache_home: dir.path().join("cache"),
        runtime_dir: None,
    };
    let paths = ProfilePaths::resolve_offline("work", &roots).expect("paths");
    let minted = endpoint_key::mint_filesystem(&paths).expect("minted");
    let with_key =
        json!({"key": endpoint_key::key_file_name(&minted), "enabled": true, "advertise": false});
    let (mut overlay, _) = loads(
        &json!({"enabled": {}, "added": [with_key, added("d", Some("d"))],
                "inbound": {minted.as_str(): [], id("d").as_str(): []},
                "outbound": {minted.as_str(): [], id("d").as_str(): []}}),
        &config(),
    )
    .expect("loads");
    assert!(
        overlay
            .added()
            .all(|e| e.key_state == EndpointKeyState::NotRead)
    );
    assert!(overlay.read_added_keys(&paths).is_empty());
    let composed = overlay.apply(&config());
    let state_of = |i: &EndpointId| {
        composed
            .entries
            .iter()
            .find(|e| &e.id == i)
            .expect("composed")
            .key_state
    };
    assert!(matches!(
        state_of(&minted),
        EndpointKeyState::Present { .. }
    ));
    assert_eq!(
        state_of(&id("d")),
        EndpointKeyState::Missing(KeyMissing::Absent)
    );
}

/// The client resolver finds an added alias through the composed entries,
/// and refuses an alias nothing carries.
#[test]
fn an_added_alias_resolves_and_an_unknown_one_is_refused() {
    let (overlay, _) = loads(
        &with_empty_sets(
            json!({"enabled": {}, "added": [added("d", Some("fabric"))]}),
            &["d"],
        ),
        &config(),
    )
    .expect("loads");
    assert_eq!(resolve_endpoint(&config(), &overlay, "fabric"), Ok(id("d")));
    assert_eq!(
        resolve_endpoint(&config(), &overlay, id("d").as_str()),
        Ok(id("d"))
    );
    assert_eq!(resolve_endpoint(&config(), &overlay, "a"), Ok(id("a")));
    assert!(matches!(
        resolve_endpoint(&config(), &overlay, "nowhere"),
        Err(ResolveEndpointError::UnknownAlias(a)) if a.as_str() == "nowhere"
    ));
    // Without the overlay's entry the same alias is unknown.
    assert!(matches!(
        resolve_endpoint(&config(), &EndpointOverlay::default(), "fabric"),
        Err(ResolveEndpointError::UnknownAlias(_))
    ));
}

// ---- advertise -----------------------------------------------------

/// A delta for an unauthored id (unknown, or an added entry) or equal to
/// the authored value is dropped, one differing is kept and applied.
#[test]
fn an_advertise_delta_is_normalised_against_the_authored_entries() {
    let file = json!({
        "enabled": {},
        "added": [added("d", Some("d"))],
        "inbound": empty_sets(&["d"]),
        "outbound": empty_sets(&["d"]),
        "advertise": {id("a").as_str(): true, id("b").as_str(): false,
                      id("gone").as_str(): true, id("d").as_str(): true},
    });
    let (_dir, path) = state();
    put(&path, &file);
    let (overlay, changes) = load(&path, &config()).expect("loads");
    let mut want = vec![
        Normalised::AdvertiseEqualsConfig(id("b")),
        Normalised::UnknownAdvertiseDropped(id("d")),
        Normalised::UnknownAdvertiseDropped(id("gone")),
    ];
    let mut got = changes;
    want.sort_by_key(|c| format!("{c:?}"));
    got.sort_by_key(|c| format!("{c:?}"));
    assert_eq!(got, want);
    assert_eq!(json_of(&path)["advertise"], json!({id("a").as_str(): true}));
    let composed = overlay.apply(&config());
    let advertised: Vec<bool> = composed.entries.iter().map(|e| e.advertise).collect();
    assert_eq!(
        advertised,
        vec![true, false, false, false],
        "a only; d keeps its own"
    );

    // The control: a differing delta alone reports nothing and is not rewritten.
    let (_overlay, changes) = loads(
        &json!({"enabled": {}, "advertise": {id("a").as_str(): true}}),
        &config(),
    )
    .expect("loads");
    assert!(changes.is_empty(), "{changes:?}");
}

// ---- the write API -------------------------------------------------

/// The first write seeds each direction from what the caller computed and
/// then applies the change to both members; later writes change the
/// sets alone.
#[test]
fn the_first_write_seeds_per_direction_then_applies_to_both() {
    let base = EndpointOverlay::default();
    let (seed_in, seed_out) = (vec![peer(1), peer(2)], vec![peer(2), peer(3)]);
    let next = base
        .set_peer(
            &config(),
            &id("a"),
            seed_in.clone(),
            seed_out.clone(),
            &peer(9),
            true,
        )
        .expect("written")
        .expect("a change");
    let (inbound, outbound) = next.own_sets(&id("a")).expect("created");
    assert_eq!(
        inbound.peers().iter().cloned().collect::<Vec<_>>(),
        vec![peer(1), peer(2), peer(9)]
    );
    assert_eq!(
        outbound.peers().iter().cloned().collect::<Vec<_>>(),
        vec![peer(2), peer(3), peer(9)]
    );
    assert!(
        next.own_sets(&id("b")).is_none(),
        "no other endpoint gets a set"
    );

    // Removing applies to both members and keeps the set, empty if need be.
    let removed = next
        .set_peer(&config(), &id("a"), [], [], &peer(2), false)
        .expect("written")
        .expect("a change");
    let (inbound, outbound) = removed.own_sets(&id("a")).expect("still own");
    assert_eq!(inbound.len(), 2);
    assert_eq!(outbound.len(), 2);
    assert!(!inbound.peers().contains(&peer(2)) && !outbound.peers().contains(&peer(2)));
    // The seed of a present set is ignored, and a no-op is `None`.
    assert_eq!(
        next.set_peer(&config(), &id("a"), [peer(50)], [peer(51)], &peer(9), true)
            .expect("written"),
        None
    );
    // Removing from a fresh endpoint creates its set (a change) with the peer left out.
    let fresh = base
        .set_peer(&config(), &id("b"), [peer(1)], [], &peer(1), false)
        .expect("written")
        .expect("creation is a change");
    let (inbound, outbound) = fresh.own_sets(&id("b")).expect("created");
    assert!(inbound.is_empty() && outbound.is_empty());
    // It is written and read back, own and empty.
    let (_dir, path) = state();
    fresh
        .write_within(&path, &TrustBoundary::root())
        .expect("written");
    let (back, _) = load(&path, &config()).expect("loads");
    assert_eq!(back, fresh);
    assert_eq!(back.every_set().count(), 2);
}

/// A seed creates the set and says it was not there; on a present set,
/// an empty one included, nothing changes.
#[test]
fn a_seed_on_a_present_set_changes_nothing() {
    let base = EndpointOverlay::default();
    let created = match base
        .seed_sets(&config(), &id("a"), [peer(1)], [])
        .expect("seeded")
    {
        Seeded::Created(o) => o,
        Seeded::AlreadyPresent => panic!("a fresh endpoint has no set"),
    };
    assert_eq!(created.own_sets(&id("a")).expect("created").0.len(), 1);
    assert_eq!(
        created
            .seed_sets(&config(), &id("a"), [peer(7)], [peer(8)])
            .expect("seeded"),
        Seeded::AlreadyPresent
    );
    // Own and empty is present: a seed never refills it.
    let empty = match base.seed_sets(&config(), &id("b"), [], []).expect("seeded") {
        Seeded::Created(o) => o,
        Seeded::AlreadyPresent => panic!("created"),
    };
    assert_eq!(
        empty
            .seed_sets(&config(), &id("b"), [peer(7)], [peer(8)])
            .expect("seeded"),
        Seeded::AlreadyPresent
    );
    assert!(empty.own_sets(&id("b")).expect("present").0.is_empty());
    // An endpoint nothing configures cannot be given a set.
    assert!(matches!(
        base.seed_sets(&config(), &id("gone"), [], []),
        Err(EndpointOverlayError::Unknown)
    ));
    assert!(matches!(
        base.set_peer(&config(), &id("gone"), [], [], &peer(1), true),
        Err(EndpointOverlayError::Unknown)
    ));
}

/// A write or a seed that would cross the row bound is refused and the
/// overlay it was asked of is as it was; the seed that reaches the bound
/// exactly is the control.
#[test]
fn a_write_crossing_the_row_bound_is_refused_and_changes_nothing() {
    let max = PeerTrustPolicy::MAX_ALLOWED_PEERS as u64;
    // `a` and `b` hold two lists each; `b`'s outbound is `short` peers
    // under full. 16384 rows at `short = 0`.
    let full = |short: u64| -> Value {
        json!({"enabled": {},
            "inbound": {id("a").as_str(): peers_json(0..max), id("b").as_str(): peers_json(0..max)},
            "outbound": {id("a").as_str(): peers_json(0..max),
                         id("b").as_str(): peers_json(0..max - short)}})
    };
    let (almost, _) = loads(&full(1), &config()).expect("loads");
    assert_eq!(almost.set_rows(), MAX_ENDPOINT_SET_ROWS - 1);
    let (exact, _) = loads(&full(0), &config()).expect("loads");
    assert_eq!(exact.set_rows(), MAX_ENDPOINT_SET_ROWS);

    // The control: one row more than `almost` holds reaches the bound.
    match almost
        .seed_sets(&config(), &id("c"), [peer(1)], [])
        .expect("reaches the bound")
    {
        Seeded::Created(o) => assert_eq!(o.set_rows(), MAX_ENDPOINT_SET_ROWS),
        Seeded::AlreadyPresent => panic!("c has no set"),
    }
    // One row past it is refused, by a seed and by a write, each naming
    // the rows the result would hold.
    assert!(matches!(
        exact.seed_sets(&config(), &id("c"), [peer(1)], []),
        Err(EndpointOverlayError::SetRowsExceeded { got }) if got == MAX_ENDPOINT_SET_ROWS + 1
    ));
    assert!(matches!(
        exact.set_peer(&config(), &id("c"), [], [], &peer(5000), true),
        Err(EndpointOverlayError::SetRowsExceeded { got }) if got == MAX_ENDPOINT_SET_ROWS + 2
    ));
    // A write into an existing set crosses as well: one peer is a row in
    // each member.
    let Seeded::Created(with_c) = almost
        .seed_sets(&config(), &id("c"), [], [])
        .expect("an empty seed adds no row")
    else {
        panic!("c has no set");
    };
    assert!(matches!(
        with_c.set_peer(&config(), &id("c"), [], [], &peer(5000), true),
        Err(EndpointOverlayError::SetRowsExceeded { got }) if got == MAX_ENDPOINT_SET_ROWS + 1
    ));
    assert_eq!(
        exact.set_rows(),
        MAX_ENDPOINT_SET_ROWS,
        "the overlay asked is as it was"
    );
    assert!(exact.own_sets(&id("c")).is_none());
    // A removal on a full overlay is allowed: it only lowers the rows.
    let lowered = exact
        .set_peer(&config(), &id("a"), [], [], &peer(0), false)
        .expect("allowed")
        .expect("a change");
    assert_eq!(lowered.set_rows(), MAX_ENDPOINT_SET_ROWS - 2);
}

/// A list at its per-list bound refuses one more peer.
#[test]
fn a_write_past_the_per_list_bound_is_refused() {
    let max = PeerTrustPolicy::MAX_ALLOWED_PEERS as u64;
    let seed: Vec<TransportIdentity> = (0..max).map(peer).collect();
    let full = EndpointOverlay::default()
        .set_peer(&config(), &id("a"), seed.clone(), [], &peer(0), true)
        .expect("a full list: the control")
        .expect("created");
    assert_eq!(
        full.own_sets(&id("a")).expect("set").0.len(),
        PeerTrustPolicy::MAX_ALLOWED_PEERS
    );
    assert!(matches!(
        full.set_peer(&config(), &id("a"), [], [], &peer(max), true),
        Err(EndpointOverlayError::SetTooLarge {
            member: OverlayMember::Inbound,
            ..
        })
    ));
    let too_long: Vec<TransportIdentity> = (0..=max).map(peer).collect();
    assert!(matches!(
        EndpointOverlay::default().seed_sets(&config(), &id("a"), too_long, []),
        Err(EndpointOverlayError::SetTooLarge {
            member: OverlayMember::Inbound,
            ..
        })
    ));
}

/// The overlay's other writers leave the new members as they were.
#[test]
fn set_enabled_and_default_keep_the_other_members() {
    let (overlay, _) = loads(
        &json!({"enabled": {}, "added": [added("d", Some("d"))],
                "inbound": {id("a").as_str(): peers_json(0..2), id("d").as_str(): []},
                "outbound": {id("a").as_str(): [], id("d").as_str(): []},
                "advertise": {id("a").as_str(): true}}),
        &config(),
    )
    .expect("loads");
    let next = overlay
        .set_enabled(&config(), &id("b"), false)
        .expect("known")
        .expect("a change");
    assert_eq!(next.own_sets(&id("a")), overlay.own_sets(&id("a")));
    assert_eq!(next.added().count(), 1);
    assert!(!next.apply(&config()).entries[1].enabled);
    assert!(next.apply(&config()).entries[0].advertise);
}

/// The pre-Stage-22 carry reads the file raw: the new members it does not
/// judge are written back as they stood.
#[test]
fn the_carry_keeps_the_new_members() {
    let (_dir, path) = state();
    let file = json!({
        "enabled": {"claude": false},
        "inbound": {id("a").as_str(): peers_json(0..2), id("d").as_str(): []},
        "outbound": {id("a").as_str(): [], id("d").as_str(): []},
        "added": [added("d", Some("d"))],
        "advertise": {id("a").as_str(): true},
    });
    put(&path, &file);
    let label = interweave_profile_config::EndpointAlias::parse("claude").expect("alias");
    let moved =
        EndpointOverlay::carry_label_within(&path, &TrustBoundary::root(), &label, &id("b"))
            .expect("carried");
    assert_eq!(
        moved,
        interweave_profile_config::endpoint_overlay::Carried::Moved
    );
    let mut want = file;
    want["enabled"] = json!({id("b").as_str(): false});
    assert_eq!(json_of(&path), want);
}

// ---- the size bound ------------------------------------------------

/// The largest legal overlay -- 64 added endpoints, each with the
/// longest alias and sixteen client kinds of 64 control characters (six
/// bytes each as JSON), all of them enabled-flipped, and 16384 set rows
/// of the longest `PeerId` spread over every id -- is written as
/// `write_within` writes it, fits `MAX_ENDPOINT_OVERLAY_BYTES`, and the
/// bound is not much larger than it.
#[test]
fn the_largest_legal_overlay_fits_its_bound_and_the_bound_is_not_loose() {
    let entries: Vec<Value> = (0..MAX_ENDPOINTS)
        .map(|i| {
            let kinds: Vec<String> = [
                1_u8, 2, 3, 4, 5, 6, 7, 0x0b, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15,
            ]
            .iter()
            .map(|c| format!("{}{}", "\u{1}".repeat(63), char::from(*c)))
            .collect();
            let mut e = added(&format!("x{i}"), None);
            e["alias"] = json!(format!("{:z<64}", format!("x{i}")));
            e["allowed_client_kinds"] = json!(kinds);
            e
        })
        .collect();
    let per_list = MAX_ENDPOINT_SET_ROWS / (2 * MAX_ENDPOINTS);
    let sets: serde_json::Map<String, Value> = (0..MAX_ENDPOINTS)
        .map(|i| {
            let from = 1000 * i as u64;
            (
                id(&format!("x{i}")).as_str().to_owned(),
                json!(peers_json(from..from + per_list as u64)),
            )
        })
        .collect();
    let (_dir, path) = state();
    put(
        &path,
        &json!({"enabled": {}, "added": entries, "inbound": sets.clone(), "outbound": sets}),
    );
    let config = config_of(&[]);
    let (mut overlay, changes) = load(&path, &config).expect("64 added endpoints load");
    assert!(changes.is_empty(), "{changes:?}");
    assert_eq!(overlay.added().count(), MAX_ENDPOINTS);

    for i in 0..MAX_ENDPOINTS {
        let ids: Vec<EndpointId> = overlay.added().map(|e| e.id.clone()).collect();
        let endpoint = &ids[i];
        overlay = overlay
            .set_enabled(&config, endpoint, false)
            .expect("known")
            .expect("a change");
    }
    assert_eq!(overlay.set_rows(), MAX_ENDPOINT_SET_ROWS);
    overlay
        .write_within(&path, &TrustBoundary::root())
        .expect("written");
    let size = std::fs::metadata(&path).expect("meta").len();
    assert!(
        size <= MAX_ENDPOINT_OVERLAY_BYTES,
        "{size} bytes over the bound"
    );
    assert!(
        MAX_ENDPOINT_OVERLAY_BYTES < 2 * size,
        "the bound {MAX_ENDPOINT_OVERLAY_BYTES} is over twice the largest file, {size}"
    );
    let (back, changes) = load(&path, &config).expect("the largest file loads as itself");
    assert_eq!(back, overlay);
    assert!(changes.is_empty(), "{changes:?}");
    eprintln!("largest overlay: {size} bytes, bound {MAX_ENDPOINT_OVERLAY_BYTES}");
}
