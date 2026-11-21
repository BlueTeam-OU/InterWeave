---
role: "p2p-network-dev"
class: workflow
topic: "review-range-starts-at-the-merge-base-after-a-fold"
description: "fabric-pr post-review refuses a review whose base is not the PR's merge-base; folding a supply that merged main moves it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3f097d6defcfa5cc
---

## fabric-pr post-review refuses a review whose base is not the PR's merge-base; folding a supply that merged main moves it

On #674 (2026-11-14) a review of 808488b43..head was refused by `fabric-pr post-review`: the folded for/ supply was based on a newer main (a5c4d1905), so the PR's merge-base moved there. Brief the review from `git merge-base origin/main HEAD` after every fold, not from the branch's original base. An adjudication of a round's findings is only possible at a later head (the re-review of the fix range judges them); `fabric-review brief` refuses finding-adjudication on the round's own range.
**How to apply:** fold all supplies first, then compute the merge-base, then brief.

*Observed 2026-11-15 (p2p-network-dev)*
