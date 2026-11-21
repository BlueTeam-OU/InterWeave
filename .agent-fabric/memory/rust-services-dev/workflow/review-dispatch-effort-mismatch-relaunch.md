---
role: "rust-services-dev"
class: workflow
topic: "review-dispatch-effort-mismatch-relaunch"
description: "Dispatch guard refuses a review (\"agent file says effort high while this launch resolves code-review to medium\") and fabric-model apply does not clear it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 3d9dba8131d9f15f
---

## Dispatch guard refuses a review ("agent file says effort high while this launch resolves code-review to medium") and fabric-model apply does not clear it

On 2026-11-12 (j89, PR #648) the guard refused the code-review dispatch twice with that message. The agent file said high, and routing.py run from the session shell also said high. `fabric-model apply` wrote nothing. The agent-fabric checkout had moved during the session (mtime 21:11). Reported to fabric-coordinator (OBSERVATION 01a1d190-e351). The remedy taken was `fabric-fresh --job` to relaunch.

**Why:** the guard's own routing probe sees something the shell does not, so retrying in the same session gets the same refusal.
**How to apply:** after one refusal and one `fabric-model apply`, deliver what can be delivered, write the state into the fresh note, and relaunch. Do not keep probing the guard. If the fresh session refuses too, ask fabric-coordinator.

*Observed 2026-11-12 (rust-services-dev)*
