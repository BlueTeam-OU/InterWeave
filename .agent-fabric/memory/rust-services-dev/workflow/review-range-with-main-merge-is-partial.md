---
role: "rust-services-dev"
class: workflow
topic: "review-range-with-main-merge-is-partial"
description: "A re-review whose range contains a merge of origin/main counts as partial coverage; review merge-base..head instead"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 0c7824fb1e4a258d
---

## A re-review whose range contains a merge of origin/main counts as partial coverage; review merge-base..head instead

In InterWeave, `fabric-pr review-status` counts a review as coverage only when its coverage block marks every path of the range "read". A re-review range that contains a merge of origin/main (e.g. `beb37ff0..e10d264b` on PR #420, 2026-10-24) lists main's files too; the reviewer marks them "not read" (out of scope), and the review is filed as `partial coverage: N path(s) not read`, so `fabric-pr arm` refuses with "no review of the current head".

**Why:** the gate requires full coverage of the head; files merged in from main are in the diff of a range whose base predates the merge.

**How to apply:** after merging origin/main into a PR branch, brief the next review as `mode: review` with range `$(git merge-base HEAD origin/main)..HEAD`: that diff holds only the PR's own paths. Later fix ranges with no merge inside can use `fabric-review next ... --pr N --head <sha>` as usual. See [[workflow-carried-2026-10-24]] for the other PR-shape rule.

*References: workflow-carried-2026-10-24*

*Anchors: `agent-fabric:tools/fabric/review_brief.py::card_base`*

*Observed 2026-10-24 (rust-services-dev)*
