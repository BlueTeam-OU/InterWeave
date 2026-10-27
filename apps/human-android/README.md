# human-android

The first-party Android human client (plan section 20): the native
library the APK loads, `interweave_human_android`, and the Gradle project
that packages it, [`android/`](android/README.md). The library's Activity
side draws the shared ui-slint views on Slint's NativeActivity backend;
its Service side hosts the embedded transport runtime
(`crates/transport/embedded`) through `crates/human/android-platform`. It
names no Slint crate itself: the toolkit is reached through ui-slint.

**Current status:** Stage 17 (plan section 20) closed 2026-10-22 on its closing record, which names what the stage did not prove; the release APK ships since v0.1.0 (ADR-0057, `packaging/android`). The
network service hosts the embedded runtime, the store and the facade;
the Activity attaches to it; unread arrivals raise a count-only
notification. The person creates an identity or restores one from its
recovery phrase on first run, and the profile's seed is wrapped by an
AndroidKeyStore key (`src/keystore.rs`, `KeystoreCipher.kt`), behind a
user-presence prompt where the profile asks for one. The debug APK
runs on a stand-in profile. Sessions run on phones: the device runs
are recorded under `device-runs/` (a Samsung A40 at API
30 and the Pixel 9a at API 37, from 2026-10-20) and the android-e2e
cases' runs under `tests/android-e2e/runs/`. Each record names what it
did not show; the plan's closing record for section 20 carries what
the stage did not prove, by name.

Build, on a host set up by `tools/host/android` (its README):

```sh
cd apps/human-android/android
echo "sdk.dir=$ANDROID_HOME" > local.properties   # not committed
PATH="$HOME/.cargo/bin:$PATH" JAVA_HOME=/opt/android-sdk/jdk ./gradlew assembleDebug
```

Dependency check, run locally by whoever changes the Gradle graph, before
the push that carries the change (plan section 20; no CI job runs it). It
reads the Gradle lockfiles against OSV.dev, so after a dependency change
regenerate them first:

```sh
cd apps/human-android/android
JAVA_HOME=/opt/android-sdk/jdk ./gradlew dependencies --write-locks   # only after a graph change
osv-scanner scan source --config=osv-scanner.toml -r .
```

It exits 0 when no known vulnerability is left, 1 when one is (the table
names each, with the lockfile it came from), and any other code for a
scan that did not run. `-r .` finds every module's `gradle.lockfile` and
the build's `buildscript-gradle.lockfile`; `--config` makes the one
[`osv-scanner.toml`](android/osv-scanner.toml) beside the build govern
them all, and that file says what an accepted finding's entry must carry.
The PR body says the check ran and what it found. osv-scanner 2.6.0 is
what this was written against.

