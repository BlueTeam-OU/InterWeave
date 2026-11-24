---
role: "ux-designer"
class: workflow
topic: "review-chain-label-collision"
description: "when a partial review leaves fabric-pr arm refusing on its findings, and fabric-review cannot chain a re-review from it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 26f907d81fb1fb40
---

## when a partial review leaves fabric-pr arm refusing on its findings, and fabric-review cannot chain a re-review from it

`fabric-pr arm` holds every `Finding:` open until a later review's `Judgement:` of its label says fixed. A judgement goes first to the newest finding of that label in its own chain, so a later round's F1-F4 hide an earlier chain's F1-F4. A review with partial coverage (an unread merged path) cannot start a re-review either: `fabric-review next` refuses, and `post-review` refuses a narrower base than the merge-base. On PR #581 (2026-11-09) the way through was:
1. a full review of merge-base..head;
2. fix and re-review that chain;
3. `--defer <head8>:F<n> "<why>"` for each finding of the stranded partial chain, citing the fix commits and the full review that did not find them again.

**Why:** round 4 of #581 had coverage "not read" for the merge's paths, so its six fixed findings could only be lifted by defer.
**How to apply:** brief a review so it reads every path in its range (scope the merge's paths out as read-not-in-scope, never as not read), or review merge-base..head from the start. Related: [[review-rounds-mechanics]].

*References: review-rounds-mechanics*

*Observed 2026-11-09 (ux-designer)*
