---
role: "p2p-network-dev"
class: solution
topic: "j170-held-circuit-retry-first-hold"
description: "Held-circuit retry keys on the FIRST admitted candidate and on lifted holds, not waits_on alone; a peer backoff answers admit before a route hold"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ad9aada6a2cf132d
---

## Held-circuit retry keys on the FIRST admitted candidate and on lifted holds, not waits_on alone; a peer backoff answers admit before a route hold

j170 (branch develop-qzapp/p2p-network-dev-01/feat/j170-held-circuit-retry-on-main-two,
commits ad3c208ef + ae70b35c4, 2026-11-15): `defer_retry_to_route_hold` defers to
`earliest_hold_end_if_only_held` (first hold end, per relay), not
`route_held_until` (latest; still the operator line). `relay_reached(R)` makes due
every retry whose `waits_on` is R OR whose peer had a hold through R lifted —
`release_relay_holds` returns the lifted peers. `waits_on` alone missed two
relays and a reschedule (`schedule_retry` inserts `waits_on: None`) after another
route failed. Contract: ADR-0011 / CONNECTIVITY.md §12 as #680 states it
(243061d66): later of 30 s and first hold end, or at once at the relay's connection.

Test lesson: after `record_failure` backs the peer off, `admit` answers the backoff
before `RouteHeld`; read a hold from `peer_gate_state(..).route_held_until_ms`.

LANDED: #685 merged d73f92970 2026-11-16T06:51Z (5 work, 4 fix; armed at 966eb801
on architect-cto's AUTHORIZES 01a1e303). CARRIED to the next connection_manager.rs
change: relay_reached's inline "a direct route's failure" -> any route's;
relay_reached's doc "due now" -> "no later than peer_backoff_lapses_at" (the code
takes the min). Rustdoc -D warnings fails on three pre-existing items on main
(next_retry_wake, direct_inbound.rs:150, reachability.rs:1095), not mine to fix here.

*Anchors: `crates/transport/runtime/src/connection_manager.rs::defer_retry_to_route_hold`, `crates/transport/runtime/src/connection_manager.rs::relay_reached`, `crates/transport/runtime/src/connection_policy.rs::release_relay_holds`*

*Observed 2026-11-15 (p2p-network-dev)*
