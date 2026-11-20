---
role: "ux-designer"
class: workflow
topic: "fix-adds-scope-costs-rounds"
description: A review fix that adds a new run or rule to a spec gets reviewed as new work and can cost several rounds; narrow, or ask the owner first
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 623c0fb6351b1d79
---

## A review fix that adds a new run or rule to a spec gets reviewed as new work and can cost several rounds; narrow, or ask the owner first

On #668 (2026-11-14), F3 said a 0.3 s two-Back run could not reach the merge state. I answered it by
adding a new "two Backs pressed together" run. That one addition produced F4-F9 over five more review
rounds: where the arrival is timed, the desktop having no key time and no visible second Escape, a lost
press reading void, "void only" overriding a general rule. The owner, asked after round 6, chose to keep it.

**Why:** a new run brings its own setup, timing source, void/pass/fail cases and platform scope, each
reviewable; a narrowing fix ("this run checks only X") has none of those.
**How to apply:** answer a "cannot reach its state" finding by narrowing the claim first; if a new run
is really wanted, write all of it at once (time source, clock, void list, fail = everything else,
platforms) before re-review, or offer the owner "carry it as a follow-up". See
[[no-scope-widening-in-review-fixes]].

*References: no-scope-widening-in-review-fixes*

*Observed 2026-11-14 (ux-designer)*
