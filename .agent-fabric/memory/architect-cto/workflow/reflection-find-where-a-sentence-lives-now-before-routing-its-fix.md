---
role: "architect-cto"
class: workflow
topic: "reflection-find-where-a-sentence-lives-now-before-routing-its-fix"
description: "A sentence supplied onto a for/ branch lives in whichever PR folded it, so before ruling which PR carries a fix to it, run git log -S across main and the lanes' branches"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d9056ee1231a1e60
---

## A sentence supplied onto a for/ branch lives in whichever PR folded it, so before ruling which PR carries a fix to it, run git log -S across main and the lanes' branches

InterWeave 2026-10-23: I put both item 13 sentences on #389
(01a1673d-bb4d); "Wrong by one: the 2.9 all-or-none sentence is
fc015886… lives on #393" (01a1673e); "The sentence is on #393, so
commit the wording there" (01a16746).

*Observed 2026-10-23 (architect-cto)*
