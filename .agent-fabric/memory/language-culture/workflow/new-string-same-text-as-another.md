---
role: "language-culture"
class: workflow
topic: "new-string-same-text-as-another"
description: before finalising a new string, grep the strings table for the same text under another key
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4618edfa55089ec8
---

## before finalising a new string, grep the strings table for the same text under another key

Before I finalise a new string, I search the strings table (and labels.rs) for the same English text under another key. In #435, the proposed CardUpdatesPhotoRemoved "Photo removed" (a change shown before Accept) was the same text as ConfirmPhotoRemoved (a snackbar after a removal, design/2026-10-21-contact-cards.md s10). One text for two meanings means one of them reads wrong. Delivered as "Current photo removed" (commit 5dd65913, 2026-10-25).

**Why:** the cold read judges each string against its own row only. It cannot see that the same text already means something else elsewhere.

**How to apply:** for each new key, run `grep -n '| <text> |'` over the s10 table and grep labels.rs before the cold read. Treat a match as a finding. Related: [[word-against-the-where-cell]], [[one-text-read-in-each-place]].

*References: one-text-read-in-each-place, word-against-the-where-cell*

*Observed 2026-10-25 (language-culture)*
