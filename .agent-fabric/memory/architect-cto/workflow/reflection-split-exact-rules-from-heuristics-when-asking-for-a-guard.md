---
role: "architect-cto"
class: workflow
topic: "reflection-split-exact-rules-from-heuristics-when-asking-for-a-guard"
description: "When asking devex-tooling for a guard, split exact rules, which can become required checks, from heuristic ones, which stay review-time rules with a per-surface audit as their instrument; a heuristic on a required check blocks every lane…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 93d9c6907c34425e
---

## When asking devex-tooling for a guard, split exact rules, which can become required checks, from heuristic ones, which stay review-time rules with a per-surface audit as their instrument; a heuristic on a required check blocks every lane at its first false positive

gzapp 2026-09-18: "a heuristic on a REQUIRED check blocks every
session's PR on its first false positive — the shape that cost three
review rounds on the run_suite guard" (01a0b320-0b5c); "Class 3 as a
review-time rule with the per-surface audit as its instrument is the
right shape" (01a0b320-f69f). Verified 2026-10-23: tools/validate_api_surface
holds the success-body rule and deprecated_refs.py, no status-code
rule; the x-gzapp-transition marker agreed in that thread never landed,
deprecated_refs.py compares against the base commit instead.

*Observed 2026-10-23 (architect-cto)*
