---
role: "architect-cto"
class: threads
topic: "j69-row-2-route-is-scan-2026-11-09"
description: "j69 row 2 (two fresh profiles added on two phones, p2p-network-dev-02's j103, 2026-11-09): ruled route (a), the real Scan; the field route cannot carry a PeerId under Slint 1.18.1 and is not the row's instrument, so its failing runs are a…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1a2195e98ead8e94
---

## j69 row 2 (two fresh profiles added on two phones, p2p-network-dev-02's j103, 2026-11-09): ruled route (a), the real Scan; the field route cannot carry a PeerId under Slint 1.18.1 and is not the row's instrument, so its failing runs are a dated note in the row's run file, never a second carry

Ruled 2026-11-09 for p2p-network-dev-02 (QUESTION 01a1bfb2, my REPLY in answer): j69 row 2 takes `add_by_scan`, route (a). The Stage 18 record in `architecture/roadmap/BOTTOM-UP-IMPLEMENTATION-PLAN.md` ("Measured, and carried as limits") already says row 2 "through the real Scan" is p2p-network-dev-02's, on the owner's word to clear both phones, its run file the carry's evidence. The field route (`adb shell input text` key events) lowercases a PeerId's capitals under Slint 1.18.1, the limit carried by name in plan §20 ([[j559-slint-shift-carry-plan-20-2026-11-08]], #596) and `apps/human-android/README.md`'s known-limit paragraph; so the two failing field runs of 08:37-08:42Z go in the row's run file as a dated note, not as a carry, and option (b) (a PR of the driver and the failing runs) was refused as proving nothing the row asks. Their supply eb3c8af9b (`add_by_field` refuses an untypable line at once) is the host-side case §20 names.

**Why:** a limit on record once is carried once; a row's instrument is what its record names, and a failed attempt with another instrument is no finding against the row.

**How to apply:** when a device row fails in a helper that is not the row's named route, check the stage record for the route before accepting a carry.

*References: j559-slint-shift-carry-plan-20-2026-11-08*

*Observed 2026-11-09 (architect-cto)*
