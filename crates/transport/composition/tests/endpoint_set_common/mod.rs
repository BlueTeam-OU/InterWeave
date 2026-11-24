// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the endpoint set tests share (IPC 3.0's `admin.endpoints.add`,
//! `remove`, `set_advertise` and `set_alias`): a profile under real
//! profile paths -- the add reads its key file's header under the state
//! directory -- the options naming its overlay, a freshly minted key, and
//! the admin port. Shared by `endpoint_set.rs` and `endpoint_set_ahead.rs`.

#![allow(dead_code, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use interweave_local_client_api::{
    AdminBinding, AdminCapability, AdminPort, EndpointAddChange, EndpointAdminView,
};
use interweave_profile_config::endpoint_overlay::EndpointOverlay;
use interweave_profile_config::{ProfileConfig, ProfilePaths, XdgRoots, endpoint_key};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use interweave_transport_composition::{ComposedRuntime, CompositionOptions, InProcessAdmin};

/// A profile's paths in a private temporary root, and its overlay's path.
pub(crate) struct Profile {
    pub(crate) _dir: tempfile::TempDir,
    pub(crate) paths: ProfilePaths,
    pub(crate) overlay: PathBuf,
}

pub(crate) fn paths() -> Profile {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("a temporary directory");
    let roots = XdgRoots {
        config_home: dir.path().join("config"),
        data_home: dir.path().join("data"),
        state_home: dir.path().join("state"),
        cache_home: dir.path().join("cache"),
        runtime_dir: None,
    };
    let paths = ProfilePaths::resolve_offline("work", &roots).expect("paths");
    std::fs::create_dir_all(paths.state_dir()).expect("a state directory");
    std::fs::set_permissions(paths.state_dir(), std::fs::Permissions::from_mode(0o700))
        .expect("private");
    let overlay = EndpointOverlay::path_for(&paths);
    Profile {
        _dir: dir,
        paths,
        overlay,
    }
}

/// `human` (the default) and `agent`, both authored, enabled and
/// unadvertised unless `advertised` names them; `allowed` the operator's
/// list; `statics` the bootstrap addresses; `max_advertised` the
/// directory's cap.
pub(crate) fn profile_with(
    allowed: &[&TransportIdentity],
    statics: &[String],
    advertised: &[&str],
    max_advertised: u32,
) -> ProfileConfig {
    let list = |items: Vec<String>| items.join(", ");
    let doc = format!(
        "schema_version: 2
trust:
  policy: static-allowlist
  allowed_peers: [{}]
endpoints:
  default_direct_endpoint: human
  directory:
    max_advertised: {max_advertised}
  entries:
    - key: 12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr.key
      alias: human
      enabled: true
      advertise: {}
    - key: 12D3KooWEFtocnyKkCXyW3kXvHejTQTV7NBZ8bAhy7jNJU4t2Zq5.key
      alias: agent
      enabled: true
      advertise: {}
discovery:
  providers:
    - type: static-bootstrap
      enabled: true
      priority: 10
      config:
        peers: [{}]
",
        list(
            allowed
                .iter()
                .map(|p| format!("\"{}\"", p.as_str()))
                .collect()
        ),
        advertised.contains(&"human"),
        advertised.contains(&"agent"),
        list(statics.iter().map(|s| format!("\"{s}\"")).collect()),
    );
    serde_norway::from_str(&doc).expect("the document parses")
}

pub(crate) fn profile() -> ProfileConfig {
    profile_with(&[], &[], &[], 8)
}

pub(crate) fn options(p: &Profile) -> CompositionOptions {
    CompositionOptions {
        endpoint_overlay_file: Some(p.overlay.clone()),
        profile_paths: Some(p.paths.clone()),
        trust_boundary: p.paths.boundary().clone(),
        ..CompositionOptions::default()
    }
}

pub(crate) fn id() -> (ProfileIdentity, TransportIdentity) {
    let identity = ProfileIdentity::generate();
    let peer = identity.transport_identity().expect("peer id");
    (identity, peer)
}

pub(crate) fn ep(s: &str) -> EndpointId {
    interweave_test_support::endpoint_keys::endpoint_id(s)
}

pub(crate) async fn admin(runtime: &ComposedRuntime) -> InProcessAdmin {
    runtime
        .sessions()
        .admin([AdminCapability::Endpoints, AdminCapability::Trust].into())
        .await
        .expect("a port")
}

/// A key minted into the profile's `endpoint-keys/`, its file's name.
pub(crate) fn mint(p: &Profile) -> (EndpointId, String) {
    let id = endpoint_key::mint_filesystem(&p.paths).expect("minted");
    let name = endpoint_key::key_file_name(&id);
    (id, name)
}

/// Mint a key and add it under `alias`, expecting it added.
pub(crate) async fn add_minted(p: &Profile, port: &impl AdminPort, alias: &str) -> EndpointId {
    let (id, key) = mint(p);
    assert_eq!(
        port.add_endpoint(key, alias.to_owned(), vec![]).await,
        Ok(EndpointAddChange::Added(id.clone()))
    );
    id
}

/// The row for `endpoint` in the runtime's list.
pub(crate) async fn row_of(
    port: &impl AdminPort,
    endpoint: &EndpointId,
) -> Option<EndpointAdminView> {
    port.leases()
        .await
        .expect("the list")
        .into_iter()
        .find(|r| &r.endpoint == endpoint)
}

/// The overlay file's bytes, `None` when there is none.
pub(crate) fn bytes(p: &Profile) -> Option<Vec<u8>> {
    std::fs::read(&p.overlay).ok()
}
