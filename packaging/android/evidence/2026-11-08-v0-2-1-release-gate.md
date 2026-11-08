<!-- SPDX-License-Identifier: Apache-2.0 -->
# Device run: the v0.2.1 release gate before the cut, 2026-11-08

The device half of plan §22's v0.2.1 gate that can run before the cut:
the RELEASE build of `main` makes an identity on a fresh install
(Create) and takes one back from a recovery phrase (Restore), on both
phones, and the service starts on what each wrote. v0.1.0 and v0.2.0
cannot do the first (`2026-11-05-pixel-9a-update-and-downgrade.md`).
Run by p2p-network-dev-01 (j152, step 1 of fabric-coordinator's phone
order of 2026-11-08), with the owner at the phones, on the owner's word
to uninstall and clear app data on both.

**Result.** Create and Restore are met on the Pixel 9a (API 37) and the
Samsung A40 (API 30). The update in place from v0.2.0 is **not run**: it
needs the v0.2.1 APK signed with the release key, which exists only
after the cut (architect-cto-01's ruling, option a), and it runs then
on the published asset.

## Device and artefacts

| phone | Android | API | adb | lease |
|---|---|---|---|---|
| Samsung A40 (SM-A405FN) | 11 | 30 | wireless debugging, 192.168.100.6 | `samsung-a40`, 12:11Z to 13:09Z |
| Pixel 9a | 17 | 37 | wireless debugging, 192.168.100.62 | `pixel-9a`, 13:09Z to 13:15Z |

Every adb step ran on this login's own adb server; no adb process of
this login remained after either phone (`pgrep -u "$(id -u)" -a adb`
empty). Both phones' clocks print UTC+4 in `dumpsys` and logcat
(`17:02:20` below is 13:02:20Z); every time in prose is UTC.

Before the run the A40 carried `org.interweave.human` 0.2.0
(versionCode 2, installed 2026-11-08 00:37 local) and
`org.interweave.human.test`; the Pixel a debuggable
`org.interweave.human` 0.2.0, the #578 debug build (installed
2026-11-07 15:30:31 local), with no test package. All were
uninstalled, with their profiles: a release signer does not install
over another.

The APK is the RELEASE variant of `main` at 4c4cf358, built here with
the steps of `release.yml`'s build job, in this order: the pinned Skia
prebuilt fetched and its SHA-256 checked (`tools/ci/skia_prebuilt.sh
fetch`); `./gradlew :app:assembleRelease` on the SDK's pinned JDK, which
ran `verifyReleaseNative` (BUILD SUCCESSFUL); after the build,
`tools/ci/skia_prebuilt.sh check target`, its profile defaulting to
`release`, which answered "skia-bindings took the pinned archive … (1
output(s) under target/aarch64-linux-android/release/build)"; then
`tools/ci/check_apk_classes.py --holds org.interweave.human.MainActivity
--holds-none org.interweave.human.test` (1143 classes, passed). The
build script's own class-check call lacked those arguments and stopped
on its usage line, so that last check was run by hand with
`release.yml`'s arguments on the same unsigned APK. The target was this
clone's, not a clean one; the check found one skia-bindings output in
it. The release key is not this login's, so the unsigned APK was
aligned and signed with a test-only key made for this run
(`CN=InterWeave TEST ONLY j152`):

| APK SHA-256 | certificate SHA-256 | signer on the phones | versionCode | versionName | debuggable |
|---|---|---|---|---|---|
| `dfd81bbe3f373601…` | `f9f821606c3c4c05…` | `c5062b9e` | 2 | 0.2.0 | no |

Its versionName reads 0.2.0 because the workspace version is the
cut's to change. It was signed with v1 off and v2 and v3 asked for, as
the release run signs. `apksigner verify` at the APK's own minimum (30)
reported v3 true and v2 false, which at that minimum it reports even
when the v2 block is there (`tools/ci/release_android_apk.sh`); the run
did not verify at `--min-sdk-version 24`, so whether the v2 block was
written is not shown. Neither phone needs it: both are API 30 or
above.

The recovery phrase is a TEST-ONLY one, minted for this run on the host
with `transportctl identity backup` in a profile under an isolated
`XDG_*` tree, and never written to the repository or a message. Its
identity is `12D3KooWD53MjpV3oBoGyjHkEb4pzLeuLzLz4essnFttDUSHieR3`. The
host checked the phrase against it, with a control:

```text
$ transportctl identity verify --expected-peer-id 12D3KooWD53M…ieR3 < phrase
verified: the phrase restores 12D3KooWD53MjpV3oBoGyjHkEb4pzLeuLzLz4essnFttDUSHieR3
$ (the same phrase, words 1 and 2 swapped)
the mnemonic has an invalid checksum            exit 1
```

The exact string sent to the phones was checked the same way and gave
the same PeerId.

## Steps

The onboarding window is `FLAG_SECURE` and a release build carries no
`ui-trace`, so the host sees neither the screens nor their state. The
owner tapped through them and read the screens aloud; the host typed
the phrase, cleared app data and read the log. Before each Continue
that seals, the log was cleared (`logcat -c`), so the lines quoted are
that Continue's.

Restore's word screen (R2) has no text field: its keypad takes letters
from a hardware keyboard and fills a word only on Enter, a space being
ignored (`crates/human/ui-slint/ui/app.slint`, the identity page:
`capture-key-pressed` fills on Enter while typing and swallows a space,
`key-pressed` takes the letters). The phrase was typed one word at a time, `adb shell
"input text <word>; input keyevent 66"`. A first attempt that sent the
24 words in one `input text` filled the first word only.

**A40, Create** (13:02Z). The owner went through Create to "Identity
created" (C4) and its Continue. The first attempt at Restore had
reached Create instead (two different random PeerIds, no phrase
entered), so this case came out of it:

```text
11-08 17:02:20.848  3553 18753 I interweave: the runtime started with the networks: 5 addresses (0 on no local network), 3 resolvers, strict Private DNS off
11-08 17:02:20.848  3553 18753 I interweave: start answered 0
```

**A40, Restore** (13:05Z to 13:08Z). `pm clear org.interweave.human`
at 13:02:43Z, then Welcome → Restore → R2; the 24 words typed
13:05:07Z–13:06:39Z. R3 showed `12D3KooW … DUSHieR3` as the owner read
it (first 8 and last 8). Its Continue:

```text
11-08 17:08:22.032 14887 20090 I interweave: the runtime started with the networks: 5 addresses (0 on no local network), 3 resolvers, strict Private DNS off
11-08 17:08:22.033 14887 20090 I interweave: start answered 0
```

**Pixel, Create** (13:10Z to 13:12Z). Installed at 13:10:04Z; the owner
went through Create to C4 and its Continue:

```text
11-08 17:11:59.935 23632 23833 I interweave: the runtime started with the networks: 4 addresses (0 on no local network), 3 resolvers, strict Private DNS off
11-08 17:11:59.935 23632 23833 I interweave: start answered 0
```

**Pixel, Restore** (13:12Z to 13:15Z). `pm clear` at 13:12:14Z; the 24
words typed 13:12:46Z–13:13:03Z. R3's PeerId matched the first 12 and
last 6 characters given (`12D3KooWD53M`, `SHieR3`), as the owner read
it. Its Continue:

```text
11-08 17:14:58.480 23984 24522 I interweave: the runtime started with the networks: 4 addresses (0 on no local network), 3 resolvers, strict Private DNS off
11-08 17:14:58.480 23984 24522 I interweave: start answered 0
```

No case logged "an identity write was refused". A release start
answers `start answered 0` only once the identity record exists
(`apps/human-android/src/jni.rs`: a release first run is
`FirstRun::Refuse`, answered `refused::NO_IDENTITY`), so each seal
succeeded at its Continue.

## What it shows

| case | Samsung A40, API 30 | Pixel 9a, API 37 |
|---|---|---|
| a fresh install of the release build makes an identity (Create) and starts | yes | yes |
| the release build takes an identity back from a phrase (Restore) and starts | yes | yes |
| R3 shows the PeerId the host derives from the same phrase | yes, `12D3KooW…DUSHieR3` | yes, first 12 and last 6 |

## What it does not show

- The release KEY: the APK was test-signed. What the release key's
  signing adds is checked by the release run
  (`tools/ci/release_android_apk.sh`); it changes nothing the native
  library does.
- The update in place from v0.2.0 onto v0.2.1, and that a phone so
  updated then onboards: it needs v0.2.1's published APK, after the
  cut.
- That the restored identity is reachable from a peer: the start says
  the runtime is up, not that a host peer dialled the phone and met
  that PeerId.
- The whole PeerId on R3: the owner read a fragment, at both ends, and
  the A40's reading was 8 and 8 characters.
- What the person saw after Continue, beyond the next screen's title
  ("Connect to someone"): the window is `FLAG_SECURE`.

## What is left on the phones

Both phones carry this test-signed release of 4c4cf358 with the TEST
identity restored, `12D3KooWD53M…ieR3`. A debug or release-signed build
installs over neither: it needs an uninstall first.
