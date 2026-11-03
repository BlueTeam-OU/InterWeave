---
role: "devex-tooling"
class: workflow
topic: "reorder-jobs-pause-needs-push"
description: "putting an owner's new job ahead of one switch already made active? pause it, push its (even empty) branch, then switch"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5a87f874814d96a7
---

## putting an owner's new job ahead of one switch already made active? pause it, push its (even empty) branch, then switch

`fabric-jobs switch` marks the next job active and creates its branch
and worktree right away. To put a newer, higher-priority job ahead of
it (j214, the owner's toolchain bump, ahead of j212 on 2026-10-24), run
`fabric-jobs pause <id> "<why>"`. Switch then refuses to leave the paused
job until two things are true: its branch is on origin (even with zero
commits) and the memory directory has changed during it.

**Why:** pausing j212 alone was refused for both reasons.
**How to apply:** pause, `git push -u origin HEAD` in its worktree, then
switch. Related: [[switch-names-the-branch]].

*References: switch-names-the-branch*

*Anchors: `agent-fabric:bin/fabric-jobs::switch`*

*Observed 2026-10-24 (devex-tooling)*
