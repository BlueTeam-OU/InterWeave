---
role: "devex-tooling"
class: workflow
topic: "review-guard-reads-session-cwd"
description: "a code-review dispatch is denied as \"from the supplier branch\" when the session's starting clone sits on an old for/ branch, even though the PR lives in a job worktree"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c40a9db4e4521c92
---

## a code-review dispatch is denied as "from the supplier branch" when the session's starting clone sits on an old for/ branch, even though the PR lives in a job worktree

agent-fabric's dispatch guard (`tools/fabric/guards/dispatch_guard.py`, `supply_branch(call.get("cwd"))`) takes the branch from the SESSION's cwd, which is the starting clone (`projects/interweave`). It does not look at the job worktree where the PR's branch is checked out. On 2026-10-22 (j205, PR #383) the starting clone was still on a pushed, merged `for/p2p-network-dev-01/matrix-cfg` branch, and the review dispatch was refused as a supplier's review.

**Why:** `cd` inside Bash does not change the cwd the hook sees.
**How to apply:** at the start of a job session, check the starting clone's branch. If it is a `for/` branch whose head is on origin and on origin/main, `git checkout main && git merge --ff-only origin/main` there before you dispatch a review.

*Anchors: `agent-fabric:tools/fabric/guards/dispatch_guard.py::supply_branch(call.get("cwd"))`*

*Observed 2026-10-22 (devex-tooling)*
