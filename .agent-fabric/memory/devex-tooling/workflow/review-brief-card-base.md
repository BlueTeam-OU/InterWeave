---
role: "devex-tooling"
class: workflow
topic: "review-brief-card-base"
description: "fabric-review brief refuses a range whose base is not on origin/main (a second change on an open PR); add card_base"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - fc81fbbab18d0e13
---

## fabric-review brief refuses a range whose base is not on origin/main (a second change on an open PR); add card_base

`fabric-review brief <req.yaml>` (no `render` subcommand on the wrapper) refuses `range: <old head>..<new head>` when the old head is an unmerged PR commit: the project review card is read from the base, which must be on the default branch. Add `card_base: <full sha on origin/main>` to the request. Seen 2026-10-19 adding the Android lockfile guard as a second change on #307 (range dc13d37e..6bc56758).

**Why:** a later piece of work on an open PR is reviewed as its own range, and that base is never on main.
**How to apply:** for any review of a range past the PR's first round, put `card_base: $(git rev-parse origin/main)` in the request up front.

*Anchors: `agent-fabric/tools/fabric/review_brief.py::card_base`*

*Observed 2026-10-19 (devex-tooling)*
