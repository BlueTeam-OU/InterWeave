---
role: "architect-cto"
class: workflow
topic: "reflection-a-stage-tied-dead-function-exemption-needs-a-real-consumer-in-its-reason"
description: "A dead-function exemption tied to a stage names in its reason something the next stage's exit gate actually requires; an entry reasoned 'first read by the X stack' goes red at every stage move until a real consumer reads it"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 21398be3a128e5f0
---

## A dead-function exemption tied to a stage names in its reason something the next stage's exit gate actually requires; an entry reasoned 'first read by the X stack' goes red at every stage move until a real consumer reads it

InterWeave 2026-09-27: "17 runtime functions exempt 'until stage
11, first read by the reachability stack' were never read" (01a0e2e7);
"that gate required only the ConnectivitySummary… at the stage-13 move
the eight would go red again exactly as the seventeen did" (01a0e2f2).
tools/checks/domain_fn_exempt.txt's header says entries expire with
their stage, not what makes a reason valid (checked 2026-10-23).

*Observed 2026-10-23 (architect-cto)*
