---
role: "language-culture"
class: workflow
topic: "name-the-referent"
description: "a \"that\"/\"which\"/\"it\" after two clauses, or before text a screen reader reads next, is replaced by the noun it means"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1f7576717e407884
---

## a "that"/"which"/"it" after two clauses, or before text a screen reader reads next, is replaced by the noun it means

On 2026-10-23, three drafts in a row had a pointing word with two referents, and the cold read caught each one:
- IdActiveElsewhere (#392): "..., which must not happen. This phone cannot check that." Is it the restore or the shared PeerId that must not happen? It became "One PeerId must never be on two devices, and this phone cannot check whether the identity is still active elsewhere."
- PeerRowGone (j138): "That address ..." is announced just before the next row's label, so "that" binds to the row the reader hears next. It became "The address you acted on ...".
- PeerStateThisDevice (j135): an "it" that could mean the address or the device. It became one subject.

**Why:** a second-language reader, or a listener with no screen to look back at, resolves "that", "which" or "it" to the nearest thing, and that is often the wrong one. Tightening the length tempts you to bring the pronoun back.

**How to apply:** in the first pass, circle every "that", "which", "it" and "this". Name the noun wherever two candidates precede it, or wherever the next text read aloud could be taken for it. When a length brief pushes back, keep the noun and state the length. Related: [[one-text-read-in-each-place]], [[cold-read-without-meanings]].

*References: cold-read-without-meanings, one-text-read-in-each-place*

*Anchors: `interweave:design/2026-10-22-add-a-peer.md`, `interweave:crates/human/ui-model/src/labels.rs`*

*Observed 2026-10-23 (language-culture)*
