<!-- SPDX-License-Identifier: Apache-2.0 -->
# Device run: v0.4.0 updated in place over an onboarded v0.3.0, 2026-11-23

The half of plan §22's post-cut case that the 2026-11-12 record left
open ("the same PeerId from the same store across an update"), run
from `v0.3.0` to its successor as §22's baseline rule places it: on a
Pixel 9a holding the published `v0.3.0` with an identity made by
Create, install the published `v0.4.0` APK in place and read the PeerId
again. Run by p2p-network-dev-01 (j195, architect-cto's REQUEST
01a20011), with the owner at the phone; the uninstall with data that
starts it is architect-cto's DECISION 01a20039, for this run.

**Result.** On the Pixel 9a (API 37) the published `v0.4.0` APK updates
an onboarded published `v0.3.0` in place: the installer answers
`Success`, versionCode goes from 3 to 4 under one signer, the
first-install time is kept, and the updated app starts its runtime on
what `v0.3.0` wrote without onboarding again. The PeerId the app shows
is the same before and after:
`12D3KooWHWLEh2AmeF9uD9M7fEuxHpjHeAwzPRDPXfpjst5wqA62`.

## Device and artefacts

| phone | Android | API | adb | lease |
|---|---|---|---|---|
| Pixel 9a | 17 | 37 | wireless debugging, 192.168.1.43, paired anew | `pixel-9a`, per adb call; the run's calls 20:29Z to 20:34Z |

Every adb step ran on this login's own adb server, under the lease;
the server was stopped after the run and no adb process of this login
remained (`pgrep -u "$(id -u)" -a adb` empty). The phone's clock prints
CET (UTC+1) in `dumpsys` and logcat (`21:32:35` below is 20:32:35Z);
every time in prose is UTC.

The two APKs are the release assets, checked against their `.sha256`
assets (`sha256sum -c`, both `OK`, before and after the run).
`apksigner verify --print-certs` reports one signer for both, the
release certificate `a7f38b5e32aec4b8…` (in full in
`packaging/android/README.md`), and `aapt2 dump badging` the versions:

| asset | SHA-256 | versionCode | versionName |
|---|---|---|---|
| `interweave-0.3.0-arm64-v8a.apk` | `3b89be89c161fd4e…` | 3 | 0.3.0 |
| `interweave-0.4.0-arm64-v8a.apk` | `7166e88f06700ea2…` | 4 | 0.4.0 |

In full, `v0.4.0`'s is
`7166e88f06700ea2a45cd0a99423a879d9292e19987f4055574a6d54a4076d5c`.
On the phone `dumpsys package` prints the release signer as
`520af074`, as the 2026-11-12 record found.

**What the phone held before.** `org.interweave.human` 0.3.0
(versionCode 3), and `org.interweave.human.test` (signer `18390359`,
first installed 2026-11-15), which this run did not touch. The app was
uninstalled with its data before the run; its signer before the
uninstall was not read.

## Steps

**`v0.3.0`, onboarded.** `adb install` of `v0.3.0` at 20:29:06Z
(`Success`): versionCode 3, first install and last update 20:29:06Z.
The activity was started from the launcher intent; the owner took
Create and its Continue, and read the PeerId from the app:
`12D3KooWHWLEh2AmeF9uD9M7fEuxHpjHeAwzPRDPXfpjst5wqA62`. The log:

```text
11-23 21:31:05.901 14740 14943 I interweave: the runtime started with the networks: 2 addresses (0 on no local network), 2 resolvers, strict Private DNS off
11-23 21:31:05.901 14740 14943 I interweave: start answered 0
11-23 21:31:46.331 14740 14943 I interweave: stopped: session closed true, runtime Ok(0)
```

**`v0.4.0` over it.** `adb install -r` of `v0.4.0` at 20:32:34Z:

```text
Performing Streamed Install
Success
versionCode=4 minSdk=30 targetSdk=36
versionName=0.4.0
lastUpdateTime=2026-11-23 21:32:34
firstInstallTime=2026-11-23 21:29:06
signatures=PackageSignatures{cbe4014 version:3, signatures:[520af074], past signatures:[]}
```

The activity was started from the launcher intent at once, by the host,
with nothing tapped, and the new process started its runtime on the
store `v0.3.0` left:

```text
11-23 21:32:35.368 15094 15144 I interweave: android_main: the Activity's views start
11-23 21:32:35.606 15094 15155 I interweave: the runtime started with the networks: 2 addresses (0 on no local network), 2 resolvers, strict Private DNS off
11-23 21:32:35.606 15094 15155 I interweave: start answered 0
```

The owner then opened the PeerId in the app and read
`12D3KooWHWLEh2AmeF9uD9M7fEuxHpjHeAwzPRDPXfpjst5wqA62`, the same as
before the update. The onboarding window is `FLAG_SECURE`
(`dumpsys window` lists `SECURE` on `MainActivity`), and a release
build is not debuggable (`run-as` refuses it), so both PeerIds are as
the owner read them on the phone; the host read the log and `dumpsys`.

## What it shows

- The published `v0.4.0` APK updates an onboarded published `v0.3.0` in
  place on a Pixel 9a: `Success`, versionCode 3 to 4, one signer
  before and after, the first-install time unchanged.
- The updated app starts on what `v0.3.0` wrote, with no onboarding:
  `start answered 0` within a second of its first start, before
  anything was tapped, which a release build answers only once the
  identity record exists.
- The PeerId shown is the same before and after the update: the
  identity crossed it.

## What it does not show

- The Samsung A40. It was paired and found holding `v0.4.0`
  (versionCode 4) already; nothing was installed or run on it here.
- That the PeerId reached a peer, before or after: the start says the
  runtime is up, not that a peer met that PeerId.
- Restore across the update: only Create ran.
- What else the person saw after the update past the owner's reading,
  the window being `FLAG_SECURE`; no notice was reported to the host.
- Why the runtime stopped about 20 seconds after each start (`stopped:
  session closed true`); both stops answered cleanly, and the start
  after the update came from a fresh process.
- An update from a `v0.3.0` holding conversations or contacts: the
  store held only the identity Create made.

## Left on the phone

The Pixel 9a carries the published `v0.4.0` (versionCode 4, release
signer) with the identity made by Create in this run, which no record
keeps, and the untouched `org.interweave.human.test`.
