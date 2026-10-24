---
role: "rust-ui-dev"
class: workflow
topic: "fabric-review-brief-usage"
description: "How to render a review brief with fabric-review and post the report with fabric-pr post-review without the refusals hit on first use"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5cd51f79e6d023f5
---

## How to render a review brief with fabric-review and post the report with fabric-pr post-review without the refusals hit on first use

Observed on 2026-10-19 (PR #315).

**Rendering the brief:** `fabric-review brief <request.yaml>`. It is neither `brief render` nor `render`.
- The keys are in `agent-fabric/tools/fabric/review_brief.py`: SCALAR_KEYS, COUNT_KEYS and LIST_KEYS.
- `lenses` names files under `runtime/claude-code/review/lenses/`.
- A re-review needs `card_base: <the first round's base sha>`. Its range base (the previous head) is not on origin/main, and the brief refuses to render without it.
- The free text is linted for verdict words ("fixes", "ensures", "correctly", "because" at sentence start, and others), so state requirements, not conclusions.

**Posting the report:** `fabric-pr post-review <n>` refuses a hand-written summary that lacks the dispatch block. Pass `--head <sha the review read>`, or post the reviewer's report whole with its `agent-fabric-dispatch v1` block first.

**Cleanup:** the reviewer builds into its own scratch directory under the session scratchpad (842M once). Remove it after the review.

*Anchors: `agent-fabric/tools/fabric/review_brief.py::card_base`*

*Observed 2026-10-19 (rust-ui-dev)*
