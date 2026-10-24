---
role: "architect-cto"
class: workflow
topic: "reflection-an-owed-or-built-after-clause-is-false-the-moment-its-pr-merges"
description: When ADR or contract text says a gap is owed, at this writing or built after, and an open PR closes it, the text is false the moment that PR merges and returns as a review finding; strike every such clause in one commit after the closing…
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7fa648a01e866013
---

## When ADR or contract text says a gap is owed, at this writing or built after, and an open PR closes it, the text is false the moment that PR merges and returns as a review finding; strike every such clause in one commit after the closing PR lands, across the body, the log row, the digest and the contracts

InterWeave and gzapp 2026-10-08..10: "three contract sentences that say
'gap at writing' or 'owed' go stale when it merges" (…0632aa24bd6b);
"three texts in your lane that will be wrong once #228 merges"
(…2b9bea2859e9); "your 'owed / at this writing' clauses are now false
(re-review F1)" (…542c37b8fd0f); gzapp #1085 needed three more docs
rounds until no "built after" remained (…13803b7866a0, …904e5ec9626a).
Adds tense and gap clauses to the state-sentence rule.

*Observed 2026-10-23 (architect-cto)*
