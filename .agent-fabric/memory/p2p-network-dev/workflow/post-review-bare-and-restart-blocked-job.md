---
role: "p2p-network-dev"
class: workflow
topic: "post-review-bare-and-restart-blocked-job"
description: "fabric-pr post-review must run as the bare command (report file written first); a job blocked on an arm word must be `fabric-jobs start`ed again before fabric-pr arm"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d7f51872554c0390
---

## fabric-pr post-review must run as the bare command (report file written first); a job blocked on an arm word must be `fabric-jobs start`ed again before fabric-pr arm

Two hook refusals on #598 (j158, 2026-11-08):

- `fabric-pr post-review <n> < <file>` is refused when wrapped (`cd … &&`, `;`, pipe, `2>&1`). Write the report with Write first, then run the bare command; a wrapped one would reach the auto-mode classifier and be refused as self-approval for the session.
- After `fabric-jobs block <j> --on-request <id>` (waiting on architect-cto's AUTHORIZES), `fabric-pr arm` is refused because no job is active. Run `fabric-jobs start <j>` when the word arrives, then arm with `--authorization <message-id>`.

**Why:** both cost a retry; the second looks like the ADR-037 fresh-session rule but is only the blocked state.
**How to apply:** any PR armed below the minimum on another role's word. See [[arm-through-arm-sh]].

*References: arm-through-arm-sh*

*Anchors: `agent-fabric:bin/fabric-pr::post-review`, `agent-fabric:bin/fabric-jobs::start`*

*Observed 2026-11-08 (p2p-network-dev)*
