---
role: "rust-ui-dev"
class: solution
topic: "j95-a5-device-case"
description: "The long-message device case (tests/android-e2e long_message.rs): its trace lines, how it drives the A40, and the paragraph figures measured"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - dbd3c8f4c8fdb388
---

## The long-message device case (tests/android-e2e long_message.rs): its trace lines, how it drives the A40, and the paragraph figures measured

- Trace lines (ui-trace feature, debug APK only): `page=reader`;
  `reader=drawn ms=N` (Open the whole message pressed to the first frame);
  `reader=end ms= frames= at-end= form=source|formatted` (End to the list
  end, 3 s patience).
- tests/android-e2e/src/long.rs grows each shape to the largest its
  envelope fits (binary search on encode_outbound). The case
  (tests/long_message.rs, #[ignore], device only) sends from a desktop
  daemon on loopback through adb reverse, provisions the app, launches it
  normally, then per shape: open, End formatted, switch, End source,
  close. INTERWEAVE_ANDROID_SHOTS saves frames.
- It needs the debug and the androidTest APK. Each TAB costs about 1.7 s
  on the A40 (API 30), so READ_TABS is 80.
- First figures (A40, 2026-11-08, paragraph shape only): drawn 233 ms,
  End 39 ms formatted / 20 ms source, one frame. The rest of the shapes
  wait for the run that the TAB fix unblocked.
- A secure keyguard makes the launch refuse after 300 s, correctly; the
  phone must stay unlocked ([[a40-device-run-traps]]).

*References: a40-device-run-traps*

*Anchors: `tests/android-e2e/src/long.rs::shapes`, `crates/human/ui-slint/src/trace.rs::reader_end_line`, `tests/android-e2e/src/screen.rs::READ_TABS`*

*Observed 2026-11-09 (rust-ui-dev)*
