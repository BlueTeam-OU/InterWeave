// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Android human client's platform side (plan section 20 steps 2-4):
//! what the foreground Service holds -- the embedded transport runtime,
//! the message store and the facade over the runtime's in-process binding
//! ([`ServiceHost`]) -- and where the Activity's view and the platform's
//! notifications meet it ([`Hub`]).
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
pub mod notice;
pub mod offline;
pub mod service;
#[cfg(feature = "dev-stand-ins")]
pub mod stand_in;
pub mod view_side;

pub use hub::{Hub, TO_VIEW, ToView, ViewLink};
pub use identity::{FirstRun, IdentityRefused, start_identity};
pub use interweave_profile_config::runtime::KeyUnlockPolicy;
/// The embedded custody path, for the app's Keystore adapter of its
/// `SeedCipher` seam.
pub use interweave_transport_embedded::custody;
pub use interweave_transport_embedded::{EmbeddedRefused, StayReachable};
pub use notice::Notices;
pub use service::{
    AvailabilityMode, CLIENT_KIND, Ended, LogLevel, ServiceHost, ServiceLaunch, StartRefused,
    Stopped, profile_log_level,
};
pub use view_side::{ListingFailed, ViewSide};
