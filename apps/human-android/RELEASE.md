<!-- SPDX-License-Identifier: Apache-2.0 -->
# Releasing the Android client

Plan §20 step 10's release metadata, as ADR-0057 sets it: the Android
client is listed on no store and is distributed from the project's own
release page, installed and updated by hand. The tagged release run
that builds and signs the APK is Stage 19's (plan §22); this file says
what that run produces and what the page carries. What the APK ships,
and under which licences, is `STORE-CHECK.md`.

## The APK

| | |
|---|---|
| application id | `org.interweave.human`, for the app's whole life: a person's update installs over the same id and key, and a new id is a new app beside the old one, its data not carried |
| version name | the release tag's version, `MAJOR.MINOR.PATCH` (today `0.0.0`, unreleased) |
| version code | strictly increasing per release, never reused: Android refuses an update whose code is not higher than the installed one |
| ABI | `arm64-v8a` only, as `android/app/build.gradle.kts` builds |
| file | `interweave-<version>-arm64-v8a.apk` |

## Signing

**Proposed, for the owner to confirm with Stage 19:**

- **Scheme.** APK Signature Scheme v2 and v3 (`apksigner`), no v1 JAR
  signature: the app's minimum SDK is 30, and v3 lets a later key
  rotation keep existing installs updatable.
- **One key for the app's life.** An update installs only if it is
  signed by the key the installed app carries, so a lost key strands
  every install: a person would have to uninstall, which deletes the
  app's data, the identity's record included.
- **Where it is kept: never in the repository**, nor in a CI log or an
  artifact. It lives in the project's encrypted secrets store, readable
  by the release run alone, with an offline backup the owner holds. The
  debug key each developer's build uses is not it, and a debug build is
  never published.
- **What the page shows of it:** the signing certificate's SHA-256
  fingerprint, the same on every release, printed as `apksigner verify
  --print-certs` prints it (lowercase hex, no colons), so a person can
  check a download against it as well as against its file hash.

Creating the key and wiring the release run's Gradle signing config
(read from the run's environment, never from a committed file) are
Stage 19's.

## The release page

`release/release-page.md.in` is the template the release run fills,
one page per release. It carries what ADR-0057 decision 1 names:

- the APK and its SHA-256, and the signing certificate's fingerprint;
- the release notes;
- the manual-install and manual-update note: the app has no update
  channel and never checks for updates itself;
- the Slint attribution the Royalty-free licence owes, as the badge on
  the page the binaries are downloaded from (ADR-0057 decision 3);
- the third-party notices the APK's native library owes
  (`STORE-CHECK.md`, "The native Skia library"), FreeType's credit line
  among them, which the release run fills from the licence files of the
  sources it built: the template carries none of those texts itself.

The page's English is final (language-culture-en, 2026-10-19); the
facts in it are this file's. Where a person finds the next release has
no stable address yet: the release run names it (Stage 19), and the
page then says to come back there.
