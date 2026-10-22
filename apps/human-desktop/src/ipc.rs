// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade side over the desktop IPC binding: the data socket for the
//! session, the admin socket -- a separate connection, a separate
//! authority (ADR-0040) -- for the facade's status reads.

use std::time::{SystemTime, UNIX_EPOCH};

use interweave_human_app_core::FacadeSide;
use interweave_human_store::HumanStore;
use interweave_human_store::identity_key::{self, FilesystemCustody, IdentityKey};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_ipc_client::{IpcBinding, SocketPaths};
use interweave_transport_api::MAX_PAYLOAD_BYTES;

use crate::startup::{CLIENT_KIND, Profile};

/// Unix milliseconds, for what the store and the envelope record.
fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// What builds the facade side for `profile` over the IPC binding, on the
/// facade's own thread. Building opens nothing: the first turn opens the
/// session, so a daemon that is not running yet is a reconnecting
/// session, not a failure to start.
pub fn facade_over_ipc(
    profile: &Profile,
    store: HumanStore,
) -> impl FnOnce() -> Result<FacadeSide<IpcBinding, IpcBinding>, interweave_human_store::StoreError>
+ Send
+ 'static {
    let binding = IpcBinding::new(
        SocketPaths {
            data: profile.data_socket.clone(),
            admin: profile.admin_socket.clone(),
        },
        CLIENT_KIND,
    );
    let identity = identity(profile);
    let config = ClientConfig {
        client_kind: CLIENT_KIND.to_owned(),
        endpoints: profile.endpoints.clone(),
        channels: profile.channels.clone(),
        max_payload_bytes: MAX_PAYLOAD_BYTES,
    };
    move || {
        let client = TransportClient::new(
            binding.clone(),
            binding,
            store,
            config,
            Box::new(wall_ms),
            0,
        )?;
        let mut facade = FacadeSide::new(client, wall_ms);
        if let Some(key) = identity {
            facade.set_identity_key(key);
        }
        Ok(facade)
    }
}

/// The identity in front's own key (ADR-0063): the first configured
/// endpoint's, opened from its file, or made with the identity the first
/// time it runs. At the desktop's filesystem floor (ADR-0060 decision 3).
/// A file this build refuses -- another identity's, a seed record, a
/// sealed one -- is left as it is, its refusal named on stderr by class
/// and never its content, and the client runs with no key: Share says
/// so, and nothing is signed.
fn identity(profile: &Profile) -> Option<IdentityKey> {
    let endpoint = profile.endpoints.first()?;
    let opened = match identity_key::open(&profile.paths, endpoint, &FilesystemCustody) {
        Ok(Some(key)) => Ok(key),
        Ok(None) => identity_key::create(&profile.paths, endpoint, &FilesystemCustody),
        Err(e) => Err(e),
    };
    match opened {
        Ok(key) => Some(key),
        Err(e) => {
            eprintln!("human-desktop: the identity's key is not usable: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::os::unix::fs::PermissionsExt as _;

    use interweave_profile_config::{ProfilePaths, TrustBoundary};
    use interweave_transport_api::EndpointId;

    use super::*;

    fn profile(dir: &std::path::Path) -> Profile {
        let paths =
            ProfilePaths::resolve_embedded("default", TrustBoundary::new(dir).expect("a boundary"))
                .expect("paths");
        Profile {
            paths,
            endpoints: vec![EndpointId::parse("human").expect("an endpoint")],
            channels: Vec::new(),
            data_socket: dir.join("data.sock"),
            admin_socket: dir.join("admin.sock"),
        }
    }

    fn private_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("chmod");
        dir
    }

    #[test]
    fn the_first_start_makes_the_identitys_key_and_every_later_one_opens_it() {
        let dir = private_dir();
        let profile = profile(dir.path());
        let first = identity(&profile).expect("made");
        let file = identity_key::key_path(&profile.paths, &profile.endpoints[0]);
        assert!(file.is_file(), "under the state root, beside the store");
        let again = identity(&profile).expect("opened");
        assert_eq!(
            again.public_key(),
            first.public_key(),
            "the same key, never a new one"
        );
    }

    #[test]
    fn a_key_file_this_build_refuses_is_left_as_it_is_and_no_key_is_used() {
        let dir = private_dir();
        let profile = profile(dir.path());
        identity(&profile).expect("made");
        let file = identity_key::key_path(&profile.paths, &profile.endpoints[0]);
        std::fs::write(&file, "not a record\n").expect("overwrite");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("chmod");
        assert!(identity(&profile).is_none(), "no key, so nothing is signed");
        assert_eq!(
            std::fs::read_to_string(&file).expect("read"),
            "not a record\n",
            "nothing replaces a refused file: a new key is the person's act"
        );
    }
}
