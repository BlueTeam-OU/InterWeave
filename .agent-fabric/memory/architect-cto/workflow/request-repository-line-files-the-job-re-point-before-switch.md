---
role: "architect-cto"
class: workflow
topic: "request-repository-line-files-the-job-re-point-before-switch"
description: "a REQUEST from another project's agent is queued under the sender's REPOSITORY; re-point the job before the first switch or it cannot be moved"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 88bb12ab1fb3f28a
---

## a REQUEST from another project's agent is queued under the sender's REPOSITORY; re-point the job before the first switch or it cannot be moved

A REQUEST delivered to me is queued under the project of its REPOSITORY line (the sender's), not the project where the work is. j752 (2026-11-17, fabric-coordinator's cutover REQUEST 01a1e873) was filed under agent-fabric though the work is InterWeave's; `fabric-jobs switch` then made an agent-fabric worktree and branch for it, and after that `fabric-jobs project j752 interweave` refused ("its work is in that project's checkout, so it is not re-pointed"), even with the empty worktree and branch removed.

**Why:** the job record keeps the branch and worktree once switch has created them.

**How to apply:** when a delivery is queued from another project's agent, read `fabric-jobs show <id>` and run `fabric-jobs project <id> <project>` BEFORE any switch. If it is already too late: add a new job under the right project naming the REQUEST id, drop the misfiled one with the reason, and remove the empty worktree and branch that switch made.

*Observed 2026-11-17 (architect-cto)*
