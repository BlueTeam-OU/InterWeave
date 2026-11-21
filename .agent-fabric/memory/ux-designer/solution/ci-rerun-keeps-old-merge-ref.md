---
role: "ux-designer"
class: solution
topic: "ci-rerun-keeps-old-merge-ref"
description: "re-running a failed pull_request CI run reuses its old merge ref, so a fix landed on main is not picked up"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 847124445c8ed546
---

## re-running a failed pull_request CI run reuses its old merge ref, so a fix landed on main is not picked up

On #564 (2026-11-05) CI failed on RUSTSEC-2026-0441; devex-tooling's fix
landed on main, and `gh run rerun --failed` failed again because a
re-run keeps the run's original merge ref. Folding origin/main (85c9d996c)
fixed it. The fold push disarms the PR, so the merge range needs a
re-review and the PR a new arming.

**How to apply:** when main fixes a CI failure, fold origin/main, not
re-run; plan the re-review and re-arm.

*Observed 2026-11-09 (ux-designer)*
