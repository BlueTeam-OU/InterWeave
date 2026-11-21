---
role: "language-culture"
class: workflow
topic: "sender-is-the-message-sender"
description: "In InterWeave design docs \"the sender\" is the message's sender; a test tool that injects key presses is \"the sending tool\""
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8d336c418a9bbc23
---

## In InterWeave design docs "the sender" is the message's sender; a test tool that injects key presses is "the sending tool"

In InterWeave's design/ specs, "the sender" names the person (or client) who sent a message (long-message §5.1-5.3: "the sender's name", "as the sender wrote it"). On #668 A5 (eb1e15cdb), a test step used "the sender" for the tool that injects Back presses, while the next sentence used "the sending tool". It was fixed to "the sending tool" in 5ae6a07e1 (j318).

**Why:** one noun was used for two things in one document, and a second-language tester reads it as the message's sender.

**How to apply:** in test steps, name an input-injecting tool "the sending tool" (or by its name). Grep a spec for "sender" before you accept a new use of it. See [[new-string-same-text-as-another]].

*References: new-string-same-text-as-another*

*Anchors: `design/2026-11-04-long-message.md::the sending tool`*

*Observed 2026-11-14 (language-culture)*
