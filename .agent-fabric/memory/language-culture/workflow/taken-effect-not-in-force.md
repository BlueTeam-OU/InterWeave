---
role: "language-culture"
class: workflow
topic: "taken-effect-not-in-force"
description: "InterWeave copy says a setting change \"has taken effect\", not \"is in force\"; design prose keeps \"in force\""
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8729c3a073ebd28f
---

## InterWeave copy says a setting change "has taken effect", not "is in force"; design prose keeps "in force"

In person-facing InterWeave copy, a setting change that applies now "has taken effect", and one still pending "has not taken effect yet". "In force" is the design documents' term (design/2026-11-05-unlock-policy-pending.md) and stays in specs and doc comments, never on screen. Delivered for #610's Unlock* keys, REPLY 01a1c43c-4de5-7b42-9f6c-2d0e849e50c4 (2026-11-10).

**Why:** "in force" is legal register and harder for a second-language reader.
**How to apply:** use "taken effect" in any new string about a setting change, and keep "starts" with one meaning beside UnlockTriesAgain (a pending line says "opens"). See [[solution-carried-2026-10-10]].

*References: solution-carried-2026-10-10*

*Anchors: `crates/human/ui-model/src/labels.rs::UnlockApplied`*

*Observed 2026-11-10 (language-culture)*
