// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The admission set's bound where the daemon enforces it (ADR-0067
//! decision 6): the operator's list united with every endpoint's own sets
//! is at most `PeerTrustPolicy::MAX_ALLOWED_PEERS`. A start past it is
//! refused by name, and each write that would cross it -- an endpoint's
//! allow, the operator's allow -- is answered `InvalidArgument` with its
//! overlay byte for byte unchanged. Each case sits beside the control at
//! exactly the bound, which is accepted.

#![allow(clippy::expect_used, clippy::panic)]

use interweave_local_client_api::{AdminBinding, AdminCapability, AdminPort, TrustChange};
use interweave_profile_config::endpoint_overlay::ENDPOINT_OVERLAY_FILE;
use interweave_profile_config::trust_overlay::TRUST_OVERLAY_FILE;
use interweave_transport_api::{EndpointId, TransportError, TransportIdentity};
use interweave_transport_composition::{ComposedRuntime, CompositionError, CompositionOptions};
use interweave_trust_api::PeerTrustPolicy;

mod common;

use common::{chmod, id, profile};

/// `human`'s id in `common::profile`.
const HUMAN: &str = "12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr";

fn human() -> EndpointId {
    EndpointId::parse_derived(HUMAN).expect("the profile's endpoint")
}

fn peers(count: usize) -> Vec<TransportIdentity> {
    (0..count).map(|_| id().1).collect()
}

/// A state directory whose endpoint overlay gives `human` an own set of
/// `own` in both directions, and the options naming both overlay files.
fn state_with(own: &[TransportIdentity]) -> (tempfile::TempDir, CompositionOptions) {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().expect("a temporary directory");
    chmod(dir.path(), 0o700);
    let ids: Vec<&str> = own.iter().map(TransportIdentity::as_str).collect();
    let doc = serde_json::json!({
        "enabled": {},
        "inbound": {HUMAN: ids},
        "outbound": {HUMAN: ids},
    });
    let path = dir.path().join(ENDPOINT_OVERLAY_FILE);
    std::fs::write(&path, serde_json::to_vec(&doc).expect("serialises")).expect("written");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    let options = CompositionOptions {
        trust_overlay_file: Some(dir.path().join(TRUST_OVERLAY_FILE)),
        endpoint_overlay_file: Some(path),
        ..CompositionOptions::default()
    };
    (dir, options)
}

/// A start whose operator's list and own sets give 4097 distinct peers is
/// refused with `AdmissionSetTooLarge`, naming the count and the bound;
/// the control, the same sets at exactly 4096 (the operator's one peer
/// inside the own set), starts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_start_past_the_admission_bound_is_refused_by_name() {
    let max = PeerTrustPolicy::MAX_ALLOWED_PEERS;
    let own = peers(max);
    let (identity, _) = id();

    let (_dir, options) = state_with(&own);
    let control = ComposedRuntime::start(&identity, &profile(&[&own[0]], &[]), options)
        .await
        .expect("exactly at the bound starts");
    control.stop().await.expect("stops");

    let (_, outside) = id();
    let (_dir, options) = state_with(&own);
    let refused = ComposedRuntime::start(&identity, &profile(&[&outside], &[]), options).await;
    assert!(
        matches!(
            refused,
            Err(CompositionError::AdmissionSetTooLarge { got, max: bound })
                if got == max + 1 && bound == max
        ),
        "{:?}",
        refused.map(drop)
    );
}

/// A running daemon at the bound: the operator's one peer and 4095 own
/// peers, 4096 distinct.
async fn at_the_bound() -> (
    tempfile::TempDir,
    CompositionOptions,
    ComposedRuntime,
    TransportIdentity,
    TransportIdentity,
) {
    let max = PeerTrustPolicy::MAX_ALLOWED_PEERS;
    let own = peers(max - 1);
    let (identity, _) = id();
    let (_, operators) = id();
    let (dir, options) = state_with(&own);
    let runtime = ComposedRuntime::start(&identity, &profile(&[&operators], &[]), options.clone())
        .await
        .expect("exactly at the bound starts");
    (dir, options, runtime, operators, own[0].clone())
}

/// An endpoint-form `admin.trust.set` that would cross the bound is
/// answered `InvalidArgument` and the endpoint overlay is byte for byte
/// what it was; the control allows the operator's peer into the set --
/// the admission set does not grow -- and does write the file.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_endpoint_allow_crossing_the_bound_is_refused_and_writes_nothing() {
    let (_dir, options, runtime, operators, _) = at_the_bound().await;
    let file = options.endpoint_overlay_file.expect("a file");
    let admin = runtime
        .sessions()
        .admin([AdminCapability::Trust].into())
        .await
        .expect("a port");
    let before = std::fs::read(&file).expect("the overlay");

    let (_, newcomer) = id();
    assert_eq!(
        admin.set_endpoint_trust(human(), newcomer, true).await,
        Err(TransportError::InvalidArgument)
    );
    assert_eq!(std::fs::read(&file).expect("the overlay"), before);

    assert_eq!(
        admin.set_endpoint_trust(human(), operators, true).await,
        Ok(TrustChange::Set),
        "the control: inside the bound"
    );
    assert_ne!(std::fs::read(&file).expect("the overlay"), before);
    runtime.stop().await.expect("stops");
}

/// The operator's allow that would cross the bound is answered
/// `InvalidArgument` and the trust overlay is not written; the control
/// allows a peer an own set already admits, which is inside the bound and
/// writes the overlay.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_operator_allow_crossing_the_bound_is_refused_and_writes_nothing() {
    let (_dir, options, runtime, _, admitted) = at_the_bound().await;
    let file = options.trust_overlay_file.expect("a file");
    let admin = runtime
        .sessions()
        .admin([AdminCapability::Trust].into())
        .await
        .expect("a port");
    assert!(!file.exists(), "no trust overlay yet");

    let (_, newcomer) = id();
    assert_eq!(
        admin.set_trust(newcomer.clone(), true).await,
        Err(TransportError::InvalidArgument)
    );
    assert!(!file.exists(), "nothing written");
    assert!(
        !admin
            .trust()
            .await
            .expect("the operator's list")
            .peers()
            .any(|p| p == &newcomer),
        "nothing changed"
    );

    assert_eq!(
        admin.set_trust(admitted, true).await,
        Ok(TrustChange::Set),
        "the control: inside the bound"
    );
    assert!(file.exists(), "the control wrote the overlay");
    runtime.stop().await.expect("stops");
}
