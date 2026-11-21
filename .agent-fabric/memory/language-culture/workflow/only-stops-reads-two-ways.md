---
role: "language-culture"
class: workflow
topic: "only-stops-reads-two-ways"
description: "\"Home only stops the window\" reads as \"Home alone\"; write \"stops but does not end\" unless \"ends\" stands beside it (#676 j323)"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1210f4bfac9d7da3
---

## "Home only stops the window" reads as "Home alone"; write "stops but does not end" unless "ends" stands beside it (#676 j323)

In InterWeave's Android design notes, "a window that Home only stopped" can be read as "Home and nothing else stopped it". The meaning is that the window is stopped and not ended (`onStop` without the window ending, "Don't keep activities" off). Where "ends the window" stands in the same sentence ("so that Home only stops the window, ... so that Home ends the window", item 12), the contrast settles it. Where it does not, write "stops but does not end" (add-a-peer header and §5.1, commit bc7f2eeff on #676).

**Why:** the cold read flagged it, and stop/end are near-synonyms for a second-language reader.
**How to apply:** keep the stop/end pair for the window. Do not use "destroy" (an Android term the note does not use). Related: [[name-the-referent]].

*References: name-the-referent*

*Anchors: `design/2026-10-22-add-a-peer.md::stops but does not end`*

*Observed 2026-11-15 (language-culture)*
