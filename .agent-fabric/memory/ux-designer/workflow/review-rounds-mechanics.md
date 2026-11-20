---
role: "ux-designer"
class: workflow
topic: "review-rounds-mechanics"
description: "how to chain fabric-review rounds and post a subagent's report whole without retyping it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7084390d543f47de
  - 759d99191c015a20
---

## how to chain fabric-review rounds and post a subagent's report whole without retyping it

`fabric-review next` takes the PREVIOUS round's request (`<pr>-rrN.json`, which it writes beside the first
yaml), with `--pr N --head <sha>`; given the first yaml it refuses because the base is wrong. Its stdout is
already the rendered brief: dispatch the header inline and point the agent at the brief file. To post a
report whole (`fabric-pr post-review` refuses one without its dispatch block), walk the agent's output
JSONL for the string containing both "agent-fabric-dispatch v1" and the "Finding:" lines; reading the
assistant text blocks alone missed it (PR #480, 2026-10-27).

**Why:** retyping a 10 KB report by hand cost a whole turn in round 1.
**How to apply:** every review round on a PR; see [[no-scope-widening-in-review-fixes]].

The job gates (#642, 2026-11-12):
- `fabric-pr post-review` refuses while the PR's job is not active. Post each round's report before the job is blocked or switched away. The reports are kept in ~/review-reports/ (prN-rM.md, with the rrM.json beside it).
- `fabric-jobs start` refuses a job blocked on a supplier's job until that supplier marks theirs delivered. Ask them to mark it, and do not fold their for/ branch before then.

*References: no-scope-widening-in-review-fixes*

*Observed 2026-11-13 (ux-designer)*
