// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//
// A software stand-in for the Android Keystore, for the store key's tests.
// It proves the record's and the store's handling of a sealing platform;
// it does not prove the Keystore (that is the device row's).
#![allow(dead_code, clippy::expect_used)]

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use aes_gcm::aead::{Aead as _, KeyInit as _, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use interweave_human_store::key_custody::{CipherFailure, Custody, KeyCipher};
use zeroize::Zeroizing;

/// One AES-256-GCM key per alias, the IV beside the ciphertext, the
/// header as associated data. Records every alias it is asked for and
/// every one it retires; an alias can be marked invalidated, as the
/// Keystore does when the screen lock is removed.
#[derive(Default)]
pub(crate) struct SoftKeystore {
    keys: RefCell<HashMap<String, [u8; 32]>>,
    invalidated: RefCell<HashSet<String>>,
    pub(crate) used: RefCell<Vec<String>>,
    pub(crate) retired: RefCell<Vec<String>>,
}

impl SoftKeystore {
    /// From now on, `alias` is refused as invalidated.
    pub(crate) fn invalidate(&self, alias: &str) {
        self.invalidated.borrow_mut().insert(alias.to_owned());
    }

    /// Forget `alias`'s key, as a wiped Keystore would.
    pub(crate) fn lose(&self, alias: &str) {
        self.keys.borrow_mut().remove(alias);
    }

    /// Whether a key is held at `alias`.
    pub(crate) fn holds(&self, alias: &str) -> bool {
        self.keys.borrow().contains_key(alias)
    }
}

impl KeyCipher for SoftKeystore {
    fn custody(&self) -> Custody {
        Custody::Keystore
    }

    fn seal(&self, alias: &str, aad: &[u8], secret: &[u8; 32]) -> Result<Vec<u8>, CipherFailure> {
        self.used.borrow_mut().push(alias.to_owned());
        // The Keystore adapter generates a fresh key at the alias on every
        // seal, as KeystoreCipher.kt does.
        self.invalidated.borrow_mut().remove(alias);
        let key: [u8; 32] = rand::random();
        self.keys.borrow_mut().insert(alias.to_owned(), key);
        let iv: [u8; 12] = rand::random();
        let sealed = Aes256Gcm::new(&key.into())
            .encrypt(Nonce::from_slice(&iv), Payload { msg: secret, aad })
            .map_err(|_| CipherFailure::Platform("seal".to_owned()))?;
        Ok([iv.as_slice(), &sealed].concat())
    }

    fn open(
        &self,
        alias: &str,
        aad: &[u8],
        sealed: &[u8],
    ) -> Result<Zeroizing<[u8; 32]>, CipherFailure> {
        self.used.borrow_mut().push(alias.to_owned());
        if self.invalidated.borrow().contains(alias) {
            return Err(CipherFailure::KeyInvalidated);
        }
        let key = *self
            .keys
            .borrow()
            .get(alias)
            .ok_or(CipherFailure::KeyUnavailable)?;
        let (iv, ct) = sealed
            .split_at_checked(12)
            .ok_or(CipherFailure::Unauthentic)?;
        let plain = Zeroizing::new(
            Aes256Gcm::new(&key.into())
                .decrypt(Nonce::from_slice(iv), Payload { msg: ct, aad })
                .map_err(|_| CipherFailure::Unauthentic)?,
        );
        let secret: [u8; 32] = plain
            .as_slice()
            .try_into()
            .map_err(|_| CipherFailure::Unauthentic)?;
        Ok(Zeroizing::new(secret))
    }

    fn retire(&self, alias: &str) -> Result<(), CipherFailure> {
        self.keys.borrow_mut().remove(alias);
        self.invalidated.borrow_mut().remove(alias);
        self.retired.borrow_mut().push(alias.to_owned());
        Ok(())
    }
}
