// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The graphical client's advertise assertion (ADR-0061 decision 5)
//! against a REAL in-process composition runtime -- the one the Android
//! service leases from -- rather than a scripted port: the facade's
//! `set_advertise(identity, false)` goes through the runtime's endpoint
//! set, and the runtime's own test hooks make its publish fail or hold
//! (`ComposedRuntime::fail_publishes`, `hold_next_endpoint_publish`).
//! What each answer becomes for the session is the transport-client's
//! `tests/hiding.rs`; this suite proves the runtime's answers reach it.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{ClientConfig, SessionState, TransportClient};
use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort as _};
use interweave_profile_config::ProfileConfig;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::EndpointId;
use interweave_transport_composition::{ComposedRuntime, CompositionOptions, InProcessBinding};

const PATIENCE: Duration = Duration::from_secs(20);

fn human() -> EndpointId {
    interweave_test_support::endpoint_keys::endpoint_id("human")
}

/// `human`, authored ADVERTISED: the assertion's `false` is a change the
/// runtime writes and publishes, so its publish can be failed or held.
async fn start() -> ComposedRuntime {
    let profile = ProfileConfig::parse_yaml(
        "schema_version: 2
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  default_direct_endpoint: human
  entries:
    - key: 12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr.key
      alias: human
      enabled: true
      advertise: true
discovery:
  providers:
    - type: static-bootstrap
      enabled: true
      priority: 10
      config:
        peers: []
",
    )
    .expect("the document parses");
    ComposedRuntime::start(
        &ProfileIdentity::generate(),
        &profile,
        CompositionOptions::default(),
    )
    .await
    .expect("composes")
}

fn client(runtime: &ComposedRuntime) -> TransportClient<InProcessBinding, InProcessBinding> {
    TransportClient::new(
        runtime.sessions(),
        runtime.sessions(),
        HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store"),
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![human()],
            channels: Vec::new(),
            max_payload_bytes: 49_152,
        },
        Box::new(|| 1_786_600_000_000),
        0,
    )
    .expect("an empty store")
    .without_status()
}

/// `human`'s row in the runtime's own list: `(leased, advertise)`.
async fn human_row(runtime: &ComposedRuntime) -> (bool, bool) {
    let port = runtime
        .sessions()
        .admin([AdminCapability::Endpoints].into())
        .await
        .expect("a port");
    let row = port
        .leases()
        .await
        .expect("the list")
        .into_iter()
        .find(|r| r.endpoint == human())
        .expect("human is listed");
    (row.lease.is_some(), row.advertise)
}

/// Tick `client` at `now` until `done` holds of its state, letting the
/// runtime's driver run between ticks.
async fn tick_until(
    client: &mut TransportClient<InProcessBinding, InProcessBinding>,
    now: u64,
    done: impl Fn(&SessionState) -> bool,
) {
    let deadline = Instant::now() + PATIENCE;
    loop {
        client.tick(now).await;
        if done(client.session_state()) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "still {:?}",
            client.session_state()
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// The runtime answers the assertion with a failed publish -- the
/// method's `BackendUnavailable`, which the client cannot tell from an
/// admin connection lost before its answer (`LOCAL-IPC.md` §Close) -- and
/// no lease is taken: the session re-opens on a timer, as with no daemon,
/// and the re-open asserts again, then leases, the endpoint unadvertised.
/// Mutation: a lease taken whatever the assertion answered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_publish_takes_no_lease_and_the_reopen_asserts_again() {
    let runtime = start().await;
    let mut client = client(&runtime);
    runtime.fail_publishes(1).await.expect("queued");
    tick_until(&mut client, 0, |s| {
        !matches!(s, SessionState::Reconnecting { attempt: 0, .. })
    })
    .await;
    let SessionState::Reconnecting { attempt, next_at } = client.session_state().clone() else {
        panic!("re-opening: {:?}", client.session_state());
    };
    assert_eq!(attempt, 1);
    assert_eq!(
        human_row(&runtime).await,
        (false, true),
        "no lease, as authored"
    );
    tick_until(&mut client, next_at, |s| {
        matches!(s, SessionState::Ready { .. })
    })
    .await;
    assert_eq!(
        human_row(&runtime).await,
        (true, false),
        "leased, unadvertised"
    );
    runtime.stop().await.expect("stops");
}

/// A publish the runtime holds holds the lease: tick after tick, the
/// session stays `Reconnecting`, its ticks returning; released, the
/// next tick leases. Mutation: the lease asked
/// before the assertion's answer.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_held_publish_takes_no_lease_until_it_is_released() {
    let runtime = start().await;
    let mut client = client(&runtime);
    let mut hold = runtime.hold_next_endpoint_publish().await.expect("held");
    client.tick(0).await;
    tokio::time::timeout(PATIENCE, hold.reached())
        .await
        .expect("the assertion reached its publish")
        .expect("the hold is taken");
    for _ in 0..50 {
        client.tick(0).await;
        assert!(
            matches!(
                client.session_state(),
                SessionState::Reconnecting { attempt: 0, .. }
            ),
            "{:?}",
            client.session_state()
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // The runtime's list is not read here: the hold stops its driver,
    // which answers that read too. A lease taken would have made the
    // session `Ready`.
    hold.release();
    tick_until(&mut client, 0, |s| matches!(s, SessionState::Ready { .. })).await;
    assert_eq!(human_row(&runtime).await, (true, false));
    runtime.stop().await.expect("stops");
}
