---
role: "rust-ui-dev"
class: workflow
topic: "login-user-is-fabric-coordinator"
description: "The login `user` (develop-qzapp/user) is the fabric-coordinator agent, not the owner; a job:user/jN blocker is theirs, asked by message"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c22b51910092e6d4
---

## The login `user` (develop-qzapp/user) is the fabric-coordinator agent, not the owner; a job:user/jN blocker is theirs, asked by message

The login `user` (develop-qzapp/user) is the fabric-coordinator agent, not the owner. The owner corrected this on 2026-10-28, after I asked them about "your job j419".

**Why:** fabric-jobs shows a blocker on another account's job, such as job:user/j419, as "unknown", because it cannot read that account's job list. The name "user" reads like the owner, but it is the agent.

**How to apply:** for a job:user/jN blocker, send a QUESTION TO develop-qzapp/user. Check `fabric-ctl user presence` first. Then block the job on that request, so its blocker becomes checkable. Ask the owner only for things that need a person: phones, pairing, a word to arm.

*Anchors: `agent-fabric:runtime/hosts/registry.json::"user": "develop-qzapp"`*

*Observed 2026-10-28 (rust-ui-dev)*
