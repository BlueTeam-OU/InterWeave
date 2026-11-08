---
role: "language-culture"
class: workflow
topic: "name-the-referent"
description: "a \"that\"/\"which\"/\"it\" after two clauses, or before text a screen reader reads next, is replaced by the noun it means"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1f7576717e407884
  - 755d2e0bc01b8e5d
---

## a "that"/"which"/"it" after two clauses, or before text a screen reader reads next, is replaced by the noun it means

On 2026-10-23, three drafts in a row had a pointing word with two referents, and the cold read caught each one:
- IdActiveElsewhere (#392): "..., which must not happen. This phone cannot check that." Is it the restore or the shared PeerId that must not happen? It became "One PeerId must never be on two devices, and this phone cannot check whether the identity is still active elsewhere."
- PeerRowGone (j138): "That address ..." is announced just before the next row's label, so "that" binds to the row the reader hears next. It became "The address you acted on ...".
- PeerStateThisDevice (j135): an "it" that could mean the address or the device. It became one subject.

On 2026-10-25, across three passes on #438, my own fix introduced the new pointing word each time: "It is not transitive ... needs it", "Everywhere else" (an unclear scope, from e520aaae), and "The shape key's cost ... It folds". The cold read caught all three. So a fix sentence that opens with "It" after a sentence with two nouns needs the same circling as the text under review.

**Why:** a second-language reader, or a listener with no screen to look back at, resolves "that", "which" or "it" to the nearest thing, and that is often the wrong one. Tightening the length tempts you to bring the pronoun back.

**How to apply:** in the first pass, circle every "that", "which", "it" and "this". Name the noun wherever two candidates precede it, or wherever the next text read aloud could be taken for it. When a length brief pushes back, keep the noun and state the length. Related: [[one-text-read-in-each-place]], [[workflow-carried-2026-10-10]].

2026-10-25, #443 (j190): again my own fixes brought new pointing words ("After it", "To undo it"), each with two candidates (Undo, Remove photo). Both caught on my reread before the push. Check every "it" a fix introduces before committing, not only the ones under review.

2026-10-25 (#447, j197): splitting a parenthesis into its own sentence made a new "it" ("which encoder it used": the run or the encoder). A passive ("was used") removes the pointer when the agent does not matter.

*References: one-text-read-in-each-place, workflow-carried-2026-10-10*

*Anchors: `interweave:design/2026-10-22-add-a-peer.md`, `interweave:crates/human/ui-model/src/labels.rs`*

*Observed 2026-10-25 (language-culture)*
