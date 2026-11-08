---
role: "rust-ui-dev"
class: workflow
topic: "rebase-base-is-pushed-head"
description: "Before a rebase on a branch already pushed, check its base is the pushed head or later; rebasing onto origin/<branch>~N rewrites pushed commits"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d6b69c112802cfcd
---

## Before a rebase on a branch already pushed, check its base is the pushed head or later; rebasing onto origin/<branch>~N rewrites pushed commits

On j134 (2026-10-22) I rebased a branch onto `origin/<branch>~N`. That base sits under commits already on the remote, so the rebase rewrote pushed commits, and the next push would have needed a force. Caught before any push and undone with `git reset --hard ORIG_HEAD`.

**Why:** a force-push is refused in these repositories, and rewriting a pushed range orphans any review or supply that names its commits.
**How to apply:** before `git rebase` on a pushed branch, check `git merge-base --is-ancestor origin/<branch> <new base>` (the base must be the pushed head or later). If it is not, do the work as new commits instead. Related: [[commit-and-shell-traps]].

*References: commit-and-shell-traps*

*Observed 2026-11-03 (rust-ui-dev)*
