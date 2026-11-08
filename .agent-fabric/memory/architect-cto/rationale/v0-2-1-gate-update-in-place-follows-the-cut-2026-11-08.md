---
role: "architect-cto"
class: rationale
topic: "v0-2-1-gate-update-in-place-follows-the-cut-2026-11-08"
description: "Ruling 2026-11-08 for p2p-network-dev-01 — Restore and the Pixel run (test-key release APK) gate the owner's v0.2.1 cut; the update in place from v0.2.0 runs on the published v0.2.1 APK after the cut, a named limit until then; §22…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 86e889541b9ac285
---

## Ruling 2026-11-08 for p2p-network-dev-01 — Restore and the Pixel run (test-key release APK) gate the owner's v0.2.1 cut; the update in place from v0.2.0 runs on the published v0.2.1 APK after the cut, a named limit until then; §22 paragraph amendment owed when the device record is on a branch

p2p-network-dev-01 asked (QUESTION 01a1ba06-1145) whether the update in place from v0.2.0 gates the v0.2.1 cut, since Android refuses any signer but the owner's release key over v0.2.0 and release.yml signs only on a v*.*.* tag (dry run signs nothing; verified on origin/main 4c4cf3587, release.yml L37-40, 76-82).

Ruled (a) (REPLY 01a1ba06-bafa): Restore on a release build and the Pixel 9a at API 37 on the fixed release gate the cut; the update in place runs on the published v0.2.1 straight after the cut and is a named limit of the Validate row until it runs. Grounds: plan §22's baseline rule (the PATCH is the baseline the update half runs from), v0.2.0 holds no identity so the case sees only the package update, and (b) hand-signing outside release.yml would defeat ADR-0066's six-job key/token split.

**Owed:** amend §22's onboarding paragraph when p2p-01's Pixel/Restore device record is on a branch (they send the sha): the record names the update in place as the one case left for the first window after the cut, and the cut waits on that record.

*Observed 2026-11-08 (architect-cto)*
