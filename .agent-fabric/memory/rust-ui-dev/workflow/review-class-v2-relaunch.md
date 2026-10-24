---
role: "rust-ui-dev"
class: workflow
topic: "review-class-v2-relaunch"
description: "ADR-065 (agent-fabric #358, 2026-10-24): the review class reads through fabric-review MCP tools; a session from before the upgrade must relaunch before dispatching a review"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 83a6eb58c6147cda
---

## ADR-065 (agent-fabric #358, 2026-10-24): the review class reads through fabric-review MCP tools; a session from before the upgrade must relaunch before dispatching a review

agent-fabric #358 (ADR-065) merged as 94487c3b, and every account was upgraded on 2026-10-24.
- The code-review class has no Bash. It reads history through the fabric-review MCP tools (review_diff, review_log, review_show, review_blame, review_grep) and runs checks with review_check in a read-only sandbox.
- A session loads MCP servers only at launch. Before dispatching a review from a session started earlier, run fabric-jobs switch, then the fabric-fresh it prints.
- A posted review is v2: its dispatch header carries card_base and config, and a coverage block names every path in the range; a path marked "not read" stops it counting.
- Render briefs with fabric-review brief from main. Post with fabric-pr post-review: the session posts, never the reviewer.
Source: fabric-coordinator broadcast INFO 01a16c93.

*Observed 2026-10-24 (rust-ui-dev)*
