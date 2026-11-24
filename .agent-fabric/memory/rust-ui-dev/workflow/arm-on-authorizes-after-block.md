---
role: "rust-ui-dev"
class: workflow
topic: "arm-on-authorizes-after-block"
description: "arming on an architect-cto AUTHORIZES after blocking my job on the ask — start the job again --despite their intake job, then fabric-pr arm --authorization <id>"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2c46cde0eab907e6
---

## arming on an architect-cto AUTHORIZES after blocking my job on the ask — start the job again --despite their intake job, then fabric-pr arm --authorization <id>

Blocking my job on an arm-word REQUEST takes it out of active, and the commit/publish hook then refuses `fabric-pr arm`. Once the AUTHORIZES REPLY arrives, `fabric-jobs start <job>` still refuses: the recipient's intake job for my REQUEST stays open. `fabric-jobs start <job> --despite job:<their login>/j<n> "<why>"` takes it back (InterWeave #678, 2026-11-15).

Then `fabric-pr arm <n> --authorization <reply message-id> --basis "..."`. A basis that only says "architect-cto's word" is refused as "an authority never arms its own PR".

**Why:** the first arm attempt on #678 was refused twice for these two reasons.
**How to apply:** keep the job active while waiting for an arm word; or, if it is blocked, use the two commands above. Related: [[authorizes-binds-head]], [[arming-tool]].

*References: arming-tool, authorizes-binds-head*

*Anchors: `agent-fabric:bin/fabric-pr`*

*Observed 2026-11-15 (rust-ui-dev)*
