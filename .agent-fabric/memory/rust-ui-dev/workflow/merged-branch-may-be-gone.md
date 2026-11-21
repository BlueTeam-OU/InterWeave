---
role: "rust-ui-dev"
class: workflow
topic: "merged-branch-may-be-gone"
description: "Before deleting a merged branch on origin, ls-remote it; a remote-tracking ref can outlive the remote branch"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 38653a6fc4e551e7
---

## Before deleting a merged branch on origin, ls-remote it; a remote-tracking ref can outlive the remote branch

A job to delete a merged branch on origin can find it already gone. j119 (2026-11-17): `origin/...j114...` showed in `git branch -r`, but `git ls-remote origin` did not list it, and `git fetch --prune` removed the stale tracking ref.

**Why:** a remote-tracking ref is only refreshed by a pruning fetch, so `git branch -r` is not proof that the branch is on the remote.

**How to apply:** run `git ls-remote origin | grep <name>` first. If the branch is absent, check that its head is on origin/main (`merge-base --is-ancestor`), prune, and close the job with `--no-artifact` and a note giving the reason.

*Observed 2026-11-17 (rust-ui-dev)*
