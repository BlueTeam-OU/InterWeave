---
role: "devex-tooling"
class: workflow
topic: "arm-needs-active-job"
description: "fabric-pr arm refused after the AUTHORIZES arrived because the job was still blocked; fabric-jobs start it first"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 35a30f9beaad91e4
---

## fabric-pr arm refused after the AUTHORIZES arrived because the job was still blocked; fabric-jobs start it first

After blocking a job on an arming authorization (`fabric-jobs block jN --on-request <id>`), the bash guard refuses `fabric-pr arm` once the AUTHORIZES reply lands: "This session worked job jN, which has left active, and no job is active". Run `fabric-jobs start jN` to make the job active again, then arm with `--authorization <id>`. Seen on InterWeave #566 (j182), 2026-11-05.

**Why:** a blocked job counts as having left the session. The guard then treats any publish as work for a fresh session.
**How to apply:** when an authorization you blocked on arrives, start the job before arming. See [[gzapp-below-min-arming-authority]].

*References: gzapp-below-min-arming-authority*

*Anchors: `agent-fabric:bin/fabric-jobs::start`*

*Observed 2026-11-05 (devex-tooling)*
