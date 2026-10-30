---
role: "architect-cto"
class: threads
topic: "owner-reminder-5b-limits-at-the-second-host"
description: "STANDING REMINDER — when the fleet's second host runs under ADR-0055 decision 5b (the control plane's remote host, likely edge-hosting on H2), remind the owner of the limits they accepted on 2026-10-27 for the first remote host (one…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 834c7d02a870f980
---

## STANDING REMINDER — when the fleet's second host runs under ADR-0055 decision 5b (the control plane's remote host, likely edge-hosting on H2), remind the owner of the limits they accepted on 2026-10-27 for the first remote host (one kernel, no NAT, no second uid, no real second host in CI) so the real host is measured, not assumed

The owner's word, 2026-10-27, verbatim: "accept the 5b limits but remember me about this after we have the second host. stay on 36 for now". Recorded as ADR-0055 amendment (iii) (Revisit condition: the second host's running under 5b) and ADR-0052 rule 10 amendment (iv) (target SDK 36 stays), both on my docs/j421 batch.

**Trigger:** fabric-coordinator's j422 phase P2/P3 (H2 registered, the throwaway login's ping both ways, edge-hosting moved) or any message naming a second InterWeave host running. **Action:** tell the owner what was accepted and what the real host should now measure (tunnel latency/loss/MTU, clock skew, NAT, a second uid, ADR-0034 §7 and SPIKE-004's deferred limits), and record the measurement in ADR-0055's history.

*Observed 2026-10-27 (architect-cto)*
