# Bounded queues and conservative resource limits

**Status:** Accepted; endpoint-routing limits added by ADR-0030.

## Context

Network producers can outrun local consumers. Unbounded queues are denial-of-service risks. Model B adds endpoint configuration/directory state and requires direct acceptance to reflect target endpoint queue admission.

## Decision

Keep 48 KiB payload and 128 KiB IPC JSON-body ceilings; 128-byte ChannelId; add 64-byte EndpointId, max 64 configured endpoints with no per-profile default (A 2026-10-21: the default of 16 this record carried was held by no key or code; `config.schema.yaml`'s `max=64` and `MAX_ENDPOINTS` are the bound), default 16/max32 advertised endpoints, one endpoint lease per local data-plane session (IPC connection on desktop, embedded session on Android), short-lived endpoint-directory cache. Direct dedup in-flight reservations are explicitly bounded at 128 global / 8 per source peer by default (ceilings 512 / 32), aligned with direct in-flight admission. Existing queue/client/discovery bounds remain.

Direct inbound messages are rejected as overloaded before `AcceptedV2` when the resolved endpoint queue is full. After Noise/trust admission, every inbound direct request also consumes a per-trusted-PeerId token bucket (default 120/minute, burst 32) and a global bucket (default 1200/minute, burst 256); rate overflow returns coarse `overloaded` before endpoint routing.

Broadcast retains bounded best-effort local drop behavior, and additionally consumes its own per-trusted-PeerId and global ingress buckets with the same defaults, accounted **separately from direct's**: one mode's traffic must not spend the other's allowance. A broadcast message over the rate is dropped before local delivery admission. It is not refused on the wire — GossipSub has no per-message refusal to a publisher, and a validation verdict answers whether the message was valid and authorized, which a local rate says nothing about — so the mesh still sees the node's ordinary Accept/Ignore/Reject report. What a session lost this way is readable by the session itself (A 2026-10-24): two monotonic counts — the copies its own queue dropped or could not take, and the copies the ingress gate refused on a channel it held — naming no peer and no channel, and never what the backend dropped before admission, which nothing counts. They differ in what they promise: an overflowed broadcast was admitted and its dedup entry recorded, so within the dedup window a republish is a duplicate and the overflow count is a lower bound on broadcasts lost; a refused broadcast has no dedup entry, so a republish may still arrive and each refused copy counts — a signal, not a count of distinct losses. Recording a refused id so that a republish is dropped was rejected: it would make a loss permanent to keep a number honest and spend at the gate the work this bucket bounds (`LOCAL-IPC.md` `session.losses`, IPC 2.11; `LOCAL-CLIENT.md` §7 item 15).

What that bucket bounds is stated exactly, because it is less than a reader would assume. It bounds the per-message work this node does **after** the message has been validated and answered: content fingerprinting over the payload, the duplicate-cache insertion, and the per-session fan-out that copies the payload once per joined consumer. It does **not** bound signature and source verification, which the GossipSub backend performs before the runtime is given the message at all, and it does **not** bound envelope decoding, because the mesh is owed a validation verdict and that verdict depends on whether the envelope decodes. Those two are bounded per message by the transmit ceiling rather than per unit time. A limit that could refuse work before the verdict would have to answer the mesh without knowing whether the bytes were valid.

## Alternatives considered

Unbounded channels; persistent overflow spool; acknowledge direct before local queue; all-client direct fan-out; unlimited endpoint directory; reduce payload just to fit smaller IPC.

## Consequences

Direct endpoint routing reduces local memory amplification versus v1 fan-out. Endpoint presence/catalog state stays bounded.

## Security implications

Bounds limit network/local endpoint probing and slow-consumer exhaustion. Early length/EndpointId validation prevents attacker-directed allocation. Per-PeerId direct ingress buckets limit a malicious-but-trusted peer without treating spoofable source EndpointIds as security principals. Pre-Noise handshake bounds are handled by the libp2p security/connection layer before a PeerId exists.

## Operational implications

Counters show endpoint lease conflicts, route/no-route/overload, directory bounds, and ordinary queue saturation; a data session reads its own broadcast losses, so a consumer can say that events were lost (the overflow count) or refused and perhaps republished (the refusal count) without an operator's view (A 2026-10-24).

## Implementation implications

Use bounded channels/semaphores, fixed directory caps, and bounded direct reservation maps. Reservation overflow returns overload instead of opening a parallel enqueue path. Golden fixtures prove max payload plus endpoint metadata fits 131072-byte IPC v2 body.

## Revisit conditions

Only with measured evidence and compatibility/security review.


ADR-0035 extends the same bounded-resource rule to AutoNAT probes, relay reservations/circuits/control requests, DCUtR attempts, and total/per-peer connections. Reachability behaviours share the root dial/connection budget and cannot create unbounded work outside it.

## Amendments

Full notes: [`history/0026-amendments.md`](./history/0026-amendments.md).

| Date | Amendment | Effect |
|---|---|---|
| 2026-10-24 | A session reads its own broadcast losses | `session.losses` (IPC 2.11; `DataSessionPort::losses` at every binding): the copies the session's queue dropped or could not take, and the copies the ingress gate refused on its channels, monotonic for the session's life, neither counting the backend's drops, no peer or channel named; the overflow count a lower bound on broadcasts lost (dedup entry recorded, a republish a duplicate within the window), the refusal count a signal (no dedup entry, a republish may arrive, each refused copy counts); a session that declines route notices over IPC speaks below 2.4 and reads none. |
| 2026-10-21 | Configured endpoints have a maximum of 64 and no per-profile default | Decision: the "default 16" for configured endpoints was held by no configuration key and no code; the bound is `config.schema.yaml`'s `max=64` (`MAX_ENDPOINTS`), which ADR-0061's identity cap cites; ENDPOINTS.md's resource table reads the same. |
| 2026-08-27 | Broadcast ingress consumes its own token buckets | Inbound broadcast is rate-bounded per trusted peer and globally, accounted separately from direct so neither mode can spend the other's allowance |
