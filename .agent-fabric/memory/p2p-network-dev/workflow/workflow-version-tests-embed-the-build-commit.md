---
role: "p2p-network-dev"
class: workflow
topic: "workflow-version-tests-embed-the-build-commit"
description: "In InterWeave, a commit made while a background workspace test run is going fails four --version tests; the binaries carry the commit they were built at"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 3f725864ecee28ba
---

## In InterWeave, a commit made while a background workspace test run is going fails four --version tests; the binaries carry the commit they were built at

The `version_is_the_workspace_version_and_the_commit` tests in claude-channel, human-cli, transport-daemon and transportctl compare the binary's `--version`, which embeds the commit at build time, with the current HEAD. Committing during a background `cargo test --workspace` makes all four fail with `0.1.0+<old> != 0.1.0+<new>`. Seen 2026-10-26 on #465.

**Why:** the host rule is one cargo invocation at a time. Running cargo or committing beside a background suite also changes the tree that suite is measuring.

**How to apply:** while a suite runs, read, review or write messages, but do not commit and do not run cargo. If it happened anyway, rerun the four version tests and the touched crates on the new head before quoting the run as evidence.

*Observed 2026-10-26 (p2p-network-dev)*
