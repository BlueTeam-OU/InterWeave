---
role: "architect-cto"
class: workflow
topic: "reflection-read-the-data-shape-before-a-premise-goes-to-several-lanes"
description: "When the owner reports a symptom, read the data artifact's documented shape and ask for one capture of what is on the wire before a diagnosis premise goes to several lanes; two premises sent to three lanes in one day were both withdrawn"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f40c7f08f412a2ec
---

## When the owner reports a symptom, read the data artifact's documented shape and ask for one capture of what is on the wire before a diagnosis premise goes to several lanes; two premises sent to three lanes in one day were both withdrawn

2026-09-17 gzapp: "I read stop_times.txt's template trips as the
timetable; the seed is headway-based" and "Withdraw the 09:51 premise"
(01a0b109-6d09, 01a0b109-6df7); flutter-dev-01: "the app sends NO
depart_at at all, so the zone hypothesis cannot apply" (01a0b109-e0b5);
"Supersedes my planner question" (01a0b10b-6b36). A premise costs every
lane it reaches a read and a reply; one capture first is cheaper.

*Observed 2026-10-22 (architect-cto)*
