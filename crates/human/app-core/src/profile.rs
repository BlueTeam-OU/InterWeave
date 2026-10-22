// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Which of a profile's endpoints a desktop-hosted human client may lease:
//! one rule for every root that connects to a shared daemon -- the window
//! and the command line -- so the two never disagree on which identity a
//! profile offers them. Android's service leases the one endpoint its
//! profile names, and keeps its own rule
//! (`crates/human/android-platform/src/service.rs` `human_endpoint`).

use interweave_profile_config::ProfileConfig;
use interweave_transport_api::EndpointId;

/// The client kind a human client leases under. A profile's endpoint
/// admits it by naming it in `allowed_client_kinds`.
pub const CLIENT_KIND: &str = "human-client";

/// The endpoints `config` lets a human client lease, in the file's order:
/// every entry naming [`CLIENT_KIND`], whatever `enabled` says -- the
/// daemon's effective state decides at open, and an administrator may have
/// enabled one since the file was written.
#[must_use]
pub fn leasable_endpoints(config: &ProfileConfig) -> Vec<EndpointId> {
    config
        .endpoints
        .entries
        .iter()
        .filter(|e| {
            e.allowed_client_kinds
                .iter()
                .any(|k| k.as_str() == CLIENT_KIND)
        })
        .map(|e| e.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn every_entry_naming_the_human_kind_in_file_order_disabled_too() {
        let config = ProfileConfig::parse_yaml(
            "schema_version: 2
profile:
  name: test
runtime:
  deployment: embedded-android
ipc:
  enabled: false
transport:
  listen:
    addresses: [\"/ip4/0.0.0.0/tcp/0\"]
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  registration_policy: configured-only
  entries:
    - { id: second, enabled: false, advertise: false, allowed_client_kinds: [human-client] }
    - { id: claude, enabled: true, advertise: true, allowed_client_kinds: [claude-channel] }
    - { id: human, enabled: true, advertise: true, allowed_client_kinds: [claude-channel, human-client] }
channels: { desired: [] }
",
        )
        .expect("parses");
        let ids: Vec<String> = leasable_endpoints(&config)
            .into_iter()
            .map(|e| e.as_str().to_owned())
            .collect();
        assert_eq!(ids, ["second", "human"]);
    }
}
