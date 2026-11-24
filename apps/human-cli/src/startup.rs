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
    /// (the daemon's effective state decides at open), or the one
    /// `--endpoint` names ([`narrow`]). The first is the identity a run
    /// speaks as: the one its session leases, whose rows its store reads
    /// and writes ([`open_store`]).
    pub(crate) endpoints: Vec<EndpointId>,
    /// The first of the endpoints above as the file lists them, kept when
    /// `--endpoint` narrows `endpoints` to the one a command speaks as: the
    /// store opens as it, and schema v14 gives it to every row an older
    /// store holds (STATE.md §Migrations; the desktop opens as the same one).
    /// Never the identity a run speaks as for that alone.
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

/// `--endpoint`: one of the profile's human endpoints, by its alias or its
/// id, is the one a run speaks as, in place of the first: the only one its
/// session leases and its store's scope. Never one the profile does not
/// offer: no endpoint is made here (G2; ENDPOINTS.md, configured only).
///
/// # Errors
/// As [`identity`].
pub(crate) fn narrow(profile: &mut Profile, selector: &EndpointSelector) -> Result<(), Failure> {
    profile.endpoints = vec![identity(profile, selector)?];
    Ok(())
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
/// It opens as the profile's first identity, the migration's target, and
/// speaks as the one the run leases ([`Profile::endpoints`]' first): the
/// client leases the identity its store speaks as, so a run with
/// `--endpoint` sends that identity's pending rows and commits what it
/// receives under it, never the first's
/// (`a_run_with_endpoint_and_a_plain_run_see_each_others_rows_never`).
///
/// # Errors
/// [`Failure::Store`] saying which.
pub(crate) fn open_store(profile: &Profile) -> Result<HumanStore, Failure> {
    // The store key at the v1 desktop's custody (ADR-0060 decision 3), as
    // the desktop's: a filesystem custody reports no invalidation.
    let store = open_store_at(&profile.paths, &profile.store_identity, &FilesystemCustody)
        .map_err(|(_, failure)| failure)?;
    let speaking = profile
        .endpoints
        .first()
        .unwrap_or(&profile.store_identity)
        .clone();
    Ok(store.speaking_as(speaking))
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
        profile_with(dir, &[])
    }

    /// As [`profile_in`], with a further human entry for each alias in
    /// `more`, by key.
    fn profile_with(dir: &std::path::Path, more: &[&str]) -> XdgRoots {
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
{more}ipc: {{ enabled: true, socket_layout: split-data-admin }}
channels: {{ desired: [] }}
",
            human = endpoint_id("human").as_str(),
            more = more.iter().fold(String::new(), |mut out, alias| {
                use std::fmt::Write as _;
                let _ = writeln!(
                    out,
                    "    - {{ key: {}.key, alias: {alias}, enabled: true, advertise: false, \
                     allowed_client_kinds: [human-client], inbound: inherit_profile_trust, \
                     outbound: inherit_profile_trust }}",
                    endpoint_id(alias).as_str()
                );
                out
            }),
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

    /// A run with `--endpoint` and a plain run each see their own identity's
    /// rows and never the other's (R16-F1): the store opens as the first
    /// identity and speaks as the one the run leases, so a `listen
    /// --endpoint spare` commits what it receives, and leaves what it sends
    /// pending, under `spare`, none of which a plain run lists; and a plain
    /// run's pending send is not listed by a run with `--endpoint`. The
    /// client is composed as `main`'s facade composes it, over the fake
    /// pair. Mutation: `open_store` without its `speaking_as`.
    #[tokio::test]
    async fn a_run_with_endpoint_and_a_plain_run_see_each_others_rows_never() {
        use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
        use interweave_human_store::{AppMessageId, NewOutbound, OutboundDestination};
        use interweave_human_transport_client::{
            ClientConfig, Destination, SessionState, TransportClient,
        };
        use interweave_local_client_api::{
            DataCapability, DataSessionBinding, DataSessionPort, SessionRequest,
        };
        use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork};
        use interweave_profile_identity::ProfileIdentity;
        use interweave_transport_api::{
            DirectDestination, MediaType, MessageId, Payload, TransportError,
        };

        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("private");
        let roots = profile_with(dir.path(), &["spare"]);
        let plain = resolve_in(PROFILE, &roots).expect("resolves");
        assert_eq!(
            plain.endpoints,
            [endpoint_id("human"), endpoint_id("spare")]
        );
        let mut narrowed = plain.clone();
        narrow(
            &mut narrowed,
            &EndpointSelector::parse("spare").expect("an alias"),
        )
        .expect("narrowed");
        assert_eq!(narrowed.endpoints, [endpoint_id("spare")]);

        let node = |endpoints: Vec<FakeEndpoint>| FakeConfig {
            peer: ProfileIdentity::generate()
                .transport_identity()
                .expect("peer id"),
            endpoints,
            default_endpoint: Some(endpoint_id("remote")),
            queue_bound: 16,
        };
        let (a, b) = FakeNetwork::pair(
            node(vec![FakeEndpoint::open(endpoint_id("remote"), false)]),
            node(vec![
                FakeEndpoint::open(endpoint_id("human"), false),
                FakeEndpoint::open(endpoint_id("spare"), false),
            ]),
        );
        let envelope = |text: &str, n: u128| HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: format!("{n:032x}"),
            text: text.to_owned(),
            reply_to: None,
            sent_at_ms: None,
            from_endpoint: None,
        };

        // `listen --endpoint spare`: what it receives, and a send left
        // pending.
        {
            let mut listen = TransportClient::new(
                b.clone(),
                b.clone(),
                open_store(&narrowed).expect("store"),
                ClientConfig {
                    client_kind: CLIENT_KIND.to_owned(),
                    endpoints: narrowed.endpoints.clone(),
                    channels: Vec::new(),
                    max_payload_bytes: 49_152,
                },
                Box::new(|| 1),
                0,
            )
            .expect("facade");
            listen.tick(0).await;
            assert_eq!(
                listen.session_state(),
                &SessionState::Ready {
                    endpoint: Some(endpoint_id("spare"))
                }
            );
            let from = a
                .open(
                    SessionRequest::new(
                        "raw",
                        Some(endpoint_id("remote")),
                        [DataCapability::Commands, DataCapability::Events],
                    )
                    .expect("bounds"),
                )
                .await
                .expect("opens");
            from.send_direct(
                DirectDestination {
                    peer: b.peer().clone(),
                    endpoint: Some(endpoint_id("spare")),
                },
                MessageId::from_bytes([7; 16]),
                Payload::at_ceiling(
                    Some(
                        MediaType::parse("application/vnd.interweave-human-chat+json;v=2")
                            .expect("valid"),
                    ),
                    serde_json::to_vec(&envelope("to the spare", 1)).expect("json"),
                )
                .expect("fits"),
            )
            .await
            .expect("accepted");
            assert_eq!(listen.drain(16, 1).await.len(), 1, "received");
            b.inject_send(TransportError::Timeout);
            listen
                .send(
                    Destination::Direct {
                        peer: a.peer().clone(),
                        endpoint: None,
                    },
                    &envelope("left pending", 2),
                    1,
                )
                .await
                .expect("committed");
            listen.tick(2).await;
            assert_eq!(
                listen.store_mut().pending_outbound().expect("read").len(),
                1
            );
            listen.close().await;
        }

        // A plain run lists none of it.
        {
            let mut store = open_store(&plain).expect("plain");
            assert!(
                store.unread_inbound().expect("read").is_empty(),
                "the spare's message is not the first's"
            );
            assert!(
                store.pending_outbound().expect("read").is_empty(),
                "the spare's pending send is not the first's"
            );
            // And the reverse: the plain run's own send, left pending.
            store
                .commit_pending_outbound(&NewOutbound {
                    app_message_id: AppMessageId::parse(format!("{:032x}", 3)).expect("an id"),
                    transport_message_id: MessageId::from_bytes([9; 16]),
                    destination: OutboundDestination::Direct(DirectDestination::to_default(
                        a.peer().clone(),
                    )),
                    media_type: None,
                    payload: serde_json::to_vec(&envelope("the first's", 3)).expect("json"),
                    created_at: 3,
                })
                .expect("committed");
        }
        let spare = open_store(&narrowed).expect("narrowed again");
        assert_eq!(spare.unread_inbound().expect("read").len(), 1, "its own");
        let pending: Vec<_> = spare
            .pending_outbound()
            .expect("read")
            .into_iter()
            .map(|r| r.app_message_id.as_str().to_owned())
            .collect();
        assert_eq!(
            pending,
            [format!("{:032x}", 2)],
            "the spare's send alone, not the plain run's"
        );
    }
}
