---
role: "rust-ui-dev"
class: threads
topic: "j218-one-unlock-prompt"
description: "j218 DONE: one Unlock prompt per showing; fix merged in #582 (a69dcabb7); A40 device run 2026-11-08 passed, also #582 F1 (two back-to-back RESTARTs)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0c0dc47ba8288d3b
---

## j218 DONE: one Unlock prompt per showing; fix merged in #582 (a69dcabb7); A40 device run 2026-11-08 passed, also #582 F1 (two back-to-back RESTARTs)

The fix c01eb6a70 (UnlockRoute.prompting, Route::Asked) merged with #582 (5 work, 1 fix; head a69dcabb7). That PR also carries j220's RESTART hook, j222's REFUSE_STORE hook and the F1 fix 7acf6e8d7, which snapshots `starts` when startClient queues the start.

Device run, Samsung A40 (API 30), debug a69dcabb7, 2026-11-08, evidence in a scratch path:
- Race: user-presence key window expired, then `am start` of the Activity plus the RESTART hook in the same showing.
  - The Activity raised the prompt (ui-trace prompt=unlock at 01:05:37.084). The Service start was refused 13 ms later (start answered -10).
  - BiometricService logged ONE "Creating auth session", and there was one BiometricPrompt3P window.
  - The owner cancelled with Back: onError 10, prompt=cancelled, focus=notice. No second session.
- F1: after the owner's Unlock (start answered 1), two RESTART broadcasts were sent back to back.
  - The second hook (01:11:50.009) fired while the first hook's start was still in flight (answered .091).
  - The runtime started each time and the last start answered 1. There was no stop after it, and the Service was still foreground 30 s later.
- Not shown: the Activity's own check has no log line when it is answered Asked; the order is read from the trace. TalkBack was not exercised.

Related: [[a40-own-adb-server]] [[j223-open-pr]]

*References: a40-own-adb-server, j223-open-pr*

*Observed 2026-11-07 (rust-ui-dev)*
