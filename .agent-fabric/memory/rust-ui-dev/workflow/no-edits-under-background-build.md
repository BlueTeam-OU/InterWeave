---
role: "rust-ui-dev"
class: workflow
topic: "no-edits-under-background-build"
description: "a background cargo build/e2e reads the working tree as it compiles; editing meanwhile makes it test a half-written tree (j134, 2026-10-24)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 74c256534d57674c
---

## a background cargo build/e2e reads the working tree as it compiles; editing meanwhile makes it test a half-written tree (j134, 2026-10-24)

A background `cargo build` / desktop-e2e run started from a worktree compiles whatever is on disk when each crate is reached, not the commit it started at. On j134 (2026-10-24) the e2e run failed to compile because I kept editing ui-model while it built: the run proved nothing and cost its 10+ minutes.

**Why:** cargo has no snapshot of the tree; the run and the edits share one working copy and one target dir.

**How to apply:** either stop editing while a long run is in the background (write messages, PR body, memory meanwhile), or run it from a separate scratch worktree at the commit (`git worktree add <scratch> <sha>`, own target dir, and remove it after — measure disk first). Name the head in the run's log (`git rev-parse --short HEAD`) so the result says which tree it proved.

Related: [[view-change-runs-desktop-e2e]] [[shared-target-dir-stale-build]]

*References: shared-target-dir-stale-build, view-change-runs-desktop-e2e*

*Anchors: `interweave:tools/ci/with_display.sh`*

*Observed 2026-10-24 (rust-ui-dev)*
