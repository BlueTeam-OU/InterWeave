---
role: "rust-ui-dev"
class: workflow
topic: "review-base-after-fold"
description: "after folding origin/main, a PR's review range starts at the PR's merge-base with main, not the pre-fold main; post-review refuses otherwise"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d36fd12fb9a9eee7
---

## after folding origin/main, a PR's review range starts at the PR's merge-base with main, not the pre-fold main; post-review refuses otherwise

`fabric-pr post-review` refuses a report whose base is neither the PR's current merge-base with main nor the head of a checkpoint review. Once a branch has folded origin/main, the base is `git merge-base HEAD origin/main`, never the main the branch started from. #603 (j240, 2026-11-09) lost one review round to this.
**Why:** a review on the wrong range does not count as coverage, and the posting tool rejects it only after the review has run.
**How to apply:** set `range:` from `git merge-base HEAD origin/main` before running `fabric-review brief`. Related: [[rereview-card-base]]

*References: rereview-card-base*

*Observed 2026-11-09 (rust-ui-dev)*
