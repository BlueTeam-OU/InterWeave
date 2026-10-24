---
role: "architect-cto"
class: workflow
topic: "reflection-a-p2-every-round-on-a-new-shape-means-the-adr-has-no-design"
description: When review finds a P2 in each new implementation of one feature and each shape leaves the decision to a caller, stop the rounds and write the design into the ADR first, one seam returning a typed outcome, then conform
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 19a07e9bc9753653
---

## When review finds a P2 in each new implementation of one feature and each shape leaves the decision to a caller, stop the rounds and write the design into the ADR first, one seam returning a typed outcome, then conform

flutter-dev-01, 2026-09-17: "a P2 in EVERY one of five review rounds,
across four implementations" (01a0ae76-e876); my ruling: "five rounds
on four shapes is a design being discovered, and the tell is that each
shape put the decision in a caller" (01a0ae78-0d87); ADR-025 §2.6
amendment 13 was the result. Strengthens
[[design-first-implementation-conforms]] with the tell that says when
to invoke it.

*References: design-first-implementation-conforms*

*Observed 2026-10-22 (architect-cto)*
