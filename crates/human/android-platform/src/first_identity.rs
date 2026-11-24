// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The phone's first identity: its endpoint key sealed and the app's
//! profile document moved from form 1 to form 2 (M3), at onboarding and
//! at every start (`ENDPOINTS.md` §Configured endpoint set, the two forms
//! and the three moves; ADR-0067 decision 4; ADR-0063 decision 1).
//!
//! No endpoint key exists before the person creates or restores the node,
//! so form 1 names no identity and the embedded host's start refuses it
//! (`AndroidNoIdentity`). [`finish`] is the app's half of M3: on form 1
//! beside a sealed seed it seals the first identity's key -- re-sealed
//! from an earlier release's `identity-keys/human.key`, or minted where
//! `endpoint-keys/` holds none -- and writes form 2 naming the id that key
//! derives. A form-1 document beside a sealed key and no earlier
//! release's is left as it is (`a_sealed_key_beside_form_1_is_left_to_the_host`): the embedded host's start finishes M3 with
//! that key's id and mints nothing (the seam agreed with p2p-network-dev-01:
//! the key file is the handoff, nothing is handed in memory).
//!
//! Every key is written by human-store's `identity_key` (its exclusive
//! create and its sync); an error here names a kind or a path's refusal,
//! and holds nothing of a key.

use std::time::Duration;

use interweave_human_store::identity_key::{self, IdentityKeyFileError, KeyCipher};
use interweave_profile_config::provision::{
    FIRST_IDENTITY_ALIAS, embedded_document, provision_identity,
};
use interweave_profile_config::{
    MAX_PROFILE_BYTES, PersistError, ProfileLock, ProfilePaths, read_private_within,
};
use interweave_transport_api::EndpointId;
use interweave_transport_embedded::custody::{self, RecordName};

/// What [`finish`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finished {
    /// The document is not form 1 -- form 2, or one this module does not
    /// move: nothing was read of a key and nothing written.
    NotForm1,
    /// Form 1 and no seed record: a first run before Create or Restore,
    /// so no key was minted.
    NoSeed,
    /// The earlier release's key was re-sealed under the id it derives,
    /// read back, its old file and platform key retired, and form 2
    /// written naming that id.
    Resealed(EndpointId),
    /// A new key was minted and sealed, and form 2 written naming its id.
    Minted(EndpointId),
    /// Form 1 beside a sealed key and no earlier release's: nothing was
    /// written, and the embedded host's start finishes M3 with that key.
    LeftToHost,
    /// Another holder has the profile's lock: nothing was tried, and the
    /// start's own take of the lock answers it.
    Held,
}

/// Why [`finish`] stopped, by kind; whatever it stopped before is done by
/// the next start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstIdentityRefused {
    /// The app's data directory or the profile's paths were refused.
    Paths(String),
    /// The profile's lock could not be taken for another reason than a
    /// holder.
    Lock(String),
    /// The profile document could not be read.
    Document(String),
    /// The seed record could not be read, so whether the node exists is
    /// unknown.
    Seed(String),
    /// An endpoint key could not be listed, opened, made or retired.
    Key(IdentityKeyFileError),
    /// The re-sealed key's file was not there to read back: the earlier
    /// release's file is kept. (One that opens to another key is
    /// [`Self::Key`], the open's refusal.)
    ReadBack,
    /// The form-2 write was refused (`ProvisionIdentityError`, said); the
    /// key it would name is sealed.
    Provision(String),
}

impl From<IdentityKeyFileError> for FirstIdentityRefused {
    fn from(e: IdentityKeyFileError) -> Self {
        Self::Key(e)
    }
}

/// M3 for `paths`' profile under `cipher`'s custody, under `held` -- the
/// caller's lock on the profile -- or, with none, under a lock taken here
/// without waiting and released on return, whatever it returns
/// (`the_lock_finish_takes_is_released_on_every_path`). See the
/// module for what it does on each document.
///
/// # Errors
/// [`FirstIdentityRefused`]; what was done before it stays, and each step
/// is one a rerun finishes.
pub fn finish(
    paths: &ProfilePaths,
    cipher: &dyn KeyCipher,
    held: Option<&ProfileLock>,
) -> Result<Finished, FirstIdentityRefused> {
    // Dropped at return, on every path: the lock is the file's flock.
    let _own = match held {
        Some(_) => None,
        None => match ProfileLock::acquire(paths, Duration::ZERO) {
            Ok(lock) => Some(lock),
            Err(PersistError::ProfileLocked { .. }) => return Ok(Finished::Held),
            Err(e) => return Err(FirstIdentityRefused::Lock(e.to_string())),
        },
    };
    let text = read_private_within(&paths.config_file(), paths.boundary(), MAX_PROFILE_BYTES)
        .map_err(|e| FirstIdentityRefused::Document(e.to_string()))?;
    if text.as_deref() != Some(embedded_document(paths.profile()).as_str()) {
        return Ok(Finished::NotForm1);
    }
    match custody::named_identity(paths) {
        Ok(RecordName::Absent) => return Ok(Finished::NoSeed),
        Ok(RecordName::Nobody | RecordName::Names(_)) => {}
        Err(e) => return Err(FirstIdentityRefused::Seed(format!("{e:?}"))),
    }
    if let Some(old) = identity_key::open_legacy(paths, FIRST_IDENTITY_ALIAS, cipher)? {
        match identity_key::install(paths, &old, cipher) {
            // Exists: a re-seal interrupted after its write, which the
            // read-back below judges.
            Ok(()) | Err(IdentityKeyFileError::Exists) => {}
            Err(e) => return Err(e.into()),
        }
        let id = old.endpoint().clone();
        // Read back before the earlier file goes: the open refuses a file
        // at the id that holds another key (its header must derive the
        // id), so the key it opens is the earlier one
        // (`a_reseal_whose_read_back_fails_keeps_the_earlier_key`).
        if identity_key::open(paths, &id, cipher)?.is_none() {
            return Err(FirstIdentityRefused::ReadBack);
        }
        identity_key::remove_legacy(paths, FIRST_IDENTITY_ALIAS, cipher)?;
        provision_identity(paths, &id)
            .map_err(|e| FirstIdentityRefused::Provision(e.to_string()))?;
        return Ok(Finished::Resealed(id));
    }
    if !identity_key::sealed(paths)?.is_empty() {
        return Ok(Finished::LeftToHost);
    }
    let id = identity_key::create(paths, cipher)?.endpoint().clone();
    provision_identity(paths, &id).map_err(|e| FirstIdentityRefused::Provision(e.to_string()))?;
    Ok(Finished::Minted(id))
}

/// [`finish`] for a start, which resolves the profile's paths under the
/// app's data directory and holds no lock of its own yet.
///
/// # Errors
/// As [`finish`].
pub fn at_start(
    app_data_dir: &std::path::Path,
    profile: &str,
    cipher: &dyn KeyCipher,
) -> Result<Finished, FirstIdentityRefused> {
    let paths = interweave_profile_config::TrustBoundary::new(app_data_dir)
        .and_then(|boundary| ProfilePaths::resolve_embedded(profile, boundary))
        .map_err(|e| FirstIdentityRefused::Paths(e.to_string()))?;
    finish(&paths, cipher, None)
}
