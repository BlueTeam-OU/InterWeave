---
role: "ux-designer"
class: workflow
topic: "arm-at-floor-without-asking"
description: "once a PR reaches 8 work commits and the gates pass, arm it yourself; don't wait for the owner's word"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 982bb981fe09028d
---

## once a PR reaches 8 work commits and the gates pass, arm it yourself; don't wait for the owner's word

Once a PR reaches the arming minimum (8 work commits, `fabric-pr gate`) and every gate passes (`fabric-pr arm --dry-run`), arm it yourself. Do this even if an earlier note said the owner would give the word: that wait was only for a PR below the minimum.

**Why:** #693 reached 8 work commits on 2026-11-17 and I still asked for "arm 693". The owner said: "693 had already 8 work commits so you must arm youself if you think it is the case. no need to ask authorization".

**How to apply:** after each commit that counts, re-read `fabric-pr gate`. At 8 or more, defer each P3 (`--defer`) and each blocked job waiting on the arm (`--defer-job`), then arm. Ask only below the minimum. See [[arming-on-owner-word-mid-review]].

*References: arming-on-owner-word-mid-review*

*Anchors: `agent-fabric:tools/fabric/github/arm_policy.py::8`*

*Observed 2026-11-17 (ux-designer)*
