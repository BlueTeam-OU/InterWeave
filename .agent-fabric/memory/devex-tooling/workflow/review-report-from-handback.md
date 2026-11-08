---
role: "devex-tooling"
class: workflow
topic: "review-report-from-handback"
description: "extracting a code-review subagent's report for fabric-pr post-review; it is not in the last assistant text"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 619168da2677e042
  - 66fbfd84a1de7a07
---

## extracting a code-review subagent's report for fabric-pr post-review; it is not in the last assistant text

A code-review agent's report reaches the session as a hand-back tool call, not as its final assistant text block, so grabbing the last assistant text from the task transcript yields nothing (PR #446, 2026-10-25). Walk every string in the JSONL and keep the longest one that holds `agent-fabric-coverage` and the `Finding:`/`Judgement:` lines; write it to the scratchpad and pipe that to `fabric-pr post-review <n>` and `fabric-review save <n> <round>`.
For a RE-review the longest matching string is the BRIEF (the prompt quotes the round-1 review, marker line included), and post-review refuses it ("line 44 looks like a review marker"): match on `# Re-review of PR #<n>` and exclude strings holding `## Previous findings` (PR #525, 2026-10-31).
Also: `fabric-review next` refuses when the round-1 range base is `origin/main` — add `card_base: <round 1 base_sha>` to the request first ([[review-brief-card-base]]).

**Why:** the first extraction attempt posted an empty body and was refused.
**How to apply:** every time a review round's report has to be posted.

*References: review-brief-card-base*

*Anchors: `agent-fabric:tools/fabric/review_rounds.py::card_base`*

*Observed 2026-10-25 (devex-tooling)*
