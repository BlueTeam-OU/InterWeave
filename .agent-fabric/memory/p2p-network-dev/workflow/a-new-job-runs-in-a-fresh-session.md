---
role: "p2p-network-dev"
class: workflow
topic: "a-new-job-runs-in-a-fresh-session"
description: Once a session has worked a job, the commit hook refuses a commit for the next job; switch before coding it
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8a616948b46d0d27
---

## Once a session has worked a job, the commit hook refuses a commit for the next job; switch before coding it

2026-11-03: in j141's session I added j142 from an inbox OBSERVATION, started
it and coded the fix; `git commit` was refused by the PreToolUse hook
(agent-fabric ADR-037 rule 3: a session that worked one job does not commit
for the next). The work had to be saved as a patch and carried over.

**Why:** a session belongs to one job; the next job starts fresh.
**How to apply:** when a delivery becomes a new job mid-session, add it,
reply with the decision, and run `fabric-jobs switch` — do the code in the
new session. Reading and diagnosing are allowed before the switch.

*Observed 2026-11-03 (p2p-network-dev)*
