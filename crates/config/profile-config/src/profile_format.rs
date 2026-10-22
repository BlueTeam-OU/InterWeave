// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The profile-format marker (ADR-0028 A 2026-10-20):
//! `<state>/profile-format.json`, `{"profile_format": N}`, the one field
//! that lets a build refuse a profile a newer build wrote.
//!
//! A strict format refuses a newer version of a file it knows; it cannot
//! refuse a file it has never heard of. Measured on the previous-build
//! axis: the Stage 13 build, started on a later build's tree, ignored
//! the trust and endpoint overlays, so a revoked peer was allowed again
//! with nothing on screen. The marker is that rule in one number.
//!
//! - [`require_supported`] runs first in [`crate::ProfileConfig::load`],
//!   so every loader -- the daemon, `transportctl`, the desktop client,
//!   the embedded host -- refuses a marker above [`PROFILE_FORMAT`]
//!   before it takes the lock, rewrites an overlay or creates a
//!   directory. It writes nothing, and judges the file, not the state
//!   directory, which stays the profile lock's to refuse ([`read_file`]).
//! - [`raise`] is called by the profile's state writer, under the lock,
//!   before it serves: the daemon, and on Android the embedded host. It
//!   writes [`PROFILE_FORMAT`] when the marker is below it and never
//!   lowers it.
//! - An absent file reads as 0, so a profile no marked build has served
//!   is accepted by every build. Deleting the marker removes the
//!   protection; it is backed up with the profile and never deleted to
//!   "reset".
//!
//! The file is judged as the overlays' are -- this uid's, readable or
//! writable by nobody else, not a link, a regular file, exactly the one
//! shape -- and a present file that fails is refused, never read as 0:
//! read as 0, a stranger's file or a damaged one would turn the
//! protection off.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::private_read::{self, PrivateReadError};
use crate::{PersistError, ProfilePaths, TrustBoundary, persist};

/// The marker's file name in the profile's state directory.
pub const PROFILE_FORMAT_FILE: &str = "profile-format.json";

/// This build's profile format. Raised by the change that adds an
/// artefact an older build would silently ignore -- a new overlay, a new
/// strictly-parsed file, a new member a strict format would not refuse
/// -- in that same change (ADR-0028 A 2026-10-20).
///
/// 1: the marker itself, written first by the build that carries the
/// trust overlay, the endpoint overlay and the availability overlay.
/// 2: the bootstrap-peers overlay (ADR-0062), which a build at 1 would
/// ignore, starting on a profile holding added peers and dialling none.
/// 3: the human store encrypted at rest (ADR-0060 decision 6, A
/// 2026-10-21): a build before it would read the encrypted store as a
/// corrupt file, so it refuses the profile by this marker instead.
pub const PROFILE_FORMAT: u32 = 3;

/// The most a marker may hold on disk; its one shape is a few dozen
/// bytes, and anything past this is refused before it is parsed.
pub const MAX_PROFILE_FORMAT_BYTES: u64 = 4096;

/// The file's one shape.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OnDisk {
    profile_format: u32,
}

/// Why the marker refuses this build, or cannot be read or raised.
#[derive(Debug)]
pub enum ProfileFormatError {
    /// The marker names a format above this build's: a newer build
    /// wrote this profile.
    Newer {
        /// The marker.
        path: PathBuf,
        /// The format it names.
        found: u32,
        /// This build's.
        own: u32,
    },
    /// The file is present and could not be read.
    Read(std::io::Error),
    /// The file, or its directory, is not private to this uid, or the
    /// file is a symbolic link or not a regular file.
    NotPrivate {
        /// What is wrong with it.
        detail: String,
    },
    /// The file is larger than [`MAX_PROFILE_FORMAT_BYTES`].
    TooLarge,
    /// The file is not the marker's one shape.
    Parse(String),
    /// The raised marker could not be written.
    Write(PersistError),
}

impl core::fmt::Display for ProfileFormatError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Newer { path, found, own } => write!(
                f,
                "{} says profile format {found}, newer than this build's {own}: a newer build \
                 wrote this profile; start that build or a later one, never an older one",
                path.display()
            ),
            Self::Read(e) => write!(f, "the profile-format marker cannot be read: {e}"),
            Self::NotPrivate { detail } => {
                write!(f, "the profile-format marker is not private: {detail}")
            }
            Self::TooLarge => write!(
                f,
                "the profile-format marker is larger than {MAX_PROFILE_FORMAT_BYTES} bytes"
            ),
            Self::Parse(e) => write!(f, "the profile-format marker does not parse: {e}"),
            Self::Write(e) => write!(f, "the profile-format marker cannot be written: {e}"),
        }
    }
}

impl core::error::Error for ProfileFormatError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Read(e) => Some(e),
            Self::Write(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PrivateReadError> for ProfileFormatError {
    fn from(e: PrivateReadError) -> Self {
        match e {
            PrivateReadError::Read(e) => Self::Read(e),
            PrivateReadError::NotPrivate { detail } => Self::NotPrivate { detail },
            PrivateReadError::TooLarge => Self::TooLarge,
        }
    }
}

/// The marker's path for `paths`' profile.
#[must_use]
pub fn path_for(paths: &ProfilePaths) -> PathBuf {
    paths.state_dir().join(PROFILE_FORMAT_FILE)
}

/// The format the marker at `path` names, its state directory judged
/// private under `boundary` as the overlays' is: 0 when it is absent,
/// its state directory included.
///
/// # Errors
/// Every [`ProfileFormatError`] but `Newer` and `Write`.
pub fn read_within(path: &Path, boundary: &TrustBoundary) -> Result<u32, ProfileFormatError> {
    parse(private_read::read_private_within(
        path,
        boundary,
        MAX_PROFILE_FORMAT_BYTES,
    )?)
}

/// The format the marker at `path` names, the FILE judged -- this uid's,
/// owner-only, not a link, a regular file, its one shape -- and its
/// directory left to the profile lock: 0 when it is absent, and when the
/// path to it refuses the lookup (a directory this uid cannot search, a
/// file where a directory should be). What [`require_supported`] reads,
/// so a load refuses a newer build's marker wherever it lies, a present
/// marker it cannot trust or open is refused as the file's, and a state
/// directory any other account can read, or this uid cannot search, is
/// refused by the lock that follows, as before the marker, rather than
/// reported as a marker error.
///
/// # Errors
/// Every [`ProfileFormatError`] but `Newer` and `Write`.
pub fn read_file(path: &Path) -> Result<u32, ProfileFormatError> {
    parse(private_read::read_private_file(
        path,
        MAX_PROFILE_FORMAT_BYTES,
    )?)
}

fn parse(text: Option<String>) -> Result<u32, ProfileFormatError> {
    let Some(text) = text else {
        return Ok(0);
    };
    let OnDisk { profile_format } =
        private_read::object_only(&text).map_err(ProfileFormatError::Parse)?;
    Ok(profile_format)
}

/// `found`, the marker at `path`'s format, refused when it is above
/// `own`.
fn at_most(path: &Path, found: u32, own: u32) -> Result<u32, ProfileFormatError> {
    if found > own {
        return Err(ProfileFormatError::Newer {
            path: path.to_owned(),
            found,
            own,
        });
    }
    Ok(found)
}

/// The format the marker at `path` names, its directory judged as
/// [`read_within`] judges it, refused when it is above `own`. Writes
/// nothing.
///
/// # Errors
/// [`ProfileFormatError::Newer`], and every error of [`read_within`].
pub fn check_within(
    path: &Path,
    boundary: &TrustBoundary,
    own: u32,
) -> Result<u32, ProfileFormatError> {
    at_most(path, read_within(path, boundary)?, own)
}

/// Raise the marker at `path` to `own`: written whole (a temporary file,
/// owner-only from creation, renamed into place) when it names less,
/// left untouched when it names `own`, refused when it names more -- a
/// build never lowers it. The caller holds the profile lock, so it is
/// the only writer.
///
/// # Errors
/// Every error of [`check_within`]; [`ProfileFormatError::Write`]. A
/// failure before the rename leaves the previous marker as it was; a
/// [`PersistError::Unsynced`] leaves the new one in place, its name
/// perhaps not durable.
pub fn raise_within(
    path: &Path,
    boundary: &TrustBoundary,
    own: u32,
) -> Result<(), ProfileFormatError> {
    if check_within(path, boundary, own)? == own {
        return Ok(());
    }
    let text = serde_json::to_vec_pretty(&OnDisk {
        profile_format: own,
    })
    .map_err(|e| ProfileFormatError::Write(PersistError::Io(std::io::Error::other(e))))?;
    persist::write_private_atomic_within(path, &text, boundary).map_err(ProfileFormatError::Write)
}

/// Refuse `paths`' profile when a newer build wrote it: the check every
/// loader makes, through [`crate::ProfileConfig::load`]. The marker read
/// by [`read_file`]: its directory is the profile lock's to judge.
///
/// # Errors
/// [`ProfileFormatError::Newer`], and every error of [`read_file`].
pub fn require_supported(paths: &ProfilePaths) -> Result<u32, ProfileFormatError> {
    let path = path_for(paths);
    at_most(&path, read_file(&path)?, PROFILE_FORMAT)
}

/// Raise `paths`' marker to [`PROFILE_FORMAT`]: the state writer's step,
/// under the profile lock, before it serves.
///
/// # Errors
/// As [`raise_within`] with [`PROFILE_FORMAT`].
pub fn raise(paths: &ProfilePaths) -> Result<(), ProfileFormatError> {
    raise_within(&path_for(paths), paths.boundary(), PROFILE_FORMAT)
}
