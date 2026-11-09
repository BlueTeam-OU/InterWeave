---
role: "language-culture"
class: workflow
topic: "qualifier-is-a-claim"
description: "adding a qualifier to a noun (\"the size\" -> \"the size in bytes\") picks one meaning; check it against the text's facts before adding it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f5cac6159ab254f8
---

## adding a qualifier to a noun ("the size" -> "the size in bytes") picks one meaning; check it against the text's facts before adding it

A qualifier added to remove an ambiguity chooses one of the readings. If it chooses the wrong one, the sentence's meaning changes. On #447 (j197, 2026-10-25) I changed "So the size stays fixed" to "So the size in bytes stays fixed", following a cold read that asked "pixel size or byte size?". The fixed size was the 256 x 256 pixels (the owner's fixed-size rule earlier in 5.2). The byte count varies under the 24,576 bound, and the paragraph's own figures show that. ux-designer-01 reverted it (4ea6fcab).

**Why:** a cold read's "this could mean A or B" is a question, not an answer. Picking one side is a decision about meaning, which is the author's.

**How to apply:** before adding a qualifier, find the fact in the text that settles it (here the figures, all different sizes in bytes). If nothing settles it, ask the author and change nothing. Related: [[keep-else-whole]], [[workflow-carried-2026-10-10]].

Applied the same day (j200): "before preparation" was added only after the text settled it (every prepared photo is 256 x 256), and "at 4:4:4" only after card_photo.rs said so (4df2fa26, line 13).

*References: keep-else-whole, workflow-carried-2026-10-10*

*Observed 2026-10-25 (language-culture)*
