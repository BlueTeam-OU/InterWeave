// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `EndpointId` against the frozen derivation vectors (ADR-0067).
//!
//! `tools/checks/verify_fixture_vectors.py` recomputes the fixture from the
//! specification and says the vectors are right; this reads the same file
//! and says the Rust type agrees with every one of them.

// A fixture that will not read means the test cannot ask its question.
#![allow(clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use interweave_transport_api::{EndpointId, IdError};

fn fixture() -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("crates/api/<crate> is three levels below the root")
        .join("fixtures/endpoints/endpoint-id-derivation-v2.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

fn key(hex: &str) -> [u8; 32] {
    assert_eq!(hex.len(), 64, "{hex}");
    let mut out = [0_u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("hex");
    }
    out
}

#[test]
fn every_derivation_vector_is_what_from_ed25519_public_writes() {
    let doc = fixture();
    let mut derived = 0;
    for v in doc["vectors"].as_array().expect("vectors") {
        let Some(pk) = v["public_key_hex"].as_str() else {
            continue;
        };
        let name = v["name"].as_str().expect("name");
        let expected = v["expected_endpoint_id"].as_str().expect("expected");
        let k = key(pk);
        let id = EndpointId::from_ed25519_public(&k);
        assert_eq!(id.as_str(), expected, "{name}");
        // The id names its key back, and the strict parser accepts its own
        // output.
        assert_eq!(id.ed25519_public(), Some(k), "{name}");
        assert_eq!(EndpointId::parse_derived(expected), Ok(id), "{name}");
        derived += 1;
    }
    assert!(derived >= 4, "the fixture lost its derivation vectors");
}

#[test]
fn every_verdict_vector_is_what_parse_derived_decides() {
    let doc = fixture();
    let (mut accepted, mut refused) = (0, 0);
    for v in doc["vectors"].as_array().expect("vectors") {
        let Some(text) = v["text"].as_str() else {
            continue;
        };
        let name = v["name"].as_str().expect("name");
        let valid = v["valid"].as_bool().expect("valid");
        let got = EndpointId::parse_derived(text);
        assert_eq!(got.is_ok(), valid, "{name}: {got:?}");
        if valid {
            accepted += 1;
        } else {
            refused += 1;
        }
    }
    // Both verdicts are exercised, so a parser that accepts or refuses
    // everything cannot pass.
    assert!(
        accepted >= 2 && refused >= 5,
        "{accepted} accepted, {refused} refused"
    );
}

#[test]
fn the_not_derived_refusals_are_named() {
    // A Qm PeerId, another key type and a changed case are PeerId-shaped
    // text that is not a derived id: the contract's EndpointIdNotDerived.
    let doc = fixture();
    for name in [
        "verdict-qm-peer-id",
        "verdict-other-key-type",
        "verdict-prefix-lowercased",
    ] {
        let v = doc["vectors"]
            .as_array()
            .expect("vectors")
            .iter()
            .find(|v| v["name"] == name)
            .unwrap_or_else(|| panic!("no vector {name}"));
        assert_eq!(
            EndpointId::parse_derived(v["text"].as_str().expect("text")),
            Err(IdError::NotDerived),
            "{name}"
        );
    }
}
