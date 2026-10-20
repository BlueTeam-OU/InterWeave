// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the identity screens ask of the platform (design/2026-10-19-
//! android-identity.md, plan section 20 steps 7-8): make a new identity in
//! memory and seal it only when the person has checked their words, check
//! a phrase they entered, restore one, and keep the one non-secret marker
//! that says a creation showed its words and never finished.
//!
//! Two refusals come after the record is published, and only those are
//! read back (#303 rounds 2 and 3): a key of the other policy that could
//! not be retired (`OldKeyRemains`), and the directory's sync after the
//! publish (`Unsynced`). The read-back is an unlock: success only when the
//! record opens, under the cipher, to the `PeerId` just shown or
//! confirmed. Reading the record's `PeerId` field alone would not do: at
//! a recovery the old record names that `PeerId` already, so a write that
//! failed before publishing would read as done. Every other refusal is
//! reported as it is.

use std::path::{Path, PathBuf};

use interweave_profile_config::{PersistError, ProfileLock, ProfilePaths, TrustBoundary};
use interweave_profile_identity::{ProfileIdentity, RecoveryPhrase};
use interweave_transport_api::TransportIdentity;
use interweave_transport_embedded::custody::{self, CipherFailure, CustodyRefused, SeedCipher};

use crate::identity::{Expected, RestoreRefused, restore_identity};

/// Words in a phrase.
pub const PHRASE_WORDS: usize = 24;

/// A phrase's words, as references into [`english`].
pub type Words = [&'static str; PHRASE_WORDS];

/// The marker's file name, in the profile's human-client directory.
const VOID_PAPER: &str = "void-paper";

/// BIP-39's English list, sorted as published: the picker's words.
#[must_use]
pub fn english() -> &'static [&'static str] {
    bip39::Language::English.word_list()
}

/// Why a write the screens asked for did not happen. In each, nothing the
/// person relies on was changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteRefused {
    /// The profile is held by a running host or another flow.
    Busy,
    /// Storage or the cipher failed.
    Storage(String),
    /// The key needs the person to authenticate first.
    UserNotAuthenticated,
    /// The profile was provisioned meanwhile: the words shown were never
    /// used.
    AlreadyProvisioned,
    /// The phrase restores another identity than expected.
    OtherIdentity,
    /// Nothing to restore against, or a record of another shape: the
    /// screens never ask this (K1 offers no restore then).
    NoRestore,
}

/// A new identity, unsealed: it exists only in memory until [`seal`].
pub struct Creation {
    identity: ProfileIdentity,
    peer: TransportIdentity,
}

impl Creation {
    /// Make one, with the words the person writes down, its `PeerId` and
    /// the two distinct positions C3 checks.
    ///
    /// # Errors
    /// [`WriteRefused::Storage`] when the identity gives no phrase or
    /// `PeerId` -- a fault, not a person's.
    pub fn new() -> Result<(Self, Words, TransportIdentity, [usize; 2]), WriteRefused> {
        let identity = ProfileIdentity::generate();
        let peer = identity
            .transport_identity()
            .map_err(|e| WriteRefused::Storage(e.to_string()))?;
        let words = words_of(
            &identity
                .recovery_phrase()
                .map_err(|e| WriteRefused::Storage(e.to_string()))?,
        )?;
        let first = rand::random_range(0..PHRASE_WORDS);
        let second = (first + rand::random_range(1..PHRASE_WORDS)) % PHRASE_WORDS;
        Ok((
            Self {
                identity,
                peer: peer.clone(),
            },
            words,
            peer,
            [first, second],
        ))
    }
}

/// `phrase`'s words as references into [`english`]: no owned copy of a
/// word is made.
///
/// # Errors
/// [`WriteRefused::Storage`] for a word outside the list, which a parsed
/// phrase never has.
pub fn words_of(phrase: &RecoveryPhrase) -> Result<Words, WriteRefused> {
    let list = english();
    let exposed = phrase.expose_words();
    let mut words = [""; PHRASE_WORDS];
    let mut count = 0;
    for (slot, word) in words.iter_mut().zip(exposed.split_whitespace()) {
        let found = list
            .binary_search(&word)
            .map_err(|_| WriteRefused::Storage("a word outside the list".to_owned()))?;
        *slot = list[found];
        count += 1;
    }
    if count == PHRASE_WORDS {
        Ok(words)
    } else {
        Err(WriteRefused::Storage(
            "a phrase of another length".to_owned(),
        ))
    }
}

fn paths(app_data_dir: &Path, profile: &str) -> Result<ProfilePaths, WriteRefused> {
    TrustBoundary::new(app_data_dir)
        .and_then(|boundary| ProfilePaths::resolve_embedded(profile, boundary))
        .map_err(|e| WriteRefused::Storage(e.to_string()))
}

/// The read-back: whether the record opens, under `cipher`, to `peer`.
fn opens_to(paths: &ProfilePaths, cipher: &dyn SeedCipher, peer: &TransportIdentity) -> bool {
    custody::unlock(paths, cipher)
        .ok()
        .and_then(|identity| identity.transport_identity().ok())
        .is_some_and(|opened| opened == *peer)
}

/// The profile's lock for a flow that shows or takes words: taken once,
/// before the first word, without waiting, and lent to [`seal`],
/// [`restore`] and the read-backs until the flow ends. Released and
/// retaken, the gap would let the service start the profile and refuse
/// the seal after all 24 words were entered (android-key-custody.md,
/// Recovery).
///
/// # Errors
/// [`WriteRefused::Busy`] while another holds it, else
/// [`WriteRefused::Storage`].
pub fn hold(app_data_dir: &Path, profile: &str) -> Result<ProfileLock, WriteRefused> {
    let paths = paths(app_data_dir, profile)?;
    ProfileLock::acquire(&paths, std::time::Duration::ZERO).map_err(|e| match e {
        PersistError::ProfileLocked { .. } => WriteRefused::Busy,
        other => WriteRefused::Storage(other.to_string()),
    })
}

/// Seal `creation` under the profile's configured unlock policy (C4),
/// under the flow's `lock` ([`hold`]), or the seal's own with none.
///
/// # Errors
/// [`WriteRefused`], after the read-back.
pub fn seal(
    app_data_dir: &Path,
    profile: &str,
    cipher: &dyn SeedCipher,
    creation: &Creation,
    lock: Option<&ProfileLock>,
) -> Result<(), WriteRefused> {
    let paths = paths(app_data_dir, profile)?;
    let policy = interweave_profile_config::ProfileConfig::load(&paths)
        .map_err(|e| WriteRefused::Storage(format!("the profile: {e}")))?
        .runtime
        .android
        .key_unlock_policy;
    let provisioned = match lock {
        Some(lock) => custody::provision_held(&paths, lock, cipher, &creation.identity, policy),
        None => custody::provision(&paths, cipher, &creation.identity, policy),
    };
    match provisioned {
        Ok(_) => Ok(()),
        // Refused after the publish: done if the record opens to this
        // identity, and the next start's reseal finishes the rest.
        Err(
            refused @ (CustodyRefused::OldKeyRemains(_)
            | CustodyRefused::Storage(PersistError::Unsynced(_))),
        ) => {
            if opens_to(&paths, cipher, &creation.peer) {
                Ok(())
            } else {
                Err(WriteRefused::Storage(format!("{refused:?}")))
            }
        }
        Err(CustodyRefused::AlreadyProvisioned) => Err(WriteRefused::AlreadyProvisioned),
        Err(CustodyRefused::ProfileLocked) => Err(WriteRefused::Busy),
        Err(CustodyRefused::Cipher(CipherFailure::UserNotAuthenticated)) => {
            Err(WriteRefused::UserNotAuthenticated)
        }
        Err(other) => Err(WriteRefused::Storage(format!("{other:?}"))),
    }
}

/// The words and `PeerId` S2 shows (Show my phrase, design section 3.5),
/// re-derived from the sealed record. The caller has taken the service
/// offline first; this only reads.
///
/// # Errors
/// [`WriteRefused::UserNotAuthenticated`] when the key needs the person
/// again, else [`WriteRefused::Storage`]: nothing is shown.
pub fn shown_phrase(
    app_data_dir: &Path,
    profile: &str,
    cipher: &dyn SeedCipher,
) -> Result<(Words, TransportIdentity), WriteRefused> {
    let paths = paths(app_data_dir, profile)?;
    let identity = custody::unlock(&paths, cipher).map_err(|refused| match refused {
        custody::UnlockRefused::UserNotAuthenticated => WriteRefused::UserNotAuthenticated,
        other => WriteRefused::Storage(format!("{other:?}")),
    })?;
    let peer = identity
        .transport_identity()
        .map_err(|e| WriteRefused::Storage(e.to_string()))?;
    let words = words_of(
        &identity
            .recovery_phrase()
            .map_err(|e| WriteRefused::Storage(e.to_string()))?,
    )?;
    Ok((words, peer))
}

/// The `PeerId` a typed phrase derives to, or `None` when it is not a
/// phrase (a failed checksum, an unknown word, another length). The text
/// is dropped here.
#[must_use]
pub fn check(text: String) -> Option<TransportIdentity> {
    let phrase = RecoveryPhrase::parse(&text).ok();
    drop(text);
    phrase.and_then(|phrase| crate::identity::phrase_peer(&phrase).ok())
}

/// Restore the typed phrase to `expected` (R3's confirmed `PeerId`, or
/// the record's after K1), under the flow's `lock` as [`seal`].
///
/// # Errors
/// [`WriteRefused`], after the read-back.
pub fn restore(
    app_data_dir: &Path,
    profile: &str,
    cipher: &dyn SeedCipher,
    text: String,
    expected: Expected<'_>,
    lock: Option<&ProfileLock>,
) -> Result<(), WriteRefused> {
    let phrase = RecoveryPhrase::parse(&text).map_err(|_| WriteRefused::OtherIdentity)?;
    drop(text);
    match restore_identity(app_data_dir, profile, cipher, &phrase, expected, lock) {
        Ok(_) => Ok(()),
        // Published, then the directory's sync failed: done if the record
        // opens to the phrase's identity. (OldKeyRemains is success
        // already, in restore_identity.)
        Err(RestoreRefused::Unsynced(why)) => {
            let peer = crate::identity::phrase_peer(&phrase).ok();
            let opened = peer
                .zip(paths(app_data_dir, profile).ok())
                .is_some_and(|(peer, paths)| opens_to(&paths, cipher, &peer));
            if opened {
                Ok(())
            } else {
                Err(WriteRefused::Storage(why))
            }
        }
        Err(refused) => Err(match refused {
            RestoreRefused::Busy => WriteRefused::Busy,
            RestoreRefused::UserNotAuthenticated => WriteRefused::UserNotAuthenticated,
            RestoreRefused::OtherIdentity => WriteRefused::OtherIdentity,
            RestoreRefused::NoRecord
            | RestoreRefused::RecordExists
            | RestoreRefused::RecordUnreadable(_) => WriteRefused::NoRestore,
            RestoreRefused::Paths(why)
            | RestoreRefused::Unavailable(why)
            | RestoreRefused::Unsynced(why) => WriteRefused::Storage(why),
        }),
    }
}

fn marker(app_data_dir: &Path, profile: &str) -> Result<PathBuf, WriteRefused> {
    Ok(paths(app_data_dir, profile)?.human_dir().join(VOID_PAPER))
}

/// Whether a creation showed its words and never finished (W's
/// `IdVoidPaper`). A marker that cannot be read is read as unset: the
/// notice is a courtesy, never a gate.
#[must_use]
pub fn void_paper(app_data_dir: &Path, profile: &str) -> bool {
    marker(app_data_dir, profile).is_ok_and(|path| path.exists())
}

/// Set or clear the marker. It holds no byte of a word, seed or `PeerId`:
/// its existence is the whole of it.
///
/// # Errors
/// [`WriteRefused::Storage`].
pub fn set_void_paper(app_data_dir: &Path, profile: &str, set: bool) -> Result<(), WriteRefused> {
    let paths = paths(app_data_dir, profile)?;
    let path = paths.human_dir().join(VOID_PAPER);
    if set {
        interweave_profile_config::persist::create_private_dir_within(
            &paths.human_dir(),
            paths.boundary(),
        )
        .map_err(|e| WriteRefused::Storage(e.to_string()))?;
        interweave_profile_config::persist::write_private_atomic_within(
            &path,
            &[],
            paths.boundary(),
        )
        .map_err(|e| WriteRefused::Storage(e.to_string()))
    } else {
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(WriteRefused::Storage(e.to_string())),
        }
    }
}
