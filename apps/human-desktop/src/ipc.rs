// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The roster over the desktop IPC binding: the data socket for each
//! identity's session, the admin socket -- a separate connection, a
//! separate authority (ADR-0040) -- for the status reads.
//!
//! The roster holds every identity of the profile (app-core's
//! `held_list`, architect-cto's REPLY 01a216b6): the first, then each
//! entry the endpoint overlay added for this client's kind, each with
//! its own key source. A roster of one answers as `main`'s one facade
//! side did (app-core's `a_roster_of_one_behaves_as_its_facade_side`).
//!
//! The roster can also hold an identity made while it runs -- by an add
//! form's Create, or found by a settle of a pending-add record -- over the
//! same binding, the profile's `endpoint-keys/` and the filesystem
//! custody each held identity's key opens under (`Roster::adding`).

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use interweave_human_app_core::{AddFactory, KeySource, Roster, RosterConfig};
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
    // Each identity's own key, by app-core's rule: an identity whose key
    // is missing or refused is leased as any other and shares no card
    // (ADR-0067 decisions 3 and 5), and no key is made for it.
    let sources: Vec<KeySource> = profile
        .held
        .iter()
        .map(|held| {
            KeySource::new(
                profile.paths.clone(),
                held.endpoint.clone(),
                Box::new(FilesystemCustody),
            )
        })
        .collect();
    let held: Vec<_> = profile.held.iter().map(|h| h.endpoint.clone()).collect();
    let config = RosterConfig {
        client_kind: CLIENT_KIND.to_owned(),
        // Every identity held, so each client's lease target is in it.
        endpoints: held.clone(),
        channels: profile.channels.clone(),
        max_payload_bytes: MAX_PAYLOAD_BYTES,
        others: held.into_iter().skip(1).collect(),
        default: profile.default.clone(),
        aliases: profile
            .held
            .iter()
            .filter_map(|h| Some((h.endpoint.clone(), h.alias.clone()?)))
            .collect(),
    };
    let factory = AddFactory {
        bindings: {
            let binding = binding.clone();
            Box::new(move |_| (binding.clone(), binding.clone()))
        },
        admin: Arc::new(binding.clone()),
        paths: profile.paths.clone(),
        cipher: Arc::new(FilesystemCustody),
    };
    move || {
        let mut roster = Roster::new(
            config,
            store,
            |_| (binding.clone(), binding.clone()),
            wall_ms,
            0,
        )
        .map_err(FacadeFault::from)?
        .adding(factory);
        // A file this build refuses is named on stderr by its kind, never
        // its content, and the identity's page says so and offers what it
        // can.
        for source in sources {
            let endpoint = source.endpoint().clone();
            if let Some(facade) = roster.side_mut(&endpoint) {
                facade.set_key_source(source);
                if let Some(problem) = facade.key_problem() {
                    eprintln!("human-desktop: the identity's key is not usable: {problem:?}");
                }
            }
        }
        Ok(roster)
    }
}
