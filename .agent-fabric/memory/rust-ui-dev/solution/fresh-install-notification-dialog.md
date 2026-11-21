---
role: "rust-ui-dev"
class: solution
topic: "fresh-install-notification-dialog"
description: "a fresh install of the InterWeave debug APK on API 33+ opens the POST_NOTIFICATIONS dialog over the app; it takes every adb key, so device cases see \"(no focus line)\" for every TAB"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9241bfcdb11c356d
---

## a fresh install of the InterWeave debug APK on API 33+ opens the POST_NOTIFICATIONS dialog over the app; it takes every adb key, so device cases see "(no focus line)" for every TAB

Measured 2026-11-13 on the Pixel 9a (API 37), j251. After a fresh install, the app's first start opened "Allow InterWeave to send you notifications?" (com.android.permissioncontroller GrantPermissionsActivity). `launch()` still passed: the trace showed page=conversations and connect=not-owed. But `open_trust`'s 40 TABs found no focus line at all, because the dialog held window focus.

- How to recognise it: every TAB shows "(no focus line)", and `dumpsys window` shows `mCurrentFocus` = GrantPermissionsActivity.
- The fix: answer the dialog first. A tap on Allow (540,1362 on the Pixel) works, or `pm grant ... POST_NOTIFICATIONS` before the first launch.
- The app's first start also makes the identity, so a fresh install reaches Chats with an identity and no peer (connect=not-owed). kept_recovery needs nothing more.
- The j251 tree's long_message case could not serve as a setup step: it needs `target/debug/transport-daemon` built (cargo build -p interweave-transport-daemon).

**How to apply:** in any device run after a fresh install, check `mCurrentFocus` before the first case, and answer the permission dialog.

Related: [[android17-internet-runtime-permission]] [[android-e2e-device-run]] [[j251-kept-k1-secure-case]]

*References: android-e2e-device-run, android17-internet-runtime-permission, j251-kept-k1-secure-case*

*Observed 2026-11-13 (rust-ui-dev)*
