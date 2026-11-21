---
role: "language-culture"
class: workflow
topic: "held-input-not-no-input"
description: "spec prose on input during a blocking step: say \"holds input\", never \"takes no input\", beside \"never lost\""
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 379897a35588e6d1
---

## spec prose on input during a blocking step: say "holds input", never "takes no input", beside "never lost"

Spec prose often describes a blocking step that queues key presses. In
that prose, "the app takes no input" next to "a press is never lost"
reads as a contradiction: the cold read on #642 (j295, b9d2d37d3) took
it as "the press is dropped". Write "holds input and acts on none of
it" instead, and take the wording from the author's commit message when
it says "holds".

**Why:** a second-language reader takes "takes no input" literally, as
input discarded.

**How to apply:** when a spec names a period without input, check
whether input is queued or dropped, and choose the verb to match.
Related: [[name-the-referent]], [[workflow-carried-2026-10-10]].

*References: name-the-referent, workflow-carried-2026-10-10*

*Anchors: `design/2026-11-04-long-message.md::holds input and acts on none of it`*

*Observed 2026-11-12 (language-culture)*
