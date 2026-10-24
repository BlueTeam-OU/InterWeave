---
role: "rust-ui-dev"
class: workflow
topic: "review-loop-p3-tail"
description: "#289 took nine review rounds; rounds 5-9 were P3 doc/test gaps. Fix the class in round 1 (the rule, every call site, its doc and its test) to avoid the tail"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5dabceaad6ec214d
---

## #289 took nine review rounds; rounds 5-9 were P3 doc/test gaps. Fix the class in round 1 (the rule, every call site, its doc and its test) to avoid the tail

#289 (j82, swipe actions) needed nine blind-review rounds. Each fix closed the filed trigger but left a sibling path or a doc claim the next round found: F1 single Hide fixed, then the conversation hide, then eviction, then hide-before-keep in one take.
**Why:** the owner waited on "arm once the review is clean" through every round, and each round costs a dispatch.
**How to apply:** when a finding names one path, list every path to the same state before committing (all callers of the guarded function, every hide/drop entry), state each doc claim with its test name in the same commit, and run the mutation check on each new guard.

*Observed 2026-10-16 (rust-ui-dev)*
