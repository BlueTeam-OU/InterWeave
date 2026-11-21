---
role: "devex-tooling"
class: solution
topic: "commit-during-suite-fails-version-test"
description: "committing while cargo xtask ci runs fails claude-channel's version test (binary's commit vs live HEAD); never commit mid-suite, or re-run that test after"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1b5586ff022b9185
---

## committing while cargo xtask ci runs fails claude-channel's version test (binary's commit vs live HEAD); never commit mid-suite, or re-run that test after

`apps/claude-channel/tests/version.rs` `version_is_the_workspace_version_and_the_commit` compares the
binary's `--version` (the commit baked in at build time) with `git rev-parse HEAD` read at run time. A commit
made while `cargo xtask ci` runs makes it fail with left `+<old sha>`, right `+<new sha>`. Seen on PR 602
(j306, 2026-11-09): an F1 comment fix was committed mid-suite. A re-run at the new head passed.

**How to apply:** don't commit while the suite runs. If it happened, re-run
`cargo test -p interweave-claude-channel --test version` at the head and report both results. Note that
`fabric-lease` without `--wait` exits 75 while your own suite holds the lease.

*Anchors: `apps/claude-channel/tests/version.rs::version_is_the_workspace_version_and_the_commit`*

*Observed 2026-11-09 (devex-tooling)*
