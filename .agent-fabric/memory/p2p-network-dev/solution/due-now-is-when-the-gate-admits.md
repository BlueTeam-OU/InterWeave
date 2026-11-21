---
role: "p2p-network-dev"
class: solution
topic: "due-now-is-when-the-gate-admits"
description: "A retry made \"due now\" in connection_manager must be clamped to a running peer backoff's end — the gate asks backoff before hold"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 63f5949c026d87bb
---

## A retry made "due now" in connection_manager must be clamped to a running peer backoff's end — the gate asks backoff before hold

In `crates/transport/runtime` the gate (`ConnectionPolicy::admit`) asks the peer backoff BEFORE a route hold, and the backoff answers only for an address with a record. So any site that makes a retry due "now" after a hop failure, or on a relay's connection, must take the later of now and the backoff's end. A hop failure gives the circuit a record and neither sets nor clears the backoff. If it doesn't, the retry is refused `PeerBackoff` on every tick and loses its `next_retry_wake` at the backoff's end (`offered` stays true).

Found on #685 (j170, 2026-11-16) as review F1 in `relay_reached`. The re-review then found the same shape at a second site, `record_relay_hop_unreached`'s `relay_came_up` branch. Both now take `peer_backoff_lapses_at`.

**Why:** `retry_backoff_ms` is a flat 30 s, while `route_hold_ms` doubles on the attempt count. So whether a hold outlasts a backoff depends on attempts. A test fixture needs one prior failure to make the hold (60 s) outlast the backoff (30 s).

**How to apply:** for a new "make due" site, grep `set_due(` and check the gate's order of checks first. `network_added` is safe because it lifts the backoff itself. `learn_address` is safe because an untried address has no record.

*Anchors: `crates/transport/runtime/src/connection_manager.rs::peer_backoff_lapses_at`*

*Observed 2026-11-16 (p2p-network-dev)*
