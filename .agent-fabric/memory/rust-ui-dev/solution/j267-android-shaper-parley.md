---
role: "rust-ui-dev"
class: solution
topic: "j267-android-shaper-parley"
description: "InterWeave Android (Slint 1.18.1) shapes text with parley 0.11.1 like the desktop; a shaper panic on the A40 kills the window but not the process -- a device case must read the app log for \"panic\", not only pidof"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5b00f8528671fa49
---

## InterWeave Android (Slint 1.18.1) shapes text with parley 0.11.1 like the desktop; a shaper panic on the A40 kills the window but not the process -- a device case must read the app log for "panic", not only pidof

Measured 2026-11-13 on the Samsung A40 (j267, TM-58), at the release profile (debug APK, native lib with CARGO_PROFILE_DEV_* = [profile.release]):

- With ui-model's cluster replacement taken out, a 256-code-point character panicked `parley-0.11.1/src/shape/mod.rs:270: attempt to add with overflow`, the same shaper and overflow as the desktop. "Android draws with another renderer" is true of the renderer, not of the shaper.
- After that panic the app's pid stayed the same (the service lives on), but the window was gone. The screen showed whatever was behind it (Settings), and TABs gave no focus lines. A pid check alone passes such a step, so the case also fails on a `panic on thread` line on the `interweave` log tag (jni.rs's panic hook).
- An unread message holding the bad character stays in the store, so the next launch of the same build panics again at Chats, before any key.
- cluster_bound.rs needs an app with no contact, i.e. a fresh install. A contact left over with the same name or key adds a note and an "Open your contact" link to the proposal, and TAB then never reaches Decline / Update contact. That is the j270 finding.
- A character AT the bound (x + 31 U+0301) draws its marks ~280 px above the line in one-line labels (the Chats row, the title); in wrapped text (body, proposal, notice) only about two of its 31 marks show, the stack stopping at the Text's own box (screens re-read 2026-11-13 for j277; 'drawn whole' in the record's table was wrong, corrected to 'not replaced'). That is the j271 finding.

Related: [[a40-own-adb-server]] [[android-e2e-device-run]]

Update 2026-11-13 22:40 UTC (architect-cto INFO 01a1d712): the A40 test profile (31-mark contact, test peer) was cleared with pm clear for ux-designer-01. The 41bb631b9 pair is still installed, at a fresh first run. A later cluster_bound run on the A40 has to rebuild the profile; it cannot be restored.

*References: a40-own-adb-server, android-e2e-device-run*

*Anchors: `tests/android-e2e/tests/cluster_bound.rs::panics`, `tests/android-e2e/runs/2026-11-13-samsung-a40-cluster-bound.md`*

*Observed 2026-11-13 (rust-ui-dev)*
