// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Human client domain state.
//!
//! Stage 2 activates the retention state machine: which message states
//! are durable, and the transitions between them (ADR-0044). It stores
//! nothing — a store carries out the `Durability` this module decides,
//! which is what lets every transition be tested by enumeration.
//! Beside it, what both the store and the client's model name and
//! neither may pull from the other: the local identifiers (`ids`) and
//! when two names are the same name (`names`).

#![forbid(unsafe_code)]

pub mod ids;
pub mod names;
pub mod retention;

pub use ids::{AppMessageId, MalformedAppMessageId, RowId};
pub use names::{NameKeys, same_name};

pub use retention::{
    DegradedResponse, Durability, InboundMessage, InboundState, KeepRefused, OutboundMessage,
    OutboundState, StorageHealth, TerminalCause,
};
