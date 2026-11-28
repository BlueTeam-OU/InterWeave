# Direct peer protocol

The prose here is normative for **behaviour**. The parts of this protocol that are document-shaped — the coarse rejection codes, the destination selector, the deduplication key — are also defined as JSON Schema under [`../../contracts/schemas/direct/`](../../contracts/schemas/direct/) (ADR-0049).

The `DirectMessageV2` byte framing below — the request and the `AcceptedV2`/`RejectedV2` response — is deliberately **not** modelled there: it is a fixed-width binary layout, not a JSON document, and cross-implementation agreement on it belongs in `fixtures/direct-v3/` as byte vectors. The family manifest records that boundary explicitly rather than leaving it as an apparent gap.

## Selected primitive

`libp2p::request_response` with a custom bounded codec remains the selected primitive. The protocol is:

```text
/interweave/direct/3.0.0
```

**`3.0.0` is the protocol of the derived EndpointId** (ADR-0067 decision 7, the Stage 22 release). Every endpoint field of the frames below carries an EndpointId in the grammar of `contracts/ENDPOINTS.md` §EndpointId — the 52 ASCII bytes of a key's PeerId-style name — and nothing else about the frames changed: the field order, the widths, the byte order, the reason numbering and every rule of §Semantics are what `/interweave/direct/2.0.0` had. The id changed because the value did: a `2.0.0` reader refuses a derived id as malformed and a `3.0.0` reader refuses a label, so two nodes on either side of the release would otherwise fail frame by frame, each answering `malformed` to a peer that sent what its own contract told it to. With the new id they fail once, at protocol negotiation and by name (§Protocol family).

**The frames keep their names.** `DirectMessageV2`, `AcceptedV2` and `RejectedV2` name the frame layout, the endpoint-addressed one ADR-0030 introduced, not the protocol's major: the layout is the one `2.0.0` carried, so the names stand on `3.0.0`, in this document, in the code and in the vector files' names. The fixture directory takes the protocol's major, `fixtures/direct-v3/`, because every vector in it was regenerated under the new grammar.

**No `2.0.0` beside it.** An implementation of this document advertises `/interweave/direct/3.0.0` and no `2.x` id: ADR-0067 is a clean break with no compatibility window, on the owner's word that no installed base exists. `/interweave/direct/2.0.0`, the endpoint-aware protocol ADR-0030 selected and every release before Stage 22's spoke, is retired with that release, as the architecture-only `/direct/1.0.0` was superseded before any implementation.

## Request

Conceptual frame:

```text
DirectMessageV2 {
  message_id: 16 bytes,          // exactly 128 bits
  sent_at_ms: u64,               // diagnostic only
  source_endpoint_len: u8,       // 52
  source_endpoint: bytes,        // an EndpointId
  destination_endpoint_len: u8,  // 52, or 0 => receiver default endpoint
  destination_endpoint: bytes,   // an EndpointId when present
  media_type_len: u8,
  media_type: bytes,
  payload_len: u32,
  payload: bytes <= effective profile max_payload_bytes <= 49152,
}
```

All multi-byte integer fields are **big-endian** (network byte order): `sent_at_ms` as u64be and `payload_len` as u32be. This matches the rest of the repository — the IPC frame's 4-byte length prefix and `DirectContentFingerprintV1`'s u16be/u32be lengths — and is the only choice under which those three agree. The single-byte length fields have no byte order.

`media_type_len = 0` encodes **absence**. No empty media-type string exists on the wire. A non-zero length encodes a present printable-ASCII media type (0x20..0x7E, A 2026-10-08) and maps to `media_present = 1`; zero maps to `media_present = 0` in `DirectContentFingerprintV1`.

Endpoint strings must satisfy `EndpointId` grammar before routing (`contracts/ENDPOINTS.md` §EndpointId: 52 ASCII bytes, `12D3KooW` and 44 base58btc characters, which decode to the identity multihash of a protobuf-encoded Ed25519 public key). The length fields stay, since the destination's zero is the default selector and a fixed-width field would have been a second layout; an endpoint length that is neither 52 nor, for the destination alone, 0 is `malformed`, and so is a 52-byte value outside the grammar — a label of the retired grammar among them. Codec rejects invalid/oversized declared lengths before allocation. `sent_at_ms` is not authorization, ordering, freshness, replay-window, or dedup input.

The local sender cannot choose `source_endpoint` arbitrarily: transport runtime obtains it from the calling IPC endpoint lease and passes it to the backend.

## Response

```text
AcceptedV2 {
  message_id,
  resolved_destination_endpoint
}

RejectedV2 {
  message_id,
  reason_code
}
```

Coarse reason codes: `no_route`, `unauthorized_peer`, `overloaded`, `malformed`, `too_large`, `shutting_down`, `unsupported`.

### Response byte layout

The response is the whole response substream, one of two shapes told apart by a leading tag:

```text
AcceptedV2: tag:u8 = 1 || message_id:16 || resolved_endpoint_len:u8 || resolved_destination_endpoint
RejectedV2: tag:u8 = 2 || message_id:16 || reason:u8
```

- `message_id` is the request's 16 bytes, echoed.
- `resolved_endpoint_len` is 52 and is never zero. The field is the endpoint that took the message, and the default has already been resolved by the time it is written. The value satisfies `EndpointId` grammar.
- `reason` numbers the coarse codes in the order of `schemas/direct/reject-reason`'s enum, from 1:

  | code | reason |
  |---|---|
  | 1 | `no_route` |
  | 2 | `unauthorized_peer` |
  | 3 | `overloaded` |
  | 4 | `malformed` |
  | 5 | `too_large` |
  | 6 | `shutting_down` |
  | 7 | `unsupported` |

  0 and 8..255 are unassigned. A reader refuses an unassigned code as malformed response metadata, a local `ProtocolViolation`. It is never read as `unsupported`: that peer negotiated this protocol and answered on it.
- A tag other than 1 or 2, a short field, or any byte after the last field is malformed response metadata as well. Every legal `AcceptedV2` is therefore 70 bytes (1 + 16 + 1 + 52), which is the longest legal response; a `RejectedV2` is 18.
- **A response stream that ends with no bytes at all is not a short field** (A 2026-10-26): a short field means at least one byte arrived and the responder answered badly. Zero bytes is the exchange NOT COMPLETING — the responder read the request and never wrote an answer: the two cases proved over real sockets are its own request deadline firing first (the same 10 s as the sender's, so which end fires first is SPIKE-002 finding 1's race, and the sender then reads a clean end of stream rather than its own `Timeout`) and a response dropped before anything was written, and any other way a responder ends with zero bytes written reads the same. The sender reports it as `PeerUnreachable`, in `contracts/TRANSPORT.md`'s outcome-unknown class beside the request-response timeout it stands in for — never as a local `ProtocolViolation`, which would report a peer that did not answer as one that answered wrongly. Both outcomes are in that class, so a client's delivery state is the same; the caller may retry with the same message ID. An honest encoder never writes zero bytes, so no legal answer is lost to the rule. (`tests/direct-v2/tests/stage6_malformed_frames.rs`: a response never written and a responder timing out first both read `PeerUnreachable`; a response of unreadable bytes stays `ProtocolViolation`.)

This layout is the response wire of `/interweave/direct/3.0.0`. It changes only additively, and only behind a new protocol id: a new reason code, tag or field is never sent on 3.0.0. The frozen vectors are `fixtures/direct-v3/direct-response-v2-frame.json`.

`no_route` deliberately collapses endpoint unknown, endpoint disabled, no active lease, missing default endpoint, and endpoint-specific policy denial. All such branches use the same wire code/response shape and shared response encoder. Exact response-time equality is **not** promised; scheduler/registry/policy differences can remain observable to a trusted probing peer, so this residual timing oracle is bounded by direct-request rate limits rather than hidden behind artificial sleeps.

A sender validates every remote response field before caching or surfacing it. `AcceptedV2.message_id` must equal the request ID. `resolved_destination_endpoint` must be 52 ASCII bytes and satisfy EndpointId grammar; for an explicit destination it must equal the requested EndpointId exactly. Invalid/mismatched response metadata is a local `ProtocolViolation`, creates no positive dedup/route result, and is never rendered as trusted Claude/human metadata.

## Semantics

- one request-response exchange per direct message;
- a new substream may be opened per exchange while the underlying peer connection is reused;
- default total deadline: 10 seconds;
- sending to the local profile PeerId is `InvalidArgument`; self-dial never occurs;
- sender applies endpoint outbound narrowing policy, then profile PeerTrustPolicy, before dialing;
- receiver authenticates PeerId, applies profile trust, validates frame, resolves endpoint/default route, applies endpoint inbound narrowing policy, checks active lease/queue capacity, and deduplicates;
- `AcceptedV2` is sent **only after** the resolved endpoint's bounded local event queue accepted the normalized direct event;
- `AcceptedV2` does not mean the human/Claude/application processed it;
- sender performs no automatic retry after timeout/connection failure;
- caller may retry with the same message ID;
- concurrent messages are unordered;
- one connection failure may fail all in-flight exchanges; each reports independently;
- cancellation only stops local waiting when already in flight.

The normalized direct dedup key is:

```text
(mode=direct, source_peer, source_endpoint, destination_selector[Explicit(id)|Default], message_id)
```

A positive duplicate entry stores the first `resolved_destination_endpoint` plus **DirectContentFingerprintV1** from `contracts/ENDPOINTS.md`: SHA-256 over the fixed domain-separated binary framing of media presence/length/value and payload length/value. Empty media type is invalid; timestamp and route fields are excluded. After current trust/structural/direct-ingress rate admission, a retry with the same key and matching fingerprint returns `AcceptedV2` for that stored route without local re-delivery, even if the profile default subsequently changes. A rate-limited retry may instead receive coarse `overloaded`; that does not delete or mutate the prior positive dedup entry. The same key with different content is rejected as malformed/duplicate conflict. `sent_at_ms` may differ on retry and is not part of the fingerprint.

Before route resolution, the runtime atomically acquires a bounded in-flight reservation for a cache miss. Only the owner executes route/queue admission. Matching concurrent duplicates attach as waiters and receive the same eventual response; content-fingerprint conflicts fail. Per the ADR-0019 amendment of 2026-08-27, that retention binds from the stage whose admission yields while holding a reservation; where admission is synchronous the branch is unreachable and must not be answered as exhaustion. The bound on waiters is unaffected and applies in every stage. Limits are 128 global / 8 per source PeerId by default, ceilings 512 / 32, aligned with direct in-flight admission, and they count outstanding **requests** rather than distinct keys — an attached waiter is charged for exactly as the owner is, because it holds a response channel until the owner resolves; overflow is `overloaded` and never creates a parallel enqueue path. This is required to uphold local at-most-once presentation under concurrent retransmission, not only sequential retry.

## Peer not connected

For an authorized target:

- no usable candidate addresses -> `PeerUnknown` without ad hoc discovery;
- usable candidates -> ConnectionManager may dial under command deadline;
- connection failure -> `PeerUnreachable`;
- the peer supports no direct protocol id this node offers — a node of a release before Stage 22's, which speaks `/interweave/direct/2.0.0` alone (§Protocol family) -> `ProtocolUnsupported`, the unsupported-major outcome, with no frame sent;
- the exchange opened but the remote never answered — the response stream ends with no bytes (§Response byte layout, A 2026-10-26) -> `PeerUnreachable`;
- successful v2 exchange but remote `no_route` -> `RemoteEndpointUnavailable` locally.

Candidate addresses the profile configures — the initial state of an enabled static-bootstrap provider, overlay entries included, and of an enabled peer-cache provider — are in the substrate's address book before the runtime's start returns and so before any session can send: the `PeerUnknown` check above reads that book directly rather than through the discovery driver, so a configured peer is not `PeerUnknown` for want of a first discovery round (`contracts/TRANSPORT.md` §send, ruled 2026-11-26). On a wildcard listener (`0.0.0.0`, `::`) a private-range candidate enters the book only beside the private listener ADR-0052 rule 3 asks for, and this holds because the substrate's `listen` answers a wildcard listen only after its listener has reported the addresses its interface watcher holds; as this was found, `listen` answered at the listener's first, loopback, address, and a cached LAN peer was refused at start and answered `PeerUnknown` in about one start in six. The limit: an interface the watcher has not seen when the listen is answered — on Android, where `getifaddrs` answers nothing until the platform's view arrives, and on a host whose address dump reaches the watcher in more than one read (hundreds of addresses), whose later part may come after the answer — is not waited for; a candidate refused for want of it is offered again at the next discovery round after the boundary widens, not before start returns. Addresses that mDNS or Kademlia would learn are not waited for.

The direct operation does not command discovery providers to perform an ad hoc global search.

## Endpoint omission

`destination_endpoint_len = 0` requests the receiver's explicit `default_direct_endpoint`. It never means all local clients. The `AcceptedV2` response reports the resolved endpoint so the caller can expose exact transport routing in diagnostics/result metadata.

## Graceful shutdown

Stop accepting new direct requests, respond `shutting_down` where possible, allow existing exchanges a short bounded grace, then close. Endpoint leases are revoked as part of daemon shutdown.

**The refusal is the stop's first act, and what was admitted is handed over (A 2026-11-17, the owner's word).** From the start of a stop (a signal, or `admin.shutdown`) the node is draining: a new direct request is answered `shutting_down` and enters no endpoint queue, before any session is closed and before any lease is released. A message answered `AcceptedV2` before that moment sits in the queue of a session that is still open, and that session is handed what its queue holds before it is closed, inside the stop's grace (`contracts/LOCAL-IPC.md`, the shutdown order). A message a session has not taken when the grace ends is dropped and counted in the stop's dropped-event count, a floor on what was dropped (`contracts/LOCAL-IPC.md`, the shutdown order). `AcceptedV2` keeps its meaning, admission to a bounded endpoint queue: a client that needs delivery still owes itself a receipt. What changes is that a stop is no longer a place where an admitted message is dropped while its reader is reading. As the gap was found, the daemon closed its sessions first and put the runtime into its drain after, and a session ended on the stop without a last read of its queue: a message answered `AcceptedV2` in the last milliseconds before a far daemon's SIGTERM, or just after it, never reached the receiving client in 6 of 12 runs, with no refusal for the sender to retry on (`tests/automated-client/runs/2026-11-17-fleet-control-path.md` §3, finding F1). Built for the daemon in the change that followed: `tests/automated-client`'s `an_inbound_direct_after_the_stop_began_is_refused_shutting_down` and `a_message_admitted_before_the_stop_is_read_before_the_close` hold it between two daemons, and `ipc-server`'s `a_stop_writes_what_the_session_held_before_the_close` and `a_stop_counts_what_a_client_that_does_not_read_left` the hand-over and its count. When that was recorded it was not built for the embedded binding: its stop drained and stopped the runtime in one command, which stopped at once when no exchange was in flight, so an in-process session was handed nothing, and the Android Service closed its facade's session before that stop, so on a phone the session closed before the refusal began. Built for the embedded binding since: `EmbeddedHost::drain` is the stop's first step and refuses from its return while the in-process sessions stay open, the owner then reads each session's queue and closes it, and `EmbeddedHost::stop` ends the runtime. `local-client-conformance`'s `the_embedded_stop_refuses_first_and_hands_over` holds it between two embedded hosts (a message answered `AcceptedV2` before the drain is read after it, one sent after it is refused); the Android Service stops in that order, held by `android-platform`'s `a_message_admitted_before_the_stop_is_unread_after_it` (the message is in the store, unread, after the stop) and `transport-client`'s `a_close_commits_what_the_session_still_held_unread`. The Service's drain before its close is held by `android-platform`'s `a_send_while_the_close_waits_is_refused`: with the facade's close held waiting, a peer's send during the stop is refused, and it fails with the drain removed. None of the four is a device record: all run on the host.

**A client's close hands over too (A 2026-11-28, architect-cto's acceptance).** When a local session closes in order, the end of its lease and the taking of its endpoint queue are one step of the runtime — the Swarm task's one `ReleaseSession` command: nothing is admitted to that queue after it, a request after it is answered `no_route` as for any unleased endpoint, and every message answered `AcceptedV2` before it that the session had not taken goes to the closing session in that command's answer rather than being dropped (`contracts/LOCAL-CLIENT.md`; over IPC, `contracts/LOCAL-IPC.md` §Disconnect/reconnect, which bounds and counts the hand-over as the stop's). `AcceptedV2` keeps its meaning. An administrative end — a revoke, a disable, a removal — still closes the queue and discards what it held, its holder told by the revocation notice. A session dropped without its close is handed nothing: the in-process binding's teardown on drop sends the same command without waiting for its answer, so what the release took is dropped, uncounted, as before (a count of it, from the answer nobody received, is possible and not built). As the gap was found, the release closed the queue and discarded what it held, and the IPC server's end of a connection a client had closed in order went the same way. Built in the change that followed: `transport-runtime`'s `taking_a_queue_returns_what_it_held_and_closes_it`, `transport-libp2p`'s `a_release_takes_the_sessions_queue_with_its_lease`, `endpoint-routing`'s `release_frees_the_endpoint_and_invalidates_its_lease` (what was admitted is in the release's answer), and the composition's `a_close_hands_over_what_was_admitted_and_after_it_no_route`.

## Protocol family / future compatibility

A future compatible implementation may advertise multiple request-response protocol IDs where safe. Endpoint-addressed sends must never silently downgrade to a protocol that cannot preserve endpoint routing.

**A newer minor lists the older minor's id beside it** (A 2026-10-08): an implementation that speaks `/interweave/direct/3.<n>.0` advertises every older `3.<m>.0` it still speaks until that minor is retired by its own record, and LISTS THEM NEWEST FIRST: the dialer proposes its ids in the order it lists them and the listener takes the first it supports (libp2p request-response over multistream-select), so two peers settle on the newest id they share — a 3.1.0 peer and a 3.0.0 peer exchange on 3.0.0 in either dial direction (when the newer peer dials, the older one declines the 3.1.0 proposal; when the older dials, only 3.0.0 is proposed). A peer that offers only a newer id against an older peer fails `UnsupportedProtocols`, which is the unsupported-major shape by design: a minor that drops the older id IS a major, and a new id is where every change a 3.0.0 reader would refuse — a new tag, field or reason code in the response layout above — goes, additively behind it. The upgrade matrix's direct rows (`testing.md` §Compatibility fixtures) assert exactly this.

**`3.0.0` against `2.0.0` is that unsupported-major shape, in both dial directions** (ADR-0067 decision 7): a node of the Stage 22 release offers `3.0.0` alone and a node of any release before it `2.0.0` alone, so whichever dials, multistream-select finds no shared id, the dialer's send fails `UnsupportedProtocols` and is reported `ProtocolUnsupported`, and no `DirectMessageV2` is written on either side — the failure is the protocol's and by name, never a `malformed` answer to a frame. The connection itself is unaffected: the two nodes still authenticate each other, and what fails is this protocol's stream. The upgrade matrix's previous-build rows for the direct wire assert the refusal, in place of the exchange they asserted while both builds spoke `2.0.0`.
