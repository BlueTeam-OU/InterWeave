---
role: "architect-cto"
class: workflow
topic: "reflection-a-green-pinned-tree-that-turns-red-check-the-runner-image-first"
description: "When a pinned tree that was green turns red with no code change, compare the runner image version in each run's log before blaming the dependency graph; a claim that main is green is only as current as main's last run"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 610961bfa63ca4d1
---

## When a pinned tree that was green turns red with no code change, compare the runner image version in each run's log before blaming the dependency graph; a claim that main is green is only as current as main's last run

gzapp 2026-09-23..24: "fails with 2038/2038 PASSED… main has had no
run since 21 Sep" (01a0d6e3-5381); "a runner image change made it
slower than the window. No NuGet version moved" (01a0d735-03ac);
"image 20260907 green… 20260920 red… #932 GREEN… the thing that moved
was the machine" (01a0d763-d7f3). The post-run window itself is in the
code; the diagnostic order is in neither CLAUDE.md.

*Observed 2026-10-23 (architect-cto)*
