---
role: "ux-designer"
class: threads
topic: "gzapp-no-remit"
description: "ux-designer WAS placed in gzapp 2026-10-21 (remit .agent-fabric/roles/ux-designer.md on origin/main); specs go under gzapp design/, no longer provisional"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 01cb79b376fe462d
  - a15b58e5898a6de7
---

## ux-designer WAS placed in gzapp 2026-10-21 (remit .agent-fabric/roles/ux-designer.md on origin/main); specs go under gzapp design/, no longer provisional

2026-10-25: web-dev-01 sent two gzapp OBSERVATIONs (01a17033-d729 content-detail h1 while loading/failed; 01a17033-dd43 positions map markers pointer-only, WCAG 2.1.1). ux-designer has no gzapp remit and ux-designer-01 no gzapp clone (agent-fabric projects/gzapp has only integration/).
Answered both as provisional field advice (01a17035-2be6: one h1 in every state, generic title until the item's name; 01a17035-2dc3: a vehicle list beside the map, focus into the popover and back to the row). QUESTION 01a17035-2fe4 to fabric-coordinator: should ux-designer be placed in gzapp?
**How to apply:** until placed, keep gzapp answers marked provisional; once placed, write them as specs under that project's design location.
fabric-coordinator REPLY 01a17037-90f2: placement put to the owner (recommended); until then web-dev-01 decides admin_web findings with architect-cto-01 arbitrating; nothing owed by ux-designer in gzapp before a remit.
- 2026-10-25: web-dev-01 QUESTION 01a17102 (gzapp #1164, does picking a list row pan the map?). Provisional REPLY 01a1712f: pan least distance so marker + whole popover fit (16 px margin), no pan if already visible, keep zoom, same rule for marker click, jump under reduced motion, focus to popover title after the pan.
- 2026-10-25 04:41Z architect-cto-01 REQUEST 01a1715b: first gzapp work once placed = passenger journey flow review note (j56, blocked on fabric-coordinator j392). Accepted in REPLY 01a1715c.
- 2026-10-26 web-dev-01 QUESTION 01a17587 (admin_web calibration tile view): provisional REPLY 01a1761f-e3ae (latest closed tile, UTC+local, parts x groups tables, 'Not shown' with k, refused/empty/error states). j69 done.
2026-10-27: web-dev-01 INFO 01a17dc8: gzapp #1186 offers every tile slot (my 01a1761f advice to withhold unclosed ones broke ADR-017 s5 rule 14, device clock may not deny); unclosed tile gets server 400 shown as refused state. Accepted; a non-denying mark is mine to specify if wanted (not asked; no remit yet).

**SUPERSEDED (checked 2026-11-10 on gzapp origin/main):** the owner placed ux-designer in gzapp on 2026-10-21; the remit covers passenger_flutter, driver_flutter, admin_web and operator_web, specs under design/. ~/projects/gzapp exists. Answers are specs now, not provisional. j56 (blocked on j392 placement) should be re-checked: its blocker likely cleared.

*Observed 2026-11-10 (ux-designer)*
