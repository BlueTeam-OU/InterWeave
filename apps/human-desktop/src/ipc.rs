// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The roster over the desktop IPC binding: the data socket for each
//! identity's session, the admin socket -- a separate connection, a
//! separate authority (ADR-0040) -- for the status reads.
//!
//! The roster holds the first identity alone in this batch (plan §30,
//! architect-cto's ruling (b) of 2026-11-25): the desktop leases as
//! `main` did with one facade side, and a roster of one answers as that
//! side did (app-core's `a_roster_of_one_behaves_as_its_facade_side`).

use std::time::{SystemTime, UNIX_EPOCH};

use interweave_human_app_core::{KeySource, Roster, RosterConfig};
use interweave_human_store::HumanStore;
use interweave_human_store::identity_key::FilesystemCustody;
use interweave_ipc_client::{IpcBinding, SocketPaths};
use interweave_transport_api::MAX_PAYLOAD_BYTES;

use crate::facade_thread::FacadeFault;
use crate::startup::{CLIENT_KIND, Profile};

/// Unix milliseconds, for what the store and the envelope record.
fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// What builds the roster for `profile` over the IPC binding, on the
/// facade's own thread. Building opens nothing: the first turn opens the
/// sessions, so a daemon that is not running yet is a reconnecting
/// session, not a failure to start. The identity in front at start is the
/// one the store last remembered, else the profile's default, else the
/// first (`Roster::new`).
pub fn facade_over_ipc(
    profile: &Profile,
    store: HumanStore,
) -> impl FnOnce() -> Result<Roster<IpcBinding, IpcBinding>, FacadeFault> + Send + 'static {
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
    let config = RosterConfig {
        client_kind: CLIENT_KIND.to_owned(),
        endpoints: profile.endpoints.clone(),
        channels: profile.channels.clone(),
        max_payload_bytes: MAX_PAYLOAD_BYTES,
        // The first identity alone: Add is not reachable in this batch.
        others: Vec::new(),
        default: profile.default.clone(),
    };
    move || {
        let mut roster = Roster::new(
            config,
            store,
            |_| (binding.clone(), binding.clone()),
            wall_ms,
            0,
        )
        .map_err(FacadeFault::from)?;
        // The identity in front's own key, by app-core's rule: a file this
        // build refuses is named on stderr by its kind, never its content,
        // and the identity's page says so and offers what it can.
        let first = roster.first().clone();
        if let Some(source) = source
            && let Some(facade) = roster.side_mut(&first)
        {
            facade.set_key_source(source);
            if let Some(problem) = facade.key_problem() {
                eprintln!("human-desktop: the identity's key is not usable: {problem:?}");
            }
        }
        Ok(roster)
    }
}
