---
role: "rust-services-dev"
class: workflow
topic: "disk-sweep-what-holds-the-bytes"
description: "Where this account's home disk goes and what is safe to remove in a disk sweep"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 125cc1e1f75358e4
---

## Where this account's home disk goes and what is safe to remove in a disk sweep

2026-11-04 sweep (job j71, the owner's ask): home 6.3G -> 3.3G. The bytes were
`~/.cargo/registry/src` (1.3G, regenerated from `registry/cache` on next build),
`~/.cargo/git/checkouts` (163M, regenerated from `git/db`), an unpinned rustup
toolchain (1.98.1, 602M; only 1.99.1 for InterWeave and 1.97.1 for
agent-fabric-gateway's rust-toolchain.toml are pinned), old Claude Code
versions under `~/.local/share/claude/versions`, and spike `target/` dirs inside a done
job's worktree (spike-004 phase-b node 424M, spike-009 harness 162M).
`fabric-branches --sweep` keeps a worktree holding ignored build output, so such
a worktree is removed with `git worktree remove --force` after checking
`git status --short --ignored` shows only build output.

Wrong call in that sweep: a Claude Code build that `~/.local/bin/claude` no longer
links to is NOT unused when SPIKE-001 measures on it. spikes/spike-001/README.md
names 2.1.286 the fleet's pin, and the next request (j72, S29) asked for runs on
it the same day. Keep every build a spike record names; when one is gone,
install it into a scratch HOME (`HOME=<dir> claude install <version>`) as S28
did for 2.1.317, so the login's own launcher stays where it is.

*Anchors: `rust-toolchain.toml::channel = "1.99.1"`*

*Observed 2026-11-04 (rust-services-dev)*
