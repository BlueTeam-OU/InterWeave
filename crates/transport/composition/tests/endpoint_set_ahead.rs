// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A removal left ahead of the runtime (LOCAL-IPC.md, `admin.endpoints.
//! remove`, A 2026-11-24): a binary of its own, as
//! `endpoint_overlay_ahead.rs` is, since the ahead warning it logs is
//! counted by no other binary's log.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use interweave_local_client_api::{AdminPort, EndpointRemoveChange};
use interweave_transport_api::TransportError;
use interweave_transport_composition::ComposedRuntime;

mod endpoint_set_common;

use endpoint_set_common::{add_minted, admin, id, options, paths, profile, row_of};

/// The removal's write lands, its publish fails and the restore's write
/// fails before its rename: answered `Internal`. The overlay on disk no
/// longer holds the entry while the runtime still serves it, so a second
/// removal is `EndpointUnknown` and the runtime's list still holds the
/// id with its alias; after a restart it is gone. The control: before the
/// faults, the list held it and a removal was answered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_removal_left_ahead_is_internal_then_unknown_and_gone_at_the_restart() {
    use interweave_transport_composition::OverlayFault::{BeforeRename, Clean};

    let p = paths();
    let (identity, _) = id();
    let runtime = ComposedRuntime::start(&identity, &profile(), options(&p))
        .await
        .expect("composes");
    let port = admin(&runtime).await;
    let work = add_minted(&p, &port, "work").await;
    let other = add_minted(&p, &port, "other").await;
    assert!(matches!(
        port.remove_endpoint(other.clone()).await,
        Ok(EndpointRemoveChange::Removed { .. })
    ));
    runtime.fail_publishes(1).await.expect("queued");
    runtime
        .fail_overlay_writes(vec![Clean, BeforeRename])
        .await
        .expect("queued");
    assert_eq!(
        port.remove_endpoint(work.clone()).await,
        Err(TransportError::Internal)
    );
    assert_eq!(
        port.remove_endpoint(work.clone()).await,
        Err(TransportError::EndpointUnknown),
        "the overlay kept on disk no longer holds it"
    );
    let row = row_of(&port, &work)
        .await
        .expect("the runtime still serves it");
    assert_eq!(row.alias.as_deref(), Some("work"));
    drop(port);
    runtime.stop().await.expect("stops");

    let again = ComposedRuntime::start(&identity, &profile(), options(&p))
        .await
        .expect("restarts");
    let port = admin(&again).await;
    assert!(
        row_of(&port, &work).await.is_none(),
        "gone at the next start"
    );
    drop(port);
    again.stop().await.expect("stops");
}
