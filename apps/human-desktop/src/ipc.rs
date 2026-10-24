// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade side over the desktop IPC binding: the data socket for the
//! session, the admin socket -- a separate connection, a separate
//! authority (ADR-0040) -- for the facade's status reads.

use std::time::{SystemTime, UNIX_EPOCH};

use interweave_human_app_core::{FacadeSide, KeySource};
use interweave_human_store::HumanStore;
use interweave_human_store::identity_key::FilesystemCustody;
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
    let source = profile.endpoints.first().map(|endpoint| {
        KeySource::new(
            profile.paths.clone(),
            endpoint.clone(),
            Box::new(FilesystemCustody),
        )
    });
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
        // The identity in front's own key, by app-core's rule: a file this
        // build refuses is named on stderr by its kind, never its content,
        // and the identity's page says so and offers what it can.
        if let Some(source) = source {
            facade.set_key_source(source);
            if let Some(problem) = facade.key_problem() {
                eprintln!("human-desktop: the identity's key is not usable: {problem:?}");
            }
        }
        Ok(facade)
    }
}
