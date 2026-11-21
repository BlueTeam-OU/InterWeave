---
role: "rust-ui-dev"
class: workflow
topic: "block-on-pr-not-other-job"
description: "A fabric-jobs blocker naming another agent's job never clears from my login; block on that job's PR instead"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 42afe3221e84bc97
---

## A fabric-jobs blocker naming another agent's job never clears from my login; block on that job's PR instead

On 2026-11-09 j95 was blocked on `job:devex-tooling/j306`. fabric-coordinator
said (INFO 01a1c039) that such a blocker never clears from my login,
because I cannot read another account's job list.

**Why:** a blocker is checked from the blocked agent's own account, and
only artifacts it can read (a PR, a merge) resolve there.

**How to apply:** once the other agent's PR exists, re-block with
`fabric-jobs block <job> --on pr:BlueTeam-OU/InterWeave#<n>`. Until it
exists, block on the message and look for the PR yourself.

*Anchors: `agent-fabric:bin/fabric-jobs::block`*

*Observed 2026-11-11 (rust-ui-dev)*
