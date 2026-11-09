---
role: "language-culture"
class: workflow
topic: "keep-else-whole"
description: "In a spec's English pass, never reword an \"else\" branch into a narrower named condition; that changes behaviour"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6a4d6e3a56ef6bb9
  - 803b952717582c61
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
Related: [[solution-carried-2026-10-10]]

A second instance, caught before commit, 2026-10-24 (#411, ea187bee): in acceptance 11, "every such identity does" stood for two verbs: it "starts in the missing state and makes no key by itself". My first expansion, "starts in it", dropped the second half. An elided verb ("does", "is", "so does") stands for the whole predicate before it. When you expand one, copy the whole predicate or write "does the same"; never only its first verb.

Related, 2026-10-24 (#416): my wording pass on s5.4.4 wrote "the Undo, which the announcement no longer names". "no longer" put a fact into the past that was never true: the old announcement said only "Contact saved". Review round 2 caught it (c8bf5f38). A wording pass adds no claim about the past. "no longer", "still" and "again" each assert an earlier state, so check that state against the source before writing one.

And a second the same day (#416, review round 4, d04c411d): I reworded "is not timed out on the way" as "the snackbar does not close while the person moves focus". "does not close" is wider than "is not timed out", because the 2-minute cap and a swipe still close it. When I replace a technical verb (time out, refuse, retain) with a plainer one, check that the plain verb claims no more than the technical one. If no plain verb is that narrow, keep the technical verb.

2026-10-25 (#443, j193): the same holds for "whenever" — it states when something IS so, not that it is not so otherwise. A cold read proposed "only if"; that adds a rule, so it went to the author as a question.

*References: solution-carried-2026-10-10*

*Observed 2026-10-25 (language-culture)*
