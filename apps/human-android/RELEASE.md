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
| version name | the release version, `MAJOR.MINOR.PATCH[-pre-release]` with no build metadata (ADR-0065): produced by `android/app/build.gradle.kts` from the root `Cargo.toml`'s `[workspace.package] version` (today `0.1.0`, unreleased), never typed by hand; the build refuses a workspace version outside that grammar, and its `verify<Variant>VersionName` fails an APK whose merged manifest says otherwise. The tag is `v` and that version, and `release_android_apk.sh` refuses an APK whose `versionName` is not the tag's |
| version code | not a version (ADR-0065 decision 5): a counter committed in `android/app/build.gradle.kts`, raised by one for every APK published, pre-releases included, never reused and never derived from the version name: Android refuses an update whose code is not higher than the installed one |
| ABI | `arm64-v8a` only, as `android/app/build.gradle.kts` builds |
| file | `interweave-<version>-arm64-v8a.apk` |

## Signing

**Confirmed** (ADR-0066, 2026-10-25, the release's signatures): the
scheme, the one key and what the page shows of it are what `v0.1.0`'s run
signed with on 2026-10-25; where the key lives is
`packaging/android/README.md`, "The signing key"; the tag's and the
Linux archive's signatures are ADR-0066's.

- **Scheme.** APK Signature Scheme v2 and v3 (`apksigner`), no v1 JAR
  signature: the app's minimum SDK is 30, and v3 lets a later key
  rotation keep existing installs updatable.
- **One key for the app's life.** An update installs only if it is
  signed by the key the installed app carries, so a lost key strands
  every install: a person would have to uninstall, which deletes the
  app's data, the identity's record included.
- **Where it is kept: never in the repository**, nor in a CI log or an
  artifact. It lives in the `release` environment of the release run,
  read by its signing job alone; keeping the key backed up is the owner's
  instruction in `packaging/android/README.md`. The
  debug key each developer's build uses is not it, and a debug build is
  never published.
- **What the page shows of it:** the signing certificate's SHA-256
  fingerprint, the same on every release, printed as `apksigner verify
  --print-certs` prints it (lowercase hex, no colons), so a person can
  check a download against it as well as against its file hash.

The key was made by the owner before the first release, and the run
signs the unsigned APK Gradle builds with `apksigner` through
`tools/ci/release_android_apk.sh`, the key read from the run's
environment and never from a committed file; no Gradle signing config
exists (`packaging/android/README.md`, "The signing key").

## The release page

`release/release-page.md.in` is the template the release run fills,
one page per release, and the one page serves EVERY platform the release
ships: the APK and the Linux archive are downloaded from it, each in its
own section (architect-cto's ruling of 2026-10-20 on Stage 19). For the
APK it carries what ADR-0057 decision 1 names:

- the APK and its SHA-256, and the signing certificate's fingerprint;
- the release notes;
- the manual-install and manual-update note: the app has no update
  channel and never checks for updates itself;
- the Slint attribution the Royalty-free licence owes, as the badge on
  the page the binaries are downloaded from (ADR-0057 decision 3), once
  for the APK and the desktop binaries alike; `tests/release_page.rs`
  fails a template without it, or with a badge from a Slint release
  other than the one `Cargo.lock` builds;
- the third-party notices the APK's native library owes
  (`STORE-CHECK.md`, "The native Skia library"), FreeType's credit line
  among them, which the release run fills from the licence files of the
  sources it built: the template carries none of those texts itself.

For the Linux archive it carries the file, its SHA-256, its minisign
signature's public key and verify command (ADR-0066: the `.minisig` is
an asset beside the archive), what it contains, and the install and
update notes; the
archive's name and contents are packaging/linux's, a proposal until that
lands, and the run fills `{linux_archive}`, `{linux_archive_url}` and
`{linux_sha256}`, and `{linux_minisign_public_key}` from the
`signing-values.json` that `tools/ci/release_sign_archive.sh --values`
writes once both signatures verify.

The page's English is final (language-culture-en, 2026-10-19 for the
Android text, 2026-10-20 for the shared page and the Linux section); the
facts in it are this file's. Where a person finds the next release has
no stable address yet: the release run names it (Stage 19), and the
page then says to come back there.
