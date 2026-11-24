// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the command line needs before a verb runs: the profile's paths,
//! the endpoints a human client may lease and its channels, by the rule
//! the desktop client uses (`interweave_human_app_core::leasable_endpoints`);
//! the instance lock, so this client and the desktop one never hold the
//! store at once; and the store, opened or refused by class.

use std::path::PathBuf;
use std::time::Duration;

use interweave_human_app_core::{IdentityRefused, leasable_endpoints, resolve_identity};
use interweave_human_store::key_custody::{FilesystemCustody, KeyCipher};
use interweave_human_store::{HumanStore, STORE_FILE, StoreError, StoreOptions};
use interweave_profile_config::{
    EndpointSelector, HumanClientLock, PersistError, ProfileConfig, ProfilePaths, XdgRoots,
};
use interweave_transport_api::{ChannelId, EndpointId};

use crate::{Failure, output};

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
    /// The first of the endpoints above as the file lists them, kept when
    /// `--endpoint` narrows `endpoints` to the one a command speaks as: the
    /// store opens as it, and schema v14 gives it to every row an older
    /// store holds (STATE.md §Migrations; the desktop opens as the same one).
    pub(crate) store_identity: EndpointId,
    /// The channels it joins.
    pub(crate) channels: Vec<ChannelId>,
    /// The daemon's data socket.
    pub(crate) data_socket: PathBuf,
    /// The daemon's admin socket.
    pub(crate) admin_socket: PathBuf,
    /// The profile document, which an alias resolves over.
    pub(crate) config: ProfileConfig,
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
    resolve_in(profile, &roots)
}

/// [`resolve`] under `roots`.
fn resolve_in(profile: &str, roots: &XdgRoots) -> Result<Profile, Failure> {
    let paths = ProfilePaths::resolve(profile, roots).map_err(|e| paths_refusal(&e))?;
    if !paths.roles_are_distinct() {
        return Err(Failure::Refused(
            "the profile's configuration, state and runtime directories overlap".to_owned(),
        ));
    }
    // The document alone: a client holds no endpoint key, so it reads
    // nothing under `endpoint-keys/` (ENDPOINTS.md, an alias is resolved
    // by the client, A 2026-11-21); a key's header is the daemon's load.
    let config = ProfileConfig::load_document(&paths)
        .map_err(|e| Failure::Refused(format!("the profile's configuration: {e}")))?;
    let endpoints = leasable_endpoints(&config);
    let Some(store_identity) = endpoints.first().cloned() else {
        return Err(Failure::Refused(format!(
            "profile {:?} configures no endpoint a human client may use",
            paths.profile()
        )));
    };
    let socket = |e: PersistError| Failure::Refused(format!("the daemon's socket paths: {e}"));
    Ok(Profile {
        endpoints,
        store_identity,
        channels: config.channels.desired.clone(),
        data_socket: paths.data_socket().map_err(socket)?,
        admin_socket: paths.admin_socket().map_err(socket)?,
        paths,
        config,
    })
}

/// The id of the human endpoint `selector` names in `profile`: what a
/// session claims and an admin call carries, never the alias
/// (`interweave_human_app_core::resolve_identity`).
///
/// # Errors
/// [`Failure::Refused`] naming what was given and why it names no
/// endpoint this client may use.
pub(crate) fn identity(
    profile: &Profile,
    selector: &EndpointSelector,
) -> Result<EndpointId, Failure> {
    resolve_identity(&profile.paths, &profile.config, selector).map_err(|e| {
        Failure::Refused(match e {
            // The overlay's own text may carry a path: its class is enough
            // to look at the file.
            IdentityRefused::Overlay(_) => format!(
                "{:?} is an alias, and the profile's endpoint overlay could not be read",
                selector.to_string()
            ),
            other => other.to_string(),
        })
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

/// Why the store did not open, by what a person can do about it: the
/// desktop's classes (`apps/human-desktop/src/startup.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Blocked {
    /// Its files or its key cannot be read by this build: recovery, where
    /// discard and recreate is offered (ADR-0060 decision 6).
    Recovery,
    /// It or its directory is not private to this user.
    NotPrivate,
    /// It could not be opened now.
    Unavailable,
}

impl Blocked {
    /// Its token, as `store health` prints it.
    pub(crate) const fn token(self) -> &'static str {
        match self {
            Self::Recovery => "needs-recovery",
            Self::NotPrivate => "not-private",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Open the store, under the lock the caller holds, by the desktop's
/// classes. Nothing here touches the file but the store's open, which
/// leaves a file needing recovery where it was
/// (`crates/human/store/tests/storage_failure.rs`,
/// `a_migration_that_fails_part_way_leaves_the_file_at_its_old_version_with_every_row`).
///
/// # Errors
/// [`Failure::Store`] saying which.
pub(crate) fn open_store(profile: &Profile) -> Result<HumanStore, Failure> {
    // The store key at the v1 desktop's custody (ADR-0060 decision 3), as
    // the desktop's: a filesystem custody reports no invalidation.
    open_store_at(&profile.paths, &profile.store_identity, &FilesystemCustody)
        .map_err(|(_, failure)| failure)
}

/// [`open_store`] under `cipher`, as `identity`, with the class it was
/// refused by.
///
/// # Errors
/// The class, and [`Failure::Store`] saying it.
pub(crate) fn open_store_at(
    paths: &ProfilePaths,
    identity: &EndpointId,
    cipher: &dyn KeyCipher,
) -> Result<HumanStore, (Blocked, Failure)> {
    let path = paths.human_dir().join(STORE_FILE);
    HumanStore::open_profile(paths, cipher, identity, StoreOptions::default()).map_err(|error| {
        if error.needs_recovery() || matches!(error, StoreError::KeyInvalidated) {
            (
                Blocked::Recovery,
                Failure::Store(format!(
                    "the message store at {} cannot be read by this version and needs \
                     recovery; it was not renamed, moved or deleted (see store discard)",
                    output::one_line(&path.display().to_string())
                )),
            )
        } else if matches!(
            error,
            StoreError::NotAFile { .. }
                | StoreError::PermissionsTooOpen { .. }
                | StoreError::DirectoryNotPrivate { .. }
        ) {
            (
                Blocked::NotPrivate,
                Failure::Store(format!(
                    "the message store at {} or its directory is not private to this user; \
                     refusing to use it: {}",
                    output::one_line(&path.display().to_string()),
                    output::one_line(&error.to_string())
                )),
            )
        } else {
            (
                Blocked::Unavailable,
                Failure::Store(format!(
                    "the message store at {} could not be opened now: {}",
                    output::one_line(&path.display().to_string()),
                    output::one_line(&error.to_string())
                )),
            )
        }
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};

    use interweave_profile_config::endpoint_key;
    use interweave_test_support::endpoint_keys::endpoint_id;

    use super::*;

    const PROFILE: &str = "p";

    /// A profile named [`PROFILE`] under private roots in `dir`, its one
    /// entry `human` by key.
    fn profile_in(dir: &std::path::Path) -> XdgRoots {
        for sub in ["config", "data", "state", "cache", "run"] {
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(dir.join(sub))
                .expect("an xdg dir");
        }
        let roots = XdgRoots {
            config_home: dir.join("config"),
            data_home: dir.join("data"),
            state_home: dir.join("state"),
            cache_home: dir.join("cache"),
            runtime_dir: Some(dir.join("run")),
        };
        let paths = ProfilePaths::resolve(PROFILE, &roots).expect("paths");
        let file = paths.config_file();
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(file.parent().expect("a parent"))
            .expect("the config dir");
        let text = format!(
            "schema_version: 2
runtime: {{ deployment: daemon-ipc }}
profile: {{ name: {PROFILE} }}
transport:
  backend: libp2p
  listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/0\"] }}
identity: {{ algorithm: ed25519 }}
trust: {{ policy: static-allowlist, allowed_peers: [] }}
endpoints:
  registration_policy: configured-only
  default_direct_endpoint: human
  entries:
    - {{ key: {human}.key, alias: human, enabled: true, advertise: false, allowed_client_kinds: [human-client], inbound: inherit_profile_trust, outbound: inherit_profile_trust }}
ipc: {{ enabled: true, socket_layout: split-data-admin }}
channels: {{ desired: [] }}
",
            human = endpoint_id("human").as_str()
        );
        std::fs::write(&file, text).expect("config");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).expect("a mode");
        roots
    }

    /// The command line reads the profile document alone, nothing under
    /// `endpoint-keys/` (ENDPOINTS.md, an alias is resolved by the client,
    /// A 2026-11-21): a key file whose header derives another id, which the
    /// daemon's load refuses, does not stop it, and the alias resolves to
    /// the entry's id. The control: `load` refuses the same profile.
    /// Mutation: `resolve_in` back on `ProfileConfig::load`.
    #[test]
    fn a_key_file_only_the_daemon_reads_does_not_stop_the_command_line() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("private");
        let roots = profile_in(dir.path());
        let paths = ProfilePaths::resolve(PROFILE, &roots).expect("paths");
        let minted = endpoint_key::mint_filesystem(&paths).expect("minted");
        std::fs::rename(
            endpoint_key::key_path(&paths, &minted),
            endpoint_key::key_path(&paths, &endpoint_id("human")),
        )
        .expect("the minted key under the entry's name");
        let control = ProfileConfig::load(&paths).expect_err("the daemon's load refuses it");
        assert!(
            format!("{control:?}").contains("EndpointKeyMismatch"),
            "the control: {control:?}"
        );
        let profile = resolve_in(PROFILE, &roots).expect("resolves");
        assert_eq!(profile.endpoints, [endpoint_id("human")]);
        let alias = EndpointSelector::parse("human").expect("an alias");
        assert_eq!(
            identity(&profile, &alias).expect("resolves"),
            endpoint_id("human")
        );
    }
}
