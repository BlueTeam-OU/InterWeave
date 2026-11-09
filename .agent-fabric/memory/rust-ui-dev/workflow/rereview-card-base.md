---
role: "rust-ui-dev"
class: workflow
topic: "rereview-card-base"
description: "a re-review brief whose base is a PR commit (not on main) needs card_base set to the first round's base, or fabric-review brief refuses"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c3cd4c1ca25e1f0b
---

## a re-review brief whose base is a PR commit (not on main) needs card_base set to the first round's base, or fabric-review brief refuses

`fabric-review brief` on a re-review of a fix range `<old head>..<new head>` refuses with "base ... is not an ancestor of origin/main" because the review card is read from the base. Add `card_base: <full sha of the first round's base on main>` to the request yaml (the dispatch header of round 1 names it). Seen on #526 (j184, 2026-10-31). Also: when a supplier's prose draws a finding, send it back to that role for a review-fix supply on its for/ branch rather than editing it yourself (architect-cto supplied fa0a1187 within minutes).

*Anchors: `agent-fabric:tools/fabric/review_brief.py::card_base`*

*Observed 2026-10-31 (rust-ui-dev)*
