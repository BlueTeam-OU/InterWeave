---
role: "devex-tooling"
class: solution
topic: "android-check-ranlib-and-target"
description: "android check fails in make (openssl-src) or with E0463 can't find core? RANLIB_<target>, and the aarch64 target on this account"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7b149c1c128335c9
---

## android check fails in make (openssl-src) or with E0463 can't find core? RANLIB_<target>, and the aarch64 target on this account

tools/ci/check_android_target.sh exports LINKER, CC, AR and (since for/rust-ui-dev-01/android-check-ranlib,
e8897709, 2026-10-22) RANLIB for aarch64-linux-android. A C dependency built by its own make
(openssl-src under rusqlite's bundled-sqlcipher-vendored-openssl) runs ranlib; without RANLIB_<target>
cc 1.6.0 tries llvm-ranlib on PATH, then aarch64-linux-android-ranlib, which the NDK does not ship ->
make Error 127. The Gradle build is unaffected: cargo-ndk sets it.

To run the script on this host: JAVA_HOME=/opt/android-sdk/jdk ANDROID_HOME=/opt/android-sdk, and the
account's pinned toolchain needs `rustup target add aarch64-linux-android` once (E0463 "can't find crate
for core" otherwise; added 2026-10-22). A cold run with vendored OpenSSL takes ~18 min and ~1.8G target.

**Why:** both failures look like the change under test, and neither is.
**How to apply:** measure an Android build change on a trial merge in scratch, with its own
CARGO_TARGET_DIR, and remove both after. See [[new-guard-needs-ci-and-xtask]].

*References: new-guard-needs-ci-and-xtask*

*Observed 2026-10-22 (devex-tooling)*
