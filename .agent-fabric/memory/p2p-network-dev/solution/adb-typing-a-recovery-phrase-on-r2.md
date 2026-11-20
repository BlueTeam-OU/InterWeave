---
role: "p2p-network-dev"
class: solution
topic: "adb-typing-a-recovery-phrase-on-r2"
description: "How to type a recovery phrase into the Android client's Restore word screen (R2) over adb, and what the host cannot see on a release build"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d9b46b651520a4c7
---

## How to type a recovery phrase into the Android client's Restore word screen (R2) over adb, and what the host cannot see on a release build

The human-android Restore word screen (R2) has no text field: its keypad takes hardware-key letters and fills a word only on Enter; a space is ignored (`crates/human/ui-slint/ui/app.slint`, the identity page's `key-pressed`). Over adb, send one word at a time: `adb shell "input text <word>; input keyevent 66"`. One `input text` with all 24 words fills only the first word.

On a RELEASE build the onboarding window is FLAG_SECURE and there is no `ui-trace` (debug only), so the host sees neither screens nor state: a person must read the screens. The host's evidence of a seal is logcat's `start answered 0` (a release first run without an identity is refused), cleared with `logcat -c` before each Continue.

Measured in j152, 2026-11-08, both phones; record `packaging/android/evidence/2026-11-08-v0-2-1-release-gate.md` (commit 6b2141041).

*Anchors: `crates/human/ui-slint/ui/app.slint::key-pressed`, `apps/human-android/src/jni.rs::NO_IDENTITY`*

*Observed 2026-11-08 (p2p-network-dev)*
