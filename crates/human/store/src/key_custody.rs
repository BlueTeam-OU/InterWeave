// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The platform's custody of a 32-byte secret at rest: the seam both
//! secrets this crate keeps beside the store go through -- each
//! identity's key ([`crate::identity_key`], ADR-0063) and the store's own
//! key (`store_key`, ADR-0060).
//!
//! One seam, two custodies: each secret names its own alias namespace
//! (`interweave.idk1.<profile>.<EndpointId>`,
//! `interweave.store.<profile>`) and its own record magic, so neither
//! record authenticates in the other's place, and neither alias is one
//! of the seed's `interweave.iwk1.<profile>.<policy>` (the embedded
//! host's `SeedCipher`, which nothing here touches).

use zeroize::Zeroizing;

/// Who holds the secret at rest, in increasing order of protection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Custody {
    /// An owner-only file and nothing more: ADR-0038's `filesystem-only`,
    /// the v1 desktop.
    FilesystemOnly,
    /// Sealed by a platform key the file does not hold: Android's
    /// `AndroidKeyStore`, `background-compatible`.
    Keystore,
}

impl Custody {
    pub(crate) const fn byte(self) -> u8 {
        match self {
            Self::FilesystemOnly => 1,
            Self::Keystore => 2,
        }
    }

    pub(crate) const fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(Self::FilesystemOnly),
            2 => Some(Self::Keystore),
            _ => None,
        }
    }
}

/// The platform half of the custody: seals and opens a secret under a
/// key the file does not hold, and retires that key.
pub trait KeyCipher {
    /// The custody this platform holds, the floor every record is held to.
    fn custody(&self) -> Custody;
    /// Seal `secret` under the key named `alias`, binding `aad`.
    ///
    /// # Errors
    /// The platform's refusal.
    fn seal(&self, alias: &str, aad: &[u8], secret: &[u8; 32]) -> Result<Vec<u8>, CipherFailure>;
    /// Open a body [`KeyCipher::seal`] produced under `alias` and `aad`.
    ///
    /// # Errors
    /// The platform's refusal: a wrong key, a changed `aad`, an
    /// invalidated or absent key.
    fn open(
        &self,
        alias: &str,
        aad: &[u8],
        sealed: &[u8],
    ) -> Result<Zeroizing<[u8; 32]>, CipherFailure>;
    /// Delete the key named `alias`, if the platform holds one.
    ///
    /// # Errors
    /// The platform's refusal.
    fn retire(&self, alias: &str) -> Result<(), CipherFailure>;
}

/// A platform cipher's refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CipherFailure {
    /// The wrapping key is gone: no key at the alias.
    KeyUnavailable,
    /// The platform holds the key and refuses to use it any more: a
    /// Keystore invalidation, after the screen lock was removed or the
    /// device's security changed. Told apart from
    /// [`CipherFailure::KeyUnavailable`] because ADR-0060 decision 6 offers
    /// the store's discard directly for this one and sends a missing key
    /// through recovery mode.
    KeyInvalidated,
    /// The body did not authenticate under the key and associated data.
    Unauthentic,
    /// Anything else the platform reported.
    Platform(String),
}

/// The desktop's custody: the file is the only protection, so sealing is
/// the identity and there is no key to retire.
#[derive(Debug, Clone, Copy, Default)]
pub struct FilesystemCustody;

impl KeyCipher for FilesystemCustody {
    fn custody(&self) -> Custody {
        Custody::FilesystemOnly
    }

    fn seal(&self, _: &str, _: &[u8], secret: &[u8; 32]) -> Result<Vec<u8>, CipherFailure> {
        Ok(secret.to_vec())
    }

    fn open(&self, _: &str, _: &[u8], sealed: &[u8]) -> Result<Zeroizing<[u8; 32]>, CipherFailure> {
        let secret: [u8; 32] = sealed.try_into().map_err(|_| CipherFailure::Unauthentic)?;
        Ok(Zeroizing::new(secret))
    }

    fn retire(&self, _: &str) -> Result<(), CipherFailure> {
        Ok(())
    }
}
