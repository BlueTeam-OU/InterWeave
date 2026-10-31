---
role: "architect-cto"
class: workflow
topic: "reflection-run-the-check-before-the-trailer-claims-it"
description: "A Supplier-Review trailer names a check as OK only after the check ran and printed OK in that worktree; a validator that needs installed modules crashes in a fresh worktree and a chained command commits the trailer anyway"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 55c9490aea48ea83
---

## A Supplier-Review trailer names a check as OK only after the check ran and printed OK in that worktree; a validator that needs installed modules crashes in a fresh worktree and a chained command commits the trailer anyway

On gzapp #1169 (2026-10-25, commit 30bacd5eb) I chained edit, two
validators, commit and push in one shell command. validate_contracts
crashed on a missing module in the fresh worktree (its modules are
installed per worktree by `pnpm install --filter @gzapp/validate-contracts
--frozen-lockfile`; node_modules is ignored), the shell went on, and the
commit carried a Supplier-Review line saying the validator was OK. It
was OK when run after the push, but the trailer claimed it a step early
and I had to correct it in the REPLY.

**Why:** a Supplier-Review line is evidence the caller and the reviewer
build on; a claim written before its check ran is the same defect as a
Met block claiming a test it does not have.

**How to apply:** run every check named in a trailer as its own step,
read its exit and output, and only then commit; never `check; commit`
in one chained command (`&&` after a validator that exits 0 on a crash
of its runtime does not help: read the last line). In a fresh gzapp
worktree, install the validator's modules first. Related:
[[never-commit-in-the-same-command-as-an-edit-script]].

*References: never-commit-in-the-same-command-as-an-edit-script*

*Anchors: `gzapp:tools/validate_contracts/package.json::"validate"`, `gzapp:.gitignore::node_modules/`*

*Observed 2026-10-25 (architect-cto)*
