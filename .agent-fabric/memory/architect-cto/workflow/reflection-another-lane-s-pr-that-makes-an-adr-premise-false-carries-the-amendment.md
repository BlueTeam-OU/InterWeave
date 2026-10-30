---
role: "architect-cto"
class: workflow
topic: "reflection-another-lane-s-pr-that-makes-an-adr-premise-false-carries-the-amendment"
description: When a PR changes a fact an ADR states as current, the ADR is amended in that same PR even if its conclusion holds, and before supplying the amendment the fact is grepped in the body, the security section, the digest and the log row
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b2d27b967d5fae34
---

## When a PR changes a fact an ADR states as current, the ADR is amended in that same PR even if its conclusion holds, and before supplying the amendment the fact is grepped in the body, the security section, the digest and the log row

2026-10-19..20: "a premise an ADR states as current is amended the
moment the tree makes it false, in the PR that makes it false"
(01a1561e-1d04); ADR-047 §2.6 stale again the next day (01a157c2-
66c0); the decision-4 amendment needed two rounds, four places then
two more (01a15aba-b00d, 01a15b98-5931). The converse of the owed-
clause rule.

*Observed 2026-10-23 (architect-cto)*
