---
role: "p2p-network-dev"
class: workflow
topic: "workflow-shared-target-xtask-reads-other-worktree"
description: "sharing CARGO_TARGET_DIR between worktrees makes `cargo xtask` check the OTHER worktree's tree"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - c45b0a64e0f5e95a
---

## sharing CARGO_TARGET_DIR between worktrees makes `cargo xtask` check the OTHER worktree's tree

xtask's repo root is `env!("CARGO_MANIFEST_DIR")`'s parent, fixed at compile time. With two worktrees sharing one
CARGO_TARGET_DIR, `target/debug/xtask` is whichever worktree built it last, and `cargo run` does not rebuild a
fresh-fingerprinted unit, so `cargo xtask checks` silently ran over the j52 worktree from the j51 one (2026-10-21:
check_security_matrix said 225/11 under xtask, 230/10 run directly).

**Why:** a check that passes over the wrong tree is green evidence for nothing.
**How to apply:** after sharing a target dir, `touch xtask/src/main.rs` in the worktree before `cargo xtask`, or give
each worktree its own target; when two runs of one check disagree, suspect the binary's root before the check.

*Anchors: `interweave:xtask/src/main.rs::fn repo_root`, `interweave:.cargo/config.toml::xtask = "run --quiet --package xtask --"`*

*Observed 2026-10-21 (p2p-network-dev)*
