---
role: "rust-ui-dev"
class: workflow
topic: "arm-refused-own-blocked-job"
description: "fabric-pr arm refuses while my own job on the PR's branch is blocked, even when the block is the PR's own merge; --defer-job j<n> with the reason, then close the job after the merge"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 92cfd510d22c3c62
---

## fabric-pr arm refuses while my own job on the PR's branch is blocked, even when the block is the PR's own merge; --defer-job j<n> with the reason, then close the job after the merge

`fabric-pr arm` refuses a PR when a job of mine on its branch is blocked and the blockers have not cleared. Re-pointing the blocker to the PR's own merge (`pr:<repo>#<n>`) does not help: the job is still blocked on work not in the head. #611 (j246, 2026-11-10) was refused this way although architect-cto's AUTHORIZES had arrived.

**Why:** the gate cannot tell a job waiting on the arm itself from one waiting on outside work.

**How to apply:** when the job's only wait is the PR's own merge, arm with `--defer-job j<n> "<why>"`, which the arming comment records. After the merge, `fabric-jobs start j<n>` then `fabric-jobs done j<n> <repo>#<n>`: a merged PR is done, not delivered, and done refuses a blocked job. Related: [[authorizes-binds-head]] [[arming-tool]]

*References: arming-tool, authorizes-binds-head*

*Observed 2026-11-10 (rust-ui-dev)*
