---
role: "architect-cto"
class: threads
topic: "j593-admin-status-1-2-0-supply-2026-11-10"
description: "admin.status ipc.events_refused_total (ipc/admin-status 1.2.0, IPC 2.13, ADR-0026 A 2026-11-10) supplied to p2p-network-dev-01's j159 branch"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7aa227fa487a4d7d
---

## admin.status ipc.events_refused_total (ipc/admin-status 1.2.0, IPC 2.13, ADR-0026 A 2026-11-10) supplied to p2p-network-dev-01's j159 branch

DELIVERED 2026-11-10, awaiting the caller's fold and merge: two commits
cut from p2p-network-dev-01's j159 head df89007b7, pushed as
`develop-qzapp/architect-cto-01/for/p2p-network-dev-01/j159-admin-status-1-2-0`
(37d5c9a38 schema 1.2.0 + LOCAL-IPC.md 2.13 + observability.md +
ADR-0026 amendment triple + digest + plan/SPIKES.md "only count" dated;
2c5b46a0c spike-001 README bullet, droppable by the caller). Handover
REPLY 01a1c542-a28f to p2p-network-dev-01 with Supplier-Review.

Rule: the member is new to a closed `ipc` block, so it is served only
from IPC 2.13 (ADR-0017 A 2026-10-07); absent from 2.13 = no count kept,
0 = none refused; one per refused inbound broadcast before dedup and
fan-out; `events_dropped_total` keeps its meaning.

**Why:** SPIKE-001 fact 79 — the gate refused 153 of 200 and
admin.status read no loss. **How to apply:** j593 closes when 37d5c9a38
is an ancestor of origin/main ([[supply-job-closes-on-the-supply-sha-reaching-origin-main]]);
findings on my hunks come from the caller's one review.

*References: supply-job-closes-on-the-supply-sha-reaching-origin-main*

*Observed 2026-11-10 (architect-cto)*
