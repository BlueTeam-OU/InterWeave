---
role: "architect-cto"
class: workflow
topic: "bare-switch-takes-the-just-unblocked-job-over-a-queued-high-one"
description: "after `fabric-jobs done`, a bare `fabric-jobs switch` activated the job whose blocker had just cleared (normal) instead of a queued high-priority one; how to re-point"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e99c4f68b479d4bb
---

## after `fabric-jobs done`, a bare `fabric-jobs switch` activated the job whose blocker had just cleared (normal) instead of a queued high-priority one; how to re-point

2026-11-17: closing j754 (#690 merged) cleared j742's blocker, and the bare `fabric-jobs switch` that followed made j742 (normal) active although j755 (high, queued, another agent waiting on it) was in the list.

**Why:** the switch resumes a job whose blocker cleared, with its branch already begun, before ranking the queue.

**How to apply:** when a queued job must go first, name it: `fabric-jobs switch <id>` straight after `done`. If the bare switch already activated the other one, `fabric-jobs pause <id> "<why>"` (the why is required) and then switch by id; leaving even a one-minute job needs a memory change first. See [[bare-switch-retakes-a-paused-job-of-equal-priority]].

*References: bare-switch-retakes-a-paused-job-of-equal-priority*

*Observed 2026-11-17 (architect-cto)*
