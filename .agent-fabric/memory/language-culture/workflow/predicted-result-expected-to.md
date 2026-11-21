---
role: "language-culture"
class: workflow
topic: "predicted-result-expected-to"
description: "test prose that states a result read from code but not run: \"is expected to\", never a bare \"would\" or \"fails\""
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 27b9eebb18cded52
---

## test prose that states a result read from code but not run: "is expected to", never a bare "would" or "fails"

On #661 (j312, 2026-11-13), item 12 said that two cases "fail on the build of #658", while §5.1 said that build had been read in the code and not run. The author's rewording, "As read in the code, not yet run, ... are expected to fail ...; Connect would open ...", attached "read" to the cases, and "would" read as a condition whose "if" was missing. The text I supplied (REPLY 01a1d606): "The first and third cases have not yet been run on the build of #658. From a reading of that build's code (§5.1), both are expected to fail on it; in the third case, Connect is expected to open ...".

**Why:** a tester who reads English as a second language takes a bare "fail" as an observed result, and looks for the "if" that "would" implies.

**How to apply:** where a spec states a result predicted from code, say in a sentence of its own that the case has not been run, then write "is expected to". Name the case in full ("the third case"), so that a figure next to it cannot attach to another case. Related: [[observed-apart-from-expected]].

*References: observed-apart-from-expected*

*Anchors: `design/2026-10-22-add-a-peer.md::expected`*

*Observed 2026-11-13 (language-culture)*
