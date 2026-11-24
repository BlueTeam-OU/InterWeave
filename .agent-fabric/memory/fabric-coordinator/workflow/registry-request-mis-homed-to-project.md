---
role: "fabric-coordinator"
class: workflow
topic: "registry-request-mis-homed-to-project"
description: "a REQUEST about projects/registry.json can arrive as a job in the PROJECT's worktree; the job guard then refuses the agent-fabric commit"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 56d57ebc38b5e8cb
---

## a REQUEST about projects/registry.json can arrive as a job in the PROJECT's worktree; the job guard then refuses the agent-fabric commit

j605 (REQUEST 01a1a761, a registry.json row for interweave) was provisioned with project interweave and an InterWeave worktree. The edit belongs in agent-fabric, on the open fabric PR (#520's copy), and the PreToolUse job guard refused `git commit` there (ADR-037 rule 3). `fabric-jobs project` refuses to re-point a job that already has a branch and worktree.

**Why:** intake derives the project from the request's subject, not from the file that changes.

**How to apply:** at the start of a request job, check which repository the change lands in before editing. If it is mis-homed: `fabric-jobs add --project agent-fabric --working-copy <the open fabric PR's copy> "<exact change>"`, then `fabric-jobs drop` the old one, then switch (done 2026-11-09: j605 -> j684). Also: verify which registry row the requester's failure comes from — the row they cited (l.372) was agent-fabric-gateway's, not InterWeave's (l.335).

*Anchors: `agent-fabric:projects/registry.json::cargo-machete`*

*Observed 2026-11-09 (fabric-coordinator)*
