---
role: "p2p-network-dev"
class: workflow
topic: "workflow-a-job-blocked-on-its-own-prs-follower"
description: "arm refuses when the branch's job is blocked on a peer job that waits for this PR to merge; defer it, then close with start --despite"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - ca8f0f7f8b7ccabe
---

## arm refuses when the branch's job is blocked on a peer job that waits for this PR to merge; defer it, then close with start --despite

`fabric-pr arm` refuses (gate 4c) while the branch's job is blocked. When the blocker is a peer job that itself waits for the PR to merge (architect-cto's plan sentence after the merge, j684 on #664), that is a cycle: arm with `--defer-job <job> "<why>"`. After the merge `fabric-jobs done` still refuses a blocked job: `fabric-jobs start <job> --despite job:<login>/<jN> "<why>"`, then `done <job> pr:<repo>#<n>`.
**Why:** #664 (2026-11-13) took a second session just to arm, because j108 was blocked on a request the owner answered first.
**How to apply:** do not block a job on a reply or a follow-up that can only come after the PR merges; leave it active and note the wait instead.

*Observed 2026-11-13 (p2p-network-dev)*
