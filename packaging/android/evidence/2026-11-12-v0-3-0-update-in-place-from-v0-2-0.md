<!-- SPDX-License-Identifier: Apache-2.0 -->
# Device run: v0.3.0 updated in place over v0.2.0, then onboarded, 2026-11-12

Plan §22's post-cut case, run as architect-cto's ruling of 2026-11-08
leaves it (REPLY 01a1ba06-bafa) and as its request of 2026-11-12 asks
(REQUEST 01a1cfb9-06d0): on a phone holding the published `v0.2.0`,
install the published `v0.3.0` APK in place, then take it through
onboarding. Run by p2p-network-dev-01 (j162), with the owner at the
phones, on the owner's word in the session to uninstall and clear the
app on both.

**Result.** On the Pixel 9a (API 37) and the Samsung A40 (API 30) the
published `v0.3.0` APK updates the published `v0.2.0` in place: the
installer answers `Success`, versionCode goes from 2 to 3 under one
signer, and the first-install time is kept. The updated app then
onboards: Create seals at its Continue and the start answers 0, which a
release build answers only once the identity record exists.

## Device and artefacts

| phone | Android | API | adb | lease |
|---|---|---|---|---|
| Pixel 9a | 17 | 37 | wireless debugging, 192.168.100.67, paired anew | `pixel-9a`, per adb call, 12:33Z to 12:38Z |
| Samsung A40 (SM-A405FN) | 11 | 30 | wireless debugging, 192.168.100.6, paired anew | `samsung-a40`, per adb call, 12:39Z to 12:45Z |

Every adb step ran on this login's own adb server, and no adb process
of this login remained after the run (`pgrep -u "$(id -u)" -a adb`
empty). Both phones' clocks print UTC+4 in `dumpsys` and logcat
(`16:36:56` below is 12:36:56Z); every time in prose is UTC.

The two APKs are the release assets, downloaded from the release pages
with `gh release download` and checked against their `.sha256` assets
(`sha256sum -c`, both `OK`); `v0.3.0`'s hash is also the one its
release page gives. `apksigner verify --print-certs` reports one signer
for both, v3 only, and `aapt2 dump badging` the versions:

| asset | SHA-256 | certificate SHA-256 | versionCode | versionName |
|---|---|---|---|---|
| `interweave-0.2.0-arm64-v8a.apk` | `5aadbfca9cc80a5d…` | `a7f38b5e32aec4b8…` | 2 | 0.2.0 |
| `interweave-0.3.0-arm64-v8a.apk` | `3b89be89c161fd4e…` | `a7f38b5e32aec4b8…` | 3 | 0.3.0 |

`a7f38b5e…` is the certificate the `v0.3.0` release page names as the
same for every release. On the phones `dumpsys package` prints that
signer as `520af074`.

**What the phones held before.** The request expected the test-signed
release build of 4c4cf3587 that the 2026-11-08 record left on both. Neither
phone held it; each held a debuggable `org.interweave.human` 0.3.0
(versionCode 3) of another signer, and `org.interweave.human.test`:

| phone | signer (`dumpsys`) | first installed | last updated |
|---|---|---|---|
| Pixel 9a | `f8b1b43e` | 2026-11-08 16:04:53Z | 2026-11-11 22:07:38Z |
| Samsung A40 | `26d1eb18` | 2026-11-12 09:20:37Z | 2026-11-12 12:31:01Z |

Both packages were uninstalled on each phone, with their data, before
the run.

## Steps

The steps were one host-side script per phone (kept outside the tree),
under that phone's lease: the state before; `adb uninstall` of both
packages; `adb install` of `v0.2.0`; its activity started once with
`am start -W` (Welcome, nothing tapped: a `v0.2.0` release build cannot
seal an identity, plan §22) and stopped after five seconds with
`am force-stop`; `adb install -r` of `v0.3.0`; the state after. Then
`logcat -c`, the activity started, and the owner took Create to
"Identity created" (C4) and its Continue. The onboarding window is
`FLAG_SECURE` and a release build carries no `ui-trace`, so the screens
are as the owner read them; the host read the log and `dumpsys`.

**Pixel 9a.** `v0.2.0` installed at 12:34:38Z (`Success`): versionCode
2, signer `520af074`, first install and last update 16:34:38.
`v0.3.0` over it with `-r` at 12:35:01Z:

```text
Performing Streamed Install
Success
firstInstallTime=2026-11-12 16:34:38
lastUpdateTime=2026-11-12 16:35:01
signatures=PackageSignatures{e28ad4 version:3, signatures:[520af074], past signatures:[]}
versionCode=3 minSdk=30 targetSdk=36
versionName=0.3.0
```

Started at 12:35:26Z; the owner reached C4 and its Continue went on
with no error shown:

```text
11-12 16:36:56.430 25691 25938 I interweave: the runtime started with the networks: 5 addresses (0 on no local network), 3 resolvers, strict Private DNS off
11-12 16:36:56.430 25691 25938 I interweave: start answered 0
```

`dumpsys activity services` then listed
`org.interweave.human/.NetworkService` in the app's process, created
at about 12:36:55Z and bound by the activity (`startForegroundCount=0`).

**Samsung A40.** `v0.2.0` installed at 12:42:10Z (`Success`):
versionCode 2, signer `520af074`, first install and last update
16:42:10. `v0.3.0` over it with `-r` at 12:42:49Z:

```text
Performing Streamed Install
Success
firstInstallTime=2026-11-12 16:42:10
lastUpdateTime=2026-11-12 16:42:49
signatures=PackageSignatures{684c1d9 version:3, signatures:[520af074], past signatures:[]}
versionCode=3 minSdk=30 targetSdk=36
versionName=0.3.0
```

Started at 12:43:09Z; the owner reached C4 and its Continue went on
with no error shown:

```text
11-12 16:44:52.426 22030 25130 I interweave: the runtime started with the networks: 8 addresses (0 on no local network), 3 resolvers, strict Private DNS off
11-12 16:44:52.426 22030 25130 I interweave: start answered 0
```

`dumpsys activity services` then listed `.NetworkService`, created
about 23 seconds before 12:45:13Z, and `startForegroundCount=0`.

Neither log carries "an identity write was refused" or "could not
save" after its `logcat -c`.

## What it shows

- The published `v0.3.0` APK updates the published `v0.2.0` in place on
  both phones: the installer's answer is `Success`, versionCode 2 to 3,
  one signer before and after, the first-install time unchanged.
- A phone so updated onboards a new user: Create sealed at its Continue
  and the service started on what it wrote (`start answered 0`), on
  the release key's build, which the 2026-11-08 record could only show
  on a test-signed build of the same fix.

## What it does not show

- **The same PeerId from the same store across an update.** `v0.2.0`
  holds no identity, so nothing crossed this update but the package and
  whatever `v0.2.0`'s first start wrote. By §22's baseline rule that
  half runs from `v0.3.0` to its successor, and stays a limit of the
  Validate row until then.
- That an identity made by `v0.3.0` is reachable from a peer: the start
  says the runtime is up, not that a peer met that PeerId.
- Restore on the release key's build: only Create ran here (Restore ran
  on the test-signed build of 2026-11-08).
- What the person saw after Continue past the owner's reading, the
  window being `FLAG_SECURE`, and whether the service runs in the
  foreground once the activity leaves: `dumpsys` was read with the
  activity in front.
- Who installed the debug builds found on the phones; nothing here
  depends on it.

## Left on the phones

Both carry the published `v0.3.0` (versionCode 3, release signer) with
an identity made by Create in this run, which no record keeps. A build
under any other signer needs an uninstall first.
