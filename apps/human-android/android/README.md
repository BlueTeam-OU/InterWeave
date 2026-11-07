# Android packaging

The Gradle build of the Android human client: one app module, `app/`,
whose `cargoNdkDebug`/`cargoNdkRelease` tasks build the native library
(`apps/human-android`) with cargo-ndk before it is packaged. Kotlin here
is the platform's glue only: the Activity and the foreground Service hand
their lifecycle to the native library, and domain logic, retention,
transport policy and crypto stay in Rust crates.

Every plugin and library version is in `gradle/libs.versions.toml`;
Gradle itself is pinned by the wrapper, whose distribution checksum is the
one in `tools/host/android/android-toolchain.pins`.

Android instrumented tests belong under `app/src/androidTest/`;
host-orchestrated Android/network E2E scenarios belong in
`tests/android-e2e/`.

## Debug-build test hooks

The debug variant's own source set, `app/src/debug/`, carries hooks a
device run fires from adb where it needs something the phone cannot do
on demand (the unlock note's section 8,
`design/2026-11-05-unlock-policy-pending.md`); the release variant's
`app/src/release/` registers none. A run that uses one names it in its
record.

- **Restart, as the platform would**
  (`adb shell am broadcast -a org.interweave.human.debug.RESTART -p org.interweave.human`):
  the running Service stops the runtime and runs the start a platform
  restart under Stay reachable runs, with no person asking for it, in
  the same process, so its state (a declined Unlock prompt included)
  is kept. A key past its user-presence window refuses that start for
  real; what the person then sees is the unlock note's L3 or L4. The
  Service registers the receiver while it runs, asking the sender for
  `DUMP`, which adb's shell holds; `logcat -s interweave` shows its
  `restart hook:` lines.
- **Refuse the store once, as needing recovery**
  (`adb shell am broadcast -a org.interweave.human.debug.REFUSE_STORE -p org.interweave.human`):
  the next start to reach the store -- the restart hook's, a launch's,
  Try again's -- is refused as a store that needs recovery, with its
  files left as they are, so the recovery screen shows (A16's discard).
  Its Discard is a real one: the plan reads the files there and the
  confirmed discard removes them, and the restart after it opens a new
  store. Under user presence the restart hook's start prompts as the
  unlock note says; fire it within the 60 s after unlocking the phone
  for the refusal to reach the store with no prompt first. The native
  entry is the debug library's alone; `verifyReleaseNative` fails a
  release library that names it. `logcat -s interweave` shows its
  `store hook:` line.
