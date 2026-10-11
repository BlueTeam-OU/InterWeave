// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An endpoint set left ahead of the runtime, through the composition: a
//! binary of its own, because the ahead warning it logs names no runtime
//! and `endpoint_overlay.rs` counts that warning in the one log its
//! binary shares. Run beside that count, this test raised it during a
//! case that expects none (8 of 8 runs on a ten-core host, 2026-10-11),
//! the class `trust_overlay_ahead.rs` was split out for (#215 re-review
//! N1).

#![allow(clippy::expect_used, clippy::panic)]

use interweave_transport_api::TransportError;

mod common;
mod endpoint_common;

use endpoint_common::{on_disk, rows, set_enabled, start, state};

/// A failed publish whose restore then fails before its rename leaves
/// the overlay AHEAD: answered `Internal`, never the publish's own
/// `BackendUnavailable`, since the set does take effect at the next
/// start -- the file says so and the runtime does not.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_publish_whose_restore_fails_is_left_ahead() {
    use interweave_transport_composition::OverlayFault::{BeforeRename, Clean};

    let (_dir, path) = state();
    let runtime = start(Some(&path)).await.expect("composes");
    runtime.fail_publishes(1).await.expect("queued");
    // The set's own write is clean; the restore's meets the fault.
    runtime
        .fail_overlay_writes(vec![Clean, BeforeRename])
        .await
        .expect("queued");
    assert_eq!(
        set_enabled(&runtime, "agent", false).await,
        Err(TransportError::Internal)
    );
    assert!(
        rows(&runtime).await[0].1,
        "not published: agent still enabled"
    );
    assert!(!on_disk(&path)[0].1, "ahead: disabled on disk");
    runtime.stop().await.expect("stops");
}
