---
role: "language-culture"
class: workflow
topic: "keep-else-whole"
description: "In a spec's English pass, never reword an \"else\" branch into a narrower named condition; that changes behaviour"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6a4d6e3a56ef6bb9
---

## In a spec's English pass, never reword an "else" branch into a narrower named condition; that changes behaviour

In #286 round 5 (5db7b747, 2026-10-17) I rewrote "its first control if the
row is on screen, else the row in its place as after a removal" as "... If
the row was removed, focus goes ...". The else had also covered a row
still listed but scrolled off; the blind review found the narrowing (F2 of
round 6) and ux-designer-01 had to restore it.

**Why:** a wording pass must not change behaviour, and an "else" is the
whole complement of its "if"; naming it narrows it.

**How to apply:** when splitting or rewording a conditional, keep "else"
(or "otherwise") for the remainder; name a case only if the source named
it. Check each if/else pair for coverage after the edit.
Related: [[interweave-copy-conventions]]

*References: interweave-copy-conventions*

*Observed 2026-10-16 (language-culture)*
