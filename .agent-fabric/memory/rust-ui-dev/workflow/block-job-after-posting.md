---
role: "rust-ui-dev"
class: workflow
topic: "block-job-after-posting"
description: "Blocking the active job stops fabric-pr post-review, commits and pushes until the blocker clears; post and push before fabric-jobs block"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0ac85eb33bb427b0
---

## Blocking the active job stops fabric-pr post-review, commits and pushes until the blocker clears; post and push before fabric-jobs block

`fabric-jobs block` takes the job out of active, and the commit/publish hook then refuses `fabric-pr post-review` (and commits) with "no job is active". `fabric-jobs start` refuses while a `request:` blocker has no reply. Seen on j110 2026-11-15: round-5 review of #677 could not be posted until architect-cto answered.

**Why:** ADR-037 rule 3: a session that worked a job publishes only under an active job.
**How to apply:** post every review, push every commit, then block. If a review arrives after blocking, it waits for the reply; post it before arming.

*Anchors: `agent-fabric:bin/fabric-jobs::block`*

*Observed 2026-11-15 (rust-ui-dev)*
