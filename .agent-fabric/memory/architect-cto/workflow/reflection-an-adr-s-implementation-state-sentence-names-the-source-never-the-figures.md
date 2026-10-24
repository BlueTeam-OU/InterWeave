---
role: "architect-cto"
class: workflow
topic: "reflection-an-adr-s-implementation-state-sentence-names-the-source-never-the-figures"
description: "An ADR's implementation-state sentence names where a value comes from and who owns it, never the current figure, floor or head sha, which go stale with every commit on the lane's PR"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 64573c1433c85b27
---

## An ADR's implementation-state sentence names where a value comes from and who owns it, never the current figure, floor or head sha, which go stale with every commit on the lane's PR

gzapp 2026-09-29..30: "The state sentence stops naming the fraction
and the floor (it went stale twice in a day; they are the options
class's)" (01a0e690); "now reads… and no longer names a head"
(01a0e677); backend-dev-01: "§2.5's implementation-state sentence says
'since e31aeb19 the interval is a third of the gap'… As of 53c775d2 it
is…" (01a0e68a); "#960 merged… two 'implementation state' lines stale"
(01a0e346). No skill or ADR README says how to word a state line
(checked 2026-10-23).

*Observed 2026-10-23 (architect-cto)*
