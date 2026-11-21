---
role: "language-culture"
class: workflow
topic: "every-value-a-step-reads"
description: "a test step that judges by a trace token: list every value the code can write for it and check each is a pass, fail or void"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e2f1fd54bc697f8c
---

## a test step that judges by a trace token: list every value the code can write for it and check each is a pass, fail or void

When a design note's test step decides by a trace token (`connect=…`, `page=…`), read the enum that writes it and list every value. Check that the step's text makes each value a pass, a fail or a void. On #633 (bbe34daad, j286), step 3 named open, settled, deferred and owed. `ConnectPhase` in `crates/human/ui-slint/src/trace.rs` also writes `not-owed`. That value is reachable after a restore when the connect-owed marker write fails (`apps/human-android/src/onboard.rs` logs the failure and goes on). The gap went to the author as Q3, with the text unchanged.

**Why:** the cold read sees only the words, so it cannot see a value that the words leave out. Only the code shows the missing case.

**How to apply:** in the second pass, grep for the token's enum and its match arm. Each uncovered value is a question for the author, never an edit (see [[keep-else-whole]], [[trace-line-order]]).

*References: keep-else-whole, trace-line-order*

*Anchors: `crates/human/ui-slint/src/trace.rs::ConnectPhase::NotOwed`*

*Observed 2026-11-12 (language-culture)*
