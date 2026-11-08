---
role: "fabric-coordinator"
class: workflow
topic: "arm-defer-carried-findings"
description: "fabric-pr arm refuses a PR whose review chain holds findings moved to other jobs until each is passed as --defer \"<chain head8>:F<n>\" \"<why>\""
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 616639c2bde46832
---

## fabric-pr arm refuses a PR whose review chain holds findings moved to other jobs until each is passed as --defer "<chain head8>:F<n>" "<why>"

An adjudicated-and-moved finding (fixed elsewhere, e.g. in an agent-fabric job) still counts as OPEN in `fabric-pr arm`'s review-chain gate: a later re-review that marks it `not-fixed` does not close it. Arm refuses naming each with its chain head (the head the finding was raised on, not the PR's current head).

Pass each one: `--defer "a1a39ac3:F1" "deferred to job j564 (...)"`, repeatable; the dry-run prints `DEFERRED` per finding and then `no finding open ... N deferred`. Seen on InterWeave #529 (j504, 2026-11-03), F1/F2/F4 → j564/j565/j566.

Also: `fabric-pr post-review <n> < <file>` must be the bare command alone in its Bash call (no cd, no `;`, no 2>&1) or the hook refuses it — write the report with Write first. See [[review-report-arrives-as-message]].

*References: review-report-arrives-as-message*

*Anchors: `agent-fabric:tools/fabric/github/arm.py::--defer`*

*Observed 2026-11-03 (fabric-coordinator)*
