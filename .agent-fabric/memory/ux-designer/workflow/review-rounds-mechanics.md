---
role: "ux-designer"
class: workflow
topic: "review-rounds-mechanics"
description: "how to chain fabric-review rounds and post a subagent's report whole without retyping it"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7084390d543f47de
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

*References: no-scope-widening-in-review-fixes*

*Observed 2026-10-27 (ux-designer)*
