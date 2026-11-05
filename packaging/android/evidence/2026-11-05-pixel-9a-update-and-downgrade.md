<!-- SPDX-License-Identifier: Apache-2.0 -->
# Device run: a release updated in place, and an older one refused, 2026-11-05

The two cases the README's "Android update/reinstall behavior" row
names beside §21's S12 rows: the previous release's APK updated in
place by this one, serving the same PeerId from the same store, and an
older APK refused over a newer one. Run by p2p-network-dev-01 (j141),
on the owner's word to uninstall the phone's debug build first.

**Result.** The refusal is met. The update in place is met for the
package and **not run** for the PeerId and the store: neither release
can make an identity on a fresh install, so there was none to keep.
That is a defect of both releases, stated below.

## Device and artefacts

Device: **Pixel 9a**, Android 17, **API 37** (build CP3A.261005.005),
over wireless debugging at 192.168.1.31, on this login's own adb
server, under the `pixel-9a` lease (taken 12:05:16Z, released after the
last step; no adb process of this login remained). The phone's clock
read UTC at 12:05:36Z (`adb shell date -u`), and `dumpsys` and logcat
print local time, UTC+1: `13:05:41` below is 12:05:41Z.

Before the run the phone carried a debuggable `org.interweave.human`
0.2.0 (versionCode 2, first installed 2026-10-10 09:40:05 local,
signer digest shown as `f8b1b43e`) and `org.interweave.human.test`:
read from `dumpsys package` before the uninstall and not kept, so not
quoted below.
A release APK cannot be installed over another signer, so both were
uninstalled, with their profile.

The APKs are the published release assets, downloaded with
`gh release download` and checked against their `.sha256`:

| release | file | SHA-256 | certificate SHA-256 (`apksigner`) | versionCode |
|---|---|---|---|---|
| v0.1.0 | `interweave-0.1.0-arm64-v8a.apk` | `f08e101fc7892719…` | `a7f38b5e32aec4b8…` | 1 |
| v0.2.0 | `interweave-0.2.0-arm64-v8a.apk` | `5aadbfca9cc80a5d…` | `a7f38b5e32aec4b8…` | 2 |

The certificate column was read with `apksigner verify --print-certs`
and its output not kept; the value is the release key's (the README's
run above), and the
package manager's own signature id for both, `520af074`, is the same
before and after the update.

## The steps

```text
== 2026-11-05T12:05:36Z phone clock: 2026-11-05T12:05:36Z
== uninstall test pkg
Success
== uninstall app
Success
no interweave package
== 2026-11-05T12:05:37Z install v0.1.0 (fresh)
Success
    versionCode=1 minSdk=30 targetSdk=36
    versionName=0.1.0
    lastUpdateTime=2026-11-05 13:05:41
    signatures=PackageSignatures{f5c532a version:3, signatures:[520af074], past signatures:[]}
    pkgFlags=[ HAS_CODE ALLOW_CLEAR_USER_DATA ]
      firstInstallTime=2026-11-05 13:05:41
```

The owner then created a new identity by hand (the onboarding window
is `FLAG_SECURE`, so a screencap shows black), and the app answered
"the app could not save your identity". Its log:

```text
11-05 13:05:53.694  5036  5075 I interweave: android_main: the Activity's views start
11-05 13:07:02.799  5036  5075 I interweave: an identity write was refused: Storage("the profile: the profile document cannot be read: No such file or directory (os error 2)")
11-05 13:07:06.282  5036  5075 I interweave: an identity write was refused: Storage("the profile: the profile document cannot be read: No such file or directory (os error 2)")
```

The update in place and the downgrade:

```text
== 2026-11-05T12:09:15Z install -r v0.2.0
Success
    versionCode=2 minSdk=30 targetSdk=36
    versionName=0.2.0
    lastUpdateTime=2026-11-05 13:09:20
    signatures=PackageSignatures{f5c532a version:3, signatures:[520af074], past signatures:[]}
      firstInstallTime=2026-11-05 13:05:41
== 2026-11-05T12:09:20Z install v0.1.0 over v0.2.0, no -d
adb: failed to install interweave-0.1.0-arm64-v8a.apk: Failure [INSTALL_FAILED_VERSION_DOWNGRADE: Downgrade detected: Update version code 1 is older than current 2]
exit=1
== 2026-11-05T12:09:23Z install -r v0.1.0 over v0.2.0, no -d
adb: failed to install interweave-0.1.0-arm64-v8a.apk: Failure [INSTALL_FAILED_VERSION_DOWNGRADE: Downgrade detected: Update version code 1 is older than current 2]
exit=1
    versionCode=2 minSdk=30 targetSdk=36
    versionName=0.2.0
    lastUpdateTime=2026-11-05 13:09:20
```

The owner created a new identity again on v0.2.0, with the same answer:

```text
11-05 13:18:27.012  5446  5490 I interweave: an identity write was refused: Storage("the profile: the profile document cannot be read: No such file or directory (os error 2)")
```

## What it shows

| case | result |
|---|---|
| an older release refused over a newer one | **Met.** v0.1.0 over v0.2.0 is refused with `INSTALL_FAILED_VERSION_DOWNGRADE`, with and without `-r` (no `-d`), and v0.2.0 stays installed at its 12:09:20Z update. |
| the previous release updated in place, the package | **Met.** `install -r` of v0.2.0 over v0.1.0 succeeds: versionCode 1 to 2, `firstInstallTime` unchanged (13:05:41 local), the same signer. |
| the same PeerId from the same store after it | **Not run.** No identity could be made on either release, so there was no PeerId or store to keep. |

## The defect: a release build writes no first-run profile

A fresh install of either release cannot finish onboarding. In
`apps/human-android/src/jni.rs`, `fn provision` writes the first-run
profile only under the feature `dev-stand-ins`
(`stand_in::provision`); without it, it writes nothing. The Release
variant builds with `--release` and no features
(`apps/human-android/android/app/build.gradle.kts`), so no
`config.yaml` exists when onboarding seals the new identity, and the
seal refuses as logged. `stand_in.rs` says profile-config's
`provision_embedded` replaces the stand-in, and nothing calls it. The
code is the same at v0.1.0, v0.2.0 and `main` (read 2026-11-05).
Every device run before this one used a debug build, or updated over
one, which is why none met it. Reported to the app's lane
(rust-ui-dev-02, OBSERVATION 01a1aba6) and to architect-cto
(01a1aba7).
The plan records it in §22, "The published Android releases cannot
onboard a new user", with the baseline rule for the half not run here:
it runs from the first published release that onboards a new user to
the release after it, never against v0.2.0.

## What this does not show

- The PeerId-and-store half of the update in place. It needs a release
  that can make an identity, updated in place by the release after it.
- Restore of an identity on a release build. It goes through the same
  seal, so it is expected to fail the same way, but it was not tried.
- Any API other than 37, or the A40. The refusal case is the package
  manager's; the defect's path is not gated by API level.

After the run the phone carries release v0.2.0 with no identity.
