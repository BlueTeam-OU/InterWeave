---
role: "architect-cto"
class: workflow
topic: "remote-branch-prune-2026-10-28"
description: "2026-10-28 the owner had every InterWeave origin branch wholly on main deleted (256 of 266); GitHub delete-on-merge covers PR heads only, so for/ supply branches grow back; proposal to fabric-coordinator that fabric-branches --sweep…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5fbd9e3334861787
---

## 2026-10-28 the owner had every InterWeave origin branch wholly on main deleted (256 of 266); GitHub delete-on-merge covers PR heads only, so for/ supply branches grow back; proposal to fabric-coordinator that fabric-branches --sweep prunes one's own remote branches on origin/main

Owner's words: "the merged should remain there?" then "clean them if no more useful, keep else". Facts: delete_branch_on_merge is on, so what remained were 234 for/ supply branches (folded, never a PR head) and 24 same-lane folds; 10 unmerged kept (my docs/j421, three pending supplies, four closed-PR leftovers #99 #115 #401 #410, two old-named, each with commits main lacks). Deleted with a script checking merge-base --is-ancestor against a fresh origin/main and skipping open PR heads (scratchpad rev/prune_merged.py); 0 failures. Local: fabric-branches --sweep took 11 branches and 12 worktrees.

**Why:** nothing records a step deleting a supplier's remote for/ branch once the caller's PR is on main.
**How to apply:** a merged branch's commits and name survive in the fold's merge commit; deleting is safe when it heads no open PR. Proposal REQUEST 01a18168-0ae7 to develop-qzapp/user (fabric-coordinator's login; `fabric-coordinator` is not a placed account) for `fabric-branches --sweep` to prune one's own remote branches on origin/main. Until it lands, prune your own after each landing. Related: [[external-review-2026-10-27-main-1b37e0ef]].

**Accepted 2026-10-28 07:28Z (fabric-coordinator REPLY 01a18168-e2c1):** the sweep takes one's own merged remote branches; python-dev-03 builds it in tools/fabric/branches.py; the branch-hygiene and pull-requests skills follow after it merges.

**12:14Z (fabric-coordinator REPLY 01a1826e-dc90):** the sweep prunes one's own merged remote branches since agent-fabric #416 (python-dev-03, merged 10:37Z); reaches accounts on the next fabric upgrade + relaunch; j448 closed.

*References: external-review-2026-10-27-main-1b37e0ef*

*Observed 2026-10-28 (architect-cto)*
