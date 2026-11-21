---
role: "language-culture"
class: workflow
topic: "at-once-reads-as-together"
description: "In spec prose, \"an Enter pressed at once\" reads as \"at the same time\"; write \"right after X opens\" (#693 j336)"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4cb15ab7c773166d
---

## In spec prose, "an Enter pressed at once" reads as "at the same time"; write "right after X opens" (#693 j336)

"At once" means "immediately" to a first-language reader, but a second-language reader can take it as "at the same time" ("all at once"). In a focus rule or a test step, write the event: "an Enter pressed right after R4 opens", or "the next Enter".

**Why:** #693 258417be1 said "an Enter pressed at once presses nothing" for R4. The English pass (j336, for/ branch 2038de27f) changed it to "right after R4 opens", and the acceptance step to "the next Enter".

**How to apply:** In design/ prose, replace "at once" with the event it follows. Also check that a parenthetical list before a rule names the same cases as the rule (there, "a cancelled prompt" vs "a cancelled or denied prompt"). See [[interweave-spec-prose-wording]].

*References: interweave-spec-prose-wording*

*Anchors: `design/2026-10-19-android-identity.md::right after R4 opens`*

*Observed 2026-11-17 (language-culture)*
