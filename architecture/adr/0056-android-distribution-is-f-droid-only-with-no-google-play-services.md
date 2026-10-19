# Android distribution is F-Droid only, and the app uses no Google Play services

**Status:** Superseded by ADR-0057 (2026-10-19, the same day: F-Droid dropped for now on Slint's licence route and Skia's prebuilt binary; the no-Google-Play-services rule of decision 2 stands there unchanged). Was: Accepted — the owner's word of 2026-10-19, ratified by the arming of #301

## Context

Until this record the Android human client was headed for Google Play by default: SPIKE-008's P1 row named "current Google Play foreground-service declaration/review requirements", ADR-0041's `remoteMessaging` category was "subject to SPIKE-008 target-SDK/Play-policy validation", plan §20 step 10 was a store-listing check quoting Play's policy, and ADR-0041's amendment of 2026-10-19 (earlier the same day) worked out what Play's declaration would say and what the fallback would be if Play refused it. The owner's decision ends that line: the app is distributed through F-Droid and nowhere else, and it uses no Google Play services.

What the app is today, read from `main`: its runtime dependencies name no Google Play services, Firebase or Play library; the `com.google` entries in `apps/human-android/android/app/gradle.lockfile` are lint and test tooling (guava, gson, error-prone annotations, jimfs, protobuf for the unified test platform), none on the application classpath; no FCM or cloud dependency was ever authorised (SPIKE-008's decision of 2026-10-07; ADR-0041: "no centralized push wake-up dependency"); backup to Google is off by the owner's decision of 2026-10-07; the native library is built from source by `cargo-ndk` in CI; `third_party/` is vendored source with recorded provenance (ADR-0051, ADR-0053, ADR-0054); first-party code and documentation are Apache-2.0 and every dependency's licence is FLOSS and on `deny.toml`'s allow-list (CLAUDE.md §8).

F-Droid's inclusion policy, read 2026-10-19: every application must be FLOSS; binary dependencies must come from source compilation or trusted repositories (Maven Central and Google Maven among those named; Rust, rustup and build tools not in Debian "can be acceptable"); "proprietary tracking or advertising libraries and analytics tools such as Google Play Services and Firebase and Crashlytics" are forbidden; F-Droid compiles from public source and verifies that the distributed binary matches it; no API keys are signed up for; an application must not download executable code without opt-in consent; anti-features are disclosed, not disqualifying; an app that fails to rebuild, such as one requiring Play Services, is rejected.

## Decision

1. **F-Droid is the only store.** The Android human client is published through F-Droid's repository, built by F-Droid's own infrastructure from this repository's public source at a tagged release, and is not listed on Google Play or any other store. Whether the project's own release page also offers the APK F-Droid built is Stage 19's packaging question (§22), never a second store.
2. **No Google Play services, in any form.** The application classpath carries no Google Play services, Firebase, Play Integrity, Play Billing, Play In-App Update, Play Install Referrer or any `com.google.android.gms`, `com.google.firebase`, `com.google.android.play` or `com.android.billingclient` artifact, and no code path depends on their presence on the device. Lint and test tooling may depend on what it depends on; the application may not. A guard over the Gradle lockfiles holds this (devex-tooling's to shape; the rule owes the test).
3. **F-Droid's inclusion policy binds the build.** Every dependency of the application is built from source or comes from a repository the policy names; the native library is built by F-Droid's recipe from this repository's source with the Rust toolchain the recipe pins; no prebuilt binary is committed for the app; no API key exists to include; the app downloads no executable code; any anti-feature is disclosed in the metadata rather than hidden. The build is reproducible to the degree the policy verifies: the reference is the APK the project's tagged release run produces from the same commit (Stage 19 defines that run, §22), and a divergence between F-Droid's build of the tag and that APK, or F-Droid's build failing, is a release blocker, not a note.
4. **ADR-0041's Play-policy half is withdrawn** (its amendment of 2026-10-19, "The Play-policy half is withdrawn"). The `remoteMessaging` foreground-service type stays, on the platform's definition and the target-SDK condition that step 3's device run closes; no Play declaration is written, no Play review is awaited, and `specialUse` is no longer a fallback for a store's refusal. Android's own enforcement of foreground-service types and permissions is unchanged and still binds.
5. **Plan §20 step 10 becomes F-Droid metadata**: the application's F-Droid metadata (summary, description, licence, the source and issue URLs, screenshots as the metadata structure takes them) under the app's directory, the draft build recipe for F-Droid's data repository, and a check that quotes the inclusion policy with the date read, lists every application dependency against the policy's repositories, and asserts decision 2 over the lockfile. Plan §22's `packaging/android` is the F-Droid release itself: the recipe submitted, the reproducibility verified, the release tag.

## Alternatives considered

Google Play as the primary store with F-Droid beside it (refused by the owner: one store, no Play). Google Play services for push wake-up (refused twice before, SPIKE-008 and ADR-0041, and forbidden by the policy). A self-hosted F-Droid repository instead of the main one (not decided; a later packaging question, since it changes who builds the binary). Keeping ADR-0041's Play half as a dormant option (a reader following it would write a declaration nobody will submit).

## Consequences

The store-listing work of §20 step 10 changes shape, not size: metadata and a recipe instead of a declaration and a demo video. The release gate gains F-Droid's build: a tagged release that F-Droid cannot rebuild from source is not a release. The app's dependency surface is now a policy boundary as well as a licence one (CLAUDE.md §8): a dependency that is proprietary, prebuilt from an unnamed source, or a Play library fails the build's inclusion. Nothing on the wire, in the contracts or in the human client's behaviour changes.

## Security implications

No Play services means no Play Integrity attestation and no Play-mediated update channel: the app's integrity rests on F-Droid's reproducible build and signature, and updates arrive through the F-Droid client the person chose. That is the trust the owner picked, and this record states it rather than implies it. The attack surface shrinks by the libraries that are not there.

## Operational implications

A F-Droid metadata recipe maintained alongside releases; the Rust toolchain pin and `cargo-ndk` named in it; a release checklist that waits for F-Droid's build result; no Play Console, no Play signing key, no Play policy calendar.

## Implementation implications

Plan §20 step 10 and §22 `packaging/android` as decision 5 says; the lockfile guard of decision 2; `human-client-packaging.md` §6's store-policy sentence inherits this record; SPIKE-008's P1 row reads as history. ADR-0041 amended ("The Play-policy half is withdrawn") in the same series.

## Revisit conditions

The owner choosing a second distribution channel; F-Droid's policy changing what it builds from (the Rust toolchain clause); a platform requirement that only a Play library can satisfy — then the requirement is dropped or ADR-0041 is revisited, never the library added quietly.

## Amendments

Full notes: [`history/0056-amendments.md`](./history/0056-amendments.md).

| Date | Amendment | Effect |
|---|---|---|
