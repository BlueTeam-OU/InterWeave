---
role: "ux-designer"
class: workflow
description: "How ux-designer reaches InterWeave's Android test phone (Pixel 9a) for a device review — the lease, pairing, cleanup"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5629f73d5ee964c4
---

## How ux-designer reaches InterWeave's Android test phone (Pixel 9a) for a device review — the lease, pairing, cleanup

From 2026-10-16 (the owner's decision, relayed by fabric-coordinator in
GZCoord 01a14648-b1ab-7341-b8f8-6bcfbb69ce42; the remits carry it from
InterWeave #286 at 7eb58c2b), ux-designer runs its own device review
on the Pixel 9a, which it shares with rust-ui-dev.
- One account drives the phone at a time. Run as
  `fabric-lease pixel-9a -- <the run>`, and check first with
  `fabric-lease pixel-9a --who`.
- End every run with `adb disconnect` and `adb kill-server`, so the
  next holder finds the phone free.
- The owner pairs the login at the phone (Wireless debugging, pairing
  code). A re-pair after the link drops is the owner's step, so ask
  for it and never work around it.
- TalkBack and the feel of a gesture stay a person's check. Script
  runs cover the rest.
- The SDK's adb is not on PATH by design. Use
  `/opt/android-sdk/platform-tools/adb` explicitly, since two adb
  versions fight over the device (tools/host/android/README.md).

Related: [[swipe-actions-j1]]
2026-10-16 ~20:00: the owner paired ux-designer-01 with the Pixel 9a (adb pair at the phone; guid adb-58091JEBF08730-a7l86p). Connect port differs from the pairing port; take it from Wireless debugging.
- The connect port (e.g. A private-network address:43191 on 2026-10-16) differs from the pairing port and can change; mDNS does not reach this VM, so ask the owner for the address shown under Wireless debugging. A different account's debug key means a reinstall wipes the app's data on the phone: ask the build's owner to install instead.
- From InterWeave #294 (2026-10-17, fabric-coordinator's INFO 01a14841-76d1): a phone run is `.agent-fabric/bin/phone-run <command>`, which takes the lease and leaves no adb process on any path; the run begins with `adb connect <ip>:<port>`. Read the remit on main before the next run; my scratchpad dev.sh wrapper is superseded by it.
- Before reviewing, prove which build is on the phone; the installed app can be an older build than the one under review (2026-10-16: a pre-final #289 build). Check a string the target head added in the installed native library, or compare the APK sha256 the installer reports, and name the commit in the review. If it is wrong, ask the build's owner to install `<commit>` with their key (`adb install -r`, data kept) and to reply with the commit and APK sha; never reinstall under your own key without the owner's word, since that wipes the app's data.

*References: swipe-actions-j1*

*Anchors: `tools/host/android/README.md::two adb servers of different versions fight over the device`*

*Observed 2026-10-16 (ux-designer)*
