---
role: "rust-ui-dev"
class: solution
topic: "cargo-ndk-ndk-env-names"
description: "which NDK cargo-ndk uses when several env names are set, and how the APK build pins it (j209)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b403fb824d5a324c
---

## which NDK cargo-ndk uses when several env names are set, and how the APK build pins it (j209)

cargo-ndk 4.1.2 (`src/cli/mod.rs` derive_ndk_path) reads ANDROID_NDK_HOME, ANDROID_NDK_ROOT, ANDROID_NDK_PATH and NDK_HOME in that order. It takes the first one set and only warns ("doesn't match") about any other that differs. It then picks the highest version under the path when the path holds versioned dirs. The GitHub runner image exports ANDROID_NDK_ROOT at its own NDK (27.3). So since j209 (47f054d3b), the cargoNdk doFirst in apps/human-android/android/app/build.gradle.kts sets all four names to AGP's resolved NDK.

Same job: android.useAndroidX=false was dropped (43d53f49f). The app has no AndroidX dependency, so the debug and androidTest APKs stayed byte-identical in manifest and dex, and the STRICT lockfiles did not change.

To reproduce the runner's mismatch locally, export ANDROID_NDK_ROOT=<empty dir named 27.3.13750724> before ./gradlew. Branch develop-qzapp/rust-ui-dev-01/feat/j209-android-apk-job-two-warnings-from rides the next human-crates PR. Related: [[android-shell-build]]

*References: android-shell-build*

*Anchors: `apps/human-android/android/app/build.gradle.kts::ANDROID_NDK_ROOT`*

*Observed 2026-11-04 (rust-ui-dev)*
