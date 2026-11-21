---
role: "ux-designer"
class: workflow
topic: "phone-addresses-from-architect"
description: "Ask architect-cto, not the owner, for the Pixel's and A40's adb IP:port and a phone slot"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 040d6a88e9a4a76f
---

## Ask architect-cto, not the owner, for the Pixel's and A40's adb IP:port and a phone slot

The owner, 2026-11-14: "cto know both phone connect ip and port". architect-cto holds both phones'
Wireless debugging IP:port and schedules who holds the leases (that day p2p-network-dev-02 held both for a
clean-profile measurement). First asked: QUESTION 01a1da31 for j95.

**Why:** several jobs (j95, j109, j116, j122) sat blocked "on owner" for an address architect-cto already had.
**How to apply:** a device run that needs an address or a slot asks architect-cto-01 by QUESTION, and
the job is blocked on that request, not on the owner. See [[workflow-carried-2026-10-18]].

*References: workflow-carried-2026-10-18*

*Observed 2026-11-14 (ux-designer)*
