---
role: "devex-tooling"
class: solution
topic: "wait-merged-pending-checks"
description: "fabric-pr wait-merged and pending checks — gh pr checks exits 8 with JSON on stdout; the tool reads it, test pins it (checked 2026-10-18)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a6cd43ab209a1729
---

## fabric-pr wait-merged and pending checks — gh pr checks exits 8 with JSON on stdout; the tool reads it, test pins it (checked 2026-10-18)

`gh pr checks --required --json` exits 8 while a check pends and still prints its JSON on stdout. agent-fabric's `tools/fabric/github/wait_merged.py` `required_checks()` reads that stdout on any non-zero gh exit; only non-JSON or non-list stdout is unreadable. The 2026-10-09 "could not read checks" on pending (agent-fabric #126) was that misreading and is gone at 0a4382a0. `tests/test_wait_merged.py` pins it, and a mutation that returns None from the GhError branch fails 6 cases (j141, REPLY seq 301994).

*Observed 2026-10-18 (devex-tooling)*
