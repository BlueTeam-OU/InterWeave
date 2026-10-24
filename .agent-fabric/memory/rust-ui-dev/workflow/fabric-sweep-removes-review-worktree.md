---
role: "rust-ui-dev"
class: workflow
topic: "fabric-sweep-removes-review-worktree"
description: "fabric-branches --sweep deletes a running review's pinned fabric-review/ worktree; don't sweep while a review dispatch is in flight"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 85cb637114b68b71
---

## fabric-branches --sweep deletes a running review's pinned fabric-review/ worktree; don't sweep while a review dispatch is in flight

On 2026-10-20 `fabric-branches --sweep`, run during the disk clean-up, removed
`~/agent-fabric-tmp/fabric-review/interweave-b964293c5866` while the round-1 review of #327
was still reading it. The sweep treated it as a pushed worktree. I recreated it at the same
commit with `git worktree add --detach <path> <sha>` within a minute, and the review's
report was unaffected. Reported to fabric-coordinator in REPLY 01a15af5-db14 (their tool).

**Why:** the review agent's brief names that path as the head it reads; without it the
review reads nothing or fails silently.

**How to apply:** before `fabric-branches --sweep`, check that no `code-review` dispatch
is running. If one was swept anyway, recreate the worktree at the head sha its brief names.
Related: [[fabric-review-brief-usage]].

*References: fabric-review-brief-usage*

*Observed 2026-10-20 (rust-ui-dev)*
