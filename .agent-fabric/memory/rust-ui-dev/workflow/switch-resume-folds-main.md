---
role: "rust-ui-dev"
class: workflow
topic: "switch-resume-folds-main"
description: "fabric-jobs switch resuming a paused job merged origin/main into its branch locally; pushing it moves a reviewed PR's head"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 86da233c0884e1f4
---

## fabric-jobs switch resuming a paused job merged origin/main into its branch locally; pushing it moves a reviewed PR's head

2026-11-07 (j224, PR #590): I paused j224 with the PR reviewed clean at dd1508c79. A bare `fabric-jobs switch` resumed it, and at the same moment (reflog 00:22:11 +0100) a merge of origin/main appeared on the branch (a7c7ee238, "Merge remote-tracking branch 'origin/main'"). I did not run that merge. switch then refused to leave the job until the branch was pushed, and pushing would have moved the head past the posted review.
**Why:** a pushed main fold needs a re-review of the new range before arming, and InterWeave's queue is non-strict, so a fold the PR does not need only costs a review round.
**How to apply:** to move on from a job that waits on someone, name the next job: `fabric-jobs switch j<n>` ([[switch-to-unblocked-job-by-id]]). Never a bare switch. Read `git log origin/<branch>..HEAD` before pushing a PR branch that is waiting to be armed.

*References: switch-to-unblocked-job-by-id*

*Anchors: `agent-fabric:bin/fabric-jobs`*

*Observed 2026-11-07 (rust-ui-dev)*
