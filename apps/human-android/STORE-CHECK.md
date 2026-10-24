<!-- SPDX-License-Identifier: Apache-2.0 -->
# Release check: the Android client's dependencies

Plan §20 step 10, as ADR-0057 decision 5 sets it (2026-10-19). The
Android client is listed on no store: it is distributed from the
project's own release page (ADR-0057 decision 1), and what that page
carries is `RELEASE.md` beside this file. This check lists every
dependency the application ships with its licence, asserts that no
Google Play services, Firebase or Play artifact is on the application's
classpath (ADR-0057 decision 2), and records SPIKE-008's target-SDK
half. Its file name is the store check's, which other records cite.

What it does not settle is named where it stands. Re-read every number
here against the tree when it is cited for a release: the graph moves
with each lockfile change.

## The application

From `android/app/build.gradle.kts` and `android/app/src/main/AndroidManifest.xml`:

| | |
|---|---|
| applicationId | `org.interweave.human` |
| minSdk / targetSdk / compileSdk | 30 / 36 / 36 |
| versionCode / versionName | 1 / the workspace version (ADR-0065 decision 5: the root `Cargo.toml`'s `[workspace.package] version`, today `0.1.0`); versionCode is a committed counter raised for every published APK (`RELEASE.md`) |
| permissions | `INTERNET`, `ACCESS_NETWORK_STATE`, `FOREGROUND_SERVICE`, `FOREGROUND_SERVICE_REMOTE_MESSAGING`, `POST_NOTIFICATIONS`, `USE_BIOMETRIC` |
| foreground service | `.NetworkService`, `foregroundServiceType="remoteMessaging"`, not exported |

## What the application ships, with its licences

Measured 2026-10-19 on `feat/j111-android-recovery` (Slint 1.18.1,
skia-bindings 0.153.3, as on `main`).

### The native library's Rust graph

`cargo deny --manifest-path apps/human-android/Cargo.toml --target
aarch64-linux-android list --layout license`: 440 crates, build-time
crates (`slint-build`, `i-slint-compiler`, `bindgen` and their
dependencies) included, so the list over-counts what the APK holds.
Every crate offers at least one of the licences below; an expression
with several is the crate's own choice of terms.

| crates | licence expression offered |
|---:|---|
| 234 | Apache-2.0 / MIT |
| 89 | MIT |
| 37 | Apache-2.0 |
| 25 | Unicode-3.0 |
| 10 | GPL-3.0-only / LicenseRef-Slint-Royalty-free-2.0 / LicenseRef-Slint-Software-3.0 (Slint) |
| 9 | BSD-3-Clause |
| 9 | Apache-2.0 / MIT / Zlib |
| 6 | MIT / Unlicense |
| 3 | CC0-1.0 |
| 3 | BSD-3-Clause / MIT |
| 2 each | Zlib; ISC; Apache-2.0 / Apache-2.0 WITH LLVM-exception / MIT; Apache-2.0 / BSD-3-Clause; Apache-2.0 / BSD-3-Clause / MIT |
| 1 each | 0BSD / Apache-2.0 / MIT; BSD-2-Clause; Apache-2.0 / ISC; Apache-2.0 / BSL-1.0; Apache-2.0 / MIT / Unicode-3.0 |

Slint's ten crates on Android are `slint`, `slint-build`,
`slint-macros`, `i-slint-core`, `i-slint-core-macros`,
`i-slint-common`, `i-slint-compiler`, `i-slint-backend-selector`,
`i-slint-backend-android-activity` and `i-slint-renderer-skia`. They are
used under the Royalty-free licence (the owner, 2026-10-01; kept for
Android by ADR-0057 decision 3), whose attribution `RELEASE.md`
carries. **The licence check sees them:** `deny.toml`'s `[graph]
targets` is `x86_64-unknown-linux-gnu` and `aarch64-linux-android`
(since #314, 2026-10-19), so one `cargo deny check` judges the phone's
graph with the desktop's, and its Royalty-free exceptions name
`i-slint-backend-android-activity` and `i-slint-renderer-skia` beside
the desktop's crates (ADR-0057 decision 3). The check this file owed
before the first release, `cargo deny --target aarch64-linux-android
check licenses`, is that run: `tools/checks/check_dependencies.sh`, in
CI's `tree checks` job. Its `unused-license-exception = "deny"` fails
the check if the Android target leaves the list, since the two
exceptions would then match nothing (`test_check_dependencies.sh`
plants it).

### The native Skia library

On Android Slint draws with Skia, which `skia-bindings` 0.153.3
downloads as a prebuilt archive pinned by the crate's version
(`github.com/rust-skia/skia-binaries`, release 0.153.3, the
`aarch64-linux-android-ganesh-gl-jpegd-jpege-pdf-vulkan` build;
ADR-0057 decision 4 pins it by a committed SHA-256 for the release run,
A 2026-10-20; a source build is that decision's revisit). What it bundles, read from its
`libskia.a` (1095 objects, grouped by the build target each came from):

| component | built in as | licence | notice owed |
|---|---|---|---|
| Skia, with skcms | `core`, `gpu`, `pdf`, `pathops`, `libskcms` and the rest | BSD-3-Clause (`LICENSE_SKIA` in the archive) | yes |
| libjpeg-turbo | `libjpeg`, `libjpeg12`, `libjpeg16` | IJG License, BSD-3-Clause, and the zlib License for parts of its SIMD code (its LICENSE.md) | yes |
| FreeType 2 | `libfreetype2` | FreeType License, of its two (LICENSE.TXT); the other is GPL-2.0 | yes, the credit line the FTL requires |
| libpng | `libpng` | PNG Reference Library License v2 | yes |
| zlib (Chromium's, with its SIMD parts) | `libzlib`, `zlib_*` | zlib (1.2.12's LICENSE) | yes |
| Expat | `libexpat` | MIT | yes |
| Wuffs | `libwuffs` | Apache-2.0 | yes |
| Vulkan Memory Allocator | `vulkanmemoryallocator` | MIT | yes |
| cpu_features | `libcpu-features`, compiled from the NDK's `sources/android/cpufeatures` | BSD-2-Clause (its file header, NDK 28.2.13676358) | yes |

The objects were read from the archive, and each licence from the
component's own file at the commit Skia's `DEPS` pins for
`rust-skia/skia` tag `m153-0.101.2` (the archive's build), read
2026-10-19 (cpu_features from the NDK the build uses). What was not
done is diffing the archive's objects against a build from that source.
The notice texts go with the APK and on the release page
(`RELEASE.md`).

### The Java side

The release runtime classpath (`android/app/gradle.lockfile`, the
`releaseRuntimeClasspath` entries) holds two artifacts, both from Maven
Central:

| artifact | licence |
|---|---|
| `org.jetbrains.kotlin:kotlin-stdlib:2.2.10` | Apache-2.0 |
| `org.jetbrains:annotations:13.0` | Apache-2.0 |

## No Google Play services on the application's classpath

ADR-0057 decision 2. Checked 2026-10-19 over all three lockfiles
(`android/app/gradle.lockfile`, `android/buildscript-gradle.lockfile`,
`android/settings-gradle.lockfile`): no `com.google.android.gms`,
`com.google.firebase`, `com.google.android.play` or
`com.android.billingclient` artifact on any configuration. The
`com.google` artifacts present are on the build's own classpath
(`buildscript-gradle.lockfile`'s `classpath`: the Android Gradle
plugin's `com.google.auto.value`, `code.findbugs`, `code.gson`,
`crypto.tink`, `dagger`, `errorprone`, `flatbuffers`, `guava`,
`j2objc`, `jimfs` and `protobuf` groups, and
`com.googlecode.juniversalchardet`),
on `androidLintTool` and on the test platform, never on a runtime
classpath. On every change it is held by
`tools/checks/check_android_no_play_services.sh`, the lockfile guard
ADR-0057 decision 2 names (run by `cargo xtask checks` and CI); this
section is the reading of 2026-10-19.

## No address book, and no platform backup

ADR-0059 decision 6's Android half and ADR-0060 decision 8: the app
declares none of `READ_CONTACTS`, `WRITE_CONTACTS` or `GET_ACCOUNTS`
(the permissions row above), reaches no contacts provider or picker
and ships no contacts library; its manifest sets
`android:allowBackup="false"` with no backup agent, and
`res/xml/data_extraction_rules.xml` excludes every domain from both
`cloud-backup` and `device-transfer`. On every change it is held by
`tools/checks/check_android_contacts_and_backup.py`, over the manifests,
the extraction rules, the Gradle lockfiles and scripts, and the Kotlin
and Rust sources of the app (run by `cargo xtask checks` and CI). The
rules are declared in the main manifest alone, and every shipped source
set's copy of the rules file is read, in `res/xml` and in each qualified
`res/xml-*` directory, since a variant's replaces main's at the resource
merge and a qualified one is the copy the platform picks.

## SPIKE-008's target-SDK half

ADR-0041's condition is that half alone (ADR-0057 decision 6): the
`remoteMessaging` foreground service working under the target SDK the
app declares, on a device of an API level that enforces it. It closes
with plan §20 step 3's device run, now on record in
`device-runs/2026-10-20-gate-b.md` ("Pixel 9a, API 37, build
35c71169"): on the Pixel 9a at API 37, an app declaring targetSdk 36
ran `.NetworkService` in the foreground with `types=0x00000200`
(`remoteMessaging`) on the `reachable` channel, kept it foreground in
the background, and restarted it as a foreground service after a
process death with no person present. That record is the run this
half waited for, and SPIKE-008's verdict in
`architecture/roadmap/SPIKES.md` records it: its "Decision taken"
paragraph ends with 2026-10-20, the target-SDK half measured by that
run and ADR-0041's condition met.
p2p-network-dev-02's six android-e2e cases ran on the same phone and
build (`tests/android-e2e/runs/2026-10-20-pixel-9a-api37.md`).

## Why F-Droid was dropped (record)

ADR-0056 (2026-10-19, superseded by ADR-0057 the same day) made F-Droid
the only store. Writing this check against F-Droid's Inclusion Policy
(read 2026-10-19) found two blockers, reported as OBSERVATION 01a153b1:

- The policy requires every application to be FLOSS by DFSG, FSF, GNU
  or OSI. Slint's ten crates are admitted under the Royalty-free
  licence, which none of them counts; Slint's one FLOSS route is
  GPL-3.0-only, which would make the distributed APK a GPLv3 work.
- The policy allows prebuilt binaries only from named sources, and the
  Skia library arrives as a download from a GitHub release of
  rust-skia. Building it from source was measured as far as it went:
  cloning Skia at `m153-0.101.2` took 28 s, its `git-sync-deps` 56 s
  for a 1.1 GB tree, and `SKIA_SOURCE_DIR` turns the bindings'
  build offline; the build itself was stopped when the decision came.

The owner chose "Drop F-Droid for now" and to keep Slint's Royalty-free
terms (ADR-0057). The GPL route and Skia from source are that record's
revisit.

## The Play half, withdrawn (record)

What this file held before ADR-0056, kept as what was read: the Play
policy's foreground-service declaration and how the client stood
against it. ADR-0041's Play-policy half is withdrawn (its amendment of
2026-10-19), no Play declaration is written, and nothing below binds a
build.

### The policy, as read on 2026-10-19

**Play Console Help, "Understanding foreground service and full-screen
intent requirements"** (`support.google.com/googleplay/android-developer/answer/13392821`):

> When your apps target Android 14 and above, you'll need to declare any
> foreground service types that you use in a new declaration on the App
> content page (Monitor and improve > App content) in Play Console.

For each type: a description of the functionality, the user impact if
the task is deferred or interrupted by the system, a link to a video
demonstrating the feature, and a use case chosen from a list that the
page calls "non-exhaustive", or entered by hand.

Its row for `TYPE_REMOTE_MESSAGING`:

> Relay text communication to another device
>
> Assists without continuity of a user's messaging tasks when they
> switch devices. For example, allow users to send text messages via a
> web client by scanning a QR code and establishing a connection
> between the web client and the phone. Not limited to SMS.
>
> Show the user initiate the syncing or transfer of text messages and
> transferring between the phone and an external web client or device.

**Device and Network Abuse policy, "Permissions for Foreground Services
(FGS)"** (`support.google.com/googleplay/android-developer/answer/13315670`):

> Apps are only allowed to declare a foreground service permission if
> the use:
>
> - provides a feature that is beneficial to the user and relevant to
>   the core functionality of the app
> - is initiated by the user or is user perceptible (for example, audio
>   from playing a song, cast media to another device, accurate and
>   clear user notification, user request to upload a photo to the
>   cloud)
> - can be terminated or stopped by the user
> - can't be interrupted or deferred by the system without causing a
>   negative user experience or causing the user anticipated feature to
>   not work as intended (for example, a phone call needs to start
>   immediately and can't be deferred by the system)
> - runs only for as long as necessary to complete the task

**Android Developers, "Foreground service types"**
(`developer.android.com/develop/background-work/services/fgs/service-types`,
page last updated 2026-10-16), Remote messaging:

> Transfer text messages from one device to another. Assists with
> continuity of a user's messaging tasks when they switch devices.

### Where the client stood

| Criterion | The client | Holds? |
|---|---|---|
| Beneficial and core | Receiving messages is the product's core | yes |
| Initiated by the user or perceptible | Stay reachable is the person's switch; an ongoing notification shows while it runs | yes |
| Can be stopped by the user | The same switch, in the app; the notification itself has no stop action (`NetworkService.kt`) | yes |
| Cannot be deferred or interrupted without harm | A deferred service misses messages sent meanwhile | yes |
| Runs only as long as necessary | It runs for as long as the person wants to stay reachable, which has no task end | **contested**: declared as an open-ended, person-ended task (ADR-0041 A 2026-10-19) |
| Type fits `remoteMessaging` | Play describes the type as relaying the person's OWN messages to another of their devices; this service receives OTHER people's messages | **gap**: the type stands on the platform's definition (ADR-0041 A 2026-10-19) |

Two rows do not clearly hold, and they are the condition ADR-0041 left
open. A hand-entered use case is allowed by the declaration form, so the
gap is not a refusal on its face. It is also not evidence of acceptance:
how Play's review judges a P2P messenger's declaration cannot be
measured from here.

### The ruling then

ADR-0041 A 2026-10-19 (architect-cto, REPLY 01a15282, commit 432a354f
on InterWeave #301), recorded here as ruled:

- `remoteMessaging` stands on the platform's definition: a text message
  is transferred from one device (the peer's) to another (this phone),
  and this service is what receives it. The gap is in Play's example,
  not in the type.
- The Play declaration states the use case by hand: the service keeps
  the person reachable for text messages from their peers' devices
  while the app is in the background, with no server to hold them;
  deferral means the person is unreachable.
- Play's acceptance of that declaration happens at the store submission
  (plan §22). This check closes the target-SDK half of SPIKE-008's P1;
  **the Play half stays open by name** until then. No build claims
  Play-store background reachability before acceptance.
- If Play refuses: `specialUse` with its
  `PROPERTY_SPECIAL_USE_FGS_SUBTYPE` explanation. ADR-0041's revisit
  fires only if Play refuses both.

- The duration criterion ("runs only for as long as necessary to
  complete the task", folded into the same amendment, e47f5fcb): the
  declaration states the task as it is, reachable for as long as the
  person keeps Stay reachable on, ended by them, by the app's stop or by
  the OS, because an open-ended, person-ended task is what serverless
  reachability is. The criterion binds `specialUse` equally, so the
  fallback answers only the type question; this criterion is the
  likeliest ground for a refusal.

The declaration's and the demo video's wording are written with the
store listing in plan §22, with language-culture.

Builds distributed outside Play are not affected by this check.
