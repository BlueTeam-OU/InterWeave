---
role: "rust-services-dev"
class: workflow
topic: "post-merge-pushed-branch-survives-sweep"
description: "a branch pushed to after its PR merged is not an ancestor of main, so the branch sweep keeps it; git push --delete is refused in auto mode"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - b96e4d5d347bb6e4
---

## a branch pushed to after its PR merged is not an ancestor of main, so the branch sweep keeps it; git push --delete is refused in auto mode

Open, 2026-11-13: on agent-fabric-gateway origin two of my branches
(`…/docs/production`, PR #36; `…/feat/live-check-6`, PR #33, both merged
2026-10-10) each carry two commits pushed after the merge. `git cherry -v
origin/main <branch>` marks all four `-` (patch-equivalent on main), but
the tips are not ancestors of main, so the 2026-11-13 branch sweep left
them. `git push origin --delete <branch>` was refused by the auto-mode
classifier ("[Git Destructive]").

**Why:** ancestry, not patch equivalence, is what the sweep reads; the
refusal is a finding, never a command for the owner.

**How to apply:** check `git cherry` to show nothing is unlanded, then
either leave the branch or propose the deletion path to fabric-coordinator
(a sweep that reads patch equivalence, or an allowlist entry); do not
retry the delete by another route.

*Anchors: `agent-fabric:tools/fabric/branches.py::sweep`*

*Observed 2026-11-13 (rust-services-dev)*
