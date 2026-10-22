// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The card parser agrees with the frozen contact-card vectors
//! (`fixtures/human-chat-v2/human-chat-v2-contact-card.json`, HUMAN-CHAT.md
//! §Fixture cases, ADR-0063).
//!
//! `verify_fixture_vectors.py` recomputes every verdict from the
//! specification, with its own strict verifier and its own canonical
//! bytes; this file holds the Rust parser to the same verdicts, by name,
//! so the two implementations agreeing is what is shown.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
use interweave_human_chat_protocol::{
    CardFault, CardRoute, Classified, EnvelopeError, HumanChatV2, canonical_card_bytes, classify,
};
use interweave_transport_api::{EndpointId, TransportIdentity};

/// Every case, and the answer the parser owes it. A vector the fixture
/// gains or loses fails `every_case_is_named_here` until this list says
/// what the parser must answer.
const CASES: &[(&str, Expect)] = &[
    ("valid-at-the-bounds", Expect::Card),
    ("valid-minimal", Expect::Card),
    ("non-nfc-name-verifies-after-normalisation", Expect::Card),
    ("identifier-like-members-ignored", Expect::Card),
    (
        "signed-for-another-peer-id",
        Expect::Fault(CardFault::SignatureFails),
    ),
    (
        "name-changed-after-signing",
        Expect::Fault(CardFault::SignatureFails),
    ),
    (
        "identity-key-31-bytes",
        Expect::Fault(CardFault::IdentityKeyLength),
    ),
    (
        "small-order-identity-key",
        Expect::Fault(CardFault::SignatureFails),
    ),
    (
        "signature-small-order-r",
        Expect::Fault(CardFault::SignatureFails),
    ),
    (
        "signature-non-canonical-s",
        Expect::Fault(CardFault::SignatureFails),
    ),
    (
        "identity-key-unpadded",
        Expect::Fault(CardFault::IdentityKeyEncoding),
    ),
    (
        "identity-key-url-safe",
        Expect::Fault(CardFault::IdentityKeyEncoding),
    ),
    ("text-present", Expect::Fault(CardFault::TextPresent)),
    ("reply-to-present", Expect::Fault(CardFault::ReplyToPresent)),
    ("name-over-the-bound", Expect::Fault(CardFault::Name)),
    (
        "photo-over-the-bound",
        Expect::Fault(CardFault::PhotoTooLarge),
    ),
    (
        "photo-third-media-type",
        Expect::Fault(CardFault::PhotoMediaType),
    ),
    (
        "photo-data-whitespace",
        Expect::Fault(CardFault::PhotoEncoding),
    ),
    (
        "photo-data-url-safe",
        Expect::Fault(CardFault::PhotoEncoding),
    ),
    ("unknown-kind", Expect::UnknownKind),
];

#[derive(Debug, Clone, Copy)]
enum Expect {
    Card,
    Fault(CardFault),
    /// Not a card, and not a message either: `HumanChatV2::parse` refuses
    /// the kind, so the envelope is malformed with no network effect.
    UnknownKind,
}

fn fixture() -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("crates/human/<crate> is three levels below the root")
        .join("fixtures/human-chat-v2/human-chat-v2-contact-card.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

fn vector<'a>(doc: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    doc["vectors"]
        .as_array()
        .expect("vectors")
        .iter()
        .find(|v| v["name"] == name)
        .unwrap_or_else(|| panic!("the fixture has no vector {name}"))
}

/// What arrives on the wire is text; the fixture holds the envelope as a
/// JSON value.
fn wire(v: &serde_json::Value) -> String {
    serde_json::to_string(&v["envelope"]).expect("re-serialises")
}

fn route(v: &serde_json::Value) -> (TransportIdentity, EndpointId) {
    let on = &v["received_on"];
    (
        TransportIdentity::parse(on["peer_id"].as_str().expect("peer_id")).expect("a PeerId"),
        EndpointId::parse(on["source_endpoint"].as_str().expect("endpoint")).expect("an endpoint"),
    )
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn b64(s: &str) -> Vec<u8> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .expect("standard base64")
}

fn classified(v: &serde_json::Value) -> Classified {
    let (peer, endpoint) = route(v);
    classify(
        &wire(v),
        CardRoute {
            source_peer: &peer,
            source_endpoint: &endpoint,
        },
    )
}

#[test]
fn every_case_is_named_here() {
    let doc = fixture();
    let mut frozen: Vec<&str> = doc["vectors"]
        .as_array()
        .expect("vectors")
        .iter()
        .map(|v| v["name"].as_str().expect("name"))
        .collect();
    let mut named: Vec<&str> = CASES.iter().map(|(n, _)| *n).collect();
    frozen.sort_unstable();
    named.sort_unstable();
    assert_eq!(frozen, named, "the fixture and this file's cases");
}

#[test]
fn every_frozen_card_gets_its_verdict_and_its_one_reason() {
    let doc = fixture();
    for (name, expect) in CASES {
        let v = vector(&doc, name);
        let got = classified(v);
        match expect {
            Expect::Card => {
                let Classified::Card(card) = &got else {
                    panic!("{name}: a valid card, got {got:?}");
                };
                assert_eq!(v["valid"], true, "{name}: the fixture's verdict");
                let raw = &v["envelope"]["card"];
                assert_eq!(
                    card.identity_key.as_slice(),
                    b64(raw["identity_key"].as_str().expect("key")),
                    "{name}: the key"
                );
                let photo = raw["photo"]["data"].as_str().map_or(0, |d| b64(d).len());
                assert_eq!(
                    card.photo_len(),
                    photo,
                    "{name}: the photo's decoded length"
                );
            }
            Expect::Fault(fault) => {
                assert_eq!(got, Classified::MalformedCard(*fault), "{name}");
                assert_eq!(v["valid"], false, "{name}: the fixture's verdict");
                // The fixture's rule and the parser's reason are one name.
                assert_eq!(v["rule"], fault.as_str(), "{name}: the fixture's rule");
            }
            Expect::UnknownKind => {
                assert_eq!(got, Classified::NotCard, "{name}");
                assert_eq!(
                    HumanChatV2::parse(&wire(v)),
                    Err(EnvelopeError::UnsupportedKind),
                    "{name}: malformed as a message too"
                );
                assert_eq!(v["rule"], "kind", "{name}: the fixture's rule");
            }
        }
    }
}

#[test]
fn the_canonical_bytes_are_the_fixtures() {
    // The Python verifier and this crate build the bytes independently.
    let doc = fixture();
    let mut compared = 0;
    for v in doc["vectors"].as_array().expect("vectors") {
        let Some(stated) = v["canonical_bytes_hex"].as_str() else {
            continue;
        };
        let card = &v["envelope"]["card"];
        let nfc = |s: &str| unicode_nfc(s);
        let name = nfc(card["name"].as_str().expect("name"));
        let surname = card["surname"].as_str().map(nfc);
        let photo = card["photo"]["data"].as_str().map(|d| {
            (
                card["photo"]["media_type"].as_str().expect("media type"),
                b64(d),
            )
        });
        let (peer, endpoint) = route(v);
        let peer = bs58::decode(peer.as_str()).into_vec().expect("base58");
        let bytes = canonical_card_bytes(
            &name,
            surname.as_deref(),
            photo.as_ref().map(|(t, b)| (*t, b.as_slice())),
            &peer,
            endpoint.as_str(),
        )
        .expect("fits");
        assert_eq!(bytes, hex(stated), "{}", v["name"]);
        compared += 1;
    }
    assert_eq!(compared, 9, "every vector that reaches the signature");
}

fn unicode_nfc(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization as _;
    s.nfc().collect()
}

fn key_sig_message(doc: &serde_json::Value, name: &str) -> (VerifyingKey, Signature, Vec<u8>) {
    let v = vector(doc, name);
    let card = &v["envelope"]["card"];
    let key: [u8; 32] = b64(card["identity_key"].as_str().expect("key"))
        .try_into()
        .expect("32 bytes");
    let sig: [u8; 64] = b64(card["signature"].as_str().expect("sig"))
        .try_into()
        .expect("64 bytes");
    (
        VerifyingKey::from_bytes(&key).expect("a point"),
        Signature::from_bytes(&sig),
        hex(v["canonical_bytes_hex"].as_str().expect("bytes")),
    )
}

#[test]
fn the_small_order_key_and_r_cards_verify_without_the_strict_checks() {
    // Each is built so the cofactorless equation holds: the crate's
    // non-strict `verify`, which lacks only the small-order checks,
    // accepts it, and `verify_strict` alone refuses it. A card that every
    // verifier refused would pin nothing (HUMAN-CHAT.md §Fixture cases).
    let doc = fixture();
    for name in ["small-order-identity-key", "signature-small-order-r"] {
        let (key, sig, message) = key_sig_message(&doc, name);
        assert!(
            key.verify(&message, &sig).is_ok(),
            "{name}: non-strict verify accepts"
        );
        assert!(
            key.verify_strict(&message, &sig).is_err(),
            "{name}: strict refuses"
        );
    }
    let (key, _, _) = key_sig_message(&doc, "small-order-identity-key");
    assert!(key.is_weak(), "the small-order card's key is small order");
    let r = key_sig_message(&doc, "signature-small-order-r")
        .1
        .r_bytes()
        .to_owned();
    assert_eq!(r[0], 1, "R is the identity point's encoding");
    assert!(
        r[1..].iter().all(|b| *b == 0),
        "R is the identity point's encoding"
    );
}

#[test]
fn the_non_canonical_s_card_is_the_valid_signature_raised_by_the_group_order() {
    // The scalar check sits in the decoding every verify of the crate
    // shares, so this card fails under `verify` too: it fails whenever the
    // build turns `legacy_compatibility` on, which would accept S >= l.
    let doc = fixture();
    let (key, high, message) = key_sig_message(&doc, "signature-non-canonical-s");
    let (_, valid, valid_message) = key_sig_message(&doc, "valid-minimal");
    assert_eq!(message, valid_message, "over the valid card's bytes");
    assert_eq!(high.r_bytes(), valid.r_bytes(), "the same R");
    // l, little-endian.
    let order: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
    ];
    let mut sum = [0u8; 32];
    let mut carry = 0u16;
    for i in 0..32 {
        let t = u16::from(valid.s_bytes()[i]) + u16::from(order[i]) + carry;
        sum[i] = (t & 0xff) as u8;
        carry = t >> 8;
    }
    assert_eq!(carry, 0, "no overflow past 256 bits");
    assert_eq!(high.s_bytes(), &sum, "S is the valid S plus l");
    assert!(
        high.s_bytes()[31] < 0x20,
        "and below 2^253, so no top-bits check sees it"
    );
    assert!(
        key.verify(&valid_message, &valid).is_ok(),
        "the valid one verifies"
    );
    assert!(
        key.verify(&message, &high).is_err(),
        "S plus l does not, under any verify"
    );
    assert!(key.verify_strict(&message, &high).is_err());
}

const ID: &str = "00000000000000000000000000000000";

fn classify_on_fixture_route(text: &str) -> Classified {
    let doc = fixture();
    let (peer, endpoint) = route(vector(&doc, "valid-minimal"));
    classify(
        text,
        CardRoute {
            source_peer: &peer,
            source_endpoint: &endpoint,
        },
    )
}

#[test]
fn any_top_level_contact_card_kind_makes_a_card_so_a_duplicate_is_malformed_never_not_a_card() {
    for kinds in [
        r#""kind":"text","kind":"contact_card""#,
        r#""kind":"contact_card","kind":"text""#,
        r#""kind":"contact_card","kind":"contact_card""#,
    ] {
        let text = format!(r#"{{"v":2,{kinds},"app_message_id":"{ID}","text":"hi"}}"#);
        assert_eq!(
            classify_on_fixture_route(&text),
            Classified::MalformedCard(CardFault::DuplicateMember),
            "{kinds}"
        );
    }
    // A duplicate anywhere inside a card is the same fault.
    let doc = fixture();
    let valid = wire(vector(&doc, "valid-minimal"));
    let doubled = valid.replacen(r#""name":"Ada""#, r#""name":"Ada","name":"Eve""#, 1);
    assert_ne!(doubled, valid, "the duplicate was planted");
    assert_eq!(
        classify_on_fixture_route(&doubled),
        Classified::MalformedCard(CardFault::DuplicateMember)
    );
    // POSITIVE CONTROL: the same card with one name is a card.
    assert!(matches!(
        classify_on_fixture_route(&valid),
        Classified::Card(_)
    ));
}

#[test]
fn json_this_parser_cannot_represent_never_hides_a_card() {
    // Each text is well-formed JSON with a top-level `kind` that is
    // `contact_card`; a reader that keeps the last copy, or takes 1e400
    // as Infinity or a lone surrogate as written, sees a card. So the
    // answer is a malformed card, never "not a card", whether the value
    // serde_json cannot hold is a second `kind`, a key, or any member.
    let doc = fixture();
    let valid = wire(vector(&doc, "valid-minimal"));
    let body = valid.strip_prefix('{').expect("an object");
    let deep = format!("{}{}", "[".repeat(200), "]".repeat(200));
    for prefix in [
        r#""kind":1e400,"#.to_owned(),
        r#""kind":"\ud800","#.to_owned(),
        format!(r#""kind":{deep},"#),
        r#""\ud800":0,"#.to_owned(),
        r#""x":1e400,"#.to_owned(),
        r#""x":"\udc00","#.to_owned(),
        format!(r#""x":{deep},"#),
    ] {
        let text = format!("{{{prefix}{body}");
        assert_eq!(
            classify_on_fixture_route(&text),
            Classified::MalformedCard(CardFault::Unreadable),
            "{prefix}"
        );
    }
    // The same, inside the card.
    let inside = valid.replacen(r#""card":{"#, r#""card":{"x":1e400,"#, 1);
    assert_ne!(inside, valid, "planted");
    assert_eq!(
        classify_on_fixture_route(&inside),
        Classified::MalformedCard(CardFault::Unreadable)
    );
}

#[test]
fn a_raw_control_character_is_not_json_in_a_key_or_a_value() {
    // RFC 8259 forbids an unescaped control character in any string, so
    // a text carrying one is not JSON and no reader sees a card in it:
    // NotCard, in a member name exactly as in a value.
    let doc = fixture();
    let valid = wire(vector(&doc, "valid-minimal"));
    let body = valid.strip_prefix('{').expect("an object");
    for prefix in ["\"a\tb\":0,", "\"x\":\"a\tb\","] {
        let text = format!("{{{prefix}{body}");
        assert!(text.contains('\t'), "the raw tab is in the text");
        assert_eq!(
            classify_on_fixture_route(&text),
            Classified::NotCard,
            "{prefix:?}"
        );
    }
    // The ESCAPED control character is JSON, in a name as in a value: an
    // ignored member, and the card stands.
    let bs = '\\';
    for prefix in [
        format!(r#""a{bs}u0009b":0,"#),
        format!(r#""x":"a{bs}u0009b","#),
    ] {
        let text = format!("{{{prefix}{body}");
        assert!(text.contains("u0009"), "the escape is in the text");
        assert!(
            matches!(classify_on_fixture_route(&text), Classified::Card(_)),
            "{prefix}"
        );
    }
}

#[test]
fn a_kind_is_read_after_its_escapes() {
    // An escaped `k` (U+006B) and `_` (U+005F) spell the same member and
    // the same value, so the escaped spelling is a card. Built from a
    // backslash so the escapes reach the parser as written.
    let bs = '\\';
    let doc = fixture();
    let valid = wire(vector(&doc, "valid-minimal"));
    let escaped = valid.replacen(
        r#""kind":"contact_card""#,
        &format!(r#""{bs}u006bind":"contact{bs}u005fcard""#),
        1,
    );
    assert_ne!(escaped, valid, "planted");
    assert!(escaped.contains("u006bind"), "the escape is in the text");
    assert!(matches!(
        classify_on_fixture_route(&escaped),
        Classified::Card(_)
    ));
    // And beside the plain spelling it is a second `kind`.
    let both = valid.replacen(
        r#""kind":"contact_card""#,
        &format!(r#""kind":"contact_card","{bs}u006bind":"text""#),
        1,
    );
    assert_eq!(
        classify_on_fixture_route(&both),
        Classified::MalformedCard(CardFault::DuplicateMember)
    );
}

#[test]
fn a_text_message_and_a_non_object_are_not_cards() {
    let text = format!(r#"{{"v":2,"kind":"text","app_message_id":"{ID}","text":"contact_card"}}"#);
    assert_eq!(classify_on_fixture_route(&text), Classified::NotCard);
    // A text message with a duplicate is malformed as a message, and not a card.
    let doubled =
        format!(r#"{{"v":2,"kind":"text","kind":"text","app_message_id":"{ID}","text":"hi"}}"#);
    assert_eq!(classify_on_fixture_route(&doubled), Classified::NotCard);
    for not_object in ["", "[]", r#""contact_card""#, "{", "null"] {
        assert_eq!(
            classify_on_fixture_route(not_object),
            Classified::NotCard,
            "{not_object:?}"
        );
    }
    // A nested kind is not the envelope's kind.
    let nested = format!(
        r#"{{"v":2,"kind":"text","app_message_id":"{ID}","text":"hi","x":{{"kind":"contact_card"}}}}"#
    );
    assert_eq!(classify_on_fixture_route(&nested), Classified::NotCard);
}

#[test]
fn a_card_on_another_endpoint_fails_its_signature() {
    // The source endpoint is the route's, never the envelope's: the
    // valid card verifies on the endpoint it was signed for and on no
    // other, whatever `from_endpoint` says.
    let doc = fixture();
    let v = vector(&doc, "valid-minimal");
    let (peer, _) = route(v);
    let other = EndpointId::parse("admin").expect("an endpoint");
    let valid = wire(v);
    let text = format!(
        r#"{},"from_endpoint":"human"}}"#,
        valid.strip_suffix('}').expect("an object")
    );
    assert_eq!(
        classify(
            &text,
            CardRoute {
                source_peer: &peer,
                source_endpoint: &other,
            }
        ),
        Classified::MalformedCard(CardFault::SignatureFails)
    );
}

#[test]
fn every_fault_has_a_distinct_fixed_name() {
    use std::collections::BTreeSet;
    let all = [
        CardFault::DuplicateMember,
        CardFault::Unreadable,
        CardFault::Version,
        CardFault::AppMessageId,
        CardFault::SentAtMs,
        CardFault::FromEndpoint,
        CardFault::TextPresent,
        CardFault::ReplyToPresent,
        CardFault::MissingCard,
        CardFault::Name,
        CardFault::Surname,
        CardFault::PhotoMediaType,
        CardFault::PhotoEncoding,
        CardFault::PhotoTooLarge,
        CardFault::IdentityKeyEncoding,
        CardFault::IdentityKeyLength,
        CardFault::SignatureEncoding,
        CardFault::SignatureLength,
        CardFault::SignatureFails,
    ];
    let names: BTreeSet<&str> = all.iter().map(|f| f.as_str()).collect();
    assert_eq!(names.len(), all.len());
}
