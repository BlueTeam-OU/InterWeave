// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the endpoint overlay tests share: the profile, a private state
//! directory, the options naming its overlay, the admin port's list and
//! sets, and what the next start would load. Shared by
//! `endpoint_overlay.rs` and `endpoint_overlay_ahead.rs`, which declare
//! `mod common;` beside it.

#![allow(dead_code, clippy::expect_used, clippy::panic)]

use std::path::Path;

use interweave_local_client_api::{AdminBinding, AdminCapability, AdminPort, EndpointAdminView};
use interweave_profile_config::endpoint_overlay::{ENDPOINT_OVERLAY_FILE, EndpointOverlay};
use interweave_profile_config::{ProfileConfig, TrustBoundary};
use interweave_transport_api::{EndpointId, TransportError};
use interweave_transport_composition::{ComposedRuntime, CompositionError, CompositionOptions};

use super::common::{chmod, id};

/// `human`, the configured default, and `agent`, both enabled.
pub(crate) fn profile() -> ProfileConfig {
    serde_norway::from_str(
        "schema_version: 2
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  default_direct_endpoint: human
  entries:
    - id: human
      enabled: true
      advertise: false
    - id: agent
      enabled: true
      advertise: false
discovery:
  providers:
    - type: static-bootstrap
      enabled: true
      priority: 10
      config:
        peers: []
",
    )
    .expect("the document parses")
}

pub(crate) fn ep(s: &str) -> EndpointId {
    EndpointId::parse(s).expect("an endpoint id")
}

/// A private state directory and the endpoint overlay's path in it.
pub(crate) fn state() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    chmod(dir.path(), 0o700);
    let path = dir.path().join(ENDPOINT_OVERLAY_FILE);
    (dir, path)
}

pub(crate) fn options(overlay: Option<&Path>) -> CompositionOptions {
    CompositionOptions {
        endpoint_overlay_file: overlay.map(Path::to_path_buf),
        ..CompositionOptions::default()
    }
}

pub(crate) async fn start(overlay: Option<&Path>) -> Result<ComposedRuntime, CompositionError> {
    let (identity, _) = id();
    ComposedRuntime::start(&identity, &profile(), options(overlay)).await
}

/// `(enabled, default, persisted)` of each endpoint, in id order.
pub(crate) async fn rows(runtime: &ComposedRuntime) -> Vec<(String, bool, bool, bool)> {
    let rows: Vec<EndpointAdminView> = runtime
        .sessions()
        .admin([AdminCapability::Endpoints].into())
        .await
        .expect("a port")
        .leases()
        .await
        .expect("the list");
    rows.into_iter()
        .map(|r| {
            (
                r.endpoint.as_str().to_owned(),
                r.enabled,
                r.default,
                r.persisted,
            )
        })
        .collect()
}

pub(crate) async fn set_enabled(
    runtime: &ComposedRuntime,
    endpoint: &str,
    enabled: bool,
) -> Result<(), TransportError> {
    runtime
        .sessions()
        .admin([AdminCapability::Endpoints].into())
        .await
        .expect("a port")
        .set_endpoint_enabled(ep(endpoint), enabled)
        .await
        .map(drop)
}

pub(crate) async fn set_default(
    runtime: &ComposedRuntime,
    endpoint: Option<&str>,
) -> Result<(), TransportError> {
    runtime
        .sessions()
        .admin([AdminCapability::Endpoints].into())
        .await
        .expect("a port")
        .set_default_endpoint(endpoint.map(ep))
        .await
}

/// What the next start would compose from the file at `path`, as rows
/// `(id, enabled, default)` in id order.
pub(crate) fn on_disk(path: &Path) -> Vec<(String, bool, bool)> {
    let config = profile().endpoints;
    let (overlay, _) =
        EndpointOverlay::load_within(path, &config, &TrustBoundary::root()).expect("loads");
    let effective = overlay.effective(&config);
    effective
        .enabled
        .iter()
        .map(|(id, on)| {
            (
                id.as_str().to_owned(),
                *on,
                effective.default.as_ref() == Some(id),
            )
        })
        .collect()
}

pub(crate) fn row(
    id: &str,
    enabled: bool,
    default: bool,
    persisted: bool,
) -> (String, bool, bool, bool) {
    (id.to_owned(), enabled, default, persisted)
}
