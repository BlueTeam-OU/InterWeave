// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The sender's half of the contact card (plan §29 step 2): a card built
//! from an identity's own draft and key verifies under `classify` on the
//! route it is sent on and on no other, refuses a draft a receiver would
//! call malformed, and reproduces the frozen vector byte for byte from
//! its test-only key.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use ed25519_dalek::{Signer as _, SigningKey};
use interweave_human_chat_protocol::{
    CardBuildError, CardDraft, CardFault, CardRoute, Classified, build_card, classify,
};
use interweave_transport_api::{EndpointId, TransportIdentity};

const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
const OTHER_PEER: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";
const ID: &str = "0123456789abcdef0123456789abcdef";

fn peer(s: &str) -> TransportIdentity {
    TransportIdentity::parse(s).expect("a PeerId")
}

fn endpoint(s: &str) -> EndpointId {
    EndpointId::parse(s).expect("an endpoint")
}

fn key() -> SigningKey {
    SigningKey::from_bytes(&[42u8; 32])
}

fn build(
    draft: CardDraft<'_>,
    sender: &TransportIdentity,
    on: &EndpointId,
) -> Result<String, CardBuildError> {
    let key = key();
    build_card(
        ID,
        draft,
        key.verifying_key().to_bytes(),
        CardRoute {
            source_peer: sender,
            source_endpoint: on,
        },
        None,
        |m| key.sign(m).to_bytes(),
    )
}

fn on(text: &str, from: &TransportIdentity, at: &EndpointId) -> Classified {
    classify(
        text,
        CardRoute {
            source_peer: from,
            source_endpoint: at,
        },
    )
}

#[test]
fn a_built_card_verifies_on_its_route_and_on_no_other() {
    let photo = [7u8; 300];
    let draft = CardDraft {
        name: "Zoe\u{0308}",
        surname: Some("Ångström"),
        photo: Some(("image/png", &photo)),
    };
    let text = build(draft, &peer(PEER), &endpoint("human")).expect("built");
    let Classified::Card(card) = on(&text, &peer(PEER), &endpoint("human")) else {
        panic!("not a card on its own route: {text}");
    };
    assert_eq!(card.name, "Zoë", "sent after NFC");
    assert_eq!(card.surname.as_deref(), Some("Ångström"));
    assert_eq!(card.photo_len(), 300);
    assert_eq!(card.identity_key, key().verifying_key().to_bytes());
    assert_eq!(card.app_message_id, ID);
    // Another PeerId, or another identity of this node: not this card.
    assert_eq!(
        on(&text, &peer(OTHER_PEER), &endpoint("human")),
        Classified::MalformedCard(CardFault::SignatureFails)
    );
    assert_eq!(
        on(&text, &peer(PEER), &endpoint("work")),
        Classified::MalformedCard(CardFault::SignatureFails)
    );
}

#[test]
fn a_draft_a_receiver_would_refuse_is_refused_before_it_is_signed() {
    let peer = peer(PEER);
    let human = endpoint("human");
    let draft = |name, surname, photo| CardDraft {
        name,
        surname,
        photo,
    };
    let long = "a".repeat(65);
    let big = vec![0u8; 24_577];
    assert_eq!(
        build(draft(&long, None, None), &peer, &human),
        Err(CardBuildError::Name)
    );
    assert_eq!(
        build(draft("", None, None), &peer, &human),
        Err(CardBuildError::Name)
    );
    assert_eq!(
        build(draft("Ada", Some("a\u{200B}b"), None), &peer, &human),
        Err(CardBuildError::Surname)
    );
    assert_eq!(
        build(
            draft("Ada", None, Some(("image/gif", &[0u8; 4]))),
            &peer,
            &human
        ),
        Err(CardBuildError::PhotoMediaType)
    );
    assert_eq!(
        build(draft("Ada", None, Some(("image/png", &big))), &peer, &human),
        Err(CardBuildError::PhotoTooLarge)
    );
    // The bounds themselves are legal.
    let at = vec![0u8; 24_576];
    assert!(
        build(
            draft(&"a".repeat(64), None, Some(("image/jpeg", &at))),
            &peer,
            &human
        )
        .is_ok()
    );
}

#[test]
fn the_frozen_valid_minimal_card_is_what_the_builder_makes_from_its_test_only_key() {
    // The Python verifier and this builder sign independently; Ed25519 is
    // deterministic, so the same key over the same draft and route gives
    // the frozen signature exactly.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the root")
        .join("fixtures/human-chat-v2/human-chat-v2-contact-card.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the fixture")).expect("json");
    let v = doc["vectors"]
        .as_array()
        .expect("vectors")
        .iter()
        .find(|v| v["name"] == "valid-minimal")
        .expect("valid-minimal");
    let seed_hex = v["signer"]["test_only_seed_hex"].as_str().expect("seed");
    let seed: [u8; 32] = (0..32)
        .map(|i| u8::from_str_radix(&seed_hex[2 * i..2 * i + 2], 16).expect("hex"))
        .collect::<Vec<_>>()
        .try_into()
        .expect("32 bytes");
    let key = SigningKey::from_bytes(&seed);
    let on = &v["received_on"];
    let sender = peer(on["peer_id"].as_str().expect("peer"));
    let endpoint = endpoint(on["source_endpoint"].as_str().expect("endpoint"));
    let text = build_card(
        v["envelope"]["app_message_id"].as_str().expect("id"),
        CardDraft {
            name: v["envelope"]["card"]["name"].as_str().expect("name"),
            surname: None,
            photo: None,
        },
        key.verifying_key().to_bytes(),
        CardRoute {
            source_peer: &sender,
            source_endpoint: &endpoint,
        },
        None,
        |m| key.sign(m).to_bytes(),
    )
    .expect("built");
    let built: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(
        built, v["envelope"],
        "the frozen envelope, member for member"
    );
}
