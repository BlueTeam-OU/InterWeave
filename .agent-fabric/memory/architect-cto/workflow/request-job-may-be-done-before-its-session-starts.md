---
role: "architect-cto"
class: workflow
topic: "request-job-may-be-done-before-its-session-starts"
description: "a request-born job can be wholly satisfied before its fresh session opens; read the PR state, the thread and origin/main first, then close with the landed evidence"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3c2c8d6830e405c5
---

## a request-born job can be wholly satisfied before its fresh session opens; read the PR state, the thread and origin/main first, then close with the landed evidence

A job queued from a REQUEST can be finished by the time its session
starts. j684 (2026-11-14, InterWeave): the arming word for #664 was
given by the owner in the requester's session, the PR had merged
(50107ae97), and the plan's S12 sentence I owed had already landed in
my own earlier PR (9428730c4 via #665); my REPLY 01a1d6d8 was sent
before the switch that queued the job's session.

**Why:** the job's title and the fenced request text are a snapshot of
the day they were written; the tree and the thread had moved twice.

**How to apply:** before any work on a request-born job, read
`gh pr view <n>` (state), `fabric-history --thread <id>` (a later
message may withdraw an item, and my own reply may exist) and grep
`origin/main` for the stale words. If all is landed: `fabric-jobs
deliver <id> "<project> commit:<sha>" "reply:<id>"`, then `sweep`,
then `switch`. See [[supply-job-closes-on-the-supply-sha-reaching-origin-main]].

*References: supply-job-closes-on-the-supply-sha-reaching-origin-main*

*Anchors: `agent-fabric:tools/fabric/jobs.py::sub.add_parser("deliver")`*

*Observed 2026-11-13 (architect-cto)*
