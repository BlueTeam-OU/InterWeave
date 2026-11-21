---
role: "language-culture"
class: workflow
topic: "catch-all-names-its-set"
description: "a \"none of the N above\" fail case lists its N members; \"X or Y, with Z\" is written as whole pairs"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0b8e9b5fec8e3e10
---

## a "none of the N above" fail case lists its N members; "X or Y, with Z" is written as whole pairs

On #633 (design/2026-10-19-android-identity.md §10 item 15 step 3, my f8b70ac19), the cold
reader could not tell which four pairs "the pair is none of the four above" meant. The
fourth (connect=deferred with page=conversations) was described in another sentence and
was not called a pair. "connect=settled or connect=not-owed, with page=conversations" also
let the "with" bind to the second value only.

**Why:** a catch-all fail case is only as exact as the set it excludes. A second-language
tester who counts the set wrong passes or fails the wrong outcome.

**How to apply:** when a test step has a catch-all ("none of the N above", "any other"),
list the N members in brackets at the catch-all. Write every pair whole ("A with B, and so
is C with B"), never "A or C, with B". Then ask the cold reader to list every possible
outcome as pass, fail or void. That question found an outcome the text did not decide
(Q4: connect=open on a phone that had entries). Related: [[every-value-a-step-reads]],
[[word-against-every-state]], [[workflow-carried-2026-10-10]].

*References: every-value-a-step-reads, word-against-every-state, workflow-carried-2026-10-10*

*Anchors: `design/2026-10-19-android-identity.md::none of the four pairs above`*

*Observed 2026-11-12 (language-culture)*
