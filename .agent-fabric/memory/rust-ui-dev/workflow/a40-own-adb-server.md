---
role: "rust-ui-dev"
class: workflow
topic: "a40-own-adb-server"
description: "Driving the Samsung A40 (InterWeave Stage 17 device, API 30): no phone-run for it; lease samsung-a40 + own adb server port, address only from the owner"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3520c6cd7c564843
---

## Driving the Samsung A40 (InterWeave Stage 17 device, API 30): no phone-run for it; lease samsung-a40 + own adb server port, address only from the owner

The remit's `.agent-fabric/bin/phone-run` holds only the `pixel-9a` lease. For
the Samsung A40 (SM-A405FN, API 30; destructive steps allowed at the owner's
word, unlike the Pixel, see [[pixel-no-destructive]]) I ran under
`fabric-lease samsung-a40 -- <script>` with a script that exports
`ANDROID_ADB_SERVER_PORT=$((15037 + uid % 1000 + 1))` (phone-run's port + 1,
so the two never share a server), `ANDROID_SERIAL=<addr:port>`, starts the
server, connects, runs, and `adb kill-server` on EXIT (2026-10-22, j131).

- The address and port come from the owner each time (the A40's DHCP address
  moved from .27 to .6 overnight; "No route to host" at the old one). mDNS
  discovery does not reach this host, so the port must be read out too.
- After a run, `pgrep -u $(id -u) -a adb` must show nothing.
- Device tests that write the profile (the android-e2e `provision` case) go
  here, never on the Pixel.
Related: [[release-phone-after-testing]] [[android-e2e-device-run]]

*References: android-e2e-device-run, pixel-no-destructive, release-phone-after-testing*

*Anchors: `.agent-fabric/bin/phone-run::ANDROID_ADB_SERVER_PORT`, `tests/android-e2e/tests/app_lifecycle.rs::AdbDevice::connect`*

*Observed 2026-10-22 (rust-ui-dev)*
