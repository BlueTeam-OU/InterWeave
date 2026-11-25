// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A removal left ahead of the runtime (LOCAL-IPC.md, `admin.endpoints.
//! remove`, A 2026-11-24): a binary of its own, as
//! `endpoint_overlay_ahead.rs` is, since the ahead warning it logs is
//! counted by no other binary's log.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_local_client_api::{
    AdminPort, DataCapability, DataSessionBinding, DataSessionPort, EndpointAddChange,
    EndpointRemoveChange, SessionRequest, TrustChange,
};
use interweave_transport_api::{
    DirectDestination, EndpointId, MessageId, Payload, TransportError, TransportEvent,
    TransportIdentity, TransportRuntime,
};
use interweave_transport_composition::{ComposedRuntime, CompositionOptions, InProcessSession};

mod endpoint_set_common;

use endpoint_set_common::{
    add_minted, admin, bytes, ep, id, mint, options, paths, profile, profile_with, row_of,
};

const PATIENCE: Duration = Duration::from_secs(20);

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

/// An add over a removal left ahead (LOCAL-IPC.md, `admin.endpoints.add`)
/// is judged by the overlay and answered `added`, and the endpoint's two
/// sets are `[]` in the runtime from the answer on: a peer its set held
/// before is refused after, while the alias and the live lease the
/// runtime held stay. The control: before the faults the peer in the set
/// is accepted. Fed what a client holds -- the same key, alias and kinds
/// sent again after the `Internal`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_add_over_a_removal_left_ahead_empties_the_runtimes_sets_and_keeps_the_lease() {
    use interweave_transport_composition::OverlayFault::{BeforeRename, Clean};

    let ip = interweave_test_support::net::require_private_interface_v4();
    let p = paths();
    let (r_id, r) = id();
    let (s_id, s) = id();
    let listen = |base: CompositionOptions| CompositionOptions {
        listen: vec![format!("/ip4/{ip}/tcp/0")],
        ..base
    };
    let mut receiver = ComposedRuntime::start(
        &r_id,
        &profile_with(&[&s], &[], &[], 8),
        listen(options(&p)),
    )
    .await
    .expect("r composes");
    let port = admin(&receiver).await;
    let (work, key) = mint(&p);
    assert_eq!(
        port.add_endpoint(key.clone(), "work".to_owned(), vec![])
            .await,
        Ok(EndpointAddChange::Added(work.clone()))
    );
    let _at_work = receiver
        .sessions()
        .open(
            SessionRequest::new("human-client", Some(work.clone()), [DataCapability::Events])
                .expect("in bounds"),
        )
        .await
        .expect("leases work");
    let _at_human = receiver
        .sessions()
        .open(
            SessionRequest::new("human-client", Some(ep("human")), [DataCapability::Events])
                .expect("in bounds"),
        )
        .await
        .expect("leases human");
    assert_eq!(
        port.set_endpoint_trust(work.clone(), s.clone(), true).await,
        Ok(TrustChange::Set)
    );
    let leased = row_of(&port, &work).await.expect("listed").lease;
    assert!(leased.is_some(), "the lease is held before the faults");

    let r_addr = format!("{}/p2p/{}", receiver.listening()[0], r.as_str());
    let sender = ComposedRuntime::start(
        &s_id,
        &profile_with(&[&r], &[r_addr], &[], 8),
        listen(CompositionOptions::default()),
    )
    .await
    .expect("s composes");
    loop {
        match tokio::time::timeout(PATIENCE, receiver.next_event()).await {
            Ok(Some(TransportEvent::PeerConnected { peer, .. })) if peer == s => break,
            Ok(Some(_)) => {}
            other => panic!("no connection from the sender: {other:?}"),
        }
    }
    let sending = sender
        .sessions()
        .open(
            SessionRequest::new(
                "human-client",
                Some(ep("human")),
                [DataCapability::Commands],
            )
            .expect("in bounds"),
        )
        .await
        .expect("s leases");
    let send = |tag: u8| {
        sending.send_direct(
            DirectDestination {
                peer: r.clone(),
                endpoint: Some(work.clone()),
            },
            MessageId::from_bytes([tag; 16]),
            Payload::at_ceiling(None, vec![tag]).expect("within the ceiling"),
        )
    };
    assert_eq!(
        tokio::time::timeout(PATIENCE, send(1))
            .await
            .expect("answered"),
        Ok(work.clone()),
        "control: the endpoint's own set names the sender"
    );

    receiver.fail_publishes(1).await.expect("queued");
    receiver
        .fail_overlay_writes(vec![Clean, BeforeRename])
        .await
        .expect("queued");
    assert_eq!(
        port.remove_endpoint(work.clone()).await,
        Err(TransportError::Internal)
    );
    assert_eq!(
        port.add_endpoint(key, "work".to_owned(), vec![]).await,
        Ok(EndpointAddChange::Added(work.clone())),
        "judged by the overlay, which no longer holds the entry"
    );
    assert_eq!(
        tokio::time::timeout(PATIENCE, send(2))
            .await
            .expect("answered"),
        Err(TransportError::RemoteEndpointUnavailable),
        "the runtime's set for the endpoint is [] from the answer on"
    );
    let row = row_of(&port, &work).await.expect("still served");
    assert_eq!(row.alias.as_deref(), Some("work"));
    assert_eq!(row.lease, leased, "the held lease stays");
    drop((sending, port));
}

/// A trust write after a removal left ahead -- the overlay no longer
/// holds `work`, the runtime still serves and leases it -- publishes
/// sources without `work`'s own set, and `work` still admits nobody in
/// either direction: an added endpoint's filters are the empty subset,
/// never the operator's list (ADR-0067 decision 6). Fed what a client
/// holds: the operator's list names the sender, which `work`'s set never
/// did, and the trust write is an ordinary one on `human`. The controls,
/// before the faults and after the write: the sender reaches `human`, and
/// `human` reaches the sender.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_trust_write_after_a_removal_left_ahead_keeps_the_added_endpoint_closed() {
    use interweave_transport_composition::OverlayFault::{BeforeRename, Clean};

    let ip = interweave_test_support::net::require_private_interface_v4();
    let p = paths();
    let (r_id, r) = id();
    let (s_id, s) = id();
    let (_, bystander) = id();
    let listen = |base: CompositionOptions| CompositionOptions {
        listen: vec![format!("/ip4/{ip}/tcp/0")],
        ..base
    };
    let mut receiver = ComposedRuntime::start(
        &r_id,
        &profile_with(&[&s], &[], &[], 8),
        listen(options(&p)),
    )
    .await
    .expect("r composes");
    let port = admin(&receiver).await;
    let work = add_minted(&p, &port, "work").await;
    let both = [DataCapability::Events, DataCapability::Commands];
    let at_work = receiver
        .sessions()
        .open(SessionRequest::new("human-client", Some(work.clone()), both).expect("in bounds"))
        .await
        .expect("leases work");
    let at_human = receiver
        .sessions()
        .open(SessionRequest::new("human-client", Some(ep("human")), both).expect("in bounds"))
        .await
        .expect("leases human");

    let r_addr = format!("{}/p2p/{}", receiver.listening()[0], r.as_str());
    let sender = ComposedRuntime::start(
        &s_id,
        &profile_with(&[&r], &[r_addr], &[], 8),
        listen(CompositionOptions::default()),
    )
    .await
    .expect("s composes");
    loop {
        match tokio::time::timeout(PATIENCE, receiver.next_event()).await {
            Ok(Some(TransportEvent::PeerConnected { peer, .. })) if peer == s => break,
            Ok(Some(_)) => {}
            other => panic!("no connection from the sender: {other:?}"),
        }
    }
    let sending = sender
        .sessions()
        .open(SessionRequest::new("human-client", Some(ep("human")), both).expect("in bounds"))
        .await
        .expect("s leases");

    let ends = Ends {
        sending: &sending,
        at_work: &at_work,
        at_human: &at_human,
        r: &r,
        s: &s,
        work: &work,
        tag: std::cell::Cell::new(0),
    };
    ends.check("before the faults").await;

    receiver.fail_publishes(1).await.expect("queued");
    receiver
        .fail_overlay_writes(vec![Clean, BeforeRename])
        .await
        .expect("queued");
    assert_eq!(
        port.remove_endpoint(work.clone()).await,
        Err(TransportError::Internal)
    );
    assert!(
        row_of(&port, &work)
            .await
            .is_some_and(|row| row.lease.is_some()),
        "the runtime still serves and leases work"
    );
    assert_eq!(
        port.set_endpoint_trust(ep("human"), bystander, true).await,
        Ok(TrustChange::Set),
        "a trust write that republishes the sources, without work's set"
    );
    ends.check("after the trust write").await;
    drop((sending, at_work, at_human, port));
}

/// The four sends of one phase: the sender to `work` and to `human`, and
/// `work` and `human` to the sender.
struct Ends<'a> {
    sending: &'a InProcessSession,
    at_work: &'a InProcessSession,
    at_human: &'a InProcessSession,
    r: &'a TransportIdentity,
    s: &'a TransportIdentity,
    work: &'a EndpointId,
    tag: std::cell::Cell<u8>,
}

impl Ends<'_> {
    async fn send(
        &self,
        from: &InProcessSession,
        peer: &TransportIdentity,
        endpoint: Option<&EndpointId>,
    ) -> Result<EndpointId, TransportError> {
        let tag = self.tag.get() + 1;
        self.tag.set(tag);
        tokio::time::timeout(
            PATIENCE,
            from.send_direct(
                DirectDestination {
                    peer: peer.clone(),
                    endpoint: endpoint.cloned(),
                },
                MessageId::from_bytes([tag; 16]),
                Payload::at_ceiling(None, vec![tag]).expect("within the ceiling"),
            ),
        )
        .await
        .expect("answered")
    }

    async fn check(&self, phase: &str) {
        assert_eq!(
            self.send(self.sending, self.r, Some(self.work)).await,
            Err(TransportError::RemoteEndpointUnavailable),
            "{phase}: the sender reaches work"
        );
        assert_eq!(
            self.send(self.sending, self.r, Some(&ep("human"))).await,
            Ok(ep("human")),
            "{phase}: control, the sender reaches human"
        );
        assert_eq!(
            self.send(self.at_work, self.s, None).await,
            Err(TransportError::UnauthorizedPeer),
            "{phase}: work reaches the sender"
        );
        assert_eq!(
            self.send(self.at_human, self.s, None).await,
            Ok(ep("human")),
            "{phase}: control, human reaches the sender"
        );
    }
}

/// An add left ahead of the runtime: its write lands, its publish fails
/// and the restore's write fails before its rename, so the overlay holds
/// the entry the runtime never registered. Answered `Internal`.
async fn add_left_ahead(
    runtime: &ComposedRuntime,
    port: &impl AdminPort,
    key: &str,
) -> Result<EndpointAddChange, TransportError> {
    use interweave_transport_composition::OverlayFault::{BeforeRename, Clean};

    runtime.fail_publishes(1).await.expect("queued");
    runtime
        .fail_overlay_writes(vec![Clean, BeforeRename])
        .await
        .expect("queued");
    port.add_endpoint(key.to_owned(), "work".to_owned(), vec![])
        .await
}

/// A lease claim on `endpoint`, its error when refused.
async fn lease(runtime: &ComposedRuntime, endpoint: &EndpointId) -> Result<(), TransportError> {
    runtime
        .sessions()
        .open(
            SessionRequest::new(
                "human-client",
                Some(endpoint.clone()),
                [DataCapability::Events],
            )
            .expect("in bounds"),
        )
        .await
        .map(drop)
}

/// The retry of an add left ahead (LOCAL-IPC.md, `admin.endpoints.add`)
/// is judged by the overlay, which holds the entry: `unchanged`, nothing
/// written -- and the endpoint registered by it, leasable from the answer
/// on. The control: after the `Internal` and before the retry, the
/// runtime lists no such endpoint and a lease claim is `EndpointUnknown`.
/// Fed what a client holds: the same key, alias and kinds sent again.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_retried_add_left_ahead_registers_the_endpoint() {
    let p = paths();
    let (identity, _) = id();
    let runtime = ComposedRuntime::start(&identity, &profile(), options(&p))
        .await
        .expect("composes");
    let port = admin(&runtime).await;
    let (work, key) = mint(&p);
    assert_eq!(
        add_left_ahead(&runtime, &port, &key).await,
        Err(TransportError::Internal)
    );
    assert!(row_of(&port, &work).await.is_none(), "control: not served");
    assert_eq!(
        lease(&runtime, &work).await,
        Err(TransportError::EndpointUnknown),
        "control: not leasable before the retry"
    );
    let written = bytes(&p);
    assert_eq!(
        port.add_endpoint(key, "work".to_owned(), vec![]).await,
        Ok(EndpointAddChange::Unchanged(work.clone()))
    );
    assert_eq!(bytes(&p), written, "nothing written");
    let row = row_of(&port, &work).await.expect("served after the retry");
    assert_eq!(row.alias.as_deref(), Some("work"));
    assert_eq!(lease(&runtime, &work).await, Ok(()), "leasable");
    drop(port);
    runtime.stop().await.expect("stops");
}

/// A remove of an add left ahead (LOCAL-IPC.md, `admin.endpoints.
/// remove`): the overlay holds the entry, so it is `removed`, no lease to
/// revoke, and the endpoint does not come back at the next start. The
/// control: before the restart the runtime never listed it, and the
/// overlay on disk held it until the remove.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_remove_of_an_add_left_ahead_is_removed_and_gone_at_the_restart() {
    let p = paths();
    let (identity, _) = id();
    let runtime = ComposedRuntime::start(&identity, &profile(), options(&p))
        .await
        .expect("composes");
    let port = admin(&runtime).await;
    let (work, key) = mint(&p);
    assert_eq!(
        add_left_ahead(&runtime, &port, &key).await,
        Err(TransportError::Internal)
    );
    assert!(row_of(&port, &work).await.is_none(), "control: not served");
    let on_disk = |p: &endpoint_set_common::Profile| {
        String::from_utf8(bytes(p).expect("an overlay")).expect("utf-8")
    };
    assert!(on_disk(&p).contains(work.as_str()), "control: held on disk");
    assert_eq!(
        port.remove_endpoint(work.clone()).await,
        Ok(EndpointRemoveChange::Removed {
            revoked_epoch: None
        })
    );
    assert!(!on_disk(&p).contains(work.as_str()), "written without it");
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
    assert_eq!(
        lease(&again, &work).await,
        Err(TransportError::EndpointUnknown)
    );
    drop(port);
    again.stop().await.expect("stops");
}
