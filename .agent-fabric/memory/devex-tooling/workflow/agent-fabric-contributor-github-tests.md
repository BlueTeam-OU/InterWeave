---
role: "devex-tooling"
class: workflow
topic: "agent-fabric-contributor-github-tests"
description: "changing agent-fabric's tools/fabric/github modules or their tests? push a for/ branch, not a patch — they are in devex-tooling's contributor entry"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 35bc7e70d352a28f
---

## changing agent-fabric's tools/fabric/github modules or their tests? push a for/ branch, not a patch — they are in devex-tooling's contributor entry

Since agent-fabric 8220c862 (2026-10-21, fabric-coordinator's reply
01a15f20 on #348), tests/test_guards_wired.py and nine other tests of the
tools/fabric/github/ modules are in devex-tooling's contributor entry
(policies/authority.json `contributors`). The next change to them goes on
a `<host>/devex-tooling/for/fabric-coordinator/<what>` branch that
fabric-coordinator folds unrebased, not as a patch through the handover
area. Read the entry on origin/main for the exact path list before
relying on it.

**Why:** j186's one-hop rule went as a branch plus a separate tests patch;
the coordinator widened the entry so the tests travel with the code.
**How to apply:** check `policies/authority.json` before choosing patch
vs branch for any agent-fabric change.

*Anchors: `agent-fabric:policies/authority.json::tests/test_guards_wired.py`*

*Observed 2026-10-21 (devex-tooling)*
