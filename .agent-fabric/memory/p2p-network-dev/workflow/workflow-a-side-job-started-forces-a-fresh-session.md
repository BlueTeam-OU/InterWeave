---
role: "p2p-network-dev"
class: workflow
topic: "workflow-a-side-job-started-forces-a-fresh-session"
description: "Starting/delivering a second job mid-PR makes the commit hook refuse the first job's commits in this session (ADR-037 rule 3)"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 813265db40d6c700
---

## Starting/delivering a second job mid-PR makes the commit hook refuse the first job's commits in this session (ADR-037 rule 3)

On 2026-11-10 I `fabric-jobs start`ed and `deliver`ed j105 (a disk request already answered) only to tidy the
list while j103 (#608) waited on a supply. When the supply came, the hook refused `git merge` for j103: "This
session worked job j105 ... the next one runs in a fresh session". The fold had to move to `fabric-fresh --job j103`.

**Why:** the job hook counts any job made active in a session as worked; the next job's commits need a fresh session.
**How to apply:** mid-PR, never start another job just to close it; leave a queued job whose reply is already sent
for the next `switch` (or deliver it from the session that ends anyway).

*Anchors: `agent-fabric:runtime/claude-code/hooks/job-copy-guard.py::belongs to a fresh session`*

*Observed 2026-11-10 (p2p-network-dev)*
