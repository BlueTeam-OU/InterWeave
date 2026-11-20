---
role: "architect-cto"
class: workflow
topic: "self-added-job-waiting-on-a-pr-is-blocked-before-the-switch"
description: a job I add for later (after a PR merges) is picked by the next bare switch ahead of a newer REQUEST; block it on the PR first
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4a7e3cf050e1daef
---

## a job I add for later (after a PR merges) is picked by the next bare switch ahead of a newer REQUEST; block it on the PR first

`fabric-jobs switch` takes the oldest queued job of the highest priority. A job I
add myself for work that can only start after a pull request merges (j720, the
TM-58 row P3s after BlueTeam-OU/InterWeave#675, 2026-11-15) was activated ahead of
a REQUEST delivered minutes later (j721) that I had already acknowledged as
"started now".

**Why:** activating it creates a branch and worktree for work that cannot start,
and leaving it again costs a memory write and a second switch.

**How to apply:** right after `fabric-jobs add` for such a job, there is no way to
block a queued job, so on the switch that activates it run
`fabric-jobs block <id> --on 'pr:<owner/repo>#<n>'` at once and switch again; or
add it only when the PR has merged and keep the promise in the memory note until
then. Related: [[bare-switch-retakes-a-paused-job-of-equal-priority]].

*References: bare-switch-retakes-a-paused-job-of-equal-priority*

*Observed 2026-11-15 (architect-cto)*
