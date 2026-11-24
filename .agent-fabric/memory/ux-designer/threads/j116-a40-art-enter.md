---
role: "ux-designer"
class: threads
topic: "j116-a40-art-enter"
description: "j116 A40 re-run of R2 'art'+Enter (#634, merged 66fbe010b) — blocked on the A40's address and its lease; run plan with start and end state"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d0c9fb41afe3a4a0
---

## j116 A40 re-run of R2 'art'+Enter (#634, merged 66fbe010b) — blocked on the A40's address and its lease; run plan with start and end state

j116 (2026-11-12), blocked:
- **Address:** a private-network address:44599 had no route. The owner said "same as last time", but the phone was not on that
  address.
- **Lease:** rust-ui-dev-02 held samsung-a40 from 12:24Z, and the owner said "someone is using a40".

**APK:** a host scratch path, sha256 47ceafb0. It is #634's 006259e90 with the Enter fix, signed
with the same debug key as the j246 build on the phone, so install -r keeps the test profile. #634 merged as
66fbe010b. A main build would also add the dialog guard and the j262 log texts, but for this check use the APK.
Tell rust-ui-dev-01 once it is installed, so they can remove a host scratch path and a host scratch path.
They expect no other reply (INFO 01a1cfb5).

**Run plan, given to the owner:**
1. Screenshot the start state first.
2. Use the app as the R2 start only if it shows R2 with words 1-23 entered.
3. On position 24, `input text art`, then `input keyevent ENTER`. Pass: the chip is taken and the trace shows the
   identity= change.
4. End: on R3 answer "No", which returns to R2 and restores nothing. Then Back, and discard in the dialog, to the
   screen R2 was opened from.
5. Final screenshot, and report the end state. See [[end-device-run-in-known-state]].

**Lease wrapper:** a40.sh in the j116 session scratchpad. It runs under fabric-lease samsung-a40 on its own adb port
and kills its server on exit. Rewrite it if the scratchpad is gone.

*References: end-device-run-in-known-state*

*Observed 2026-11-12 (ux-designer)*
