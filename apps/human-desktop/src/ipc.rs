// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade side over the desktop IPC binding: the data socket for the
//! session, the admin socket -- a separate connection, a separate
//! authority (ADR-0040) -- for the facade's status reads.

use std::time::{SystemTime, UNIX_EPOCH};

use interweave_human_app_core::{FacadeSide, identity_key};
use interweave_human_store::HumanStore;
use interweave_human_store::identity_key::IdentityKey;
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
    let identity = identity(profile, &store);
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

/// The identity in front's own key, by app-core's rule
/// (`interweave_human_app_core::identity_key`, whose tests hold it). A file
/// this build refuses is named on stderr by class, never its content, and
/// the client runs with no key: Share says so, and nothing is signed.
fn identity(profile: &Profile, store: &HumanStore) -> Option<IdentityKey> {
    match identity_key(&profile.paths, &profile.endpoints, store) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("human-desktop: the identity's key is not usable: {e}");
            None
        }
    }
}
