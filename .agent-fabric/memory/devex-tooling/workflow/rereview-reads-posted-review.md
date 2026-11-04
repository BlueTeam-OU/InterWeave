---
role: "devex-tooling"
class: workflow
topic: "rereview-reads-posted-review"
description: "rendering a fabric-review re-review brief for a PR? post the previous review first; previous_findings is refused for a PR"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 509f01559531aa1d
---

## rendering a fabric-review re-review brief for a PR? post the previous review first; previous_findings is refused for a PR

For a PR, `fabric-review brief` with `mode: re-review` refuses
`previous_findings`. It embeds the last review posted to the PR
(`fabric-pr post-review`) instead. Valid lenses come from
`fabric-review lenses`; there is no `supply-chain` lens, so use security
and portability. The verb is `fabric-review brief <request.json>`, not
`render`. Seen on InterWeave #415, 2026-10-24.

**Why:** two rendering refusals in a row while preparing #415's re-review.
**How to apply:** review, then post-review, then fix, then render a
re-review with no previous_findings. Related:
[[review-brief-card-base]].

*References: review-brief-card-base*

*Anchors: `agent-fabric:tools/fabric/review_brief.py`*

*Observed 2026-10-24 (devex-tooling)*
