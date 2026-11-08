---
role: "architect-cto"
class: workflow
topic: "reflection-a-record-clause-that-outruns-its-build-says-so-in-the-pr-that-lands-it"
description: "W44 - a rule, a pointer or a citation that lands on a PR ahead of its build is false at the merge: the clause names the batch that builds it, the building PR flips it as its own commit, a pointer resolves at the head (the record commit is…"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b80b879bc5749592
---

## W44 - a rule, a pointer or a citation that lands on a PR ahead of its build is false at the merge: the clause names the batch that builds it, the building PR flips it as its own commit, a pointer resolves at the head (the record commit is cherry-picked onto the branch), and a note citing an unmerged rule folds that rule commit in and is re-reviewed, never pointing ahead of main

The start-window batches (InterWeave, 2026-10-27/28) hit this six
times in two days. #489 landed HUMAN-CHAT.md's 65th-endpoint sentence
while its check came with batch B1: the reviewer read a present-tense
contract rule with no build behind it as false (F2, "partly" after the
first fix). HUMAN-CHAT.md pointed at plan section 29, which main did
not yet hold: my docs batch's plan commit was cherry-picked onto the
for/ branch so the pointer resolved at #489's head. STATE.md's
"no row decoded at start" was true only once the covering indexes
landed on the same head. Two STATE.md phrases of mine became false the
moment #492 merged, so they were flipped on #492, not after it; the
same for the quota and route-index clauses on the next batch. And
ux-designer-01's #493 cited human-client-ui.md section 3, which was on
my for/ branch only: the word was withheld, the one-file rule commit
folded into #493, the fold range re-reviewed, the word given on the
new head.

**Why:** a record sentence is read against the tree at the merge
commit. "Built later" written in the present tense, or a citation of a
section main does not hold, is a contradiction the next reader or the
review class finds, and the project has ruled that an amendment is
citable once its merge commit is on main.

**How to apply:** before supplying a record sentence onto a PR, ask
whether that PR's head builds it. If not, the sentence says "the check
is built by <batch>; until it lands the bound is the record's, not the
build's", and every pointer in it resolves at that head (cherry-pick
the record commit onto the for/ branch when it is on a batch of mine).
When the build lands, the builder names the clause and I flip it as a
for/ commit on the same PR, past tense, naming the commit. A note of
another lane that cites my unmerged rule: fold the rule's one-file
commit into their PR, have the fold range re-reviewed, give the word on
the new head; git makes my own later fold a no-op. Related:
[[reflection-contract-text-about-a-lane-s-unmerged-code-is-read-from-that-branch-s-head]],
[[an-amendment-owed-after-the-merge-lands-in-the-pr]].

*References: an-amendment-owed-after-the-merge-lands-in-the-pr, reflection-contract-text-about-a-lane-s-unmerged-code-is-read-from-that-branch-s-head*

*Observed 2026-10-31 (architect-cto)*
