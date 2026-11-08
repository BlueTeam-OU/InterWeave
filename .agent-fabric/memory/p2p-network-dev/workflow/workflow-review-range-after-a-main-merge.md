---
role: "p2p-network-dev"
class: workflow
topic: "workflow-review-range-after-a-main-merge"
description: "After merging origin/main into a PR branch, brief the review on merge-base..head (a full review), never prev-head..head"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - c0a0e447e872458a
---

## After merging origin/main into a PR branch, brief the review on merge-base..head (a full review), never prev-head..head

After `git merge origin/main` into a PR branch, a review of `<last reviewed head>..<new head>` lists every file the merge brought in; marking them "not read, out of scope" makes `fabric-pr review-status` count it as PARTIAL (not coverage). A narrower base (the merge commit) is refused by `fabric-pr post-review` (base must be the merge-base with main or the last checkpoint head). The working shape: a full review, `range: <merge-base with main>..<head>` with `card_base` the same sha — since main is merged in, that diff is only the PR's own files. Seen on #459, 2026-10-26 (two dispatches wasted).

**Why:** coverage is checked against `git diff --name-only base..head`; merge content inflates it.
**How to apply:** after a main merge, compute `git merge-base origin/main HEAD` and review from there.

*Anchors: `agent-fabric:tools/fabric/github/review_coverage.py::diff_paths`, `agent-fabric:tools/fabric/github/post_review.py::merge-base`*

*Observed 2026-10-26 (p2p-network-dev)*
