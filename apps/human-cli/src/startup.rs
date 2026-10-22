// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the command line needs before a verb runs: the profile's paths,
//! the endpoints a human client may lease and its channels, by the rule
//! the desktop client uses (`interweave_human_app_core::leasable_endpoints`);
//! the instance lock, so this client and the desktop one never hold the
//! store at once; and the store, opened or refused by class.

use std::path::PathBuf;
use std::time::Duration;

use interweave_human_app_core::leasable_endpoints;
use interweave_human_store::{HumanStore, STORE_FILE, StoreError, StoreOptions};
use interweave_profile_config::{
    HumanClientLock, PersistError, ProfileConfig, ProfilePaths, XdgRoots,
};
use interweave_transport_api::{ChannelId, EndpointId};

use crate::Failure;

/// The client kind this client leases under: the human client's, the
/// same as the desktop's, since the profile's endpoints allow that kind.
pub(crate) use interweave_human_app_core::CLIENT_KIND;

/// A resolved profile.
#[derive(Debug, Clone)]
pub(crate) struct Profile {
    /// Its paths.
    pub(crate) paths: ProfilePaths,
    /// The endpoints this client may lease, in the file's order: every
    /// entry that allows the `human-client` kind, whatever `enabled` says
    /// (the daemon's effective state decides at open).
    pub(crate) endpoints: Vec<EndpointId>,
    /// The channels it joins.
    pub(crate) channels: Vec<ChannelId>,
    /// The daemon's data socket.
    pub(crate) data_socket: PathBuf,
    /// The daemon's admin socket.
    pub(crate) admin_socket: PathBuf,
}

impl Profile {
    /// The store's file.
    pub(crate) fn store_path(&self) -> PathBuf {
        self.paths.human_dir().join(STORE_FILE)
    }
}

/// An invalid profile NAME is said without the name: it is argv.
fn paths_refusal(e: &PersistError) -> Failure {
    match e {
        PersistError::InvalidProfileName { .. } => {
            Failure::Refused("the profile name is not a valid one".to_owned())
        }
        other => Failure::Refused(format!("the profile's paths: {other}")),
    }
}

/// Resolve `profile`'s paths, endpoints and channels. Nothing is created.
///
/// # Errors
/// [`Failure::Refused`] naming the step that refused.
pub(crate) fn resolve(profile: &str) -> Result<Profile, Failure> {
    let roots =
        XdgRoots::from_env().map_err(|e| Failure::Refused(format!("the XDG directories: {e}")))?;
    let paths = ProfilePaths::resolve(profile, &roots).map_err(|e| paths_refusal(&e))?;
    if !paths.roles_are_distinct() {
        return Err(Failure::Refused(
            "the profile's configuration, state and runtime directories overlap".to_owned(),
        ));
    }
    let config = ProfileConfig::load(&paths)
        .map_err(|e| Failure::Refused(format!("the profile's configuration: {e}")))?;
    let endpoints = leasable_endpoints(&config);
    if endpoints.is_empty() {
        return Err(Failure::Refused(format!(
            "profile {:?} configures no endpoint a human client may use",
            paths.profile()
        )));
    }
    let socket = |e: PersistError| Failure::Refused(format!("the daemon's socket paths: {e}"));
    Ok(Profile {
        endpoints,
        channels: config.channels.desired.clone(),
        data_socket: paths.data_socket().map_err(socket)?,
        admin_socket: paths.admin_socket().map_err(socket)?,
        paths,
    })
}

/// Take the human client's instance lock, at once: a second client of the
/// store -- this one again, or the desktop -- is refused, never waited on.
///
/// # Errors
/// [`Failure::Locked`] when another client holds it, else
/// [`Failure::Refused`].
pub(crate) fn lock(profile: &Profile) -> Result<HumanClientLock, Failure> {
    HumanClientLock::acquire(&profile.paths, Duration::ZERO).map_err(|e| match e {
        PersistError::InstanceLocked { .. } => Failure::Locked(format!(
            "another client holds profile {:?}'s message store; close it and try again",
            profile.paths.profile()
        )),
        other => Failure::Refused(format!("the instance lock: {other}")),
    })
}

/// Open the store, under the lock the caller holds, by the desktop's
/// classes: recovery, not private, or not now. Nothing here touches the
/// file but the store's open, which leaves a file needing recovery where
/// it was (`crates/human/store/tests/storage_failure.rs`,
/// `a_migration_that_fails_part_way_leaves_the_file_at_its_old_version_with_every_row`).
///
/// # Errors
/// [`Failure::Store`] saying which.
pub(crate) fn open_store(profile: &Profile) -> Result<HumanStore, Failure> {
    let path = profile.store_path();
    // The store key at the v1 desktop's custody (ADR-0060 decision 3), as
    // the desktop's: a filesystem custody reports no invalidation.
    HumanStore::open_profile(
        &profile.paths,
        &interweave_human_store::key_custody::FilesystemCustody,
        StoreOptions::default(),
    )
    .map_err(|error| {
        Failure::Store(
            if error.needs_recovery() || matches!(error, StoreError::KeyInvalidated) {
                format!(
                    "the message store at {} cannot be read by this version and needs recovery; \
                 it was not renamed, moved or deleted",
                    path.display()
                )
            } else if matches!(
                error,
                StoreError::NotAFile { .. }
                    | StoreError::PermissionsTooOpen { .. }
                    | StoreError::DirectoryNotPrivate { .. }
            ) {
                format!(
                    "the message store at {} or its directory is not private to this user; \
                 refusing to use it: {error}",
                    path.display()
                )
            } else {
                format!(
                    "the message store at {} could not be opened now: {error}",
                    path.display()
                )
            },
        )
    })
}
