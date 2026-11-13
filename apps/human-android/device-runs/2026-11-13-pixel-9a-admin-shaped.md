# Pixel 9a device run, 2026-11-13: TM-42, admin-shaped messages on the app's own client

Device: the **Pixel 9a** (product `tegu`, **API 37**) through
`.agent-fabric/bin/phone-run` under the `pixel-9a` lease, on this login's
own adb server, over wireless debugging at the owner's address. No adb
process of this login remained after any run (`pgrep -u "$(id -u)" -a adb`
empty). Every time below is UTC.

The case is TM-42's (`tests/SECURITY-MATRIX.md`):
`tests/android-e2e/tests/admin_deputy.rs::admin_shaped_messages_on_a_device_reach_the_facade_and_change_nothing`,
whose Android half is `interweave-android-e2e-cases`' `admin_shaped`, run
in the app's instrumentation on the app's own client (the facade, store
and hub the app's service composed). Until this run it had passed on the
stand-in only.

## The build

The debug APK and its androidTest APK of main at **`260a2a43c`**, built by
this login with `./gradlew assembleDebug assembleDebugAndroidTest` from
`apps/human-android/android` on the pinned JDK (`JAVA_HOME=/opt/android-sdk/jdk`),
the native library at the debug variant's settings (no `CARGO_PROFILE_DEV_*`
override: the case is not timed).

- app sha256 `6afbca865f0fa869aae9db34ece175779a17e67cb8f218958284f6c7bc32e7e9`
- androidTest sha256 `308f891fccaeeeda9d64eca3195bdd439180c5d12c21edc45c6f0c6e0cd9d95e`
- signer certificate SHA-256 `00a92280…` (this login's debug key)

The host side, `admin_deputy` and `target/debug/transport-daemon`, was
built from the same tree.

Main has moved since: `crates/human/android-platform` (the start rules,
the hub, its e2e `HubClient` the case drives, the view side) and `apps/human-android/src/activity.rs` changed after
`260a2a43c`. This run is of `260a2a43c`, not of those changes.

## The phone before the run

Probed before the build, the phone held the release app, versionName 0.3.0
(signer `520af074`, installed 2026-11-12). A debug key cannot update it,
so on the owner's word of 2026-11-13 ("you can uninstall") it was
uninstalled at 20:09Z, its app data with it (not read), and the debug
pair above installed. No androidTest package was installed before.
Keyguard not showing at the run's start (`isKeyguardShowing=false`).

## The run

```sh
ANDROID_SERIAL=192.168.100.69:<port> \
INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/org.interweave.human.test.E2eCase \
  cargo test -p interweave-android-e2e-tests --test admin_deputy -- --ignored --test-threads 1 --nocapture
```

**2026-11-13 21:13:52Z to 21:14:08Z: passed** (`1 passed`, 11.52 s in
the test). The test's line: `admin_deputy: close accepted, handed Some(String("5"))`.

What passing means, by the test and the case as written at `260a2a43c`:

- `AdbDevice::connect` read the app's `PeerId` through the `identity`
  case, force-stopped the app, read it again and found it the same.
- D, a `transport-daemon` on this host's loopback reached by the phone
  through `adb reverse`, sent the app's `human` endpoint the four raw
  admin-shaped payloads (trust a stranger, revoke `human`, add the
  stranger's address, shut down) and one `HumanChatV2` message carrying
  the trust request. Each was refused until the case had read its
  baseline and trusted D through the app's admin port, and D retried
  each until accepted; every acceptance named the `human` endpoint.
- The case reported all five handed to the facade (`handed` 5): the
  control that the would-be deputy got them.
- Read through the Android side's admin port, the endpoint leases and
  bootstrap addresses were as at the baseline, and the trust list
  differed only by D.
- The positive controls passed on the phone: a trust change, an address
  add and a revoke of `human`, each made through the admin port itself,
  were seen by the same reads.
- D's closing exchange was accepted.

## Attempts that did not reach the case

None of these ran the case; each is listed so the one pass above is
not read as the first try.

- 20:33Z: failed at the desktop's start, after `AdbDevice::connect` had
  run the `identity` case twice and force-stopped the app between them:
  `target/debug/transport-daemon is missing` (the host's daemon had not
  been built in this tree).
- 20:41Z: the wireless-debugging port had moved (`adb connect` failed),
  and the test waited on no device until the run's 1500 s timeout.
- Two further connects refused (a port gone, and once this host's adb
  server resetting its client) until the owner gave port 39655.

## What this run does not show

- The threat model's residual: code already running in the app's
  process calling its internals (threat-model.md, TM-42).
- A radio network: D reached the phone over adb's tunnel to this host's
  loopback, not over Wi-Fi or a relay.
- A person's hand: the case drives the app's client from the
  instrumentation, not through the UI.
- The release build: the case entry exists only in the debug native
  library, so a release APK cannot run it.
- The changes on main after `260a2a43c` named above.
