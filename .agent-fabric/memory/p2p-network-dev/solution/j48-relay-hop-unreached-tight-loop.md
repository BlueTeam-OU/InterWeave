---
role: "p2p-network-dev"
class: solution
topic: "j48-relay-hop-unreached-tight-loop"
description: "A circuit through a down relay was redialled each second by the composition's reconnect round; fixed by a route hold (ADR-0011 A 2026-10-21) on #329"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 1ec7bb70e3f8b542
---

## A circuit through a down relay was redialled each second by the composition's reconnect round; fixed by a route hold (ADR-0011 A 2026-10-21) on #329

rust-ui-dev-01 measured on a Pixel (01a15d44, 2026-10-21): one peer's relay_circuit dial
failed about once a second, and each line said delay_ms=300000. The cause was that
ADR-0011 A 2026-10-09 put no backoff on a hop failure, so every caller with its own
cadence was admitted again at once. The caller was the composition's discovery round,
which ticks every 1 s and reconnects every allowed peer that is not connected. It was
not a client send.

The fix, option B, is merged with #329 (1a4fa590, 2026-10-21):
- `ConnectionPolicy` holds the failed circuit (`route_held_until_ms`). `admit` refuses
  `RouteHeld` for every circuit to that peer through the same relay, whoever dials.
- `relay_reached` lifts the hold.
- The hold is reported as `PeerGateState::route_held_until_ms`, as the admin row's
  `route_held_until` (served only from IPC 2.6), and as the reconnect_held log line.

Review lessons, F1 to F5:
- A new refusal class must also be handled by the retry scheduler. Releasing the claim
  re-offered the peer every tick into the same refusal, so it is now deferred to the
  hold's end.
- A scheduler walk's verdict must be the aggregate of its candidates' refusals, not
  the last one (`record_refusal`).
- An optional field on a closed IPC result needs a new minor after production
  (deny_unknown_fields). The schema text is architect-cto's.
- Dedupe of a refusal log line must end when the streak ends.
Related: [[j46-unobserved-vs-unavailable-dns]].

*References: j46-unobserved-vs-unavailable-dns*

*Observed 2026-10-21 (p2p-network-dev)*
