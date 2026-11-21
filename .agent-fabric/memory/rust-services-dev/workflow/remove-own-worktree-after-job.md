---
role: "rust-services-dev"
class: workflow
topic: "remove-own-worktree-after-job"
description: "Removing the job's own worktree at the end of a session stops the inbox watch, and the job hook then refuses git merge in the main clone"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 3b15bde15068b072
---

## Removing the job's own worktree at the end of a session stops the inbox watch, and the job hook then refuses git merge in the main clone

When a job is done and its PR is merged, the session still stands in that job's
worktree. If the owner asks to remove it (2026-11-12, j88, `git -C <main clone>
worktree remove <own worktree>`):

- The running `gzcoord-inbox --until-delivery` watch, started from that
  directory, exits 1 at its next poll ("pwd: error retrieving current
  directory"), delivering nothing. Re-arm it with `cd <main clone> &&
  gzcoord-inbox --until-delivery`.
- Deleting the branch from the main clone warns "not yet merged to HEAD" when
  the main clone's local `main` is stale. Check `git merge-base --is-ancestor
  <head> origin/main` instead of trusting the warning, then `git fetch --prune`
  to drop the stale tracking ref.
- Fast-forwarding the main clone's `main` (`git merge --ff-only origin/main`)
  is refused by the fabric job hook once no job is active (agent-fabric ADR-037
  rule 3). Do not route around it: offer the owner `! git -C <clone> merge
  --ff-only origin/main`, or queue it as a job.

**Why:** the session's cwd and the watch's cwd are the same directory, and the
job hook classifies any git merge as committing.
**How to apply:** before removing your own worktree, plan to re-arm the watch
from the main clone at once. Hand the owner the fast-forward command rather
than trying another git verb.

*Anchors: `agent-fabric:runtime/claude-code/hooks/job-copy-guard.py::commits or publishes`*

*Observed 2026-11-12 (rust-services-dev)*
