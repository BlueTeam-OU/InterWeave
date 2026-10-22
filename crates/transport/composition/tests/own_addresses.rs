// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The show side through a running composition (ADR-0062 decision 4,
//! `admin.peers.own_addresses`): what a node shows is its bound
//! listeners in the classes an add admits, in bind order, each ending in
//! its own `/p2p/<peer>` -- the loopback listener beside them is not
//! shown -- and what it shows, added on another node as a scan would,
//! is dialled and connects; a node bound on loopback alone shows no
//! listen row (LOCAL-CLIENT.md §7 item 14's control). The read needs
//! `admin.peers`.
//!
//! Confirmed external and circuit rows need an AutoNAT verdict and a
//! held reservation, which a host without a public address cannot
//! produce: their order and the relay-only rule are the selector's unit
//! tests in `transport-libp2p`.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_local_client_api::{
    AddressChange, AdminBinding as _, AdminCapability, AdminPort as _, OwnAddress, OwnAddressKind,
};
use interweave_profile_config::ProfileConfig;
use interweave_profile_config::bootstrap_overlay::BOOTSTRAP_OVERLAY_FILE;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{
    TransportError, TransportEvent, TransportIdentity, TransportRuntime as _,
};
use interweave_transport_composition::{ComposedRuntime, CompositionOptions};

const PATIENCE: Duration = Duration::from_secs(30);

fn id() -> (ProfileIdentity, TransportIdentity) {
    let identity = ProfileIdentity::generate();
    let peer = identity.transport_identity().expect("peer id");
    (identity, peer)
}

fn profile(trusted: &TransportIdentity) -> ProfileConfig {
    let doc = format!(
        "schema_version: 2
trust:
  policy: static-allowlist
  allowed_peers: [\"{}\"]
endpoints:
  entries:
    - id: human
      enabled: true
      advertise: false
discovery:
  providers: []
",
        trusted.as_str()
    );
    serde_norway::from_str(&doc).expect("the document parses")
}

async fn wait_connected(runtime: &mut ComposedRuntime, peer: &TransportIdentity) {
    tokio::time::timeout(PATIENCE, async {
        loop {
            match runtime.next_event().await {
                Some(TransportEvent::PeerConnected { peer: p, .. }) if &p == peer => return,
                Some(_) => {}
                None => panic!("the runtime ended before connecting"),
            }
        }
    })
    .await
    .expect("connected in time");
}

/// A shows its listeners -- two private ones in bind order, the
/// loopback one beside them omitted (the control: A IS bound there) --
/// and B, adding the first line as a scan would, connects to A.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_node_shows_its_admitted_listeners_and_what_it_shows_is_dialled() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let (a_id, a) = id();
    let (b_id, b) = id();
    let shown = ComposedRuntime::start(
        &a_id,
        &profile(&b),
        CompositionOptions {
            listen: vec![
                format!("/ip4/{ip}/tcp/0"),
                "/ip4/127.0.0.1/tcp/0".to_owned(),
                format!("/ip4/{ip}/tcp/0"),
            ],
            ..CompositionOptions::default()
        },
    )
    .await
    .expect("a composes");
    let bound = shown.listening().to_vec();
    assert_eq!(bound.len(), 3, "the control: all three listeners bound");
    assert!(bound[1].starts_with("/ip4/127.0.0.1/"));

    let own = shown
        .sessions()
        .admin([AdminCapability::Peers].into())
        .await
        .expect("a port")
        .own_addresses()
        .await
        .expect("answered");
    assert_eq!(own.peer, a);
    assert!(!own.truncated);
    let suffixed = |listener: &String| OwnAddress {
        address: format!("{listener}/p2p/{}", a.as_str()),
        kind: OwnAddressKind::Listen,
    };
    assert_eq!(
        own.addresses,
        vec![suffixed(&bound[0]), suffixed(&bound[2])]
    );

    let dir = tempfile::tempdir().expect("a temporary directory");
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("chmod");
    }
    let mut scanner = ComposedRuntime::start(
        &b_id,
        &profile(&a),
        CompositionOptions {
            listen: vec![format!("/ip4/{ip}/tcp/0")],
            bootstrap_overlay_file: Some(dir.path().join(BOOTSTRAP_OVERLAY_FILE)),
            ..CompositionOptions::default()
        },
    )
    .await
    .expect("b composes");
    assert!(
        tokio::time::timeout(Duration::from_secs(2), wait_connected(&mut scanner, &a))
            .await
            .is_err(),
        "the control: nothing connects before the add"
    );
    let added = scanner
        .sessions()
        .admin([AdminCapability::Peers].into())
        .await
        .expect("a port")
        .add_bootstrap_address(own.addresses[0].address.clone())
        .await
        .expect("answered");
    assert_eq!(
        added,
        AddressChange::Added,
        "a shown line is one an add takes"
    );
    wait_connected(&mut scanner, &a).await;

    scanner.shutdown().await.expect("clean shutdown");
    shown.shutdown().await.expect("clean shutdown");
}

/// The read is `admin.peers`'s: a port opened without it is refused,
/// one opened with it answers (the control).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_read_needs_the_peers_capability() {
    let (a_id, _) = id();
    let (_, b) = id();
    let runtime = ComposedRuntime::start(&a_id, &profile(&b), CompositionOptions::default())
        .await
        .expect("composes");
    let without = runtime
        .sessions()
        .admin([AdminCapability::Status].into())
        .await
        .expect("a port");
    assert_eq!(
        without.own_addresses().await,
        Err(TransportError::CapabilityDenied)
    );
    let with = runtime
        .sessions()
        .admin([AdminCapability::Peers].into())
        .await
        .expect("a port");
    assert!(with.own_addresses().await.is_ok());
    runtime.shutdown().await.expect("clean shutdown");
}

/// LOCAL-CLIENT.md §7 item 14's control: a node bound on loopback alone
/// shows no listen row -- it shows nothing at all, holding no
/// confirmation and no reservation -- though it is listening (the
/// control: `listening` names the loopback listener).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_loopback_only_node_shows_no_listen_row() {
    let (a_id, a) = id();
    let (_, b) = id();
    let runtime = ComposedRuntime::start(
        &a_id,
        &profile(&b),
        CompositionOptions {
            listen: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            ..CompositionOptions::default()
        },
    )
    .await
    .expect("composes");
    assert!(
        runtime
            .listening()
            .iter()
            .any(|l| l.starts_with("/ip4/127.0.0.1/tcp/")),
        "the control: bound on loopback {:?}",
        runtime.listening()
    );
    let own = runtime
        .sessions()
        .admin([AdminCapability::Peers].into())
        .await
        .expect("a port")
        .own_addresses()
        .await
        .expect("answered");
    assert_eq!(own.peer, a);
    assert_eq!(own.addresses, Vec::new(), "{own:?}");
    assert!(!own.truncated);
    runtime.shutdown().await.expect("clean shutdown");
}
