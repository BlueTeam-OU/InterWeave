---
role: "devex-tooling"
class: workflow
topic: "switch-cuts-for-branch-off-main"
description: "fabric-jobs switch cuts a requested for/ branch off origin/main even when the REQUEST's DELIVER-TO names another base; reset it before the first commit"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - dde518f4ebc41376
---

## fabric-jobs switch cuts a requested for/ branch off origin/main even when the REQUEST's DELIVER-TO names another base; reset it before the first commit

`fabric-jobs switch` creates the for/ branch a REQUEST names, but always off
origin/main ("new off origin/main"), even when DELIVER-TO says "cut from
origin/<caller's branch>". Seen 2026-10-31 at j260 (agent-fabric j468, base
the j444 branch of PR #437). Before the first commit, check
`git log origin/main..HEAD` is empty and the branch is not on origin, then
`git reset --hard <caller's head sha>` in the job's worktree. A supply cut
from main instead would bring commits the caller's PR does not have when
it is folded unrebased. See [[switch-names-the-branch]].

*References: switch-names-the-branch*

*Anchors: `agent-fabric:tools/fabric/jobsparts/branching.py::new off origin/`*

*Observed 2026-10-31 (devex-tooling)*
