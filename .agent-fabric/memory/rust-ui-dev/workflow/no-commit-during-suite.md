---
role: "rust-ui-dev"
class: workflow
topic: "no-commit-during-suite"
description: "Never commit while the workspace suite runs; human-cli's version test pins HEAD, and cargo test stops at the first failing binary without --no-fail-fast"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2f9f9a00f06fbfff
---

## Never commit while the workspace suite runs; human-cli's version test pins HEAD, and cargo test stops at the first failing binary without --no-fail-fast

`apps/human-cli/tests/version.rs` compares `interweave-human --version` with the
current `git rev-parse` HEAD. A commit made while `cargo test --workspace` runs
(2026-10-25, #453) made the built binary's commit differ from HEAD, so the test
failed. And without `--no-fail-fast`, cargo then stopped: no later crate ran,
and the run looked like a suite with one failure.

**Why:** a failed suite read that way hides how much was never tested, and the
failure was not about the change.

**How to apply:** commit first, then run
`tools/ci/with_display.sh cargo test -q --workspace --no-fail-fast`. Make no
commits in that worktree until it ends. Also, `pkill -f` on a wait loop's
pattern killed both loops ([[pgrep-f-matches-itself]]): stop a background task
by its task id instead.

*References: pgrep-f-matches-itself*

*Anchors: `apps/human-cli/tests/version.rs::version_is_the_workspace_version_and_the_commit`*

*Observed 2026-10-25 (rust-ui-dev)*
