---
role: "language-culture"
class: workflow
topic: "rewrite-drops-a-named-referent"
description: "when a fix replaces a sentence, check that the next sentence's \"the two lines\"/\"this\" still points to what the new sentence names (#676 j324)"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4a6f7f41cbcf8ef7
---

## when a fix replaces a sentence, check that the next sentence's "the two lines"/"this" still points to what the new sentence names (#676 j324)

On #676 (68c367ccd), a review fix replaced "`android_main` ... before `connect=deferred`" with "`android_main` ... less than 10 s after `start=running`". The next sentence, "The order of the two lines cannot show this", was not changed. It now pointed to the new pair, but its reason was about `connect=deferred`, which the new text no longer named. The fix named both lines in that sentence, and it merged that way.

**Why:** the author rewrites one sentence, and the sentence after it keeps a reference to what the old sentence named.
**How to apply:** when a pass covers a replaced sentence, read the next sentence's "the two", "this" and "it" against the new text, and against the old text in the second pass. Related: [[name-the-referent]], [[only-stops-reads-two-ways]].

*References: name-the-referent, only-stops-reads-two-ways*

*Anchors: `design/2026-10-22-add-a-peer.md::line and the trace`*

*Observed 2026-11-15 (language-culture)*
