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
//! onboarding ([`crate::onboarding`]), which the Activity enters on
//! [`IdentityRefused::Unprovisioned`]. The caller says what a start meets
//! with ([`FirstRun`]): the app's release build refuses, and its debug
//! build's Service (`dev-stand-ins`) creates one and provisions it under
//! the profile's configured unlock policy, so the instrumentation's cases
//! start with no person and keep one `PeerId` across restarts.
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

use interweave_profile_config::runtime::{self, AvailabilityMode, KeyUnlockPolicy};

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
    /// configured unlock policy: the debug build's Service, so a case
    /// starts with no person at the onboarding screens.
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
    open_identity(&embedded(app_data_dir, profile)?, cipher, first_run)
        .map(|(identity, _)| identity)
}

fn embedded(app_data_dir: &Path, profile: &str) -> Result<ProfilePaths, IdentityRefused> {
    TrustBoundary::new(app_data_dir)
        .and_then(|boundary| ProfilePaths::resolve_embedded(profile, boundary))
        .map_err(|e| IdentityRefused::Paths(e.to_string()))
}

/// [`start_identity`], and the policy of the record it opened, or of the
/// one a first run provisioned.
fn open_identity(
    paths: &ProfilePaths,
    cipher: &dyn SeedCipher,
    first_run: FirstRun,
) -> Result<(ProfileIdentity, KeyUnlockPolicy), IdentityRefused> {
    match custody::unlock_opened(paths, cipher) {
        Ok(opened) => Ok(opened),
        Err(UnlockRefused::Unprovisioned) => match first_run {
            FirstRun::Create => create(paths, cipher),
            FirstRun::Refuse => Err(IdentityRefused::Unprovisioned),
        },
        Err(UnlockRefused::RecoveryRequired(cause)) => {
            Err(IdentityRefused::RecoveryRequired(cause))
        }
        Err(UnlockRefused::UserNotAuthenticated) => Err(IdentityRefused::UserNotAuthenticated),
        Err(UnlockRefused::Unavailable(why)) => Err(IdentityRefused::Unavailable(why)),
    }
}

/// What a start's reseal came to ([`reseal_identity`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resealed {
    /// The record was already under the configured policy, and no key of
    /// the other policy remains.
    Unchanged,
    /// The record was sealed again under the configured policy, and the
    /// old policy's key deleted.
    Changed,
    /// Another holder has the profile, so nothing was tried: not a
    /// reseal's outcome, since the start's own take of the lock answers
    /// it (ADR-0042 A 2026-11-05).
    Held,
    /// The reseal did not finish. The record still opens to the identity
    /// the start unlocked -- the old one, or the new one written with the
    /// old policy's key left -- and the next start tries again.
    Pending {
        /// Which kind of fault, for the status the person can see.
        why: ResealFailure,
        /// The refusal, for the log only.
        detail: String,
    },
}

/// Why a reseal did not finish, by class (ADR-0042 A 2026-11-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResealFailure {
    /// The Keystore could not seal under the configured policy.
    Keystore,
    /// The new record could not be written, or its directory not synced.
    Storage,
    /// The record is under the configured policy, and the key of the
    /// other policy could not be deleted.
    OldKeyRemains,
    /// Anything else: the profile unreadable, the record changed under
    /// the start.
    Other,
}

/// The unlock policy a start left in force, beside the configured one
/// and a reseal that did not finish: the status member the person's view
/// reads (ADR-0042 A 2026-11-05; design 2026-11-05-unlock-policy-pending.md
/// section 3). A change is pending wherever `in_force` and `configured`
/// differ, whatever `pending` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnlockStatus {
    /// The policy of the record on disk when the start's custody step
    /// ended: the configured one when the reseal wrote the record (sealed
    /// again, `OldKeyRemains`, an unsynced write), else the record the
    /// start opened; on a locked start, the record header's claim
    /// ([`locked_status`]).
    pub in_force: KeyUnlockPolicy,
    /// The configured `key_unlock_policy`; `None` when the profile could
    /// not be read.
    pub configured: Option<KeyUnlockPolicy>,
    /// Why the start's reseal did not finish, if it did not.
    pub pending: Option<ResealFailure>,
}

impl UnlockStatus {
    /// The status after a start that opened a record under `opened` and
    /// resealed it as `resealed` against `configured`.
    #[must_use]
    pub fn after_start(
        opened: KeyUnlockPolicy,
        configured: Option<KeyUnlockPolicy>,
        resealed: &Resealed,
    ) -> Self {
        let wrote = match resealed {
            Resealed::Unchanged | Resealed::Changed => true,
            Resealed::Pending { why, .. } => *why == ResealFailure::OldKeyRemains,
            Resealed::Held => false,
        };
        let pending = match resealed {
            Resealed::Pending { why, .. } => Some(*why),
            Resealed::Unchanged | Resealed::Changed | Resealed::Held => None,
        };
        Self {
            in_force: configured.filter(|_| wrote).unwrap_or(opened),
            configured,
            pending,
        }
    }

    /// `background_restart_requires_user_authentication` (human-client-ui.md
    /// section 14) under the `effective` mode: profile-config's rule, given
    /// the policy in force and never the configured one (ADR-0041 A
    /// 2026-11-05).
    #[must_use]
    pub fn restart_requires_user_authentication(&self, effective: AvailabilityMode) -> bool {
        runtime::background_restart_requires_user_authentication(effective, self.in_force)
    }
}

/// A start's identity, what its reseal came to, and the status it leaves.
#[derive(Debug)]
pub struct Started {
    /// The identity the runtime starts with.
    pub identity: ProfileIdentity,
    /// What the reseal did, for the log.
    pub resealed: Resealed,
    /// The status member's value for this start.
    pub unlock: UnlockStatus,
}

/// The service's start: [`start_identity`], then [`reseal_identity`], one
/// reading of the configuration serving both the reseal and the status.
///
/// # Errors
/// [`IdentityRefused`], as the two do.
pub fn start_resealed(
    app_data_dir: &Path,
    profile: &str,
    cipher: &dyn SeedCipher,
    first_run: FirstRun,
) -> Result<Started, IdentityRefused> {
    let paths = embedded(app_data_dir, profile)?;
    let (identity, opened) = open_identity(&paths, cipher, first_run)?;
    let configured = configured(&paths);
    let known = configured.as_ref().ok().copied();
    let resealed = reseal(&paths, cipher, &identity, configured)?;
    let unlock = UnlockStatus::after_start(opened, known, &resealed);
    Ok(Started {
        identity,
        resealed,
        unlock,
    })
}

/// The status of a start refused [`IdentityRefused::UserNotAuthenticated`]:
/// nothing opened, so the policy in force is the record header's byte,
/// read before any cipher call (ADR-0041 A 2026-11-05) -- a claim the open
/// would authenticate, and the user-presence key's alone refuses so.
/// Nothing was tried, so what is pending is `last`'s, the last start that
/// ran, while the record is still under the policy it left in force: the
/// record has not changed since, so neither has why (design
/// 2026-11-05-unlock-policy-pending.md section 3). `None` when the record
/// cannot be read or names no policy.
#[must_use]
pub fn locked_status(
    app_data_dir: &Path,
    profile: &str,
    last: Option<UnlockStatus>,
) -> Option<UnlockStatus> {
    let paths = embedded(app_data_dir, profile).ok()?;
    let in_force = custody::recorded_policy(&paths).ok()??;
    Some(UnlockStatus {
        in_force,
        configured: configured(&paths).ok(),
        pending: last
            .filter(|last| last.in_force == in_force)
            .and_then(|last| last.pending),
    })
}

/// Whether this start's reseal will need the person: the configured
/// policy is user-presence and the record is not yet under it, so sealing
/// it again takes the user-presence key, which the Keystore refuses
/// outside the presence window. The Activity's check asks it before the
/// start, so the prompt comes first and the start runs in the window
/// (ADR-0042 A 2026-11-05: the prompt, then a retried start). Reads only.
#[must_use]
pub fn reseal_needs_presence(app_data_dir: &Path, profile: &str) -> bool {
    let Ok(paths) = embedded(app_data_dir, profile) else {
        return false;
    };
    configured(&paths) == Ok(KeyUnlockPolicy::UserPresence)
        && matches!(custody::recorded_policy(&paths), Ok(Some(recorded)) if recorded != KeyUnlockPolicy::UserPresence)
}

/// Whether opening `profile`'s record needs the person: the policy the
/// record is under, its header's byte, since the open uses that policy's
/// key whatever is configured (ADR-0042 A 2026-10-10 (ii)) -- so while a
/// change is pending the configured policy would prompt for nothing, or
/// let an open through that the key then refuses. With no record to read,
/// the configured policy, which a first run seals under.
#[must_use]
pub fn opening_needs_presence(app_data_dir: &Path, profile: &str) -> bool {
    let Ok(paths) = embedded(app_data_dir, profile) else {
        return false;
    };
    let policy = match custody::recorded_policy(&paths) {
        Ok(Some(recorded)) => Some(recorded),
        Ok(None) | Err(_) => configured(&paths).ok(),
    };
    policy == Some(KeyUnlockPolicy::UserPresence)
}

/// [`UnlockStatus::restart_requires_user_authentication`] with no host
/// running: the effective mode read from disk
/// (`availability_overlay::effective_availability_mode`), as a locked
/// start's diagnostic needs it (ADR-0041 A 2026-11-05). `None` when the
/// mode cannot be read.
#[must_use]
pub fn restart_requires_user_authentication_at_rest(
    app_data_dir: &Path,
    profile: &str,
    unlock: &UnlockStatus,
) -> Option<bool> {
    let paths = embedded(app_data_dir, profile).ok()?;
    let mode = interweave_profile_config::availability_overlay::effective_availability_mode(&paths)
        .ok()?;
    Some(unlock.restart_requires_user_authentication(mode))
}

/// Bring `profile`'s record to its configured `key_unlock_policy` after
/// [`start_identity`] unlocked `identity`, and delete the key of the
/// other policy if it is still there (ADR-0042 A 2026-10-10 (ii); an
/// `OldKeyRemains` of a provision or restore is retried here, design
/// 2026-10-19-android-identity.md section 6). Called by the service's
/// start before the host takes the profile's lock, never by a reading of
/// whether the identity opens: it writes.
///
/// Only a refusal the person can answer stops the start: the new
/// policy's key needs them present
/// ([`IdentityRefused::UserNotAuthenticated`], answered by the prompt and
/// a retried start, ADR-0042 A 2026-10-20). Every other refusal leaves
/// the record opening to `identity`, so the start goes on with it and
/// the next start retries ([`Resealed::Pending`]; ADR-0042 A 2026-11-05).
///
/// # Errors
/// [`IdentityRefused::Paths`] and [`IdentityRefused::UserNotAuthenticated`].
pub fn reseal_identity(
    app_data_dir: &Path,
    profile: &str,
    cipher: &dyn SeedCipher,
    identity: &ProfileIdentity,
) -> Result<Resealed, IdentityRefused> {
    let paths = embedded(app_data_dir, profile)?;
    reseal(&paths, cipher, identity, configured(&paths))
}

/// The profile's configured unlock policy, or why it could not be read.
fn configured(paths: &ProfilePaths) -> Result<KeyUnlockPolicy, String> {
    interweave_profile_config::ProfileConfig::load(paths)
        .map(|config| config.runtime.android.key_unlock_policy)
        .map_err(|e| format!("the profile: {e}"))
}

fn reseal(
    paths: &ProfilePaths,
    cipher: &dyn SeedCipher,
    identity: &ProfileIdentity,
    configured: Result<KeyUnlockPolicy, String>,
) -> Result<Resealed, IdentityRefused> {
    let policy = match configured {
        Ok(policy) => policy,
        Err(detail) => {
            return Ok(Resealed::Pending {
                why: ResealFailure::Other,
                detail,
            });
        }
    };
    let refused = match custody::reseal(paths, cipher, identity, policy) {
        Ok(false) => return Ok(Resealed::Unchanged),
        Ok(true) => return Ok(Resealed::Changed),
        Err(CustodyRefused::Cipher(custody::CipherFailure::UserNotAuthenticated)) => {
            return Err(IdentityRefused::UserNotAuthenticated);
        }
        Err(CustodyRefused::ProfileLocked) => return Ok(Resealed::Held),
        Err(refused) => refused,
    };
    let why = match refused {
        CustodyRefused::Cipher(_) | CustodyRefused::CipherShape => ResealFailure::Keystore,
        // An unsynced write is in place, under the configured policy, with
        // the old policy's key not retired: what `OldKeyRemains` says, and
        // what the status shows (design 2026-11-05-unlock-policy-pending.md
        // section 3). Asked of the record rather than assumed: a `Storage`
        // refusal means nothing was written, but for an unsynced write
        // whose record could not be read back, which stays `Storage`.
        CustodyRefused::Storage(PersistError::Unsynced(_)) if matches!(custody::recorded_policy(paths), Ok(Some(p)) if p == policy) => {
            ResealFailure::OldKeyRemains
        }
        CustodyRefused::Storage(_) => ResealFailure::Storage,
        CustodyRefused::OldKeyRemains(_) => ResealFailure::OldKeyRemains,
        // The record changed or vanished between the unlock and the
        // reseal, or the identity gave no seed; listed rather than
        // matched by `_`, so a new refusal is classed by a decision.
        CustodyRefused::NotProvisioned
        | CustodyRefused::RecordNamesOther
        | CustodyRefused::RecordNamesNobody
        | CustodyRefused::Unreadable(_)
        | CustodyRefused::Identity(_)
        | CustodyRefused::AlreadyProvisioned
        | CustodyRefused::ProfileLocked
        | CustodyRefused::LockOfAnotherProfile
        | CustodyRefused::OtherIdentity => ResealFailure::Other,
    };
    Ok(Resealed::Pending {
        why,
        detail: format!("{refused:?}"),
    })
}

/// A first run's identity, created and provisioned. Custody's
/// `OldKeyRemains` comes AFTER the record is written: the profile is
/// provisioned and unlocks, and only a stale key of the other policy
/// remains for the next start's [`reseal_identity`] to retire -- so it is
/// this start's identity, not a refusal. `Unsynced` comes after the write too, so the record is read
/// back: this identity if it opens to it.
fn create(
    paths: &ProfilePaths,
    cipher: &dyn SeedCipher,
) -> Result<(ProfileIdentity, KeyUnlockPolicy), IdentityRefused> {
    let policy = interweave_profile_config::ProfileConfig::load(paths)
        .map_err(|e| IdentityRefused::Provisioning(format!("the profile: {e}")))?
        .runtime
        .android
        .key_unlock_policy;
    let identity = ProfileIdentity::generate();
    match custody::provision(paths, cipher, &identity, policy) {
        Ok(_) | Err(CustodyRefused::OldKeyRemains(_)) => Ok((identity, policy)),
        Err(refused @ CustodyRefused::Storage(PersistError::Unsynced(_))) => {
            let opened = custody::unlock(paths, cipher)
                .ok()
                .and_then(|opened| opened.transport_identity().ok());
            if opened.is_some() && opened == identity.transport_identity().ok() {
                Ok((identity, policy))
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
        // stale key of the other policy is left for the next start's
        // `reseal_identity` to retire.
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
