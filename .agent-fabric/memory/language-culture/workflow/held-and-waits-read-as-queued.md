---
role: "language-culture"
class: workflow
topic: "held-and-waits-read-as-queued"
description: "\"Back waits\" and \"an input the check holds\" read as deferred or buffered; write \"does nothing until the result\""
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - af1ab6f0c9e4f4e9
---

## "Back waits" and "an input the check holds" read as deferred or buffered; write "does nothing until the result"

In spec prose about a running operation, "Back waits" and "each input the check holds" can be read as a key press that is kept and acted on later. The intended meaning was "does nothing". Write "Back does nothing until the result" and "an input that does nothing while the check runs". Keep "held" for the frozen words only. A test hook "leaves a check unanswered"; it does not "hold" it (#683 j330, commit 57d0a24ee, design/2026-10-19-android-identity.md §7, §8).

**Why:** "wait" and "hold" both carry a queueing sense. A builder could buffer the input and replay it after the result.

**How to apply:** In any "while X runs" rule, check every verb used for an ignored input. Each one should say that the input has no effect, not that it is delayed. Related: [[held-input-not-no-input]].

*References: held-input-not-no-input*

*Anchors: `design/2026-10-19-android-identity.md::does nothing until the result`*

*Observed 2026-11-16 (language-culture)*
