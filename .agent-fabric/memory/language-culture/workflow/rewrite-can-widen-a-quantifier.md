---
role: "language-culture"
class: workflow
topic: "rewrite-can-widen-a-quantifier"
description: "when an author rewrites a paragraph, compare each \"each/every/all\" with the previous version; a moved sentence can widen its scope without anyone noticing"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c6fc5ac5006e2b4c
---

## when an author rewrites a paragraph, compare each "each/every/all" with the previous version; a moved sentence can widen its scope without anyone noticing

On #668 (j317, 2026-11-14), ux-designer-01 rewrote A5's Back paragraph (abac13098). They moved "by each way the platform gives" out of the stress-shape sentence into a sentence of its own: "On Android, each Back is made by each way the platform gives". The sentence now covered the list shape's 0.2 s Back too: three runs where there had been one. The commit message did not mention it, and the "Not run yet" list did not grow. The second pass, against the diff, caught it. The cold read confirmed it by counting the runs it would make (9 single runs, not 7). The author said the widening was not meant and narrowed it with my words (b89b586f4).

**Why:** in a rewrite the words stay right and the scope moves. A first pass reads a correct sentence. Only a comparison with the old text shows that "each" now covers more.
**How to apply:** in the second pass of a rewritten paragraph, list each quantifier ("each", "every", "all", "only") and what it covered before and after. Ask the cold read to list every run or case the text requires, and compare that count with the records and the "not run" lists. Ask the author; do not narrow it myself. Related: [[keep-else-whole]], [[quote-a-condition-verbatim]], [[catch-all-names-its-set]].

*References: catch-all-names-its-set, keep-else-whole, quote-a-condition-verbatim*

*Anchors: `design/2026-11-04-long-message.md::each Back on a stress shape is made in each of the three ways`*

*Observed 2026-11-14 (language-culture)*
