---
role: "rust-ui-dev"
class: workflow
topic: "a40-signer-forms-and-facts"
description: "Debug APK signer: dumpsys [26d1eb18] is not the cert SHA-256 (0392b790); read with apksigner, not keytool; KeySender harness facts hold on the A40 (j114)"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - dbc90f43f09cb647
---

## Debug APK signer: dumpsys [26d1eb18] is not the cert SHA-256 (0392b790); read with apksigner, not keytool; KeySender harness facts hold on the A40 (j114)

Learned in j114 (2026-11-16, PR #689).

- **Two forms of one signer.** `dumpsys package` prints `signatures:[26d1eb18]`, a hash code; this login's
  debug key's certificate SHA-256 is `0392b790…`. Earlier A40 records call the first "signer". Compare
  installed builds by certificate digest: pull the APK, `apksigner verify --print-certs`
  (/opt/android-sdk/build-tools/36.0.0, JDK on PATH). `keytool -printcert -jarfile` prints NOTHING for these
  APKs (v2 signature only), so a check built on it reads every signer as empty.
- **The four harness facts #669 left open hold on the A40, API 30**: adb passes exit status through; KeySender
  injects via app_process (InputDispatcher logs `Inject key` with the sender's stamp as event time); one
  clock (stamps bracketed by `log` marks in the same shell); logcat monotonic has exactly 3 ms digits.
- **A5 asks for a screen recording**; the case only makes one with `INTERWEAVE_ANDROID_SHOTS=<dir>`. I ran
  without it, so the bar frames went uninspected. Set it on any A5 Back run.
- The run script (lease, own adb on 15065, probe/replace/install/facts/run) was in the j114 scratchpad;
  the A40 was at a private-network address:37606 that day.

Related: [[shared-phone-install-check]], [[j109-a5-back-runs-keysender]], [[a40-device-run-traps]]

*References: a40-device-run-traps, j109-a5-back-runs-keysender, shared-phone-install-check*

*Anchors: `apps/human-android/device-runs/2026-11-16-samsung-a40-back-runs.md`, `tests/android-e2e/src/back_run.rs::judge`*

*Observed 2026-11-16 (rust-ui-dev)*
