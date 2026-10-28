# Local endpoint addressing contract

Status: **architecture contract, transport v2 draft**.

This contract defines Model B: one profile-scoped transport identity / PeerId with multiple independently addressable local application endpoints.

The prose here is normative for **behaviour**. The shapes it describes are also defined as JSON Schema under [`schemas/endpoints/`](./schemas/endpoints/) — normative for **shape**, and carrying an `x-contract.status` that says what each is authoritative about (ADR-0049). Where the two describe the same field they must agree; a disagreement is a bug in whichever one drifted.

## Purpose

A transport profile remains one cryptographic network node:

```text
PeerId P
  |
  +-- EndpointId "human"
  +-- EndpointId "claude"
  +-- EndpointId "automation.build"
```

`EndpointId` is a routing selector inside the node. It is **not** a second PeerId, an organizational role, or an authorization proof. Since ADR-0067 (A 2026-10-28) it is the name of the endpoint's own key pair, derived as a PeerId is; the names in the diagram are the endpoints' local aliases, never what the wire carries.

## EndpointId

**An EndpointId is derived from the endpoint's own Ed25519 public key exactly as a PeerId is from the node's** (ADR-0067, A 2026-10-28): the libp2p identity multihash over the protobuf-encoded public key, written in the canonical base58btc text. Canonical grammar, the PeerId's:

```regex
^12D3KooW[1-9A-HJ-NP-Za-km-z]{44}$
```

Rules:

- 52 ASCII bytes, under every 64-byte ceiling the label grammar met (`ipc/hello`, the payload budgets), so no ceiling moves;
- case-sensitive, one canonical form: the text is a base58btc number, compared byte for byte;
- derived, never chosen: an id written by hand in a configuration or an administrative add is refused as `EndpointIdNotDerived`, and the daemon or embedded host derives every id from the key the entry names at load;
- unique by construction within a node and across nodes — two equal ids are one key;
- no wildcard endpoint exists;
- no endpoint may be addressed as a broadcast substitute;
- endpoint IDs are peer-asserted routing metadata when received from the network: a remote `source_endpoint` proves that the authenticated PeerId claimed that endpoint, and a signature under the endpoint's key — a contact card's (HUMAN-CHAT.md §Contact card), whose key the receiver checks derives the endpoint — proves the key behind it.

The grammar before ADR-0067 — `^[a-z][a-z0-9._-]{0,63}$`, a lowercase ASCII label of 1 to 64 bytes, "stable while configured, but not cryptographically derived" — is retired without a compatibility window on the owner's word, since no installed base exists: the endpoint-id schema's `1.0.0` stays the wire until the release that builds Stage 22 (plan §30), whose batch writes the endpoint-id and endpoint-config schemas' `2.0.0` with this grammar and entry (`approved` targets until served, ADR-0049), takes a major on every schema that `$ref`s the endpoint-id, moves the direct protocol to `/interweave/direct/3.0.0` and the IPC major to `3` (ADR-0067 decision 7), and regenerates every fixture family that carries an EndpointId; `ipc/hello` takes the id alone, a client resolving its alias from the profile's configuration. What a person or an operator types to pick an endpoint — `human`, `claude`, `fabric-control` — is a **local alias** of the configuration entry and of the clients' command lines: it never reaches the wire, is unique only within the profile and proves nothing. The complete network route is `EndpointAddress = (TransportIdentity, EndpointId)`.

## Configured endpoint set

A profile may configure at most 64 endpoints. Registration is **configured-only** in transport v2: an ordinary local data-plane client cannot create a new endpoint by choosing an arbitrary name at session/handshake establishment.

Conceptually:

```text
EndpointConfig {
  key: reference to the endpoint's sealed key file   -- <state_dir>/endpoint-keys/<EndpointId>.key, ADR-0067; the id is derived from it at load
  alias: local display name, never on the wire       -- ADR-0067
  enabled: bool
  advertise: bool
  allowed_client_kinds: bounded list[string]
  inbound: inherit_profile_trust | static_subset[PeerId]   -- ADR-0067 decision 6: an identity's set lives in the endpoint overlay; an authored static_subset narrows the operator's list; inherit reads the operator's list, never the admission set
  outbound: inherit_profile_trust | static_subset[PeerId]
}
```

Before ADR-0067 the entry carried `id: EndpointId`; an entry by id is refused at load (`EndpointIdNotDerived`), an entry whose key file is missing or unreadable loads in the key-missing state (its lease and routes kept, nothing signed), and the endpoint-config schema follows the Stage 22 batch as the endpoint-id schema does. The key is minted where the endpoint is created — by the client on an identity's creation through the administrative add, which takes a key and returns the derived id, or by `transportctl endpoint init --profile <name> [--alias <alias>]` with no daemon, which prints the id for a rendered deployment to record (ADR-0055 decision 4).

**Trust is per endpoint** (ADR-0067 decision 6): each endpoint's `inbound` and `outbound` subset is its own trusted set, the person's trust acts being the identity in front's; the node's allow-list (ADR-0028: `trust.allowed_peers` plus the overlay's added minus its revoked, bounded by `MAX_ALLOWED_PEERS`) is the **operator's list**, written only by an endpoint-less `admin.trust.set` or `transportctl trust allow|revoke`; an **identity's set** lives in the endpoint overlay (`<state>/endpoint-overlay.json`, ADR-0028, which gains per-endpoint `inbound` and `outbound` members keyed by EndpointId), written by the human client for every identity through the endpoint dimension of those methods (an IPC minor, Stage 22) and on Android through the embedded host, the authored first identity's seeded at the client's first run on the Stage 22 release (the authored subset if any, else the operator's list at that moment; until that run the authored entry reads the operator's list, as before); the **admission set** at the connection is derived by the transport at load and on every overlay write as the union of the operator's list and every endpoint's overlay set (an identity's, or any endpoint the operator gave one), bounded as one set by `MAX_ALLOWED_PEERS` (FATAL at start past it, a crossing write refused by name), never stored or maintained by a client. At the route an endpoint's effective set is its overlay set when the overlay carries one, else an authored `static_subset` (which narrows the operator's list and admits nothing beyond it), else under `inherit_profile_trust` the operator's list and never the admission set — so an endpoint that is no identity (an automated client's, a rendered profile's) trusts what the operator trusts and nothing a person trusted in an identity, and an endpoint-less call reaches every inheriting endpoint and no identity. So endpoint policy still never authorizes a PeerId the profile rejects, a peer trusted by any endpoint may connect, and an endpoint that does not trust it refuses its route with the coarse `no_route`. Broadcast admission, subscription and the ingress bucket stay the node's.

`allowed_client_kinds` is an accidental-misbinding guard, not authentication. On desktop the same-OS-user residual threat remains governed by the IPC security model. Android embedded mode treats this field only as in-process configuration hygiene.

## Default direct endpoint

A profile may configure zero or one `default_direct_endpoint`.

A direct request with no explicit destination endpoint means:

```text
route to the receiver profile's configured default_direct_endpoint
```

It never means "fan out to every local client".

The default must reference an enabled configured endpoint. If no default exists, or the default endpoint has no active local lease, the remote request receives the same coarse `no_route` rejection used for unknown/unavailable/endpoint-policy-denied routes.

## Local endpoint lease

Every direct-capable `LocalDataSession` that sends or receives direct messages owns at most one exclusive endpoint lease. Desktop binds that session to one IPC connection; Android embedded mode binds it to the foreground-service/runtime session.

Properties:

- lease is bound to one local data-plane session (one IPC connection on desktop; one embedded session generation on Android);
- malformed EndpointId -> local `InvalidArgument` (reachable on the IPC surface for a claim within `ipc/hello` 1.2.0's 1–64-character bounds that breaks the grammar — outside the bounds the frame itself is refused at parse; an embedded binding's claim is a typed `EndpointId` and is always well-formed, so the IPC wire test is the row's proof; A 2026-10-01);
- configured endpoint absent -> local `EndpointUnknown`;
- configured endpoint disabled -> local `EndpointDisabled`;
- configured `allowed_client_kinds` mismatch -> local `EndpointClientKindDenied`;
- ungranted capability/connection authorization -> local `CapabilityDenied`;
- at most one live connection owns an EndpointId at a time;
- duplicate claim -> `EndpointInUse`;
- lease disappears immediately on local-session teardown/revocation; desktop additionally uses IPC disconnect or bounded negotiated keepalive expiry; Android uses foreground-service/process session lifetime;
- no message buffering is created while an endpoint is unleased;
- reconnect/recreate performs a new local-session establishment and obtains a fresh opaque 128-bit local lease epoch; the value must not repeat across daemon restart/reconnect within any practical stale-route lifetime;
- ordinary remote messages can never create, steal, transfer, or enable an endpoint lease.

Desktop administration uses the separate admin IPC socket and `admin.endpoints`; Android embedded mode uses a distinct in-process `LocalAdminPort` that is never handed to remote-event/message handlers. In neither mode can a data-plane session grant itself administrative authority.

## Direct destination

Transport v2 represents a directed destination as:

```text
DirectDestination {
  peer: TransportIdentity,
  endpoint: EndpointId?   // absent => remote default endpoint
}
```

The sending local data-plane session's active endpoint lease supplies the required `source_endpoint`; callers cannot spoof another local endpoint.

## Inbound routing order

For an authenticated direct request:

1. authenticate remote PeerId through Noise;
2. apply profile `PeerTrustPolicy`;
3. validate bounded frame/header/message-ID fields sufficiently for safe admission and apply the mandatory per-source-PeerId/global direct ingress token buckets; overflow returns coarse `overloaded`;
4. validate remaining endpoint/media/payload bounds and protocol version, then construct the accepted-message dedup key and content fingerprint; on positive hit return the stored `AcceptedV2(resolved_endpoint)` without re-enqueue; otherwise atomically acquire/join the bounded in-flight reservation for that key (matching duplicates share the owner result; fingerprint conflicts fail);
5. reservation owner resolves destination endpoint: explicit endpoint or configured default;
6. apply endpoint inbound policy as a narrowing filter over profile trust;
7. require an active exclusive local lease for that endpoint;
8. require capacity in that endpoint owner's bounded local event queue;
9. enqueue exactly one normalized `MessageReceived` event to the owning local data-plane session;
10. store the positive dedup entry with resolved endpoint;
11. only then send transport `Accepted`.

If endpoint resolution/policy/lease steps 5-7 cannot produce a local route, the wire response is the coarse `no_route` rejection. Queue admission failure at step 8 returns coarse `overloaded`, not `Accepted`. Local diagnostics retain bounded reason classes (`endpoint_unknown`, `endpoint_offline`, `endpoint_policy`, `endpoint_overloaded`) without disclosing endpoint existence/policy detail remotely.

`Accepted` therefore means the remote daemon admitted the message into the bounded local event queue of the resolved endpoint. It still does not mean the human, Claude instance, or other application processed it. All endpoint no-route branches emit the same coarse code/response shape through a shared encoder. The transport does not claim constant-time endpoint-policy evaluation; timing differences remain a residual oracle available only to an already trusted peer and are bounded by mandatory per-PeerId direct ingress rate limits.

## Outbound routing order

For `send(destination, payload)`:

1. caller must own an active endpoint lease or receive `EndpointNotRegistered`;
2. validate the optional remote EndpointId grammar; malformed/noncanonical endpoint input is `InvalidArgument`;
3. local endpoint outbound policy is applied; a destination excluded by that narrowing policy returns `UnauthorizedPeer` locally;
4. profile outbound `PeerTrustPolicy` is applied;
5. self PeerId is rejected as `InvalidArgument`;
6. ConnectionManager resolves/dials under existing trust/backoff/global-limit policy;
7. direct protocol v2 carries the caller's leased `source_endpoint` and requested optional `destination_endpoint`;
8. remote `Accepted` includes the endpoint that actually accepted the message; before the result is cached/surfaced, the sender validates the response message ID, EndpointId length/grammar, and (for explicit routing) exact equality with the requested endpoint. Invalid remote metadata is `ProtocolViolation` and does not become tool/UI metadata. A response stream that ends with no bytes is not invalid metadata but the exchange not completing — the remote read the request and never answered — and is `PeerUnreachable`, in TRANSPORT.md's outcome-unknown class (`transport/libp2p/DIRECT.md` §Response byte layout, A 2026-10-26).

A caller cannot select a different `source_endpoint` field in command arguments.

## Deduplication

The normalized direct accepted-message dedup key is based on the **wire destination selector**, not the receiver's current default:

```text
(
  mode=direct,
  source_peer,
  source_endpoint,
  destination_selector = Explicit(EndpointId) | Default,
  message_id
)
```

A successful cache entry stores the first `resolved_destination_endpoint` plus **DirectContentFingerprintV1**; `sent_at_ms` is excluded. The fingerprint is fixed for cross-implementation fixtures:

```text
domain = UTF8("interweave/direct-content-fingerprint/v1\0")

canonical =
  domain ||
  media_present:u8 ||
  [media_len:u16be || media_ascii] ||   # only when media_present = 1
  payload_len:u32be ||
  payload_bytes

fingerprint = SHA-256(canonical)
```

Rules: `media_present` is exactly `0` or `1`; an absent media type uses `0` and has no media-length field; a present media type uses `1`, must be 1..128 printable ASCII bytes (0x20..0x7E — the bound the schemas pin and the decoders enforce, A 2026-10-08), and includes its two-byte big-endian length. Empty media type is invalid rather than an alias for absence. Payload length is the exact byte length before the payload. No JSON, UTF-8 normalization, endpoint fields, message ID, or timestamp participates.

Golden vector, re-frozen by ADR-0047 and held with its edge cases in
[`fixtures/direct-v2/direct-content-fingerprint-v1.json`](../../fixtures/direct-v2/direct-content-fingerprint-v1.json),
which `tools/checks/verify_fixture_vectors.py` recomputes — including this copy:

```text
media_type = "text/plain"
payload    = UTF8("hello")
SHA-256    = d73342f033f00fca9c4ffcced6f9e6debaeb53e3743049ee9aaf227a55f9bf15
```

After the request passes current trust/structural/direct-ingress rate admission, a duplicate accepted request within TTL is handled as follows:

- matching fingerprint -> return `AcceptedV2` with the same stored resolved endpoint and do **not** enqueue another local event, even if the endpoint disconnected or the profile default changed;
- different fingerprint under the same dedup key -> reject as a duplicate-ID/content conflict (`malformed` on the coarse wire, detailed only locally).

This prevents admitted retries from being rerouted to a different local application and prevents one idempotency key from silently aliasing two different message bodies. A retry rejected by the current direct-ingress token bucket receives coarse `overloaded` before dedup lookup; that rejection neither removes nor rewrites an existing positive dedup entry, so a later admitted retry can still receive the stored acceptance without a second enqueue. Concurrent same-key requests are serialized by a bounded in-flight reservation so at most one local enqueue can occur. Reservation capacity is aligned with direct in-flight admission: **128 global / 8 per source PeerId by default, ceilings 512 / 32**. Those budgets count outstanding **requests**, not distinct keys: an attached waiter holds a response channel until the owner's admission resolves, so it is state the sender chose to make this daemon hold and is charged for exactly as the owner is. Releasing a key returns the owner's and every waiter's share together. Per the ADR-0019 amendment of 2026-08-27, holding the waiter's response channel binds from the stage whose admission yields; a synchronous admission may treat the branch as unreachable but must not answer it as exhaustion. The charging and release rules in this paragraph are unaffected and bind in every stage. Counting keys alone would leave one reserved key able to accumulate unbounded waiters — SPIKE-002/A11 measured 39 attached with zero refusals before this was stated. Capacity exhaustion rejects the new request as coarse wire `overloaded` / local `Overloaded`; it must not fall through to a second enqueue path. Rejected/no-route requests are not positive acceptance records: matching in-flight waiters receive that rejection, then the reservation is removed, allowing a later retry to succeed after route recovery. After the bounded dedup TTL expires, the transport no longer promises retry idempotency for that ID.

Including `source_endpoint` prevents a message ID collision between two endpoints on the same authenticated peer from suppressing an independent delivery.

## Direct MessageReceived event

Transport v2 direct events contain:

```text
MessageReceived {
  message_id,
  mode: direct,
  source_peer,
  source_endpoint,
  destination_endpoint,   // resolved local endpoint, never absent
  payload,
  received_at,
}
```

Broadcast events do not acquire endpoint routing metadata. Broadcast remains ChannelId-scoped and is delivered according to local join references. Consequently, two local endpoints sharing one PeerId are intentionally indistinguishable as **transport-level broadcast originators**; a human/app protocol that needs author/service provenance must carry and authenticate it above the transport. A private reply to a broadcast cannot infer the originating EndpointId from transport metadata and must use an explicit/out-of-band/directory-selected endpoint or the remote default.

## Reply routing

A local reply route for a direct inbound event contains:

```text
remote_peer = source_peer
remote_endpoint = source_endpoint
local_endpoint = destination_endpoint
```

A reply is allowed only while the replying local data-plane session still owns the same `local_endpoint` lease. The reply uses that leased endpoint as the source and sends to the original remote source endpoint.

A stale token or route must never silently fall back to the profile default endpoint or another local endpoint.

## Endpoint directory

Transport v2 defines an optional remote endpoint-directory capability, specified in `transport/libp2p/ENDPOINTS.md` and ADR-0031.

Important properties:

- trust-gated direct control protocol, not GossipSub;
- lists only endpoints with `advertise: true` **and an active local lease** — `advertise` being the effective value, the endpoint overlay applied over the authored configuration (ADR-0028 A 2026-10-21; added 2026-10-21 with ADR-0061, which turns a human identity's advertisement off through that overlay);
- maximum 32 advertised endpoints per response;
- endpoint identifiers only: no human name, avatar, role, prompt, application payload, or permission claim;
- results are advisory and short-lived;
- a caller may send to an out-of-band EndpointId without first querying the directory;
- directory support is not required for endpoint-addressed delivery itself;
- directory requests are rate/concurrency bounded independently of direct-message requests. The per-peer rate is a **pre-trust** budget: it is charged for every syntactically valid query that reaches the handler, refused ones included, because an infrastructure-only connection can open a directory substream and the in-flight bound caps concurrency rather than frequency. An allowance granted to a peer already found data-plane trusted is a separate concept and is not this one;
- remote directory data is validated before cache/tool/UI use: >32 entries, invalid EndpointId grammar, or duplicates are `ProtocolViolation` — each of which is an answer that arrived; a response stream ending with no bytes is the remote not answering, `PeerUnreachable`, never `ProtocolViolation` (`transport/libp2p/ENDPOINTS.md` §Endpoint directory protocol, A 2026-10-26); valid unsorted unique lists are sorted locally; `ttl_ms` is clamped to the local/hard 5-minute ceiling and freshness starts at local receipt, while `generated_at_ms` is diagnostic only.
- `advertise: false` controls directory listing only; if such an endpoint is selected explicitly or as the profile default and accepts a direct message, normal direct-protocol routing metadata/acceptance may reveal that route to the communicating peer.

## Endpoint-specific authorization

Endpoint policy is an intersection:

```text
admitted(peer, endpoint)
  = profile_trust(peer)
    AND endpoint_policy(peer)
```

No endpoint configuration can widen profile trust.

For privacy, a trusted peer denied by an endpoint-specific inbound policy receives `no_route`, not an oracle that confirms the endpoint exists but is forbidden.

## Human identity and application semantics

A human client may maintain application data such as:

```text
Contact {
  display_name,
  peer,
  endpoint,
  avatar,
  verification_state,
}
```

That mapping is outside this transport contract. The transport never asserts that `PeerId + EndpointId` is a particular person, employee, agent, or application role.

## Persistence

Transport-owned persistent endpoint state is configuration only. Active leases, remote directory results, reply routes, and endpoint availability are runtime state.

The daemon does not persist application messages for an offline endpoint. ADR-0044 permits the first-party human application to retain only pending outbound, unread inbound, and receiver-kept-after-read inbound content above the transport boundary; that application state is not an offline network mailbox.

## Resource limits

Defaults / ceilings:

| Resource | Default | Ceiling |
|---|---:|---:|
| configured endpoints/profile | 64 (no per-profile key; the schema's `max=64`, `MAX_ENDPOINTS`; corrected 2026-10-21 with ADR-0061 from a default of 16 nothing held) | 64 |
| advertised endpoints | 16 | 32 |
| EndpointId | 64 bytes | 64 bytes |
| endpoint-directory cache TTL | 60 s | 5 min |
| endpoint-directory queries/peer/minute | 12 | 60 |
| endpoint-directory inflight/profile | 16 | 64 |
| one endpoint lease per local data-plane session | 1 | 1 |
| direct dedup reservation/global | 128 | 512 |
| direct dedup reservation/source peer | 8 | 32 |

Existing per-session event-queue and direct-send concurrency limits continue to apply. Desktop maps these to IPC-client limits; Android maps them to the embedded local-session adapter.

## Versioning

Endpoint addressing is a **transport contract major-version change** from v1 to v2 and a local IPC major-version change from v1 to v2.

The direct wire protocol is `/interweave/direct/2.0.0`. Because this repository still has no production v1 implementation or deployed compatibility obligation, the first production implementation should target v2 directly. Implementing `/direct/1.0.0` compatibility is optional and must not be used as an implicit fan-out escape hatch.

Endpoint-directory protocol versioning is independent from direct protocol versioning.

## Required conformance cases

- two local endpoints on one PeerId receive only explicitly/default-routed direct traffic intended for them;
- endpoint handshake error mapping is exact: malformed=`InvalidArgument`, absent=`EndpointUnknown`, disabled=`EndpointDisabled`, kind mismatch=`EndpointClientKindDenied`, capability denied=`CapabilityDenied`, collision=`EndpointInUse`;
- disconnect makes the endpoint unavailable immediately and creates no queue;
- peer-only destination resolves exactly one configured default endpoint;
- no default -> coarse remote `no_route`;
- endpoint-specific allowlist is the endpoint's own trusted set: an authored subset never widens the operator's list, an identity's overlay set is an admission source of its own, and the admission set is the union the transport derives from both (ADR-0067);
- source endpoint is taken from the local-session lease, not caller-controlled payload/params;
- reply routes back to the original remote source endpoint;
- stale reply token never falls back to another local route;
- directory lists only active advertised endpoints and respects trust;
- endpoint directory can be disabled while explicit endpoint sends continue to work;
- same PeerId using the same `message_id` from two source endpoints yields two independent deliveries;
- DirectContentFingerprintV1 fixture is byte-identical across implementations and reservation overflow returns overload without a second enqueue;
- an offline human endpoint has no daemon-side message backlog.
