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

A release is a tag `v` and a Semantic Versioning 2.0.0 version on `main`,
`v<MAJOR>.<MINOR>.<PATCH>[-<pre-release>]`: pre-release identifiers are
admitted (`v0.4.0-rc.1`, published as a pre-release), build metadata is
not (ADR-0065 decisions 1 and 6; `tools/ci/release_tag.sh`). The workflow
that runs on it is devex-tooling's `.github/workflows/release.yml`;
every step it runs is
a script in `tools/ci/` with a self-test beside it, so the workflow
holds no release logic of its own. In order:

1. **The version is already committed.** The root `Cargo.toml`'s
   `[workspace.package] version` is the tag's version, and the APK's
   `versionName` is produced from it by
   `apps/human-android/android/app/build.gradle.kts`; `versionCode`
   there is raised by one in a commit before the tag, higher than every
   earlier published APK's (ADR-0065 decision 5); step 5 refuses an APK
   whose `versionName` is not the tag's.
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
   `THIRD-PARTY-NOTICES-android.md` and their `.sha256` files, the
   archive's and its `.sha256`'s `.minisig`, as the release of the tag,
   after a provenance attestation of the APK and the archive.

Around those steps, the run holds the tag and the archive to the keys
`packaging/release-keys.toml` pins ("The release keys" below): before
anything builds, the tag must be signed by the pinned tag key
(`tools/ci/release_verify_tag.sh`); the sign job checks it again with
main's verifier and main's pin before the release environment is read,
then signs the archive and its `.sha256` with the archive key and
verifies both signatures with main's pinned public key
(`tools/ci/release_sign_archive.sh`). Step 6 builds in the glibc floor's
image, ubuntu:22.04 (glibc 2.35), every program held to that floor, and
a further job starts each program in a clean image of it
(`tools/ci/release_linux_run.sh`; `packaging/linux/README.md`).

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
and a required reviewer -- is what an edited copy has to get past, and
it holds only as far as the reviewer checks the commit before approving
(step 5 below).

### Making the key and the environment, once

Nothing hands the key over: before the first release it does not
exist, and the owner makes it, in the owner's own account and never an
agent's (an agent login's home is readable by every session of that
agent). It is kept for good, backed up with its passwords: Android
installs an update only over an APK signed by the same key, so a lost
key means no later release installs over the ones already on phones.
A release whose `release` environment is still empty fails at step 5
with `the release environment has no INTERWEAVE_RELEASE_KEYSTORE_BASE64`
and publishes nothing.

Run steps 1 and 2 outside any checkout. They write one file, the
`.jks`, which `.gitignore` covers in case.

1. Make the keystore (`keytool` ships with any JDK; on Fedora,
   `java-21-openjdk-headless`), and answer its prompts:

   ```sh
   keytool -genkeypair -v -keystore interweave-release.jks -alias interweave \
       -keyalg RSA -keysize 4096 -validity 10000
   ```

2. Encode it straight to the clipboard, never to a file or the
   terminal (`wl-copy` on Wayland, `xclip -selection clipboard` on X11;
   neither is installed by default on Fedora: `wl-clipboard`, `xclip`):

   ```sh
   base64 -w0 interweave-release.jks | wl-copy
   ```

3. In the repository's Settings → Environments → `release` (New
   environment, if no release has run yet), add the environment
   secrets: `INTERWEAVE_RELEASE_KEYSTORE_BASE64` (the clipboard),
   `INTERWEAVE_RELEASE_KEY_ALIAS` (`interweave` above),
   `INTERWEAVE_RELEASE_KEYSTORE_PASS`, and `INTERWEAVE_RELEASE_KEY_PASS`
   only if the key's password differs. Leave
   `INTERWEAVE_RELEASE_CERT_SHA256` unset. Keep the `.jks`.
4. On the same page, the environment variable
   `INTERWEAVE_RELEASE_FIRST` = `true`; under deployment branches and
   tags, only the tag pattern `v*`; the owner as a required reviewer;
   and "Allow administrators to bypass configured protection rules"
   unticked.
5. Run the release: a new tag, or "Re-run failed jobs" on a run that
   failed at step 5 within 7 days (the built APK and archive are kept
   that long; a later re-run fails at their download, and a new tag
   needs the version and `versionCode` raised first, step 1 above).
   **Approving is the key's only guard**, so check before you approve.
   The agents here push and tag as the owner's own GitHub account, so
   neither the `v*` rule nor the reviewer tells an agent's tag from the
   owner's, and the run's workflow is the tagged commit's own copy.
   Approve only when the commit the approval page names is on `main`
   (`git fetch && git merge-base --is-ancestor <sha> origin/main`
   exits 0): then its workflow and scripts are main's, reviewed, and
   not an edited copy.
6. The signing job's log prints `cert_sha256 <hex>`. Record that value
   as the secret `INTERWEAVE_RELEASE_CERT_SHA256` and delete the
   variable `INTERWEAVE_RELEASE_FIRST`: from then on a run signed with
   any other key fails.

## The release keys

Two more keys, beside the APK's: the **tag key**, which signs each
release tag, and the **archive key**, which signs the Linux archive.
Both are the owner's, made in the owner's own account and never an
agent's, for the keystore's reason above; the release run holds every
release to their public halves, pinned in `packaging/release-keys.toml`
on `main`. That file lands empty, and until both are pinned every
release fails closed: the tag job refuses the tag, the sign job the
archive. A public half is not secret: the owner may hand it to an agent
to commit, and a changed pin is a pull request to `main` like any
other, reviewed before it merges. The agents here push and approve as
the owner's GitHub account, so that review, and the tag signed by a key
no agent holds, are what tell the owner's release from an agent's.

### The tag key, once

An Ed25519 OpenPGP key that signs and does nothing else, made with the
owner's gpg (the address is the owner's choice; `never` is no expiry
date, and a key given one must be re-pinned when it is extended, or
every tag after the date is refused):

```sh
gpg --quick-gen-key "InterWeave release tags <you@example.org>" ed25519 sign never
```

Its fingerprint, the pin's `[tag] fingerprint`, and its public block,
the pin's `[tag] public_key`:

```sh
gpg --list-keys --with-colons "InterWeave release tags" | awk -F: '$1 == "fpr" { print $10; exit }'
gpg --armor --export <fingerprint>
```

Back it up with gpg's `--export-secret-keys`, kept off the agents'
accounts. A lost tag key costs no install: a new one is made, pinned by
a pull request, and signs the next tag.

### Signing a release tag

The tag is the owner's act, made by hand in the owner's own clone (an
agent prepares the version commit on `main` and hands over the line,
filled in). The tag message is the release notes, so the tag is
annotated, and it must be the commit `main` holds:

```sh
git fetch origin && git tag -s -u <fingerprint> -m "<release notes>" v<version> <sha on main>
git push origin v<version>
```

v0.1.0's tag is signed with the owner's personal key, which every agent
account also signs with; it is recorded as the gap these keys close,
not as the policy.

### The archive key, once

A minisign key (`dnf install minisign` on Fedora, or the release run's
pinned build: `tools/ci/minisign_install.sh <dir>`), made outside any
checkout; it asks for a password:

```sh
minisign -G -p interweave-archive.pub -s interweave-archive.key
```

In the `release` environment (Settings → Environments → `release`), the
secret `INTERWEAVE_RELEASE_MINISIGN_KEY` holds the whole `.key` file,
copied straight to the clipboard (`wl-copy < interweave-archive.key`, or
`xclip -selection clipboard < interweave-archive.key`), and
`INTERWEAVE_RELEASE_MINISIGN_PASS` its password. The `.pub` file's
second line is the pin's `[archive] minisign_public_key`. Keep the
`.key` backed up with its password; a lost archive key is replaced like
the tag key, by a new key and a new pin, and a person checking an older
archive checks it against the pin of its time.

## What this does not prove

- The prebuilt Skia is the same bytes on every run, not reviewed bytes
  (ADR-0057 decision 4).
- Nothing here installs the APK on a phone: a release's update and
  reinstall behaviour is plan §22's Validate row, read from a device
  run (§21's S12).
- The self-tests sign minimal APKs with throwaway keys made at test
  time; the release key and a real `assembleRelease` APK meet the
  scripts only in the run itself. The tag and archive checks are tested
  the same way, with throwaway gpg and minisign keys.
- A provenance attestation names the repository, the workflow, the
  commit and the run that built a file; it does not show the build is
  reproducible, nor that the workflow it names is `main`'s: a tag of an
  edited commit runs that commit's copy, which only the environment's
  reviewer and the tag key stand in front of.
