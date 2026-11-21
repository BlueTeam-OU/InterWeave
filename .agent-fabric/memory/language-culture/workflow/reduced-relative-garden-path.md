---
role: "language-culture"
class: workflow
topic: "reduced-relative-garden-path"
description: "\"the thread step 2 names\" reads as a noun \"thread step 2\"; add \"that\" when a numbered name follows a noun"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8c3df87e8f5b9010
---

## "the thread step 2 names" reads as a noun "thread step 2"; add "that" when a numbered name follows a noun

A relative clause with no "that", where the clause starts with a numbered name, makes one long noun: "in the thread step 2 names", "in the run step 4 names" (#659, 4e0d7f2aa, 2026-11-13). The reader takes "thread step 2" as one noun and then finds a verb with no subject. Add "that": "the thread that step 2 names". In the same steps, "the same is expected of both" also needed a change, because "expected of" can mean "demanded of". It became "both are expected to end the process as `pm revoke` did".

**Why:** a second-language reader parses noun + label + number as one compound noun.
**How to apply:** in spec prose, look for a noun followed straight away by "step N", "§N" or another name, then a verb; add "that". Related: [[mode-name-mid-sentence]], [[name-the-referent]].

*References: mode-name-mid-sentence, name-the-referent*

*Anchors: `architecture/docs/architecture/human-client-android.md::the run that step 4 names`*

*Observed 2026-11-13 (language-culture)*
