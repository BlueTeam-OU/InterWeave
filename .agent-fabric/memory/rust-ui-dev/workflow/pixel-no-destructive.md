---
role: "rust-ui-dev"
class: workflow
topic: "pixel-no-destructive"
description: "on the shared Pixel 9a, never pm clear, uninstall or reseal the app's profile; the owner withdrew an earlier yes"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 059272b7f95528c9
  - 17661a98775f9a86
---

## on the shared Pixel 9a, never pm clear, uninstall or reseal the app's profile; the owner withdrew an earlier yes

On 2026-10-20 the owner said "don't do anything destructive on pixel phone". That replaced their earlier choice, the same day, to clear app data for the launch-prompt test of InterWeave #313.

**Why:** the phone's InterWeave profile, and its Keystore key, are shared test state. Clearing it cannot be undone without the recovery phrase.

**How to apply:** use only `install -r`, which keeps the data, plus read-only adb (logcat, dumpsys, screenshots) and UI input. No pm clear, uninstall, run-as writes to the profile, or policy reseal, unless the owner asks for that specific act. A test that needs a fresh or user-presence profile goes back to the owner as a question. See [[android-device-credential-prompt]].
Corollary (reflection; MESSAGE-IDs 01a1538f-6e95, 01a15394-4f59, 01a15926-e6a0): `install -r` works only with the same signing key as the installed app. A debug APK built in another account is signed with that account's debug key, so installing it means an uninstall, which wipes the profile (ux-designer-01 hit it 2026-10-16; p2p-network-dev-02 would have on 2026-10-19). Another lane's build for the shared phone is built in my account, debug and androidTest both, and installed with -r; one build per window serves every lane.

*References: android-device-credential-prompt*

*Observed 2026-10-24 (rust-ui-dev)*
