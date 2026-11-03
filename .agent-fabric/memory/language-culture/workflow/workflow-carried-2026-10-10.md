---
role: "language-culture"
class: workflow
topic: "workflow-carried-2026-10-10"
description: How the English cold read behaves when given only the text and the rules — what it catches and what to check before taking a finding
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 55f2fdcdfd91fd88
  - 8cc880570d5f10c5
  - fb7bf265621d313b
---

## How the English cold read behaves when given only the text and the rules — what it catches and what to check before taking a finding

The English holder's cold read (a fresh read-only dispatch given only the
composed text and the vocabulary rules, never the request) caught my own
overclaim ("Published to the channel") and real ambiguities (pronouns, a
status indistinguishable from another). But 9 of its 25 findings
(2026-10-09, labels.rs) were withdrawn on a check against the code: it
cannot know which values fill a template, which labels are inbound-only,
or that a string is for a screen reader only. So judge each finding
against the code before taking it, and record what you withdraw and why.
Related: [[interweave-copy-conventions]]
- Spec cold reads (j50, 2026-10-16): give the reader the spec's defined terms, or most findings are 'X is undefined' (armed, Unkeep, store) and withdrawn; its useful catches are word collisions with the product's own verbs (keep/Keep, hide/Hide).
- A last read before arming weighs the fold and re-review an edit costs: a cold-read "its" with only one sensible referent in the spec is declined, not reworded (#432, 8638ebb7, j178).

2026-10-25 (j191): spans cut with `git diff -U1` gave the cold read half-sentences, and about half its findings were "this sentence is cut off". Give it whole paragraphs around each change (or the whole section), not diff context lines.
Same day (j192): whole paragraphs worked — the cut-sentence findings went away, and what was left were real or outside the range.

*References: interweave-copy-conventions*

*Observed 2026-10-25 (language-culture)*
