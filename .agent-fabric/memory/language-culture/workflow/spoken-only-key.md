---
role: "language-culture"
class: workflow
topic: "spoken-only-key"
description: a key heard only by a screen reader, in the middle of text — lowercase, no brackets, and ask the build for a pause around it
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - dba4e6d72d19e3e8
---

## a key heard only by a screen reader, in the middle of text — lowercase, no brackets, and ask the build for a pause around it

A string that is never on screen and is spoken in the middle of other text (ReplacedCharacter, InterWeave #564, 2026-11-05: "character not shown") starts lowercase, because braille displays show it mid-sentence. It carries no brackets, because Orca may speak them at some punctuation levels. The words alone do not show where they start and end ("The café character not shown is open"). The cold read caught this.

**Why:** a speech engine joins the words to the letter beside them unless the accessible text puts a space and a pause there, and that is build behaviour, not the key's text.
**How to apply:** propose the words only, and ask the design's author to require a pause (a comma) on each side and a space where the replaced item touches a letter. Reuse the wording of an existing on-screen sibling (ImageNotShown) where one exists. Reject a phrase that is false about the state ("unknown character" when the app holds the character). See [[workflow-carried-2026-10-10]].

**Update 2026-11-07 (j249):** the long-message note's section 6 now puts the pause in the string, not in the build. The final English is ", character not shown,". It has a comma at each end, and the client adds a space where the mark touches a non-space. The cost is that braille shows a lone " , " before it. That cost goes to rust-ui-dev as their call. Where a spec puts the pause in the string, write the commas; otherwise ask the build for them, as above.

*References: workflow-carried-2026-10-10*

*Observed 2026-11-07 (language-culture)*
