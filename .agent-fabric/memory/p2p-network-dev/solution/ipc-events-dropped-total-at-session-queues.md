---
role: "p2p-network-dev"
class: solution
topic: "ipc-events-dropped-total-at-session-queues"
description: "where admin.status's ipc.events_dropped_total is counted and why it took no IPC minor (j144, PR #554)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 79847451803bce85
---

## where admin.status's ipc.events_dropped_total is counted and why it took no IPC minor (j144, PR #554)

`ipc.events_dropped_total` is one profile counter, `SessionQueues::dropped_total`
(crates/transport/runtime/src/session_queue.rs), moved on both drop branches of
`push` (oldest dropped, NotOpen) for ANY session, joined or not — not the 2.11
per-session `LossLedger` sum, which ends with the session and keeps nothing for
an unjoined one (architect-cto's ruling, LOCAL-IPC A 2026-11-04, supply a6aec03f).
Carried: libp2p `RuntimeStatus.broadcast_queue_overflow_total` → neutral
`AdminStatus.events_dropped: Option<u64>` → `AdminStatusResult::new`.

No minor: the member was already an optional property of admin-status 1.1.0, so
emitting it is not ADR-0017's widen-behind-a-minor case — I had planned 2.13 and
the ruling said no. Ask before bumping a minor for a member the schema already has.

The daemon's `stopped events_dropped=` log line is a DIFFERENT count (composition
`Diagnostics.events_dropped`, runtime→consumer channel). Related: [[non-reading-ipc-client-is-closed-by-write-stall]].

*References: non-reading-ipc-client-is-closed-by-write-stall*

*Anchors: `crates/transport/runtime/src/session_queue.rs::dropped_total`, `crates/api/local-client-api/src/binding.rs::events_dropped`*

*Observed 2026-11-04 (p2p-network-dev)*
