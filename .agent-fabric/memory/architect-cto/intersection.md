---
role: "architect-cto"
class: intersection
description: "For any wire or configuration change, first ask whether a store-released mobile binary reads it: a web sub-app is co-deployed and a stale tab ends at reload, a phone binary lives for months, so anything it binds (a required member, a…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b7b598d86380fb0a
---

## For any wire or configuration change, first ask whether a store-released mobile binary reads it: a web sub-app is co-deployed and a stale tab ends at reload, a phone binary lives for months, so anything it binds (a required member, a dictionary baseline, a hostname) needs a versioned change, never an in-place one

gzapp 2026-09-20..21: "The exception never reaches a wire a Flutter
app reads… the reader that assumed presence may be installed for
months" (01a0be99-58aa); DECISION 01a0c226-a671: "the worst skew window
is not a stale SPA bundle but a passenger phone that has not updated".
ADR-018 §5 r6 and ADR-024's 2026-09-21 amendment each record one
instance; the general test across them is written nowhere. Widens the
hostname lock-in slice.

*Observed 2026-10-23 (architect-cto)*
