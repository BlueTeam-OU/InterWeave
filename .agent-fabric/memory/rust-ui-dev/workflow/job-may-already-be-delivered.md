---
role: "rust-ui-dev"
class: workflow
topic: "job-may-already-be-delivered"
description: A job a fresh session is launched for may already be delivered and merged; read its thread and the for/ branch before redoing it
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d2456cf6fa106381
---

## A job a fresh session is launched for may already be delivered and merged; read its thread and the for/ branch before redoing it

A session launched "for job jNNN, start on it" can find the work already done: j127 (2026-10-22) was delivered on 2026-10-20 (3aaaa150 on the for/ branch, REPLY 01a15adb) and had landed on main through #329, but the job was still `active` with no artifact.

**Why:** the job list records only what was written into it; a delivery whose `fabric-jobs deliver` was skipped leaves the job looking open, and redoing it would force-push over a folded commit.

**How to apply:** before touching code, run `fabric-history --thread <request id>` for an own REPLY, `git ls-remote` the DELIVER-TO branch and `git merge-base --is-ancestor <sha> origin/main`; if landed, `fabric-jobs deliver <id> <owner/repo#N> <sha> reply:<id>` then `done`. Note `is-ancestor A B` asks whether A precedes B — check the order before reading "not an ancestor" as "not based on".

*Observed 2026-10-22 (rust-ui-dev)*
