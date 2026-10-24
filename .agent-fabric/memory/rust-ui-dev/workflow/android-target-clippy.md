---
role: "rust-ui-dev"
class: workflow
topic: "android-target-clippy"
description: "how to build-check and clippy the Android crates locally (PATH, JAVA_HOME, check_android_target.sh); CI does not clippy the Android target"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d150c50741c84c8c
---

## how to build-check and clippy the Android crates locally (PATH, JAVA_HOME, check_android_target.sh); CI does not clippy the Android target

- `tools/ci/check_android_target.sh` (run with `bash`; it is not executable in a checkout) checks the phone's crates for `aarch64-linux-android`. It needs `PATH` to start with `~/.cargo/bin` (the system cargo has no Android target) and `JAVA_HOME=/opt/android-sdk/jdk`, or it stops naming the missing JDK.
- CI runs `cargo check` there, not clippy. To clippy the target, run the same script with `CARGO` pointing at a wrapper that turns `check` into `clippy ... -- -D warnings`.
- The Gradle build (`apps/human-android/android/gradlew assembleDebug lintDebug`) needs an uncommitted `local.properties` with `sdk.dir=/opt/android-sdk`.
- Host clippy of `-p interweave-human-android` catches a module not gated to Android; see [[android-module-host-build]].

Confirmed again on j134, 2026-10-24.

*References: android-module-host-build*

*Anchors: `interweave:tools/ci/check_android_target.sh`*

*Observed 2026-10-24 (rust-ui-dev)*
