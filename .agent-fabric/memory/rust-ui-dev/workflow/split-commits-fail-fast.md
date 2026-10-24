---
role: "rust-ui-dev"
class: workflow
topic: "split-commits-fail-fast"
description: "When splitting a finished change into per-root-cause commits by script, save the final files, build every stage first, and run the commit chain under set -e"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 69f094dc093f3e4d
---

## When splitting a finished change into per-root-cause commits by script, save the final files, build every stage first, and run the commit chain under set -e

On #384 (2026-10-23) a Python split script failed on an ambiguous match, but the shell chain that committed its stages ran on regardless. Two commits went in with the wrong contents under the wrong messages. They were unpushed, so `git reset --soft` to the last good commit and a redo fixed it.

**Why:** CLAUDE.md's rule is one commit per root cause. When several fixes share a file and its test file, the split has to be scripted, and a failed stage must stop everything after it.

**How to apply:**
- Copy the finished files to the scratchpad first.
- Build every stage file (each removal anchored on unique context) and check they all exist before any commit.
- Run the commit chain under `set -e`, with each stage's tests run before its commit.
- Diff the end state against the saved files.

Related: [[mutate-from-a-scratch-copy]]

**Never stage a subset of hunks with `git apply --cached --unidiff-zero`.** This happened on #384 too, the same day. Every zero-context hunk carries the line numbers of the original file, so leaving out an earlier hunk shifts where a later one lands. The patch still applies, but lines go in the wrong place: an assignment ended up inside the wrong `if`. Use an applier that places each chosen hunk by its original line plus the offset of the hunks applied before it, and asserts that the lines it removes are the ones there. Build each stage cumulatively from the base and stage it with `git hash-object -w` and `git update-index --cacheinfo`. Then check that applying every hunk reproduces the working tree, and run each commit's tests in a scratch worktree.

*References: mutate-from-a-scratch-copy*

*Observed 2026-10-23 (rust-ui-dev)*
