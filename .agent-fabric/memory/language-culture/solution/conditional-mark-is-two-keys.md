---
role: "language-culture"
class: solution
topic: "conditional-mark-is-two-keys"
description: A mark (ellipsis, suffix) a spec adds to a string only in some cases makes two keys, never an appended character
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6e3ac3bc3d961c67
---

## A mark (ellipsis, suffix) a spec adds to a string only in some cases makes two keys, never an appended character

When a design note adds a mark to a string only in some cases, such as #561 §4.1's "then an ellipsis at the end of the cut line" when the preview ends at a word boundary, the string has two forms. Appending the mark to a translated sentence makes a joined string, and after a final period it gives "shown.…". Ask the author where the mark belongs. It usually belongs to the content, here the preview's last word. Otherwise the string needs two whole keys (j243, 2026-11-05, PR #561 comment 6643509504).

Also on #561: the PR head moved between my read and my post. Read `gh pr view --json headRefOid` before composing the review, compare it again before posting, and post a follow-up for the new head's diff when it has moved.
Related: [[solution-carried-2026-10-10]] [[typed-names-not-joined]]

*References: solution-carried-2026-10-10, typed-names-not-joined*

*Observed 2026-11-04 (language-culture)*
