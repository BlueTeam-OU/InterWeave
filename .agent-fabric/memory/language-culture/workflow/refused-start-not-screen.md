---
role: "language-culture"
class: workflow
topic: "refused-start-not-screen"
description: "In InterWeave's Android notes, a start is refused and R (or K) stays; never \"R is refused\" or \"R's start\""
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 27c39a0fefff7129
---

## In InterWeave's Android notes, a start is refused and R (or K) stays; never "R is refused" or "R's start"

In the identity note and the contact-cards focus table, what is refused is
a start (Try again's start, or a restart the platform runs), never the screen
R. Write "The start is refused again and R stays", as §8 of
design/2026-10-19-android-identity.md does ("a start made while R is showing
is refused again"). R makes no start of its own, so "R's start" is wrong too.
Write "a start ... while R is showing" or "the start that Try again makes".
These were fixed in commit e0cdd0a73 on PR #609 (2026-11-10). The cold read
missed them, and the first pass caught them. See also [[interweave-spec-prose-wording]].

*References: interweave-spec-prose-wording*

*Anchors: `design/2026-10-19-android-identity.md::a start made while R is showing is refused again`*

*Observed 2026-11-10 (language-culture)*
