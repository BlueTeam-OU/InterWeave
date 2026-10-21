// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Stage 18's local adversarial suite (plan section 21, S9-S11).
//!
//! The tests are under `tests/`. What they share is here: the [`secret`]
//! scan S11 runs over every stream it captured, and the [`tap`] that
//! captures the IPC frames between a client and the daemon.

#![cfg(unix)]
// A harness helper panics when the case cannot be set up -- that is its
// failure, by name.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    reason = "test harness: every helper panics on a broken setup"
)]

pub mod secret;
pub mod tap;
