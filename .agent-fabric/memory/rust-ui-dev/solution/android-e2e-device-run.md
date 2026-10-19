---
role: "rust-ui-dev"
class: solution
topic: "android-e2e-device-run"
description: "running InterWeave's android-e2e --ignored cases on the Pixel over wireless adb -- the traps that cost a run on 2026-10-16"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3eb1b2ac38fe58f7
---

## running InterWeave's android-e2e --ignored cases on the Pixel over wireless adb -- the traps that cost a run on 2026-10-16

Learned on j66's device runs (InterWeave#283, merged f4104bce):
- Gradle can call the native library up to date after a Rust-only
  commit; check `cargo` reported the crate fresh against the sources,
  or `--rerun-tasks`, before trusting an installed APK as the head.
- Wireless adb drops when Wi-Fi does (and Wireless debugging may come
  back on a new IP AND port); a case failing with "adb: device offline"
  is the link, not the code -- reconnect and rerun that case.
- audit_gate clears logcat; capture the phone log right after any
  failing case, before the next one runs.
- Each case's instrumented half ends its runtime when it answers, so
  peer-scoped view state (route indicator) is gone after the loop;
  read it while the side runs (human_chat.rs seen_paths).
- paths' relayed case fails intermittently (1 in 5 on 2026-10-16:
  circuit closed after ~2 s, D's 30 s backoff outlived the deadline);
  p2p-network-dev-02's j36 owns it.
- Run order that works: `./gradlew installDebug installDebugAndroidTest`
  from apps/human-android/android with ANDROID_SERIAL set, then each
  test file with `-- --ignored --test-threads 1`.
Related: [[device-case-reads-while-connected]].

*References: device-case-reads-while-connected*

*Anchors: `tests/android-e2e/README.md::installDebug installDebugAndroidTest`*

*Observed 2026-10-16 (rust-ui-dev)*
