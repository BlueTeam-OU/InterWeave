---
role: "p2p-network-dev"
class: solution
topic: "domain-android-ui-driving-needs-a-trace"
description: "driving the Android human client's screens from the host for a device row: no a11y tree, FLAG_SECURE, keys work, a debug trace is the done signal"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - dd6364ca44a6381f
---

## driving the Android human client's screens from the host for a device row: no a11y tree, FLAG_SECURE, keys work, a debug trace is the done signal

A device row that must go through the real screens (row 2 via Scan, j69, 2026-10-25) cannot be a case under the
E2eCase instrumentation: that process runs the Rust service and never starts the Activity. Measured by rust-ui-dev-01
at 8f978966:
- Slint draws no Android accessibility tree, so UiAutomator and the host cannot find a control by label.
- Show my address sets FLAG_SECURE, so a host screencap of the code is black; the code's text comes from the
  own_addresses case, never from the screen.
- Injected adb keyevents TAB / ENTER / ESCAPE do move Slint's focus and press controls (A40). Focus survives
  `am start`, so each step force-stops and relaunches.
- Counting TABs has no check, so the client gained a debug-only logcat UI trace (page, focus key, outcome kind; no
  address/PeerId/name; release build refused by verifyReleaseNative) as each step's done signal.
Joining the halves: a screen step in the app process, stop the app only after the screen shows the answer (a trust
or add is on disk before it is answered, LOCAL-IPC.md ~390), then an instrumentation case reads the network outcome.
Branch: develop-qzapp/rust-ui-dev-01/for/p2p-network-dev-02/scan-path-ui (86ff7386). Related: [[j69-cache-and-reboot-rows]].

*References: j69-cache-and-reboot-rows*

*Anchors: `apps/human-android/android/app/src/main/kotlin/org/interweave/human/MainActivity.kt::FLAG_SECURE`, `tests/android-e2e-cases/src/pairing.rs::AwaitPeerArgs`*

*Observed 2026-10-25 (p2p-network-dev)*
