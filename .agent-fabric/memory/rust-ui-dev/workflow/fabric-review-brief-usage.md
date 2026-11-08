---
role: "rust-ui-dev"
class: workflow
topic: "fabric-review-brief-usage"
description: "How to render a review brief with fabric-review and post the report with fabric-pr post-review without the refusals hit on first use"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5cd51f79e6d023f5
  - 9e9d4ee227734a1c
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

**A fold inside a re-review range (2026-10-25, PR #433):** `out_of_scope` for main's folded paths makes the
reviewer mark them "not read", so the posted review is only *partial coverage* and does not count. Worse, a later
`mode: re-review` then refuses to render ("no review a re-review can start from"). Either have the re-review read
every listed path (no out_of_scope), or fold main BEFORE the fix commits. Recovery: a full `mode: review` over
`$(git merge-base HEAD origin/main)..HEAD`, whose diff holds only the PR's own paths.

**Re-review rounds (2026-10-31, #519/#520):** `fabric-review next <REQUEST> --pr N --head SHA` must be given the
PREVIOUS round's request: round 1's yaml for round 2, then the `review-*-rr2.json` it writes for round 3 (giving
round 1's again is refused: "a re-review of #N starts at its last review's head"). Point the reviewer at the
rendered brief file rather than pasting it; never pass a `$(...)` placeholder in an Agent prompt (it is sent
literally).

*Anchors: `agent-fabric/tools/fabric/review_brief.py::card_base`*

*Observed 2026-10-19 (rust-ui-dev)*
