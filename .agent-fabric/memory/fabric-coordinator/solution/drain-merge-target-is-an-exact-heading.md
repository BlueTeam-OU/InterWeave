---
role: "fabric-coordinator"
class: solution
topic: "drain-merge-target-is-an-exact-heading"
description: "a merge_target that paraphrases or names a topic slug matches nothing and is filed as a new topic, silently except for merge_target_unresolved; counts.reflection needs origin exactly \"reflection\""
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 38e470ee7ba4fa1d
---

## a merge_target that paraphrases or names a topic slug matches nothing and is filed as a new topic, silently except for merge_target_unresolved; counts.reflection needs origin exactly "reflection"

In the 2026-10-24 InterWeave drain, 28 of architect-cto-01's W43 reflection
memories carried a `merge_target` that named a topic's words ("an error label
names whose state caused it") or a heading's opening words, not a section heading
in the slice. The assembler matched none of them and wrote each one as a topic of
its own; the only trace is `merge_target_unresolved` in `last-drain-report.json`.
A correction aimed that way stands beside the text it meant to replace. Here it did
not matter, because none of the named sections existed in the corpus.
Separately, the harvester counts a reflection claim only when `origin` is exactly
`reflection` (`tools/fabric/harvest_memory.py`, line 578). architect-cto-01 wrote
"reflection 2026-W43", so the row showed 0 from reflection against 74.

**How to apply:** when a drain is asked for a batch of corrections, read
`merge_target_unresolved` and tell the author which ones fell through, and why.

*Anchors: `agent-fabric:tools/fabric/harvest_memory.py::merge_target`, `agent-fabric:tools/fabric/harvest_memory.py::reflection += 1`*

*Observed 2026-10-24 (fabric-coordinator)*
