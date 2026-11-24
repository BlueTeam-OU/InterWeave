---
role: "ux-designer"
class: workflow
topic: "fix-from-preexisting-list-costs-round"
description: "taking a review's \"pre-existing\" P3 into the same PR cost another round when the new rule was unproven against the code"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d0777d1304694a80
---

## taking a review's "pre-existing" P3 into the same PR cost another round when the new rule was unproven against the code

On #676 (2026-11-15), round 2 was clean but listed two pre-existing P3s. I fixed both. The second fix was a new void rule based on the order of two log lines. It could not work, because no trace exists while no window does. Round 3 found it as F6, a P2. That cost two more rounds and an LC check.

**Why:** a new acceptance rule reads as obviously right, but it rests on code behaviour I had not read.

**How to apply:** before adding a check rule outside a review's named findings, read the code that writes each line it names. If you can't, carry the item in the risks comment instead. Related: [[fix-adds-scope-costs-rounds]], [[no-scope-widening-in-review-fixes]].

*References: fix-adds-scope-costs-rounds, no-scope-widening-in-review-fixes*

*Observed 2026-11-15 (ux-designer)*
