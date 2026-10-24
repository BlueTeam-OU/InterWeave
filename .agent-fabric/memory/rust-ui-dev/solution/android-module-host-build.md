---
role: "rust-ui-dev"
class: solution
topic: "android-module-host-build"
description: "A new module in apps/human-android must be cfg(target_os = \"android\") when it uses jni/keystore/ui-slint; per-crate test runs and the Android check miss a broken host build"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d37697e6ae897ea4
---

## A new module in apps/human-android must be cfg(target_os = "android") when it uses jni/keystore/ui-slint; per-crate test runs and the Android check miss a broken host build

apps/human-android/src/lib.rs gates activity, jni and keystore with `#[cfg(target_os = "android")]`. onboard.rs (d5b9b9fc, 2026-10-19) was declared ungated, so the workspace's host build of `interweave-human-android` failed with 9 unresolved-import errors. That held for the branch's whole life until `cargo xtask ci` was run before the PR; fixed in d87a93ce. Running nextest per crate, `check_android_target.sh` and gradle assembleDebug all passed, because none of them builds that crate for the host.

**How to apply:** after adding a module to apps/human-android, run `cargo clippy -p interweave-human-android --all-targets -- -D warnings` on the host, or the whole of `cargo xtask ci`, before pushing. A branch with no PR has no CI to catch it. See [[view-change-runs-desktop-e2e]] for the desktop suite's prerequisites.

*References: view-change-runs-desktop-e2e*

*Observed 2026-10-19 (rust-ui-dev)*
