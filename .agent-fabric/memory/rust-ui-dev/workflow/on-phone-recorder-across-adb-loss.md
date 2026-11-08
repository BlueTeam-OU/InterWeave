---
role: "rust-ui-dev"
class: workflow
topic: "on-phone-recorder-across-adb-loss"
description: "how to record a Wi-Fi drop on a wireless-adb phone: a detached script on the phone writes to /data/local/tmp, pulled after; the phone rejoins at a new IP and port"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - fc4bf708533f75d2
---

## how to record a Wi-Fi drop on a wireless-adb phone: a detached script on the phone writes to /data/local/tmp, pulled after; the phone rejoins at a new IP and port

A Wi-Fi toggle drops wireless adb, so the phone records the run itself (j183, 2026-10-31; record in apps/human-android/device-runs/2026-10-31-pixel-9a.md, e63313a9):
- push a sh script to /data/local/tmp, start it with `adb shell "setsid nohup sh <script> > /dev/null 2>&1 < /dev/null &"`. It survived the adb loss and ran to the end.
- `cmd wifi set-wifi-enabled disabled|enabled` works from shell at API 37. Snapshot `dumpsys connectivity`, `mCurrentFocus`, `pidof`, `screencap`, with `logcat -v time` in the background.
- After the toggle the Pixel rejoined at a NEW DHCP IP (.5 -> .9) and a new port. From this Qubes VM, no MAC is visible (NAT). Scanning other LAN hosts for the port was refused by the safety classifier. Ask the owner for the address:port.
- The app's log shows the drop: "the runtime was told the networks: 0 addresses", then the LTE address. The status line stays "Online, limited" on the example profile.
- Remove your /data/local/tmp files after pulling them. j44-net-* there is p2p-network-dev's.
Related: [[release-phone-after-testing]] [[pixel-checks-owed]]

*References: pixel-checks-owed, release-phone-after-testing*

*Observed 2026-10-31 (rust-ui-dev)*
