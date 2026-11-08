---
role: "devex-tooling"
class: workflow
topic: "one-session-one-job-publish-hook"
description: "starting a second job mid-session (pause A, start B) makes the PreToolUse hook refuse every commit/publish for both; switch to a fresh session instead"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e14299c26943a201
---

## starting a second job mid-session (pause A, start B) makes the PreToolUse hook refuse every commit/publish for both; switch to a fresh session instead

In a session launched for job A, `fabric-jobs pause A` + `fabric-jobs start B`
marks the session as having worked B; from then the PreToolUse Bash hook refuses
any commit/publish (`fabric-pr arm` included) whatever job is active (ADR-037
rule 3). Seen 2026-11-08: launched for j298, started j297 to arm InterWeave #591,
arm refused. The "arm needs an active job" trick ([[arm-needs-active-job]]) is
for the job the session was launched for, not a second one.

**Why:** one session works one job; the next job runs in `fabric-fresh --job`.
**How to apply:** when a blocked job unblocks mid-session, finish or pause the
current job and `fabric-jobs switch <it>` — never pause/start in place.
Also: before sending an AUTHORIZES request, check the relay for one already
answered — j297's head had been authorized at 07:00Z and I asked again.

*References: arm-needs-active-job*

*Anchors: `agent-fabric:docs/adr/ADR-037-each-agent-keeps-a-job-list.md::rule 3`*

*Observed 2026-11-08 (devex-tooling)*
