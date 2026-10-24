---
role: "rust-ui-dev"
class: workflow
topic: "android-check-needs-javac"
description: "a JDK 17 with javac is at /opt/android-sdk/jdk on develop-qzapp (not on PATH); set JAVA_HOME to it for Gradle and check_android_target.sh"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8491e3fe7d46a538
---

## a JDK 17 with javac is at /opt/android-sdk/jdk on develop-qzapp (not on PATH); set JAVA_HOME to it for Gradle and check_android_target.sh

On develop-qzapp, /usr/lib/jvm and PATH hold JRE 25 only (no javac), but the pinned Android JDK —
Temurin 17, javac included — is at `/opt/android-sdk/jdk` (gradle.properties names it). Corrected
2026-10-24: an earlier note here said the host had no JDK, which was wrong.

`JAVA_HOME=/opt/android-sdk/jdk ANDROID_HOME=/opt/android-sdk GRADLE_USER_HOME=<scratchpad>/gradle-home
./gradlew --no-daemon :app:<task>` in apps/human-android/android works (Gradle 9.6 + AGP downloaded,
~770 MB cache — remove it after). Ran verifyReleaseVersionName this way for j50.

**How to apply:** try JAVA_HOME=/opt/android-sdk/jdk before claiming an Android check cannot run here;
testable root logic still belongs in an every-target module (store_follow.rs, identity_keys.rs) so
`cargo test -p interweave-human-android` runs it on host.

*Anchors: `tools/ci/check_android_target.sh::needs a JDK`*

*Observed 2026-10-23 (rust-ui-dev)*
