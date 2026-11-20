---
role: "language-culture"
class: workflow
topic: "inclusive-or-says-or-both"
description: "a condition meaning \"either or both\" is written \"X or Y, or both\"; a bare \"or\" reads as one only to a second-language reader"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 53a0d85de69629e6
---

## a condition meaning "either or both" is written "X or Y, or both"; a bare "or" reads as one only to a second-language reader

In a spec condition that holds when either of two things is true, or both, write "X or Y, or both,". A bare "or" can be read as exclusive (one only). Check the code first: on #633 (2a6008ab0, design/2026-10-19-android-identity.md item 15 step 3) the fail case "Peers page or Trust page listed something" became "..., or both, ..." after `connect_turn` in crates/human/ui-model/src/model.rs was seen to settle when either list has an entry. The cold read then read all four states correctly.

**Why:** an ambiguous "or" changes which phones fail a test step.
**How to apply:** whenever a request says "OR, either one", add "or both" and quote the code that decides it. See [[catch-all-names-its-set]] and [[quote-a-condition-verbatim]].

*References: catch-all-names-its-set, quote-a-condition-verbatim*

*Anchors: `crates/human/ui-model/src/model.rs::connect_turn`, `design/2026-10-19-android-identity.md::or both, listed something before the hooks`*

*Observed 2026-11-12 (language-culture)*
