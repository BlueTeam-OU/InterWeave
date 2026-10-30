// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The human client's headless root (Stage 15; architect-cto's Q1 ruling,
//! relay seq 11163).
//!
//! A root is two sides that speak one protocol:
//! - the [`FacadeSide`] owns the transport facade, and through it the
//!   store. It carries out [`Command`]s and reports [`Update`]s.
//! - the [`ModelSide`] owns the `UiModel` and a view behind [`Surface`].
//!   It applies updates and turns what the person did into commands.
//!
//! On the desktop the two run on different threads: the facade side on
//! its own runtime, because a session whose events go undrained loses its
//! lease; the model side on the toolkit's. Everything that crosses is a
//! plain value. No tokio, no toolkit and no platform code here, so
//! Android reuses it at Stage 17.

#![forbid(unsafe_code)]

pub mod card_photo;
mod cards;
mod facade;
mod listing;
mod model_side;
mod profile;
mod protocol;
#[cfg(feature = "test-hooks")]
pub mod test_hooks;

pub use facade::{CANCELLED_CAP, FacadeSide, READ_COPY_CAP};
/// Why the identity's key cannot be used, as a root logs it.
pub use interweave_human_ui_model::KeyProblem;
pub use listing::kind_of as stored_kind;
pub use model_side::{ModelSide, Opener, PROBLEM_CAP, Problem, Surface};
pub use profile::{CLIENT_KIND, KeySource, identity_key, identity_key_under, leasable_endpoints};
pub use protocol::{Command, Failure, Listing, Start, Update, WindowCommand};
