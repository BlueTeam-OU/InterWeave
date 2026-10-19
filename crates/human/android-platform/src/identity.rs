// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The profile's identity at a start (plan section 20 step 6): unwrapped
//! from its Keystore-wrapped record by the embedded custody path, through
//! the platform's cipher ([`SeedCipher`], the app's Keystore adapter) --
//! or why there is none, which the Service answers as a refusal to start
//! (android-key-custody.md, "Failure behavior").
//!
//! A refusal of the record never leads to a new identity: a key the
//! platform invalidated, a record that does not open or a `PeerId` that
//! does not match is [`IdentityRefused::RecoveryRequired`], where the app's
//! recovery flow (step 7) takes over, and the record is left as it was.
//!
//! A profile with no record at all is a first run. Creating its identity
//! or restoring one from a phrase is the person's choice in step 7's
//! onboarding, which does not exist yet. Until it does the caller says
//! which ([`FirstRun`]): the app's debug build (`dev-stand-ins`) creates
//! one and provisions it under the profile's configured unlock policy, so
//! the debug app keeps one `PeerId` across restarts, and its release build
//! answers [`IdentityRefused::Unprovisioned`].
//!
//! Recovery ([`restore_identity`], step 7's host side) restores from the
//! person's phrase only the `PeerId` it was told to expect, because
//! BIP-39's checksum is eight bits and a phrase that parses can be another
//! identity's (IDENTITY-RECOVERY.md, "Restore semantics"). Where a record
//! exists, the expected `PeerId` is the one it carries, read without the
//! cipher. On a fresh install there is none: the app shows the `PeerId`
//! the phrase derives to ([`phrase_peer`]), and what the person confirms
//! is what is expected. That restore only ever creates the record; it
//! never replaces one (architect-cto, 01a15277).

use std::path::Path;

use interweave_profile_config::{PersistError, ProfileLock, ProfilePaths, TrustBoundary};
use interweave_profile_identity::{ProfileIdentity, RecoveryPhrase};
use interweave_transport_api::TransportIdentity;
use interweave_transport_embedded::custody::{
    self, CustodyRefused, RecordName, RecoveryCause, SeedCipher, UnlockRefused,
};

/// Why a start has no identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityRefused {
    /// The profile's paths could not be resolved under the app data
    /// directory.
    Paths(String),
    /// No record, and this build creates none: step 7's onboarding is
    /// where the person creates or restores one.
    Unprovisioned,
    /// The record exists and cannot give its identity back: the app
    /// enters recovery. The record is untouched.
    RecoveryRequired(RecoveryCause),
    /// The key needs the person to authenticate: stay offline until they
    /// do.
    UserNotAuthenticated,
    /// The cipher or the storage failed for a reason that says nothing
    /// about the record: try again, decide nothing.
    Unavailable(String),
    /// The first run's provisioning wrote nothing, and why.
    Provisioning(String),
}

/// What a start does with a profile that has no record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRun {
    /// Create its identity and provision it under the profile's
    /// configured unlock policy: the debug build's stand-in for step 7's
    /// onboarding.
    Create,
    /// Answer [`IdentityRefused::Unprovisioned`]: creating or restoring
    /// is the person's choice, which step 7 asks for.
    Refuse,
}

/// The identity `profile` starts with, under `app_data_dir`, a profile
/// with no record being handled as `first_run` says.
///
/// # Errors
/// [`IdentityRefused`].
pub fn start_identity(
    app_data_dir: &Path,
    profile: &str,
    cipher: &dyn SeedCipher,
    first_run: FirstRun,
) -> Result<ProfileIdentity, IdentityRefused> {
    let paths = TrustBoundary::new(app_data_dir)
        .and_then(|boundary| ProfilePaths::resolve_embedded(profile, boundary))
        .map_err(|e| IdentityRefused::Paths(e.to_string()))?;
    match custody::unlock(&paths, cipher) {
        Ok(identity) => Ok(identity),
        Err(UnlockRefused::Unprovisioned) => match first_run {
            FirstRun::Create => create(&paths, cipher),
            FirstRun::Refuse => Err(IdentityRefused::Unprovisioned),
        },
        Err(UnlockRefused::RecoveryRequired(cause)) => {
            Err(IdentityRefused::RecoveryRequired(cause))
        }
        Err(UnlockRefused::UserNotAuthenticated) => Err(IdentityRefused::UserNotAuthenticated),
        Err(UnlockRefused::Unavailable(why)) => Err(IdentityRefused::Unavailable(why)),
    }
}

/// A first run's identity, created and provisioned. Custody's
/// `OldKeyRemains` comes AFTER the record is written: the profile is
/// provisioned and unlocks, and only a stale key of the other policy
/// remains for `reseal` to retire -- so it is this start's identity, not
/// a refusal. `Unsynced` comes after the write too, so the record is read
/// back: this identity if it opens to it.
fn create(
    paths: &ProfilePaths,
    cipher: &dyn SeedCipher,
) -> Result<ProfileIdentity, IdentityRefused> {
    let policy = interweave_profile_config::ProfileConfig::load(paths)
        .map_err(|e| IdentityRefused::Provisioning(format!("the profile: {e}")))?
        .runtime
        .android
        .key_unlock_policy;
    let identity = ProfileIdentity::generate();
    match custody::provision(paths, cipher, &identity, policy) {
        Ok(_) | Err(CustodyRefused::OldKeyRemains(_)) => Ok(identity),
        Err(refused @ CustodyRefused::Storage(PersistError::Unsynced(_))) => {
            let opened = custody::unlock(paths, cipher)
                .ok()
                .and_then(|opened| opened.transport_identity().ok());
            if opened.is_some() && opened == identity.transport_identity().ok() {
                Ok(identity)
            } else {
                Err(IdentityRefused::Provisioning(format!("{refused:?}")))
            }
        }
        Err(e) => Err(IdentityRefused::Provisioning(format!("{e:?}"))),
    }
}

/// Why a recovery restored nothing. In every case the record is left as
/// it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreRefused {
    /// The profile's paths could not be resolved under the app data
    /// directory.
    Paths(String),
    /// [`Expected::Recorded`], and no record names a `PeerId`: a fresh
    /// install, which restores only a confirmed one.
    NoRecord,
    /// [`Expected::Confirmed`] over a profile that has a record: an
    /// established identity is restored only as its record names it.
    RecordExists,
    /// The record is there but names no `PeerId`, or could not be read:
    /// an established profile's, never sealed over.
    RecordUnreadable(String),
    /// The phrase is a valid phrase of another identity, or the record
    /// names another than the phrase restores.
    OtherIdentity,
    /// The profile is held by a running host or another flow.
    Busy,
    /// The key needs the person to authenticate first.
    UserNotAuthenticated,
    /// The cipher or the storage failed for a reason that says nothing
    /// about the phrase: try again.
    Unavailable(String),
    /// The record was published and the directory's sync after it
    /// failed: it may be on disk. Only this, of the storage failures, is
    /// read back (design #303 round 3).
    Unsynced(String),
}

/// Which `PeerId` a restore must give back.
#[derive(Debug, Clone, Copy)]
pub enum Expected<'a> {
    /// The one the profile's record names: recovery after the platform
    /// lost or invalidated the key.
    Recorded,
    /// The one the person confirmed on a fresh install, after the app
    /// showed what the phrase derives to ([`phrase_peer`]).
    Confirmed(&'a TransportIdentity),
}

/// The `PeerId` `phrase` derives to, for the person to check against the
/// one they recorded before a fresh install's restore.
///
/// # Errors
/// [`RestoreRefused::Unavailable`] when the phrase gives no identity.
pub fn phrase_peer(phrase: &RecoveryPhrase) -> Result<TransportIdentity, RestoreRefused> {
    ProfileIdentity::from_phrase(phrase)
        .and_then(|identity| identity.transport_identity())
        .map_err(|e| RestoreRefused::Unavailable(e.to_string()))
}

/// Restore `profile`'s identity from `phrase`, under the profile's
/// configured unlock policy, only if the phrase gives back the `PeerId`
/// `expected` names. The Service must not be running. `lock` is the
/// recovery flow's, taken before its first word and lent to the seal,
/// which keeps it held whatever the outcome (android-key-custody.md,
/// Recovery); with none the restore takes the profile's lock itself.
///
/// # Errors
/// [`RestoreRefused`]; nothing is written in any of them.
pub fn restore_identity(
    app_data_dir: &Path,
    profile: &str,
    cipher: &dyn SeedCipher,
    phrase: &RecoveryPhrase,
    expected: Expected<'_>,
    lock: Option<&ProfileLock>,
) -> Result<ProfileIdentity, RestoreRefused> {
    let paths = TrustBoundary::new(app_data_dir)
        .and_then(|boundary| ProfilePaths::resolve_embedded(profile, boundary))
        .map_err(|e| RestoreRefused::Paths(e.to_string()))?;
    // The PeerId field, read without the cipher whatever the envelope's
    // state: a record refused for its shape still names whose it is.
    let recorded = match custody::named_identity(&paths)
        .map_err(|why| RestoreRefused::RecordUnreadable(format!("{why:?}")))?
    {
        RecordName::Absent => None,
        RecordName::Nobody => {
            return Err(RestoreRefused::RecordUnreadable(
                "the record names no PeerId".to_owned(),
            ));
        }
        RecordName::Names(peer) => Some(peer),
    };
    let expected = match (expected, recorded) {
        (Expected::Recorded, Some(peer)) => peer,
        (Expected::Recorded, None) => return Err(RestoreRefused::NoRecord),
        (Expected::Confirmed(peer), None) => peer.clone(),
        (Expected::Confirmed(_), Some(_)) => return Err(RestoreRefused::RecordExists),
    };
    let policy = interweave_profile_config::ProfileConfig::load(&paths)
        .map_err(|e| RestoreRefused::Unavailable(format!("the profile: {e}")))?
        .runtime
        .android
        .key_unlock_policy;
    let restored = match lock {
        Some(lock) => custody::restore_held(&paths, lock, cipher, phrase, &expected, policy),
        None => custody::restore(&paths, cipher, phrase, &expected, policy),
    };
    match restored {
        Ok(identity) => Ok(identity),
        // As for a first run: the record is written and opens, and only a
        // stale key of the other policy is left for `reseal` to retire.
        // The identity is the phrase's, which `restore` verified.
        Err(CustodyRefused::OldKeyRemains(_)) => ProfileIdentity::from_phrase(phrase)
            .map_err(|e| RestoreRefused::Unavailable(e.to_string())),
        Err(CustodyRefused::OtherIdentity | CustodyRefused::RecordNamesOther) => {
            Err(RestoreRefused::OtherIdentity)
        }
        Err(CustodyRefused::ProfileLocked) => Err(RestoreRefused::Busy),
        Err(CustodyRefused::Cipher(custody::CipherFailure::UserNotAuthenticated)) => {
            Err(RestoreRefused::UserNotAuthenticated)
        }
        Err(CustodyRefused::Unreadable(why)) => Err(RestoreRefused::RecordUnreadable(why)),
        Err(CustodyRefused::RecordNamesNobody) => Err(RestoreRefused::RecordUnreadable(
            "the record names no PeerId".to_owned(),
        )),
        Err(CustodyRefused::Storage(interweave_profile_config::PersistError::Unsynced(e))) => {
            Err(RestoreRefused::Unsynced(e.to_string()))
        }
        Err(other) => Err(RestoreRefused::Unavailable(format!("{other:?}"))),
    }
}
