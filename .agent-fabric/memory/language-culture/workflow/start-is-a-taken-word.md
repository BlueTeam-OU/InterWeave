---
role: "language-culture"
class: workflow
topic: "start-is-a-taken-word"
description: "In InterWeave's Android notes \"start\" is the service session; say \"from zero\" for a restarted timer, \"service start\" for the process"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 722d1451131017f5
---

## In InterWeave's Android notes "start" is the service session; say "from zero" for a restarted timer, "service start" for the process

In the Android design notes (add-a-peer §5.1, the identity note), "a start" / "the next start" means one start of the service session, and "the service start" is the act of starting it. So a timer that restarts is written "starts again from zero", never "counts again from the start", and a timer that resumes "goes on from where it stopped, not from zero". Where both senses are near, write "service start" for the act (#642, j296, e98ac8051).

**Why:** the author's "(b) counts again FROM THE START" and "(d) counts from the start" each read two ways beside "the service start succeeding"; (d) went back as a question.

**How to apply:** in any pass on these notes, grep the passage for "start" and name which one each is; check a timer's origin against every screen it covers (on R4 the identity was committed at R3, so "10 s after the identity was committed" needed R4's press named apart). A pointer inside the text is never "the start of this item": write "the first paragraph of this item" (#681, j326, 2522bb45b). See [[name-the-referent]], [[qualifier-is-a-claim]].

*References: name-the-referent, qualifier-is-a-claim*

*Anchors: `design/2026-10-22-add-a-peer.md::starts again from zero`*

*Observed 2026-11-15 (language-culture)*
