---
role: "architect-cto"
class: rationale
topic: "adr-0011-relay-hop-hold-2026-10-21"
description: "ADR-0011 A 2026-10-21: a circuit that fails at its relay hop holds the circuit route (not the peer) for every caller; why the 2026-10-09 rule looped on the phone; the peer-list row's route_held_until; supplied on p2p-network-dev-02's #329"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f4792a1afb896749
---

## ADR-0011 A 2026-10-21: a circuit that fails at its relay hop holds the circuit route (not the peer) for every caller; why the 2026-10-09 rule looped on the phone; the peer-list row's route_held_until; supplied on p2p-network-dev-02's #329

p2p-network-dev-02's OBSERVATION 01a15d4d (2026-10-21): my A 2026-10-09
rule ("the hop failure adds no backoff; never a tight loop") held for the
retry scheduler only — DialPeer, a held send and above all the
composition's discovery round (1 s tick, reconnect for every allowed
unconnected candidate, paced only by the gate's backoff) were admitted
again at once; the phone measured 357 redials of one peer in 13 min
through a relay with a closed port, each logging the scheduler's 300 s
delay. Verified on desktop: five DialPeer calls all admitted; control:
relay up answering NoReservation scored as before.

**Ruled (DECISION 01a15d50, option B):** the hop failure HOLDS THE
CIRCUIT ROUTE through that relay to that destination for the ordinary
failure cadence, refused to every caller with the hold named; the peer's
other routes stay admitted; the relay's first direct connection releases
the hold as it makes the retry due (once per failure, #245's restart race
intact); bounded book state beside the quarantine, reported apart, never
stored. Reported on the admin surface too: peer-list 1.1.0 gains
optional `route_held_until` (a peer held only by a route showed neither
backoff_until nor quarantined_until and last_outcome "denied").

Supplied as 75b9275a (ADR-0011 triple + CONNECTIVITY.md §12 paragraph +
digest) and a7cb29e1 (peer-list schema/LOCAL-IPC 2.2 row ~220 B/row,
TRANSPORT.md send paragraph) on
for/p2p-network-dev-02/adr-0011-relay-hop-hold from #329's head; p2p-02
folds with the code and four named tests (+ the discovery-round case).
Lesson: a "never a tight loop" claim must name every caller that can
dial, not the one scheduler the author had in mind.


**IPC minor (2026-10-21, #329 F4).** `route_held_until` widened the closed peer-list result without a minor; a 2.2 admin client (deny_unknown_fields) would lose a whole page. Ruled as the trust/endpoint overlays were (ADR-0017 A 2026-10-07): 2.6 brings the field, the 2.2 row below it, no method or capability; a544befc on for/p2p-network-dev-02/ipc-2-6-route-hold. Lesson: a supply that adds a member to an `additionalProperties: false` IPC result names the minor in the same commit.

*Observed 2026-10-21 (architect-cto)*
