---
role: "rust-services-dev"
class: workflow
topic: "arm-word-request-blocks-the-job"
description: "Blocking a job on an arm-word REQUEST stops fabric-pr arm even after the owner gives the word; the request becomes the peer's job"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 630803c3d31d1bf1
---

## Blocking a job on an arm-word REQUEST stops fabric-pr arm even after the owner gives the word; the request becomes the peer's job

`fabric-jobs block <id> --on-request <arm-word REQUEST>` leaves no job active, and the fabric hook then refuses `fabric-pr arm`. `fabric-jobs start` refuses while the wait is open, and there is no flag to clear it. When the owner gave "arm 605" in session on 2026-11-09 (#605, job j63), it took about 30 minutes to clear:
- the peer's decline REPLY only moved the blocker to `job:<peer>/j<n>`, the job the request became on their list;
- that job dropped at 22:06Z, and the blocker cleared on a later re-check (re-checks run at most every ten minutes).

**Why:** a "declined, not needed" REPLY is not the end of a request-blocker. It clears only once the peer's job closes.

**How to apply:** after asking for an arm word, use `fabric-jobs pause`, not `block --on-request`, if the owner may answer in session. If you did block, ask the peer to *drop* the job the request became, not just to reply, then retry start in a background loop. Related: [[arm-word-by-message]].

*References: arm-word-by-message*

*Anchors: `agent-fabric:tools/fabric/jobs.py::now waiting on`*

*Observed 2026-11-09 (rust-services-dev)*
