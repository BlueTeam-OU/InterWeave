---
role: "rust-ui-dev"
class: workflow
topic: "release-phone-after-testing"
description: "every Pixel run goes through the remit's .agent-fabric/bin/phone-run (own lease, own adb server, nothing left); never adb on the shared port 5037; how a lost pairing looks"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 466cb4a21bc24d1f
  - 85285401b18a3e8d
---

## every Pixel run goes through the remit's .agent-fabric/bin/phone-run (own lease, own adb server, nothing left); never adb on the shared port 5037; how a lost pairing looks

The remit's paragraph "The test phone" (InterWeave main) is the procedure: every run on the
Pixel, a probe too, is `.agent-fabric/bin/phone-run <command> [args...]` from the working
copy's root. It takes the `pixel-9a` lease, starts an adb server of this login's own on a port
of its own, and leaves no adb process on any exit path. Never `adb kill-server`, nor any adb on
the default port 5037, outside it. Every login on the host shares 5037: another account's
kill-server there dropped my device (2026-10-16), and in turn I once had a stale server of my
own still connected after telling a peer I held none. What the remit does not say:

- **Before you tell anyone the phone is free,** run `pgrep -u "$(id -u)" -a adb`. Do not
  answer from memory of what you started.
- **A lost pairing looks like this:** ping answers and the port accepts TCP, but `adb connect`
  says "failed to connect" right after the connection opens, and mDNS lists nothing. The phone
  is refusing this login's adb key, not the address. A pairing is per login. A new address or
  a reset of Wireless debugging voids it. Ask the owner for "Pair device with pairing code":
  the pairing IP:port and the six-digit code. They are valid only while that dialog stays
  open, so pair at once, then connect to the port the main screen shows.
- **"No route to host"** means the phone is off the network or asleep. A refused port means
  Wireless debugging is off or has moved. Either way it is the owner's step.
- **The link drops mid-run:** reconnect before each adb step (`adb get-state || adb connect`).
- Report the release, and the state the app is left in, with the run's results.

**Why:** the phone is shared between rust-ui-dev and ux-designer, one holder at a time. A
release done wrong costs the other holder a dropped device, and a wrong "free" costs them a
failed connect.
**How to apply:** read the remit's phone paragraph on main before the first device step of a
session. Related: [[android-spike-device]] [[release-disk-after-tests]] [[device-review-run-method]]

*References: android-spike-device, device-review-run-method, release-disk-after-tests*

*Anchors: `.agent-fabric/roles/rust-ui-dev.md::phone-run`*

*Observed 2026-10-18 (rust-ui-dev)*
