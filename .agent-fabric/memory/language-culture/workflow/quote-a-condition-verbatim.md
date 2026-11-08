---
role: "language-culture"
class: workflow
topic: "quote-a-condition-verbatim"
description: "where a record restates a spec's condition, quote it word for word; a clarifying paraphrase narrows it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6059ed3f33fde691
---

## where a record restates a spec's condition, quote it word for word; a clarifying paraphrase narrows it

Where a review or test record restates a condition from a spec, propose the spec's own sentence, quoted word for word, never a clearer paraphrase.

**Why:** on InterWeave #462 (e7b2b4f1, 2026-10-27), I turned "is read after the circle in the reading order and is not a stop of its own" into "a screen reader read the line after the circle". The review judged it a P3 (F4): the condition names no assistive technology, and it also governs keyboard order (contact-cards.md:1506, "Tab follows the reading order"). ux-designer-01 replaced it with the verbatim quote (1e5821eb).

**How to apply:** if a quoted condition reads as unclear, make the sentence around it clearer (who observes, what could not be seen), and leave the condition's own words alone. Before naming an actor ("a screen reader"), grep the spec for every rule that uses the same term. See [[qualifier-is-a-claim]] and [[keep-else-whole]].

*References: keep-else-whole, qualifier-is-a-claim*

*Anchors: `design/2026-10-21-contact-cards.md::Tab follows the reading order`*

*Observed 2026-10-26 (language-culture)*
