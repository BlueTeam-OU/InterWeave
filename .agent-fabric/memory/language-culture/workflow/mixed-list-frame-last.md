---
role: "language-culture"
class: workflow
topic: "mixed-list-frame-last"
description: "in a list where some items share a frame (\"a tap on X, on Y, or on Z\"), put the other items first; a trailing \", Back and Escape\" read as taps (#645 j300)"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a091866900f3188d
---

## in a list where some items share a frame ("a tap on X, on Y, or on Z"), put the other items first; a trailing ", Back and Escape" read as taps (#645 j300)

In spec prose, a list that mixes items under a shared frame ("a tap on a disabled key, on text ..., or on no control at all") with items outside it (Back, Escape) is ordered so that the items outside the frame come first and the framed items come last. A sentence that follows ("Such a tap ...") then sits next to the framed items it refers to. On #645 (commit ea79c410c), "... or on no control at all, Back and Escape" made the cold read ask whether Back and Escape were things tapped. Reordering a list changes no behaviour, but check that the set of items stays the same ([[keep-else-whole]]).

**Why:** a reader who learned English as a second language extends the last frame over every item that follows it.

**How to apply:** when a list item gains a long parenthesis, read where the list ends; move items without the frame to the front.

*References: keep-else-whole*

*Anchors: `design/2026-10-19-android-identity.md::Every other input leaves the mode as it was: Back, Escape`*

*Observed 2026-11-12 (language-culture)*
