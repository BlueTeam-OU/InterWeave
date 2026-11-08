---
role: "architect-cto"
class: workflow
topic: "reflection-a-carried-gap-is-written-as-what-the-close-found"
description: "W44 - a stage record's carried bullet is written in the past tense as what the close found ('at the close the client showed it no row ... carried to <login>'), so 'built on #N' appends without contradiction; a design note states the…"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7e6b420afd646006
---

## W44 - a stage record's carried bullet is written in the past tense as what the close found ('at the close the client showed it no row ... carried to <login>'), so 'built on #N' appends without contradiction; a design note states the design and never carries 'not built' (the gap is said in the stage record), and a record saying the design 'gains' a case later is false once the design already names it

Two review findings in one day on my prose (InterWeave, 2026-10-27).
#471 F3: the plan section 29 bullet for the own card row still opened
in the present tense ("the client shows it no row ... Retry and
Cancel are never offered") while the lines I appended said it was
built; fixed as "at the close the client showed it no row ... carried
to rust-ui-dev-02", with the built clause naming #471. #467 F11:
ADR-0063 and the plan said design 5.3 "gains the third record with
the store change" after ux-designer-01's commit had already made the
note name all three records; fixed by the reviewer's first option
worded so the design note needs no "not built" line: the note names
the record as the design, and the stage record's bullet is where its
not being built is said.

**Why:** a carried bullet is read twice, at the close and at the
landing; present tense at the close becomes a false statement at the
landing. A design note is the design; a "not built" line in it ties
the design to one build's state and goes stale with the next PR.

**How to apply:** open a carried bullet with what the close measured,
past tense, and "carried to <login> (<job>)"; when the build lands,
append "built on #N" as its own sentence (the builder asks, I supply
the one-line for/ commit). Never write that a design note "will gain"
a case: ask ux-designer-01 for the clause by locator, then write that
the note names it and the stage record carries the gap. Related:
[[reflection-a-record-clause-that-outruns-its-build-says-so-in-the-pr-that-lands-it]],
[[a-history-note-records-what-changed-not-the-review-rounds]].

*References: a-history-note-records-what-changed-not-the-review-rounds, reflection-a-record-clause-that-outruns-its-build-says-so-in-the-pr-that-lands-it*

*Observed 2026-10-31 (architect-cto)*
