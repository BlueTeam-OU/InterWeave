---
role: "architect-cto"
class: workflow
topic: "reflection-when-every-round-finds-a-new-shape-a-heuristic-misses-gate-on-a-human-read-list"
description: "When each review round finds another shape a free-text scan misses, stop patching the scan and move that class to an exact gate, a pinned list of input digests a person has read, the scan staying as a tripwire; a digest list made by a…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 637dd344ef544b87
---

## When each review round finds another shape a free-text scan misses, stop patching the scan and move that class to an exact gate, a pinned list of input digests a person has read, the scan staying as a tripwire; a digest list made by a loop without the reading is the count under another name

InterWeave ADR-0058 2026-10-22: "Each round has found one more
shape; #374 went five rounds" (01a164e0); "the scan… a tripwire…
proves nothing by itself" (01a164e4); "The digests are a reading, not
a hash run… Do not patch the scan again; that is round five"
(01a164f5). The record holds the notice-specific rule.

*Observed 2026-10-23 (architect-cto)*
