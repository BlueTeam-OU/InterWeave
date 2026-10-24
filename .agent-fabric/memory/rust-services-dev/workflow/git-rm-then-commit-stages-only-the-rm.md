---
role: "rust-services-dev"
class: workflow
topic: "git-rm-then-commit-stages-only-the-rm"
description: "After `git rm` plus file edits, a bare `git commit` commits only the deletion; a build passing on the tree proves nothing about the commit"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 471ce64ae62d55bd
---

## After `git rm` plus file edits, a bare `git commit` commits only the deletion; a build passing on the tree proves nothing about the commit

`git rm` stages the deletion, and edits made beside it stay unstaged, so a bare `git commit` publishes a head that does not build. Cargo built from the working tree and the test passed against edits that were never committed. This happened on InterWeave #401 (e2b1e05a deleted apps/claude-channel/build.rs without the manifest's `build =` line; fixed in the follow-up 8c5e1bb6).

**Why:** a verification run on the working tree tells you nothing about what was committed, and history that has been pushed is not rewritten.

**How to apply:** before every commit, run `git status --short` and check that nothing you meant to commit is still ` M`. Run the commit's tests after the commit with the tree clean, not before it. Related: [[mutation-runs-save-the-fix-first]].

*References: mutation-runs-save-the-fix-first*

*Anchors: `apps/claude-channel/Cargo.toml::commit_stamp.rs`*

*Observed 2026-10-23 (rust-services-dev)*
