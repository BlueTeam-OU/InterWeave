// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

//! Deterministic, TEST-ONLY endpoint key pairs and the derived ids they name.
//!
//! **TEST-ONLY KEY MATERIAL.** The seed of every key here is
//! `SHA-256(DOMAIN ‖ name)`, public by construction: anyone can recompute
//! the secret from the name. It is for tests that need a real Ed25519 key
//! behind an `EndpointId` (ADR-0067) and must never stand in for an identity
//! outside a test.

use ed25519_dalek::SigningKey;
use interweave_transport_api::EndpointId;
use sha2::{Digest, Sha256};

/// Domain separator of the seeds; a test-only string, never a secret.
pub const DOMAIN: &[u8] = b"interweave/test-only/endpoint-key/v1\x00";

/// The test-only signing key for `name`: seed `SHA-256(DOMAIN ‖ name)`.
#[must_use]
pub fn signing_key(name: &str) -> SigningKey {
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update(name.as_bytes());
    SigningKey::from_bytes(&hasher.finalize().into())
}

/// The derived [`EndpointId`] of [`signing_key`]`(name)`: the same name always
/// gives the same id, distinct names distinct ids.
#[must_use]
pub fn endpoint_id(name: &str) -> EndpointId {
    EndpointId::from_ed25519_public(&signing_key(name).verifying_key().to_bytes())
}

/// The key pair and its derived id together.
#[must_use]
pub fn endpoint_keys(name: &str) -> (SigningKey, EndpointId) {
    let key = signing_key(name);
    let id = EndpointId::from_ed25519_public(&key.verifying_key().to_bytes());
    (key, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_id_is_derived_deterministic_and_per_name() {
        let a = endpoint_id("human");
        assert_eq!(a, endpoint_id("human"));
        assert_ne!(a, endpoint_id("claude"));
        assert!(EndpointId::parse_derived(a.as_str()).is_ok());
        let (key, id) = endpoint_keys("human");
        assert_eq!(id, a);
        assert_eq!(id.ed25519_public(), Some(key.verifying_key().to_bytes()));
    }
}
