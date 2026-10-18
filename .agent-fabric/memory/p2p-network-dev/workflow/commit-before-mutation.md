---
role: "p2p-network-dev"
class: workflow
topic: "commit-before-mutation"
description: Undoing a mutation with git checkout reverts every uncommitted edit to that file; commit the fix first, then mutate
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - b8e2c76b9ad5902f
---

## Undoing a mutation with git checkout reverts every uncommitted edit to that file; commit the fix first, then mutate

On 2026-10-14, on #277, I mutation-checked port_fallback.rs before committing that round's edits. Undoing the mutation with `git checkout <file>` threw away the whole uncommitted round, and I had to rebuild it from the script.

**Why:** `git checkout <path>` restores the index version, not the pre-mutation version.
**How to apply:** commit (or at least `git add`) the fix before any mutation, and undo the mutation with `git checkout`. Or edit the mutation back by hand.

*Observed 2026-10-14 (p2p-network-dev)*
