---
role: "devex-tooling"
class: workflow
topic: "disk-sweep-traps"
description: "freeing disk on this login — pnpm prune no-op, fabric-branches has no --all, git merge in other copies blocked"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d2c98f2d5ce268fa
---

## freeing disk on this login — pnpm prune no-op, fabric-branches has no --all, git merge in other copies blocked

Measured during the owner's disk-and-branch sweep, j281 (2026-11-04): home went from 19G to 5.5G.
- `pnpm store prune` (pnpm 12.3.4) freed nothing while gzapp's per-package `node_modules` still hard-linked the store. Remove every `node_modules` under gzapp/apps, packages and tools first. After that the store (`~/.local/share/pnpm/store`) is only a cache and can be deleted.
- `fabric-branches` takes no `--all` flag, so run it in each working copy in turn. It does not move a copy off a branch that is checked out there (the current branch) or on main in a worktree. Remove that worktree first, then `git checkout main`.
- The job hook refuses `git merge` (a fast-forward included) in any working copy other than the job's own. A plain `git checkout` there is allowed.
- `podman system df` reports overlay-shared volume sizes, and its total is several times the real `du`: about 4G reported against 521M on disk.
- Kept on purpose: the three gzapp-devex-tooling DB volumes (postgres, keycloak_postgres, nominatim, which are slow to rebuild) and the ubuntu:26.04 image used for CI repros ([[uutils-date-3n-wrong]]).

*References: uutils-date-3n-wrong*

*Anchors: `agent-fabric:bin/fabric-branches::--sweep`*

*Observed 2026-11-04 (devex-tooling)*
