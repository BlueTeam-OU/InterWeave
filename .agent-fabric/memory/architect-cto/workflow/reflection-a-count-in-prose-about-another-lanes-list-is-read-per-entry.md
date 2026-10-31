---
role: "architect-cto"
class: workflow
topic: "reflection-a-count-in-prose-about-another-lanes-list-is-read-per-entry"
description: "A count or 'the one X today' sentence in a record about another lane's list is read from that list at the PR head, entry by entry, never inferred from the paragraph's own rule (gzapp #1164 F4, 2026-10-26)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d0dcf32a56c4cc31
---

## A count or 'the one X today' sentence in a record about another lane's list is read from that list at the PR head, entry by entry, never inferred from the paragraph's own rule (gzapp #1164 F4, 2026-10-26)

On gzapp #1164 my suite record said "the one entry today is the content
detail page's level-one heading while its item loads or fails", while
`apps/admin_web/src/a11y/exemptions.ts` at the head built one
AxeExemption per state (loading, error): two entries, and the sentence
contradicted the one-audited-state rule stated in the same paragraph.
Caught by the blind review's F4 (P3), fixed as c94eb84c4.

**Why:** a record's example is checked by the same reader as its rule;
an example that breaks the rule it illustrates teaches the opposite.

**How to apply:** before writing "the N entries today" or "the one X"
about a list another lane owns, open the list at the PR head and count
entries as the code builds them (a `.map` over states is one per
state); where a count will drift, name the shape ("one entry per
audited state") rather than the number.

*Observed 2026-10-26 (architect-cto)*
