// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The administrative port over the daemon's admin socket.

use std::collections::BTreeSet;
use std::time::Duration;

use interweave_ipc_protocol::{
    AddressList, AddressParams, AddressResult, AddressesParams, AddressesResult, AdminStatusResult,
    EmptyResult, EndpointAddParams, EndpointAddResult, EndpointList, EndpointParams,
    EndpointRemoveResult, IpcVersion, MAX_BOOTSTRAP_ADDRESS_CHARS, MAX_SHUTDOWN_GRACE_MS, Method,
    OwnAddressList, PeerList, PeerListParams, Request, RequestedCapability, SetAdvertiseParams,
    SetAliasParams, SetAliasResult, SetDefaultParams, SetEnabledParams, SetEnabledResult,
    ShutdownParams, TrustList, TrustListParams, TrustSeedParams, TrustSeedResult, TrustSetParams,
    TrustSetResult,
};
use interweave_local_client_api::{
    AddressChange, AddressRefusal, AddressesChange, AdminBinding, AdminCapability, AdminPort,
    AdminStatus, AliasChange, BootstrapAddress, EndpointAddChange, EndpointAdminView,
    EndpointRemoveChange, EndpointTrustView, Generation, LocalAdminPort, OwnAddresses,
    PeerGateView, PeerLastSeen, TrustAdminView, TrustChange, TrustDirection, TrustSeed,
    TrustedPeer,
};
use interweave_transport_api::{EndpointId, TransportError, TransportIdentity};

use crate::connection::{Connection, open};
use crate::session::{IpcBinding, hello, mint};

const fn requested(capability: AdminCapability) -> RequestedCapability {
    match capability {
        AdminCapability::Status => RequestedCapability::AdminStatus,
        AdminCapability::Endpoints => RequestedCapability::AdminEndpoints,
        AdminCapability::Shutdown => RequestedCapability::AdminShutdown,
        AdminCapability::Trust => RequestedCapability::AdminTrust,
        AdminCapability::Peers => RequestedCapability::AdminPeers,
    }
}

/// Pages `trust` reads at most: a full allowlist is four (4096 peers,
/// 1024 a page), and a list changing while it is read may add a page or
/// so. A server past this is not paging, and the read ends `Internal`
/// rather than following it.
const MAX_TRUST_PAGES: usize = 8;

/// Pages `peers` reads at most: a full allowlist is nine (4096 peers, 456
/// a page, every row carrying `last_seen_ms`), and the gate's rows may
/// move while they are read.
const MAX_PEER_PAGES: usize = 10;

impl IpcBinding {
    /// Open an admin connection asking exactly `asked`.
    async fn admin_hello(
        &self,
        asked: BTreeSet<RequestedCapability>,
    ) -> Result<crate::connection::Opened, TransportError> {
        // No endpoint: an admin connection never holds a lease.
        let hello = hello(self.admin_kind(), None, asked, BTreeSet::new());
        open(&self.paths().admin, hello, |_| None).await
    }
}

impl AdminBinding for IpcBinding {
    type Admin = IpcAdmin;

    /// Every capability is named in the first hello: each is a 3.0
    /// capability (`LOCAL-IPC.md` §Version negotiation and phases), so the
    /// capability rule's probe connection -- owed only for a capability a
    /// minor above 3.0 brings -- is owed for none. Major 2's probe for
    /// `admin.trust` (2.1) and `admin.peers` (2.7) is gone with major 2;
    /// a capability arriving at 3.1 brings it back
    /// (`every_capability_is_named_in_a_first_hello`).
    async fn admin(
        &self,
        capabilities: BTreeSet<AdminCapability>,
    ) -> Result<IpcAdmin, TransportError> {
        let wanted: BTreeSet<RequestedCapability> =
            capabilities.into_iter().map(requested).collect();
        let opened = self.admin_hello(wanted).await?;
        let granted = opened
            .response
            .granted_capabilities
            .iter()
            .filter_map(|c| c.as_admin());
        Ok(IpcAdmin {
            port: LocalAdminPort::new(mint(), granted),
            version: opened.response.ipc_version,
            connection: opened.connection,
        })
    }
}

/// An administrative port over the daemon's admin socket. Dropped, its
/// connection ends.
pub struct IpcAdmin {
    port: LocalAdminPort,
    /// The version this port's connection negotiated: a method introduced
    /// after it is not sent ([`Self::speaks`]).
    version: IpcVersion,
    connection: Connection,
}

impl std::fmt::Debug for IpcAdmin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IpcAdmin")
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

impl IpcAdmin {
    /// Whether the connection's version has `method`: one it does not is
    /// refused `ProtocolUnsupported` here, with nothing sent, as the
    /// daemon would answer it (an unknown method). Every method is 3.0's,
    /// so on 3.0 this refuses none; it holds for a method a later minor
    /// brings.
    fn speaks(&self, method: Method) -> Result<(), TransportError> {
        if method.available_at(self.version) {
            Ok(())
        } else {
            Err(TransportError::ProtocolUnsupported)
        }
    }

    /// `admin.status`'s result object as the daemon sent it, the IPC
    /// server's counters included -- which [`AdminPort::status`]'s neutral
    /// view has no field for. What `transportctl status --json` prints
    /// (plan §16 (11)), so its output validates against the method's
    /// schema.
    ///
    /// # Errors
    /// As [`AdminPort::status`].
    pub async fn status_result(&self) -> Result<AdminStatusResult, TransportError> {
        self.connection
            .call::<AdminStatusResult>(Request::AdminStatus)
            .await
    }

    /// `admin.peers.list_addresses`'s result object as the daemon sent it,
    /// for `transportctl peers addresses --json` and
    /// [`AdminPort::bootstrap_addresses`].
    ///
    /// # Errors
    /// The call's, and `ProtocolUnsupported` on a version without the
    /// method ([`Self::speaks`]).
    pub async fn addresses_result(&self) -> Result<AddressList, TransportError> {
        self.speaks(Method::AdminPeersListAddresses)?;
        self.connection
            .call::<AddressList>(Request::AdminPeersListAddresses)
            .await
    }

    /// `admin.peers.own_addresses`'s result object as the daemon sent it,
    /// for `transportctl peers show --json` and
    /// [`AdminPort::own_addresses`].
    ///
    /// # Errors
    /// The call's, and `ProtocolUnsupported` on a version without the
    /// method ([`Self::speaks`]).
    pub async fn own_addresses_result(&self) -> Result<OwnAddressList, TransportError> {
        self.speaks(Method::AdminPeersOwnAddresses)?;
        self.connection
            .call::<OwnAddressList>(Request::AdminPeersOwnAddresses)
            .await
    }

    /// `admin.endpoints.list`'s result object as the daemon sent it, for
    /// `transportctl endpoints list --json` and [`AdminPort::leases`]: the
    /// one reader, so both hold the row to the one shape 3.0 serves, the
    /// 2.5 row, `persisted: true` -- a production daemon keeps the
    /// endpoint overlay, and one without it is never served. A row
    /// `persisted: false` is a daemon this client cannot read truthfully:
    /// refused `Internal`, never read as a value (ADR-0028 A 2026-10-11),
    /// as an unpersisted trust row is.
    ///
    /// # Errors
    /// The call's, and `Internal` for a row that does not persist.
    pub async fn endpoints_result(&self) -> Result<EndpointList, TransportError> {
        let list = self
            .connection
            .call::<EndpointList>(Request::AdminEndpointsList)
            .await?;
        if list.endpoints.iter().any(|row| !row.persisted) {
            return Err(TransportError::Internal);
        }
        Ok(list)
    }

    /// Every page of `admin.trust.list` as the daemon sent it, in order,
    /// for `transportctl trust list --json`: one `ipc/trust-list` each.
    ///
    /// # Errors
    /// As [`AdminPort::trust`]; `Internal` for a daemon whose cursor does
    /// not advance or that pages past [`MAX_TRUST_PAGES`].
    pub async fn trust_pages(&self) -> Result<Vec<TrustList>, TransportError> {
        let mut pages: Vec<TrustList> = Vec::new();
        let mut after: Option<TransportIdentity> = None;
        loop {
            if pages.len() == MAX_TRUST_PAGES {
                return Err(TransportError::Internal);
            }
            let page = self
                .connection
                .call::<TrustList>(Request::AdminTrustList(TrustListParams {
                    after: after.clone(),
                    ..TrustListParams::default()
                }))
                .await?;
            let next = page.next.clone();
            pages.push(page);
            match next {
                // A cursor that does not move would read the same page
                // for ever.
                Some(next) if after.as_ref().is_none_or(|after| next > *after) => {
                    after = Some(next);
                }
                Some(_) => return Err(TransportError::Internal),
                None => return Ok(pages),
            }
        }
    }

    /// Every page of one direction of an endpoint's effective set as the
    /// daemon sent it, in order, for `transportctl trust list --endpoint
    /// --json`: one `ipc/trust-list` each, every one carrying `set`.
    ///
    /// # Errors
    /// As [`AdminPort::endpoint_trust`]; `Internal` for a daemon whose
    /// cursor does not advance, that pages past [`MAX_TRUST_PAGES`], or
    /// that answers a page without `set` or with one that differs from
    /// the first's: the answer to an endpoint request is an endpoint
    /// answer, so a client reading it as the operator's would be shown
    /// the wrong list.
    pub async fn endpoint_trust_pages(
        &self,
        endpoint: &EndpointId,
        direction: TrustDirection,
    ) -> Result<Vec<TrustList>, TransportError> {
        self.speaks(Method::AdminTrustList)?;
        let mut pages: Vec<TrustList> = Vec::new();
        let mut after: Option<TransportIdentity> = None;
        loop {
            if pages.len() == MAX_TRUST_PAGES {
                return Err(TransportError::Internal);
            }
            let page = self
                .connection
                .call::<TrustList>(Request::AdminTrustList(TrustListParams {
                    after: after.clone(),
                    endpoint: Some(endpoint.clone()),
                    direction: Some(direction),
                }))
                .await?;
            if page.set.is_none() || page.set != pages.first().map_or(page.set, |p| p.set) {
                return Err(TransportError::Internal);
            }
            let next = page.next.clone();
            pages.push(page);
            match next {
                Some(next) if after.as_ref().is_none_or(|after| next > *after) => {
                    after = Some(next);
                }
                Some(_) => return Err(TransportError::Internal),
                None => return Ok(pages),
            }
        }
    }

    /// Every page of `admin.peers.list` as the daemon sent it, in order,
    /// for `transportctl peers list --json`: one `ipc/peer-list` each.
    ///
    /// # Errors
    /// `ProtocolUnsupported` on a version without the method
    /// ([`Self::speaks`]); as [`AdminPort::peers`]; and `Internal` for a
    /// daemon whose cursor does not advance or that pages past
    /// [`MAX_PEER_PAGES`].
    pub async fn peers_pages(&self) -> Result<Vec<PeerList>, TransportError> {
        self.speaks(Method::AdminPeersList)?;
        let mut pages: Vec<PeerList> = Vec::new();
        let mut after: Option<TransportIdentity> = None;
        loop {
            if pages.len() == MAX_PEER_PAGES {
                return Err(TransportError::Internal);
            }
            let page = self
                .connection
                .call::<PeerList>(Request::AdminPeersList(PeerListParams {
                    after: after.clone(),
                }))
                .await?;
            let next = page.next.clone();
            pages.push(page);
            match next {
                Some(next) if after.as_ref().is_none_or(|after| next > *after) => {
                    after = Some(next);
                }
                Some(_) => return Err(TransportError::Internal),
                None => return Ok(pages),
            }
        }
    }

    /// `admin.shutdown`, its grace ABSENT when `None` -- "the daemon's
    /// default" (`ipc/shutdown-params`), which the neutral
    /// [`AdminPort::shutdown`] cannot say, since it always names one. A
    /// grace past the wire's ceiling is that ceiling, not a refusal.
    ///
    /// # Errors
    /// As [`AdminPort::shutdown`].
    pub async fn request_shutdown(&self, grace: Option<Duration>) -> Result<(), TransportError> {
        let grace_ms = grace.map(|grace| {
            u32::try_from(grace.as_millis())
                .unwrap_or(u32::MAX)
                .min(MAX_SHUTDOWN_GRACE_MS)
        });
        self.connection
            .call::<EmptyResult>(Request::AdminShutdown(ShutdownParams { grace_ms }))
            .await
            .map(|_| ())
    }
}

impl AdminPort for IpcAdmin {
    fn port(&self) -> &LocalAdminPort {
        &self.port
    }

    async fn status(&self) -> Result<AdminStatus, TransportError> {
        self.status_result().await.map(AdminStatus::from)
    }

    async fn leases(&self) -> Result<Vec<EndpointAdminView>, TransportError> {
        self.endpoints_result().await.map(|list| {
            list.endpoints
                .into_iter()
                .map(EndpointAdminView::from)
                .collect()
        })
    }

    async fn revoke_endpoint(&self, endpoint: EndpointId) -> Result<(), TransportError> {
        self.connection
            .call::<EmptyResult>(Request::AdminEndpointsRevoke(EndpointParams { endpoint }))
            .await
            .map(|_| ())
    }

    async fn set_endpoint_enabled(
        &self,
        endpoint: EndpointId,
        enabled: bool,
    ) -> Result<Option<Generation>, TransportError> {
        self.connection
            .call::<SetEnabledResult>(Request::AdminEndpointsSetEnabled(SetEnabledParams {
                endpoint,
                enabled,
            }))
            .await
            .map(|result| result.revoked_epoch)
    }

    async fn set_default_endpoint(
        &self,
        endpoint: Option<EndpointId>,
    ) -> Result<(), TransportError> {
        self.connection
            .call::<EmptyResult>(Request::AdminEndpointsSetDefault(SetDefaultParams {
                endpoint,
            }))
            .await
            .map(|_| ())
    }

    /// `endpoint-add-result`: a refusal by its name, never an error.
    async fn add_endpoint(
        &self,
        key: String,
        alias: String,
        allowed_client_kinds: Vec<String>,
    ) -> Result<EndpointAddChange, TransportError> {
        self.connection
            .call::<EndpointAddResult>(Request::AdminEndpointsAdd(EndpointAddParams {
                key,
                alias,
                allowed_client_kinds,
            }))
            .await
            .map(|result| result.0)
    }

    /// `endpoint-remove-result`: a refusal by its name, never an error.
    async fn remove_endpoint(
        &self,
        endpoint: EndpointId,
    ) -> Result<EndpointRemoveChange, TransportError> {
        self.connection
            .call::<EndpointRemoveResult>(Request::AdminEndpointsRemove(EndpointParams {
                endpoint,
            }))
            .await
            .map(|result| result.0)
    }

    async fn set_endpoint_advertise(
        &self,
        endpoint: EndpointId,
        advertise: bool,
    ) -> Result<(), TransportError> {
        self.connection
            .call::<EmptyResult>(Request::AdminEndpointsSetAdvertise(SetAdvertiseParams {
                endpoint,
                advertise,
            }))
            .await
            .map(|_| ())
    }

    /// `set-alias-result`: a refusal by its name, never an error.
    async fn set_endpoint_alias(
        &self,
        endpoint: EndpointId,
        alias: String,
    ) -> Result<AliasChange, TransportError> {
        self.connection
            .call::<SetAliasResult>(Request::AdminEndpointsSetAlias(SetAliasParams {
                endpoint,
                alias,
            }))
            .await
            .map(|result| result.0)
    }

    async fn shutdown(&self, grace: Duration) -> Result<(), TransportError> {
        self.request_shutdown(Some(grace)).await
    }

    /// Read page by page and joined, the rows in order.
    async fn peers(&self) -> Result<Vec<PeerGateView>, TransportError> {
        Ok(self
            .peers_pages()
            .await?
            .into_iter()
            .flat_map(|page| page.peers)
            .map(|row| PeerGateView {
                peer: row.peer,
                connected: row.connected,
                last_seen: match row.last_seen_ms {
                    None => PeerLastSeen::Unreported,
                    Some(None) => PeerLastSeen::Never,
                    Some(Some(at)) => PeerLastSeen::At(at),
                },
                backoff_until: row.backoff_until,
                quarantined_until: row.quarantined_until,
                route_held_until: row.route_held_until,
                last_outcome: row.last_outcome,
            })
            .collect())
    }

    /// Read page by page and joined: the local peer from the first page,
    /// the rows in order, each saying where its peer comes from (the 2.3
    /// row, the one 3.0 serves).
    async fn trust(&self) -> Result<TrustAdminView, TransportError> {
        self.speaks(Method::AdminTrustList)?;
        let pages = self.trust_pages().await?;
        let local_peer = pages.first().and_then(|page| page.local_peer.clone());
        // Every row a production daemon sends is persisted with its
        // source (a composition without a store is never served over IPC,
        // LOCAL-CLIENT.md §7 item 11); a row without one is a daemon this
        // client cannot read truthfully.
        let allowed = pages
            .into_iter()
            .flat_map(|page| page.allowed)
            .map(|row| match row.source {
                Some(source) if row.persisted => Ok(TrustedPeer {
                    peer: row.peer,
                    persisted: true,
                    source,
                }),
                _ => Err(TransportError::Internal),
            })
            .collect::<Result<_, _>>()?;
        Ok(TrustAdminView {
            local_peer,
            allowed,
        })
    }

    /// `trust-set-result`, which names a superseded set (the 2.10 result,
    /// the one 3.0 serves).
    async fn set_trust(
        &self,
        peer: TransportIdentity,
        allowed: bool,
    ) -> Result<TrustChange, TransportError> {
        self.connection
            .call::<TrustSetResult>(Request::AdminTrustSet(TrustSetParams {
                peer,
                allowed,
                endpoint: None,
            }))
            .await
            .map(TrustChange::from)
    }

    /// Read page by page and joined, as [`Self::trust`] reads the
    /// operator's list, with the `set` the daemon names; every row
    /// persisted with its source, as there.
    async fn endpoint_trust(
        &self,
        endpoint: EndpointId,
        direction: TrustDirection,
    ) -> Result<EndpointTrustView, TransportError> {
        let pages = self.endpoint_trust_pages(&endpoint, direction).await?;
        let set = pages
            .first()
            .and_then(|page| page.set)
            .ok_or(TransportError::Internal)?;
        let allowed = pages
            .into_iter()
            .flat_map(|page| page.allowed)
            .map(|row| match row.source {
                Some(source) if row.persisted => Ok(TrustedPeer {
                    peer: row.peer,
                    persisted: true,
                    source,
                }),
                _ => Err(TransportError::Internal),
            })
            .collect::<Result<_, _>>()?;
        Ok(EndpointTrustView { set, allowed })
    }

    async fn set_endpoint_trust(
        &self,
        endpoint: EndpointId,
        peer: TransportIdentity,
        allowed: bool,
    ) -> Result<TrustChange, TransportError> {
        self.speaks(Method::AdminTrustSet)?;
        self.connection
            .call::<TrustSetResult>(Request::AdminTrustSet(TrustSetParams {
                peer,
                allowed,
                endpoint: Some(endpoint),
            }))
            .await
            .map(TrustChange::from)
    }

    async fn seed_endpoint_trust(&self, endpoint: EndpointId) -> Result<TrustSeed, TransportError> {
        self.speaks(Method::AdminTrustSeed)?;
        self.connection
            .call::<TrustSeedResult>(Request::AdminTrustSeed(TrustSeedParams { endpoint }))
            .await
            .map(TrustSeed::from)
    }

    async fn add_bootstrap_address(
        &self,
        address: String,
    ) -> Result<AddressChange, TransportError> {
        self.address_move(Method::AdminPeersAddAddress, address)
            .await
    }

    /// A list the schema refuses (none, or more than 16 lines) is refused
    /// `InvalidArgument` without a round trip. An answer
    /// whose rows do not match the lines sent one for one is the daemon's
    /// `ProtocolViolation`: a client could not say which line each row is.
    ///
    /// A LINE OUTSIDE THE SCHEMA'S LENGTH BOUND IS SENT AS [`WITHHELD`], so
    /// the list is answered as the embedded binding answers it: every
    /// line's grammar is checked first, in order, and the first line that
    /// fails is named -- an earlier line outside the grammar, or the
    /// withheld one, which no entry of the grammar is that long or empty
    /// to pass. Refusing the withheld line locally instead would name it
    /// over an earlier bad line; sending it would make the whole list a
    /// params error, `InvalidArgument`, where the method names a line
    /// (`a_list_is_added_all_or_none_at_every_binding`).
    async fn add_bootstrap_addresses(
        &self,
        addresses: Vec<String>,
    ) -> Result<AddressesChange, TransportError> {
        self.speaks(Method::AdminPeersAddAddresses)?;
        // The capability before any local answer, in the server's own
        // order: a port without `admin.peers` is `CapabilityDenied` for
        // every list, as at every other binding.
        if !self.port.holds(AdminCapability::Peers) {
            return Err(TransportError::CapabilityDenied);
        }
        let lines = addresses.len();
        if lines == 0 || lines > interweave_local_client_api::MAX_ADDRESSES_PER_ADD {
            return Err(TransportError::InvalidArgument);
        }
        let withheld = addresses.iter().position(|a| outside_the_bound(a));
        let addresses = addresses
            .into_iter()
            .map(|a| {
                if outside_the_bound(&a) {
                    WITHHELD.to_owned()
                } else {
                    a
                }
            })
            .collect();
        let change: AddressesChange = self
            .connection
            .call::<AddressesResult>(Request::AdminPeersAddAddresses(AddressesParams {
                addresses,
            }))
            .await?
            .into();
        match (change, withheld) {
            (AddressesChange::Added(rows), None) if rows.len() == lines => {
                Ok(AddressesChange::Added(rows))
            }
            (
                AddressesChange::Refused {
                    reason,
                    line: Some(line),
                    detail,
                },
                withheld,
            ) if line < lines
                && withheld.is_none_or(|w| reason == AddressRefusal::Grammar && line <= w) =>
            {
                let detail = if withheld == Some(line) {
                    BOUND_DETAIL.to_owned()
                } else {
                    detail
                };
                Ok(AddressesChange::Refused {
                    reason,
                    line: Some(line),
                    detail,
                })
            }
            (change @ AddressesChange::Refused { line: None, .. }, None) => Ok(change),
            // Rows that do not match the lines, a line out of range, or --
            // with a line withheld -- anything but a grammar refusal at or
            // before it: the grammar is checked first, and the withheld
            // line fails it.
            _ => Err(TransportError::ProtocolViolation),
        }
    }

    async fn remove_bootstrap_address(
        &self,
        address: String,
    ) -> Result<AddressChange, TransportError> {
        self.address_move(Method::AdminPeersRemoveAddress, address)
            .await
    }

    async fn bootstrap_addresses(&self) -> Result<Vec<BootstrapAddress>, TransportError> {
        Ok(self
            .addresses_result()
            .await?
            .addresses
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn own_addresses(&self) -> Result<OwnAddresses, TransportError> {
        Ok(self.own_addresses_result().await?.into())
    }
}

/// What a line outside the schema's length bound is sent as in an
/// all-or-none add: inside the bound, and outside the grammar, which
/// every entry of needs a `/p2p/` component.
const WITHHELD: &str = "a line outside 1..=1024 characters, withheld by the client";

/// The refusal's detail for a line outside the bound.
const BOUND_DETAIL: &str = "the address is not 1..=1024 characters";

/// Outside `peers-address-params`' bound: empty, or more than
/// [`MAX_BOOTSTRAP_ADDRESS_CHARS`] characters (counted as the schema
/// counts, in characters).
fn outside_the_bound(address: &str) -> bool {
    address.is_empty() || address.chars().count() > MAX_BOOTSTRAP_ADDRESS_CHARS
}

impl IpcAdmin {
    async fn address_move(
        &self,
        method: Method,
        address: String,
    ) -> Result<AddressChange, TransportError> {
        self.speaks(method)?;
        // The capability first, as the server judges it before the params
        // (`bootstrap_addresses_answer_alike_at_every_binding`).
        if !self.port.holds(AdminCapability::Peers) {
            return Err(TransportError::CapabilityDenied);
        }
        // Answered here as the method answers it, since the schema's
        // length bound would make it a params error, `InvalidArgument`:
        // no entry of the grammar is that long or empty, so an add is a
        // grammar refusal and a removal finds nothing to remove.
        if outside_the_bound(&address) {
            return Ok(if method == Method::AdminPeersAddAddress {
                AddressChange::Refused {
                    reason: AddressRefusal::Grammar,
                    detail: BOUND_DETAIL.to_owned(),
                }
            } else {
                AddressChange::Unchanged
            });
        }
        let params = AddressParams { address };
        let request = if method == Method::AdminPeersAddAddress {
            Request::AdminPeersAddAddress(params)
        } else {
            Request::AdminPeersRemoveAddress(params)
        };
        Ok(self.connection.call::<AddressResult>(request).await?.into())
    }
}

#[cfg(test)]
mod tests {
    use interweave_ipc_protocol::{IPC_MAJOR, RequestedCapability, Since};

    /// The binding names every capability in its first hello, with no
    /// probe connection (`IpcBinding::admin`). That is right only while
    /// every capability is a minor-0 capability of the major spoken; one a
    /// later minor brings fails here, and the capability rule's probe
    /// (`LOCAL-IPC.md` §Version negotiation) must come back with it.
    #[test]
    fn every_capability_is_named_in_a_first_hello() {
        for capability in [
            RequestedCapability::Events,
            RequestedCapability::Commands,
            RequestedCapability::EndpointsQuery,
            RequestedCapability::AdminStatus,
            RequestedCapability::AdminEndpoints,
            RequestedCapability::AdminShutdown,
            RequestedCapability::AdminTrust,
            RequestedCapability::AdminPeers,
        ] {
            assert_eq!(
                capability.since(),
                Since {
                    major: IPC_MAJOR,
                    minor: 0
                },
                "{capability:?}"
            );
        }
    }
}
