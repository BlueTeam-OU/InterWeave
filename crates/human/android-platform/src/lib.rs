// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Android human client's platform side (plan section 20 steps 2-4):
//! what the foreground Service holds -- the embedded transport runtime,
//! the message store and the facade over the runtime's in-process binding
//! ([`ServiceHost`]) -- and where the Activity's view and the platform's
//! notifications meet it ([`Hub`]), and what the platform reports of its
//! networks ([`network`]).
//!
//! No Android, JNI or Slint type: the app's native library carries the
//! JNI entry points and the views, so everything here builds and is
//! tested on the host.

#![forbid(unsafe_code)]

#[cfg(feature = "e2e-cases")]
pub mod e2e;
pub mod facade;
pub mod hub;
pub mod identity;
pub mod network;
pub mod notice;
pub mod offline;
pub mod onboarding;
mod profile;
pub mod service;
#[cfg(feature = "dev-stand-ins")]
pub mod stand_in;
pub mod start_rules;
pub mod view_side;

pub use hub::{
    Hub, StoreAnswer, StoreGeneration, StoreRefusal, TO_VIEW, ToView, UnlockSaid, ViewLink,
};
pub use identity::{
    Expected, FirstRun, IdentityRefused, ResealFailure, Resealed, RestoreRefused, Started,
    UnlockStatus, locked_status, opening_needs_presence, phrase_peer, reseal_identity,
    reseal_needs_presence, restart_requires_user_authentication_at_rest, restore_identity,
    start_identity, start_resealed, unlock_facts,
};
pub use interweave_profile_config::runtime::KeyUnlockPolicy;
/// The embedded custody path, for the app's Keystore adapter of its
/// `SeedCipher` seam.
pub use interweave_transport_embedded::custody;
pub use interweave_transport_embedded::{EmbeddedRefused, StayReachable, log_admits};
pub use network::{Link, NetworkReport, Told, Transports, addresses_from_wire};
pub use notice::Notices;
pub use profile::provision_profile;
pub use service::{
    AvailabilityMode, CLIENT_KIND, Ended, LogLevel, ServiceHost, ServiceLaunch, StartRefused,
    Stopped, StoreActionRefused, StoreDiscarded, profile_log_level,
};
pub use view_side::{ListingFailed, ViewSide};
