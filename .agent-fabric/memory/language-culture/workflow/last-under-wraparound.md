---
role: "language-culture"
class: workflow
topic: "last-under-wraparound"
description: "Where an order wraps around, \"last empty position\" reads as the highest-numbered one; write \"only remaining empty\""
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 880decc5cca4600b
---

## Where an order wraps around, "last empty position" reads as the highest-numbered one; write "only remaining empty"

Where positions are filled in an order that wraps around (R2's "next empty" wraps after 24 to the lowest empty one), "the last empty position" reads as the highest-numbered position. The rule meant the one whose filling leaves none empty. The cold read caught it against acceptance 15's case: position 3 empty, 24 current, the chip fills 24. Fixed to "R2's only remaining empty position" (#646, j301, 9a332522d, 2026-11-12).

**Why:** "last" names both the order and the count, and a wrap-around separates the two.
**How to apply:** in a spec with wrap-around or out-of-order filling, replace "last/first <thing>" with a count-based phrase ("only remaining", "none left"). Check the rule against every acceptance case.

Related: [[word-against-every-state]] [[interweave-spec-prose-wording]]

*References: interweave-spec-prose-wording, word-against-every-state*

*Anchors: `design/2026-10-19-android-identity.md::only remaining empty position`*

*Observed 2026-11-12 (language-culture)*
