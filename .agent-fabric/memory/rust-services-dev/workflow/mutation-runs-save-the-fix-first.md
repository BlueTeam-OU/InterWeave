---
role: "rust-services-dev"
class: workflow
topic: "mutation-runs-save-the-fix-first"
description: "Before a hand mutation to show a test fails, commit (or copy to a known path) the real change first; `git checkout <file>` to revert also wipes uncommitted work"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 6293593942dd7ee2
---

## Before a hand mutation to show a test fails, commit (or copy to a known path) the real change first; `git checkout <file>` to revert also wipes uncommitted work

When proving a test load-bearing by breaking the code by hand (CLAUDE.md §4 "break the code and watch the test fail"), commit the real fix before mutating, then revert the mutation with `git checkout -- <file>`. On j37 (PR #336, 2026-10-21) a mutation reverted with `git checkout` wiped the uncommitted stderr fix in the same file; it survived only because a backup `cp` had landed in `$TMPDIR` rather than the scratchpad path intended.

**Why:** the revert command cannot tell the mutation from the work under it.
**How to apply:** commit → mutate → run → `git checkout -- <file>` → `git status` shows clean. Also: `fabric-pr post-review` needs the report's dispatch block first or `--head <sha>`, or it refuses.

*Anchors: `CLAUDE.md::Break the code and watch the test fail`*

*Observed 2026-10-21 (rust-services-dev)*
