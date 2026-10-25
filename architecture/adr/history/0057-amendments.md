# ADR-0057 — amendment history

### Amendment 2026-10-19 — `com.google.gms` is a fifth refused group

**Trigger.** devex-tooling built decision 2's guard (`tools/checks/check_android_no_play_services.sh`, InterWeave #307) as the decision named it and reported one group it therefore does not refuse: `com.google.gms`, the Google services Gradle plugin's group, which is how Firebase and Play services are usually wired into a build. The decision's four groups were the artifacts, not the plugin that pulls them in.

**Ruled.** The plugin's group joins the list; the guard refuses it on every application configuration as it refuses the four, the buildscript classpath included, since the decision exempts lint and test tooling only. No other rule changes; the Gradle lockfiles on `main` carry none of the five today.

### Amendment 2026-10-20 — The release run uses the Skia bindings' prebuilt archive, pinned by a committed SHA-256 that the android check job reads too; the silent source fallback is a failed run

**Trigger.** Decision 4 left to Stage 19 (§22) whether the release run builds Skia from source or pins the bindings' prebuilt archive by checksum, to be stated before the first release. p2p-network-dev-01 put the question with the costs (01a15a16-9bd2, 2026-10-20) after reading `skia-bindings` 0.153.3 as `Cargo.lock` pins it: on every Android build, including the android check job, `build.rs` downloads `skia-binaries-<key>.tar.gz` from the bindings' release page, where the key is the bindings repository's half hash, the target triple and the sorted feature set; it verifies nothing about the archive (`download.rs:218`, "TODO: verify key"); and on a download failure it falls back silently to a full source build, which `FORCE_SKIA_BINARIES_DOWNLOAD` cannot prevent for a registry crate. Verified in the registry source by architect-cto the same day.

**Ruled.** The prebuilt archive, pinned: the release script fetches the one archive itself, checks it against a SHA-256 committed in the tree, points `SKIA_BINARIES_URL` at it as a `file://` path (the only non-URL form the bindings read), and asserts the bindings' "DOWNLOAD AND INSTALL SUCCEEDED" line in the build output, so the silent fallback is a red run. The android check job reads the same committed pin, so one line governs both builds and a stale pin after a bump or a feature change fails rather than passes. Refused for now: a source build, whose toolchain (python3, ninja, the NDK's clang, git, Skia's gn and git-sync-deps fetching DEPS by full commit hash from many hosts) is a runner image devex-tooling would have to keep and whose time is tens of minutes per target, unmeasured here — the revisit if a store returns (the ADR's revisit conditions) or the owner wants no third-party binary in the APK. Neither option makes the bytes reviewed; the native library's notices are owed either way (plan §20 step 10).

**Propagation.** Decision 4; this note; the log row. The release script and its pin are p2p-network-dev's (the Android release run, §22); the check job's read of the pin is devex-tooling's (`.github/`). Plan §22's sentence that the choice is architect-cto's "on p2p-network-dev's question with the costs" is answered by this amendment and is reworded with the plan's next change.

### Amendment 2026-10-21 — The release page carries the notices of every crate compiled into the native library, under ADR-0058

**Trigger.** The review of #324 found that crates publishing a licence expression and no licence file were listed in the Linux archive's notices without the notice MIT and BSD put in every copy, and p2p-network-dev-01's OBSERVATION 01a15c55 asked, beside the Linux decision, whether the Android release owes the same crate notices; its page's `{third_party_notice_texts}` carried the native libraries' notices alone.

**Ruled.** It does, under ADR-0058: the rule it cites was proposed with this amendment and the owner accepted both on 2026-10-21, before the pull request that lands them merged. ADR-0058 decides the source order and the reviewed list for every artifact; decision 5 names the crate notices for the Android target, produced by the same tool as the Linux archive's, beside the native libraries' notices already there.

**Propagation.** Decision 5; this note; the log row; the digest line. The page template is rust-ui-dev's, the tool and the release run p2p-network-dev's (ADR-0058, Implementation implications).

### Amendment 2026-10-21 — `deny.toml` judges the Android target and names its two Slint crates

Decision 3 said the licence admission "does not yet cover" the Android
graph: `deny.toml` judged `x86_64-unknown-linux-gnu` alone and named the
desktop's crates. #314 (p2p-network-dev, merged 2026-10-20) put
`aarch64-linux-android` in `[graph] targets` and the two Android Slint
crates among the exceptions, with `unused-license-exception = "deny"` so
that dropping the target fails the check. The sentence is now past
tense with the merge date; the owed list keeps only the Skia notice set
and the release-gate run, the latter held by the same invocation;
decision 5 says "as admitted"; `apps/human-android/STORE-CHECK.md`'s
paragraph was rewritten the same way on #348 (it dates #314 by its
commit, 2026-10-19; this record dates it by its merge). Propagation: decisions 3 and 5; this
note; the log row; the digest entry.

### Amendment 2026-10-22 — The Android crate notices are a release asset, per ADR-0058 A 2026-10-22 (ii)

The 2026-10-21 amendment put the crate notices into the release page's
body; the first derivation measured them at about 790,000 characters
against a release body of about 125,000 (ADR-0058's history note of
the same day). Decision 5 now reads: the crate notices are a Markdown
file attached to the release beside the APK, its SHA-256 in the page's
hash list, `{crate_notices}` holding the link and `{crate_notices_sha256}` the hash; the native
libraries' notices stay in the body. The template's sentence is
rust-ui-dev-02's and language-culture-en's; the upload is
devex-tooling's `release.yml`. Propagation: decision 5; this note; the
log row; the digest entry.

### Amendment 2026-10-25 — The first release met decision 3's two owed items

Decision 3 closed with "What is owed, and by whom: the Skia notice set
... and `cargo deny --target aarch64-linux-android check licenses`
green before the first release". The first release, v0.1.0, was
published on 2026-10-25 (run 40179792207, second attempt; plan §22):
its page carries the Skia and skcms notices inline and the crates'
notices as `THIRD-PARTY-NOTICES-android.md`, and the one `cargo deny`
invocation that judges both targets was green on 71961578, the commit
the tag was cut on. The sentence now says both are met and where the
evidence is, so a reader does not take the first release as still
ahead. Propagation: this note; the log row; the digest entry.
