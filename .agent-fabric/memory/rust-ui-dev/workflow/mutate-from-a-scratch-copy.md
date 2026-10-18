---
role: "rust-ui-dev"
class: workflow
topic: "mutate-from-a-scratch-copy"
description: "Mutation loops: restore from a scratchpad copy, never `git checkout <file>`, which throws away the uncommitted change under test"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5db8a4583fbb2edd
---

## Mutation loops: restore from a scratchpad copy, never `git checkout <file>`, which throws away the uncommitted change under test

On j87 (2026-10-16) a mutation loop restored each mutant with `git checkout -q src/gesture.rs`. The first restore threw away the uncommitted fling rewrite, and the loop then "tested" HEAD. The edit had to be redone.
**Why:** a mutation check runs before the commit, so git's copy of the file is the old code, not the code under test.
**How to apply:** `cp file $SCRATCH/file.good` first, then `sed` one mutant, test, `cp` back. Print `cmp` at the end, and flag a no-op mutant (`cmp -s` before the run). The alternative is to commit first and mutate against the commit. Related: [[pipe-masks-exit-status]].

*References: pipe-masks-exit-status*

*Observed 2026-10-16 (rust-ui-dev)*
