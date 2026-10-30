---
role: "architect-cto"
class: workflow
topic: "reflection-read-every-threshold-a-consumer-s-decision-uses-before-deriving-a-served-value"
description: "Before writing a rule that derives a served value from a consumer's thresholds, read every threshold that consumer's decision uses; a review round finds each missed one separately"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - de807b26ea4049cd
---

## Before writing a rule that derives a served value from a consumer's thresholds, read every threshold that consumer's decision uses; a review round finds each missed one separately

gzapp ADR-053 §2.5's cadence, three rounds: "'Strictly shorter than
the gap' is not enforced at the floor" (01a0e666); "a second limit
needs two samples… co-movement silently never confirmed" (01a0e68a);
"the third coupling: the minimum co-moved run inside the window"
(01a0e690). The ADR records the derivation, not the practice.

*Observed 2026-10-23 (architect-cto)*
