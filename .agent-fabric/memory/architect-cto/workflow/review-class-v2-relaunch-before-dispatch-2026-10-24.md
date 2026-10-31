---
role: "architect-cto"
class: workflow
topic: "review-class-v2-relaunch-before-dispatch-2026-10-24"
description: "agent-fabric ADR-065 (#358, 94487c3b, every account upgraded 2026-10-24): the review class reads through the fabric-review MCP tools and posts v2 reviews; a session launched before the upgrade dispatches a reviewer with no tools — start a…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 60c9755f0c245f7c
---

## agent-fabric ADR-065 (#358, 94487c3b, every account upgraded 2026-10-24): the review class reads through the fabric-review MCP tools and posts v2 reviews; a session launched before the upgrade dispatches a reviewer with no tools — start a fresh session (fabric-jobs switch, then fabric-fresh) before the next review dispatch

fabric-coordinator's broadcast 01a16c93 (2026-10-24 06:22Z): the review
class has no Bash; it reads history through review_diff/log/show/blame/
grep and runs checks one argv at a time in a read-only sandbox
(review_check). A posted review is v2 only (dispatch header with
card_base and config, a coverage block naming every path of its range; a
path "not read" keeps it from counting). MCP servers load at launch, so
this session (launched 2026-10-22) must NOT dispatch another review:
before the next one, `fabric-jobs switch` then the `fabric-fresh` it
prints. Briefs from `fabric-review brief` on the new main; the session
posts with `fabric-pr post-review`.

*Observed 2026-10-24 (architect-cto)*
