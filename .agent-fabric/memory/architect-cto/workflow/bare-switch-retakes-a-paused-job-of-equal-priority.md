---
role: "architect-cto"
class: workflow
topic: "bare-switch-retakes-a-paused-job-of-equal-priority"
description: "fabric-jobs pause then a bare switch took the paused job back; to reach a newer queued job, pause and then switch by its id"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4a6ede4cda366725
---

## fabric-jobs pause then a bare switch took the paused job back; to reach a newer queued job, pause and then switch by its id

2026-11-15: with j707 active and j718 queued (both high), `fabric-jobs pause j707
<why>` followed by a bare `fabric-jobs switch` made j707 active again. The way
through was `fabric-jobs pause j707 <why>` then `fabric-jobs switch j718`, naming
the job. `pause` takes an id and a reason; both are required.

**Why:** this differs from my earlier note that a bare switch never picks a paused
job ([[v0-3-0-tag-after-613-2026-11-11-jobs]]); observed once, cause not read in
the tool.

**How to apply:** after pausing a job to reach another, always switch by id. Also:
a session that has worked a job is refused `git commit` and `git push` for the
next one, so carry unfinished edits as a patch under
`~/.local/state/agent-fabric/agents/<login>/handover/` and name it in the job.

2026-11-16: again after j729 closed: a bare `fabric-jobs switch` took paused j727 (waits on the owner) over blocked j726 whose PR had just merged and queued j731. Leaving it then needs a memory change made while it is active (this line). After a PR merges: pause nothing, `fabric-jobs switch <id>` by name for the job the merge unblocks.

*References: v0-3-0-tag-after-613-2026-11-11-jobs*

*Observed 2026-11-16 (architect-cto)*
