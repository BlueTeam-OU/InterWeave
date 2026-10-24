---
role: "p2p-network-dev"
class: workflow
topic: "workflow-cite-the-contracts-classes-not-their-codes"
description: "a comment that restates a contract's enumerated list (error codes per class) gets a finding every review round; name the contract's classes and restate no codes"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 2833e7f10d8427df
---

## a comment that restates a contract's enumerated list (error codes per class) gets a finding every review round; name the contract's classes and restate no codes

A doc comment that sorts error codes by a contract's classes, with examples, drew a new P3 in three re-review rounds in a row on InterWeave #322 (2026-10-20, `close_exchange` in tests/android-e2e-cases):
- first it overclaimed;
- then an example sat in the wrong class (`Overloaded` is dispatched-and-refused, not pre-dispatch);
- then both "complete-looking" lists missed codes (`ShuttingDown`, `VersionIncompatible`).

The fix that ended it named only the three class headings of TRANSPORT.md's error model and restated no codes (bae27538).

**Why:** a restated list is a second copy of a normative constant. Every edit to it is checked against the contract, and every copy drifts.

**How to apply:** when prose must say how to read a contract's enum, cite the section and its class names. Never list members, even "e.g.". On the second round of findings about the same sentence, replace the sentence with a reference; do not patch it again. Related: [[workflow-to-role-finding-goes-to-the-holder-in-the-crate]].

*References: workflow-to-role-finding-goes-to-the-holder-in-the-crate*

*Anchors: `tests/android-e2e-cases/src/lib.rs::close_exchange`, `architecture/contracts/TRANSPORT.md::Outcome unknown`*

*Observed 2026-10-20 (p2p-network-dev)*
