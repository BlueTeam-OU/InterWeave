// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The closed method catalogue (`ipc/method.schema.json`) and THE table
//! of method -> authority domain -> required capability -> since
//! (`LOCAL-IPC.md` §Method catalogue; plan §16 (3)).
//!
//! The capability a method needs is not in the schema -- the contract
//! meta-schema admits no such annotation -- so it lives in two places:
//! the prose table and [`Method::entry`]. `tests/schema_agreement.rs`
//! binds the two, and the names to the schema, in both directions.

use serde::{Deserialize, Serialize};

use crate::handshake::{AuthorityDomain, RequestedCapability};
use crate::version::{IpcVersion, Since};

/// One request method of the closed catalogue.
///
/// A name outside it is answered `ProtocolUnsupported` and the connection
/// stays, which is why a request's envelope carries the name as text and
/// [`Method::parse`] is asked afterwards rather than serde refusing the
/// frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Method {
    /// Take a join reference on a channel.
    #[serde(rename = "channel.join")]
    ChannelJoin,
    /// Release a join reference.
    #[serde(rename = "channel.leave")]
    ChannelLeave,
    /// Publish on a joined channel.
    #[serde(rename = "broadcast.publish")]
    BroadcastPublish,
    /// Send a directed message from the connection's leased endpoint.
    #[serde(rename = "direct.send")]
    DirectSend,
    /// Query a trusted peer's endpoint directory.
    #[serde(rename = "endpoints.query")]
    EndpointsQuery,
    /// Read the administrative status view.
    #[serde(rename = "admin.status")]
    AdminStatus,
    /// List the configured endpoints and their runtime state.
    #[serde(rename = "admin.endpoints.list")]
    AdminEndpointsList,
    /// Revoke an endpoint's live lease.
    #[serde(rename = "admin.endpoints.revoke")]
    AdminEndpointsRevoke,
    /// Enable or disable an endpoint (kept in the endpoint overlay).
    #[serde(rename = "admin.endpoints.set_enabled")]
    AdminEndpointsSetEnabled,
    /// Set or clear the default endpoint (kept in the endpoint overlay).
    #[serde(rename = "admin.endpoints.set_default")]
    AdminEndpointsSetDefault,
    /// Add an endpoint by its sealed key file's name (3.0, ADR-0061
    /// decision 2); kept in the endpoint overlay's `added` member.
    #[serde(rename = "admin.endpoints.add")]
    AdminEndpointsAdd,
    /// Remove an added endpoint (3.0, ADR-0061 decision 7).
    #[serde(rename = "admin.endpoints.remove")]
    AdminEndpointsRemove,
    /// Set whether an endpoint may appear in the directory (3.0, ADR-0061
    /// decision 5).
    #[serde(rename = "admin.endpoints.set_advertise")]
    AdminEndpointsSetAdvertise,
    /// Rename an added endpoint's local alias (3.0, ADR-0061 decision 7).
    #[serde(rename = "admin.endpoints.set_alias")]
    AdminEndpointsSetAlias,
    /// Shut the runtime down.
    #[serde(rename = "admin.shutdown")]
    AdminShutdown,
    /// One page of the data-plane allowlist (2.1).
    #[serde(rename = "admin.trust.list")]
    AdminTrustList,
    /// Allow a peer or revoke it (2.1); kept in the state directory's
    /// trust overlay across restarts (ADR-0028 A 2026-10-07).
    #[serde(rename = "admin.trust.set")]
    AdminTrustSet,
    /// Give a configured endpoint its own trust set from what it admits
    /// now, if it has none (3.0, ADR-0067 decision 6).
    #[serde(rename = "admin.trust.seed")]
    AdminTrustSeed,
    /// One page of the dial gate's per-peer state (2.2, under
    /// `admin.status`).
    #[serde(rename = "admin.peers.list")]
    AdminPeersList,
    /// Add a bootstrap address to the static-bootstrap provider (2.7);
    /// kept in the bootstrap overlay across restarts (ADR-0062).
    #[serde(rename = "admin.peers.add_address")]
    AdminPeersAddAddress,
    /// Remove an address the overlay added (2.7).
    #[serde(rename = "admin.peers.remove_address")]
    AdminPeersRemoveAddress,
    /// Every bootstrap address, authored and added (2.7).
    #[serde(rename = "admin.peers.list_addresses")]
    AdminPeersListAddresses,
    /// This node's own addresses for the show side (2.8, ADR-0062
    /// decision 4).
    #[serde(rename = "admin.peers.own_addresses")]
    AdminPeersOwnAddresses,
    /// Bootstrap addresses added all or none (2.9, ADR-0062 decision 4).
    #[serde(rename = "admin.peers.add_addresses")]
    AdminPeersAddAddresses,
    /// Read this session's broadcast losses (2.11).
    #[serde(rename = "session.losses")]
    SessionLosses,
}

/// One row of the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodEntry {
    /// The socket the method is served on; on the other it is refused
    /// `CapabilityDenied` before dispatch (ADR-0037).
    pub domain: AuthorityDomain,
    /// The capability the connection must hold.
    pub capability: RequestedCapability,
    /// The IPC version that introduced the method: it is accepted only on
    /// a connection that negotiated this pair or a later one. Every method
    /// is [`Since::THREE_ZERO`]: the version each arrived at on major 2 is
    /// its history, in its doc comment and `LOCAL-IPC.md`'s table.
    pub since: Since,
}

impl Method {
    /// Every method, in catalogue order. `tests/schema_agreement.rs`
    /// holds this list to the enum's own variants, so a variant missing
    /// here fails a test rather than escaping one.
    pub const ALL: [Self; 25] = [
        Self::ChannelJoin,
        Self::ChannelLeave,
        Self::BroadcastPublish,
        Self::DirectSend,
        Self::EndpointsQuery,
        Self::AdminStatus,
        Self::AdminEndpointsList,
        Self::AdminEndpointsRevoke,
        Self::AdminEndpointsSetEnabled,
        Self::AdminEndpointsSetDefault,
        Self::AdminEndpointsAdd,
        Self::AdminEndpointsRemove,
        Self::AdminEndpointsSetAdvertise,
        Self::AdminEndpointsSetAlias,
        Self::AdminShutdown,
        Self::AdminTrustList,
        Self::AdminTrustSet,
        Self::AdminTrustSeed,
        Self::AdminPeersList,
        Self::AdminPeersAddAddress,
        Self::AdminPeersRemoveAddress,
        Self::AdminPeersListAddresses,
        Self::AdminPeersOwnAddresses,
        Self::AdminPeersAddAddresses,
        Self::SessionLosses,
    ];

    /// THE table: an exhaustive match, so a new method does not compile
    /// until it has a domain and a capability; its `since` is 3.0 until
    /// a method arrives on a later minor, which names its own.
    #[must_use]
    pub const fn entry(self) -> MethodEntry {
        use AuthorityDomain::{Admin, Data};
        use RequestedCapability as C;
        let (domain, capability) = match self {
            Self::ChannelJoin
            | Self::ChannelLeave
            | Self::BroadcastPublish
            | Self::DirectSend
            | Self::SessionLosses => (Data, C::Commands),
            Self::EndpointsQuery => (Data, C::EndpointsQuery),
            Self::AdminStatus | Self::AdminPeersList => (Admin, C::AdminStatus),
            Self::AdminEndpointsList
            | Self::AdminEndpointsRevoke
            | Self::AdminEndpointsSetEnabled
            | Self::AdminEndpointsSetDefault
            | Self::AdminEndpointsAdd
            | Self::AdminEndpointsRemove
            | Self::AdminEndpointsSetAdvertise
            | Self::AdminEndpointsSetAlias => (Admin, C::AdminEndpoints),
            Self::AdminShutdown => (Admin, C::AdminShutdown),
            Self::AdminTrustList | Self::AdminTrustSet | Self::AdminTrustSeed => {
                (Admin, C::AdminTrust)
            }
            Self::AdminPeersAddAddress
            | Self::AdminPeersRemoveAddress
            | Self::AdminPeersListAddresses
            | Self::AdminPeersOwnAddresses
            | Self::AdminPeersAddAddresses => (Admin, C::AdminPeers),
        };
        MethodEntry {
            domain,
            capability,
            since: Since::THREE_ZERO,
        }
    }

    /// The method's wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ChannelJoin => "channel.join",
            Self::ChannelLeave => "channel.leave",
            Self::BroadcastPublish => "broadcast.publish",
            Self::DirectSend => "direct.send",
            Self::EndpointsQuery => "endpoints.query",
            Self::AdminStatus => "admin.status",
            Self::AdminEndpointsList => "admin.endpoints.list",
            Self::AdminEndpointsRevoke => "admin.endpoints.revoke",
            Self::AdminEndpointsSetEnabled => "admin.endpoints.set_enabled",
            Self::AdminEndpointsSetDefault => "admin.endpoints.set_default",
            Self::AdminEndpointsAdd => "admin.endpoints.add",
            Self::AdminEndpointsRemove => "admin.endpoints.remove",
            Self::AdminEndpointsSetAdvertise => "admin.endpoints.set_advertise",
            Self::AdminEndpointsSetAlias => "admin.endpoints.set_alias",
            Self::AdminShutdown => "admin.shutdown",
            Self::AdminTrustList => "admin.trust.list",
            Self::AdminTrustSet => "admin.trust.set",
            Self::AdminTrustSeed => "admin.trust.seed",
            Self::AdminPeersList => "admin.peers.list",
            Self::AdminPeersAddAddress => "admin.peers.add_address",
            Self::AdminPeersRemoveAddress => "admin.peers.remove_address",
            Self::AdminPeersListAddresses => "admin.peers.list_addresses",
            Self::AdminPeersOwnAddresses => "admin.peers.own_addresses",
            Self::AdminPeersAddAddresses => "admin.peers.add_addresses",
            Self::SessionLosses => "session.losses",
        }
    }

    /// The method a wire name names, or `None` for a name outside the
    /// catalogue (answered `ProtocolUnsupported`).
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.as_str() == name)
    }

    /// Whether a connection that negotiated `version` may use the method:
    /// minors are additive, so a method is unknown below the (major,
    /// minor) pair that introduced it, compared as a pair.
    #[must_use]
    pub const fn available_at(self, version: IpcVersion) -> bool {
        self.entry().since.admits(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wire_name_is_the_serde_name_and_parses_back() {
        for method in Method::ALL {
            assert_eq!(
                serde_json::to_value(method).expect("ser"),
                serde_json::json!(method.as_str())
            );
            assert_eq!(Method::parse(method.as_str()), Some(method));
        }
        assert_eq!(Method::parse("admin.trust.add"), None);
        assert_eq!(Method::parse("Channel.Join"), None, "names are exact");
    }

    #[test]
    fn a_domain_and_its_capabilities_never_cross() {
        for method in Method::ALL {
            let entry = method.entry();
            let admin = entry.capability.as_admin().is_some();
            assert_eq!(
                admin,
                entry.domain == AuthorityDomain::Admin,
                "{}: an admin capability is served only on the admin socket",
                method.as_str()
            );
            assert_eq!(
                method.as_str().starts_with("admin."),
                admin,
                "{}",
                method.as_str()
            );
        }
    }

    /// Every method is a 3.0 method: available on a 3.0 connection and
    /// unknown to a 2.13 one, whose minor is higher than 3.0's -- the
    /// case a comparison of the minor alone gets wrong.
    #[test]
    fn a_method_is_available_from_three_zero_and_not_at_two_thirteen() {
        let at = |major, minor| IpcVersion { major, minor };
        for method in Method::ALL {
            assert_eq!(
                method.entry().since,
                Since::THREE_ZERO,
                "{}",
                method.as_str()
            );
            assert!(method.available_at(at(3, 0)), "{}", method.as_str());
            assert!(!method.available_at(at(2, 13)), "{}", method.as_str());
            assert!(!method.available_at(at(2, 0)), "{}", method.as_str());
        }
    }

    #[test]
    fn the_trust_methods_are_admin_under_trust() {
        for method in [
            Method::AdminTrustList,
            Method::AdminTrustSet,
            Method::AdminTrustSeed,
        ] {
            let entry = method.entry();
            assert_eq!(entry.domain, AuthorityDomain::Admin);
            assert_eq!(entry.capability, RequestedCapability::AdminTrust);
        }
    }

    /// A method never arrives before its capability: one stated at an
    /// earlier version than its capability could be asked on a connection
    /// that could not hold it. It may arrive AFTER it -- a later read
    /// under an existing capability, as `admin.peers.list` (2.2) was under
    /// `admin.status` (2.0) -- since a connection below the method's
    /// version is answered as for an unknown method whatever it holds
    /// (`available_at`).
    #[test]
    fn no_method_arrives_before_its_capability() {
        for method in Method::ALL {
            let entry = method.entry();
            assert!(
                entry.since >= entry.capability.since(),
                "{}",
                method.as_str()
            );
        }
    }

    /// The endpoint set's four writes (3.0) are admin methods under
    /// `admin.endpoints`, never on the data socket.
    #[test]
    fn the_endpoint_set_methods_are_admin_under_endpoints() {
        for method in [
            Method::AdminEndpointsAdd,
            Method::AdminEndpointsRemove,
            Method::AdminEndpointsSetAdvertise,
            Method::AdminEndpointsSetAlias,
        ] {
            let entry = method.entry();
            assert_eq!(entry.domain, AuthorityDomain::Admin, "{}", method.as_str());
            assert_eq!(
                entry.capability,
                RequestedCapability::AdminEndpoints,
                "{}",
                method.as_str()
            );
        }
    }

    #[test]
    fn the_peers_read_is_admin_under_status() {
        let entry = Method::AdminPeersList.entry();
        assert_eq!(entry.domain, AuthorityDomain::Admin);
        assert_eq!(entry.capability, RequestedCapability::AdminStatus);
    }

    /// `admin.trust.seed` is a 3.0 method: major 3 is the only major a
    /// server speaks, so no connection reaches it below 3.0, and the gate
    /// is the table's `since` (the same one every method is held to).
    #[test]
    fn the_seed_method_is_gated_at_three_zero() {
        use crate::version::IpcVersion;
        let entry = Method::AdminTrustSeed.entry();
        assert_eq!(entry.since, Since::THREE_ZERO);
        let at = |major, minor| IpcVersion { major, minor };
        assert!(entry.since.admits(at(3, 0)), "the control");
        assert!(!entry.since.admits(at(2, 13)), "the last minor of major 2");
        assert_eq!(
            Method::parse("admin.trust.seed"),
            Some(Method::AdminTrustSeed)
        );
    }
}
