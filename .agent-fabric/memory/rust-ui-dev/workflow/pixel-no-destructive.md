---
role: "rust-ui-dev"
class: workflow
topic: "pixel-no-destructive"
description: "on the shared Pixel 9a, never pm clear, uninstall or reseal the app's profile; the owner withdrew an earlier yes"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 059272b7f95528c9
---

## on the shared Pixel 9a, never pm clear, uninstall or reseal the app's profile; the owner withdrew an earlier yes

On 2026-10-20 the owner said "don't do anything destructive on pixel phone". That replaced their earlier choice, the same day, to clear app data for the launch-prompt test of InterWeave #313.

**Why:** the phone's InterWeave profile, and its Keystore key, are shared test state. Clearing it cannot be undone without the recovery phrase.

**How to apply:** use only `install -r`, which keeps the data, plus read-only adb (logcat, dumpsys, screenshots) and UI input. No pm clear, uninstall, run-as writes to the profile, or policy reseal, unless the owner asks for that specific act. A test that needs a fresh or user-presence profile goes back to the owner as a question. See [[interweave-stage17-close-j111]].

*References: interweave-stage17-close-j111*

*Observed 2026-10-20 (rust-ui-dev)*
