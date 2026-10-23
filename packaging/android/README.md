<!-- SPDX-License-Identifier: Apache-2.0 -->
# android

The Android release run (ADR-0057, plan §22): how a tag becomes the
signed APK, its SHA-256 and the release page. What the APK is — its id,
versions, signing scheme, the page's text — is
`apps/human-android/RELEASE.md`, and what it ships under which licences
is `apps/human-android/STORE-CHECK.md`; the app's Gradle sources stay
under `apps/human-android/android/`. There is no store and no update
channel: a person installs and updates the APK by hand, from the page.

## The release run

A release is a tag `v<MAJOR>.<MINOR>.<PATCH>` on `main`. The workflow
that runs on it is devex-tooling's `.github/workflows/release.yml`;
every step it runs is
a script in `tools/ci/` with a self-test beside it, so the workflow
holds no release logic of its own. In order:

1. **The version is already committed.** `versionName` in
   `apps/human-android/android/app/build.gradle.kts` is the tag's
   version and `versionCode` is higher than every earlier release's;
   step 5 refuses an APK whose `versionName` is not the tag's.
2. **Skia, pinned.** `tools/ci/skia_prebuilt.sh fetch <dir>` downloads
   the one archive `tools/ci/skia-binaries.pin` names, checks its
   SHA-256, and prints `SKIA_BINARIES_URL=file://…` for the build's
   environment (ADR-0057 decision 4, A 2026-10-20).
3. **The build.** `./gradlew assembleRelease` in
   `apps/human-android/android`, with that variable set: Gradle runs
   `cargo ndk … build --locked --release` for `arm64-v8a`, checks that
   the release library carries no e2e entry, and writes
   `app/build/outputs/apk/release/app-release-unsigned.apk`.
4. **Skia, checked.** `tools/ci/skia_prebuilt.sh check <target-dir>`
   (the workspace's `target/`) fails the run unless the bindings took
   the pinned file, so a silent fallback to a source build is a red run.
5. **Signing.** `tools/ci/release_android_apk.sh <version>
   <unsigned.apk> <out-dir>` first refuses a `<version>` that is not
   the workspace version (ADR-0065: `release_tag.sh --manifest`, the
   Semantic Versioning grammar with no build metadata and the root
   manifest's `[workspace.package] version`, read with no cargo, since
   this job holds the key). It then refuses the APK unless it is
   `org.interweave.human` at that version, not debuggable, with
   native code for `arm64-v8a` alone; then aligns it, signs it with APK
   Signature Scheme v2 and v3 and no v1, verifies it, and refuses a
   signing certificate other than `INTERWEAVE_RELEASE_CERT_SHA256`. It
   writes `interweave-<version>-arm64-v8a.apk`, its `.sha256` and
   `android-values.json`.
6. **The Linux archive**, for the same page:
   `tools/ci/release_linux_archive.sh <version> <out-dir>`
   (`packaging/linux/README.md`).
7. **The notices.** `tools/ci/skia_notices.py <out.json>` fetches every
   licence text the APK's native Skia library owes, each checked
   against `tools/ci/skia-notices.pin`, with FreeType's credit line;
   `tools/ci/release_crate_notices.sh aarch64-linux-android
   interweave-human-android <version> <out-dir> <download-url>` writes
   the notices of every Rust crate compiled into the native library as
   `THIRD-PARTY-NOTICES-android.md`, its `.sha256`, and
   `crate-notices.json`: the file's URL for `{crate_notices}` and its
   hash for `{crate_notices_sha256}` (ADR-0058 decision 5, A 2026-10-22
   (ii): the notices are a release asset, too long for the page).
8. **The page.** `tools/ci/release_page.py
   apps/human-android/release/release-page.md.in <page.md>
   <run.json> android-values.json linux-values.json <notices.json>
   <crate-notices.json>`,
   `<run.json>` holding the version, the release notes and the two
   download URLs, the rest the files steps 5–7 wrote. It refuses a
   placeholder left unfilled, a value with no placeholder, and a value
   two files give. Then `tools/ci/check_release_body.sh <page.md>`
   refuses a page over the release-body limit, before anything is
   published.
9. **Publishing**: the page, the APK, the archive,
   `THIRD-PARTY-NOTICES-android.md` and their `.sha256` files, as the
   release of the tag.

## The signing key

The owner's, never in the repository, a CI log or an artifact
(`CLAUDE.md` §6, `RELEASE.md` "Signing"). It lives in the `release`
environment of `.github/workflows/release.yml`, whose `sign` job alone
reads it and builds nothing. The script reads it as environment
variables and hands the passwords to `apksigner` as `env:` references,
never on a command line:

| variable | holds |
|---|---|
| `INTERWEAVE_RELEASE_KEYSTORE` | the path of the keystore: the workflow writes the secret `INTERWEAVE_RELEASE_KEYSTORE_BASE64` (the keystore file, base64) to a file only that job reads, removed after it, and sets this to its path |
| `INTERWEAVE_RELEASE_KEY_ALIAS` | the key's alias |
| `INTERWEAVE_RELEASE_KEYSTORE_PASS` | the keystore's password |
| `INTERWEAVE_RELEASE_KEY_PASS` | the key's password, if it differs |
| `INTERWEAVE_RELEASE_CERT_SHA256` | the certificate's SHA-256, lowercase hex: the same on every release |

The first release runs step 5 with `--first-release` and no
`INTERWEAVE_RELEASE_CERT_SHA256` (the two together are refused), which
prints the fingerprint instead of checking it -- in the workflow, the
environment variable `INTERWEAVE_RELEASE_FIRST=true`, set for that run
only. The owner records the fingerprint as the
`INTERWEAVE_RELEASE_CERT_SHA256` secret, and every later run that signs
with any other key fails. The workflow's tag check runs from the tagged
commit's own copy, so the environment's protection -- only `v*` tags,
and a required reviewer -- is what an edited copy cannot get past.

## What this does not prove

- The prebuilt Skia is the same bytes on every run, not reviewed bytes
  (ADR-0057 decision 4).
- Nothing here installs the APK on a phone: a release's update and
  reinstall behaviour is plan §22's Validate row, read from a device
  run (§21's S12).
- The self-tests sign minimal APKs with throwaway keys made at test
  time; the release key and a real `assembleRelease` APK meet the
  scripts only in the run itself.
