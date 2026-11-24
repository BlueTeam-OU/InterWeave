// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The result each method answers with (`LOCAL-IPC.md` §Method catalogue's
//! Result column): `empty-result`, `send-result`,
//! `endpoints:directory-response`, `admin-status`, `endpoint-list` and
//! `set-enabled-result`.
//!
//! Each is built FROM the neutral port's answer, so the server never
//! assembles a result by hand, and each deserializes with the schema's
//! bounds, so a client never trusts a server's list to be the size the
//! contract says.

use std::collections::BTreeSet;

use interweave_local_client_api::{
    AddressChange, AddressRefusal, AddressSource, AddressesChange, AdminStatus, BootstrapAddress,
    EndpointAdminView, EndpointSetKind, EndpointTrustView, Generation, IngressCounts, LeaseRecord,
    LineChange, MAX_ADDRESSES_PER_ADD, MAX_CLIENT_KIND_CHARS, MAX_OWN_ADDRESSES, OwnAddress,
    OwnAddressKind, OwnAddresses, PeerGateView, PeerLastSeen, PeerOutcome, PreAuthCounts,
    TrustAdminView, TrustChange, TrustRefusal, TrustSeed, TrustSource,
};
use interweave_transport_api::{
    ConnectivitySummary, EndpointDirectoryV1, EndpointId, Health, MAX_DIRECTORY_ENTRIES,
    TransportError, TransportIdentity,
};
use serde::{Deserialize, Serialize};

/// The most rows `admin.endpoints.list` carries
/// (`ipc/endpoint-list.schema.json` `maxItems`).
pub const MAX_ENDPOINT_ROWS: usize = 64;

/// Rows on one `ipc/trust-list` page: a full allowlist of 4096 is four
/// pages, and a page stays under the 128 KiB body with the first page's
/// `local_peer` -- 82-byte rows at 2.1 (architect-cto's ruling of
/// 2026-10-04, LOCAL-IPC.md `admin.trust.list`), 105-byte rows at 2.3,
/// which carry `source` -- each with its array comma and a 52-character
/// peer id; about 105 KiB a page.
/// `a_full_trust_page_of_the_largest_rows_fits_the_body`.
pub const MAX_TRUST_PAGE_ROWS: usize = 1024;

// The shapes each widened behind a minor of major 2 -- the persisted
// endpoint row (2.5), the trust row's `source` (2.3), the peer row's route
// hold (2.6) and last-seen time (2.12), the address results' `own_peer`
// (2.9) and `superseded` (2.10), `admin-status`'s refusal total (2.13) --
// are served on major 3 in the shape 2.13 served, to every connection,
// and the older shape of each to none (`LOCAL-IPC.md` §Version
// negotiation and phases, "One shape per document on 3.0"). So no
// builder here takes the connection's minor: 3.0 has one shape per
// document, until a shape widens behind a 3.<n>.

/// Rows on one `ipc/peer-list` page: a row is up to 194 bytes, so 1024
/// would overflow the 128 KiB body and 512 is about 100 KB; a full
/// allowlist of 4096 is eight pages (architect-cto's ruling of
/// 2026-10-07, LOCAL-IPC.md `admin.peers.list`).
/// `a_full_page_of_the_largest_rows_fits_the_body`.
pub const MAX_PEER_PAGE_ROWS: usize = 512;

/// Rows a server puts on one `ipc/peer-list` page, every row carrying
/// `last_seen_ms` (since 2.12, so on every 3.0 row): it adds up to 36
/// bytes a row, and 512 of the largest rows would be about 139 KB against
/// the 128 KiB body; 456 is about 124 KB, and a full allowlist of 4096 is
/// nine pages. A reader still accepts [`MAX_PEER_PAGE_ROWS`], the
/// schema's bound.
/// `a_full_page_of_the_largest_rows_fits_the_body`.
pub const MAX_PEER_PAGE_ROWS_SINCE_LAST_SEEN: usize = 456;

/// The most rows `admin.peers.list_addresses` carries
/// (`ipc/peers-address-list.schema.json` `maxItems`): the static-bootstrap
/// provider's bound, authored and added together (ADR-0062).
pub const MAX_BOOTSTRAP_ADDRESS_ROWS: usize = 64;

/// The most characters an `admin.peers` answer's `detail` carries
/// (`ipc/peers-address-result.schema.json` `maxLength`).
pub const MAX_ADDRESS_DETAIL_CHARS: usize = 512;

/// `ipc/empty-result`: the `{}` of a method with nothing to say.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyResult {}

/// `ipc/send-result`: the endpoint the remote resolved the send to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendResult {
    /// Never absent: an accepted send resolved to exactly one endpoint.
    #[serde(deserialize_with = "crate::derived::endpoint")]
    pub resolved_endpoint: EndpointId,
}

/// `endpoints:directory-response`: a remote peer's endpoint directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DirectoryResult {
    /// At most 32, unique.
    pub endpoints: Vec<EndpointId>,
    /// The remote's suggested freshness, as sent: the full u32 is legal,
    /// and clamping is the receiver's.
    pub ttl_ms: u32,
    /// Diagnostic only; never used for freshness.
    pub generated_at_ms: u64,
}

impl From<EndpointDirectoryV1> for DirectoryResult {
    fn from(directory: EndpointDirectoryV1) -> Self {
        Self {
            endpoints: directory.endpoints,
            ttl_ms: directory.ttl_ms,
            generated_at_ms: directory.generated_at_ms,
        }
    }
}

impl From<DirectoryResult> for EndpointDirectoryV1 {
    fn from(result: DirectoryResult) -> Self {
        Self {
            generated_at_ms: result.generated_at_ms,
            ttl_ms: result.ttl_ms,
            endpoints: result.endpoints,
        }
    }
}

impl<'de> Deserialize<'de> for DirectoryResult {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            #[serde(deserialize_with = "crate::derived::endpoints")]
            endpoints: Vec<EndpointId>,
            ttl_ms: u32,
            generated_at_ms: u64,
        }
        let wire = Wire::deserialize(d)?;
        bounded_unique(&wire.endpoints, MAX_DIRECTORY_ENTRIES).map_err(serde::de::Error::custom)?;
        Ok(Self {
            endpoints: wire.endpoints,
            ttl_ms: wire.ttl_ms,
            generated_at_ms: wire.generated_at_ms,
        })
    }
}

/// `ipc/admin-status`: the administrative status view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminStatusResult {
    /// The runtime's aggregate health.
    pub health: Health,
    /// This profile's identity.
    pub peer: TransportIdentity,
    /// The full connectivity summary.
    pub connectivity: ConnectivitySummary,
    /// The IPC server's own counters.
    pub ipc: IpcCounters,
    /// Pre-authentication admission counters, when the binding has them.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "absent_or_pre_auth"
    )]
    pub pre_auth: Option<PreAuthCounters>,
    /// The post-authentication ingress limiters' state, when the binding
    /// has it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "absent_or_ingress"
    )]
    pub ingress: Option<IngressCounters>,
}

impl AdminStatusResult {
    /// The port's view, with the counters only the IPC server can keep:
    /// the refusal total whenever the port keeps one
    /// (`the_refused_total_is_served_when_kept`).
    /// `ipc.active_leases` is the port's count, never the server's.
    #[must_use]
    pub fn new(status: AdminStatus, counters: ServerCounters) -> Self {
        Self {
            health: status.health,
            peer: status.peer,
            connectivity: status.connectivity,
            ipc: IpcCounters {
                data_connections: counters.data_connections,
                admin_connections: counters.admin_connections,
                active_leases: u64::try_from(status.active_leases).unwrap_or(u64::MAX),
                cross_domain_capability_denied_total: counters.cross_domain_capability_denied_total,
                peer_credential_refused_total: counters.peer_credential_refused_total,
                events_dropped_total: status.events_dropped,
                events_refused_total: status.events_refused,
            },
            pre_auth: status.pre_auth.map(|counts| PreAuthCounters {
                tracked_sources: Some(u64::try_from(counts.tracked_sources).unwrap_or(u64::MAX)),
                pending_total: Some(u64::try_from(counts.pending).unwrap_or(u64::MAX)),
            }),
            ingress: status.ingress.map(|counts| IngressCounters {
                direct_tracked_peers: Some(
                    u64::try_from(counts.direct_tracked_peers).unwrap_or(u64::MAX),
                ),
                broadcast_tracked_peers: Some(
                    u64::try_from(counts.broadcast_tracked_peers).unwrap_or(u64::MAX),
                ),
            }),
        }
    }
}

impl From<AdminStatusResult> for AdminStatus {
    /// The port's view as a client reads it back. The IPC counters are
    /// the server's and have no field in the neutral view but
    /// `active_leases`, which the server took from the port.
    fn from(result: AdminStatusResult) -> Self {
        Self {
            health: result.health,
            peer: result.peer,
            connectivity: result.connectivity,
            active_leases: usize::try_from(result.ipc.active_leases).unwrap_or(usize::MAX),
            events_dropped: result.ipc.events_dropped_total,
            events_refused: result.ipc.events_refused_total,
            // A block missing either count is not the funnel's view.
            pre_auth: result.pre_auth.and_then(|counters| {
                Some(PreAuthCounts {
                    tracked_sources: usize::try_from(counters.tracked_sources?)
                        .unwrap_or(usize::MAX),
                    pending: usize::try_from(counters.pending_total?).unwrap_or(usize::MAX),
                })
            }),
            // Likewise: a block missing either lane is not the limiters'.
            ingress: result.ingress.and_then(|counters| {
                Some(IngressCounts {
                    direct_tracked_peers: usize::try_from(counters.direct_tracked_peers?)
                        .unwrap_or(usize::MAX),
                    broadcast_tracked_peers: usize::try_from(counters.broadcast_tracked_peers?)
                        .unwrap_or(usize::MAX),
                })
            }),
        }
    }
}

/// What the IPC server counts itself.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServerCounters {
    /// Open data-socket connections.
    pub data_connections: u64,
    /// Open admin-socket connections.
    pub admin_connections: u64,
    /// Requests refused for naming the other domain's method.
    pub cross_domain_capability_denied_total: u64,
    /// Connections refused on their peer credential.
    pub peer_credential_refused_total: u64,
}

/// `admin-status.ipc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IpcCounters {
    /// Open data-socket connections.
    pub data_connections: u64,
    /// Open admin-socket connections.
    pub admin_connections: u64,
    /// Endpoints currently leased.
    pub active_leases: u64,
    /// Always emitted; absent on the wire reads as 0.
    #[serde(default)]
    pub cross_domain_capability_denied_total: u64,
    /// Always emitted; absent on the wire reads as 0.
    #[serde(default)]
    pub peer_credential_refused_total: u64,
    /// Copies the session queues dropped or could not queue, every
    /// session's since the daemon started: the port's count, emitted only
    /// when the port keeps one (`LOCAL-IPC.md` A 2026-11-04). Absent says
    /// "not counted", where a 0 says "none dropped", so a port that
    /// keeps none leaves it out. No minor: the member has been optional
    /// in `admin-status` 1.1.0 since before this build emitted it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "absent_or_count"
    )]
    pub events_dropped_total: Option<u64>,
    /// Inbound broadcasts the ingress gate refused, one per refused
    /// message, on any channel since the daemon started: the port's
    /// count, emitted only when the port keeps one. Absent says "not
    /// counted", where a 0 says "none refused". A signal: a republished
    /// refused copy counts again.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "absent_or_count"
    )]
    pub events_refused_total: Option<u64>,
}

/// `admin-status.pre_auth`: every field optional on the wire.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreAuthCounters {
    /// Sources with pending pre-authentication state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracked_sources: Option<u64>,
    /// Pre-authentication attempts pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_total: Option<u64>,
}

/// `admin-status.ingress`: every field optional on the wire.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IngressCounters {
    /// Peers the direct lane's ingress limiter tracks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_tracked_peers: Option<u64>,
    /// Peers the broadcast lane's ingress limiter tracks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broadcast_tracked_peers: Option<u64>,
}

/// `ipc/endpoint-list`: every configured endpoint and its runtime state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EndpointList {
    /// At most [`MAX_ENDPOINT_ROWS`], unique by id.
    pub endpoints: Vec<EndpointRow>,
}

impl EndpointList {
    /// The port's rows, each with the view's own `persisted`.
    ///
    /// # Errors
    /// [`TransportError::Internal`] for more rows than the contract
    /// carries, or a lease whose client kind is outside its bounds: a
    /// port that answered either broke its own contract, and the server
    /// answers that rather than truncating.
    pub fn from_views(views: Vec<EndpointAdminView>) -> Result<Self, TransportError> {
        if views.len() > MAX_ENDPOINT_ROWS {
            return Err(TransportError::Internal);
        }
        let endpoints = views
            .into_iter()
            .map(EndpointRow::from_view)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { endpoints })
    }
}

impl<'de> Deserialize<'de> for EndpointList {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            endpoints: Vec<EndpointRow>,
        }
        let wire = Wire::deserialize(d)?;
        let ids: Vec<_> = wire.endpoints.iter().map(|row| row.id.clone()).collect();
        bounded_unique(&ids, MAX_ENDPOINT_ROWS).map_err(serde::de::Error::custom)?;
        Ok(Self {
            endpoints: wire.endpoints,
        })
    }
}

/// One `endpoint-list` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointRow {
    /// The endpoint.
    #[serde(deserialize_with = "crate::derived::endpoint")]
    pub id: EndpointId,
    /// Whether it accepts traffic and may be claimed.
    pub enabled: bool,
    /// Whether it receives directed sends naming no endpoint.
    pub default: bool,
    /// Whether `enabled` and `default` survive a daemon restart: `true`
    /// for a runtime keeping the endpoint overlay (ADR-0028 A
    /// 2026-10-11).
    pub persisted: bool,
    /// Present while a connection holds the lease.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "absent_or_lease"
    )]
    pub lease: Option<LeaseRow>,
}

impl EndpointRow {
    /// `view` as its row ([`EndpointList::from_views`]).
    ///
    /// # Errors
    /// [`TransportError::Internal`] for a lease outside its bounds.
    pub(crate) fn from_view(view: EndpointAdminView) -> Result<Self, TransportError> {
        Ok(Self {
            id: view.endpoint,
            enabled: view.enabled,
            default: view.default,
            persisted: view.persisted,
            lease: view.lease.map(LeaseRow::try_from).transpose()?,
        })
    }
}

impl From<EndpointRow> for EndpointAdminView {
    /// A row as a client reads it back. The lease's `session_id` is
    /// binding-local and never on the wire, so it is `None` here
    /// (`LOCAL-CLIENT.md`, A 2026-09-30).
    fn from(row: EndpointRow) -> Self {
        let endpoint = row.id;
        Self {
            lease: row.lease.map(|lease| LeaseRecord {
                endpoint: endpoint.clone(),
                epoch: lease.epoch,
                client_kind: lease.client_kind,
                session_id: None,
            }),
            endpoint,
            enabled: row.enabled,
            default: row.default,
            persisted: row.persisted,
        }
    }
}

/// `ipc/trust-list` (2.1): one page of the data-plane allowlist, in the
/// ascending order of each peer's canonical string.
///
/// There is no default and no per-peer decision here: every peer not
/// listed is denied, which is the policy's shape rather than a setting
/// (ADR-0032).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrustList {
    /// This profile's identity, on the FIRST page only (absent on every
    /// later one, and when the policy has none bound).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_peer: Option<TransportIdentity>,
    /// At most [`MAX_TRUST_PAGE_ROWS`], strictly ascending by peer.
    pub allowed: Vec<TrustRow>,
    /// The last row's peer, when more remain: the next page's `after`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<TransportIdentity>,
    /// Which set an ENDPOINT answer is of (IPC 3.0, ADR-0067 decision 6);
    /// absent on the operator's list. An endpoint answer has no
    /// `local_peer`, which the decoder holds.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_set_kind"
    )]
    pub set: Option<EndpointSetKind>,
}

#[expect(
    clippy::ref_option,
    clippy::trivially_copy_pass_by_ref,
    reason = "the signature serde's serialize_with calls"
)]
fn serialize_set_kind<S: serde::Serializer>(
    kind: &Option<EndpointSetKind>,
    s: S,
) -> Result<S::Ok, S::Error> {
    // `skip_serializing_if` takes the `None` before this is called.
    match kind {
        Some(k) => s.serialize_str(k.as_str()),
        None => s.serialize_none(),
    }
}

impl TrustList {
    /// The page of `view` that follows `after` (the first when `None`),
    /// each persisted view row carrying its `source`.
    ///
    /// The cursor is a POSITION, not a row: `after` need not be listed,
    /// so a peer revoked between two reads still names the page after
    /// it. Read against the live policy, so pages are not a snapshot.
    /// `a_trust_list_pages_in_order_and_says_when_more_remain`,
    /// `a_trust_row_carries_its_source_when_persisted`.
    #[must_use]
    pub fn page(view: TrustAdminView, after: Option<&TransportIdentity>) -> Self {
        let mut page = Self::paged(view.allowed, after);
        if after.is_none() {
            page.local_peer = view.local_peer;
        }
        page
    }

    /// The page of one direction of an endpoint's effective set that
    /// follows `after`: the operator form's page, order and cursor, the
    /// answer's `set` saying which set it is and no `local_peer`
    /// (`admin.trust.list {endpoint, direction}`, `LOCAL-IPC.md` §The
    /// endpoint dimension). The one constructor of an endpoint answer, so
    /// `set` is on every one.
    /// `an_endpoint_page_says_its_set_and_has_no_local_peer`.
    #[must_use]
    pub fn page_endpoint(view: EndpointTrustView, after: Option<&TransportIdentity>) -> Self {
        let mut page = Self::paged(view.allowed, after);
        page.set = Some(view.set);
        page
    }

    fn paged(
        rows: Vec<interweave_local_client_api::TrustedPeer>,
        after: Option<&TransportIdentity>,
    ) -> Self {
        let mut rows = rows;
        rows.sort_by(|a, b| a.peer.cmp(&b.peer));
        rows.dedup_by(|a, b| a.peer == b.peer);
        let mut rest = rows
            .into_iter()
            .filter(|row| after.is_none_or(|after| row.peer > *after))
            .peekable();
        let allowed: Vec<TrustRow> = rest
            .by_ref()
            .take(MAX_TRUST_PAGE_ROWS)
            .map(|row| {
                let persisted = row.persisted;
                TrustRow {
                    peer: row.peer,
                    persisted,
                    source: persisted.then_some(row.source),
                }
            })
            .collect();
        let next = rest
            .peek()
            .is_some()
            .then(|| allowed.last().map(|row| row.peer.clone()))
            .flatten();
        Self {
            local_peer: None,
            allowed,
            next,
            set: None,
        }
    }
}

impl<'de> Deserialize<'de> for TrustList {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            #[serde(default, deserialize_with = "absent_or_identity")]
            local_peer: Option<TransportIdentity>,
            allowed: Vec<TrustRow>,
            #[serde(default, deserialize_with = "absent_or_identity")]
            next: Option<TransportIdentity>,
            #[serde(default, deserialize_with = "absent_or_set_kind")]
            set: Option<EndpointSetKind>,
        }
        let wire = Wire::deserialize(d)?;
        // The schema's `dependentSchemas` on `set`: an endpoint answer has
        // no `local_peer`, and an own set's rows are all administered and
        // persisted.
        if wire.set.is_some() && wire.local_peer.is_some() {
            return Err(serde::de::Error::custom(
                "an endpoint answer has no local_peer",
            ));
        }
        if wire.set == Some(EndpointSetKind::Own)
            && wire
                .allowed
                .iter()
                .any(|row| !row.persisted || row.source != Some(TrustSource::Administered))
        {
            return Err(serde::de::Error::custom(
                "an own set's rows are administered and persisted",
            ));
        }
        if wire.allowed.len() > MAX_TRUST_PAGE_ROWS {
            return Err(serde::de::Error::custom(format!(
                "at most {MAX_TRUST_PAGE_ROWS} rows, got {}",
                wire.allowed.len()
            )));
        }
        // Strictly ascending, which is unique too: the order is what makes
        // `next` a cursor a reader can trust.
        if wire
            .allowed
            .windows(2)
            .any(|pair| pair[0].peer >= pair[1].peer)
        {
            return Err(serde::de::Error::custom(
                "rows must be unique and in ascending peer order",
            ));
        }
        Ok(Self {
            local_peer: wire.local_peer,
            allowed: wire.allowed,
            next: wire.next,
            set: wire.set,
        })
    }
}

fn absent_or_set_kind<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<EndpointSetKind>, D::Error> {
    let name = String::deserialize(d)?;
    EndpointSetKind::ALL
        .into_iter()
        .find(|known| known.as_str() == name)
        .map(Some)
        .ok_or_else(|| serde::de::Error::custom("set is own, authored or inherited"))
}

/// `ipc/trust-seed-result` (IPC 3.0): what an `admin.trust.seed` did.
/// `outcome` is closed at the two values, and a refusal is an error, never
/// a third outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TrustSeedResult {
    outcome: &'static str,
    #[serde(skip)]
    seed: TrustSeed,
}

impl From<TrustSeed> for TrustSeedResult {
    fn from(seed: TrustSeed) -> Self {
        Self {
            outcome: seed.as_str(),
            seed,
        }
    }
}

impl From<TrustSeedResult> for TrustSeed {
    fn from(result: TrustSeedResult) -> Self {
        result.seed
    }
}

impl<'de> Deserialize<'de> for TrustSeedResult {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            outcome: String,
        }
        let wire = Wire::deserialize(d)?;
        [TrustSeed::Seeded, TrustSeed::Present]
            .into_iter()
            .find(|known| known.as_str() == wire.outcome)
            .map(Self::from)
            .ok_or_else(|| serde::de::Error::custom("an unknown outcome"))
    }
}

/// `ipc/peer-list` (2.2): one page of the dial gate's state per
/// allowlisted peer, in the ascending order of each peer's canonical
/// string (`CONNECTIVITY.md` §19). Never an address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PeerList {
    /// At most [`MAX_PEER_PAGE_ROWS`], strictly ascending by peer.
    pub peers: Vec<PeerRow>,
    /// The last row's peer, when more remain: the next page's `after`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<TransportIdentity>,
}

impl PeerList {
    /// The page of `rows` that follows `after` (the first when `None`):
    /// sorted and deduplicated by peer here, whatever order the port gave,
    /// so the cursor is a position as [`TrustList::page`]'s is.
    /// `a_peer_list_pages_in_order_and_says_when_more_remain`; at most
    /// [`MAX_PEER_PAGE_ROWS_SINCE_LAST_SEEN`] rows, every one carrying
    /// `last_seen_ms`.
    #[must_use]
    pub fn page(mut rows: Vec<PeerGateView>, after: Option<&TransportIdentity>) -> Self {
        rows.sort_by(|a, b| a.peer.cmp(&b.peer));
        rows.dedup_by(|a, b| a.peer == b.peer);
        let mut rest = rows
            .into_iter()
            .filter(|row| after.is_none_or(|after| &row.peer > after))
            .peekable();
        let peers: Vec<PeerRow> = rest
            .by_ref()
            .take(MAX_PEER_PAGE_ROWS_SINCE_LAST_SEEN)
            .map(PeerRow::from_view)
            .collect();
        let next = rest
            .peek()
            .is_some()
            .then(|| peers.last().map(|row| row.peer.clone()))
            .flatten();
        Self { peers, next }
    }
}

impl<'de> Deserialize<'de> for PeerList {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            peers: Vec<PeerRow>,
            #[serde(default, deserialize_with = "absent_or_identity")]
            next: Option<TransportIdentity>,
        }
        let wire = Wire::deserialize(d)?;
        if wire.peers.len() > MAX_PEER_PAGE_ROWS {
            return Err(serde::de::Error::custom(format!(
                "at most {MAX_PEER_PAGE_ROWS} rows, got {}",
                wire.peers.len()
            )));
        }
        if wire
            .peers
            .windows(2)
            .any(|pair| pair[0].peer >= pair[1].peer)
        {
            return Err(serde::de::Error::custom(
                "rows must be unique and in ascending peer order",
            ));
        }
        Ok(Self {
            peers: wire.peers,
            next: wire.next,
        })
    }
}

/// One `peer-list` row: what the dial gate holds against an allowlisted
/// peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerRow {
    /// The peer.
    pub peer: TransportIdentity,
    /// Whether any connection to it is open.
    pub connected: bool,
    /// The last moment a retained connection to it was open,
    /// milliseconds since the Unix epoch -- the reading's time while
    /// `connected`, else when the last one closed -- `Some(None)` (`null`)
    /// when none has been since the daemon started, and `None` (absent)
    /// when the port does not say.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "millis_or_null"
    )]
    pub last_seen_ms: Option<Option<u64>>,
    /// Dials to it are refused until then, milliseconds since the Unix
    /// epoch.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "absent_or_millis"
    )]
    pub backoff_until: Option<u64>,
    /// Every known address of it is quarantined until then, the earliest
    /// release; absent while any is dialable (`CONNECTIVITY.md` §19).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "absent_or_millis"
    )]
    pub quarantined_until: Option<u64>,
    /// The latest hold among its circuit routes, whose relay was not
    /// reached (ADR-0011 A 2026-10-21); absent with none held (peer-list
    /// 1.1.0).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "absent_or_millis"
    )]
    pub route_held_until: Option<u64>,
    /// How the last dial or connection ended; absent until the first.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "outcome")]
    pub last_outcome: Option<PeerOutcome>,
}

impl PeerRow {
    /// `view` as its row ([`PeerList::page`]).
    fn from_view(view: PeerGateView) -> Self {
        Self {
            peer: view.peer,
            connected: view.connected,
            last_seen_ms: match view.last_seen {
                PeerLastSeen::At(at) => Some(Some(at)),
                PeerLastSeen::Never => Some(None),
                // A port that does not say is answered as one: no key.
                PeerLastSeen::Unreported => None,
            },
            backoff_until: view.backoff_until,
            quarantined_until: view.quarantined_until,
            route_held_until: view.route_held_until,
            last_outcome: view.last_outcome,
        }
    }
}

/// A key that is present holds a value: an explicit `null` is refused,
/// as the schemas refuse it, where `Option`'s own reading would take it
/// for an absent key (`the_peers_address_shapes_hold_what_the_contract_says`).
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

fn absent_or_millis<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    u64::deserialize(d).map(Some)
}

/// A key that is present is a time or `null`, kept apart from an absent
/// key (`last_seen_ms`), which `Option`'s own reading would fold together.
#[expect(
    clippy::option_option,
    reason = "the wire's three states: a time, `null`, absent"
)]
fn millis_or_null<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<u64>>, D::Error> {
    Option::<u64>::deserialize(d).map(Some)
}

/// `last_outcome` on the wire: the neutral `PeerOutcome` by the names
/// `CONNECTIVITY.md` §19 gives it; an explicit `null` is refused, as the
/// schema does.
mod outcome {
    use interweave_local_client_api::PeerOutcome;
    use serde::{Deserialize, Deserializer, Serializer};

    const ALL: [PeerOutcome; 4] = [
        PeerOutcome::Connected,
        PeerOutcome::DialFailed,
        PeerOutcome::IdentityMismatch,
        PeerOutcome::Denied,
    ];

    #[expect(
        clippy::ref_option,
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's `with` hands the field by reference, whatever its size"
    )]
    pub(super) fn serialize<S: Serializer>(
        value: &Option<PeerOutcome>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(outcome) => s.serialize_str(outcome.as_str()),
            None => s.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<PeerOutcome>, D::Error> {
        let name = String::deserialize(d)?;
        ALL.into_iter()
            .find(|o| o.as_str() == name)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown last_outcome {name:?}")))
    }
}

/// One `trust-list` row: an allowed peer. A row the runtime does not
/// persist is `persisted` false with no `source`; a persisted row is
/// `persisted` true with its `source` -- the schema's if/then/else, held
/// here in both directions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrustRow {
    /// The allowed peer.
    pub peer: TransportIdentity,
    /// The row survives a restart: configured rows by `config.yaml`,
    /// administered rows by the trust overlay (ADR-0028 A 2026-10-07).
    pub persisted: bool,
    /// Where the row comes from, present exactly when `persisted` is.
    #[serde(skip_serializing_if = "Option::is_none", with = "source")]
    pub source: Option<TrustSource>,
}

impl<'de> Deserialize<'de> for TrustRow {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            peer: TransportIdentity,
            persisted: bool,
            #[serde(default, with = "source")]
            source: Option<TrustSource>,
        }
        let wire = Wire::deserialize(d)?;
        if wire.persisted != wire.source.is_some() {
            return Err(serde::de::Error::custom(
                "a persisted row names its source, and only a persisted row does",
            ));
        }
        Ok(Self {
            peer: wire.peer,
            persisted: wire.persisted,
            source: wire.source,
        })
    }
}

/// `source` on the wire: the neutral `TrustSource` by the schema's names;
/// an explicit `null` is refused.
mod source {
    use interweave_local_client_api::TrustSource;
    use serde::{Deserialize, Deserializer, Serializer};

    const ALL: [(TrustSource, &str); 2] = [
        (TrustSource::Configured, "configured"),
        (TrustSource::Administered, "administered"),
    ];

    #[expect(
        clippy::ref_option,
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's `with` hands the field by reference, whatever its size"
    )]
    pub(super) fn serialize<S: Serializer>(
        value: &Option<TrustSource>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match value.and_then(|v| ALL.into_iter().find(|(s, _)| *s == v)) {
            Some((_, name)) => s.serialize_str(name),
            None => s.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<TrustSource>, D::Error> {
        let name = String::deserialize(d)?;
        ALL.into_iter()
            .find(|(_, n)| *n == name)
            .map(|(s, _)| Some(s))
            .ok_or_else(|| serde::de::Error::custom(format!("unknown source {name:?}")))
    }
}

fn absent_or_identity<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<TransportIdentity>, D::Error> {
    TransportIdentity::deserialize(d).map(Some)
}

/// A row's live lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseRow {
    /// The lease's epoch.
    pub epoch: Generation,
    /// The label its holder claimed under.
    #[serde(deserialize_with = "client_kind")]
    pub client_kind: String,
}

impl TryFrom<LeaseRecord> for LeaseRow {
    type Error = TransportError;

    fn try_from(lease: LeaseRecord) -> Result<Self, TransportError> {
        let chars = lease.client_kind.chars().count();
        if chars == 0 || chars > MAX_CLIENT_KIND_CHARS {
            return Err(TransportError::Internal);
        }
        Ok(Self {
            epoch: lease.epoch,
            client_kind: lease.client_kind,
        })
    }
}

/// `ipc/session-losses-result` (2.11): the broadcasts this
/// connection's session has lost since it opened, two monotonic counts:
/// the overflow a lower bound on losses, the refusals a signal (the
/// neutral API's [`interweave_local_client_api::SessionLosses`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionLossesResult {
    /// Copies the session's queue dropped or could not queue.
    pub queue_overflow: u64,
    /// Copies the ingress gate refused on a channel the session held.
    pub ingress_refused: u64,
}

impl From<interweave_local_client_api::SessionLosses> for SessionLossesResult {
    fn from(losses: interweave_local_client_api::SessionLosses) -> Self {
        Self {
            queue_overflow: losses.queue_overflow,
            ingress_refused: losses.ingress_refused,
        }
    }
}

impl From<SessionLossesResult> for interweave_local_client_api::SessionLosses {
    fn from(result: SessionLossesResult) -> Self {
        Self {
            queue_overflow: result.queue_overflow,
            ingress_refused: result.ingress_refused,
        }
    }
}

/// `ipc/set-enabled-result`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetEnabledResult {
    /// The epoch disabling revoked, when a lease was live.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "absent_or_generation"
    )]
    pub revoked_epoch: Option<Generation>,
}

fn bounded_unique<T: Ord>(items: &[T], max: usize) -> Result<(), String> {
    if items.len() > max {
        return Err(format!("at most {max} entries, got {}", items.len()));
    }
    if items.iter().collect::<BTreeSet<_>>().len() != items.len() {
        return Err("entries must be unique".to_owned());
    }
    Ok(())
}

fn client_kind<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let kind = String::deserialize(d)?;
    if kind.is_empty() || kind.chars().count() > MAX_CLIENT_KIND_CHARS {
        return Err(serde::de::Error::custom(format!(
            "a client kind is 1..={MAX_CLIENT_KIND_CHARS} characters"
        )));
    }
    Ok(kind)
}

fn absent_or_count<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    u64::deserialize(d).map(Some)
}

fn absent_or_pre_auth<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<PreAuthCounters>, D::Error> {
    PreAuthCounters::deserialize(d).map(Some)
}

fn absent_or_ingress<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<IngressCounters>, D::Error> {
    IngressCounters::deserialize(d).map(Some)
}

fn absent_or_lease<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<LeaseRow>, D::Error> {
    LeaseRow::deserialize(d).map(Some)
}

fn absent_or_generation<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Generation>, D::Error> {
    Generation::deserialize(d).map(Some)
}

/// `ipc/peers-address-result`: what an `admin.peers.add_address` or
/// `admin.peers.remove_address` did. A refusal is an ANSWER naming its
/// rule, never an error (ADR-0062).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddressResult {
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    refused: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

impl From<AddressChange> for AddressResult {
    /// A `detail` past [`MAX_ADDRESS_DETAIL_CHARS`] is cut there: it is
    /// for a log, and the reason a client renders is `refused`.
    fn from(change: AddressChange) -> Self {
        let (outcome, refused, detail) = match change {
            AddressChange::Added => ("added", None, None),
            AddressChange::Removed => ("removed", None, None),
            AddressChange::Unchanged => ("unchanged", None, None),
            AddressChange::Refused { reason, detail } => (
                "refused",
                Some(reason.as_str()),
                Some(detail.chars().take(MAX_ADDRESS_DETAIL_CHARS).collect()),
            ),
        };
        Self {
            outcome,
            refused,
            detail,
        }
    }
}

impl From<AddressResult> for AddressChange {
    fn from(result: AddressResult) -> Self {
        match (result.outcome, result.refused) {
            ("added", _) => Self::Added,
            ("removed", _) => Self::Removed,
            ("refused", Some(reason)) => Self::Refused {
                reason: refusal_named(reason).unwrap_or(AddressRefusal::Grammar),
                detail: result.detail.unwrap_or_default(),
            },
            _ => Self::Unchanged,
        }
    }
}

fn refusal_named(name: &str) -> Option<AddressRefusal> {
    AddressRefusal::ALL
        .into_iter()
        .find(|reason| reason.as_str() == name)
}

impl<'de> Deserialize<'de> for AddressResult {
    /// The schema's closed sets, and `refused` present exactly when the
    /// outcome is `refused`.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            outcome: String,
            #[serde(default, deserialize_with = "present")]
            refused: Option<String>,
            #[serde(default, deserialize_with = "present")]
            detail: Option<String>,
        }
        let wire = Wire::deserialize(d)?;
        let outcome = ["added", "removed", "unchanged", "refused"]
            .into_iter()
            .find(|o| *o == wire.outcome)
            .ok_or_else(|| serde::de::Error::custom("an unknown outcome"))?;
        let refused = match (outcome, wire.refused.as_deref()) {
            ("refused", Some(name)) => Some(
                refusal_named(name)
                    .ok_or_else(|| serde::de::Error::custom("an unknown refusal"))?
                    .as_str(),
            ),
            ("refused", None) => return Err(serde::de::Error::custom("a refusal names its rule")),
            (_, Some(_)) => {
                return Err(serde::de::Error::custom("only a refusal names a rule"));
            }
            (_, None) => None,
        };
        if wire
            .detail
            .as_ref()
            .is_some_and(|detail| detail.chars().count() > MAX_ADDRESS_DETAIL_CHARS)
        {
            return Err(serde::de::Error::custom(
                "a detail is at most 512 characters",
            ));
        }
        Ok(Self {
            outcome,
            refused,
            detail: wire.detail,
        })
    }
}

/// `ipc/trust-set-result`: what an `admin.trust.set` did. A refusal is
/// an ANSWER naming its rule, never an error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrustSetResult {
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    refused: Option<&'static str>,
    /// What `refused` names, kept typed so the mirror reads it back
    /// without matching the string again.
    #[serde(skip)]
    reason: Option<TrustRefusal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

impl From<TrustChange> for TrustSetResult {
    fn from(change: TrustChange) -> Self {
        match change {
            TrustChange::Set => Self {
                outcome: "set",
                refused: None,
                reason: None,
                detail: None,
            },
            TrustChange::Refused(reason) => Self {
                outcome: "refused",
                refused: Some(reason.as_str()),
                reason: Some(reason),
                detail: None,
            },
        }
    }
}

impl From<TrustSetResult> for TrustChange {
    fn from(result: TrustSetResult) -> Self {
        result.reason.map_or(Self::Set, Self::Refused)
    }
}

impl<'de> Deserialize<'de> for TrustSetResult {
    /// The schema's closed sets, and `refused` present exactly when the
    /// outcome is `refused`.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            outcome: String,
            #[serde(default, deserialize_with = "present")]
            refused: Option<String>,
            #[serde(default, deserialize_with = "present")]
            detail: Option<String>,
        }
        let bad = |what: &str| serde::de::Error::custom(what);
        let wire = Wire::deserialize(d)?;
        if wire
            .detail
            .as_ref()
            .is_some_and(|detail| detail.chars().count() > MAX_ADDRESS_DETAIL_CHARS)
        {
            return Err(bad("a detail is at most 512 characters"));
        }
        let reason = match (wire.outcome.as_str(), wire.refused.as_deref()) {
            ("set", None) => None,
            ("refused", Some(name)) => Some(
                TrustRefusal::ALL
                    .into_iter()
                    .find(|known| known.as_str() == name)
                    .ok_or_else(|| bad("an unknown refusal"))?,
            ),
            ("refused", None) => return Err(bad("a refusal names its rule")),
            ("set", Some(_)) => return Err(bad("only a refusal names a rule")),
            _ => return Err(bad("an unknown outcome")),
        };
        Ok(Self {
            outcome: if reason.is_some() { "refused" } else { "set" },
            refused: reason.map(TrustRefusal::as_str),
            reason,
            detail: wire.detail,
        })
    }
}

/// `ipc/peers-addresses-result`: what an `admin.peers.add_addresses` did
/// (ADR-0062 decision 4). Every line in force, one row per line in the
/// order given; or a refusal naming its rule and -- but for
/// `provider_disabled`, which refuses the list whole -- its line, nothing
/// written. A refusal is an ANSWER, never an error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddressesResult {
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines: Option<Vec<LineRow>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refused: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

/// One `peers-addresses-result` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineRow {
    /// What the add did with the line.
    pub outcome: LineOutcome,
}

/// A row's closed `outcome` set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineOutcome {
    /// Added and in force.
    Added,
    /// Already present, or repeated earlier in the list.
    Unchanged,
}

impl AddressesResult {
    /// The port's answer to a list of `lines` addresses.
    ///
    /// # Errors
    /// [`TransportError::Internal`] for an answer the contract cannot
    /// carry -- a row count other than `lines`, so rows a client cannot
    /// match to its lines, more than [`MAX_ADDRESSES_PER_ADD`], a refusal
    /// `authored`, a line named where the rule names none or none where
    /// it names one: a port that answered that broke its own rule, and
    /// the server answers so rather than mending it.
    pub fn from_view(change: AddressesChange, lines: usize) -> Result<Self, TransportError> {
        Ok(match change {
            AddressesChange::Added(rows) => {
                if rows.len() != lines || rows.is_empty() || rows.len() > MAX_ADDRESSES_PER_ADD {
                    return Err(TransportError::Internal);
                }
                Self {
                    outcome: "added",
                    lines: Some(
                        rows.into_iter()
                            .map(|line| LineRow {
                                outcome: match line {
                                    LineChange::Added => LineOutcome::Added,
                                    LineChange::Unchanged => LineOutcome::Unchanged,
                                },
                            })
                            .collect(),
                    ),
                    refused: None,
                    index: None,
                    detail: None,
                }
            }
            AddressesChange::Refused {
                reason,
                line,
                detail,
            } => {
                let whole = reason == AddressRefusal::ProviderDisabled;
                if reason == AddressRefusal::Authored
                    || whole != line.is_none()
                    || line.is_some_and(|line| line >= lines.min(MAX_ADDRESSES_PER_ADD))
                {
                    return Err(TransportError::Internal);
                }
                Self {
                    outcome: "refused",
                    lines: None,
                    refused: Some(reason.as_str()),
                    index: line,
                    // Cut, not refused: it is for a log, and the reason a
                    // client renders is `refused`.
                    detail: Some(detail.chars().take(MAX_ADDRESS_DETAIL_CHARS).collect()),
                }
            }
        })
    }
}

impl From<AddressesResult> for AddressesChange {
    fn from(result: AddressesResult) -> Self {
        match (result.lines, result.refused) {
            (Some(lines), _) => Self::Added(
                lines
                    .into_iter()
                    .map(|row| match row.outcome {
                        LineOutcome::Added => LineChange::Added,
                        LineOutcome::Unchanged => LineChange::Unchanged,
                    })
                    .collect(),
            ),
            (None, refused) => Self::Refused {
                reason: refused
                    .and_then(refusal_named)
                    .unwrap_or(AddressRefusal::Grammar),
                line: result.index,
                detail: result.detail.unwrap_or_default(),
            },
        }
    }
}

impl<'de> Deserialize<'de> for AddressesResult {
    /// The schema's closed sets and its two halves never mixed: `added`
    /// with 1..=16 rows and nothing of a refusal; `refused` with its rule,
    /// never `authored`, and `index` (0..=15) present exactly when the
    /// rule is not `provider_disabled`.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            outcome: String,
            #[serde(default, deserialize_with = "present")]
            lines: Option<Vec<LineRow>>,
            #[serde(default, deserialize_with = "present")]
            refused: Option<String>,
            #[serde(default, deserialize_with = "present")]
            index: Option<usize>,
            #[serde(default, deserialize_with = "present")]
            detail: Option<String>,
        }
        let bad = |what: &str| serde::de::Error::custom(what);
        let wire = Wire::deserialize(d)?;
        if wire
            .detail
            .as_ref()
            .is_some_and(|detail| detail.chars().count() > MAX_ADDRESS_DETAIL_CHARS)
        {
            return Err(bad("a detail is at most 512 characters"));
        }
        match (wire.outcome.as_str(), wire.lines, wire.refused) {
            ("added", Some(lines), None) if wire.index.is_none() => {
                if lines.is_empty() || lines.len() > MAX_ADDRESSES_PER_ADD {
                    return Err(bad("an add answers 1..=16 rows"));
                }
                Ok(Self {
                    outcome: "added",
                    lines: Some(lines),
                    refused: None,
                    index: None,
                    detail: wire.detail,
                })
            }
            ("refused", None, Some(name)) => {
                let reason = refusal_named(&name)
                    .filter(|reason| *reason != AddressRefusal::Authored)
                    .ok_or_else(|| bad("an unknown refusal"))?;
                let whole = reason == AddressRefusal::ProviderDisabled;
                if whole != wire.index.is_none()
                    || wire.index.is_some_and(|i| i >= MAX_ADDRESSES_PER_ADD)
                {
                    return Err(bad(
                        "a refusal names its line unless it refuses the list whole",
                    ));
                }
                Ok(Self {
                    outcome: "refused",
                    lines: None,
                    refused: Some(reason.as_str()),
                    index: wire.index,
                    detail: wire.detail,
                })
            }
            _ => Err(bad("added carries its rows, refused its rule, never both")),
        }
    }
}

/// `ipc/peers-own-addresses`: this node's own addresses for the show
/// side (ADR-0062 decision 4), one ordered list the transport selected
/// and floored, which a client renders as it comes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnAddressList {
    /// This node's `PeerId`.
    pub peer: TransportIdentity,
    /// At most [`MAX_OWN_ADDRESSES`], in the order the transport chose.
    pub addresses: Vec<OwnAddressRow>,
    /// Whether rows past the bound were dropped.
    pub truncated: bool,
}

/// One `peers-own-addresses` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnAddressRow {
    /// A `multiaddr-with-peer-id` an add takes.
    #[serde(deserialize_with = "crate::request::bounded_address")]
    pub address: String,
    /// What it is.
    pub kind: OwnKind,
}

/// `peers-own-addresses`' closed `kind` set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnKind {
    /// Verified reachable by AutoNAT.
    ConfirmedExternal,
    /// A circuit route through a held reservation.
    RelayCircuit,
    /// A bound listener.
    Listen,
}

impl From<OwnAddressKind> for OwnKind {
    fn from(kind: OwnAddressKind) -> Self {
        match kind {
            OwnAddressKind::ConfirmedExternal => Self::ConfirmedExternal,
            OwnAddressKind::RelayCircuit => Self::RelayCircuit,
            OwnAddressKind::Listen => Self::Listen,
        }
    }
}

impl From<OwnKind> for OwnAddressKind {
    fn from(kind: OwnKind) -> Self {
        match kind {
            OwnKind::ConfirmedExternal => Self::ConfirmedExternal,
            OwnKind::RelayCircuit => Self::RelayCircuit,
            OwnKind::Listen => Self::Listen,
        }
    }
}

impl OwnAddressList {
    /// The port's answer.
    ///
    /// # Errors
    /// [`TransportError::Internal`] for more rows than the contract
    /// carries: a port that answered that broke its own bound, and the
    /// server answers so rather than truncating.
    pub fn from_view(own: OwnAddresses) -> Result<Self, TransportError> {
        if own.addresses.len() > MAX_OWN_ADDRESSES {
            return Err(TransportError::Internal);
        }
        Ok(Self {
            peer: own.peer,
            addresses: own
                .addresses
                .into_iter()
                .map(|row| OwnAddressRow {
                    address: row.address,
                    kind: row.kind.into(),
                })
                .collect(),
            truncated: own.truncated,
        })
    }
}

impl From<OwnAddressList> for OwnAddresses {
    fn from(list: OwnAddressList) -> Self {
        Self {
            peer: list.peer,
            addresses: list
                .addresses
                .into_iter()
                .map(|row| OwnAddress {
                    address: row.address,
                    kind: row.kind.into(),
                })
                .collect(),
            truncated: list.truncated,
        }
    }
}

impl<'de> Deserialize<'de> for OwnAddressList {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            peer: TransportIdentity,
            addresses: Vec<OwnAddressRow>,
            truncated: bool,
        }
        let wire = Wire::deserialize(d)?;
        if wire.addresses.len() > MAX_OWN_ADDRESSES {
            return Err(serde::de::Error::custom(format!(
                "at most {MAX_OWN_ADDRESSES} addresses"
            )));
        }
        Ok(Self {
            peer: wire.peer,
            addresses: wire.addresses,
            truncated: wire.truncated,
        })
    }
}

/// `ipc/peers-address-list`: every bootstrap address, authored first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddressList {
    /// At most [`MAX_BOOTSTRAP_ADDRESS_ROWS`].
    pub addresses: Vec<AddressRow>,
}

impl AddressList {
    /// The port's rows.
    ///
    /// # Errors
    /// [`TransportError::Internal`] for more rows than the contract
    /// carries: a port that answered that broke its own bound, and the
    /// server answers so rather than truncating.
    pub fn from_views(views: Vec<BootstrapAddress>) -> Result<Self, TransportError> {
        if views.len() > MAX_BOOTSTRAP_ADDRESS_ROWS {
            return Err(TransportError::Internal);
        }
        Ok(Self {
            addresses: views.into_iter().map(AddressRow::from).collect(),
        })
    }
}

impl<'de> Deserialize<'de> for AddressList {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            addresses: Vec<AddressRow>,
        }
        let wire = Wire::deserialize(d)?;
        if wire.addresses.len() > MAX_BOOTSTRAP_ADDRESS_ROWS {
            return Err(serde::de::Error::custom(format!(
                "at most {MAX_BOOTSTRAP_ADDRESS_ROWS} addresses"
            )));
        }
        Ok(Self {
            addresses: wire.addresses,
        })
    }
}

/// One `peers-address-list` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddressRow {
    /// The address, as the provider holds it.
    pub address: String,
    /// `authored` or `added`.
    pub source: RowSource,
    /// Whether the provider emits it.
    pub dialled: bool,
    /// Why not, present exactly when `dialled` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_dialled: Option<NotDialled>,
}

/// Where a row's address comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowSource {
    /// `config.yaml`'s.
    Authored,
    /// The overlay's.
    Added,
}

/// Why a row's address is not dialled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotDialled {
    /// The profile authors the static-bootstrap provider `enabled: false`.
    ProviderDisabled,
}

impl<'de> Deserialize<'de> for AddressRow {
    /// The schema's row, `not_dialled` present exactly when `dialled` is
    /// false (`the_peers_address_shapes_hold_what_the_contract_says`).
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            #[serde(deserialize_with = "crate::request::bounded_address")]
            address: String,
            source: RowSource,
            dialled: bool,
            #[serde(default, deserialize_with = "present")]
            not_dialled: Option<NotDialled>,
        }
        let wire = Wire::deserialize(d)?;
        if wire.dialled == wire.not_dialled.is_some() {
            return Err(serde::de::Error::custom(
                "not_dialled is present exactly when dialled is false",
            ));
        }
        Ok(Self {
            address: wire.address,
            source: wire.source,
            dialled: wire.dialled,
            not_dialled: wire.not_dialled,
        })
    }
}

impl From<BootstrapAddress> for AddressRow {
    fn from(view: BootstrapAddress) -> Self {
        Self {
            address: view.address,
            source: match view.source {
                AddressSource::Authored => RowSource::Authored,
                AddressSource::Added => RowSource::Added,
            },
            dialled: view.dialled,
            not_dialled: (!view.dialled).then_some(NotDialled::ProviderDisabled),
        }
    }
}

impl From<AddressRow> for BootstrapAddress {
    fn from(row: AddressRow) -> Self {
        Self {
            address: row.address,
            source: match row.source {
                RowSource::Authored => AddressSource::Authored,
                RowSource::Added => AddressSource::Added,
            },
            dialled: row.dialled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ep(name: &str) -> EndpointId {
        interweave_test_support::endpoint_keys::endpoint_id(name)
    }

    fn epoch() -> Generation {
        Generation::parse("AAAAAAAAAAAAAAAA").expect("generation")
    }

    #[test]
    fn a_row_says_not_persisted_and_carries_the_lease() {
        let view = EndpointAdminView {
            endpoint: ep("human"),
            enabled: true,
            default: true,
            persisted: false,
            lease: Some(LeaseRecord {
                endpoint: ep("human"),
                epoch: epoch(),
                client_kind: "human-client".into(),
                session_id: Some("s1".into()),
            }),
        };
        let list = EndpointList::from_views(vec![view]).expect("one row");
        assert_eq!(
            serde_json::to_value(&list).expect("ser"),
            json!({"endpoints": [{
                "id": ep("human").as_str(), "enabled": true, "default": true, "persisted": false,
                "lease": {"epoch": "AAAAAAAAAAAAAAAA", "client_kind": "human-client"}
            }]})
        );
        let back: EndpointList =
            serde_json::from_value(serde_json::to_value(&list).expect("ser")).expect("de");
        assert_eq!(back, list);
    }

    /// The row says what the view says: `persisted` as the port has it,
    /// on every 3.0 connection, and it reads back as the view the client
    /// is owed.
    #[test]
    fn an_endpoint_row_says_what_the_view_persists() {
        let view = |persisted| EndpointAdminView {
            endpoint: ep("human"),
            enabled: false,
            default: false,
            persisted,
            lease: None,
        };
        for persisted in [true, false] {
            let list = EndpointList::from_views(vec![view(persisted)]).expect("a row");
            assert_eq!(
                serde_json::to_value(&list).expect("ser"),
                json!({"endpoints": [{
                    "id": ep("human").as_str(), "enabled": false, "default": false,
                    "persisted": persisted
                }]}),
                "persisted={persisted}"
            );
            let back = EndpointAdminView::from(list.endpoints[0].clone());
            assert_eq!(back.persisted, persisted);
        }
    }

    /// A client reads back what the server built from the port, less the
    /// binding-local session id, which the wire never carries.
    #[test]
    fn a_row_reads_back_as_the_view_without_its_session_id() {
        let lease = LeaseRecord {
            endpoint: ep("human"),
            epoch: epoch(),
            client_kind: "human-client".into(),
            session_id: Some("s1".into()),
        };
        let view = EndpointAdminView {
            endpoint: ep("human"),
            enabled: true,
            default: false,
            persisted: false,
            lease: Some(lease.clone()),
        };
        let row = EndpointRow::from_view(view.clone()).expect("a row");
        let back = EndpointAdminView::from(row);
        assert_eq!(
            back,
            EndpointAdminView {
                lease: Some(LeaseRecord {
                    session_id: None,
                    ..lease
                }),
                ..view
            }
        );
    }

    #[test]
    fn a_directory_and_a_status_read_back_as_the_port_gave_them() {
        let directory = EndpointDirectoryV1 {
            generated_at_ms: 5,
            ttl_ms: 60_000,
            endpoints: vec![ep("human"), ep("bot")],
        };
        assert_eq!(
            EndpointDirectoryV1::from(DirectoryResult::from(directory.clone())),
            directory
        );
        let status = AdminStatus {
            health: Health::Degraded,
            peer: TransportIdentity::parse("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN")
                .expect("peer"),
            connectivity: interweave_transport_api::ConnectivitySummary {
                direct_inbound: interweave_transport_api::DirectInboundState::VerifiedPublic,
                relay_inbound: interweave_transport_api::PathReadiness::Ready,
                active_relay_reservations: 1,
                target_relay_reservations: 2,
                active_relayed_peer_paths: 3,
                hole_punch_inflight: 0,
                preferred_path_policy: interweave_transport_api::PreferredPathPolicy::DirectFirst,
                updated_at: 1_700_000_000_000,
            },
            active_leases: 3,
            pre_auth: Some(interweave_local_client_api::PreAuthCounts {
                tracked_sources: 4,
                pending: 2,
            }),
            ingress: Some(IngressCounts {
                direct_tracked_peers: 5,
                broadcast_tracked_peers: 6,
            }),
            events_dropped: Some(7),
            events_refused: Some(8),
        };
        let result = AdminStatusResult::new(status.clone(), ServerCounters::default());
        let wire = serde_json::to_value(&result).expect("ser");
        assert_eq!(
            wire["pre_auth"],
            serde_json::json!({"tracked_sources": 4, "pending_total": 2}),
            "the pre-auth counts, and nothing peer-keyed"
        );
        assert_eq!(
            wire["ingress"],
            serde_json::json!({"direct_tracked_peers": 5, "broadcast_tracked_peers": 6}),
            "each lane under its own name"
        );
        assert_eq!(AdminStatus::from(result), status);
        // A block missing a lane is not the limiters' view, and reads as none.
        let mut half = wire.clone();
        half["ingress"] = serde_json::json!({"direct_tracked_peers": 5});
        let half: AdminStatusResult = serde_json::from_value(half).expect("de");
        assert_eq!(AdminStatus::from(half).ingress, None);
        // 1.1.0 dropped pre_auth.tracked_peers: a frame naming it is refused.
        let mut stale = wire.clone();
        stale["pre_auth"]["tracked_peers"] = serde_json::json!(1);
        assert!(serde_json::from_value::<AdminStatusResult>(stale).is_err());
        let none = AdminStatus {
            pre_auth: None,
            ingress: None,
            events_dropped: None,
            events_refused: None,
            ..status
        };
        let wire = serde_json::to_value(AdminStatusResult::new(
            none.clone(),
            ServerCounters::default(),
        ))
        .expect("ser");
        assert!(
            wire.get("pre_auth").is_none() && wire.get("ingress").is_none(),
            "no funnel or limiter, no block: {wire}"
        );
        assert_eq!(
            AdminStatus::from(AdminStatusResult::new(
                none.clone(),
                ServerCounters::default(),
            )),
            none
        );
    }

    /// The row this crate SENDS is bounded in characters too, so a session
    /// the data side admitted with a 64-character, 128-byte kind lists.
    #[test]
    fn a_lease_row_bounds_its_client_kind_in_characters() {
        let view = |kind: String| EndpointAdminView {
            endpoint: ep("human"),
            enabled: true,
            default: false,
            persisted: false,
            lease: Some(LeaseRecord {
                endpoint: ep("human"),
                epoch: epoch(),
                client_kind: kind,
                session_id: Some("s".into()),
            }),
        };
        assert!(EndpointList::from_views(vec![view("é".repeat(MAX_CLIENT_KIND_CHARS))]).is_ok());
        assert_eq!(
            EndpointList::from_views(vec![view("é".repeat(MAX_CLIENT_KIND_CHARS + 1))]),
            Err(TransportError::Internal)
        );
        // And the lower bound: an empty kind is not a label (`minLength: 1`).
        assert_eq!(
            EndpointList::from_views(vec![view(String::new())]),
            Err(TransportError::Internal)
        );
    }

    #[test]
    fn a_list_past_its_bound_or_repeating_an_id_is_refused_both_ways() {
        let view = |i: usize| EndpointAdminView {
            endpoint: ep(&format!("e{i}")),
            enabled: true,
            default: false,
            persisted: false,
            lease: None,
        };
        assert!(EndpointList::from_views((0..MAX_ENDPOINT_ROWS).map(view).collect()).is_ok());
        assert_eq!(
            EndpointList::from_views((0..=MAX_ENDPOINT_ROWS).map(view).collect()),
            Err(TransportError::Internal)
        );
        let row =
            json!({"id": ep("a").as_str(), "enabled": true, "default": false, "persisted": false});
        assert!(
            serde_json::from_value::<EndpointList>(json!({"endpoints": [row.clone(), row]}))
                .is_err()
        );
        let not_a_bool = json!({"endpoints": [
            {"id": ep("a").as_str(), "enabled": true, "default": false, "persisted": "yes"}
        ]});
        assert!(serde_json::from_value::<EndpointList>(not_a_bool).is_err());
    }

    #[test]
    fn a_directory_past_32_or_repeating_is_refused() {
        let ids = |n: usize| {
            (0..n)
                .map(|i| ep(&format!("e{i}")).as_str().to_owned())
                .collect::<Vec<_>>()
        };
        let doc = |endpoints: Vec<String>| json!({"endpoints": endpoints, "ttl_ms": 4_294_967_295_u64, "generated_at_ms": 0});
        assert!(serde_json::from_value::<DirectoryResult>(doc(ids(32))).is_ok());
        assert!(serde_json::from_value::<DirectoryResult>(doc(ids(33))).is_err());
        assert!(
            serde_json::from_value::<DirectoryResult>(doc(vec![
                ep("a").as_str().into(),
                ep("a").as_str().into()
            ]))
            .is_err()
        );
    }

    /// A counter nothing keeps is absent, not 0.
    #[test]
    fn the_status_names_no_dropped_count_it_does_not_keep() {
        use interweave_transport_api::{
            DirectInboundState, PathReadiness, PreferredPathPolicy, TransportIdentity,
        };
        let status = AdminStatus {
            health: Health::Healthy,
            peer: TransportIdentity::parse("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN")
                .expect("peer"),
            connectivity: ConnectivitySummary {
                direct_inbound: DirectInboundState::Unknown,
                relay_inbound: PathReadiness::Unavailable,
                active_relay_reservations: 0,
                target_relay_reservations: 0,
                active_relayed_peer_paths: 0,
                hole_punch_inflight: 0,
                preferred_path_policy: PreferredPathPolicy::DirectFirst,
                updated_at: 0,
            },
            active_leases: 0,
            pre_auth: None,
            ingress: None,
            events_dropped: None,
            events_refused: None,
        };
        let json = |status: &AdminStatus| {
            serde_json::to_value(AdminStatusResult::new(
                status.clone(),
                ServerCounters::default(),
            ))
            .expect("ser")
        };
        let unkept = json(&status);
        assert!(
            unkept["ipc"].get("events_dropped_total").is_none(),
            "{unkept}"
        );
        assert!(
            unkept["ipc"].get("peer_credential_refused_total").is_some(),
            "the kept ones are"
        );
        // A port that keeps the count: a 0 is "none dropped" and emitted,
        // and a count reads back as the port gave it.
        for kept in [0, 3] {
            let status = AdminStatus {
                events_dropped: Some(kept),
                ..status.clone()
            };
            let wire = json(&status);
            assert_eq!(wire["ipc"]["events_dropped_total"], kept, "{wire}");
            let back: AdminStatusResult = serde_json::from_value(wire).expect("de");
            assert_eq!(AdminStatus::from(back), status);
        }
    }

    /// `ipc.events_refused_total` is served whenever the port keeps the
    /// count (`ipc/admin-status`'s 2.13 block is 3.0's): absent is "not
    /// counted" and 0 is "none refused", as for `events_dropped_total`,
    /// and each reads back as the port gave it.
    #[test]
    fn the_refused_total_is_served_when_kept() {
        use interweave_transport_api::{
            DirectInboundState, PathReadiness, PreferredPathPolicy, TransportIdentity,
        };
        let unkept = AdminStatus {
            health: Health::Healthy,
            peer: TransportIdentity::parse("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN")
                .expect("peer"),
            connectivity: ConnectivitySummary {
                direct_inbound: DirectInboundState::Unknown,
                relay_inbound: PathReadiness::Unavailable,
                active_relay_reservations: 0,
                target_relay_reservations: 0,
                active_relayed_peer_paths: 0,
                hole_punch_inflight: 0,
                preferred_path_policy: PreferredPathPolicy::DirectFirst,
                updated_at: 0,
            },
            active_leases: 0,
            pre_auth: None,
            ingress: None,
            events_dropped: Some(2),
            events_refused: None,
        };
        let wire = |status: &AdminStatus| {
            serde_json::to_value(AdminStatusResult::new(
                status.clone(),
                ServerCounters::default(),
            ))
            .expect("ser")
        };
        for kept in [0, 153] {
            let status = AdminStatus {
                events_refused: Some(kept),
                ..unkept.clone()
            };
            let wire = wire(&status);
            assert_eq!(wire["ipc"]["events_refused_total"], kept, "{wire}");
            assert_eq!(wire["ipc"]["events_dropped_total"], 2, "its sibling apart");
            let back: AdminStatusResult = serde_json::from_value(wire).expect("de");
            assert_eq!(AdminStatus::from(back), status);
        }
        let none = wire(&unkept);
        assert!(
            none["ipc"].get("events_refused_total").is_none(),
            "a port that keeps no count names none: {none}"
        );
        let back: AdminStatusResult = serde_json::from_value(none).expect("de");
        assert_eq!(AdminStatus::from(back).events_refused, None, "never 0");
    }

    #[test]
    fn set_enabled_and_empty_results_have_their_shapes() {
        assert_eq!(
            serde_json::to_value(EmptyResult {}).expect("ser"),
            json!({})
        );
        assert!(serde_json::from_value::<EmptyResult>(json!({"x": 1})).is_err());
        assert_eq!(
            serde_json::to_value(SetEnabledResult {
                revoked_epoch: None
            })
            .expect("ser"),
            json!({})
        );
        assert!(
            serde_json::from_value::<SetEnabledResult>(json!({"revoked_epoch": null})).is_err()
        );
    }

    fn synthetic_peer(i: usize) -> TransportIdentity {
        let tail = format!("{i:044}").replace('0', "a");
        TransportIdentity::parse(format!("Qm{}", &tail[..44])).expect("peer")
    }

    fn gate_row(i: usize) -> PeerGateView {
        PeerGateView {
            peer: synthetic_peer(i),
            connected: i.is_multiple_of(2),
            last_seen: if i.is_multiple_of(11) {
                PeerLastSeen::Never
            } else {
                PeerLastSeen::At(1_791_329_940_227)
            },
            backoff_until: i.is_multiple_of(3).then_some(1_791_329_950_227),
            quarantined_until: None,
            route_held_until: i.is_multiple_of(7).then_some(1_791_329_960_227),
            last_outcome: (!i.is_multiple_of(5)).then_some(PeerOutcome::Denied),
        }
    }

    /// The bound is what the body holds: a full page of the largest rows
    /// the schema admits -- every optional field, u64-maximum times, the
    /// longest outcome, an Ed25519 `PeerId`'s 52 characters (the longest a
    /// profile's key gives; a 46-character `Qm` id understates each row
    /// by six bytes) -- and a `next` serializes within `MAX_BODY_BYTES`
    /// with room for the response frame around it. Serializing does not
    /// ask the rows to differ, so one valid id fills every row.
    #[test]
    fn a_full_page_of_the_largest_rows_fits_the_body() {
        let longest = TransportIdentity::parse(
            "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN".to_owned(),
        )
        .expect("an Ed25519 PeerId");
        assert_eq!(longest.as_str().len(), 52);
        let page = |rows: usize, last_seen_ms: Option<Option<u64>>| {
            let page = PeerList {
                peers: (0..rows)
                    .map(|_| PeerRow {
                        peer: longest.clone(),
                        connected: false,
                        last_seen_ms,
                        backoff_until: Some(u64::MAX),
                        quarantined_until: Some(u64::MAX),
                        route_held_until: Some(u64::MAX),
                        last_outcome: Some(PeerOutcome::IdentityMismatch),
                    })
                    .collect(),
                next: Some(longest.clone()),
            };
            serde_json::to_vec(&page).expect("serializes").len()
        };
        // The 1.1 row, served below 2.12, 512 a page.
        let bytes = page(MAX_PEER_PAGE_ROWS, None);
        assert!(
            bytes + 1_024 <= crate::framing::MAX_BODY_BYTES,
            "{bytes} bytes for {MAX_PEER_PAGE_ROWS} rows"
        );
        // And twice the bound does not: the reason the bound is 512.
        assert!(2 * bytes > crate::framing::MAX_BODY_BYTES);
        // The 1.2 row, served from 2.12, fits at its own bound -- and
        // would not at 512: the reason a 2.12 page is smaller.
        let largest = Some(Some(u64::MAX));
        let bytes = page(MAX_PEER_PAGE_ROWS_SINCE_LAST_SEEN, largest);
        assert!(
            bytes + 1_024 <= crate::framing::MAX_BODY_BYTES,
            "{bytes} bytes for {MAX_PEER_PAGE_ROWS_SINCE_LAST_SEEN} rows"
        );
        assert!(page(MAX_PEER_PAGE_ROWS, largest) > crate::framing::MAX_BODY_BYTES);
    }

    /// `admin.peers.list_addresses` answers in one body, unpaged: its 64
    /// rows at the schema's longest address -- 1024 characters, past the
    /// longest entry the configuration's rule admits -- each with
    /// `not_dialled`, fit with room for the frame. A row is ASCII, as the
    /// grammar admits nothing else, so a character is a byte.
    #[test]
    fn a_full_address_list_of_the_longest_rows_fits_the_body() {
        let row = AddressRow {
            address: "a".repeat(crate::MAX_BOOTSTRAP_ADDRESS_CHARS),
            source: RowSource::Authored,
            dialled: false,
            not_dialled: Some(NotDialled::ProviderDisabled),
        };
        let list = AddressList {
            addresses: vec![row; MAX_BOOTSTRAP_ADDRESS_ROWS],
        };
        let bytes = serde_json::to_vec(&list).expect("serializes").len();
        assert!(
            bytes + 1_024 <= crate::framing::MAX_BODY_BYTES,
            "{bytes} bytes for {MAX_BOOTSTRAP_ADDRESS_ROWS} rows"
        );
    }

    /// The own-address list at its bound -- 64 rows of the longest
    /// address the wire admits, the longest kind name each -- fits the
    /// body with room for the frame, so one answer and no paging.
    #[test]
    fn a_full_own_address_list_of_the_longest_rows_fits_the_body() {
        let list = OwnAddressList {
            peer: synthetic_peer(1),
            addresses: vec![
                OwnAddressRow {
                    address: "a".repeat(crate::MAX_BOOTSTRAP_ADDRESS_CHARS),
                    kind: OwnKind::ConfirmedExternal,
                };
                MAX_OWN_ADDRESSES
            ],
            truncated: true,
        };
        let bytes = serde_json::to_vec(&list).expect("serializes").len();
        assert!(
            bytes + 1_024 <= crate::framing::MAX_BODY_BYTES,
            "{bytes} bytes for {MAX_OWN_ADDRESSES} rows"
        );
    }

    #[test]
    fn a_peer_list_pages_in_order_and_says_when_more_remain() {
        // Two full pages and one row, offered out of order and with one
        // peer doubled.
        let mut rows: Vec<_> = (0..=2 * MAX_PEER_PAGE_ROWS).map(gate_row).collect();
        rows.push(gate_row(7));
        rows.reverse();
        let mut peers: Vec<_> = (0..=2 * MAX_PEER_PAGE_ROWS).map(synthetic_peer).collect();
        peers.sort();

        // The server's page bound is the smaller one, every row carrying
        // `last_seen_ms`; a reader still accepts the schema's 512.
        let per_page = MAX_PEER_PAGE_ROWS_SINCE_LAST_SEEN;
        let mut after = None;
        let mut read = Vec::new();
        let mut pages = 0;
        loop {
            let page = PeerList::page(rows.clone(), after.as_ref());
            pages += 1;
            assert!(page.peers.len() <= per_page);
            read.extend(page.peers.iter().map(|row| row.peer.clone()));
            let Some(next) = page.next else { break };
            assert_eq!(page.peers.len(), per_page, "a page before the last is full");
            assert_eq!(Some(&next), read.last(), "the last row is the cursor");
            after = Some(next);
        }
        assert_eq!(pages, peers.len().div_ceil(per_page));
        assert_eq!(read, peers, "every peer once, ascending");

        // A row says what the view said, and the cursor is a position.
        let page = PeerList::page(vec![gate_row(3)], None);
        assert_eq!(page.peers, vec![PeerRow::from_view(gate_row(3))]);
        assert_eq!(page.next, None);
        let gone = peers[10].clone();
        let without: Vec<_> = rows.iter().filter(|r| r.peer != gone).cloned().collect();
        let page = PeerList::page(without, Some(&gone));
        assert_eq!(page.peers[0].peer, peers[11]);
    }

    /// A row carries `route_held_until` when the view holds a route, and
    /// no key when it holds none.
    #[test]
    fn a_peer_row_carries_the_route_hold() {
        let held = PeerGateView {
            route_held_until: Some(1_791_329_960_227),
            ..gate_row(1)
        };
        let page = |view: PeerGateView| {
            serde_json::to_value(PeerList::page(vec![view], None)).expect("serializes")
        };
        assert_eq!(
            page(held.clone())["peers"][0]["route_held_until"],
            json!(1_791_329_960_227_u64)
        );
        let without = page(PeerGateView {
            route_held_until: None,
            ..held
        });
        assert_eq!(
            without["peers"][0].get("route_held_until"),
            None,
            "{without}"
        );
    }

    /// Every row carries `last_seen_ms`: the time, or `null` for a peer
    /// not seen since the daemon started; a port that does not say is
    /// answered with no key. The three wire states read back apart.
    #[test]
    fn a_peer_row_carries_last_seen() {
        let seen = PeerGateView {
            last_seen: PeerLastSeen::At(1_791_329_940_227),
            ..gate_row(1)
        };
        let never = PeerGateView {
            last_seen: PeerLastSeen::Never,
            ..gate_row(1)
        };
        let at = |view: &PeerGateView| {
            serde_json::to_value(PeerList::page(vec![view.clone()], None)).expect("serializes")
        };
        assert_eq!(
            at(&seen)["peers"][0]["last_seen_ms"],
            json!(1_791_329_940_227_u64)
        );
        let null = at(&never);
        assert_eq!(
            null["peers"][0].get("last_seen_ms"),
            Some(&json!(null)),
            "{null}"
        );
        // A port that does not say is answered as one: no key, never
        // `null`, which would claim the peer was never seen.
        let unreported = PeerGateView {
            last_seen: PeerLastSeen::Unreported,
            ..gate_row(1)
        };
        let absent = at(&unreported);
        assert_eq!(absent["peers"][0].get("last_seen_ms"), None, "{absent}");

        let read = |row: serde_json::Value| -> PeerRow {
            serde_json::from_value(row).expect("a legal row")
        };
        let base = json!({"peer": synthetic_peer(1).as_str(), "connected": false});
        let with = |value: serde_json::Value| {
            let mut row = base.clone();
            row["last_seen_ms"] = value;
            row
        };
        assert_eq!(read(with(json!(7))).last_seen_ms, Some(Some(7)));
        assert_eq!(read(with(json!(null))).last_seen_ms, Some(None));
        assert_eq!(read(base.clone()).last_seen_ms, None);
        for bad in [json!(-1), json!(1.5), json!("7")] {
            assert!(
                serde_json::from_value::<PeerRow>(with(bad.clone())).is_err(),
                "{bad} refused"
            );
        }
    }

    #[test]
    fn a_peer_list_is_held_to_its_bound_its_order_and_its_names() {
        let row = |i: usize| json!({"peer": synthetic_peer(i).as_str(), "connected": true});
        let sorted = |n: usize| {
            let mut rows: Vec<_> = (0..n).map(row).collect();
            rows.sort_by(|a, b| a["peer"].as_str().cmp(&b["peer"].as_str()));
            rows
        };
        assert!(
            serde_json::from_value::<PeerList>(json!({"peers": sorted(MAX_PEER_PAGE_ROWS)}))
                .is_ok()
        );
        assert!(
            serde_json::from_value::<PeerList>(json!({"peers": sorted(MAX_PEER_PAGE_ROWS + 1)}))
                .is_err()
        );
        let mut unordered = sorted(3);
        unordered.swap(0, 2);
        assert!(serde_json::from_value::<PeerList>(json!({"peers": unordered})).is_err());
        let doubled = vec![row(1), row(1)];
        assert!(serde_json::from_value::<PeerList>(json!({"peers": doubled})).is_err());
        assert!(serde_json::from_value::<PeerList>(json!({"peers": [], "next": null})).is_err());
        // Every outcome by its section 19 name, both ways, and nothing else.
        for outcome in [
            PeerOutcome::Connected,
            PeerOutcome::DialFailed,
            PeerOutcome::IdentityMismatch,
            PeerOutcome::Denied,
        ] {
            let mut r = row(1);
            r["last_outcome"] = json!(outcome.as_str());
            let page: PeerList =
                serde_json::from_value(json!({"peers": [r.clone()]})).expect("valid");
            assert_eq!(page.peers[0].last_outcome, Some(outcome));
            assert_eq!(
                serde_json::to_value(&page).expect("serializes")["peers"][0],
                r
            );
        }
        for bad in [json!("refused"), json!("timeout"), json!(null), json!(3)] {
            let mut r = row(1);
            r["last_outcome"] = bad;
            assert!(serde_json::from_value::<PeerList>(json!({"peers": [r]})).is_err());
        }
        let mut r = row(1);
        r["backoff_until"] = json!(null);
        assert!(
            serde_json::from_value::<PeerList>(json!({"peers": [r]})).is_err(),
            "a null time"
        );
        let mut r = row(1);
        r["address"] = json!("/ip4/10.0.0.1/tcp/1");
        assert!(
            serde_json::from_value::<PeerList>(json!({"peers": [r]})).is_err(),
            "no address field"
        );
    }

    #[test]
    fn a_trust_list_pages_in_order_and_says_when_more_remain() {
        let local = synthetic_peer(9_999);
        // Two full pages and one row, offered out of order.
        let mut peers: Vec<_> = (0..=2 * MAX_TRUST_PAGE_ROWS).map(synthetic_peer).collect();
        peers.reverse();
        let view = TrustAdminView {
            local_peer: Some(local.clone()),
            allowed: peers
                .clone()
                .into_iter()
                .map(|peer| interweave_local_client_api::TrustedPeer {
                    peer,
                    persisted: false,
                    source: interweave_local_client_api::TrustSource::Configured,
                })
                .collect(),
        };
        peers.sort();

        let mut after = None;
        let mut read = Vec::new();
        let mut pages = 0;
        loop {
            let page = TrustList::page(view.clone(), after.as_ref());
            pages += 1;
            assert_eq!(
                page.local_peer.as_ref(),
                (pages == 1).then_some(&local),
                "the local peer on the first page only"
            );
            assert!(page.allowed.len() <= MAX_TRUST_PAGE_ROWS);
            read.extend(page.allowed.iter().map(|row| row.peer.clone()));
            match page.next {
                Some(next) => {
                    assert_eq!(Some(&next), read.last(), "the last row is the cursor");
                    after = Some(next);
                }
                None => break,
            }
        }
        assert_eq!(pages, 3);
        assert_eq!(read, peers, "every peer once, ascending");

        // Exactly one page: no `next`, so the reader stops.
        let full = TrustAdminView {
            local_peer: None,
            allowed: peers[..MAX_TRUST_PAGE_ROWS]
                .iter()
                .cloned()
                .map(|peer| interweave_local_client_api::TrustedPeer {
                    peer,
                    persisted: false,
                    source: interweave_local_client_api::TrustSource::Configured,
                })
                .collect(),
        };
        let page = TrustList::page(full, None);
        assert_eq!(page.allowed.len(), MAX_TRUST_PAGE_ROWS);
        assert_eq!(page.next, None);
        assert_eq!(page.local_peer, None, "absent when the policy has none");

        // A cursor naming a peer no longer listed is a position.
        let gone = peers[10].clone();
        let mut without = view.clone();
        without.allowed.retain(|row| row.peer != gone);
        let page = TrustList::page(without, Some(&gone));
        assert_eq!(page.allowed[0].peer, peers[11]);
    }

    #[test]
    fn a_trust_list_is_held_to_its_bound_and_its_order() {
        let row = |i| json!({"peer": synthetic_peer(i).as_str(), "persisted": false});
        let rows = |n: usize| (0..n).map(row).collect::<Vec<_>>();
        let mut sorted = rows(MAX_TRUST_PAGE_ROWS);
        sorted.sort_by(|a, b| a["peer"].as_str().cmp(&b["peer"].as_str()));
        assert!(serde_json::from_value::<TrustList>(json!({"allowed": sorted})).is_ok());
        let mut over = rows(MAX_TRUST_PAGE_ROWS + 1);
        over.sort_by(|a, b| a["peer"].as_str().cmp(&b["peer"].as_str()));
        assert!(serde_json::from_value::<TrustList>(json!({"allowed": over})).is_err());
        let (a, b) = (row(1), row(2));
        let (low, high) = if a["peer"].as_str() < b["peer"].as_str() {
            (a, b)
        } else {
            (b, a)
        };
        assert!(
            serde_json::from_value::<TrustList>(json!({"allowed": [low.clone(), high.clone()]}))
                .is_ok()
        );
        assert!(
            serde_json::from_value::<TrustList>(json!({"allowed": [high, low.clone()]})).is_err(),
            "out of order"
        );
        assert!(
            serde_json::from_value::<TrustList>(json!({"allowed": [low.clone(), low]})).is_err(),
            "a duplicate"
        );
        assert!(
            serde_json::from_value::<TrustList>(json!({"allowed": [], "next": null})).is_err(),
            "next is a peer or absent"
        );
        // The schema's if/then/else: a persisted row names its source,
        // and only a persisted row does.
        let one = |row: serde_json::Value| {
            serde_json::from_value::<TrustList>(json!({ "allowed": [row] })).map(|_| ())
        };
        let p = synthetic_peer(0);
        assert!(
            one(json!({"peer": p.as_str(), "persisted": true, "source": "configured"})).is_ok()
        );
        assert!(
            one(json!({"peer": p.as_str(), "persisted": true, "source": "administered"})).is_ok()
        );
        assert!(one(json!({"peer": p.as_str(), "persisted": false})).is_ok());
        assert!(
            one(json!({"peer": p.as_str(), "persisted": true})).is_err(),
            "persisted without its source"
        );
        assert!(
            one(json!({"peer": p.as_str(), "persisted": false, "source": "configured"})).is_err(),
            "a source on the 2.1 row"
        );
        assert!(
            one(json!({"peer": p.as_str(), "persisted": true, "source": null})).is_err(),
            "an explicit null"
        );
        assert!(
            one(json!({"peer": p.as_str(), "persisted": true, "source": "someone"})).is_err(),
            "an unknown source"
        );
    }

    /// A persisted row carries its source; the page renders a row the
    /// binding does not persist without one, and the server refuses to
    /// serve such a row at all (ipc-server's
    /// `an_unpersisted_trust_row_is_refused`).
    #[test]
    fn a_trust_row_carries_its_source_when_persisted() {
        let (configured, administered) = (synthetic_peer(1), synthetic_peer(2));
        let view = |persisted| TrustAdminView {
            local_peer: None,
            allowed: vec![
                interweave_local_client_api::TrustedPeer {
                    peer: configured.clone(),
                    persisted,
                    source: TrustSource::Configured,
                },
                interweave_local_client_api::TrustedPeer {
                    peer: administered.clone(),
                    persisted,
                    source: TrustSource::Administered,
                },
            ],
        };
        let rows = |persisted| {
            let page =
                serde_json::to_value(TrustList::page(view(persisted), None)).expect("serializes");
            let mut rows = page["allowed"].as_array().expect("rows").clone();
            rows.sort_by(|a, b| a["peer"].as_str().cmp(&b["peer"].as_str()));
            rows
        };
        assert_eq!(
            rows(true),
            [
                json!({"peer": configured.as_str(), "persisted": true, "source": "configured"}),
                json!({"peer": administered.as_str(), "persisted": true, "source": "administered"}),
            ]
        );
        let old = |p: &TransportIdentity| json!({"peer": p.as_str(), "persisted": false});
        assert_eq!(rows(false), [old(&configured), old(&administered)]);
    }

    /// A full page of the largest 2.3 rows -- 52-character Ed25519 ids,
    /// `administered`, the first page's `local_peer` and a `next` -- fits
    /// the body with room for the frame.
    #[test]
    fn a_full_trust_page_of_the_largest_rows_fits_the_body() {
        let longest = TransportIdentity::parse(
            "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN".to_owned(),
        )
        .expect("an Ed25519 PeerId");
        let page = TrustList {
            local_peer: Some(longest.clone()),
            allowed: (0..MAX_TRUST_PAGE_ROWS)
                .map(|_| TrustRow {
                    peer: longest.clone(),
                    persisted: true,
                    source: Some(TrustSource::Administered),
                })
                .collect(),
            next: Some(longest),
            set: None,
        };
        let bytes = serde_json::to_vec(&page).expect("serializes").len();
        assert!(
            bytes + 1_024 <= crate::framing::MAX_BODY_BYTES,
            "{bytes} bytes for {MAX_TRUST_PAGE_ROWS} rows"
        );
    }

    /// `TrustList::page_endpoint`: the operator form's page, order and
    /// cursor, `set` on the answer and no `local_peer`.
    #[test]
    fn an_endpoint_page_says_its_set_and_has_no_local_peer() {
        use interweave_local_client_api::TrustedPeer;
        let peers: Vec<_> = (0..=MAX_TRUST_PAGE_ROWS + 1).map(synthetic_peer).collect();
        let view = |set| EndpointTrustView {
            set,
            allowed: peers
                .iter()
                .rev()
                .map(|peer| TrustedPeer {
                    peer: peer.clone(),
                    persisted: true,
                    source: TrustSource::Administered,
                })
                .collect(),
        };
        let mut sorted = peers.clone();
        sorted.sort();
        let first = TrustList::page_endpoint(view(EndpointSetKind::Own), None);
        assert_eq!(first.set, Some(EndpointSetKind::Own));
        assert_eq!(first.local_peer, None);
        assert_eq!(first.allowed.len(), MAX_TRUST_PAGE_ROWS);
        assert_eq!(
            first.allowed[0].peer, sorted[0],
            "sorted whatever the port's order"
        );
        assert_eq!(first.next.as_ref(), Some(&sorted[MAX_TRUST_PAGE_ROWS - 1]));
        let second = TrustList::page_endpoint(view(EndpointSetKind::Own), first.next.as_ref());
        assert_eq!(second.allowed.len(), 2);
        assert_eq!(second.next, None);
        assert_eq!(second.set, Some(EndpointSetKind::Own), "on every page");
        // The control: the operator's page carries no `set`.
        let operator = TrustList::page(
            TrustAdminView {
                local_peer: Some(synthetic_peer(9999)),
                allowed: Vec::new(),
            },
            None,
        );
        assert_eq!(operator.set, None);
        assert!(operator.local_peer.is_some());
        let wire = serde_json::to_value(&first).expect("ser");
        assert_eq!(wire["set"], "own");
    }
}
