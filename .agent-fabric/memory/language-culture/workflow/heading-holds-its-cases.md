---
role: "language-culture"
class: workflow
topic: "heading-holds-its-cases"
description: "when a spec moves a case to another class, read the heading or row label it sits under; A16 \"L1\" on #659"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 79c61422db0a8fa2
---

## when a spec moves a case to another class, read the heading or row label it sits under; A16 "L1" on #659

When a design note moves a case from one class to another, read again every heading, lead-in and row label that still files it under the old class. On #659 (j305, 2026-11-13), §11 said that a network grant that ends the process "is not an L1 case". A16 still filed it under "L1, the prompt cancelled after:", and F11 under "§11, L1". I found this only in the second pass, against §11. The author then rewrote A16's lead-in too (4150d1ba1).

Also check the state the test reaches: "The conversation no longer shows the message" came right after a locked launch, and a locked launch shows no conversations. The author placed the check after "Unlock, granted".

**Why:** a heading frames every bullet under it. A tester who reads "L1" expects a prompt that rises by itself, not a launch.

**How to apply:** in a review where a case changes class, grep for the old class's label (L1, F11, A16 and so on) near the case. A step that checks what a screen shows must name a state in which that screen can show it. Related: [[word-against-the-where-cell]], [[read-the-cited-section]].

*References: read-the-cited-section, word-against-the-where-cell*

*Observed 2026-11-13 (language-culture)*
