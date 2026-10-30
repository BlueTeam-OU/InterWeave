---
role: "architect-cto"
class: workflow
topic: "review-tools-bound-to-the-session-clone-2026-10-25"
description: "The review class's MCP tools (review_diff/log/show/check) are bound to the SESSION's project clone; a review of another project's PR dispatched from this session reads nothing through them and is not coverage — dispatch a gzapp review…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1c573f04ca5bc572
---

## The review class's MCP tools (review_diff/log/show/check) are bound to the SESSION's project clone; a review of another project's PR dispatched from this session reads nothing through them and is not coverage — dispatch a gzapp review only from a gzapp-bound session

2026-10-25 09:40Z: from this InterWeave-bound session I dispatched the
blind review of gzapp #1166. The reviewer's `review_diff`, `review_log`
and `review_check` refused the gzapp clone and the head worktree
("repository is not the session's clone") and found the base "not a
commit of this repository" in the InterWeave clone. It read the head
worktree's files by hand and reported two real P2s, but its coverage
block marks fourteen paths "not read", so `fabric-pr review-status`
does not count it.

**Why:** agent-fabric ADR-065 pins the reviewer's tools to the clone the
session was launched in; a cross-project dispatch cannot be evidence of
that project's range.
**How to apply:** a PR of project X is reviewed only from a session whose
working copy is X (`fabric-jobs switch` into an X job, then the
`fabric-fresh` it prints). Supplies onto another lane's branch need no
review of mine (the caller reviews); my OWN PRs in gzapp (#1166) get
their coverage review from a gzapp session. A partial review is still
posted as the record of its findings, with a dispatcher's note.

*Anchors: `agent-fabric:docs/adr/ADR-065-the-reviewer-reads-through-pinned-tools.md::## 1`*

*Observed 2026-10-25 (architect-cto)*
