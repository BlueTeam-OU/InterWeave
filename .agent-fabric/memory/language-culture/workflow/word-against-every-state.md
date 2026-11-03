---
role: "language-culture"
class: workflow
topic: "word-against-every-state"
description: a string shown or spoken beside a message status is read with every state of that status; no tense or verb may claim delivery
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 26e06ce552f4ce51
---

## a string shown or spoken beside a message status is read with every state of that status; no tense or verb may claim delivery

A string that sits on a row with a message status (Sending, Not sent, Accepted by remote transport) must read true beside every one of those statuses. So it never says "was sent", "shared" or "delivered".

**Why:** on InterWeave #469 (2026-10-27), I worded CardSentPhoto as "Your photo was sent with the card." against the Accepted row alone, and the re-review found it contradicted "Not sent" (F6). The final string is "The card includes your photo." The same check found CardSentLabel, "You shared your card", contradicting "Not sent". I asked ux-designer-01 about it (REPLY 01a17b8b).

**How to apply:** before wording such a string, list the row's states from the spec, and for the cold read give the reader the whole label once per state. Treat a requester's "what the other side received" as describing the content, not as a claim of delivery. See [[word-against-the-where-cell]].

*References: word-against-the-where-cell*

*Anchors: `design/2026-10-21-contact-cards.md::CardSentPhoto`*

*Observed 2026-10-27 (language-culture)*
