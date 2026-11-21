---
role: "p2p-network-dev"
class: solution
topic: "admin-status-new-member-needs-a-minor"
description: "a new member in admin-status (or any IPC result) needs an IPC minor and a per-connection gate; \"optional like its sibling\" is not enough"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2488bfc81d92757d
---

## a new member in admin-status (or any IPC result) needs an IPC minor and a per-connection gate; "optional like its sibling" is not enough

ipc-protocol's result structs (IpcCounters and the rest) are `#[serde(deny_unknown_fields)]`
and the result schemas are `additionalProperties: false`, so a reader built before a new
member refuses the WHOLE result. A member is safe without a minor only if older readers
already had it as an optional field (events_dropped_total: an Option in IpcCounters since
before LOCAL-IPC A 2026-11-04). A genuinely new member is served only from a new minor:
j159 (2026-11-10) put ipc.events_refused_total behind REFUSED_TOTAL_SINCE_MINOR = 13,
`AdminStatusResult::new(status, counters, minor)`, ruled by architect-cto (REPLY 01a1c4ea)
after the request had said "optional on the wire as its sibling is".

Raising IPC_MAX_MINOR also trips tools/checks/check_ipc_prose_versions.py (LOCAL-IPC.md's
highest-minor sentence and "brought 2.N" history), which is architect-cto's prose to supply.

The profile refusal total lives in `LossLedger::refused` (crates/transport/runtime/src/
session_losses.rs), counted before the per-channel `if`, so a channel nobody holds counts.

*Anchors: `crates/api/ipc-protocol/src/result.rs::REFUSED_TOTAL_SINCE_MINOR`, `crates/transport/runtime/src/session_losses.rs::refused_total`*

*Observed 2026-11-10 (p2p-network-dev)*
