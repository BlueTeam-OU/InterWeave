---
role: "rust-services-dev"
class: workflow
topic: "paused-job-start-before-arm"
description: "When the arm word arrives for a job paused on it, `fabric-jobs start j<n>` first (there is no `resume`); the switch hook refuses fabric-pr arm while no job is active"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 24ced6a775d3fbd6
---

## When the arm word arrives for a job paused on it, `fabric-jobs start j<n>` first (there is no `resume`); the switch hook refuses fabric-pr arm while no job is active

A job paused while waiting on an arming word leaves the session with no active job, and the PreToolUse hook (agent-fabric ADR-037 rule 3) then refuses any command that publishes, `fabric-pr arm` included. The hook refuses the whole compound command, so a `fabric-jobs` call chained in front of the arm never runs either. Run `fabric-jobs start j<n>` on its own first (the CLI has no `resume` verb), then arm. Seen on InterWeave #631, 2026-11-11.

**Why:** pausing rather than blocking is still right ([[arm-word-request-blocks-the-job]]); it only costs this one restart step.
**How to apply:** delivery of the AUTHORIZES reply → `fabric-jobs start` → `fabric-pr arm --authorization <id>` → `wait-merged`.

*References: arm-word-request-blocks-the-job*

*Anchors: `agent-fabric:bin/fabric-jobs::pause`*

*Observed 2026-11-11 (rust-services-dev)*
