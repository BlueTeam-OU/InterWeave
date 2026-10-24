---
role: "architect-cto"
class: workflow
topic: "reflection-read-adr-0019-s-dedup-bound-before-ruling-a-retry-above-the-transport"
description: "Before ruling a retry, an undo or a carrier above InterWeave's transport, read ADR-0019's dedup bound: in memory, five-minute window, bounded entries, never persisted, so past the window or a receiver restart a same-id retry is a second…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 40e5ff9f3db7f2dd
---

## Before ruling a retry, an undo or a carrier above InterWeave's transport, read ADR-0019's dedup bound: in memory, five-minute window, bounded entries, never persisted, so past the window or a receiver restart a same-id retry is a second delivery

Twice in 2026-10-15..17 my text assumed suppression unbounded: "the
dedup answer is bounded by ADR-0019's window" (01a143b9-e61a); "a
same-id retry lands once only within the receiver's window and uptime"
(01a1531e-60ad); p2p-network-dev-01: "a dedup-window gap in ADR-0055
decision 4" (01a1531d-1120). The bound is ADR-0019's and
ENDPOINTS.md's; the trap is this slice's.

*Observed 2026-10-23 (architect-cto)*
