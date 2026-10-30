---
role: "architect-cto"
class: workflow
topic: "disk-cleanup-2026-10-27"
description: "fleet disk cleanup 2026-10-27 (owner's ask via fabric-coordinator broadcast 01a17e45) — what this account removed and what is never removed (the installed InterWeave programs under ~/.local/bin serve the gzcoord units)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 90cf9df914a72a1b
---

## fleet disk cleanup 2026-10-27 (owner's ask via fabric-coordinator broadcast 01a17e45) — what this account removed and what is never removed (the installed InterWeave programs under ~/.local/bin serve the gzcoord units)

2026-10-27: /home at 97% (21 GB free) fleet-wide. My account 16 GB → 6.4 GB by removing, in my own account only: InterWeave `spikes/*/target` (ignored build output, `git clean -fdX -- spikes`, ~4.5 GB), `~/.cargo/registry/{src,cache}` (1.5 GB, re-extracted on build), `~/projects/gzapp/node_modules` (835 MB) plus the pnpm store and cache (1.9 GB), the NuGet cache (308 MB), unused podman images (1.2 GB, no containers or volumes). Kept: `~/.local/bin` (human-desktop, transport-daemon, ori: the installed InterWeave release the account's `interweave-daemon@gzcoord` and `interweave-client@gzcoord` units run), `~/.local/share/claude`, the two live worktrees (unmerged branches), ~/agent-fabric-tmp (small).

**How to apply:** measure with `du` first; delete only ignored build output and package caches; never the installed programs under ~/.local/bin, never a working copy with unpushed commits.

*Observed 2026-10-27 (architect-cto)*
