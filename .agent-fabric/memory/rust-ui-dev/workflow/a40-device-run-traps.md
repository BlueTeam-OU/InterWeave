---
role: "rust-ui-dev"
class: workflow
topic: "a40-device-run-traps"
description: "Driving the InterWeave app on the A40 over wireless adb - a conversation opened is read and leaves the list; host-side timing is seconds off; FLAG_SECURE onboarding; provision writes config"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 336eae87d5b60b74
---

## Driving the InterWeave app on the A40 over wireless adb - a conversation opened is read and leaves the list; host-side timing is seconds off; FLAG_SECURE onboarding; provision writes config

Learned in j89 (2026-11-04, `apps/human-android/device-runs/2026-11-04-samsung-a40.md`):

- **Opening a conversation reads it, and a read, unkept conversation leaves the store (ADR-0044), so it
  drops off the list.** A TAB walk after that sees one row fewer. I took the missing row for "TAB skips
  the second row" and wrote a host test for it before a screenshot showed only one row was left.
  Screenshot the list before calling something a reach defect. The chat cases (`human_chat.rs on_a_device`)
  make fresh unread conversations.
- **Host-clock timing over wireless adb is seconds off**: `adb shell input keyevent` came back about 10 s
  after the trace logged the page. Time a tap-to-draw on the phone itself, in one `adb shell '...; date
  +%s.%N; screencap ...'`.
- `am start -W` hangs while the keyguard shows: check `isKeyguardShowing` first and put `timeout` on it.
- The onboarding window is FLAG_SECURE, so a screencap of R1/K1 is empty. Read the `ui-trace` `identity=` and
  `warns=` lines instead.
- The android-e2e `provision` case replaces config.yaml (owner-only since 797eef6c). Save it with run-as cp
  to `cache/` and restore it after. `run-as <pkg> sh -c "..."` mis-quotes, so use one `run-as` per command.
- A plain uiautomator dump shows Slint as one empty View, with TalkBack on or off (no AccessKit adapter;
  plan section 19 TalkBack carry). The owner called TalkBack on the card page "unusuable".

Related: [[a40-lock-change-word]] [[slint-conditional-wrapped-text-short]]

*References: a40-lock-change-word, slint-conditional-wrapped-text-short*

*Anchors: `crates/human/android-platform/src/stand_in.rs::write_private_atomic_within`, `tests/android-e2e/tests/human_chat.rs::on_a_device_the_phone_chats_over_the_relayed_path_with_a_stand_in_as_the_control`, `crates/human/ui-slint/src/trace.rs::warns=`*

*Observed 2026-11-04 (rust-ui-dev)*
