<!-- SPDX-License-Identifier: Apache-2.0 -->
# Device run: the same two cases on the A40, API 30, 2026-11-05

The Pixel 9a run of the same day
([`2026-11-05-pixel-9a-update-and-downgrade.md`](2026-11-05-pixel-9a-update-and-downgrade.md))
on the project's second test phone, at the app's `minSdk`: the previous
release's APK updated in place by this one, and an older APK refused
over a newer one. Run by p2p-network-dev-01 (j141), on the owner's word
to uninstall the phone's debug build, once p2p-network-dev-02 (j17) and
rust-ui-dev-02 (j90) had each said they needed nothing on it kept.

**Result.** As on the Pixel: the refusal is met, the update in place is
met for the package and **not run** for the PeerId and the store. A
fresh install of v0.1.0 refuses to save a new identity here too, so the
defect plan §22 records under "The published Android releases cannot
onboard a new user" is not particular to API 37.

## Device and artefacts

Device: **Samsung Galaxy A40** (SM-A405FN), Android 11, **API 30**
(build RP1A.200720.012.A405FNXXU4CWC3), over wireless debugging at
192.168.1.11, on this login's own adb server, under the `samsung-a40`
lease (taken for each of the two steps below, 14:07:42Z and 15:30:20Z;
adb disconnected and its server stopped after the last, no adb process
of this login remained). The phone's clock read UTC to within two
seconds (`adb shell date -u`), and `dumpsys` and logcat print local
time, UTC+1: `15:07:56` below is 14:07:56Z.

Before the run the phone carried a debuggable `org.interweave.human`
0.2.0 (versionCode 2, first installed 2026-11-03 13:38:09 local, signer
digest shown as `26d1eb18`) and `org.interweave.human.test`, as the
`before` lines of the first step's output show (quoted below); by
p2p-network-dev-02's word, not shown here, it held an identity made
that morning for its cases. A release
APK cannot be installed over another signer, so both were uninstalled,
with their profile.

The APKs are the published release assets, downloaded with
`gh release download` and checked against their `.sha256`:

| release | file | SHA-256 | versionCode |
|---|---|---|---|
| v0.1.0 | `interweave-0.1.0-arm64-v8a.apk` | `f08e101fc7892719c5ab3c685700d337c209fe098aa8595223133fcbb18b541e` | 1 |
| v0.2.0 | `interweave-0.2.0-arm64-v8a.apk` | `5aadbfca9cc80a5dd2d420b6e0e3fe04cab4f6bc0400692c485fa28dbb67c2c9` | 2 |

The same files the Pixel run installed (its table's prefixes). The
package manager's signature id for both is `520af074`, the Pixel's, the
same before and after the update.

## The steps

```text
== 2026-11-05T14:07:42Z phone clock: 2026-11-05T14:07:44Z
== 2026-11-05T14:07:42Z before
package:org.interweave.human.test versionCode:0
package:org.interweave.human versionCode:2
    firstInstallTime=2026-11-03 13:38:09
    lastUpdateTime=2026-11-05 08:21:25
    pkgFlags=[ DEBUGGABLE HAS_CODE ALLOW_CLEAR_USER_DATA ]
    signatures=PackageSignatures{ab6dc4b version:2, signatures:[26d1eb18], past signatures:[]}
    versionCode=2 minSdk=30 targetSdk=36
    versionName=0.2.0
== 2026-11-05T14:07:43Z uninstall test pkg
Success
== 2026-11-05T14:07:44Z uninstall app
Success
no interweave package
== 2026-11-05T14:07:46Z install v0.1.0 (fresh)
Performing Streamed Install
Success
exit=0
    firstInstallTime=2026-11-05 15:07:56
    lastUpdateTime=2026-11-05 15:07:56
    pkgFlags=[ HAS_CODE ALLOW_CLEAR_USER_DATA ]
    signatures=PackageSignatures{9426449 version:3, signatures:[520af074], past signatures:[]}
    versionCode=1 minSdk=30 targetSdk=36
    versionName=0.1.0
```

The owner then went through Create by hand (the onboarding window is
`FLAG_SECURE`). "Identity created" (C4) showed; it comes before the
seal, which C4's Continue asks for (`ui-model`'s `identity_flow.rs`).
Continue answered as on the Pixel, and the app's log, cleared at
14:11:09Z, says:

```text
11-05 15:12:44.885   384  3407 I interweave: android_main: the Activity's views start
11-05 16:29:32.230   384  3407 I interweave: an identity write was refused: Storage("the profile: the profile document cannot be read: No such file or directory (os error 2)")
```

The update in place and the downgrade:

```text
== 2026-11-05T15:30:20Z phone clock: 2026-11-05T15:30:22Z
== 2026-11-05T15:30:20Z install -r v0.2.0
Performing Streamed Install
Success
exit=0
    firstInstallTime=2026-11-05 15:07:56
    lastUpdateTime=2026-11-05 16:30:28
    pkgFlags=[ HAS_CODE ALLOW_CLEAR_USER_DATA ]
    signatures=PackageSignatures{9426449 version:3, signatures:[520af074], past signatures:[]}
    versionCode=2 minSdk=30 targetSdk=36
    versionName=0.2.0
== 2026-11-05T15:30:30Z install v0.1.0 over v0.2.0, no -d
Performing Streamed Install
adb: failed to install interweave-0.1.0-arm64-v8a.apk: Failure [INSTALL_FAILED_VERSION_DOWNGRADE]
exit=1
== 2026-11-05T15:30:37Z install -r v0.1.0 over v0.2.0, no -d
Performing Streamed Install
adb: failed to install interweave-0.1.0-arm64-v8a.apk: Failure [INSTALL_FAILED_VERSION_DOWNGRADE]
exit=1
== 2026-11-05T15:30:46Z after
    firstInstallTime=2026-11-05 15:07:56
    lastUpdateTime=2026-11-05 16:30:28
    signatures=PackageSignatures{9426449 version:3, signatures:[520af074], past signatures:[]}
    versionCode=2 minSdk=30 targetSdk=36
    versionName=0.2.0
```

(The APK path adb printed is shortened to the file's name; Android 11
gives the failure without the reason text API 37 adds.)

## What it shows

| case | result |
|---|---|
| an older release refused over a newer one | **Met.** v0.1.0 over v0.2.0 is refused with `INSTALL_FAILED_VERSION_DOWNGRADE`, with and without `-r` (no `-d`), and v0.2.0 stays installed at its 15:30:28Z update. |
| the previous release updated in place, the package | **Met.** `install -r` of v0.2.0 over v0.1.0 succeeds: versionCode 1 to 2, `firstInstallTime` unchanged (15:07:56 local), the same signer. |
| the same PeerId from the same store after it | **Not run.** No identity could be made on v0.1.0, so there was no PeerId or store to keep. By §22's baseline rule it runs from the first published release that onboards a new user to the release after it, never against v0.2.0. |

## What this does not show

- The PeerId-and-store half of the update in place, as above.
- Create on v0.2.0 at API 30: it was not tried here. The Pixel run
  tried it at API 37 and it refused the same way; the code path is the
  same in both releases.
- Restore of an identity on a release build.

After the run the phone carries release v0.2.0 with no identity.
