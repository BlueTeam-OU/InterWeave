---
role: "rust-ui-dev"
class: workflow
topic: "review-rounds-dont-chase-unreachable"
description: "in a PR's review rounds, a risk on a path no view reaches is carried to the batch that reaches it, not built into the fix range"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a164317256312155
---

## in a PR's review rounds, a risk on a path no view reaches is carried to the batch that reaches it, not built into the fix range

A reviewer's "risk (unproven)" about a path no person can reach yet (e.g. Undo of a card Cancel when no view offers Cancel on a card row) is carried by name to the batch that makes it reachable, in the PR body's limits -- not answered with new machinery in a fix commit.

**Why:** #467 (2026-10-27): answering round 4's unreachable Undo risk with a cancelled-card record produced three new findings in round 5 (F13/F15/F16) on that record alone; it was removed again. Each fix round re-reviewed costs ~3 min and a full suite.

**How to apply:** before fixing a risk, check reachability from the view; if unreachable, state it as a carried limit with its owning batch and move on. Related: [[j54-progress]].

*References: j54-progress*

*Observed 2026-10-27 (rust-ui-dev)*
