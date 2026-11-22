// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

//! The schema's `# Runtime cross-field validation`, the rules the model
//! makes checkable (`runtime.rs`'s module note names all seven): an
//! embedded-android profile leases an enabled configured endpoint, runs
//! no infrastructure server, and listens on wildcard addresses only. Each refusal beside its control -- the
//! same document as a daemon, or with the endpoint enabled -- so a
//! refusal is the rule's and not the document's.

#![allow(clippy::expect_used)]

use interweave_profile_config::{ConfigError, ProfileConfig};

/// The leased endpoint's id: the entry below, named by id, since
/// `runtime.android.endpoint` takes a derived id and never an alias
/// (ADR-0067).
const LEASED: &str = "12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr";

fn profile(deployment: &str, endpoint_enabled: bool, extra: &str) -> ProfileConfig {
    let doc = format!(
        "schema_version: 2
runtime:
  deployment: {deployment}
  android:
    endpoint: {LEASED}
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  entries:
    - key: 12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr.key
      alias: human
      enabled: {endpoint_enabled}
      advertise: false
{extra}"
    );
    serde_norway::from_str(&doc).expect("the document parses")
}

fn android_errors(profile: &ProfileConfig) -> Vec<ConfigError> {
    profile
        .validate()
        .into_iter()
        .filter(|e| {
            matches!(
                e,
                ConfigError::AndroidEndpointNotEnabled { .. }
                    | ConfigError::AndroidServesInfrastructure { .. }
                    | ConfigError::AndroidListenerNotWildcard { .. }
            )
        })
        .collect()
}

#[test]
fn an_android_profile_with_its_endpoint_enabled_is_accepted() {
    assert!(android_errors(&profile("embedded-android", true, "")).is_empty());
}

/// Rule 3's split (`ENDPOINTS.md`, A 2026-11-21): at load the named
/// endpoint must be configured, and whether it is enabled is the start's.
#[test]
fn an_android_profile_must_lease_an_enabled_configured_endpoint() {
    let disabled = profile("embedded-android", false, "");
    assert!(
        android_errors(&disabled).is_empty(),
        "a configured endpoint, disabled, loads"
    );
    assert_eq!(
        disabled
            .runtime
            .require_android_identity(&disabled.endpoints),
        Err(ConfigError::AndroidNoIdentity),
        "and its start is refused: no identity is enabled"
    );
    let mut named = profile("embedded-android", true, "");
    let pager = interweave_test_support::endpoint_keys::endpoint_id("pager");
    named.runtime.android.endpoint = Some(pager.clone());
    assert_eq!(
        android_errors(&named),
        vec![ConfigError::AndroidEndpointNotEnabled {
            endpoint: pager.as_str().to_owned()
        }],
        "an endpoint that is not configured at all"
    );
    assert!(
        android_errors(&profile("daemon-ipc", false, "")).is_empty(),
        "the control: a daemon leases nothing at start"
    );
}

#[test]
fn an_android_profile_runs_no_infrastructure_server() {
    for (extra, field) in [
        (
            "transport:\n  connectivity:\n    relay:\n      server:\n        enabled: true\n",
            "transport.connectivity.relay.server.enabled",
        ),
        (
            "transport:\n  connectivity:\n    autonat:\n      server:\n        enabled: true\n",
            "transport.connectivity.autonat.server.enabled",
        ),
        (
            "discovery:\n  providers:\n    - type: kademlia\n      enabled: true\n      priority: 40\n      config:\n        mode: server\n        network_id: interweave-test\n",
            "discovery.providers[kademlia].config.mode",
        ),
    ] {
        assert_eq!(
            android_errors(&profile("embedded-android", true, extra)),
            vec![ConfigError::AndroidServesInfrastructure { field }],
            "{field}"
        );
        assert!(
            android_errors(&profile("daemon-ipc", true, extra)).is_empty(),
            "the control: a daemon may serve {field}"
        );
    }
}

#[test]
fn an_android_profile_listens_on_wildcard_addresses_only() {
    let listening = |addresses: &[&str]| {
        let quoted: Vec<String> = addresses.iter().map(|a| format!("\"{a}\"")).collect();
        format!(
            "transport:\n  listen:\n    addresses: [{}]\n",
            quoted.join(", ")
        )
    };
    let wildcards = listening(&[
        "/ip4/0.0.0.0/tcp/0",
        "/ip6/::/tcp/4001",
        "/ip4/0.0.0.0/udp/0/quic-v1",
    ]);
    assert!(
        android_errors(&profile("embedded-android", true, &wildcards)).is_empty(),
        "the control: every wildcard is accepted"
    );
    for refused in [
        "/ip4/127.0.0.1/tcp/0",
        "/ip4/192.168.1.5/tcp/4001",
        "/ip6/::1/tcp/0",
        "/ip6/0.0.0.0/tcp/0",
        "/ip4/::/tcp/0",
        "/dns4/localhost/tcp/0",
        "ip4/0.0.0.0/tcp/0",
    ] {
        let extra = listening(&["/ip4/0.0.0.0/tcp/0", refused]);
        assert_eq!(
            android_errors(&profile("embedded-android", true, &extra)),
            vec![ConfigError::AndroidListenerNotWildcard {
                address: refused.to_owned()
            }],
            "{refused}"
        );
        assert!(
            !android_errors(&profile("daemon-ipc", true, &extra))
                .iter()
                .any(|e| matches!(e, ConfigError::AndroidListenerNotWildcard { .. })),
            "the control: a daemon may listen on {refused}"
        );
    }
}

/// The field takes an id, NEVER AN ALIAS: `human` is the entry's alias,
/// and no entry's id equals it, so it is refused as naming no enabled
/// endpoint -- beside the same entry named by its id, accepted.
#[test]
fn an_android_endpoint_named_by_alias_is_refused() {
    let doc = format!(
        "schema_version: 2
runtime:
  deployment: embedded-android
  android:
    endpoint: human
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  entries:
    - key: {LEASED}.key
      alias: human
      enabled: true
      advertise: false
"
    );
    let by_alias: ProfileConfig = serde_norway::from_str(&doc).expect("parses");
    assert_eq!(
        android_errors(&by_alias),
        vec![ConfigError::AndroidEndpointNotEnabled {
            endpoint: "human".to_owned()
        }]
    );
    let by_id: ProfileConfig =
        serde_norway::from_str(&doc.replace("endpoint: human", &format!("endpoint: {LEASED}")))
            .expect("parses");
    assert!(android_errors(&by_id).is_empty(), "the control: by id");
}

/// Rule 3 BINDS THE START (j185): an embedded-android profile with no
/// `runtime.android.endpoint` validates -- the first-run document before
/// onboarding loads -- and its start's check refuses it
/// `AndroidNoIdentity`; so does one whose composed set enables no
/// endpoint. The controls: the same profile naming its enabled endpoint
/// passes both, and a daemon is asked nothing.
#[test]
fn an_android_start_needs_an_identity() {
    let mut absent = profile("embedded-android", true, "");
    absent.runtime.android.endpoint = None;
    assert!(
        android_errors(&absent).is_empty(),
        "absent is no load refusal"
    );
    assert_eq!(
        absent.runtime.require_android_identity(&absent.endpoints),
        Err(ConfigError::AndroidNoIdentity)
    );
    let named = profile("embedded-android", true, "");
    assert_eq!(
        named.runtime.require_android_identity(&named.endpoints),
        Ok(()),
        "the control: named and enabled"
    );
    // The named entry disabled while another is enabled: an identity to
    // start as (ADR-0061 decision 7).
    let mut other = named.endpoints.clone();
    let mut second = other.entries[0].clone();
    second.id = interweave_test_support::endpoint_keys::endpoint_id("other");
    second.alias = None;
    other.entries[0].enabled = false;
    other.entries.push(second);
    assert_eq!(
        named.runtime.require_android_identity(&other),
        Ok(()),
        "the named endpoint disabled beside an enabled one"
    );
    // The composed set (an overlay disabling the one entry) enables none.
    let mut composed = named.endpoints.clone();
    composed.entries[0].enabled = false;
    assert_eq!(
        named.runtime.require_android_identity(&composed),
        Err(ConfigError::AndroidNoIdentity)
    );
    let mut daemon = profile("daemon-ipc", true, "");
    daemon.runtime.android.endpoint = None;
    assert_eq!(
        daemon.runtime.require_android_identity(&daemon.endpoints),
        Ok(()),
        "a daemon leases nothing at start"
    );
    assert!(
        ConfigError::AndroidNoIdentity
            .to_string()
            .contains("onboarding seals the first identity"),
        "{}",
        ConfigError::AndroidNoIdentity
    );
}

#[test]
fn a_disabled_kademlia_entry_in_server_mode_runs_nothing_and_is_not_refused() {
    let extra = "discovery:\n  providers:\n    - type: kademlia\n      enabled: false\n      priority: 40\n      config:\n        mode: server\n        network_id: interweave-test\n";
    assert!(android_errors(&profile("embedded-android", true, extra)).is_empty());
}

#[test]
fn an_unknown_deployment_is_refused_at_parse() {
    let doc = "schema_version: 2
runtime:
  deployment: bare-metal
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  entries: []
";
    assert!(serde_norway::from_str::<ProfileConfig>(doc).is_err());
}

fn ipc_errors(profile: &ProfileConfig) -> Vec<ConfigError> {
    profile
        .validate()
        .into_iter()
        .filter(|e| matches!(e, ConfigError::IpcContradictsDeployment { .. }))
        .collect()
}

/// Rules 1 and 2: a daemon runs its IPC boundary and an embedded Android
/// runtime does not -- each refusal beside the deployment it suits.
#[test]
fn ipc_enabled_follows_the_deployment() {
    assert!(
        ipc_errors(&profile("daemon-ipc", true, "")).is_empty(),
        "the default"
    );
    assert!(
        ipc_errors(&profile(
            "embedded-android",
            true,
            "ipc:\n  enabled: false\n"
        ))
        .is_empty(),
        "android without IPC"
    );
    assert_eq!(
        ipc_errors(&profile("daemon-ipc", true, "ipc:\n  enabled: false\n")),
        vec![ConfigError::IpcContradictsDeployment {
            deployment: "daemon-ipc",
            ipc_enabled: false,
        }]
    );
    assert_eq!(
        ipc_errors(&profile("embedded-android", true, "")),
        vec![ConfigError::IpcContradictsDeployment {
            deployment: "embedded-android",
            ipc_enabled: true,
        }],
        "the default ipc.enabled is a daemon's"
    );
}
