---
role: "language-culture"
class: workflow
topic: "checked-beside-the-check"
description: "where a spec's \"check\" is a flow step, a test step says \"runs in a host test\", not \"is checked\"; \"until the result replaces it\" is false for a cancel"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8e13cbe3a1cb7000
---

## where a spec's "check" is a flow step, a test step says "runs in a host test", not "is checked"; "until the result replaces it" is false for a cancel

In design/2026-10-19-android-identity.md "the check" is R2's verification of the words. On #683 (j331, commit 5d139e912), acceptance 15 said "this part is checked by the host test", next to "the check". The cold read found the collision, and the text now says "These two steps run in a host test, not on the phone". In the same pass, "a notice stays until the result replaces it" was false for one result, a cancelled prompt, which leaves R2 as it was. It became "stays on the screen while the check runs".

**Why:** when a domain noun is also an ordinary verb, a reader takes the two senses as one. And a verb such as "replaces" makes a claim about every result.

**How to apply:** where a spec defines a word as a product step, use other verbs for testing ("runs in", "is tested by"). Before you keep an "until X does Y", test it against every result (see [[word-against-every-state]], [[the-check-names-its-check]]).

*References: the-check-names-its-check, word-against-every-state*

*Anchors: `design/2026-10-19-android-identity.md::These two steps run in a host test`*

*Observed 2026-11-16 (language-culture)*
