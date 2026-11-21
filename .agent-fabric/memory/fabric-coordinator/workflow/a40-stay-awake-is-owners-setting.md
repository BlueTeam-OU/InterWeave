---
role: "fabric-coordinator"
class: workflow
topic: "a40-stay-awake-is-owners-setting"
description: "Both test phones' (A40, Pixel 9a) stay_on_while_plugged_in=7 is the owner's own setting (cable + stay-awake); a run must never reset it to 0"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 4aa0c0329093d547
---

## Both test phones' (A40, Pixel 9a) stay_on_while_plugged_in=7 is the owner's own setting (cable + stay-awake); a run must never reset it to 0

The owner keeps both test phones, the Samsung A40 and the Pixel 9a, on their cables in stay-awake mode (`settings global stay_on_while_plugged_in` = 7). On 2026-11-09 the j69 row 2 run (phone order step 5) "restored" it to 0 after its run. The screen then timed out, the secure keyguard came back, and the next run (j240) waited for an owner unlock.

**Why:** a run that undoes the owner's phone setting creates owner work, which the phone order exists to avoid.
**How to apply:** when a phone order or device-run brief mentions either phone, say: leave stay_on_while_plugged_in at 7 before and after; never set it to 0. TalkBack was set aside the same day: the owner will not spend time testing it (only a person-free path may run).

*Observed 2026-11-09 (fabric-coordinator)*
