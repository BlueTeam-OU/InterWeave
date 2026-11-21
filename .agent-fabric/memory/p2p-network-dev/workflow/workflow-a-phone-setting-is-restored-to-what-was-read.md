---
role: "p2p-network-dev"
class: workflow
topic: "workflow-a-phone-setting-is-restored-to-what-was-read"
description: "on a shared test phone, read a setting before a run changes it and put back that value, never a default; the A40's stay-awake 7 is the owner's"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - b97014f68f186f0c
---

## on a shared test phone, read a setting before a run changes it and put back that value, never a default; the A40's stay-awake 7 is the owner's

On 2026-11-09 my step-5 run script set `svc power stayon true` on both phones before the run and `svc power stayon false` after it. That left `stay_on_while_plugged_in` at 0. On the A40, 7 was the owner's own setting (the phone lives on its cable in stay-awake mode). Its screen then timed out and the secure keyguard came back. The next lane's run (rust-ui-dev-01's j240) waited for the owner to unlock it. The owner's rule, relayed by fabric-coordinator (01a1bf93): leave the A40 at 7, before and after a run, and never set it to 0.

**Why:** a shared phone's settings belong to the owner and the other lanes. "Off again at the end" turned the owner's setting off.

**How to apply:** before a run changes any device setting, read it (`settings get global stay_on_while_plugged_in`). Afterwards, put back the value you read, never a default. On the A40, do not touch stay-on at all. A run that needs the screen on checks `deviceLocked=0` instead.
Related: [[workflow-two-phones-wireless-adb-on-a-shared-host]].

*References: workflow-two-phones-wireless-adb-on-a-shared-host*

*Anchors: `tests/android-e2e/src/adb.rs::INSTRUMENTATION_ENV`*

*Observed 2026-11-09 (p2p-network-dev)*
