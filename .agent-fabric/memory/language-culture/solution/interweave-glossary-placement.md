---
role: "language-culture"
class: solution
topic: "interweave-glossary-placement"
description: "Where InterWeave's person-facing glossary lives, how it is shaped and committed, and the platform split for daemon texts"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 54936e88093bb414
---

## Where InterWeave's person-facing glossary lives, how it is shaped and committed, and the platform split for daemon texts

- The glossary is architecture/docs/architecture/human-client-glossary.md
  (architect-cto-01's placement, REPLY 01a1248b, 2026-10-10): one table per
  step group, each row term | what it names (cited) | stays out; a later
  locale adds a column, not a file. My text by locator; the caller commits
  it onto its branch (first supplied as REQUEST 01a12492 to rust-ui-dev-01).
- labels.rs texts naming "the transport daemon" are desktop-only. Android
  has no daemon, and rust-ui-dev-01 decided platform values per key
  (REPLY 01a12491-8757, 2026-10-10); Android's term is "the network service".
  Some trust keys may not exist on Android at all: wait for the caller's key
  list before writing them.
- The store listing: my words, each claim tied to a fact [F#] that the owner
  verifies; architect-cto-01 carries it to the owner; the Play policy quote
  is plan §20 step 10's check, not mine.
Related: [[interweave-copy-conventions]]

*References: interweave-copy-conventions*

*Observed 2026-10-10 (language-culture)*
