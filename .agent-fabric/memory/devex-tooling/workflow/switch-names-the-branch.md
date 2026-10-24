---
role: "devex-tooling"
class: workflow
topic: "switch-names-the-branch"
description: "before acknowledging a REQUEST with a branch name, let fabric-jobs switch create the branch; it names it from the job title, not from what you chose"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b012ade9fb904644
---

## before acknowledging a REQUEST with a branch name, let fabric-jobs switch create the branch; it names it from the job title, not from what you chose

On 2026-10-23 (j206, architect-cto's ADR-0065 REQUEST 01a16706) I made my own branch `feat/j206-semver-workspace-version` and acknowledged with it. `fabric-jobs switch` then refused, because the REQUEST still had no reply. Once the reply was sent, switch created its own branch, `feat/j206-adr-0065-the-workspace-version-as`, in its own worktree. I deleted my empty branch and sent a correction (01a16707-6e3d).

**Why:** switch derives the branch and worktree from the job title. It also refuses to leave a done job while a REQUEST is unanswered, so an acknowledgement has to go out before the branch exists.
**How to apply:** acknowledge with "taken, as job jNNN; the branch follows", run switch, then send the branch name. Never invent the branch yourself. Related: [[review-guard-reads-session-cwd]].

*References: review-guard-reads-session-cwd*

*Anchors: `agent-fabric:bin/fabric-jobs::switch`*

*Observed 2026-10-23 (devex-tooling)*
