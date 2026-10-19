---
role: "rust-ui-dev"
class: solution
topic: "android-spike-device"
description: "The two Android test devices (Samsung A40 of SPIKE-008/009, Pixel 9a for API 34+) and the facts a device run depends on; toolchain provisioning is devex-tooling's"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2676716fc87ae1aa
  - d159d9b141db464a
---

## The two Android test devices (Samsung A40 of SPIKE-008/009, Pixel 9a for API 34+) and the facts a device run depends on; toolchain provisioning is devex-tooling's

**Samsung SM-A405FN (a40eea), SPIKE-008/009's device** (connected by the owner 2026-10-06):
- Dedicated test device: reboots, network toggles and backup/restore are allowed, on the harness apps only.
- Android 11 / API 30, first_api 28, arm64-v8a, security patch 2023-03-01, FBE, verified boot green.
- Keystore: NO StrongBox (TEE keystore HAL "mdfpp"). It has a device credential (the owner's PIN, unknown to me) and no fingerprint unless the owner enrols one.
- Power: Doze light and deep enabled. Samsung's power manager (com.samsung.android.lool, sm.policy) is present, and "Deep sleeping apps" kills a foreground service with no sticky restart (SPIKE-008 L6).
- Backup: Google BackupTransportService, LocalTransport and GMS D2dTransport. Samsung Smart Switch is an OEM device-transfer path outside Android's backup rules, so a transfer test must include it. The owner ruled Google cloud backup out by design. There is no mobile data (SIMs out of service).

**Pixel 9a (tegu), the API 34+ device** (from 2026-10-10, j51):
- Android 17 / API 37, first_api 35, arm64-v8a, patch 2026-10-05, dark theme.
- On API 37 INTERNET is a runtime permission.

**Both, over wireless adb:**
- The address and port are the owner's to read out, and are never recorded. The port changes on a Wi-Fi drop or a reboot, and a lost pairing needs the owner's pairing code.
- Every adb step goes through the remit's `.agent-fabric/bin/phone-run`: the lease, an adb server on this login's own port, and nothing left behind. Never the shared default port 5037.
- After a run, restore every setting changed (stay-on, backup transport) and uninstall the harness apps ([[release-phone-after-testing]]). The owner unlocks a locked phone; adb cannot.

**Host:** no /dev/kvm, so no emulator. The Android SDK/NDK/JDK/Gradle is devex-tooling's to provision (/opt/android-sdk); the Rust Android target is installed in this account by `tools/host/android/rust-android.sh`.

**Keystore invalidation, learned in SPIKE-009:** one cause needs one fresh key. Removing the screen lock kills every auth-bound key. Enrolling a biometric invalidates per-operation keys only, and a per-operation key needs an enrolled Class-3 biometric before it can be generated.

Related: [[release-disk-after-tests]] [[release-phone-after-testing]]

*References: release-disk-after-tests, release-phone-after-testing*

*Anchors: `.agent-fabric/bin/phone-run::fabric-lease`, `tools/host/android/rust-android.sh::cargo-ndk`*

*Observed 2026-10-19 (rust-ui-dev)*
