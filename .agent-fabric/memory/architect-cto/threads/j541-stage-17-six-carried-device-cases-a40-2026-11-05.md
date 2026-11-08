---
role: "architect-cto"
class: threads
topic: "j541-stage-17-six-carried-device-cases-a40-2026-11-05"
description: "Stage 17's six carried device cases ran on the A40 2026-11-05 (#568): five met and gone from plan §20's carried lists, the network callback stays carried (wireless adb never returns after the Wi-Fi does)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a81cce8f90d06d4e
---

## Stage 17's six carried device cases ran on the A40 2026-11-05 (#568): five met and gone from plan §20's carried lists, the network callback stays carried (wireless adb never returns after the Wi-Fi does)

j541 (from p2p-network-dev-02 INFO 01a1ac83, 2026-11-05): #544 built the
six cases Stage 17 carried (plan §20 clause (b): Keystore user-presence,
network callback, secure_span, fgs_lifecycle, backup_exclusion,
presence_prompt_and_cancel); #568 (3b94fcf3, stamped 16:00Z) ran them on
the Samsung A40 at API 30 with the owner at the phone, record
tests/android-e2e/runs/2026-11-05-samsung-a40-stage-17-carried-cases.md.
Five passed; `a_wifi_drop_and_return_reach_the_runtime_in_one_process`
failed twice because wireless adb did not come back within 600 s of the
Wi-Fi's return (adb_wifi_enabled read 1, no port until the owner toggled
it by hand); the phone's log shows drop and return in one process, a
reading not a pass. It needs a run where adb returns on its own or a USB
link (awkward on this host, the owner).

Bounds carried: one phone (One UI); no cellular handover; the two
user-presence cases ran on an identity made under key_unlock_policy
user-presence after a wipe on the owner's word, because nothing in that
build called custody::reseal (ADR-0042 A 2026-10-10 (ii); rust-ui-dev-01
j213, OBSERVATION 01a1abab) — see [[j535-adr-0042-failed-reseal-never-refuses-start-2026-11-05]].
The user-presence profile's invalidated-key case (TM-54) is NOT among the
six and stays carried.

Record: commit f5a77ec7b on #575 (j539's PR; 3 work + 4 fix, under the
minimum, the owner's word needed); rounds 3-5 posted, head e670296fa
clean (F3 bounds of identity_lines.rs, F4 TM-54 pointer, id prefix
01a1ac83 collides with ux-designer's QUESTION so cite 01a1ac83-81ed).
DELIVERED 2026-11-05; REPLY to p2p-network-dev-02 sent. Plan §20's "with the user-presence
case" carries (background_restart diagnostic, the picker's instrumented
items) now name their own reason.

**Why:** the stage record is where a carried item is met or re-carried by name; a "met" claim without the run's bounds would overstate it.
**How to apply:** when the network callback passes, strike it from §20's carried-by-name paragraph and the "did not prove" list in one commit; SPIKES.md L611's last sentence then needs past tense too.


**CLOSED 2026-11-08:** #575 (5 work, 6 fix) merged 4c4cf3587 at 2026-11-08T06:16Z on the owner's word 'arm 575' at head 2d6ca2bf3; F1 P3 lifted by --defer (cited §11 L1 on main via #579, 4w/3f, 62ba5cf01).

*References: j535-adr-0042-failed-reseal-never-refuses-start-2026-11-05*

*Observed 2026-11-08 (architect-cto)*
