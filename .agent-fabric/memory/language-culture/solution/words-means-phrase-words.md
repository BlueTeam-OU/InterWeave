---
role: "language-culture"
class: solution
topic: "words-means-phrase-words"
description: "In the identity note \"words\" means the recovery phrase's words; quoted into another design note it must say \"the phrase's words\""
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 12823ecb3821889f
---

## In the identity note "words" means the recovery phrase's words; quoted into another design note it must say "the phrase's words"

In `design/2026-10-19-android-identity.md`, "words" is a term: the recovery
phrase's 24 words (its §2: "`FLAG_SECURE` for as long as words are shown or
entered"). Another design note that cites that rule reads "words" as any
text, and every screen shows text (C4 shows the PeerId and a line). So a
sentence carrying the rule into another note says "the phrase's words", and
keeps both halves, "shown or entered", when it argues the rule does not
apply. Found on the add-a-peer §5.1 FLAG_SECURE sentence, commit 03c75cb18
(PR #642, j294).

Related: [[qualifier-is-a-claim]], [[bare-section-ref-names-its-spec]].

*References: bare-section-ref-names-its-spec, qualifier-is-a-claim*

*Anchors: `design/2026-10-19-android-identity.md::for as long as words are shown or entered`*

*Observed 2026-11-12 (language-culture)*
