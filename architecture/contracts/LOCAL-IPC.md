# Local IPC contract

Applies because ADR-0015 selects a separate daemon. Model B endpoint addressing made this IPC major version 2; the derived EndpointId makes it **IPC major version 3** from the Stage 22 release (ADR-0067 decision 7; §Version negotiation and phases states what the major holds).

The prose here is normative for **behaviour**. The shapes it describes are also defined as JSON Schema under [`schemas/ipc/`](./schemas/ipc/) — normative for **shape**, carrying an `x-contract.status` that says what each is authoritative about (ADR-0049). Where the two describe the same field they must agree; a disagreement is a bug in whichever one drifted.

## Transport choice and authority domains

The IPC uses **two distinct local endpoints**:

- data-plane socket: Unix domain socket `<runtime>/<profile>.sock`; Windows named-pipe equivalent;
- administrative socket: Unix domain socket `<runtime>/<profile>.admin.sock` — the `.` sits outside the profile-name alphabet (`[A-Za-z0-9_-]`), so no profile's data socket can share a path with another's admin socket (A 2026-10-01); Windows named-pipe equivalent.

Loopback TCP is not a default fallback; enabling it later requires a separate authentication design. The socket selected by the client is an authority-domain input: the data-plane socket can never grant `admin.*`, regardless of `client.kind` or requested capability names. The admin socket never grants an EndpointId lease and is not used for ordinary direct/broadcast message delivery.

## Security boundary

The daemon creates the runtime directory owner-only (`0700` on Unix) and both sockets owner-only (`0600` equivalent where applicable) by default. Peer credentials are a **MUST** on Unix (§Peer identity, lock and stale sockets): the peer uid equals the runtime directory's owner uid, or the connection is closed before `hello` (A 2026-09-28). Deployments may apply a stricter owner/group/service-account ACL to the admin socket than to the data socket. The split is an enforceable protocol/capability boundary against client-kind spoofing and accidental privilege crossover, but default same-UID filesystem permissions still do **not** protect against a malicious process already running as the same OS user. Strong same-user executable/user-presence authentication remains SPIKE-005 territory.

Private identity keys never cross IPC.

## Framing

Each frame:

```text
4-byte unsigned big-endian length N
N bytes UTF-8 JSON object
```

- `N` maximum: **131,072 bytes (128 KiB)**; the 4-byte prefix is outside `N`;
- zero length is invalid;
- invalid UTF-8/JSON/version closes the offending client after a structured protocol error when possible;
- application payload bytes are base64url in JSON frames;
- strings and diagnostics have individual length caps.

### Payload-fit invariant

Every payload legal under the transport contract must be representable in both an IPC command and IPC `MessageReceived` event without exceeding the frame ceiling. 49,152 payload bytes expand to 65,536 base64url characters before JSON envelope overhead, so the 128 KiB body remains mandatory.

Phase 1 compatibility fixtures include both directions with exactly 49,152 opaque bytes and maximal bounded v2 metadata, including 64-character source/destination EndpointIds.

Envelope limits:

- `media_type`: 128 characters;
- `ChannelId`: 128 characters;
- `EndpointId`: 64 characters;
- normalized PeerId / transport identity string: the `common/peer-id`
  grammar (46 or 52 characters); the 256 an implementation may reserve
  (`TransportIdentity::MAX_BYTES`) is a code ceiling, not a schema bound;
- request/error diagnostic code: 128 characters;
- human-readable diagnostic message: 2,048 characters;
- client version string: 128 characters.

Every bound on a string in this contract is in **characters** — Unicode
code points, the unit JSON Schema's `maxLength` counts — so the schema
is the one authority for it and this document states no second unit
(A 2026-09-29). For the ASCII-patterned fields characters and bytes
coincide; for every string without an ASCII pattern (the diagnostic
message, the client version and kind, feature names, the request id,
`reason_class`) a maximal value occupies at most four bytes per code
point as raw UTF-8 — 8 KiB for the message — and at most twelve when
a writer escapes every code point (`\uD83D\uDE00`), 24 KiB, both inside
the 128 KiB frame ceiling, which remains the only bound in bytes. An
implementation sends and reads by the character count; the Rust
mirrors that still count bytes — the client version, the client kind on
the paths that bound it, feature names, and the message's send-side cut
(`Close::with_message`) — move to characters on the B1 pull request
(#147), where the request id, `reason_class` and the message's read
side already did.

## Handshake, endpoint claim, and client capabilities

Client first frame:

```json
{
  "type": "hello",
  "ipc_version": {"major": 3, "minor": 0},
  "client": {"kind": "human-client", "version": "..."},
  "endpoint": {"id": "12D3KooWD3eckifWpRn9wQpMG9R9hX3sD158z7EqHWmweQAJU5SA"},
  "requested_capabilities": ["events", "commands", "endpoints.query"],
  "features": ["keepalive"]
}
```

On the data-plane socket `endpoint` may be omitted. A connection that omits it holds no lease: it may hold `commands` and `events`, join, leave and publish; its `direct.send` is answered `EndpointNotRegistered` at the port, before the network, and no direct message is ever routed to it — the non-spoofable source of ADR-0030 is derived from the lease at the send, so a session without one has no source, not a forged one. A read-only diagnostics client is the same shape with `events` alone (A 2026-09-30). Administrative clients connect to the separate admin socket and MUST omit endpoint claims; the admin socket never owns an EndpointId lease.

Server validates endpoint claim before completing handshake. Phase 1 fixtures use these exact local error codes:

1. EndpointId grammar invalid -> `InvalidArgument` — a label of the grammar ADR-0067 retired and a local alias among them: the hello takes the derived id alone, and a client resolves an alias from the profile's configuration before it connects (`ENDPOINTS.md` §EndpointId);
2. configured endpoint does not exist -> `EndpointUnknown`;
3. configured endpoint exists but is disabled -> `EndpointDisabled`;
4. optional `allowed_client_kinds` rejects the declared client kind -> `EndpointClientKindDenied`;
5. another live lease already owns the endpoint -> `EndpointInUse`;
6. requested capability or connection authorization is denied -> `CapabilityDenied`.

The list names the codes, not the order they are judged in. The order
(A 2026-10-01, from proving the Stage 13 deferrals): a capability
above the minor the hello negotiates — the lower of the hello's and the
server's, for a major-3 hello — is `ProtocolViolation`, judged right
after the parse and before item 1 on either socket: a daemon that does
not know the name refuses it in the parse itself, one that knows it
refuses it in this step, so every daemon answers the same (A 2026-10-04,
§Version negotiation's capability rule; a hello that does not parse is
`ProtocolViolation` whatever its major, and an unsupported major that
parses is `VersionIncompatible`); within the negotiated minor, on
the data socket
the claim's grammar is read first (item 1), then the capability checks
that need no lease — `admin.*` requested on the data socket, or a claim
without `keepalive` where `require_for_endpoint_lease` holds, each
`CapabilityDenied` (item 6) — and only then the binding's claim: items
2, 3, 4 and 5 in that order; the capabilities themselves are then
judged per request, not at the handshake (`CapabilityDenied` on a
request a session's grant does not cover). So a hello claiming a malformed id and asking for
`admin.*` is `InvalidArgument`, and a well-formed claim of an absent
endpoint that omits `keepalive` is `CapabilityDenied`, not
`EndpointUnknown`. On the admin socket any endpoint claim within the hello's
length bounds is refused as `CapabilityDenied` before its grammar is
read, since no lease is ever
granted there (§Transport choice and authority domains). The claim's
id travels as the client wrote it (`ipc/hello` 1.2.0, and `2.0.0` on major 3 with the same bounds): the schema and
the Rust mirror bound its length alike (1 to 64 characters, a framing
error outside them on both), and within the bounds the handshake judges
its grammar, so a schema-driven server and the Rust mirror answer the
same code for the same well-formed JSON.

These are local IPC errors and intentionally more precise than the remote direct-protocol `no_route` privacy class. A remote peer never receives `EndpointUnknown`, `EndpointDisabled`, or `EndpointClientKindDenied`. If profile policy sets `ipc.keepalive.require_for_endpoint_lease=true`, a client that claims an EndpointId but did not negotiate `keepalive` is denied with `CapabilityDenied`; the daemon does not grant a lease first and revoke it later.

Server reply includes selected compatible IPC version, transport contract version, profile PeerId, caller endpoint (if any), a fresh local `endpoint_lease_epoch`, the granted event queue bound (`event_queue`, present with `endpoint`), and granted capabilities. `endpoint_lease_epoch` is an opaque **128-bit lease-generation value** unique to that grant across reconnects and daemon restarts (for example random, or daemon-instance nonce + counter). It is not a bearer credential; it exists only to invalidate stale local route/reply state.

Endpoint lease is exclusive and connection-bound. Client cannot change EndpointId on an established IPC connection. Rebinding requires reconnect/new handshake.

### Client-kind warning

`client.kind` is not cryptographic authentication. It can prevent accidental endpoint misbinding but cannot select the administrative authority domain. A client claiming `kind: transportctl` on the data-plane socket is still categorically ineligible for `admin.*`; a client on the admin socket is ineligible for EndpointId leases/data-plane messaging. Same-user code that can open the admin socket remains inside the documented residual boundary unless the deployment applies stricter OS ACLs or future SPIKE-005 authentication.

### Capabilities

- `events`: receive eligible runtime events;
- `commands`: the data-domain methods of the catalogue (`channel.join`, `channel.leave`, `broadcast.publish`, `direct.send`; from 2.11 `session.losses`); connectivity reaches a data client only as the normalized `server_state.connectivity` push, never as a method;
- `endpoints.query`: query a trusted remote peer's advertised endpoint directory;
- `admin.status`: read the administrative status view (`admin-status`: health, the full connectivity summary, counters, lease count) — read-only, admin socket only (A 2026-09-28); `ipc.events_dropped_total` is emitted only by a binding that keeps a per-client drop count; while none did, the member was omitted — a counter the server cannot keep is absent, never `0` (A 2026-09-29). The count is kept where the drop happens (A 2026-11-04, ruled with p2p-network-dev's batch that keeps it, #554): one counter for the profile, in the session queues' one push path, incremented for every copy a session queue drops under §Push events item 1 — its oldest, to take a newer one — or cannot queue, whichever session it was, joined or not, and whether or not that session is still open; monotonic since the daemon started, never reset while it runs, and `0` once it keeps the count and nothing has been dropped. It counts no refusal under item 2 (the sender was answered), nothing in item 3's reserved lane (never dropped), no held-back path notice (it waits), and nothing GossipSub or the ingress gate refused before admission; so it is the profile-wide total of every session's `queue_overflow` as `session.losses` would have read it, the sessions that joined nothing and the sessions since closed included, which the per-session ledger does not keep. No minor: the member has been an optional property of `admin-status` 1.1.0 since 2026-10-01, so a reader built against the schema accepts it, and a server that does not keep it still omits it. What the ingress gate refuses has a counter of its own beside it, `ipc.events_refused_total` (A 2026-11-10, `ipc/admin-status` 1.2.0, ADR-0026 A 2026-11-10; ruled on SPIKE-001's fact 79, where the gate refused 153 of 200 broadcasts and `admin.status` read no loss at all): one counter for the profile, moved where the broadcast ingress gate refuses an inbound broadcast — over the publisher's rate or the profile's shared ingress rate, or while draining — one per refused message, before dedup and before the fan-out, so a refusal counts once whatever the number of sessions on its channel, and counts on a channel no local session holds; monotonic since the daemon started, never reset while it runs, and `0` once the server keeps the count and nothing has been refused. It is a signal, as a session's `ingress_refused` is (§Push events and overload), never a count of distinct losses: a refused message leaves no dedup entry, so a republished copy that is refused counts again. It counts remote broadcasts only — a local publish refused while draining is the caller's error on that call — and nothing GossipSub dropped before the runtime saw the message. The two counters stay apart: a refusal never moves `events_dropped_total` and a queue drop never moves this one. It takes a minor, where its sibling took none: the member is new to the closed `ipc` block, and a reader built against 1.1.0 refuses an unknown member and would lose the whole status (ADR-0017 A 2026-10-07), so it is served on a connection that negotiated 2.13 or later, and below 2.13 the 1.1 block is served unchanged, without it. From 2.13 a server that does not keep the count omits it, so a 2.13 client tells absent, not counted, from `0`, none refused; a client below 2.13 learns neither. 2.13 adds no method and no capability.
- `admin.endpoints`: inspect/revoke local endpoint leases or change the endpoint overlay (enable/disable, default, and from 3.0 add an endpoint, remove an added one, set `advertise` and rename an added one's alias) through an administrative adapter;
- `admin.shutdown`: invoke transport `shutdown(grace)`.
- `admin.peers` (approved 2026-10-21, ADR-0062; its methods `admin.peers.add_address`, `admin.peers.remove_address` and `admin.peers.list_addresses`, their params and results and the minor that brings them are p2p-network-dev's rows under ADR-0017's rule): add a bootstrap address — a `multiaddr-with-peer-id` validated by the configuration's grammar, the method itself refusing by name every special-use class of ADR-0052 rule 2 in any component (private ranges admitted), the 65th entry, and any add while the profile authors the static-bootstrap provider `enabled: false` — to the profile's bootstrap-peers overlay, or remove one (an authored entry refused by name), or list the overlay's entries, each marked when the provider is disabled (ADR-0028 A 2026-10-21 (ii)); a DNS name that arrived by scan is the client's to refuse, since the method carries no provenance; admin socket only, never on the data socket under any `client.kind`; `admin.peers.list` (2.2) stays under `admin.status`.
- `admin.trust` (2.1, A 2026-10-03): read the profile's peer trust policy and mutate it through an administrative adapter — admin socket only, never on the data socket under any `client.kind` (ADR-0037 A 2026-10-03; ADR-0032: trust mutation requires the platform admin binding).

`claude-channel` is never granted `admin.endpoints` or `admin.shutdown`. A human UI data-plane connection is likewise non-admin; its settings/control surface opens the separate administrative socket. The data-plane socket rejects every `admin.*` request with `CapabilityDenied` before dispatch even if `client.kind` claims an administrative name.

Unknown/unauthorized capability requests are not silently elevated. Default grant policy is:

- `human-client`: `events`, `commands`, and `endpoints.query` when the profile endpoint-directory feature is enabled;
- `claude-channel`: `events` and `commands`; **not** `endpoints.query` by default;
- diagnostics clients: only explicitly configured read-only capabilities;
- administrative/control clients: `admin.*` capabilities only on the admin socket and only when local policy/OS access permits the administrative connection; the admin socket does not grant `events`/`commands` for application messaging or EndpointId leases.

A future Claude `peer_endpoints` tool therefore requires an explicit capability-policy and tool-surface security review rather than inheriting access accidentally.

## Message classes

- `hello` (the client's first frame) and `hello_response` (server-only)
- `close {code, message?, supported?}` (server-only; connection-fatal)
- `request {id, method, params, deadline_ms?}` — `method` from the closed catalogue below
- `response {id, ok, result? | error?}`
- `cancel {id}`
- `event {sequence, event_type, data}` — `event_type` from the closed event catalogue below
- `server_state {health, connectivity?}`
- `ping {nonce}` / `pong {nonce}`

One envelope, [`schemas/ipc/frame.schema.json`](./schemas/ipc/frame.schema.json) 2.1.0, covers all ten classes.

Request IDs are unique per connection. A request whose id is still outstanding on the connection — in flight or waiting — is a protocol violation: the server answers `close{ProtocolViolation}` and closes, since no response bearing that id could be told from the first's (A 2026-09-29). Event sequence is per IPC connection for diagnostics/gap detection only; it is not a durable replay cursor.

A request whose method requires an ungranted capability fails locally with a stable authorization error and is not dispatched to the transport runtime.

## Multiple clients

The daemon supports up to **16 IPC connections total** by default across both sockets, with at most **4 admin-socket connections** by default. The limit counts connections, not applications: if a human application opens one data-plane connection and one administrative connection, it consumes **two** total slots. Each connection has independent bounded request/event state. One slow client cannot backpressure the entire network event loop. A connection from the owner's uid accepted past either limit is answered `close{Overloaded}` before any hello is read; nothing about it is read first, and the peer-credential check of §Peer identity comes before the limits, so a foreign uid is still closed silently (A 2026-09-29).

Each direct-capable client owns at most one EndpointId lease. Multiple local clients intentionally sharing a profile therefore use distinct endpoint IDs.

### Message-event routing

Local routing is normative:

- **broadcast `MessageReceived`:** enqueue only to connected IPC clients with `events` that currently hold a join reference for that ChannelId; `channels.desired` does not create local interest;
- **direct `MessageReceived`:** enqueue exactly one copy to the connected IPC client that owns the resolved `destination_endpoint` lease.

There is no first-client, round-robin, or all-client direct fan-out in this IPC.

If the resolved endpoint is not leased, the daemon rejects the inbound direct request with coarse remote `no_route`. It does not acknowledge then drop, and it does not buffer for a future client.

## Local endpoint lease lifecycle

- a lease grant is learned from `hello_response` (endpoint + `endpoint_lease_epoch`); no event announces it;
- normal disconnect releases lease and all ephemeral join references;
- administrative revocation, or the endpoint being disabled, produces `endpoint.lease_changed {endpoint, revoked_epoch}` and stops direct routing immediately;
- endpoint configuration disable/reload revokes an active lease;
- a second live claim returns `EndpointInUse`;
- a reconnect receives a new lease epoch;
- no stale local reply route may authorize a new connection merely because it later claims the same EndpointId.

Bridge-local reply tokens disappear on bridge restart, so the Claude path naturally satisfies the stale-route rule. Other local apps must bind stored ephemeral reply routes to their current lease epoch.

## Direct command caller context

IPC `direct.send` params (`send-params` 2.0.0) carry the remote destination, the caller's message identity and the payload:

```json
{
  "peer": "...",
  "endpoint": "human",
  "message_id": "...",
  "payload": {"media_type": "text/plain", "bytes": "..."}
}
```

`endpoint` here is the **remote destination endpoint** and may be omitted to request the remote default endpoint. `message_id` is the caller's and REQUIRED (send-params 2.0.0): a retry after a lost response must carry the same identity or dedup cannot recognise it. There is no `source_endpoint` parameter. The daemon derives source from the caller's active lease.

A client without an endpoint lease receives `EndpointNotRegistered` for direct send.

## Push events and overload

Each client event queue defaults to 256. When full:

1. drop oldest ordinary broadcast events for that client as configured;
2. for an inbound direct message targeted at this endpoint, reject before transport `Accepted` if the event cannot be admitted;
3. preserve a reserved lane for `server_state`, `endpoint.lease_changed`, `peer.disconnected` and `close`, which are never dropped in favour of ordinary broadcast events;
4. increment drop/rejection counters — from 2.11 a session reads its own, below;
5. never spill into an unbounded disk queue.

A `peer.path_changed` notice (2.1, A 2026-10-03; from 2.4 also sent when the route BEGINS, with `previous` absent and `reason_class` `route_established`, and when a routed peer connects again after a `peer.disconnected`, `previous` absent and `reason_class` `reconnected`, A 2026-10-09 — a major-2 connection below 2.4 was sent nothing at route begin or at a reconnect, its session declining route notices and owed every change with its `previous`; no major-3 connection is below it and every session takes them) is in the ORDINARY lane
with a rule of its own: per peer at most one is pending; a newer one
replaces it, keeping the pending one's `previous` (an absent one stays absent) and taking the newer
`current` and `observed_at` (so a client never sees a `previous` it was not shown), and the replacement is counted; a merge whose `previous` equals its `current`
announces no change and is withdrawn, counted as a replacement (never a route_established notice, whose `previous` is absent); under
pressure a pending notice WAITS — it is taken after every message, under
what is left of the pump's room — and is never dropped, so a route
indicator stays stale until it is taken (A 2026-10-04, correcting A
2026-10-03's "dropped before any message", which the server never
implemented); it is never in the reserved lane of item 3. A route ends only at a revocation that changed the policy: `admin.trust.set` to `false` ends every connection's route to the peer and withdraws its pending notice, counted as a replacement (the composition's `Diagnostics::peer_notices.paths_replaced_total`, not on the IPC wire); a message from the peer taken afterwards, or a send whose acceptance is recorded afterwards, makes a new route; a disconnect ends none, but withdraws the peer's pending notice, counted as a replacement, so a `peer.path_changed` read after a `peer.disconnected` never names the connection that is gone (A 2026-10-05, #190; the withdrawal A 2026-10-05 (ii), #192; LOCAL-CLIENT.md §2). A connection is held to have a route to at most `MAX_ROUTED_PEERS` peers (the trust allowlist's own ceiling, `PeerTrustPolicy::MAX_ALLOWED_PEERS`): a route past it is counted (the composition's `Diagnostics::peer_notices.routes_refused_total`, not on the IPC wire) and not kept, so no notice is owed for that peer; the pending notices are held one per routed peer, apart from the ordinary queue and its bound — that, not a drop, bounds their memory (A 2026-10-04).

Over IPC the server pumps the session queue into its event lane and the socket, and the client into its own bounded buffer, so what a sender can get accepted while the reader does not drain is the whole pipeline's capacity: the session queue, the event lane, the client's buffer, and the socket — whose share is the kernel's send buffer, bounded in bytes, not events, and therefore hundreds of small frames or a handful of large ones. Bounded, larger than one `event_queue`, and no number this contract states. Acceptance still follows admission at the session queue and every accepted message is held and delivered to a client that keeps draining; a client the server closes as wedged loses what the pipeline held for it, bounded as above and never silently to that client — its session ends, and `CHANNEL-EVENT.md` §Delivery says what a bridge tells its host; the sender, already answered Accepted, is not told (A 2026-10-08, from ADR-0002 A 2026-10-07); nothing is buffered anywhere a bound does not name (A 2026-09-30).

What a session lost is readable by the session: `session.losses` (data socket, `commands`, no params, `ipc/session-losses-result` 1.0.0, from 2.11; ADR-0026 A 2026-10-24) answers two counts — `queue_overflow`, the copies item 1 dropped from this connection's session queue (its oldest, to take a newer one) or could not queue, and `ingress_refused`, the copies the ingress gate refused (over the publisher's rate or the profile's shared ingress rate, or while draining) on a channel the session held at the time, counted from each join. Both are monotonic for the connection's life — a leave and a rejoin lower neither — and neither counts what GossipSub dropped before admission (validation, the mesh, the backend's queue) or a conflicting body, so zero means none counted, never none lost; a session that joined nothing reads zeros. The two counts give different guarantees, and a consumer keeps them apart. An overflowed broadcast was ADMITTED: its dedup entry is recorded after the fan-out whatever was dropped, so within the dedup window (ADR-0019; 10,000 entries, 5 min, oldest evicted) a republish of the same id is a duplicate — `queue_overflow` counts broadcasts lost, not delayed, for that window, and a lower bound on them. A refused broadcast was NOT admitted: the ingress gate refuses before any dedup entry exists, so a republish of the same id may still arrive and be delivered, and one broadcast refused several times counts each time — `ingress_refused` counts refused COPIES on the session's channels, a signal that broadcasts were refused there, never a count of distinct broadcasts lost (the alternative, recording a refused id so a republish is dropped, would make a loss permanent to keep the number honest, and spend at the gate the work the bucket exists to bound; rejected, ADR-0026 A 2026-10-24). The answer names no peer and no channel. Without `commands` it is `CapabilityDenied`. Below 2.11 a client does not send it and answers `ProtocolUnsupported` itself, as a 2.10 server would answer an unknown method — and on major 2 a session that declined route notices was one of those: over IPC the way to decline them was to speak below 2.4 (the client's negotiation, #245 N5), so such a session read no losses. On major 3 no IPC session declines them (§Version negotiation and phases, the major's rules), so every IPC session reads its losses, and a consumer that does not want the notices takes them and discards them. No production client declines route notices today; a client that one day must decline them over IPC makes declining a `hello` feature of its own, decided then, never a cap on the minor. What the pipeline past the session queue held for a client the server closed as wedged is not in either count: that loss ends the session (above).

Event order over IPC: within one server pump the grouped order of `events()` holds (session notices, then direct, then broadcast, each oldest first, then the pending path notices under what room is left — A 2026-10-04); across pumps the client reads batches as they arrive, so a notice pumped after a direct message follows it. A consumer that needs one order across a session uses the receipt times a direct message and a broadcast carry; a notice carries none and is read as of its arrival (A 2026-09-30).

## Disconnect/reconnect and optional keepalive

A client disconnect releases its EndpointId lease, ephemeral subscription references, and outstanding response waiters. Reconnect performs a fresh handshake and resubscription. There is no event replay. A late response to a disconnected client is discarded after internal cleanup.

The IPC also supports an optional negotiated liveness feature for detecting half-open/wedged clients that still hold endpoint leases:

```text
server -> ping { nonce }
client -> pong { nonce }

nonce = 128-bit CSPRNG value, encoded canonically (for example base64url without padding)
```

When enabled by profile policy and negotiated in `hello`, defaults are `interval=30s`, `response_timeout=10s`, `max_missed=3`. The server has at most one outstanding keepalive nonce per connection; only an exact pong for the current 128-bit nonce satisfies the probe. Stale/duplicate/wrong nonces do not reset liveness state. After the configured miss threshold the daemon closes that IPC connection and releases its endpoint lease exactly as for an ordinary disconnect. Keepalive is local liveness detection only: it is not authentication, replay protection for application messages, a network heartbeat, or a lease-renewal credential.

The bound runs both ways (A 2026-10-08). The wire tells the client neither the three values nor whether keepalive was granted (`hello_response` has no keepalive members, and a client offers the feature in every hello), so the client's bound is a protocol constant that ARMS AT THE FIRST PING THE CLIENT READS: `CLIENT_SILENCE_TIMEOUT = 120 s`. A server that has pinged has negotiated keepalive and pings every `interval` while healthy, so 120 s without a frame read after that is a real silence; before the first ping only the caller's own timeout and OS connection liveness bound the wait, as on a connection without keepalive — a daemon whose profile turns keepalive off never pings, and never arms the timer. Armed, a client that has read no frame of any kind for 120 s ends the connection `Timeout`, answering its waiting calls `Timeout`. The constant sits above the server's own threshold for a wedged client — `max_missed × interval + response_timeout`, 100 s at the defaults (probes every `interval`, a miss counted at `response_timeout` after each) — so at the defaults the server judges first; a profile raising `max_missed` can place its own threshold above the constant, which the client's armed timer, suspended while the client is not reading, does not contradict. The timer counts frames the client reads, so a client that has stopped reading its socket (its buffer full, below) suspends it: that client's fate is the server's wedge close, not a timeout it blames on the server. A profile keeps `interval + response_timeout` at or below the constant, so a healthy server's next ping always lands inside the armed bound (a validation rule in `profile-config`; `resource-limits.md`'s keepalive rows carry it). The validation landed with #228 (241b7598, 2026-10-08): `profile-config` refuses a profile whose `interval + response_timeout` exceeds `CLIENT_SILENCE_TIMEOUT_MS` (120 000) while keepalive is enabled, naming both values. The client's timer landed with #230 (2373d967): `ipc-protocol`'s `CLIENT_SILENCE_TIMEOUT`, armed at the first ping read, as above.

The profile policy `ipc.keepalive.require_for_endpoint_lease` defaults to `true`. When true, any client that claims a data-plane EndpointId lease must negotiate keepalive during `hello`; otherwise endpoint claim fails with `CapabilityDenied`. Connections that do not claim an endpoint (for example a separate admin or diagnostics session) do not need keepalive solely because of this rule. Operators may set the policy false for compatibility with third-party clients, accepting that a half-open client may retain its lease until OS-level failure detection or explicit `admin.endpoints` revocation.

An IPC client's receive buffer is bounded at its granted `event_queue` plus the one event its reader holds while it pauses; a client whose buffer is full stops reading its socket, so responses wait behind undrained events and, past the keepalive miss threshold, the server closes it as wedged. Draining events is part of holding a lease (A 2026-09-30).

## Cancellation

Cancel is advisory. If an operation has crossed an irreversible network boundary, completion may race cancellation. Responses distinguish `CancelledBeforeDispatch` from `CancellationRaced` where observable.

## Method catalogue

Every request names one method of the closed catalogue
[`schemas/ipc/method.schema.json`](./schemas/ipc/method.schema.json);
[`schemas/ipc/request.schema.json`](./schemas/ipc/request.schema.json)
binds each name to its params shape. A name outside the catalogue is
answered `ProtocolUnsupported` and the connection stays; a name of the
admin domain arriving on the data socket is answered `CapabilityDenied`
before dispatch, whatever the client's kind, and counted
(`ipc_cross_domain_capability_denied_total`). The capability a method
needs is this table's and the mirror table in `crates/api/ipc-protocol`;
the schema-agreement test binds the two.

| Method | Domain | Capability | Params | Result | Since |
|---|---|---|---|---|---|
| `channel.join` | data | `commands` | `channel-params` | `empty-result` | 2.0 |
| `channel.leave` | data | `commands` | `channel-params` | `empty-result` | 2.0 |
| `broadcast.publish` | data | `commands` | `publish-params` | `empty-result` | 2.0 |
| `direct.send` | data | `commands` (a lease required: `EndpointNotRegistered` otherwise) | `send-params` | `send-result` | 2.0 |
| `endpoints.query` | data | `endpoints.query` | `query-params` | `endpoints:directory-response` | 2.0 |
| `admin.status` | admin | `admin.status` | none | `admin-status` | 2.0 |
| `admin.endpoints.list` | admin | `admin.endpoints` | none | `endpoint-list` | 2.0 |
| `admin.endpoints.revoke` | admin | `admin.endpoints` | `endpoint-params` | `empty-result` | 2.0 |
| `admin.endpoints.set_enabled` | admin | `admin.endpoints` | `set-enabled-params` | `set-enabled-result` | 2.0 |
| `admin.endpoints.set_default` | admin | `admin.endpoints` | `set-default-params` | `empty-result` | 2.0 |
| `admin.endpoints.add` | admin | `admin.endpoints` | `endpoint-add-params` | `endpoint-add-result` | 3.0 |
| `admin.endpoints.remove` | admin | `admin.endpoints` | `endpoint-params` | `endpoint-remove-result` | 3.0 |
| `admin.endpoints.set_advertise` | admin | `admin.endpoints` | `set-advertise-params` | `empty-result` | 3.0 |
| `admin.endpoints.set_alias` | admin | `admin.endpoints` | `set-alias-params` | `set-alias-result` | 3.0 |
| `admin.shutdown` | admin | `admin.shutdown` | `shutdown-params` | `empty-result` | 2.0 |
| `admin.trust.list` | admin | `admin.trust` | `trust-list-params` | `trust-list` | 2.1 |
| `admin.trust.set` | admin | `admin.trust` | `trust-set-params` | `trust-set-result` | 2.1 |
| `admin.trust.seed` | admin | `admin.trust` | `trust-seed-params` | `trust-seed-result` | 3.0 |
| `admin.peers.list` | admin | `admin.status` | `peer-list-params` | `peer-list` | 2.2 |
| `admin.peers.add_address` | admin | `admin.peers` | `peers-address-params` | `peers-address-result` | 2.7 |
| `admin.peers.remove_address` | admin | `admin.peers` | `peers-address-params` | `peers-address-result` | 2.7 |
| `admin.peers.list_addresses` | admin | `admin.peers` | none | `peers-address-list` | 2.7 |
| `admin.peers.own_addresses` | admin | `admin.peers` | none | `peers-own-addresses` | 2.8 |
| `admin.peers.add_addresses` | admin | `admin.peers` | `peers-addresses-params` | `peers-addresses-result` | 2.9 |
| `session.losses` | data | `commands` | none | `session-losses-result` | 2.11 |

`admin.endpoints.set_enabled(false)` revokes a live lease at once
(`endpoint.lease_changed`) and never auto-rebinds. `set_enabled` and
`set_default` are a **persisted overlay** (ADR-0028 A 2026-10-11): never
written to `config.yaml`, they are kept in the state directory's
endpoint overlay, written before the change is published and before the
set is answered, so a restart starts with them. A set whose result,
composed over `config.yaml`, would fail the profile's validation -- a
rule reading the enabled set, such as `directory.max_advertised` -- is
refused `InvalidArgument` with nothing written, as the next start would
refuse it; `set_default` naming an endpoint that is not configured or is
disabled is `EndpointUnknown` or `EndpointDisabled`. Neither set is
audited, unlike `admin.trust.set`. `admin.endpoints.revoke`
stays runtime-only, a lease being a session's and not configuration.
`admin.endpoints.list` answers each endpoint's effective state, and on a
connection that negotiated 2.5 or later `persisted: true` on every row (a
row the runtime cannot persist is answered `Internal` there, a state only
a store-less runtime reports, never served over IPC); below 2.5 the 2.0
row, `persisted: false`, is served unchanged (`ipc/endpoint-list` 1.1.0,
ADR-0017 A 2026-10-07: a closed result shape widens behind a new minor;
2.5 adds no method or capability). Until 2026-10-11 all three were a
runtime overlay, a restart returning to the configured state, and every
row said `persisted: false`. Trust administration
(ADR-0032) arrived in 2.1 (A 2026-10-03, Stage 15's R2): the two
`admin.trust.*` rows above moved into the table from an approved list,
with their schemas and the Rust mirror, in the batch that implements
them; the list method's params were `none` there and are a page cursor
here (architect-cto's ruling of 2026-10-04, below).

`admin.peers.list` (2.2, A 2026-10-06, `CONNECTIVITY.md` §19) moved into the table with the batch that implements it, as `admin.trust.*` did: under the `admin.status` capability, the dial gate's state for each peer of the admission set (A 2026-11-24; ADR-0028 A 2026-10-28, `ENDPOINTS.md`, "Trust is per endpoint": the union of the operator's list and every endpoint's overlay set, which is what the gate admits, so a peer that an identity's set alone holds has its row, its dial outcomes and its connectivity lines as a peer of the operator's list has; the text said "each allowlisted peer" while the node held one list, and read after per-endpoint trust as the operator's list it left such a peer connected with no row; a row does not say which set holds the peer, and `admin.trust.list` without an endpoint is still the operator's list) — `{peer, connected, backoff_until?, quarantined_until?, route_held_until?, last_outcome?}` (`route_held_until` since A 2026-10-21: the latest hold among the peer's circuit routes whose dial failed at its relay hop, ADR-0011 A 2026-10-21 — apart from the other two because a route hold never backs the peer off; served on a connection that negotiated 2.6 or later, and below 2.6 the 2.2 row, without it, is served unchanged — `ipc/peer-list` 1.1.0, ADR-0017 A 2026-10-07: a closed result shape widens behind a new minor, since a 2.2 client refuses an unknown member and would lose the whole page; 2.6 adds no method or capability), the three times in milliseconds since the Unix epoch, `last_outcome` absent until the peer's first dial or connection since the daemon started — never an address. `last_seen_ms` (A 2026-10-28, `ipc/peer-list` 1.2.0, for the fabric's control plane: an unreachable host told apart from a silent agent, ADR-0055 decision 5b's hosts; served on a connection that negotiated 2.12 or later and absent below it, where the 1.1 row is served unchanged, so a 2.12 client tells `null`, never seen, from absent, not reported): the last moment a retained connection to the peer was open, in milliseconds since the Unix epoch on the daemon's wall clock — the time of the reading while `connected` is true, else the moment the last one closed — one thing, the connection's lifetime, neither the last frame, which only the muxer sees, nor the last establishment, which would age a connection up for a week; `null` when the peer has had no retained connection since the daemon started, since the daemon keeps it in memory only and a restart reads `null` until the peer connects again; kept in the substrate, where a retained connection leaves the open set, bounded by `MAX_ALLOWED_PEERS` (4096) plus the infrastructure set's 256, the peer closed longest ago dropped past that. From 2.12 the row is `{peer, connected, backoff_until?, quarantined_until?, route_held_until?, last_outcome?, last_seen_ms}`. It is paged as `admin.trust.list` is (`{after?: peer-id}` → `{peers, next?}`, ascending by canonical string), at most 512 rows a page (`MAX_PEER_PAGE_ROWS`; ruled 2026-10-07 on p2p-network-dev's measurement: a peer-list row is up to 194 bytes of compact JSON with every field present — about 220 since `route_held_until` — so a trust-list page's 1024 would be about 200 KB, since 225 KB, against the 128 KiB body ceiling, `MAX_BODY_BYTES`; 512 is about 100 KB worst case, since 110 KB, and `MAX_ALLOWED_PEERS` at 4096 makes at most eight pages; from 2.12 a row carries `last_seen_ms` too, about 270 bytes at the largest times, so 512 such rows measured 138,825 bytes, over the ceiling, and the server puts at most 456 rows on a page, about 124 KB, an admission set of 4096 being nine pages, a reader still accepting 512 and the schema's `maxItems` unchanged, A 2026-10-28). Its schemas `peer-list-params` and `peer-list` (1.2.0 since `last_seen_ms`, which 2.12 brings, A 2026-10-28; 1.1.0 since the route hold's field, which 2.6 brings) are `approved`, and the method and request enums carry its minor (`ipc/method` 1.2.0, `ipc/request` 1.2.0). A client requests it only on a connection that negotiated minor 2.2 or later; `admin.status` being a 2.0 capability, the capability rule below does not apply, so the client reads the minor from that connection's own `hello_response` and does not send the method below 2.2, where the server answers it as an unknown method (§Version negotiation).

`admin.peers.add_address`, `admin.peers.remove_address` and `admin.peers.list_addresses` (approved 2026-10-22, ADR-0062, p2p-network-dev's rows under ADR-0017) are the bootstrap-peers overlay's methods (ADR-0028 A 2026-10-21 (ii)), under the `admin.peers` capability on the admin socket only, and come with minor **2.7**: a client sends them only on a connection whose `hello_response` granted `admin.peers`, which a server grants only at 2.7 or later (the capability rule, §Version negotiation). They moved into the method table above, and their names into the method, request and capability enums (`ipc/method` 1.3.0, `ipc/request` 1.3.0, `ipc/capability` 1.3.0), with the batch that implements them, as `admin.trust.*` and `admin.peers.list` did. `add_address` and `remove_address` take `peers-address-params` (`{address}`, a `multiaddr-with-peer-id` in the configuration's grammar, at most 1024 characters) and answer `peers-address-result`: `{outcome: added | removed | unchanged | refused, refused?, detail?}`. **A refusal is an answer, not an error**, so a client can say why in its own words without parsing a message: `refused` is one of `grammar` (not the grammar), `own_peer` (an add of an address whose `PeerId`, the node it reaches, is this profile's own: a row nothing can dial, refused by the method after the grammar whoever the caller; from minor 2.9, `ipc/peers-address-result` 1.1.0, and below it the same add is answered `grammar`, the reason an older client's decoder knows), `special_use` (a component, the relay's prefix included, in a class ADR-0052 rule 2 refuses -- loopback, link-local, unspecified, multicast or another special-use range; private ranges admitted -- checked by the method whoever the caller, `transportctl` included), `bound` (authored and added entries would pass 64), `provider_disabled` (an add while the profile authors the static-bootstrap provider `enabled: false`), `authored` (a removal of a `config.yaml` entry) or `superseded` (a write every other rule admits, from an admin connection admitted before the one whose write for the same address the daemon last applied: the later connection's write stands, §Cancellation mapping and request concurrency; from minor 2.10, `ipc/peers-address-result` 1.2.0, and below it the same write is the error `Internal`, nothing changed, since no older decoder knows the name and every older client reads `Internal` as not applied), and nothing is written; `unchanged` is an add of an address already present or a removal of one the overlay does not hold; `detail` is for a log. A write that fails is the error `Internal` with nothing changed: the overlay is written before the change reaches the running provider and before the answer (ADR-0028). `list_addresses` takes the empty object and answers `peers-address-list`: `{addresses: [{address, source: authored | added, dialled, not_dialled?: provider_disabled}]}`, the authored entries first and then the added ones, each in the provider's order, at most 64 rows, so one answer and no paging. An add or a removal takes effect in the running provider without a restart. The typed-only rule for a DNS name is the client's, since the method carries no provenance (ADR-0052 rule 9). The three schemas are `approved`.

`admin.peers.own_addresses` (approved 2026-10-22, ADR-0062 decision 4, p2p-network-dev's row under ADR-0017) is the show side's read: under the `admin.peers` capability on the admin socket only, with minor **2.8** — a client sends it only on a connection that granted `admin.peers` AND negotiated 2.8 or later, since `admin.peers` is granted from 2.7 and a 2.7 server answers it as an unknown method; it moved into the method table above, and its name into the method and request enums (`ipc/method` 1.4.0, `ipc/request` 1.4.0), with the batch that serves it. It takes the empty object and answers `peers-own-addresses`: `{peer, addresses: [{address, kind}], truncated}`, ONE ordered list in decision 4's order, which a client renders as it comes and never filters or reorders: every confirmed external address (`confirmed_external`: AutoNAT-verified, `CONNECTIVITY.md`, ADR-0035) in the order it was confirmed, an address re-verified keeping its place; then, only when the node is relay-only — ADR-0062 decision 4's definition: no externally confirmed address exists at the read, whether or not the floor below would show it — each circuit route through a reservation the node holds as active (`relay_circuit`), in the relay manager's order; then every listen address bound on an interface the host still holds (`listen`), in bind order. A listener's circuit address is never a `listen` row. Every row is in the classes the add admits, the floor applied by the transport so the client filters nothing: it parses by the configuration's `multiaddr-with-peer-id` grammar and ends in `/p2p/<peer>`, `peer` being this node's, and no component is in a special-use class of ADR-0052 rule 2 (loopback, link-local, unspecified, multicast or another special-use range; private ranges admitted); a bound address outside them is omitted, not marked. At most 64 rows (`MAX_OWN_ADDRESSES`): past them the rest is dropped in that order and `truncated` is `true`. The QR's 16-line cut is the client's, from the head of the list. An empty list is an answer — no admitted listener, no confirmation, no reservation — and so is its absence of external rows: whether AutoNAT has concluded is `admin.status`'s `connectivity`, not this read's. Reading it causes nothing (no probe, reservation or dial). The schema is `approved`.

`admin.peers.add_addresses` (approved 2026-10-22, ADR-0062 decision 4, p2p-network-dev's row under ADR-0017) is the all-or-none add a scan and a pasted list of lines need, since N calls of `add_address` cannot be all or none: a line refused after others were written leaves them written, and a kill between calls leaves part of the list. Under the `admin.peers` capability on the admin socket only, with minor **2.9** — a client sends it only on a connection that granted `admin.peers` AND negotiated 2.9 or later, a 2.8 server answering it as an unknown method; it moved into the method table above, and its name into the method and request enums (`ipc/method` 1.5.0, `ipc/request` 1.5.0), with the batch that serves it, and `add_address` stays as it is. It takes `peers-addresses-params`, `{addresses}`: 1 to 16 strings in the order given, each as `add_address`'s `address`; the 16 is the QR's cut, and which lines a client keeps — trimmed, empty lines dropped, a DNS name from a scan refused — is the client's (ADR-0052 rule 9). It answers `peers-addresses-result`. **The whole list is checked before anything is written**, in `add_address`'s order so that a one-line list is answered as `add_address` answers that line: every line's grammar; then every line's `PeerId` against this profile's own (`own_peer`); then the provider (`provider_disabled`, which refuses the list whole); then every line's special-use floor (ADR-0052 rule 2, private ranges admitted, applied by the method whoever the caller); then the bound, counting authored entries, added entries and the list's new lines, a line already present (authored or added) or repeated earlier in the list counting nothing; then, from minor 2.10, every line's `superseded` (as `add_address`'s, below 2.10 the error `Internal`; `ipc/peers-addresses-result` 1.1.0). A refusal is an answer, not an error, and nothing is written: `{outcome: refused, refused, index?, detail?}`, `refused` being `add_address`'s set less `authored` (`grammar`, `own_peer`, `special_use`, `bound`, `provider_disabled`, `superseded`) and `index` the 0-based position of the line it names — the first line that fails the rule in force, for `bound` the first new line past 64 — absent exactly for `provider_disabled`. Otherwise `{outcome: added, lines}`, one row per line in the order given, each `{outcome: added | unchanged}`, `unchanged` for a line already present or repeated; a list of lines all present answers `added` with every row `unchanged`. The accepted lines are written in ONE overlay write before the running provider and the operator's door are told and before the answer, so a kill leaves all of them or none; a write that fails is the error `Internal` with nothing changed (ADR-0028). The two schemas are `approved`.

`admin.trust.list` answers the profile's allowlist as `trust-api`'s `PeerTrustPolicy` holds it, ONE PAGE at a time: the allowed peers in the
ascending order of their canonical strings, at most 1024 a page, with
on a connection that negotiated 2.3 or later `persisted: true` on every
row (a row the runtime cannot persist is answered `Internal` there, a
state only a store-less runtime reports and `LOCAL-CLIENT.md` §7 item
11 never serves over IPC) and each row's `source` — `configured` for a peer `config.yaml`
lists, `administered` for one added by `admin.trust.set` (ADR-0028 A
2026-10-07) — and below 2.3 the 2.1 row, `persisted: false` and no
`source` (ADR-0017 A 2026-10-07, §Version negotiation: a closed result shape
widens behind a new minor while the old shape is served below it; 2.3
adds no method or capability), and `next` — the last row's peer — when
more remain; `trust-list-params.after` names the previous page's `next`,
exclusive, and is absent for the first page. The cursor is a position,
not a row, so the server keeps no state between pages and an `after`
naming a peer no longer listed still answers the page that follows it.
Pages are read against the live policy, not a snapshot: a set between
two reads may show or hide a peer across the boundary. The local peer —
self-authorised, never an allowlist entry — is the first page's
`local_peer`, absent on later pages and when the policy has none bound.
A page because the allowlist holds up to
`PeerTrustPolicy::MAX_ALLOWED_PEERS` (4096) peers, about 420 KB as
2.3 rows (a row with a 52-character peer id is 104 bytes with `source`,
105 with its array comma; 81 bytes as the 2.1 row, 82 with the comma),
against this protocol's 128 KiB body; a page of 1024 is about
105 KiB, so a full allowlist is four requests
(`a_full_trust_page_of_the_largest_rows_fits_the_body` pins the page). Every peer not listed is
denied (deny-by-default is the policy's shape, not a setting; there is
no default to report and no `TrustDecision` on the wire — that enum and
its `DenyReason` are local diagnostics). `admin.trust.set` takes one
`peer` and `allowed: true | false`: `true` adds the peer to the allowlist
and is refused with `InvalidArgument` past `PeerTrustPolicy::MAX_ALLOWED_PEERS`,
for the local peer, or — while an overlay is ahead of the runtime
(ADR-0028) — when the policy's bound admits it today but the overlay
on disk would exceed `MAX_ALLOWED_PEERS` at the next start (audited
`refused`, nothing written); `false` removes it, closes every connection
the peer holds at once, drops its cached directory
(`DirectoryCache::forget`, §16's carry), and every connection with
`events` sees `peer.disconnected` with `reason_class: policy` (below). Until the Stage 22 release every revoke does this, the node holding one list; from that release the close, the dropped directory and the event come when the write removes the peer from the admission set, and a revoke that leaves it in another set does none of them (ADR-0067 decision 6, A 2026-11-19).
Endpoint narrowing (`EndpointTrustPolicy`) was not reachable through these
methods until the Stage 22 release; from it the endpoint dimension below reaches it. Each set that reaches the runtime is recorded
under the audit log target (`interweave::audit`: peer, `allowed`, its
outcome, time) so trust changes can be audited (ADR-0012's consequence);
the record is the composition's and binds every host (LOCAL-CLIENT.md
§5, A 2026-10-04) — the daemon writes it to its log and admits the
target at INFO whatever `observability.log_level` says (#186). On Unix
every admin connection is the run-dir owner's (ADR-0037), so the log
says a set happened, not who among the owner's processes made it. A set that changes the policy is written to the
state directory's trust overlay (`<state>/trust-overlay.json`, ADR-0028 A
2026-10-07: one of four moves on the normalised `added`/`revoked` lists)
before the new policy is published to the runtime and before the set is
answered, so an answered set survives a restart and a crash; a set whose
write fails is answered `Internal` and changes nothing — no connection
closes, no row moves, and the audit line's outcome is `unwritten`; a
publish that fails after the write, or a directory sync that fails after
the rename, restores the previous overlay before the set is answered
failed; when that restore fails too the overlay is ahead of the runtime
(answered `Internal`, audited `failed`, logged) and the set — an allow as
much as a revocation — takes effect at the next start (ADR-0028). Adding a peer already listed and removing one not
listed are no-ops that answer `ok` — from 2.10 `trust-set-result`'s
`{outcome: set}`, and below 2.10 `empty-result`. From 2.10 a set the
policy admits, from an admin connection admitted before the one whose
set for the same peer the daemon last applied, is refused
`{outcome: refused, refused: superseded}` with nothing written, and
audited `superseded` (§Cancellation mapping and request concurrency);
below 2.10 the same set is the error `Internal`, nothing changed, the
answer every older client already reads as not applied
(`trust-set-result` is `approved`). Both methods are granted only to a
connection that negotiated minor 2.1 or later, and `admin.trust` is
requested only in a hello sent after the client has learnt the daemon
speaks 2.1 (§Version negotiation's capability rule); the `close` frame's
`supported` list was `[{major: 2, minor: 1}]` from R1, `[{major: 2,
minor: 2}]` from `admin.peers.list` (A 2026-10-06), `[{major: 2,
minor: 3}]` from the trust overlay (A 2026-10-07), `[{major: 2,
minor: 4}]` from the route notices (A 2026-10-09), `[{major: 2,
minor: 5}]` from the endpoint overlay (A 2026-10-11), `[{major: 2,
minor: 6}]` from the route-hold peer row (`ipc/peer-list` 1.1.0,
`route_held_until`, ADR-0011 A 2026-10-21), `[{major: 2,
minor: 7}]` from the `admin.peers` address methods (ADR-0062),
`[{major: 2, minor: 8}]` from `admin.peers.own_addresses` (ADR-0062
decision 4), `[{major: 2, minor: 9}]` from
`admin.peers.add_addresses` (ADR-0062 decision 4), and is
`[{major: 2, minor: 10}]` since the superseded refusal (§Cancellation
mapping and request concurrency): it names the
server's `IPC_MAX_MINOR`, never a literal. The two were the
same runtime overlay as `admin.endpoints.*` then was — never written to
`config.yaml`, `persisted: false` — until the owner decided persistence
on 2026-10-07 (ADR-0028's question, routed with the Stage 15 record):
since then a set is a persisted overlay in the state directory, still
never `config.yaml`; the row says so behind minor 2.3 (`ipc/trust-list`
1.1.0: `persisted` a boolean and `source` optional, each shape named
with the minor that serves it), the 2.1 row unchanged below, the
`close` frame's `supported` list `[{major: 2, minor: 3}]` from the batch
that implements it, its mirror and the daemon changing in one PR. The
client's half (A 2026-10-07): a binding that fills the neutral trust row
(`LOCAL-CLIENT.md` §7 item 11: `persisted` and a `source` every listed
peer has) reads `admin.trust.list` only on a connection that negotiated
2.3 or later and refuses the read below it with `ProtocolUnsupported`,
no round trip — as `admin.peers.list` is refused below 2.2 — because the
2.1 row carries no `source` and an unknown is never shown as a value;
`admin.trust.set` carries no row and its params are unchanged from 2.1, its result `trust-set-result` from 2.10; a raw reader
(`transportctl trust list`) prints whatever row the negotiated minor
gives. Their schemas, `trust-list-params`, `trust-list` and `trust-set-params`, were `approved`, and the method and capability enums carry their minor bumps (`ipc/method`
1.1.0, `ipc/capability` 1.2.0, `ipc/request` 1.1.0), as every 2.0 shape
did (plan §16 (3)); they flipped `active` with Stage 15's close (2026-10-06, with `ipc/path-changed`). Discovery and bootstrap administration still have no method; Stage 15's close carried them to the owner (plan §18's record, with ADR-0032's revisit).

**The endpoint dimension** (IPC 3.0, ADR-0067 decision 6; built by Stage 22's trust batch). Without an `endpoint` member the two methods read and write the operator's list, as above and as before. With one they read and write that endpoint's own set, which lives in the endpoint overlay (`<state>/endpoint-overlay.json`, its top-level `inbound` and `outbound` members, each a map from EndpointId to a sorted list of PeerIds, an endpoint in both or in neither, ADR-0028 A 2026-11-22) and never in the operator's list:

- `endpoint` is the EndpointId of a configured endpoint, authored or added, enabled or not — the set is configuration, as the lease is. A value outside the grammar is `InvalidArgument`; an id no entry holds is `EndpointUnknown`. The alias is not accepted: a client resolves it (`ENDPOINTS.md` §Configured endpoint set).
- `admin.trust.set {endpoint, peer, allowed}` writes the peer into, or out of, BOTH members of that endpoint's overlay set: a trust act is the endpoint's in both directions, and the method has no way to write one alone. The refusals are the operator form's — the local peer, and a write after which the admission set would hold more than `MAX_ALLOWED_PEERS` peers, each `InvalidArgument`, audited `refused`, nothing written — with one of the dimension's own, the same answer: a write after which all endpoint sets together, both members counted, would hold more than 4 × `MAX_ALLOWED_PEERS` rows (16384; ADR-0028 A 2026-11-22), which the first write's copy below counts toward — and so are the no-op answers, the superseded refusal (keyed by the peer and the endpoint together, so a set for one endpoint never supersedes a set for another or for the operator's list), the write-before-publish order and its failure answers, which hold of the endpoint overlay as they hold of the trust overlay. The audit line carries the endpoint.
- **The first write creates the set from what the endpoint admitted.** When the overlay carries no set for the endpoint, a set first copies the endpoint's effective set at that moment — per direction: an authored `static_subset` as it narrows the operator's list, else the operator's list — into the overlay's two members and then applies the change, in one overlay write. From that write the endpoint holds its own set: the operator's list no longer reaches it, and an endpoint-less call no longer changes what it admits. So giving an inheriting endpoint its first own entry never drops what it admitted the moment before.
- `admin.trust.seed {endpoint}` does that copy alone, with no change after it, and answers `{outcome: seeded}`, or `{outcome: present}` with nothing written when the overlay already carries a set for the endpoint — an empty one included, which is what an added identity is given at its add. It is the human client's call at its first run on the Stage 22 release, for the authored first identity (ADR-0067 decision 6), and it is safe to repeat. Its refusals are the set's: an `endpoint` outside the grammar is `InvalidArgument`, an id no entry holds is `EndpointUnknown`, and a seed whose copy would cross the row bound is that bound's `InvalidArgument`, audited `refused`, nothing written; `outcome` is closed at those two values, and there is no third. Capability `admin.trust`; audited as a set is.
- **What a write closes is decided by the admission set, not by the write** (ADR-0067 decision 6, A 2026-11-19): a peer's connections close, `peer.disconnected` with `reason_class: policy`, its cached directory dropped, when the write removes it from the last set that held it, the operator's or any endpoint's; a revoke for one endpoint that leaves the peer in another set closes nothing and takes effect at that endpoint's route, `no_route` to the peer from the write on. An allow for an endpoint admits the peer at the connection whether or not the operator's list holds it.
- `admin.trust.list {endpoint, direction, after?}` answers one direction of the endpoint's effective set, `direction` being `inbound` or `outbound`, required with `endpoint` and refused `InvalidArgument` without it. The page, the order, the cursor and the row are the operator form's. An endpoint with an overlay set answers that member's peers, every row `administered` and `persisted: true`, the seeded ones included: after the seed the configuration no longer decides the set, and the row carries no origin of it. An endpoint with none answers what it admits as derived — an authored `static_subset`'s rows `configured`, an inheriting endpoint's the operator's rows with their own sources — so an operator reads what the endpoint admits before giving it a set. The answer's `set` member says which of the three it is, `own`, `authored` or `inherited`, and is present only on an endpoint answer; `local_peer` is absent from it. Every endpoint answer carries `set`, on each of its pages. `ipc/trust-list` cannot require that, since a result does not record what was asked: the daemon's guarantee is its one constructor of an endpoint page (`TrustList::page_endpoint` in the Rust mirror), and a client that asked with `endpoint` and reads a page without `set`, or with a `set` that differs from the first page's of that direction, refuses the read as `Internal` and shows nothing of it — read as the operator's list, it would be the wrong list.
- Where the two members of an own set differ — an overlay seeded from an entry whose authored subsets differed, or written by hand — each direction applies as written, and the person's view (`human-client-ui.md`) is the peers both hold: a client shows a peer in one member alone as not trusted, and the trust act on it writes both.
- `transportctl trust list|allow|revoke` take `--endpoint <alias or id>` for the same dimension, and `trust list --endpoint` prints both directions: with `--json`, each direction's `ipc/trust-list` pages as the daemon sent them, one a line, inbound first, a page without `next` ending a direction, since a page does not carry its direction. There is no `trust seed` verb: the seed is the human client's call, above.

**The endpoint set's methods** (IPC 3.0; ADR-0061 decisions 2, 5 and 7, ADR-0067 decisions 3, 4 and 6, ADR-0028 A 2026-10-21; shaped by p2p-network-dev and ruled 2026-11-20; built by Stage 22's batch after the trust dimension). `admin.endpoints.add`, `admin.endpoints.remove` and `admin.endpoints.set_advertise` write the endpoint overlay (`<state>/endpoint-overlay.json`) and never `config.yaml`. They are 3.0 methods under the pre-release rule of §Version negotiation and phases, not a minor, and the Stage 22 release is not tagged without them. On Android the embedded host's configuration path calls the same runtime function as the IPC binding, with the same refusals in the same order. Every write is the overlay's: written whole and synced before the change is published to the runtime and before the answer, a failed write answered `Internal` with nothing changed, as `set_enabled`'s.

- **A refusal of an add or a remove is an answer, not an error**, as `admin.peers.add_address`'s is and for its reason: a client says why in its own words without parsing a message, and the error vocabulary (`ipc/error-code`) gains no name. The errors that remain are the ones every endpoint method has: `InvalidArgument` for params outside their schema and for a result the composed profile's validation would refuse (as `set_enabled`), `EndpointUnknown` for an id no entry holds, `Internal` for a failed write, and `BackendUnavailable` for a publish the runtime did not take (it has stopped, or is stopping): where the call had written the overlay, the previous one was put back before the answer, and nothing changed — not the runtime, not a lease, not what the next start loads. It is the answer `set_enabled` and `set_default` already give there, and it is kept apart from `Internal` since a client branches on the code (§Close: `BackendUnavailable` is retried): after it the same call sent again is the whole recovery, while `Internal` also covers an overlay left ahead, which each method's bullet below reads. An add's `unchanged` publishes too (its bullet below), so a retried add can be answered `BackendUnavailable` in place of `unchanged`, with the entry still in the overlay and still not registered (A 2026-11-25).
- `admin.endpoints.add` takes `endpoint-add-params`, closed: `{key, alias, allowed_client_kinds}`. `key` is the entry's `key` value, the key file's name `<EndpointId>.key` (`ENDPOINTS.md` §Configured endpoint set), for a file the client sealed under `<state_dir>/endpoint-keys/` before the call: of the two forms ADR-0067 decision 4 allowed, the method takes the file's name and no public key, since the daemon seals nothing and an add by public key alone would create an endpoint in the key-missing state. No key material crosses the IPC in either direction. `alias` and `allowed_client_kinds` are the entry's. The params carry no `id` (it is derived), no `advertise` (the add writes `false`, ADR-0061 decision 5), no `enabled` (an added endpoint is enabled; `set_enabled` turns it off) and no `inbound` or `outbound`: the add writes the endpoint's two overlay sets EMPTY (ADR-0067 decision 6), so it admits nobody until `admin.trust.set` names it, and `admin.trust.seed` answers `present` for it. It leaves the default where it is.
- The add answers `endpoint-add-result`: `{outcome: added | unchanged | refused, endpoint?, refused?, detail?}`, `endpoint` the derived EndpointId on `added` and `unchanged`, `detail` for a log. On `added` the entry is in the overlay's `added` member, carrying no `inbound` or `outbound` field of its own, and its id is in the overlay's top-level `inbound` and `outbound` members with `[]` in each — its own set, empty (ADR-0028 A 2026-11-22) — and the endpoint is registered and leasable from the answer on, with no restart. `unchanged` is the answer to an add whose id the overlay's `added` member already holds with the same alias and the same client kinds, nothing written — a retry after a lost answer; the kinds are compared as a set, order and repeats aside, and it is decided where `exists` is, after the key checks, so a retry whose key file has gone since is `key_missing` (A 2026-11-24). An add of an id the runtime still serves after a remove left the overlay ahead (the remove's bullet below) is judged by the overlay, as that remove's repeat is: it writes the entry and answers `added`, the two sets `[]` in the overlay and in the runtime from the answer on, while the alias, the kinds and a live lease the runtime held for the id stay as they were until the daemon's next start, which loads the entry. An add that leaves the overlay ahead of the runtime (ADR-0028: the write installed, the publish failed, the restore failed) is answered `Internal` as a failed write is, and a client does not tell the two apart: the overlay kept on disk holds the entry while the runtime has not registered the endpoint, so until it is registered `admin.endpoints.list`, the runtime's view, lacks the id and a lease claim on it is `EndpointUnknown`. The add sent again with the same alias and the same kinds is judged by the overlay and is `unchanged`, nothing written, and it registers the endpoint: an `unchanged` publishes the entry the overlay holds, as a set the overlay already records still publishes, and is answered only once that publish succeeded, so `unchanged` too means registered and leasable from the answer on. An add left ahead that is neither sent again nor removed takes effect at the daemon's next start, which loads the entry with its two sets empty (A 2026-11-25). `refused` is one of, checked in this order with nothing written: `id_not_derived` (a `key` value that is not `<EndpointId>.key` under the full grammar check, the load's `EndpointIdNotDerived`); `alias_invalid` (outside the alias grammar, the load's `AliasInvalid`); `key_missing` (the file absent or unreadable: unlike the load, which keeps such an entry in the key-missing state, the add creates no endpoint without its key); `key_mismatch` (the file's header derives another id than the name's stem, the load's header-mismatch refusal); `exists` (the id is an authored entry's, or an added one's with another alias or other kinds); `alias_taken` (another entry, authored or added, holds the alias; the load's `AliasTaken`); `bound` (authored and added entries would pass 64, `MAX_ENDPOINTS`, ADR-0061 decision 2).
- `admin.endpoints.remove` takes `endpoint-params` and removes an ADDED endpoint. It answers `endpoint-remove-result`: `{outcome: removed | refused, refused?, revoked_epoch?, detail?}`. `refused` is `authored` (a `config.yaml` entry, which no program rewrites: an authored endpoint is turned off, not removed) or `default` (the default endpoint: the client moves the default first, ADR-0061 decision 7), in that order, nothing written. On `removed`, in one overlay write before the answer: the `added` entry and every overlay member keyed by the id — `enabled`, `advertise`, `inbound`, `outbound` — are gone; a live lease is revoked at once (`endpoint.lease_changed`, its epoch in `revoked_epoch`) as `set_enabled(false)` revokes it; the admission set is derived again, and a peer the removal took from the last set that held it is closed, `peer.disconnected` with `reason_class: policy`, its cached directory dropped (the endpoint dimension above). The daemon does not delete `endpoint-keys/<EndpointId>.key`: the client sealed it and the client disposes of it, with the store rows of the identity (ADR-0061 decision 7). A removal of an id no entry holds is `EndpointUnknown`, which a client resuming a Remove whose answer was lost reads as done once `admin.endpoints.list` confirms it. A remove that leaves the overlay ahead of the runtime (ADR-0028: the write installed, the publish failed, the restore failed) is answered `Internal` as a failed write is, and a client does not tell the two apart: the overlay kept on disk no longer holds the entry while the runtime still serves the endpoint, so until the daemon's next start a remove sent again is `EndpointUnknown` and `admin.endpoints.list`, the runtime's view, still holds the id; at that start the endpoint is gone (A 2026-11-24; ADR-0061 decision 7 has the client's reading). A remove of an endpoint an add left ahead (the add's bullet above) is judged by the overlay, which holds the entry: it is answered `removed` with no `revoked_epoch`, there being no lease to revoke; the runtime's not holding the endpoint is agreement and no failure, the overlay is written without the entry, and the daemon's next start does not bring the endpoint in (A 2026-11-25).
- `admin.endpoints.set_advertise` takes `set-advertise-params`, `{endpoint, advertise}`, and answers `empty-result`. For an authored endpoint it writes the overlay's `advertise` delta, for an added one its entry's member; a value already in effect is answered with nothing written, and a delta equal to the authored value is not kept. A value the composed profile would refuse (`directory.max_advertised`) is `InvalidArgument`, nothing written. The overlay is synced and the directory's answer source updated BEFORE the answer: that order is what the human client's assertion leans on (ADR-0061 decision 5; plan §30's exit gate (b)) — it takes an identity's lease after this answer and not before, so a call that fails or blocks grants no lease and no listed-and-leased identity answers a directory query.
- `admin.endpoints.set_alias` (ADR-0061 decision 7, A 2026-11-20; a fourth method, added by the ruling since a rename had no door) takes `set-alias-params`, `{endpoint, alias}`, and answers `set-alias-result`: `{outcome: renamed | unchanged | refused, refused?, detail?}`. It rewrites an ADDED entry's alias in the overlay, written and synced before the answer; `unchanged` is the alias the entry already has, nothing written; `refused` is `authored` (the alias of a `config.yaml` entry is the file's, which no program rewrites, and the overlay carries no alias delta: a person's rename of the first identity changes its card and not its alias), `alias_invalid` or `alias_taken`, in that order, nothing written; an id no entry holds is `EndpointUnknown`, checked before `authored`. A rename of an endpoint an add left ahead (the add's bullet above) is judged by the overlay, which holds the entry, as the remove of one is: it is answered `renamed` and the overlay keeps the new alias, the runtime's not holding the endpoint being agreement and no failure; the rename does not register the endpoint, so the list lacks the id until the add is sent again with the new alias (one sent with the alias the entry had is refused `exists`) or until the daemon's next start, which loads the entry under the new alias. An alias is no wire field and keys nothing: no lease, route, trust set or overlay member moves, and a client resolves an alias when it starts (`ENDPOINTS.md` §Configured endpoint set), so a rename reaches a running client at its next resolution. It is audited with add and remove, is not ordered by connection, and its two schemas arrive with theirs.
- `admin.endpoints.list`'s 3.0 row is `{id, alias?, source, enabled, default, advertise, persisted, lease?}`: `alias` the entry's local alias, absent for an authored entry that carries none (the entry's `alias` is optional, `endpoints/endpoint-config`; an added entry always carries one, the add's params requiring it) and never an empty string or the id in its place (A 2026-11-24), `source` `authored` or `added` (what `remove` accepts, in `admin.peers.list_addresses`' words), `advertise` the effective value, which is how a client reads that an identity is unadvertised. The endpoint-list schema's Stage 22 major, the one the derived id gives it, carries the three members.
- Add and remove are audited as `admin.trust.set` is — the method, the endpoint, the outcome and a refusal's reason — since they create and destroy an admission source; `set_advertise` is not, as `set_enabled` is not. None of the three is ordered by connection: the `superseded` rule (§Cancellation mapping and request concurrency) stays the trust and peers overlays'. An add applied late, after the client removed the identity and deleted its key file, is refused `key_missing`. The methods that write the endpoint set (add, remove, `set_advertise`, `set_alias`, `set_enabled` and `set_default`), and with them the two that write an endpoint's own trust set into the same overlay (`admin.trust.set` with an `endpoint`, and `admin.trust.seed`), are answered one at a time by the runtime that holds the set and its overlay, each one's check of the id and its write being one answer, `admin.endpoints.list` being its view and ordering nothing, an add's reading of the key file and its write being one answer and the file being read in that answer's turn, not when the request is accepted, so an add and a remove of one id do not overlap: a remove answered `EndpointUnknown` found no overlay entry for the id, which the list may still hold after a remove left ahead (the remove's bullet above), and an add answered after it reads the key path as it is then, the order across connections staying the order of arrival, which no client chooses (ADR-0061 A 2026-11-26 (ii)). A remove applied late, after the client stopped waiting and read the list, is the remove that was asked: a lost admin write may follow a later read (§Cancellation mapping and request concurrency), so the id in a list read after a lost answer says that the remove had not committed when that list answered, not that it will not (A 2026-11-24).
- The schemas (`endpoint-add-params`, `endpoint-add-result`, `endpoint-remove-result`, `set-advertise-params`, `set-alias-params`, `set-alias-result`), the method and request enums and the Rust mirror arrive with the batch that implements the methods, as `admin.peers.*` did.

## Event catalogue

Every `event` frame's `event_type` binds its `data` to a shape
([`schemas/ipc/event.schema.json`](./schemas/ipc/event.schema.json)):

| Event type | Data | Delivered to | Since |
|---|---|---|---|
| `message.direct` | `endpoints:message-received` | exactly the connection holding the destination endpoint's lease | 2.0 |
| `message.broadcast` | `ipc:broadcast-received` | every connection with `events` holding a join reference for the channel | 2.0 |
| `endpoint.lease_changed` | `ipc:lease-changed` | the connection whose lease was revoked | 2.0 |
| `peer.disconnected` | `{peer, reason_class}` | every connection with `events` | 2.0 |
| `peer.path_changed` | `ipc:path-changed` | every connection with `events` that has a route to the peer: a direct message exchanged with it, or a broadcast received from it on one of its joins — a received message counting from the moment the daemon took it from the session for the connection (the IPC projection of LOCAL-CLIENT.md §2's take, A 2026-10-04), a sent one from its acceptance, until a revocation ends the route (A 2026-10-05); from 2.4 also when the route begins (`reason_class` `route_established`) and when a routed peer connects again after a `peer.disconnected` (`reconnected`), `previous` absent in both (`ipc/path-changed` 1.1.0, A 2026-10-09) | 2.1 (route begin, reconnect: 2.4) |

A lease GRANT is learned from `hello_response`, not from an event;
`endpoint.lease_changed` carries revocation only: it is the IPC
projection of TRANSPORT.md's `EndpointLeaseChanged { state: registered |
released | revoked }` — a grant is learned from `hello_response` and a
release ends with the connection, so only `revoked` crosses the wire.
`peer.disconnected` is the runtime's `PeerDisconnected` (TRANSPORT.md
§Events) delivered to every connection holding `events`; its
`reason_class` is `policy` for a trust revocation (ADR-0012) — when a
trust change closed every connection the peer held — and `closed`
otherwise, the runtime's own name (#162); any further class is the
runtime's to name when it produces the event.
`peer.path_changed` (2.1, A 2026-10-03, Stage 15's R1) is the runtime's
`PeerPathChanged` (TRANSPORT.md §Events: `direct | relayed` either way,
with its `reason_class` and `observed_at`), the IPC projection of
LOCAL-CLIENT.md's session notice of the same name: delivered only to a
connection that has a route to the peer, coalesced per peer to the
latest pending (a replaced pending one is counted), in the ORDINARY
lane under §Push events' path-notice rule — taken after every message within the pump's room, waiting rather than dropped under pressure (A 2026-10-04), unlike the four in the reserved lane, which are never held back. Its schema `ipc:path-changed` lands `approved` with
the implementing batch and its mirror, as above.

## Version negotiation and phases

`hello.ipc_version.major` accepts any positive integer, so an unsupported
major is a well-formed hello: the server answers
`close{code: VersionIncompatible, supported: [{major: 3, minor: IPC_MAX_MINOR}]}`
— the server's highest minor on major 3, 0 since the Stage 22 release opened the major (ADR-0067 decision 7); major 2, which every release before it spoke, ended at minor 13 — 13 since `admin.status`'s refusal total (`ipc/admin-status` 1.2.0, A 2026-11-10; 12 since the last-seen peer row, `ipc/peer-list` 1.2.0, A 2026-10-28; 11 since `session.losses`, ADR-0026 A 2026-10-24; 10 since the superseded refusal, §Cancellation mapping and request concurrency; 9 since `admin.peers.add_addresses`, ADR-0062 decision 4; 8 since `admin.peers.own_addresses`, ADR-0062 decision 4; 7 since the `admin.peers` address methods, ADR-0062; 6 since the route-hold peer row, A 2026-10-21; 5 since the endpoint overlay, A 2026-10-11; 4 since the route-begin notices, A 2026-10-09; 3 since the trust overlay, A 2026-10-07),
never a literal older than the server — and closes. For major 3 the server selects `minor = min(client, server)` and
returns it in `hello_response`. The client holds the server to that
rule: a `hello_response` whose minor is above the minor the client
offered, or above the highest minor the client speaks, could not have
been selected by it, and the client treats it as the server's protocol
violation — the connection is refused as `ProtocolViolation`, nothing
is sent on it, and no minor is remembered from it (A 2026-10-05, #192,
the mirror of the server's capability rule below). Minors are **additive only**: a new
method, event type or feature is emitted or accepted only when the
negotiated minor is at least the one that introduced it (the `Since`
columns above). A CAPABILITY is the one such thing a client names before
the minor is negotiated, in its `hello`, whose vocabulary is closed (a
`feature` is named there too, but as an open string: an unknown one is
simply not negotiated, while an unknown capability fails the closed
`ipc/capability` parse) (A 2026-10-03, on #175's review):
a capability introduced at minor m is requested only in a hello sent
after the client has learnt, from a `hello_response` to an earlier
connection to the same daemon, that the server selects a minor ≥ m when
offered it; a first hello names only the capabilities of its major's minor 0 — on major 2 the 2.0 capabilities, on major 3 every capability major 2 reached (the major's rules below). A hello naming a
capability above the minor it negotiates is the client's protocol
violation and is answered `close{ProtocolViolation}`, whatever minor the
server speaks — a 2.0 daemon's closed `ipc/capability` parse refuses it
before negotiating, and a 2.1 daemon refuses it the same way for a
minor-0 hello, so the client sees one result either way. A client that
receives that close re-learns the minor and retries without the
capability: a daemon restart may change the minor between connections.
The cost is one probe connection per daemon instance for an
administrative client that wants a 2.1 capability; adding, removing or changing a property of an existing closed shape is
a major once the first production build speaks 2.0; before it, an `approved`
schema takes an additive property into 2.0 itself (`event_queue` on
`hello_response`, A 2026-09-30), may remove a property no build has ever
emitted, its mirror refusing the old name (`pre_auth.tracked_peers` off
`admin-status` 1.1.0, A 2026-10-01), and treats a change as that removal
plus that addition — its own version moving 1.x → 1.(x+1) each time
(ADR-0017 records the rule and its one bound; the bound holds for a schema flipped `active` before that build as for an `approved` one, and a pure relaxation of a bound is additive before the build, when no consumer built against the old bound exists; after it a result relaxation widens behind a new minor as below and a params relaxation is a major — A 2026-10-07). A closed RESULT shape may
widen behind a new minor (ADR-0017 A 2026-10-07): a property gains a
value or a new optional property appears only on a connection that
negotiated that minor or later, the shape served below it stays
byte-identical to the schema's earlier description, the schema names the
minor each shape applies from, and the old shape is served on every
supported minor below the new one; first use `ipc/trust-list` 1.1.0 behind
2.3. The exception covers a closed EVENT data shape as it covers a result
shape, and on either a REQUIRED member may become OPTIONAL when every
instance that omits it is itself new behind the minor, so below the minor
no instance lacks it — and, the shape being an event's, the bound holds on
the stream: the emitter takes no new-only instance for a connection below
the minor, so none can absorb a change the old rule owed; the serialiser's
filter is the backstop (ADR-0017 A 2026-10-09); first use `ipc/path-changed`
1.1.0 behind 2.4, `previous` absent at route begin and reconnect, a
connection below 2.4 opened as a session that declined route notices
(a connection of major 2; on major 3 none is below it, none declines and
the request option that asked it is gone, the major's rules below).
The first production build spoke 2.0; Stage 15's R1 batch, which
brought `peer.path_changed`, spoke 2.1, and R2 added `admin.trust.*` to
it; `admin.peers.list` brought 2.2 (A 2026-10-06); the persisted trust
row brought 2.3 (A 2026-10-07, #215); the route-begin and reconnect `peer.path_changed` (`previous` absent, `reason_class` `route_established` or `reconnected`; `ipc/path-changed` 1.1.0) brought 2.4 (A 2026-10-09, Stage 17's relay pair) — below 2.4 nothing is sent when a route begins or a routed peer reconnects; the persisted endpoint row (`ipc/endpoint-list` 1.1.0, `persisted: true`) brought 2.5 (ADR-0028 A 2026-10-11, plan §20 (e)) — below 2.5 the 2.0 row is served; the peer row's route hold (`ipc/peer-list` 1.1.0) brought 2.6 (ADR-0011 A 2026-10-21); the `admin.peers` capability with its three address methods brought 2.7 (ADR-0062); its show side's read, `admin.peers.own_addresses`, brought 2.8 (ADR-0062 decision 4) — below 2.8 a client does not send it, and a 2.7 server answers it as an unknown method; its all-or-none add, `admin.peers.add_addresses`, brought 2.9 (ADR-0062 decision 4) — below 2.9 a client does not send it, and a 2.8 server answers it as an unknown method; the superseded refusal of an admin write (`trust-set-result`, `superseded` in the two address results) brought 2.10 — below 2.10 a superseded write is still refused, answered the error `Internal`; and the session's own broadcast losses, `session.losses` (`ipc/session-losses-result` 1.0.0), brought 2.11 (ADR-0026 A 2026-10-24) — below 2.11 a client does not send it and answers `ProtocolUnsupported` itself, a session that declined route notices among them, and a 2.10 server answers it as an unknown method. The last-seen peer row brought 2.12 (`ipc/peer-list` 1.2.0, `last_seen_ms`, A 2026-10-28): served on a connection that negotiated 2.12 or later, and below 2.12 the 1.1 row is served unchanged, without the field; 2.12 adds no method and no capability. `admin.status`'s profile total of what the broadcast ingress gate refuses brought 2.13 (`ipc/admin-status` 1.2.0, `ipc.events_refused_total`, ADR-0026 A 2026-11-10): served on a connection that negotiated 2.13 or later, and below 2.13 the 1.1 `ipc` block is served unchanged, without the member; 2.13 adds no method and no capability.

**Major 3 is the IPC of the derived EndpointId** (ADR-0067 decision 7, the Stage 22 release). Every EndpointId an IPC document carries — the hello's claim, `hello_response.endpoint`, `send-params`, the endpoint rows and params, `message.received`, `endpoint.lease_changed` — is a derived id in the grammar of `ENDPOINTS.md` §EndpointId, and no reader of major 2 tolerates the value, which is ADR-0017's case for a major. The rules of the major:

- **One major is spoken.** A daemon or embedded host of that release speaks major 3 and no other: a major-2 hello is the unsupported major above, answered `close{VersionIncompatible, supported: [{major: 3, minor: IPC_MAX_MINOR}]}`, and a major-3 client reads the same close from a daemon of a release before it, with major 2 in `supported`. No adapter for major 2 exists and none may be added without its own record (ADR-0067: a clean break, on the owner's word that no installed base exists). A client that reads the close says the two programs are of different releases; it does not retry on the other major.
- **3.0 holds everything 2.13 held.** No method, event, capability, feature, error code or close code was added or removed by the major: the catalogue of 3.0 is the catalogue 2.13 reached, with the id's grammar changed. Major 3 starts at minor 0.
- **`Since` is a (major, minor) pair, compared as a pair.** The `Since` columns keep the major-2 minor at which each row arrived, as its history; every one of them is at or below 2.13 and is therefore in 3.0, and a row added on major 3 is written `3.<n>`. Wherever this document says a method, an event, a row or a shape is served, granted or sent "only on a connection that negotiated 2.<n> or later", every major-3 connection meets it, whatever its minor; "below 2.<n>" describes a connection of major 2, which no release from Stage 22's serves. A mirror that gates on the minor alone is wrong from 3.0: a 3.0 connection would be refused what 2.1 brought.
- **One shape per document on 3.0.** A result or event shape that widened behind a major-2 minor (`ipc/trust-list` behind 2.3, `ipc/path-changed` behind 2.4, `ipc/endpoint-list` behind 2.5, `ipc/peer-list` behind 2.6 and 2.12, `trust-set-result` and the superseded refusals behind 2.10, `ipc/admin-status` behind 2.13) is served on major 3 in the shape 2.13 served, and the older shape of each is served to no connection. The widening rule (ADR-0017 A 2026-10-07, A 2026-10-09) applies again from 3.0: a shape that widens behind 3.<n> keeps its 3.0 shape below it.
- **Every session takes route notices; the option to decline them left with major 2.** Major 2 let a client decline the route-begin and return notices by offering a minor below 2.4, and the server opened that connection as a session without them, which was the only use the request option ever had: it existed for a client that cannot read a `peer.path_changed` whose `previous` is absent. Major 3 has no minor below what 2.13 held and serves `ipc/path-changed` in its one shape, `previous` absent where 2.4 made it so, and a client of the in-process binding is compiled against the crate that defines the shape, so no client of either binding can need the option. It is therefore not part of the session request from the Stage 22 release (ADR-0017 A 2026-11-22): no binding offers a session without route notices, and the path notices a session is owed are `LOCAL-CLIENT.md`'s, with no second form. The rule the option served stands: an event shape's bound holds on the stream (ADR-0017 A 2026-10-09), so a shape that widens behind 3.<n> is kept from a connection below that minor by the emitter, by whatever means that change's own record names. Declining over IPC, if a client ever needs it, is a `hello` feature added at a 3.<n> minor under its own decision.
- **Every capability major 2 reached is a 3.0 capability.** A first hello of major 3 may name any of them, `admin.trust` and `admin.peers` included; the probe connection the capability rule below costs is owed again only for a capability introduced at 3.1 or later.
- **The schemas take their majors once.** Every `ipc` schema that `$ref`s the endpoint-id takes a major in the batch that builds this (ADR-0067 decision 7), and `ipc/frame` with them, since it carries those shapes; the closed-shape rule below binds from 3.0 as it bound from 2.0, once a production build speaks it.
- **Major 3's first production build is the Stage 22 release, and what the stage adds before it is 3.0.** Between the batch that opens the major and that release's tag no production build speaks major 3, which is the pre-release case of ADR-0017 (A 2026-10-01, A 2026-10-07): a schema of major 3 takes an additive property into 3.0 itself, and a method the stage adds is a 3.0 method. So the endpoint dimension of `admin.trust.set` and `admin.trust.list` (ADR-0067 decision 6) is an optional member of their 3.0 params, not a minor — as a minor it could not have been one, a params change being a major once a build speaks the shape — and `IPC_MAX_MINOR` is 0 at the release. The release is not tagged with a part of the stage's IPC surface missing: a build that spoke 3.0 without it would make the rest a major.
- **`transport_contract_version` is `"3.0"`.** The value names the transport contract's major (`TRANSPORT.md` §Versioning), which moved with the same decision (ADR-0067 decision 7, A 2026-11-24); it is not the IPC version and is negotiated by nothing.

Phases and directions, which JSON Schema cannot express and
`tests/ipc-v2` asserts: `hello` is the client's first frame and only its
first; `hello_response` and `close` are server-only; no `request`,
`cancel` or `pong` before `hello_response`; a `hello` not received within
**5 s** of the connection (a protocol constant, not a profile value) is
answered `close{Timeout}`.

## Close

`close` ([`schemas/ipc/close.schema.json`](./schemas/ipc/close.schema.json))
is the server's connection-fatal reply, sent — when the transport still
allows a write — before the connection is closed: for a handshake
refusal (with the handshake error codes above), for a framing or
protocol error before any request id exists (`ProtocolViolation`), for an
unsupported major (`VersionIncompatible`, with `supported`), for
keepalive expiry (`Timeout`), for shutdown (`ShuttingDown`), and for a
connection accepted past the connection limits of §Multiple clients
(`Overloaded`, before any hello is read; A 2026-09-29), and for a
request id reused while outstanding (`ProtocolViolation`). An error
that has a request id is a `response{ok: false}`, never a `close`.

A call that is still waiting when the connection ends answers the code
the session ends with (A 2026-10-08): the `close` frame's code when one
was read, `Timeout` when the client's own keepalive bound ended it
(below; armed only after a ping was read), and `BackendUnavailable`
when the connection ended otherwise with no `close` frame read. Every binding keeps the server's last frame readable
after the server's close — a client whose write failed keeps reading
what already arrived and answers its waiting calls once the reader
stops, a reset after the `close` frame counting as the end — a
requirement on the binding, not a property of one transport (a Unix
socket gives it; the Windows named pipe must flush before it
disconnects). The call and the session's end carry one code, read once:
a client branches on a call's code (`ProtocolViolation` is not retried,
`BackendUnavailable` is), and a `BackendUnavailable` answered to a call
whose connection is about to read `ProtocolViolation` would tell it to
retry a session the daemon refused. No timer of its own: on a connection
that negotiated keepalive, the client's keepalive bound (§Disconnect/
reconnect and optional keepalive, A 2026-10-08: `CLIENT_SILENCE_TIMEOUT`,
120 s with no frame read, armed at the first ping) ends a server that
keeps its write half open and sends nothing with `Timeout`; without
keepalive only the caller's own timeout and OS connection liveness do. Landed with #230 (2373d967, 2026-10-08): ipc-client answers a waiting
call with the session's end code, pinned by
`a_failed_write_keeps_the_servers_close_code` and
`a_failed_write_ends_the_session_though_the_server_keeps_writing_open`.
Five behaviours of that drain are the rule for every binding, the
figure the binding's own: a writer's provisional end is shown to no
caller, so a call, `events` and `ready` read one code; the drain after a
failed write is BOUNDED in bytes (ipc-client: 1 MiB, `DRAIN_BUDGET`,
read in frames so the last read may overshoot it), so a daemon that
keeps writing cannot hold the calls — past the bound the connection ends
`BackendUnavailable` with no `close` frame read; the drain stops the
same way, `BackendUnavailable`, when the client's event buffer is full
and the next frame does not fit — it never waits for room, so a `close`
behind that event goes unread; a frame the client refuses during the
drain keeps its code, `ProtocolViolation`, as without the failed write;
and a request frame that cannot be encoded fails its call alone,
`PayloadTooLarge`, the connection staying up (the arm is unreachable
through the typed API, which bounds the request at the protocol's
maximum).

## Cancellation mapping and request concurrency

A `cancel` for a request the server still holds pending — not yet handed
to the session — answers `CancelledBeforeDispatch` and the request is
dropped; a `cancel` for a request already handed over answers
`CancellationRaced` and the outcome, if any, is discarded. Each connection
has at most **16 requests in flight** (a protocol constant); further
requests wait in a pending queue of at most **48** (16 + 48 = the 64
outstanding commands per client of TRANSPORT.md §Backpressure); past
that bound the request is answered `Overloaded`.

`deadline_ms` is the caller's command deadline (TRANSPORT.md
§Cancellation): absent, the profile's command-deadline default;
present, clamped into 1..60 s, never refused — the range TRANSPORT.md
§send(destination, payload, options?) gives the direct-send command
deadline, adopted here for every IPC request.
A request whose deadline passes while pending is answered `Timeout`
and dropped; one whose deadline passes after hand-over is answered
`Timeout` and its later outcome discarded, as `CancellationRaced`
discards it — the port carries no deadline, so the server can stop
waiting, which is all the caller was promised, and cannot make the
binding stop. The server never lets a request outlive its deadline
silently (A 2026-09-29).

**An admin write whose answer was lost is ordered against later writes
by connection, and may still follow a later read** (A 2026-10-23). The
server cannot make the binding stop, so a `admin.trust.set` or an
`admin.peers.*` write that was read from a connection the client gave up
on can enter the driver's queue after the client has written again on
another connection. The daemon bounds that without a wire change: every
admin connection that holds `admin.trust` or `admin.peers` is stamped
with its admission order where its port is opened, and the driver keeps,
per key — a PeerId for `admin.trust.*`, an address exactly as written for
`admin.peers.*`, as the overlay tells its entries apart — the stamp of the connection whose
write for that key it last applied, an `unchanged` answer counting as
applied since that write's intent is the state that stands. A write from
a connection older than that stamp is refused, not applied: `superseded`,
an answer and never an error, checked last so that it names only a write
every other rule admitted (`admin.peers.add_addresses` refuses the whole
list, `index` naming the first such line), and the admin audit record
names it as it names the applied change. The memory is bounded: an entry
is dropped once no open writing connection is older than it, and past
the map's ceiling the oldest entry is evicted and its stamp becomes a
floor below which every write is refused `superseded`, failing closed.
Below minor 2.10 the same write is answered the error `Internal`, which
every client already reads as nothing changed and re-reads. What is not
bounded, and is stated here: a lost write may still take effect after a
LATER READ on another connection — never after a later write for the
same key — so a client that lost an answer re-reads and shows what it
holds meanwhile as pending; a read orders nothing, on any connection; a write is
ordered only by the order its port opened, so a client that opens a
second admin port while keeping the first finds the first's writes
refused `superseded` for every key the second has written. The two cases are
pinned at every binding as two admin ports opened in order
(`tests/local-client-conformance`): a write on the later port, then one
on the earlier, refused `superseded`; a read on the later port, then a
write on the earlier, applied. A write that outlives its connection, and
the audit line, are the composition's own tests.

## Peer identity, lock and stale sockets (Unix)

Peer credentials are a **MUST** on Unix: the connecting peer's uid must
equal the owner uid of the runtime directory the daemon created; any
other uid is closed before `hello` and counted
(`ipc_peer_credential_refused_total`). This is the same-user boundary
ADR-0037 draws; a hostile same-uid process is SPIKE-005's, not v2.0's.

The daemon holds one exclusive lock, `<state>/profile.lock` (0600, an
advisory whole-file lock), for its lifetime; `transportctl identity
backup` and `restore` take the same lock, so they cannot run beside a
daemon. The lock file is released, never unlinked. A stale socket path
found at start is removed only by the lock holder, and only when it is a
socket owned by the daemon's uid; anything else at that path is fatal.
Shutdown order: refuse new inbound direct → stop accepting → hand each
session what its queue holds → revoke leases → settle bounded direct
responses → close the Swarm → unlink both sockets → release the lock.

The first and third steps are A 2026-11-17 (the owner's word;
`DIRECT.md` §Graceful shutdown). The runtime drains before the server
stops: from then an inbound direct request is answered `shutting_down`
and no endpoint queue takes a new message. "Stop accepting" is the
server's: no new connection, no new request. Each data session is then
written the events its queue held at that moment, ahead of its
`close{ShuttingDown}`, and its lease is released before the `close` is
written, so a client that reconnects on it finds the lease free.
The hand-over is bounded by the session's queue and by the stop's
grace: a client that does not read is closed when the grace ends, or
sooner when its writer gives up on it after the write stall, and
what it did not take is counted as dropped. As the gap was found the
order began at the server's stop, the runtime drained after every lease
was released, and a connection ended on the stop without reading its
session's queue again (`tests/automated-client/runs/2026-11-17-fleet-control-path.md` §3, finding F1).
Built in the change that followed (`ipc-server`'s
`a_stop_writes_what_the_session_held_before_the_close` and
`a_stop_counts_what_a_client_that_does_not_read_left`; between two
daemons, `tests/automated-client`'s
`an_inbound_direct_after_the_stop_began_is_refused_shutting_down` and
`a_message_admitted_before_the_stop_is_read_before_the_close`). Not
counted: an event the connection's writer was part-way through writing
when its writer was aborted, and an event the writer could not encode,
which ends the writer with that event uncounted and the rest of its lane
counted (a frame the server built past the body ceiling, its own defect;
read from `frames.rs`'s `write_loop` and held by no test). The count is a
floor on what a stop dropped: no delivered event is in it, and it can
fall short. The floor is held for a client that does not read, by
`a_stop_counts_what_a_client_that_does_not_read_left` (what was read
and what was counted do not pass the queue) and, where the writer gave
up, by `a_stop_counts_what_a_writer_that_gave_up_left`; for a client
that reads slowly, and for the part-way event, whose loss the next
sentences trace, it is read from the code and held by no test. Once the connection has queued its `close`,
or closed its lanes without one, the writer takes no further event from the
connection's lane, so what the lane held when the stop read it was lost
whole, and the event the writer was part-way through had left the lane
before the reading. A writer that ended on its own -- on the `close`,
or giving up on a client that stopped reading, the stop's abort
reaching it too late to cancel it -- is counted by its own count and
not by the reading: the event it failed on and what its lane holds
once every sender is gone, the lane closed and not dropped, so an
event sent into room reserved before the writer ended is counted with
the rest (`a_stop_takes_the_writers_own_count_unless_the_abort_cancelled_it`,
`a_writer_that_gave_up_keeps_what_is_sent_after_it`; as this gap was
found, such a writer dropped its lane, and the count missed what the
lane held and what was sent after). The reading before the abort is
used only for a writer the abort cancelled. What the count can still
miss is the event a cancelled writer was part-way through, and the
whole lane of a writer that panicked, counted as nothing. #709's
review (F7) read an over-count in the reading coming before the abort;
#724's review and its judgement found no path to one.

## Platform scope of the v1 build

The first production build implements the Unix domain socket binding on
Linux. macOS is not decided: `ProfilePaths::resolve` fails without
`XDG_RUNTIME_DIR` by design and no macOS runtime-directory rule exists. The Windows named-pipe equivalent named throughout this
contract is the design, not the build: its ACL model and peer identity
are carried by name in the implementation plan (§22, Stage 19, since
2026-10-11; §18 and §21 before it).

## IPC v1 compatibility

There is no production v1 deployment requirement, and no major-1 IPC was ever built: the first production implementation spoke major 2. Major 2 is not served beside major 3 (§Version negotiation and phases). If an adapter for an older major is ever added, it must be explicit, under its own record, and cannot reintroduce undocumented direct all-client fan-out into the routing model.

## Connectivity status over IPC

There is no connectivity method. A data client holding `events` receives the normalized `server_state.connectivity` push (A 2026-10-04: it surfaces only as the session's `ServerState` notice through `events()`, which that capability gates; a connection without it is sent none — was `commands`; ipc-server sent the frame to every data connection until Stage 15's R1 batch (#180) gated it on `events`, `a_data_client_without_events_is_sent_no_view` pinning it) — direct/relay state and counts only — on connect and on change; the full backend-neutral `ConnectivitySummary` is `admin.status`'s, on the admin socket. Raw AutoNAT probe-server identities, relay PeerIds, relay multiaddrs, and server-capacity detail require a local diagnostics/admin capability and are never inferred as trust.

The runtime's `ConnectivityChanged` (TRANSPORT.md §Events) reaches IPC only as a `server_state` push, coalesced to at most one pending to avoid state-flap floods. It is not an `event` frame, not a durable replay stream, and does not change endpoint lease semantics.
