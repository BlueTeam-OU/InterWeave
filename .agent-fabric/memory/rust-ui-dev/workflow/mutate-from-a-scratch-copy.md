---
role: "rust-ui-dev"
class: workflow
topic: "mutate-from-a-scratch-copy"
description: "Mutation loops: restore from a scratchpad copy, never `git checkout <file>`, which throws away the uncommitted change under test"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 431c985cc375b029
  - 5db8a4583fbb2edd
---

## Mutation loops: restore from a scratchpad copy, never `git checkout <file>`, which throws away the uncommitted change under test

On j87 (2026-10-16) a mutation loop restored each mutant with `git checkout -q src/gesture.rs`. The first restore threw away the uncommitted fling rewrite, and the loop then "tested" HEAD. The edit had to be redone.
**Why:** a mutation check runs before the commit, so git's copy of the file is the old code, not the code under test.
**How to apply:** `cp file $SCRATCH/file.good` first, then `sed` one mutant, test, `cp` back. Print `cmp` at the end, and flag a no-op mutant (`cmp -s` before the run). The alternative is to commit first and mutate against the commit. Related: [[pipe-masks-exit-status]].

**Name the copies by full path, never by basename.** On j134 (2026-10-23) a loop saved `ui-model/src/peers.rs` and `ui-slint/src/peers.rs` as `f3_peers.rs`: the second copy overwrote the first, and the restore wrote the view's code into the model's file. It was recoverable only because the model file had no uncommitted change. Key each copy on `$(echo $f | tr / _)`, and after the loop `git diff --stat` should show only the intended files.

*References: pipe-masks-exit-status*

*Observed 2026-10-23 (rust-ui-dev)*
