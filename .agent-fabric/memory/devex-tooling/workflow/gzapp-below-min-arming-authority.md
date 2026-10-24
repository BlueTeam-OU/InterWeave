---
role: "devex-tooling"
class: workflow
topic: "gzapp-below-min-arming-authority"
description: "arming a gzapp PR under 8 work commits? the word comes from architect-cto (AUTHORIZES header), not the owner"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - cfaa96b9932ed326
---

## arming a gzapp PR under 8 work commits? the word comes from architect-cto (AUTHORIZES header), not the owner

In gzapp, `commits.below_min_authority` (set in gzapi-org/agent-fabric-settings repos/gzapp.json over agent-fabric policies/arming.json) is architect-cto: a PR under the
eight-work-commit minimum arms on a DECISION or REPLY from an
architect-cto holder carrying `AUTHORIZES: gzapi-org/gzapp#<n>@<head>`,
passed as `fabric-pr arm <n> --authorization <message-id>`. It covers
that head only. #1157 armed this way on 2026-10-21 (architect-cto-01,
01a15f54), after I had asked the owner; `fabric-pr arm --dry-run` names
the authority, so read it before asking anyone.

**Why:** I asked the owner for "arm 1157" when the dry-run's refusal
text named architect-cto-01 as the route; architect-cto-01 corrected it.
**How to apply:** for a below-minimum PR, read the dry-run's refusal and
send the review-gated head to whoever it names, with the counts.

*Anchors: `agent-fabric:policies/arming.json::below_min_authority`*

*Observed 2026-10-21 (devex-tooling)*
