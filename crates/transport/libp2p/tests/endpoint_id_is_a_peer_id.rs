// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An `EndpointId` is derived exactly as libp2p derives a `PeerId` (ADR-0067
//! decision 2).
//!
//! The neutral contract crate hand-rolls the derivation (`00 24 08 01 12 20`
//! then the key, base58btc) because it may not depend on libp2p. This crate
//! does, so it is where the hand-rolled bytes are held to the real mechanism:
//! if libp2p's encoding of an Ed25519 `PeerId` ever differed, the contract
//! would be describing something the transport does not do.

// Key construction from fixed bytes is infallible for 32 bytes; a failure is
// a broken test, so panicking is correct.
#![allow(clippy::expect_used)]

use interweave_test_support::endpoint_keys;
use interweave_transport_api::EndpointId;
use libp2p::PeerId;
use libp2p::identity::{PublicKey, ed25519};

fn libp2p_key(seed: [u8; 32]) -> ed25519::PublicKey {
    let secret = ed25519::SecretKey::try_from_bytes(seed).expect("32-byte seed");
    ed25519::Keypair::from(secret).public()
}

#[test]
fn the_derived_id_text_is_the_peer_id_text_of_the_same_key() {
    for seed in [[0_u8; 32], [1; 32], [0x7f; 32], [0xff; 32], [0xa5; 32]] {
        let public = libp2p_key(seed);
        let id = EndpointId::from_ed25519_public(&public.to_bytes());
        let peer = PeerId::from_public_key(&PublicKey::from(public));
        assert_eq!(id.as_str(), peer.to_base58(), "seed {seed:02x?}");
        // And the strict parser takes the transport's own text.
        assert_eq!(
            EndpointId::parse_derived(peer.to_base58()).expect("derived"),
            id
        );
    }
}

#[test]
fn the_test_support_keys_derive_the_peer_id_of_their_public_key() {
    for name in ["human", "claude", "fabric-control"] {
        let (signing, id) = endpoint_keys::endpoint_keys(name);
        let public = ed25519::PublicKey::try_from_bytes(&signing.verifying_key().to_bytes())
            .expect("a valid Ed25519 public key");
        let peer = PeerId::from_public_key(&PublicKey::from(public));
        assert_eq!(id.as_str(), peer.to_base58(), "{name}");
    }
}
