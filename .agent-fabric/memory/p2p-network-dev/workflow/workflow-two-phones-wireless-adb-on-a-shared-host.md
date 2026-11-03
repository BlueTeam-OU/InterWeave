---
role: "p2p-network-dev"
class: workflow
topic: "workflow-two-phones-wireless-adb-on-a-shared-host"
description: "driving two phones over wireless adb from the shared develop-qzapp host: own server port, both leases, the first connect refused, pairing per account"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 92166e233795a5f8
---

## driving two phones over wireless adb from the shared develop-qzapp host: own server port, both leases, the first connect refused, pairing per account

Measured 2026-10-24 with the Pixel 9a and the A40 for plan §29 (e)'s two-phone rows (j66/j67):
- The default adb server port 5037 is host-wide: one account's server holds it, and another account's
  `adb` talks to that server (wrong key, wrong devices) or kills it. Run under your own port:
  `ANDROID_ADB_SERVER_PORT=5047` (rust-ui-dev-01 uses its own the same way), and `adb kill-server` at the end.
- Hold both device leases for the whole run: `fabric-lease pixel-9a -- fabric-lease samsung-a40 -- <script>`.
- Either phone refuses, or accepts and then drops, the FIRST `adb connect` after a server starts. Loop
  `adb connect` until `adb devices` lists each serial as `device` before starting the test.
- Wireless-debugging pairing is per adb key, so per account: a code the owner gave another login does not
  pair yours. A phone may also already accept your key from an earlier pairing; try `adb connect` first.
- A reboot moves the phone: the wireless-debugging port changes, the DHCP IP may change too (the Pixel went
  .52 -> .53 -> .54 over two reboots, 2026-10-24), and wireless debugging may come back off. Someone at the phone
  reads the new "IP address & port" after each boot. Give that person the exact screen path, and give the test a
  patient wait (20 min) keyed on the hardware serial and a new boot id. Each reboot run costs the owner two
  readings, so one passing run beside a timed-out one is the record (01a16ec6, 01a16ed7, 01a16ed9).
- An open TCP port plus "failed to connect" is a lost pairing OR a moved port. Try the same address from another
  login: if it connects there, the refusal is your key's pairing; if both fail, the port moved or debugging is off
  (01a1702f, 01a17030).
- Watch a long device test's output for the line that needs a person (the reboot being issued), not only for its
  end. A missed line delayed the request to the owner by 10 minutes (01a16ebd).
- `tests/android-e2e/tests/pairing.rs` selects phones by `INTERWEAVE_ANDROID_SERIAL_A/_B` and their LAN
  addresses by `INTERWEAVE_ANDROID_WIFI_ADDRESS_A/_B` (`AdbDevice::connect_to`).
Related: [[workflow-shared-device-build-by-the-installed-signer]], [[j66-stage-21-device-rows]].

*References: j66-stage-21-device-rows, workflow-shared-device-build-by-the-installed-signer*

*Observed 2026-10-25 (p2p-network-dev)*
