// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `transport/libp2p/CONNECTIVITY.md` §14 on the wire, from both of the
//! detector's sources: the listeners' bound set and the platform's view.
//!
//! Item 5: a network change CLOSES every connection running from an IP
//! it took off this host, and keeps the rest (the rule since
//! 2026-09-26, SPIKE-004 phase B's `ifchange` row). On one host an
//! interface going away is a listener going away -- the same listener
//! events the OS raises -- so the subject holds two listeners on this
//! host's private address and drops them in turn:
//!
//! - the first, while the second still carries the IP: the host is on
//!   the same network, so NO change is reported and NOTHING closes --
//!   the control, and the case a match on the removed address alone
//!   would get wrong;
//! - the second: the IP departs, and both connections running from it
//!   close -- one that arrived on the first listener, and one the
//!   subject DIALLED, whose local IP libp2p never reports and the
//!   runtime reads from the kernel's route;
//! - a connection over loopback, which no removal touched, stays.
//!
//! The platform's view (§20 step 5) does the same with every listener
//! still bound -- the hand-over seen before the listener poll -- and an
//! ADDITION it reports makes a peer held off by its dial backoff
//! dialable at once (architect-cto's ruling of 2026-10-09, relay seq
//! 33736), where without a view the peer waits its backoff.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_libp2p::{NetworkView, SubstrateConfig, SwarmEvent, SwarmRuntime};
use interweave_transport_runtime::TrustSources;
use interweave_trust_api::{InfrastructureSet, PeerTrustPolicy};
use libp2p::Multiaddr;

const PATIENCE: Duration = Duration::from_secs(20);
const WINDOW: Duration = Duration::from_secs(2);

fn trusting(peers: &[&TransportIdentity]) -> TrustSources {
    TrustSources::new(
        PeerTrustPolicy::new(peers.iter().map(|p| (*p).clone())).expect("a small allowlist"),
        InfrastructureSet::default(),
    )
}

struct Node {
    runtime: SwarmRuntime,
    peer: TransportIdentity,
}

fn node(trusted: &[&TransportIdentity], id: &ProfileIdentity) -> Node {
    Node {
        runtime: SwarmRuntime::start(id, SubstrateConfig::default(), trusting(trusted))
            .expect("starts"),
        peer: id.transport_identity().expect("peer id"),
    }
}

/// Drive every runtime for `window`, or until `pred` matches one of the
/// SUBJECT's events; returns the subject's events.
async fn drive(
    subject: &mut SwarmRuntime,
    others: &mut [&mut SwarmRuntime],
    what: &str,
    window: Duration,
    mut pred: Option<impl FnMut(&[SwarmEvent]) -> bool>,
) -> Vec<SwarmEvent> {
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + window;
    loop {
        if let Some(p) = pred.as_mut()
            && p(&events)
        {
            return events;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            assert!(pred.is_none(), "timed out waiting for {what}: {events:?}");
            return events;
        }
        let next_other = async {
            let futures: Vec<_> = others
                .iter_mut()
                .map(|o| Box::pin(o.next_event()))
                .collect();
            futures::future::select_all(futures).await
        };
        tokio::select! {
            event = subject.next_event() => events.push(event.expect("the subject is alive")),
            _ = next_other => {}
            () = tokio::time::sleep(remaining) => {}
        }
    }
}

fn disconnected(events: &[SwarmEvent], peer: &TransportIdentity) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SwarmEvent::Disconnected { peer: p, .. } if p == peer))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_removal_closes_what_ran_from_the_departed_ip_and_keeps_the_rest() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, arriving_id, dialled_id, loopback_id) = (
        ProfileIdentity::generate(),
        ProfileIdentity::generate(),
        ProfileIdentity::generate(),
        ProfileIdentity::generate(),
    );
    let subject_peer = subject_id.transport_identity().expect("peer id");
    let (arriving_peer, dialled_peer, loopback_peer) = (
        arriving_id.transport_identity().expect("peer id"),
        dialled_id.transport_identity().expect("peer id"),
        loopback_id.transport_identity().expect("peer id"),
    );
    let mut subject = node(
        &[&arriving_peer, &dialled_peer, &loopback_peer],
        &subject_id,
    );
    let mut arriving = node(&[&subject_peer], &arriving_id);
    let mut dialled = node(&[&subject_peer], &dialled_id);
    let mut on_loopback = node(&[&subject_peer], &loopback_id);

    let private: Multiaddr = format!("/ip4/{ip}/tcp/0").parse().expect("valid");
    let first = subject
        .runtime
        .listen(private.clone())
        .await
        .expect("the subject's first private listener");
    let second = subject
        .runtime
        .listen(private.clone())
        .await
        .expect("the subject's second private listener");
    let dialled_addr = dialled
        .runtime
        .listen(private)
        .await
        .expect("the dialled peer listens on the private address");
    let loopback_addr = on_loopback
        .runtime
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("the loopback peer listens");

    // THREE CONNECTIONS: one arriving on the FIRST listener, one the
    // subject dials from the private address, one over loopback.
    arriving
        .runtime
        .dial(subject_peer.clone(), first.clone())
        .await
        .expect("reaches the task")
        .expect("admitted");
    subject
        .runtime
        .dial(dialled.peer.clone(), dialled_addr)
        .await
        .expect("reaches the task")
        .expect("admitted");
    subject
        .runtime
        .dial(on_loopback.peer.clone(), loopback_addr)
        .await
        .expect("reaches the task")
        .expect("admitted");
    let peers = [
        arriving.peer.clone(),
        dialled.peer.clone(),
        on_loopback.peer.clone(),
    ];
    let mut others = [
        &mut arriving.runtime,
        &mut dialled.runtime,
        &mut on_loopback.runtime,
    ];
    drive(
        &mut subject.runtime,
        &mut others,
        "all three connected",
        PATIENCE,
        Some(|events: &[SwarmEvent]| {
            peers.iter().all(|p| {
                events
                    .iter()
                    .any(|e| matches!(e, SwarmEvent::Connected { peer, .. } if peer == p))
            })
        }),
    )
    .await;

    // THE CONTROL: the first listener goes, the second still carries the
    // IP. The host is on the same network: no change, and nothing closes.
    // The listener's own close is what shows the window was live.
    assert!(
        subject
            .runtime
            .stop_listening(first.clone())
            .await
            .expect("reaches the task")
    );
    let mut events = drive(
        &mut subject.runtime,
        &mut others,
        "the first listener's close",
        PATIENCE,
        Some(|events: &[SwarmEvent]| {
            events
                .iter()
                .any(|e| matches!(e, SwarmEvent::ListeningStopped { addresses, .. } if addresses.contains(&first)))
        }),
    )
    .await;
    events.extend(
        drive(
            &mut subject.runtime,
            &mut others,
            "settling",
            WINDOW,
            None::<fn(&[SwarmEvent]) -> bool>,
        )
        .await,
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SwarmEvent::NetworkChanged { .. })),
        "the IP is still bound, so the network did not move: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SwarmEvent::Disconnected { .. })),
        "the IP is still bound, so nothing closed -- not even the connection that arrived on \
         the listener that went: {events:?}"
    );

    // THE RULE: the second goes, the IP departs, and what ran from it
    // closes -- arrived and dialled alike -- while loopback stays.
    assert!(
        subject
            .runtime
            .stop_listening(second.clone())
            .await
            .expect("reaches the task")
    );
    let mut events = drive(
        &mut subject.runtime,
        &mut others,
        "both private connections closed",
        PATIENCE,
        Some(|events: &[SwarmEvent]| {
            disconnected(events, &peers[0]) && disconnected(events, &peers[1])
        }),
    )
    .await;
    events.extend(
        drive(
            &mut subject.runtime,
            &mut others,
            "settling",
            WINDOW,
            None::<fn(&[SwarmEvent]) -> bool>,
        )
        .await,
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SwarmEvent::NetworkChanged { removed, .. }
            if *removed == vec![std::net::IpAddr::from(ip)])),
        "{events:?}"
    );
    assert!(
        !disconnected(&events, &peers[2]),
        "the loopback connection is over no departed IP and stays: {events:?}"
    );

    for n in [subject, arriving, dialled, on_loopback] {
        n.runtime.shutdown().await.expect("shutdown");
    }
}

/// The platform's view, with every listener still bound: a first view
/// adding an address closes nothing, and an empty one -- offline -- takes
/// the private IP off the host, so what ran from it closes and
/// the loopback connection stays. The hand-over seen before the listener
/// poll, which on a device is 10 s behind or never comes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_view_without_the_ip_closes_what_ran_from_it_while_its_listener_is_still_bound() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, arriving_id, loopback_id) = (
        ProfileIdentity::generate(),
        ProfileIdentity::generate(),
        ProfileIdentity::generate(),
    );
    let subject_peer = subject_id.transport_identity().expect("peer id");
    let (arriving_peer, loopback_peer) = (
        arriving_id.transport_identity().expect("peer id"),
        loopback_id.transport_identity().expect("peer id"),
    );
    let mut subject = node(&[&arriving_peer, &loopback_peer], &subject_id);
    let mut arriving = node(&[&subject_peer], &arriving_id);
    let mut on_loopback = node(&[&subject_peer], &loopback_id);
    let private = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the subject's private listener");
    let loopback_addr = on_loopback
        .runtime
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("the loopback peer listens");
    arriving
        .runtime
        .dial(subject_peer.clone(), private)
        .await
        .expect("reaches the task")
        .expect("admitted");
    subject
        .runtime
        .dial(loopback_peer.clone(), loopback_addr)
        .await
        .expect("reaches the task")
        .expect("admitted");
    let peers = [arriving_peer.clone(), loopback_peer.clone()];
    let mut others = [&mut arriving.runtime, &mut on_loopback.runtime];
    drive(
        &mut subject.runtime,
        &mut others,
        "both connected",
        PATIENCE,
        Some(|events: &[SwarmEvent]| {
            peers.iter().all(|p| {
                events
                    .iter()
                    .any(|e| matches!(e, SwarmEvent::Connected { peer, .. } if peer == p))
            })
        }),
    )
    .await;

    // The first view must be READ before the empty one is sent -- the
    // latest view replaces one not yet read, and a first view removes
    // nothing it never named -- so it names an address the listeners do
    // not, and its reading is the change it reports. Nothing closes for
    // it: an addition invalidates nothing. (A view that only agrees with
    // the listeners is no change: `the_two_sources_report_one_move_once`.)
    let extra = std::net::IpAddr::from(std::net::Ipv4Addr::new(10, 255, 0, 6));
    subject.runtime.network_changed(NetworkView::new(vec![
        ip.into(),
        "127.0.0.1".parse().expect("an ip"),
        extra,
    ]));
    let events = drive(
        &mut subject.runtime,
        &mut others,
        "the first view read",
        PATIENCE,
        Some(|events: &[SwarmEvent]| {
            events
                .iter()
                .any(|e| matches!(e, SwarmEvent::NetworkChanged { .. }))
        }),
    )
    .await;
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SwarmEvent::Disconnected { .. })),
        "an addition closes nothing: {events:?}"
    );

    // OFFLINE: the private IP departs though its listener is bound.
    subject.runtime.network_changed(NetworkView::default());
    let mut events = drive(
        &mut subject.runtime,
        &mut others,
        "the arrived connection closed",
        PATIENCE,
        Some(|events: &[SwarmEvent]| disconnected(events, &arriving_peer)),
    )
    .await;
    events.extend(
        drive(
            &mut subject.runtime,
            &mut others,
            "settling",
            WINDOW,
            None::<fn(&[SwarmEvent]) -> bool>,
        )
        .await,
    );
    assert!(
        events.iter().any(
            |e| matches!(e, SwarmEvent::NetworkChanged { removed, added }
            if removed.contains(&std::net::IpAddr::from(ip)) && added.is_empty())
        ),
        "{events:?}"
    );
    assert!(
        !disconnected(&events, &loopback_peer),
        "the loopback connection is over no departed IP and stays: {events:?}"
    );

    for n in [subject, arriving, on_loopback] {
        n.runtime.shutdown().await.expect("shutdown");
    }
}

/// A view that ADDS an address makes a peer held off by its dial backoff
/// dialable at once: the scheduler redials it within a tick, through the
/// gate, long before its 30 s retry -- and without the view, the control,
/// it waits. The control's window is longer than conntrack's 10 s CLOSE
/// state for the failed dial's port pair, so the redial is not dropped
/// before TCP sees it (a same-port redial inside that window hangs).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_addition_redials_a_held_off_peer_at_once_and_without_one_it_waits() {
    const CONTROL: Duration = Duration::from_secs(12);
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, far_id) = (ProfileIdentity::generate(), ProfileIdentity::generate());
    let subject_peer = subject_id.transport_identity().expect("peer id");
    let far_peer = far_id.transport_identity().expect("peer id");
    let mut subject = node(&[&far_peer], &subject_id);
    let mut far = node(&[&subject_peer], &far_id);
    // The subject's private listener fills its set; the view later adds
    // to it.
    let _private = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the subject's private listener");
    let far_addr = far
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the far peer listens");
    assert!(
        far.runtime
            .stop_listening(far_addr.clone())
            .await
            .expect("reaches the task")
    );

    // THE FAILURE: the far peer is not there, so the dial is refused and
    // the peer held off for the first retry's 30 s.
    subject
        .runtime
        .dial(far_peer.clone(), far_addr.clone())
        .await
        .expect("reaches the task")
        .expect("admitted");
    let mut others = [&mut far.runtime];
    drive(
        &mut subject.runtime,
        &mut others,
        "the dial to fail",
        PATIENCE,
        Some(|events: &[SwarmEvent]| {
            events.iter().any(
                |e| matches!(e, SwarmEvent::DialFailed { peer: Some(p), .. } if *p == far_peer),
            )
        }),
    )
    .await;
    let failed_at = tokio::time::Instant::now();
    // Back where it was.
    let _ = others[0]
        .listen(far_addr.clone())
        .await
        .expect("the far peer listens again on the same address");

    // THE CONTROL: no view, and the peer waits its backoff.
    let events = drive(
        &mut subject.runtime,
        &mut others,
        "the control window",
        CONTROL,
        None::<fn(&[SwarmEvent]) -> bool>,
    )
    .await;
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SwarmEvent::Connected { peer, .. } if *peer == far_peer)),
        "without a change the peer waits its backoff: {events:?}"
    );

    // THE ADDITION: redialled at once.
    let added = std::net::IpAddr::from(std::net::Ipv4Addr::new(10, 255, 0, 1));
    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into(), added]));
    let events = drive(
        &mut subject.runtime,
        &mut others,
        "the held-off peer redialled",
        PATIENCE,
        Some(|events: &[SwarmEvent]| {
            events
                .iter()
                .any(|e| matches!(e, SwarmEvent::Connected { peer, .. } if *peer == far_peer))
        }),
    )
    .await;
    assert!(
        failed_at.elapsed() < Duration::from_secs(30),
        "connected inside the backoff, so the change made it due: {:?}",
        failed_at.elapsed()
    );
    assert!(
        events.iter().any(
            |e| matches!(e, SwarmEvent::NetworkChanged { removed, added: a }
            if removed.is_empty() && *a == vec![added])
        ),
        "{events:?}"
    );

    for n in [subject, far] {
        n.runtime.shutdown().await.expect("shutdown");
    }
}

/// SPIKE-008 L7, plan §20 step 5: a Wi-Fi reconnect brings the SAME
/// network back as a new one, and it must re-bind the dials. The
/// platform's view loses the private IP (offline), a dial made while
/// offline fails and holds the peer off for its 30 s retry; the view then
/// names that same IP and nothing else, which is reported as an ADDITION
/// of it alone, and the held-off peer is redialled at once. The control:
/// the same window with the empty view still in force, in which the peer
/// waits -- longer than conntrack's 10 s CLOSE state for the failed
/// dial's port pair, so the redial is not dropped before TCP sees it. The
/// return carries no second address, as on a device in L7, so a detector
/// that kept the IP across the empty view would report no addition and
/// leave the peer waiting out its backoff.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_same_ip_returning_after_offline_redials_a_peer_that_failed_while_offline() {
    const CONTROL: Duration = Duration::from_secs(12);
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, far_id) = (ProfileIdentity::generate(), ProfileIdentity::generate());
    let subject_peer = subject_id.transport_identity().expect("peer id");
    let far_peer = far_id.transport_identity().expect("peer id");
    let mut subject = node(&[&far_peer], &subject_id);
    let mut far = node(&[&subject_peer], &far_id);
    let _private = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the subject's private listener");
    let far_addr = far
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the far peer listens");
    assert!(
        far.runtime
            .stop_listening(far_addr.clone())
            .await
            .expect("reaches the task")
    );
    let changed = |pred: fn(&[std::net::IpAddr], &[std::net::IpAddr]) -> bool| {
        move |events: &[SwarmEvent]| {
            events.iter().any(|e| {
                matches!(e,
                SwarmEvent::NetworkChanged { removed, added } if pred(removed, added))
            })
        }
    };
    let mut others = [&mut far.runtime];

    // ONLINE, then OFFLINE: each view read before the next is handed in
    // (the latest view replaces one not yet read).
    let lo: std::net::IpAddr = "127.0.0.1".parse().expect("an ip");
    let extra = std::net::IpAddr::from(std::net::Ipv4Addr::new(10, 255, 0, 7));
    let online = NetworkView::new(vec![ip.into(), lo, extra]);
    subject.runtime.network_changed(online.clone());
    drive(
        &mut subject.runtime,
        &mut others,
        "the first view read",
        PATIENCE,
        Some(changed(|_, added| !added.is_empty())),
    )
    .await;
    subject.runtime.network_changed(NetworkView::default());
    drive(
        &mut subject.runtime,
        &mut others,
        "offline read",
        PATIENCE,
        Some(|events: &[SwarmEvent]| {
            events.iter().any(|e| {
                matches!(e,
                SwarmEvent::NetworkChanged { removed, .. }
                if removed.contains(&std::net::IpAddr::from(ip)))
            })
        }),
    )
    .await;

    // A DIAL WHILE OFFLINE fails: the far peer is not there either, and
    // the peer is held off for the first retry's 30 s.
    subject
        .runtime
        .dial(far_peer.clone(), far_addr.clone())
        .await
        .expect("reaches the task")
        .expect("admitted");
    drive(
        &mut subject.runtime,
        &mut others,
        "the dial to fail",
        PATIENCE,
        Some(|events: &[SwarmEvent]| {
            events.iter().any(
                |e| matches!(e, SwarmEvent::DialFailed { peer: Some(p), .. } if *p == far_peer),
            )
        }),
    )
    .await;
    let failed_at = tokio::time::Instant::now();
    let _ = others[0]
        .listen(far_addr.clone())
        .await
        .expect("the far peer listens again on the same address");
    let connected = |events: &[SwarmEvent]| {
        events
            .iter()
            .any(|e| matches!(e, SwarmEvent::Connected { peer, .. } if *peer == far_peer))
    };

    // THE CONTROL: still offline, and the peer waits its backoff.
    let events = drive(
        &mut subject.runtime,
        &mut others,
        "the control window",
        CONTROL,
        None::<fn(&[SwarmEvent]) -> bool>,
    )
    .await;
    assert!(!connected(&events), "offline, the peer waits: {events:?}");

    // BACK ON THE SAME NETWORK: the same IP alone, reported as added,
    // and the peer redialled at once -- nothing else returns to carry the
    // lift.
    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into()]));
    let events = drive(
        &mut subject.runtime,
        &mut others,
        "the held-off peer redialled",
        PATIENCE,
        Some(connected),
    )
    .await;
    assert!(
        failed_at.elapsed() < Duration::from_secs(30),
        "connected inside the backoff, so the return made it due: {:?}",
        failed_at.elapsed()
    );
    assert!(
        events.iter().any(
            |e| matches!(e, SwarmEvent::NetworkChanged { removed, added }
            if removed.is_empty() && *added == vec![std::net::IpAddr::from(ip)])
        ),
        "the same IP returning is an addition: {events:?}"
    );

    for n in [subject, far] {
        n.runtime.shutdown().await.expect("shutdown");
    }
}

/// The view is a snapshot slot, not a queue: two views handed in before
/// the runtime reads either count as the LATEST one, so the change is
/// computed against it alone -- the first view's address is never
/// reported -- and nothing blocks the caller. On the current-thread
/// runtime the task cannot run between the two reports.
#[tokio::test]
async fn the_latest_view_replaces_one_not_yet_read() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let id = ProfileIdentity::generate();
    let mut subject = node(&[], &id);
    let _private = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the subject's private listener");
    let (superseded, latest) = (
        std::net::IpAddr::from(std::net::Ipv4Addr::new(10, 255, 0, 1)),
        std::net::IpAddr::from(std::net::Ipv4Addr::new(10, 255, 0, 2)),
    );
    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into(), superseded]));
    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into(), latest]));
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + WINDOW;
    while let Ok(Some(event)) =
        tokio::time::timeout_at(deadline, subject.runtime.next_event()).await
    {
        events.push(event);
    }
    let changes: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, SwarmEvent::NetworkChanged { .. }))
        .collect();
    assert!(
        matches!(changes.as_slice(), [SwarmEvent::NetworkChanged { removed, added }]
            if removed.is_empty() && *added == vec![latest]),
        "one change, against the latest view only: {changes:?}"
    );
    subject.runtime.shutdown().await.expect("shutdown");
}

/// Read `runtime`'s events until it reports a network change whose
/// removed IPs satisfy `removed`.
async fn until_changed(runtime: &mut SwarmRuntime, removed: impl Fn(&[std::net::IpAddr]) -> bool) {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        let event = tokio::time::timeout_at(deadline, runtime.next_event())
            .await
            .expect("the network change within the patience")
            .expect("the runtime is alive");
        if matches!(&event, SwarmEvent::NetworkChanged { removed: r, .. } if removed(r)) {
            return;
        }
    }
}

/// ADR-0052 rule 3 (A 2026-10-09) at a runtime learn site: a peer's
/// private candidate is admitted beside a private listener on an IP the
/// host holds -- the control -- and refused once the platform's view
/// says that IP departed, though its listener is still bound.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_departed_ips_listener_admits_no_private_candidate() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, far_id) = (ProfileIdentity::generate(), ProfileIdentity::generate());
    let far_peer = far_id.transport_identity().expect("peer id");
    let mut subject = node(&[&far_peer], &subject_id);
    let _private = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the subject's private listener");
    // The first view must be READ before the second is sent -- the latest
    // view replaces one not yet read, and a first view removes nothing it
    // never named -- so it names an address the listeners do not, and its
    // reading is the change it reports.
    subject.runtime.network_changed(NetworkView::new(vec![
        ip.into(),
        std::net::Ipv4Addr::new(10, 255, 0, 4).into(),
    ]));
    until_changed(&mut subject.runtime, |_| true).await;
    let admitted = subject
        .runtime
        .learn(far_peer.clone(), ["/ip4/10.1.2.3/tcp/4001".to_owned()])
        .await
        .expect("reaches the task");
    assert_eq!(admitted, 1, "the control: beside a held private listener");

    subject.runtime.network_changed(NetworkView::default());
    // The view and a command reach the task on two channels, so wait for
    // the removal the view makes -- the proof it was read -- before
    // asking; a timed settle lost that race under a loaded suite.
    let removal = std::net::IpAddr::from(ip);
    until_changed(&mut subject.runtime, |removed| removed.contains(&removal)).await;
    let admitted = subject
        .runtime
        .learn(far_peer.clone(), ["/ip4/10.1.2.4/tcp/4001".to_owned()])
        .await
        .expect("reaches the task");
    assert_eq!(
        admitted, 0,
        "the listener's IP departed: no private listener of the family is held"
    );
    subject.runtime.shutdown().await.expect("shutdown");
}

/// ADR-0052 rule 3's boundary WIDENING is counted where a caller outside
/// the Swarm task can read it (`SwarmRuntime::listener_widenings`), as
/// a wildcard listener makes it at startup: its loopback address reported
/// first and its LAN address after. A private candidate offered between
/// the two is refused and the count stands; the LAN listener binding
/// moves the count, and the same candidate offered again is admitted --
/// the move a caller remembering its offer (composition's discovery
/// memo) must see, or it keeps the refusal for its relearn interval
/// (j133: the android-e2e cache-alone row, 1 run in 24 under load).
/// Each reading follows a `learn` reply, which the task sends only
/// after it handled the listener event before it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_private_listener_bound_after_a_refusal_widens_the_boundary_and_admits_it() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, far_id) = (ProfileIdentity::generate(), ProfileIdentity::generate());
    let far_peer = far_id.transport_identity().expect("peer id");
    let subject = node(&[&far_peer], &subject_id);
    let widenings = subject.runtime.listener_widenings();
    let candidate = || ["/ip4/10.1.2.3/tcp/4001".to_owned()];
    let _loopback = subject
        .runtime
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("the loopback listener");
    let admitted = subject
        .runtime
        .learn(far_peer.clone(), candidate())
        .await
        .expect("reaches the task");
    assert_eq!(admitted, 0, "loopback alone is no private listener");
    let before = widenings.count();

    let _private = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the subject's private listener");
    let admitted = subject
        .runtime
        .learn(far_peer.clone(), candidate())
        .await
        .expect("reaches the task");
    assert_eq!(admitted, 1, "the control: beside a private listener");
    assert!(
        widenings.count() > before,
        "the boundary widened and the count did not move ({before})"
    );

    let settled = widenings.count();
    let _second_loopback = subject
        .runtime
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("a second loopback listener");
    let _ = subject
        .runtime
        .learn(far_peer.clone(), candidate())
        .await
        .expect("reaches the task");
    assert_eq!(
        widenings.count(),
        settled,
        "a loopback listener widens nothing"
    );
    subject.runtime.shutdown().await.expect("shutdown");
}

/// The Identify sibling of the discovery memo's refusal: a connected
/// peer's advertised LAN address, refused while the subject listened on
/// loopback alone, enters the subject's book when its own private
/// listener binds -- at the widening, not at the peer's next Identify,
/// which the crate sends every five minutes. Read from the subject's
/// address-book counts: the refusal is counted first (the control that
/// the Identify arrived and rule 3 judged it), and the admission after
/// the listen.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_peers_lan_address_refused_by_rule_3_is_learned_when_a_private_listener_binds() {
    use interweave_transport_libp2p::store_refusals::store;
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, far_id) = (ProfileIdentity::generate(), ProfileIdentity::generate());
    let (subject_peer, far_peer) = (
        subject_id.transport_identity().expect("peer id"),
        far_id.transport_identity().expect("peer id"),
    );
    let mut subject = node(&[&far_peer], &subject_id);
    let mut far = node(&[&subject_peer], &far_id);
    let _far_lan = far
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the far end's LAN listener");
    let subject_loopback = subject
        .runtime
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("the subject's loopback listener");
    let book = |runtime: &SwarmRuntime| {
        runtime
            .store_refusals()
            .get(store::ADDRESS_BOOK)
            .cloned()
            .unwrap_or_default()
    };
    let refused = |runtime: &SwarmRuntime| {
        book(runtime)
            .refused
            .get("private_without_private_listener")
            .copied()
            .unwrap_or(0)
    };
    assert!(
        matches!(
            far.runtime
                .dial(subject_peer.clone(), subject_loopback)
                .await,
            Ok(Ok(()))
        ),
        "the far end dials the subject over loopback"
    );
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while refused(&subject.runtime) == 0 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the far end's Identify was never judged: {:?}",
            book(&subject.runtime)
        );
        drive(
            &mut subject.runtime,
            &mut [&mut far.runtime],
            "identify",
            Duration::from_millis(100),
            None::<fn(&[SwarmEvent]) -> bool>,
        )
        .await;
    }
    let admitted = book(&subject.runtime).admitted;

    let _subject_lan = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the subject's LAN listener");
    // A command after the listen: the task handled the listener event,
    // and the re-judgement it runs, before answering it.
    let _ = subject.runtime.status(None).await.expect("answered");
    assert_eq!(
        book(&subject.runtime).admitted,
        admitted + 1,
        "the far end's LAN address entered the book at the widening: {:?}",
        book(&subject.runtime)
    );
    subject.runtime.shutdown().await.expect("shutdown");
    far.runtime.shutdown().await.expect("shutdown");
}

/// The same refusal, and then the far end LEAVES before the subject's
/// private listener binds: what its Identify had refused is forgotten
/// with its last connection (`HeldBack::closed`, review F2 of #468), so
/// the widening judges nothing again and the book admits nothing. The
/// test above, where the far end stays, is the positive control: the
/// same widening admits its LAN address.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_peer_that_left_is_not_judged_again_when_a_private_listener_binds() {
    use interweave_transport_libp2p::store_refusals::store;
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, far_id) = (ProfileIdentity::generate(), ProfileIdentity::generate());
    let (subject_peer, far_peer) = (
        subject_id.transport_identity().expect("peer id"),
        far_id.transport_identity().expect("peer id"),
    );
    let mut subject = node(&[&far_peer], &subject_id);
    let mut far = node(&[&subject_peer], &far_id);
    let _far_lan = far
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the far end's LAN listener");
    let subject_loopback = subject
        .runtime
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("the subject's loopback listener");
    let book = |runtime: &SwarmRuntime| {
        runtime
            .store_refusals()
            .get(store::ADDRESS_BOOK)
            .cloned()
            .unwrap_or_default()
    };
    assert!(
        matches!(
            far.runtime
                .dial(subject_peer.clone(), subject_loopback)
                .await,
            Ok(Ok(()))
        ),
        "the far end dials the subject over loopback"
    );
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while !book(&subject.runtime)
        .refused
        .contains_key("private_without_private_listener")
    {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the far end's Identify was never judged: {:?}",
            book(&subject.runtime)
        );
        drive(
            &mut subject.runtime,
            &mut [&mut far.runtime],
            "identify",
            Duration::from_millis(100),
            None::<fn(&[SwarmEvent]) -> bool>,
        )
        .await;
    }
    let admitted = book(&subject.runtime).admitted;

    far.runtime.shutdown().await.expect("shutdown");
    tokio::time::timeout(PATIENCE, async {
        loop {
            match subject.runtime.next_event().await {
                Some(SwarmEvent::Disconnected { peer, .. }) if peer == far_peer => return,
                Some(_) => {}
                None => panic!("the subject stopped"),
            }
        }
    })
    .await
    .expect("the far end's disconnect");
    let _subject_lan = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the subject's LAN listener");
    let _ = subject.runtime.status(None).await.expect("answered");
    assert_eq!(
        book(&subject.runtime).admitted,
        admitted,
        "a peer gone was judged again at the widening: {:?}",
        book(&subject.runtime)
    );
    subject.runtime.shutdown().await.expect("shutdown");
}

/// What `subject` advertises as its listen addresses to a fresh observer
/// holding `keys` -- a peer the subject trusts, or it closes the
/// connection before Identify runs -- that dials it at `address`: the
/// Identify exchange's `listen_addrs`. A new swarm each time, so each
/// reading is one exchange made now.
async fn advertised(
    keys: &libp2p::identity::Keypair,
    subject: &TransportIdentity,
    address: Multiaddr,
) -> Vec<Multiaddr> {
    use futures::StreamExt as _;
    let mut observer = libp2p::SwarmBuilder::with_existing_identity(keys.clone())
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("the subject's transport stack")
        .with_behaviour(|k| {
            libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
                "/interweave-test-observer/1".to_owned(),
                k.public(),
            ))
        })
        .expect("behaviour")
        .build();
    let peer: libp2p::PeerId = subject.as_str().parse().expect("a libp2p identity");
    observer
        .dial(
            libp2p::swarm::dial_opts::DialOpts::peer_id(peer)
                .addresses(vec![address])
                .build(),
        )
        .expect("dial accepted");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        match tokio::time::timeout_at(deadline, observer.select_next_some())
            .await
            .expect("an Identify exchange within the patience")
        {
            libp2p::swarm::SwarmEvent::Behaviour(libp2p::identify::Event::Received {
                info,
                ..
            }) => return info.listen_addrs,
            libp2p::swarm::SwarmEvent::OutgoingConnectionError { error, .. } => {
                panic!("the observer could not reach the subject: {error}")
            }
            _ => {}
        }
    }
}

/// ADR-0052 rule 10 (A 2026-10-26) between real runtimes, over this
/// host's private address standing for a phone's carrier-assigned one: a
/// listener bound on it is advertised by Identify, shown, and counted as
/// an own private listener while the platform's view leaves it
/// unclassified -- THE CONTROL -- and is none of the three once the view
/// classifies it as on no local network, while it stays bound and still
/// answers the observer's dial; a view that drops the classification
/// gives all three back, and so does one naming it again after a view
/// dropped it while it stayed bound -- which withholds it as well. The
/// loopback listener is advertised throughout, the in-exchange control.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_private_address_on_no_local_network_is_not_advertised_shown_or_counted() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, far_id) = (ProfileIdentity::generate(), ProfileIdentity::generate());
    let far_peer = far_id.transport_identity().expect("peer id");
    let observer = libp2p::identity::Keypair::generate_ed25519();
    let observer_peer = TransportIdentity::parse(observer.public().to_peer_id().to_base58())
        .expect("a canonical identity");
    let mut subject = node(&[&far_peer, &observer_peer], &subject_id);
    let carrier = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the carrier-address listener");
    let loopback = subject
        .runtime
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("the loopback listener");
    let other: std::net::IpAddr = std::net::Ipv4Addr::new(10, 255, 0, 5).into();
    let shown = |own: &interweave_local_client_api::OwnAddresses| {
        own.addresses
            .iter()
            .any(|row| row.address.starts_with(&format!("/ip4/{ip}/tcp/")))
    };
    let mut candidate = 0u8;
    let mut learns = async |subject: &mut Node| {
        candidate += 1;
        subject
            .runtime
            .learn(
                far_peer.clone(),
                [format!("/ip4/10.1.2.{candidate}/tcp/4001")],
            )
            .await
            .expect("reaches the task")
    };

    // Each view moves the known set by `other`, so its reading is a
    // change the test waits for -- a classification alone reports none.
    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into(), other]));
    until_changed(&mut subject.runtime, |_| true).await;
    let told = advertised(&observer, &subject.peer, carrier.clone()).await;
    assert!(told.contains(&carrier), "the control: advertised, {told:?}");
    assert!(told.contains(&loopback));
    assert!(
        shown(&subject.runtime.own_addresses().await.expect("own")),
        "the control: shown"
    );
    assert_eq!(
        learns(&mut subject).await,
        1,
        "the control: an own private listener"
    );

    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into()]).with_off_local_network(vec![ip.into()]));
    until_changed(&mut subject.runtime, |removed| removed.contains(&other)).await;
    let told = advertised(&observer, &subject.peer, carrier.clone()).await;
    assert!(
        !told.contains(&carrier),
        "classified: not advertised, {told:?}"
    );
    assert!(told.contains(&loopback), "the same exchange's control");
    assert!(
        !shown(&subject.runtime.own_addresses().await.expect("own")),
        "classified: not shown"
    );
    assert_eq!(
        learns(&mut subject).await,
        0,
        "classified: no own private listener"
    );

    // The view drops the classified IP while its listener stays bound,
    // as when the platform reports a carrier network lost before the
    // listener poll does: still not advertised. The observer reaches the
    // subject over loopback, since the departure closes what ran from
    // the carrier's IP.
    subject
        .runtime
        .network_changed(NetworkView::new(vec![other]));
    until_changed(&mut subject.runtime, |removed| removed.contains(&ip.into())).await;
    let told = advertised(&observer, &subject.peer, loopback.clone()).await;
    assert!(
        !told.contains(&carrier),
        "departed while bound: not advertised, {told:?}"
    );
    assert!(told.contains(&loopback), "the same exchange's control");
    assert!(
        !shown(&subject.runtime.own_addresses().await.expect("own")),
        "departed: not shown"
    );
    assert_eq!(
        learns(&mut subject).await,
        0,
        "departed: no own private listener"
    );

    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into(), other]));
    until_changed(&mut subject.runtime, |_| true).await;
    let told = advertised(&observer, &subject.peer, carrier.clone()).await;
    assert!(
        told.contains(&carrier),
        "released: advertised again, {told:?}"
    );
    assert!(
        shown(&subject.runtime.own_addresses().await.expect("own")),
        "released: shown"
    );
    assert_eq!(learns(&mut subject).await, 1, "released: counted again");
    subject.runtime.shutdown().await.expect("shutdown");
}

/// ADR-0052 rule 10 in the other order: the view classifies an IP no
/// listener binds yet -- a carrier network coming up before the listener
/// poll reports its address -- and a listener bound on it afterwards is
/// not advertised by Identify. A view releasing the classification gives
/// it back, THE CONTROL that the late listener is one Identify would
/// advertise; loopback is the in-exchange control.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_listener_bound_after_its_ip_is_classified_is_not_advertised() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let subject_id = ProfileIdentity::generate();
    let observer = libp2p::identity::Keypair::generate_ed25519();
    let observer_peer = TransportIdentity::parse(observer.public().to_peer_id().to_base58())
        .expect("a canonical identity");
    let mut subject = node(&[&observer_peer], &subject_id);
    let first = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the first listener, which fills the known set");
    let loopback = subject
        .runtime
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("the loopback listener");
    let other: std::net::IpAddr = std::net::Ipv4Addr::new(10, 255, 0, 7).into();
    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into(), other]));
    until_changed(&mut subject.runtime, |_| true).await;
    let first_shown = |own: &interweave_local_client_api::OwnAddresses| {
        own.addresses
            .iter()
            .any(|row| row.address.starts_with(&first.to_string()))
    };
    assert!(
        first_shown(&subject.runtime.own_addresses().await.expect("own")),
        "the control: the first listener is shown while bound"
    );
    assert!(
        subject
            .runtime
            .stop_listening(first.clone())
            .await
            .expect("reaches the task"),
        "the IP is left with no listener, and the view still holds it"
    );
    // The stop is answered before the listener's close reaches the
    // runtime's listener set, and a view read in between would find the
    // IP still bound -- the order this test is not about. What is shown
    // comes from that set, so wait until it no longer shows the listener.
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the first listener's close within the patience"
        );
        if !first_shown(&subject.runtime.own_addresses().await.expect("own")) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    // Each view moves the known set by `other`, so its reading is a
    // change the test waits for.
    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into()]).with_off_local_network(vec![ip.into()]));
    until_changed(&mut subject.runtime, |removed| removed.contains(&other)).await;
    let late = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("a listener bound after the classification");
    let told = advertised(&observer, &subject.peer, loopback.clone()).await;
    assert!(
        !told.contains(&late),
        "classified before it was bound: not advertised, {told:?}"
    );
    assert!(told.contains(&loopback), "the same exchange's control");

    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into(), other]));
    until_changed(&mut subject.runtime, |_| true).await;
    let told = advertised(&observer, &subject.peer, loopback.clone()).await;
    assert!(
        told.contains(&late),
        "the control: released, the late listener is advertised, {told:?}"
    );
    subject.runtime.shutdown().await.expect("shutdown");
}

/// ADR-0052 rule 10's second case (A 2026-10-27) between real runtimes,
/// over this host's private address standing for a phone's carrier
/// address on a network the platform never reports to the app: with NO
/// VIEW -- THE CONTROL, the desktop -- a listener on it is advertised by
/// Identify, shown, and counted as an own private listener; once a view
/// is present that never names it, that listener and a second one bound
/// on the same IP after the view are none of the three, though both stay
/// bound and the first still answers the observer's dial; a view naming
/// the IP gives all three back. The loopback listener is advertised
/// throughout, the in-exchange control.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_private_address_no_view_holds_is_not_advertised_shown_or_counted() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (subject_id, far_id) = (ProfileIdentity::generate(), ProfileIdentity::generate());
    let far_peer = far_id.transport_identity().expect("peer id");
    let observer = libp2p::identity::Keypair::generate_ed25519();
    let observer_peer = TransportIdentity::parse(observer.public().to_peer_id().to_base58())
        .expect("a canonical identity");
    let mut subject = node(&[&far_peer, &observer_peer], &subject_id);
    let carrier = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("the carrier-address listener");
    let loopback = subject
        .runtime
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("valid"))
        .await
        .expect("the loopback listener");
    let (other, third): (std::net::IpAddr, std::net::IpAddr) = (
        std::net::Ipv4Addr::new(10, 255, 0, 8).into(),
        std::net::Ipv4Addr::new(10, 255, 0, 9).into(),
    );
    // Whole components only: `/tcp/3456` must not match `/tcp/34567`.
    let shown = |own: &interweave_local_client_api::OwnAddresses, address: &Multiaddr| {
        let address = address.to_string();
        own.addresses
            .iter()
            .any(|row| row.address == address || row.address.starts_with(&format!("{address}/")))
    };
    let mut candidate = 0u8;
    let mut learns = async |subject: &mut Node| {
        candidate += 1;
        subject
            .runtime
            .learn(
                far_peer.clone(),
                [format!("/ip4/10.1.3.{candidate}/tcp/4001")],
            )
            .await
            .expect("reaches the task")
    };

    // THE CONTROL: no view has arrived.
    let told = advertised(&observer, &subject.peer, carrier.clone()).await;
    assert!(told.contains(&carrier), "no view: advertised, {told:?}");
    assert!(told.contains(&loopback));
    assert!(
        shown(
            &subject.runtime.own_addresses().await.expect("own"),
            &carrier
        ),
        "no view: shown"
    );
    assert_eq!(
        learns(&mut subject).await,
        1,
        "no view: an own private listener"
    );

    // A view that never names the IP; it moves the known set by `other`,
    // so its reading is a change the test waits for.
    subject
        .runtime
        .network_changed(NetworkView::new(vec![other]));
    until_changed(&mut subject.runtime, |_| true).await;
    let late = subject
        .runtime
        .listen(format!("/ip4/{ip}/tcp/0").parse().expect("valid"))
        .await
        .expect("a listener bound on the IP after the view");
    let told = advertised(&observer, &subject.peer, carrier.clone()).await;
    assert!(
        !told.contains(&carrier),
        "outside the view: not advertised, though it answered this dial, {told:?}"
    );
    assert!(
        !told.contains(&late),
        "bound after the view: not advertised, {told:?}"
    );
    assert!(told.contains(&loopback), "the same exchange's control");
    let own = subject.runtime.own_addresses().await.expect("own");
    assert!(!shown(&own, &carrier), "outside the view: not shown");
    assert!(!shown(&own, &late), "bound after the view: not shown");
    assert_eq!(
        learns(&mut subject).await,
        0,
        "outside the view: no own private listener"
    );

    // A view naming the IP; `third` makes its reading a change, since the
    // IP is already known from the listeners.
    subject
        .runtime
        .network_changed(NetworkView::new(vec![ip.into(), other, third]));
    until_changed(&mut subject.runtime, |_| true).await;
    let told = advertised(&observer, &subject.peer, carrier.clone()).await;
    assert!(
        told.contains(&carrier) && told.contains(&late),
        "named by the view: advertised, {told:?}"
    );
    let own = subject.runtime.own_addresses().await.expect("own");
    assert!(shown(&own, &carrier) && shown(&own, &late), "named: shown");
    assert_eq!(learns(&mut subject).await, 1, "named: counted again");
    subject.runtime.shutdown().await.expect("shutdown");
}
