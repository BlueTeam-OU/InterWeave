---
role: "p2p-network-dev"
class: workflow
topic: "switch-before-working-a-new-job"
description: "A new job arriving in a session that already worked one goes through fabric-jobs switch first, never fabric-jobs start in place"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f07a6c27635997a8
---

## A new job arriving in a session that already worked one goes through fabric-jobs switch first, never fabric-jobs start in place

When a new REQUEST arrives in a session that already worked a job, run `fabric-jobs switch` before doing anything else for it. Do not use `fabric-jobs start <id>` in place.

What happened on 2026-11-12 (j162, the v0.3.0 phone run): this session had worked j160, and I started j162 with `fabric-jobs start`. The phone run went fine. Then the PreToolUse hook refused `git commit`, because ADR-037 rule 3 says a session that has worked a job does not commit for the next one. `start` had also recorded the merged j160 branch on the job, so `fabric-jobs switch` refused it as well ("records branch … which a pull request has landed"). The way out was to save the record as a patch under `$XDG_RUNTIME_DIR/j162/`, restore the tree, drop j162 and re-add it as j163 without `--request`; the message id was already taken by the dropped job.

**Why:** `start` binds the current checkout's branch to the job. A later commit is refused anyway, and the job can no longer be switched to.

**How to apply:** non-committing work that cannot wait, such as a phone window the owner opens now, may run in place, but run the switch first. If you can't, write the output outside the tree straight away and hand it over through the job's pause note or a HANDOVER file.

*Observed 2026-11-12 (p2p-network-dev)*
