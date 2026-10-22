// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A `HumanChatV2` contact card at the bridge (CHANNEL-EVENT.md §Content's one
//! parsing exception, ADR-0059 decision 7), fed the frozen contact-card
//! vectors (`fixtures/human-chat-v2/human-chat-v2-contact-card.json`) as a
//! session event, the way the server feeds every other one.
//!
//! A valid direct card becomes `meta` alone, with `content` empty and
//! never the photo. A malformed card, or any card over broadcast, becomes
//! the reason it is counted under, and mints no reply token. Everything
//! else stays unparsed: a `text` envelope, an envelope of another kind,
//! or a card's JSON under any other media type.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use interweave_claude_channel_core::{BridgeState, CardDrop, ConvertError};
use interweave_human_chat_protocol::{
    Card, CardRoute, Classified, MAX_CARD_PHOTO_BYTES, MEDIA_TYPE_V2, classify,
};
use interweave_local_client_api::{Generation, ReceivedBroadcast, ReceivedDirect, SessionEvent};
use interweave_transport_api::{
    ChannelId, EndpointId, MAX_PAYLOAD_BYTES, MediaType, MessageId, Payload, TransportIdentity,
};
use serde_json::Value;

const NOW: u64 = 1_791_227_222_497;

fn vectors() -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/human-chat-v2/human-chat-v2-contact-card.json");
    let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the frozen vectors"))
            .expect("JSON");
    doc["vectors"].as_array().expect("vectors").clone()
}

fn vector(name: &str) -> Value {
    vectors()
        .into_iter()
        .find(|v| v["name"] == name)
        .unwrap_or_else(|| panic!("no vector {name}"))
}

/// The envelope as the sender put it on the wire.
fn wire(v: &Value) -> String {
    serde_json::to_string(&v["envelope"]).expect("serializes")
}

fn peer(v: &Value) -> TransportIdentity {
    TransportIdentity::parse(v["received_on"]["peer_id"].as_str().expect("a peer")).expect("peer")
}

fn source_endpoint(v: &Value) -> EndpointId {
    EndpointId::parse(
        v["received_on"]["source_endpoint"]
            .as_str()
            .expect("an endpoint"),
    )
    .expect("endpoint")
}

fn payload(media_type: &str, bytes: Vec<u8>) -> Payload {
    Payload::new(
        Some(MediaType::parse(media_type).expect("a media type")),
        bytes,
        MAX_PAYLOAD_BYTES,
    )
    .expect("a payload")
}

fn brotli(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut out = Vec::new();
    {
        let mut w = brotli::CompressorWriter::new(&mut out, 4096, 11, 22);
        w.write_all(bytes).expect("compresses");
    }
    out
}

fn direct(v: &Value, payload: Payload) -> SessionEvent {
    SessionEvent::Direct(ReceivedDirect {
        source_peer: peer(v),
        source_endpoint: source_endpoint(v),
        destination_endpoint: EndpointId::parse("claude").expect("endpoint"),
        message_id: MessageId::parse_hex("000102030405060708090a0b0c0d0e0f").expect("id"),
        payload,
        received_at_ms: NOW,
    })
}

fn broadcast(v: &Value, payload: Payload) -> SessionEvent {
    SessionEvent::Broadcast(ReceivedBroadcast {
        source_peer: peer(v),
        channel: ChannelId::parse("general").expect("channel"),
        message_id: MessageId::parse_hex("000102030405060708090a0b0c0d0e0f").expect("id"),
        payload,
        received_at_ms: NOW,
    })
}

fn leased() -> BridgeState {
    let mut bridge = BridgeState::new();
    bridge.leased(
        EndpointId::parse("claude").expect("endpoint"),
        Generation::parse("AAAAAAAAAAAAAAAAAAAAAQ").expect("epoch"),
    );
    bridge
}

/// What the parser itself makes of the vector: the values `meta` must hold.
fn parsed(v: &Value) -> Card {
    match classify(
        &wire(v),
        CardRoute {
            source_peer: &peer(v),
            source_endpoint: &source_endpoint(v),
        },
    ) {
        Classified::Card(card) => card,
        other => panic!("{}: {other:?}", v["name"]),
    }
}

/// The notification as the host reads it: one JSON line, `meta` in order.
fn line(bridge: &mut BridgeState, event: &SessionEvent) -> (String, Value) {
    let n = bridge
        .notification(event, [7; 16], NOW)
        .expect("forwarded")
        .expect("a message event");
    // Built as text: a `Value` would sort `meta`'s keys.
    let ordered = format!(
        "{{\"content\":{},\"meta\":{}}}",
        serde_json::to_string(&n.content).expect("content"),
        serde_json::to_string(&n.meta).expect("meta")
    );
    let value = serde_json::from_str(&ordered).expect("JSON");
    (ordered, value)
}

/// No run of eight or more characters of `needle` -- the photo's base64,
/// or its decoded bytes read as text -- is in `haystack`.
fn carries_none_of(haystack: &str, needle: &str) -> bool {
    let chars: Vec<char> = needle.chars().collect();
    chars
        .windows(8)
        .all(|w| !haystack.contains(&w.iter().collect::<String>()))
}

/// A card at the photo bound reaches the model as what it is: the kind,
/// the authenticated source peer and its endpoint, the name and surname
/// as the parser gave them, the photo's size and type, and the identity
/// key as received -- `content` empty, and nothing of the photo, base64 or
/// decoded, anywhere in the line. Plain and `;ce=br` alike, the second
/// with its `ce` removed from `content_type`.
#[test]
fn a_card_at_the_photo_bound_reaches_the_model_as_meta_alone() {
    let v = vector("valid-at-the-bounds");
    let card = parsed(&v);
    let photo = card.photo.as_ref().expect("the vector has a photo");
    assert_eq!(photo.bytes.len(), MAX_CARD_PHOTO_BYTES, "at the bound");
    let photo_b64 = v["envelope"]["card"]["photo"]["data"]
        .as_str()
        .expect("the photo's base64")
        .to_owned();
    let photo_text = String::from_utf8_lossy(&photo.bytes).into_owned();

    for (media_type, bytes) in [
        (MEDIA_TYPE_V2.to_owned(), wire(&v).into_bytes()),
        (
            format!("{MEDIA_TYPE_V2};ce=br"),
            brotli(wire(&v).as_bytes()),
        ),
    ] {
        let mut bridge = leased();
        let (ordered, n) = line(&mut bridge, &direct(&v, payload(&media_type, bytes)));
        assert_eq!(n["content"], "", "{media_type}: nothing in content");
        let meta = &n["meta"];
        assert_eq!(meta["delivery_mode"], "direct");
        assert_eq!(meta["source_peer"], v["received_on"]["peer_id"]);
        assert_eq!(meta["source_endpoint"], v["received_on"]["source_endpoint"]);
        assert_eq!(meta["content_type"], MEDIA_TYPE_V2, "{media_type}");
        assert_eq!(meta["payload_encoding"], "utf8");
        assert_eq!(meta["app_kind"], "contact_card");
        assert_eq!(meta["card_name"], card.name.as_str());
        assert_eq!(
            meta["card_surname"],
            card.surname.as_deref().expect("a surname")
        );
        assert_eq!(meta["card_photo_bytes"], MAX_CARD_PHOTO_BYTES.to_string());
        assert_eq!(meta["card_photo_type"], photo.media_type);
        assert_eq!(
            meta["card_identity_key"], v["envelope"]["card"]["identity_key"],
            "as received"
        );
        assert!(
            meta["reply_token"].is_string(),
            "the sender can be answered"
        );
        assert!(
            carries_none_of(&ordered, &photo_b64),
            "{media_type}: no run of the photo's base64"
        );
        assert!(
            carries_none_of(&ordered, &photo_text),
            "{media_type}: none of its decoded bytes"
        );
        assert!(
            !ordered.contains("signature")
                && !ordered.contains(
                    v["envelope"]["card"]["signature"]
                        .as_str()
                        .expect("a signature")
                ),
            "the signature is not forwarded"
        );
        // The card's keys follow every other key, in the table's order.
        let at = |k: &str| {
            ordered
                .find(&format!("\"{k}\":"))
                .unwrap_or_else(|| panic!("{k}"))
        };
        let order = [
            "content_type",
            "app_kind",
            "card_name",
            "card_surname",
            "card_photo_bytes",
            "card_photo_type",
            "card_identity_key",
        ];
        for pair in order.windows(2) {
            assert!(at(pair[0]) < at(pair[1]), "{pair:?} in order: {ordered}");
        }
    }
}

/// A card with neither surname nor photo: `card_photo_bytes` is `0`, and
/// the keys for what it lacks are absent, not empty.
#[test]
fn a_minimal_card_says_it_has_no_photo() {
    let v = vector("valid-minimal");
    let card = parsed(&v);
    assert!(
        card.surname.is_none() && card.photo.is_none(),
        "the vector is minimal"
    );
    let (_, n) = line(
        &mut leased(),
        &direct(&v, payload(MEDIA_TYPE_V2, wire(&v).into_bytes())),
    );
    assert_eq!(n["content"], "");
    assert_eq!(n["meta"]["card_name"], card.name.as_str());
    assert_eq!(n["meta"]["card_photo_bytes"], "0");
    for absent in ["card_surname", "card_photo_type"] {
        assert!(n["meta"].get(absent).is_none(), "{absent}");
    }
}

/// Every malformed frozen vector is dropped under the reason its fault
/// class is counted under, and mints no reply token.
#[test]
fn every_malformed_vector_is_dropped_under_its_reason_and_mints_nothing() {
    let reason = |rule: &str| match rule {
        "signature-fails" => CardDrop::Signature,
        "identity-key-length" | "identity-key-encoding" => CardDrop::IdentityKey,
        "text-present" | "reply-to-present" => CardDrop::StrayMember,
        "name" => CardDrop::Name,
        "photo-size" => CardDrop::PhotoSize,
        "photo-media-type" => CardDrop::PhotoMediaType,
        "photo-encoding" => CardDrop::PhotoEncoding,
        other => panic!("a rule this test does not place: {other}"),
    };
    let mut seen = 0;
    for v in vectors() {
        if v["valid"] == true || v["rule"] == "kind" {
            continue;
        }
        let expected = reason(v["rule"].as_str().expect("a rule"));
        let mut bridge = leased();
        let got = bridge.notification(
            &direct(&v, payload(MEDIA_TYPE_V2, wire(&v).into_bytes())),
            [7; 16],
            NOW,
        );
        assert_eq!(got, Err(ConvertError::Card(expected)), "{}", v["name"]);
        assert_eq!(bridge.live_tokens(NOW), 0, "{}: no token", v["name"]);
        for (r, count) in bridge.dropped_cards() {
            assert_eq!(
                count,
                u64::from(r == expected),
                "{}: {r:?} counted",
                v["name"]
            );
        }
        seen += 1;
    }
    assert!(seen >= 15, "the malformed vectors were all fed ({seen})");
}

/// Faults no frozen vector carries, built on a valid card's text: each is
/// decided before the signature, so the card need not be re-signed.
#[test]
fn the_other_fault_classes_are_dropped_under_theirs() {
    let v = vector("valid-minimal");
    let text = wire(&v);
    let envelope: Value = serde_json::from_str(&text).expect("JSON");
    let edit = |f: &dyn Fn(&mut Value)| {
        let mut e = envelope.clone();
        f(&mut e);
        serde_json::to_string(&e).expect("serializes")
    };
    let cases: Vec<(&str, String, CardDrop)> = vec![
        (
            "a duplicated kind, one copy a card",
            text.replacen("\"kind\":", "\"kind\":\"text\",\"kind\":", 1),
            CardDrop::DuplicateMember,
        ),
        (
            "a number no value holds beside the card",
            text.replacen('{', "{\"x\":1e400,", 1),
            CardDrop::Unreadable,
        ),
        (
            "no card",
            edit(&|e| {
                e.as_object_mut().expect("object").remove("card");
            }),
            CardDrop::MissingField,
        ),
        ("v 3", edit(&|e| e["v"] = 3.into()), CardDrop::Envelope),
        (
            "a bad app_message_id",
            edit(&|e| e["app_message_id"] = "nothex".into()),
            CardDrop::Envelope,
        ),
    ];
    for (what, text, expected) in cases {
        let mut bridge = leased();
        let got = bridge.notification(
            &direct(&v, payload(MEDIA_TYPE_V2, text.into_bytes())),
            [7; 16],
            NOW,
        );
        assert_eq!(got, Err(ConvertError::Card(expected)), "{what}");
        assert_eq!(bridge.live_tokens(NOW), 0, "{what}: no token");
    }
}

/// A card over broadcast is dropped as `broadcast` whatever its shape --
/// a card that would verify direct, and a malformed one -- while a `text`
/// envelope on the same channel is forwarded as before.
#[test]
fn a_card_over_broadcast_is_dropped_and_a_text_envelope_is_not() {
    for name in ["valid-at-the-bounds", "photo-over-the-bound"] {
        let v = vector(name);
        let mut bridge = leased();
        let got = bridge.notification(
            &broadcast(&v, payload(MEDIA_TYPE_V2, wire(&v).into_bytes())),
            [7; 16],
            NOW,
        );
        assert_eq!(got, Err(ConvertError::Card(CardDrop::Broadcast)), "{name}");
        assert_eq!(bridge.live_tokens(NOW), 0, "{name}: no token");
        for (r, count) in bridge.dropped_cards() {
            assert_eq!(
                count,
                u64::from(r == CardDrop::Broadcast),
                "{name}: {r:?} counted"
            );
        }
    }
    let v = vector("valid-minimal");
    let text = r#"{"v":2,"kind":"text","app_message_id":"0123456789abcdef0123456789abcdef","text":"hello"}"#;
    let (_, n) = line(
        &mut leased(),
        &broadcast(&v, payload(MEDIA_TYPE_V2, text.into())),
    );
    assert_eq!(n["content"], text);
    assert!(n["meta"].get("app_kind").is_none());
}

/// What is not a card stays unparsed: a `text` envelope, valid or not, an
/// envelope of a kind the parser does not know, and a card's very JSON
/// under any other media type -- forwarded as the text it is, with no
/// card key in `meta`.
#[test]
fn what_is_not_a_card_is_forwarded_as_before() {
    let card = vector("valid-at-the-bounds");
    let unknown = vector("unknown-kind");
    let cases = [
        (
            MEDIA_TYPE_V2,
            r#"{"v":2,"kind":"text","app_message_id":"0123456789abcdef0123456789abcdef","text":"app_kind=contact_card"}"#
                .to_owned(),
        ),
        (MEDIA_TYPE_V2, r#"{"v":2,"kind":"text","text":"no id"}"#.to_owned()),
        (MEDIA_TYPE_V2, wire(&unknown)),
        ("text/plain", wire(&card)),
        ("application/json", wire(&card)),
    ];
    for (media_type, text) in cases {
        let (_, n) = line(
            &mut leased(),
            &direct(&card, payload(media_type, text.clone().into_bytes())),
        );
        assert_eq!(n["content"], text.as_str(), "{media_type}: unchanged");
        for key in [
            "app_kind",
            "card_name",
            "card_photo_bytes",
            "card_identity_key",
        ] {
            assert!(n["meta"].get(key).is_none(), "{media_type}: no {key}");
        }
    }
}
