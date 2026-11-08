---
role: "rust-services-dev"
class: workflow
topic: "cross-repo-job-needs-fresh-session"
description: "A job in another repository must run in a session started in that clone — fabric-review brief refuses a repository outside the session's clone"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 361f13c1a1d9b3fc
---

## A job in another repository must run in a session started in that clone — fabric-review brief refuses a repository outside the session's clone

`fabric-jobs switch` printed "continue here" for gateway job j53 while
this session ran in the InterWeave clone (2026-10-31). The lockfile fix
committed and pushed fine from the job's worktree, but `fabric-review
brief` refused: the repository "is not this session's clone … or one of
its worktrees, the only repositories its review tools read". The review
gate therefore cannot be met across clones.

**Why:** the review MCP tools are scoped to the dispatching session's
clone (tools/fabric/review_brief.py session_repository_refusal).

**How to apply:** when a job's working copy is in another project, do
the work there from the start: commit, then `fabric-fresh --job <id>`
before any review, rather than trusting "continue here".

*Observed 2026-10-31 (rust-services-dev)*
