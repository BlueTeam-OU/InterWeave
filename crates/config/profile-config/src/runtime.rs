// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

//! The `runtime` block: which deployment binding this profile runs under.
//!
//! `config.schema.yaml` is normative for the shape; this module is the
//! first code to read it (plan §15's Stage 12 decisions, 2026-09-27:
//! composition needs `runtime.deployment` to choose what it builds, and
//! modelling the block is what lets the schema's `# Runtime cross-field
//! validation` be enforced at all).
//!
//! # Which of the seven runtime rules are checked here
//!
//! The schema lists seven, and a rule is checked only where the model
//! holds both its sides (the wildcard-listener rule is the schema's
//! fifth, written last here as the one added in 2026-10):
//!
//! 1. and 2. `daemon-ipc` runs the IPC boundary and `embedded-android`
//!    does not (`ipc.enabled`) -- checked since Stage 13 modelled `ipc`
//!    ([`RuntimeConfig::validate_into`]).
//! 3. `embedded-android` names an enabled configured endpoint -- split
//!    between the load and the START (`ENDPOINTS.md`, A 2026-11-21): the
//!    first-run document has no identity until onboarding seals one, and
//!    it must load. At load a `runtime.android.endpoint` that is present
//!    must be a configured entry's id ([`RuntimeConfig::validate_into`]);
//!    at the start it must be present and the composed set must enable
//!    at least one identity ([`RuntimeConfig::require_android_identity`]).
//! 4. `embedded-android` runs no AutoNAT or relay server and Kademlia
//!    only as a client -- checked.
//! 5. `stay-reachable` means `foreground_service_type = remoteMessaging`
//!    -- held by the TYPE: the field is `literal[remoteMessaging]`, so no
//!    other value parses whatever the availability mode.
//! 6. `stay-reachable` with `user-presence` derives a diagnostic, not a
//!    refusal -- [`background_restart_requires_user_authentication`], of the
//!    policy in force, never the configured one (ADR-0041 A 2026-11-05).
//!
//! And the wildcard-listener rule (architect-cto's ruling of
//! 2026-10-09, relay seq 33736, §20 step 5): `embedded-android`
//! listens on WILDCARD addresses only (`/ip4/0.0.0.0`, `/ip6/::`). A
//! listener on one address dies with it and nothing issues it again,
//! and Android names no address that stays; a wildcard listener follows
//! the interfaces as they come and go. Checked
//! ([`RuntimeConfig::validate_into`]); a daemon may name a specific one.

use interweave_transport_api::EndpointId;
use serde::{Deserialize, Serialize};

use crate::{ConfigError, DiscoveryProviderType, EndpointsConfig};

/// The `runtime` block.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    /// The deployment binding (`daemon-ipc` by default).
    #[serde(default)]
    pub deployment: Deployment,
    /// Read only when `deployment` is `embedded-android`.
    #[serde(default)]
    pub android: AndroidRuntimeConfig,
}

/// How the transport runtime is hosted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Deployment {
    /// A daemon serving local clients over IPC.
    #[default]
    DaemonIpc,
    /// In-process in the Android app's foreground service.
    EmbeddedAndroid,
}

/// `runtime.android`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AndroidRuntimeConfig {
    /// The endpoint the foreground service leases, by its derived id --
    /// never an alias, which no entry's id equals -- and absent until
    /// onboarding seals the first identity: no default, since an id is
    /// derived from a key no document can know in advance (ADR-0067).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<EndpointId>,
    /// The AUTHORED default for whether the service keeps the node
    /// reachable in the background. The person's choice overrides it
    /// (ADR-0041 A 2026-10-10), so what decides is the effective mode,
    /// [`effective_availability_mode`](crate::availability_overlay::effective_availability_mode),
    /// never this field alone.
    #[serde(default)]
    pub availability_mode: AvailabilityMode,
    /// `literal[remoteMessaging]`.
    #[serde(default)]
    pub foreground_service_type: ForegroundServiceType,
    /// `literal[android-keystore-aesgcm-wrap]`.
    #[serde(default)]
    pub key_storage: KeyStorage,
    /// Whether unlocking the key needs the user present.
    #[serde(default)]
    pub key_unlock_policy: KeyUnlockPolicy,
}

impl AndroidRuntimeConfig {
    /// The endpoint the service leases, if the profile names one. There
    /// is no default.
    #[must_use]
    pub fn endpoint(&self) -> Option<EndpointId> {
        self.endpoint.clone()
    }
}

/// The schema's sixth runtime rule: a node that stays reachable but
/// whose key needs the user present cannot come back on its own after a
/// restart. A derived status, not a refusal -- the combination is legal.
///
/// Judged on the EFFECTIVE mode
/// ([`effective_availability_mode`](crate::availability_overlay::effective_availability_mode)),
/// never on `availability_mode` alone: that field is only the authored
/// default, and the person's choice overrides it (ADR-0041 A 2026-10-10).
/// And on the policy IN FORCE -- the authenticated policy byte of the
/// identity record the start's custody step left on disk -- never on
/// the configured `key_unlock_policy`: while a reseal is pending the two
/// differ, and a restart opens the record under its own policy (ADR-0041
/// A 2026-11-05). The configured policy is the pending change shown
/// beside this, not an input to it: the signature takes no
/// configuration, so it cannot be read here.
#[must_use]
pub fn background_restart_requires_user_authentication(
    effective: AvailabilityMode,
    in_force: KeyUnlockPolicy,
) -> bool {
    effective == AvailabilityMode::StayReachable && in_force == KeyUnlockPolicy::UserPresence
}

/// `runtime.android.availability_mode`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AvailabilityMode {
    /// Reachable while the app is in the foreground.
    #[default]
    ForegroundOnly,
    /// Kept reachable by the foreground service.
    StayReachable,
}

/// `runtime.android.foreground_service_type`: one value, so a profile may
/// state it and may state nothing else.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForegroundServiceType {
    /// Android's `remoteMessaging` foreground-service type.
    #[default]
    #[serde(rename = "remoteMessaging")]
    RemoteMessaging,
}

/// `runtime.android.key_storage`: one value, as above.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyStorage {
    /// The identity key wrapped by an Android Keystore AES-GCM key.
    #[default]
    #[serde(rename = "android-keystore-aesgcm-wrap")]
    AndroidKeystoreAesgcmWrap,
}

/// `runtime.android.key_unlock_policy`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyUnlockPolicy {
    /// Usable after a restart without the user.
    #[default]
    BackgroundCompatible,
    /// Needs local user authentication to unlock.
    UserPresence,
}

/// What the runtime rules read from the rest of the document.
pub(crate) struct RuntimeContext<'a> {
    pub endpoints: &'a EndpointsConfig,
    pub autonat_server_enabled: bool,
    pub relay_server_enabled: bool,
    /// Each enabled Kademlia entry's `mode`, as written (`None` is the
    /// schema default, `client`).
    pub enabled_kademlia_modes: Vec<Option<&'a str>>,
    /// `ipc.enabled`.
    pub ipc_enabled: bool,
    /// `transport.listen.addresses`, as written.
    pub listen_addresses: &'a [String],
}

impl RuntimeConfig {
    /// Rules 1 to 4 and the wildcard-listener rule (see the module note
    /// for all seven).
    pub(crate) fn validate_into(
        &self,
        context: &RuntimeContext<'_>,
        errors: &mut Vec<ConfigError>,
    ) {
        let android = self.deployment == Deployment::EmbeddedAndroid;
        // Rules 1 and 2: the daemon serves its clients over IPC; the
        // embedded runtime has no IPC boundary to open.
        if context.ipc_enabled == android {
            errors.push(ConfigError::IpcContradictsDeployment {
                deployment: if android {
                    "embedded-android"
                } else {
                    "daemon-ipc"
                },
                ipc_enabled: context.ipc_enabled,
            });
        }
        if !android {
            return;
        }
        // Rule 3, of a field that is PRESENT: absent is the first-run
        // document before onboarding, which loads
        // (`tests/provision.rs`, `form_1_loads`), and its start is
        // refused by `require_android_identity`.
        if let Some(endpoint) = &self.android.endpoint
            && !context.endpoints.entries.iter().any(|e| &e.id == endpoint)
        {
            errors.push(ConfigError::AndroidEndpointNotEnabled {
                endpoint: endpoint.as_str().to_owned(),
            });
        }
        for (field, on) in [
            (
                "transport.connectivity.autonat.server.enabled",
                context.autonat_server_enabled,
            ),
            (
                "transport.connectivity.relay.server.enabled",
                context.relay_server_enabled,
            ),
        ] {
            if on {
                errors.push(ConfigError::AndroidServesInfrastructure { field });
            }
        }
        if context
            .enabled_kademlia_modes
            .iter()
            .any(|mode| mode.is_some_and(|m| m != "client"))
        {
            errors.push(ConfigError::AndroidServesInfrastructure {
                field: "discovery.providers[kademlia].config.mode",
            });
        }
        for address in context.listen_addresses {
            if !is_wildcard_listener(address) {
                errors.push(ConfigError::AndroidListenerNotWildcard {
                    address: address.clone(),
                });
            }
        }
    }
}

impl RuntimeConfig {
    /// The start's half of rule 3, judged on the COMPOSED endpoints --
    /// `config.yaml`'s with the endpoint overlay applied: an
    /// embedded-android profile with no `runtime.android.endpoint`, or
    /// whose composed set enables no endpoint at all, has no identity to
    /// start as; any enabled identity will do, the named one included or
    /// not (ADR-0061 decision 7).
    /// Nothing is asked of a daemon
    /// (`tests/runtime_rules.rs`, `an_android_start_needs_an_identity`).
    ///
    /// # Errors
    /// [`ConfigError::AndroidNoIdentity`].
    pub fn require_android_identity(&self, composed: &EndpointsConfig) -> Result<(), ConfigError> {
        if self.deployment != Deployment::EmbeddedAndroid {
            return Ok(());
        }
        // ANY enabled identity, not the named one: the first identity may
        // be turned off once another is enabled (ADR-0061 decision 7).
        if self.android.endpoint.is_none() || !composed.entries.iter().any(|e| e.enabled) {
            return Err(ConfigError::AndroidNoIdentity);
        }
        Ok(())
    }
}

/// Whether `address` binds every interface of its family: its first
/// component is `/ip4/0.0.0.0` or `/ip6/::`. Read as text, since this
/// crate names no backend type; anything else -- a specific IP, a name,
/// a string that is no multiaddr -- is not a wildcard.
fn is_wildcard_listener(address: &str) -> bool {
    let mut parts = address.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(""), Some(family @ ("ip4" | "ip6")), Some(host)) => host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_unspecified() && (family == "ip4") == ip.is_ipv4()),
        _ => false,
    }
}

/// The context [`RuntimeConfig::validate_into`] reads, gathered from a
/// whole profile.
pub(crate) fn context(profile: &crate::ProfileConfig) -> RuntimeContext<'_> {
    let connectivity = &profile.transport.connectivity;
    RuntimeContext {
        endpoints: &profile.endpoints,
        autonat_server_enabled: connectivity.autonat.server.enabled,
        relay_server_enabled: connectivity.relay.server.enabled,
        enabled_kademlia_modes: profile
            .discovery
            .providers
            .iter()
            .filter(|p| p.enabled && p.provider_type == DiscoveryProviderType::Kademlia)
            .map(|p| p.config.mode.as_deref())
            .collect(),
        ipc_enabled: profile.ipc.enabled,
        listen_addresses: &profile.transport.listen.addresses,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::{
        AndroidRuntimeConfig, AvailabilityMode, KeyUnlockPolicy, RuntimeConfig,
        background_restart_requires_user_authentication,
    };

    /// The schema's `EndpointId?` with no default (j185): absent is
    /// absent, never a label standing in for the first identity.
    #[test]
    fn the_android_endpoint_has_no_default() {
        assert_eq!(AndroidRuntimeConfig::default().endpoint(), None);
        let parsed: RuntimeConfig =
            serde_json::from_str(r#"{"deployment":"embedded-android","android":{}}"#)
                .expect("parses");
        assert_eq!(parsed.android.endpoint(), None);
    }

    #[test]
    fn the_literals_refuse_any_other_value() {
        for (key, value) in [
            ("foreground_service_type", "dataSync"),
            ("key_storage", "plaintext"),
        ] {
            let doc =
                format!(r#"{{"deployment":"embedded-android","android":{{"{key}":"{value}"}}}}"#);
            assert!(
                serde_json::from_str::<RuntimeConfig>(&doc).is_err(),
                "{key}={value} parsed"
            );
        }
        let stated = r#"{"android":{"foreground_service_type":"remoteMessaging",
            "key_storage":"android-keystore-aesgcm-wrap"}}"#;
        assert!(
            serde_json::from_str::<RuntimeConfig>(stated).is_ok(),
            "the control: the literal values themselves parse"
        );
    }

    #[test]
    fn only_stay_reachable_with_user_presence_needs_the_user_after_restart() {
        for (mode, policy, expected) in [
            (
                AvailabilityMode::StayReachable,
                KeyUnlockPolicy::UserPresence,
                true,
            ),
            (
                AvailabilityMode::StayReachable,
                KeyUnlockPolicy::BackgroundCompatible,
                false,
            ),
            (
                AvailabilityMode::ForegroundOnly,
                KeyUnlockPolicy::UserPresence,
                false,
            ),
            (
                AvailabilityMode::ForegroundOnly,
                KeyUnlockPolicy::BackgroundCompatible,
                false,
            ),
        ] {
            assert_eq!(
                background_restart_requires_user_authentication(mode, policy),
                expected,
                "{mode:?} + {policy:?}"
            );
        }
    }
}
