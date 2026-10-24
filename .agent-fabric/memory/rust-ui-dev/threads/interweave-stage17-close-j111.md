---
role: "rust-ui-dev"
class: threads
topic: "interweave-stage17-close-j111"
description: "j111 (InterWeave Stage 17 close, plan §20): what is decided, which branch carries what, and what waits on whom (2026-10-19)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6cfb81fb25a7548b
---

## j111 (InterWeave Stage 17 close, plan §20): what is decided, which branch carries what, and what waits on whom (2026-10-19)

architect-cto-01 asked on 2026-10-19 (REQUEST 01a1526e) for my lane's evidence to close Stage 17 (plan §20). My reply is 01a15272; the agreed order is clause (j), then items 1 to 5.

**Decided by architect-cto-01:**
- 01a15277:
  - Android shows the phrase at creation and on demand. On-demand display is preceded by a device-credential or Class-3 biometric prompt, whatever the key policy.
  - A fresh-install restore displays the PeerId the phrase derives to, and the person confirms it. It only ever creates a record, never replaces one.
  - Step 10 is §20's: package metadata plus a store-listing check quoting Play.
  - D2D transfer is measured with a positive control, and dataExtractionRules excluding everything lands with it.
  - The four device cases (Keystore, FGS lifecycle, process death, network callbacks) are steps 2-5's device evidence.
- The custody-doc amendment ecf20942 is folded unrebased into feat/j111-android-recovery as b4f772c5.

**Design:** ux-designer-01's design/2026-10-19-android-identity.md at d0307136, on their branch docs/android-identity. My checks went in REPLY 01a15281.

**More rulings:**
- 01a15282-ac06 and supply 425ec5d0, folded as b6f2508c:
  - a stopped Activity keeps the words in process memory, never in saved-instance state;
  - a focused word's own label is not an announcement;
  - the seal happens only at C4;
  - there is no in-app replace path in v1.
- ADR-0041 A 2026-10-19 (432a354f on #301): remoteMessaging stands with a hand-entered use case, the Play half stays open until §22, and specialUse is the fallback. Recorded in STORE-CHECK.md at 26b34071; the duration criterion was put to architect-cto as OBSERVATION 01a15283.

**One PR:** the one-open-PR rule means every j111 item lands as commits on feat/j111-android-recovery, not on four branches.

**Commits on feat/j111-android-recovery so far (2026-10-19):**
- 19af27a0: restore_identity, Expected and phrase_peer.
- b4f772c5 and b6f2508c: architect-cto's supply, folded.
- 2724eaef, 26b34071 and 7986b471: STORE-CHECK.md.
- d708ef90: dataExtractionRules excluding everything.
- c85d54ec: ui-model picker.rs, the word picker.
- 2c81203d: ui-model identity_flow.rs, the screens' state machine, with 10 tests, two of them mutation-checked.

**Later commits (pushed 2026-10-19):**
- c0c355ba: labels, 57 Id* UiTexts.
- 36bbd39c: whole-word chip, K1 with no PeerId offering no restore, AlreadyProvisioned.
- 4fb8c553: ui-slint identity.rs, the IdentityScreen in app.slint, View::show_identity and on_identity. Rendered under Xvfb at 360 and 320 dp with the scratch example j111_identity_look (kept in the scratchpad, not the tree).
- Supply folded: ecf20942..252a6f45 (four commits).

**Design churn:** #303 is in review rounds; watch its INFO messages.
- Round 2 (14d65c62) adds:
  - read the record back after any refused write: a matching PeerId is success;
  - the format identifier on C2 and S2, under IdNotWalletSeed (text pending);
  - the profile lock held from the first word to the seal.

**Waiting on p2p-network-dev-01:** a public PeerId-field reader, plus provision and restore that accept a held lock. Due by 2026-10-21, delivered onto my branch; the PR is armed only after it is folded.

**State at 2026-10-19 ~08:10:**
- Pushed up to 4fb8c553. Local, not pushed: 14532b61 (onboarding.rs), a96995b9 (the read-back narrowed to OldKeyRemains or Storage(Unsynced) and done by unlock, after #303 round 3's P1), and the supply folds through 0902e979 (merge 6a39a104).
- apps/human-android/src/onboard.rs is written but UNCOMMITTED: the Activity-side flow root, with asks to Kotlin and the cipher kept from Native.identityState. Still needed:
  - jni.rs entries: identityState, waitIdentityAsk, prompted, and text key 7 for IdS1Title;
  - Kotlin: MainActivity holds back bind() until the identity is ready, plus FLAG_SECURE, BiometricPrompt and finish();
  - activity.rs: call onboard::start before Root::attach;
  - the Settings entry for S1/S2 with the service's stop and restart.
- The lock: the seal borrows the flow's &ProfileLock, never consuming it (0902e979). This waits on p2p-network-dev-01's supply, due 2026-10-21.
- The Pixel 9a is back (API 37); the address comes from the owner by message only. Main ffce28df is installed for ux-designer-01's (j) review (j113, done). Device runs of j111 must not collide with their review: use the lease.

**Next:**
- The Slint views (W, C1-C4, R1-R4, K1, S1-S2, the picker).
- The platform glue: FLAG_SECURE over JNI, the BiometricPrompt credential prompt in Kotlin, stopping and starting the service, the void-paper marker file, BIP-39's list from the workspace bip39 pin in android-platform.
- The device cases.

**Open:** QUESTION 01a1528d to ux-designer-01, asking whether a restore's failure shows on R3/R2 rather than R4.

**Branches** (all develop-qzapp/rust-ui-dev-01/feat/...):
- j111-android-recovery: worktree ~/projects/interweave-j111-android-recovery. Holds 19af27a0: restore_identity, Expected::{Recorded, Confirmed} and phrase_peer, with 5 tests in crates/human/android-platform/tests/identity.rs.
- j111-android-platform-tests: planned; the device cases plus dataExtractionRules.
- j111-android-store-check: planned.
- j111-ux-findings: waits on ux-designer-01's clause (j) REQUEST.

**Waits on:**
- ux-designer-01: the screen design (my REQUEST 01a1527b). Copy comes from language-culture-en after the design.
- The owner: the Pixel 9a. Unreachable at a private-network address on 2026-10-19, so every device item waits.

**Findings:**
- Play's remoteMessaging use case is "relay text communication to another device", meaning continuity of the person's own messaging across their devices (support answer 13392821, read 2026-10-19). InterWeave uses the FGS to stay reachable for other people's messages. Whether that fits is a risk to ADR-0041's Play condition, and architect-cto's to judge.
- The Android 12 behaviour-changes page (last updated 2026-10-16) says allowBackup=false does not disable D2D transfer on devices from some manufacturers.

**State at 2026-10-19 (later session):** pushed through 4159658e. d5b9b9fc and 3aceb479 carry onboard.rs, the jni.rs entries, MainActivity's held bind, FLAG_SECURE, BiometricPrompt and finish (assembleDebug + lintDebug OK, no device run). 4159658e: the platform's Back goes to the window (OnBackInvokedCallback at PRIORITY_OVERLAY on API 33+, onBackPressed below, Native.back/waitLeave, app.slint handle-back), with two tests in ui-slint tests/swipe.rs, mutation-checked.
- Left: the Settings entry "Show my phrase" (S1/S2, with the service's stop and restart); the profile lock (waits on p2p-network-dev-01, due 2026-10-21); device cases.
- Pre-existing, not mine to fold silently: clippy -D warnings for aarch64 refuses jni.rs:235 `start(.., app_data_dir: String)` (needless_pass_by_value, from 1eb77c96). CI does not clippy the Android target.
- Checking the Android target here: PATH=$HOME/.cargo/bin first (the shell's /usr/bin cargo has no android target), JAVA_HOME=/opt/android-sdk/jdk; clippy for it = tools/ci/check_android_target.sh with CARGO set to a wrapper turning `check` into `clippy ... -- -D warnings`.
- 0954162d + c9ef9c67 (pushed): Show recovery phrase on Settings (View::offer_show_phrase), onboarding::shown_phrase, NetworkService.Local offline()/online() with `held` (awaitEnd and recheck stand aside), MainActivity per-instance phraseShown holds back connect(), onDestroy brings the service back, identity relay thread now always runs. Open: lock (supply), user-presence prompt before restart (acceptance 10), device run.
- (j) review findings: ux-designer-01 REQUEST 01a15305 (#304 dcdb2b40 design/2026-10-19-android-review.md §6), my REPLY 01a15308: all taken on feat/j111-android-recovery; order F1 (done 4159658e), F3, F9, F8, F4, F2, then ranks 2-3; F6 drops "/ human". N1: neither permission declared (targetSdk 36 on API 37); release install would wipe data, not done. F13 = configChanges lacks fontScale. Seeding: (a) today via scratch peer vs (b) debug seed hook — waits on their choice.
- ux-designer-01 REPLY 01a1530e: seeding (a) today; add one received message to keep (kind B) in place of T3, which their run lost; F10 spec at 129e1e8d on #304 (one status while waiting, how long, last problem class, written time kept; never given up per architect-cto); N1 carried, no release install; targetSdk is mine to raise with architect-cto-01. Swipe §11 on ffce28df: 1,2,5,7,8,11,15,16,17 pass.
- Built and pushed: F3 5d2d027d (composer own height; test in look.rs), F8 f38aec0d (system bars appearance from uiMode). F9: Slint 1.18.1 hard-codes IME_FLAG_NO_EXTRACT_UI (SlintAndroidJavaHelper.java:180), asked QUESTION 01a15310; ruling REPLY 01a15310-f17c: (a), §6 F9 at 5e27abc8 — H above keyboard, C composer, B app bar, M=120 dp: two-pane drops the list when messages < M; then notice steps aside, then the app bar; composer never under keyboard, draft in view; all returns at once on close; focus stays in composer; Back with keyboard up closes keyboard first; acceptance landscape at 100 % and 200 % font.
- Scratch render: A scratch path (3.5 GB) builds a throwaway examples/composer_look.rs (kept in the scratchpad) with --features desktop, run under Xvfb :97, import -window root. Remove the target when j111 closes.
- F4 part 1 4f784a1e (confirmation fits; Text min-width 0 under word-wrap; overflow test views.rs). Trust look asked QUESTION 01a15315; ruling REPLY 01a15316, spec §6 F4 at 3b19110a: app bar w/ back arrow, title keeps "Trust"; PeerIds wrap whole; outlined field with error under, button beside or under (200 %); rows as list items (PeerId + origin line); confirmation becomes a Material dialog over a scrim, "Do not change" focused + "Remove trust" button; Back/scrim tap = Do not change; focus back to the row's button, after removal next row/previous/heading. Fallback: notice under its row — tell them before building it. Acceptance 100/200 % both themes nothing past the edge.
- F9 built f27f0e60 (list/notice/bar aside, keyboard-overlap in-out for tests, Back hides IME first). Flake seen: look.rs two_leaving_messages_hand_their_days_separator_on_as_one failed 1 of 5 suite runs (wall-clock sleeps, 170 ms leave) under parallel load; passes alone. Mine (ui-slint); look at it after rank 1.
- F4 done: 4f784a1e (overflow), 90ac3242 (app bar, whole PeerIds code font, error text buttons, type-scale captions), 483ae108 (outlined field, error under, filled button beside/under by pane width), f087d0c2 (doc lints), 5b3b2cff (dialog naming its act, Back/scrim/Escape = Do not change, focus return to row / next / previous / heading; one focus slot for all focus moves). Lesson: run `cargo clippy --all-targets -D warnings` BEFORE committing (pushed a lint failure once).
- Rank 1 left: F2 (message actions behind long-press + swipe, bubble at rest = bubble + status). Then ranks 2-3, seeding (a), device re-review install.
- F13 built 6c96c005 (fontScale in configChanges; Slint helper view applies it). F2 ruling REPLY 01a1533d, specs #304 3eacb594 (swipe §4/§7, look §6): long-press within slop for platform time opens Material menu anchored to bubble, LONG_PRESS haptic; also Enter, Menu key, Shift+F10, secondary click, accessible default action; focus first item; Escape/Back/tap outside closes, focus on bubble; bubble = each rule's first control, links own stops after it; Retry from menu → focus bubble; row actions change while open → close menu, focus bubble; no chips at rest; menu items 48 dp with 24 dp icon; bubble pressed state layer (on-surface 10 %) while open; desktop same, no platform switch; no Copy (text not selectable, my REPLY).
- F2 built 80fe2281 (row menu: long-press/secondary click/Enter/Menu/Shift+F10/default action; menu items keep focus keys action:/source:; bubble = first control; stale menu closes; Chip removed; TouchSettings.long_press_ms over JNI). RANK 1 COMPLETE at host level. Installed 80fe2281 debug (APK sha c85eb889) on the Pixel 12:57 phone time, data kept; INFO 01a15362 to ux-designer-01. Install script: scratchpad install.sh <addr> <apk> under .agent-fabric/bin/phone-run (address only from messages; today's via fabric-history "Pixel").
- (fixed 2026-10-19: leave flake, set_monotonic) Left: seeding (a) for tasks 9-10 (+ kind-B row replacing T3), ranks 2-3 F5 F6 F7 F10 F11 F12, flake look.rs two_leaving_messages (1 in ~5 under load), F6 note: title() cites "agreed item 3c" for the route label — check before dropping "/ human".
- DECISION 01a1537c (architect-cto, owner's word 2026-10-19): Android is F-Droid only, no Google Play services (ADR-0056, #301 c6a59648). Step 10 = F-Droid metadata under apps/human-android (summary, description, licence, source/issue URLs, screenshots in F-Droid's structure) + DRAFT F-Droid build recipe (Rust toolchain + cargo-ndk pinned, native lib from source) + a check quoting F-Droid's inclusion policy with date read, each app dependency vs the policy's named repos, no GMS/Firebase/Play artifact on the app classpath (lockfile com.google = lint/test tooling only), SPIKE-008 P1 target-SDK half closed by step 3's device run with its API level. STORE-CHECK.md (26b34071) becomes that; Play quotations kept under a "Play half withdrawn" heading. No Play declaration/video/specialUse. Any dependency checked against ADR-0056 decision 2 + F-Droid policy.
- Round 1 re-review (ux REPLY 01a15368, #304 §7 at b96e4bec): pass F1 F2 F8 F13; owe F3 (handle after Back), F4 (monospace), F9 (anchored at newest, new step 5); new rank-2 F14 (boxed Show recovery phrase on Trust); question: landscape ~125 px strip left of conversation with a line — cutout inset or list pane remains? Built since: 992a30b6 (monospace Android), 40bd1a42 (F5 keyboard-only rings).
- INFO 01a15386 (architect-cto): step 10's F-Droid check decides gradle-wrapper.jar deliberately (keep + why, or drop for the system Gradle the recipe names); ADR-0056 at 4d9cf47f: reference build = the tagged release run's APK from the same commit (Stage 19).
- Round 2 ready: ebeccf2c installed (APK 8a3e871e, 14:17 phone), REPLY 01a153ac to ux-designer-01. Built since round 1: b010afb9 (F3 composer lets focus go when keyboard closes for touch users; F9 step 5 list anchored at newest), ebeccf2c (F14 text button), 40bd1a42 (F5), 992a30b6 (F4 monospace). Seeded 28 unread rows from host peer ...DEoNJsP (24 short, link, quote, 6-line, "K8: keep this one"); ux keeps K8 themselves. Seeding scripts in a scratch path (seed.sh, storecheck.sh); the scratch test (tests/android-e2e/tests/seed_review.rs) was deleted, rebuild from: Desktop D + Relay + AdbDevice::connect + c.start(relayed_example_of("human-android.yaml",[D],WILDCARD,relay,Some(direct_to_d))) + am start MainActivity + TransportClient on d.binding() sending; needs cargo build -p interweave-transport-daemon -p interweave-transportctl; relay port = segment after "/tcp/".
- Slint fact: inside a Flickable, a VerticalLayout's preferred-height/content-height read stale (the padding alone) right after a change; re-anchor on `changed content-height`. Flickable clamps content-y to content-height; drag scrolling works regardless.
- j116 queued: build p2p-network-dev-02's 99d66031 with my key after round 2.
- Step 10 F-Droid blockers sent to architect-cto-01 (OBSERVATION 01a153b1): (1) Slint 1.18.1's 10 crates are GPL-3.0-only OR Slint-RF OR Slint-Software; owner admitted RF (not FLOSS) → F-Droid needs the GPLv3 route (APK GPLv3) or no F-Droid; all other 440 crates have a FLOSS option. (2) skia-bindings 0.153.3 downloads prebuilt Skia from github.com/rust-skia/skia-binaries (binary-cache default) → recipe must build Skia from source (FORCE_SKIA_BUILD / no binary-cache, Skia srclib). F-Droid policy pages saved in scratchpad/fdroid (Inclusion_Policy, descriptions/graphics, Build_Metadata_Reference), read 2026-10-19.
- architect-cto REPLY 01a153b3: blocker 2 RULED — Skia from source for F-Droid (LLVM, Python 3, Ninja on builder; Skia sources at rust-skia's pinned commit chrome/m155 for 0.153.3 as a srclib; binary download disabled); the project's tagged release run builds Skia from source the same way; measure in step 10: offline srclib variable, build time, fits builder; if not → ADR-0039 revisit (owner). Blocker 1 to owner with recommendation GPL-3.0-only for the F-Droid APK (repo stays Apache-2.0, desktop keeps RF). Write step 10: metadata placeholders, dependency listing + lockfile assertion, wrapper-jar call, check naming 1 open and 2 ruled with measurements pending.
- DECISION 01a153c4 (owner via architect-cto): F-Droid DROPPED for now; ADR-0057 supersedes ADR-0056 on distribution (branch develop-qzapp/architect-cto-01/docs/android-release-no-store): signed APK of tagged release run on the project's release page with SHA-256, release notes, Slint attribution, manual install/update, no store, no update channel; Slint stays Royalty-free; no Play services (ADR-0057 decision 2). Step 10 now: app id, version name/code, signing scheme + where key is kept (never in repo), release-page template (attribution, hash, manual-update note); dependency check listing every app dependency with licence (RF as admitted) + lockfile assertion; SPIKE-008 target-SDK half closed by step 3's device run with API level. F-Droid parts kept only as the record of why dropped. Skia measurement stopped & removed (clone 28 s, git-sync-deps 56 s/1.1 GB, offline build needs ANDROID_NDK; log a scratch path). pkill -f killed my own shell again (exit 144): use pgrep + kill PID.
- Step 10 (ADR-0057 shape) written, uncommitted at time of note: STORE-CHECK.md rewritten as the release check (app table; Rust graph 440 crates by licence expression; Slint 10 Android crates RF, android-activity + renderer-skia pending deny.toml (p2p j91 fix/deny-android-target); Skia prebuilt archive contents read from libskia.a by GN target prefix: Skia+skcms BSD-3, libjpeg-turbo IJG+BSD, FreeType FTL (credit line), libpng v2, zlib, Expat MIT, Wuffs Apache-2.0, VMA MIT, cpu_features BSD-2 from NDK; licences read from each component's file at Skia DEPS for m153-0.101.2; Java runtime classpath kotlin-stdlib 2.2.10 + annotations 13.0; no GMS/Firebase/Play in 3 lockfiles; SPIKE-008 target-SDK half OPEN until step 3 device run (p2p-dev-02, j116); F-Droid record; Play record). RELEASE.md (app id, version, signing PROPOSED v2+v3, one key, secrets store + offline backup, cert fingerprint on page) + release/release-page.md template (Slint badge pinned v1.18.1, notices, manual update) — copy placeholders for language-culture-en.
- F6 ruling REPLY 01a153e8 (architect-cto): label leaves the title while a peer has ONE conversation (title = short PeerId; route on Details as peer-asserted); when a SECOND conversation with the same peer on another route exists, every title for that peer carries its route label (worded as a route, never a role). Doc sentence human-client-ui.md §4 at a5ecfc04 on develop-qzapp/architect-cto-01/for/rust-ui-dev-01/direct-title-route-label — fold unrebased into the branch that builds F6, name the range; update title() comment ('agreed item 3c').
- Pushed later: 51a00fb5 (long status wraps, time whole), 9801deed (leave flake: View::set_monotonic, tests step it), 62fad28a (Android monospace via fontique generic lookup — round-2 F4 on phone NOT yet re-checked), 602716f6 + 05dfcdea (step 10 docs: STORE-CHECK.md release check, RELEASE.md, release/release-page.md; template holds no notice text — check_license_headers flags "All rights reserved", docs_integrity flags {url} links), 53574453 (fold a5ecfc04), 13377009 (F6 title rule + desktop-e2e follows F2/F4: list item actions ["click"], menu via item action, dialog act "Remove trust"; all 19 human_app pass). Copy REQUEST 01a153e8-0ec2 to language-culture-en for release-page.md.
- Open: ux round 2 (paused; person on phone) — F3/F5/F9 checks, keyboard-up-on-Chats sighting (not reproduced), K8 kept by them; next phone install should carry 62fad28a+; j116 after round 2; F7, F10, F11, F12 rank 2-3 not built (F10 needs strings/clock, F11/F12/F7 need ux spec words) — carry or build; S1/S2 user-presence prompt before restart (acceptance 10); profile lock supply (p2p-dev-01, due 2026-10-21); SPIKE-008 target-SDK half after step 3 device run.
- Pushed: d7b79e65 (release-page.md.in, LC final English; answers REPLY 01a15428), 3aa186c1 (acceptance 10: ui-model PresenceGate 60 s window over Seal/Restore*/StartService; IdentityFlow::cancelled_step; onboard reads key_unlock_policy). NB 3aa186c1's message says "the Pixel's profile is background-compatible" — UNVERIFIED; say so in the PR body. Commit-msg hook: trailers must be one block, each on one line (a wrapped Supplier-Review breaks Kind detection).
- Remaining j111: device re-check (round 2 resume; next install must carry 62fad28a..3aa186c1), j116 after round 2, F7/F10/F11/F12 (rank 2-3: build or carry by name; F10 needs LC words + clock), profile lock supply (p2p-dev-01, 2026-10-21), SPIKE-008 target-SDK half (step 3 device run), PR for feat/j111-android-recovery (one PR; arm needs supply folded).

- 2026-10-19 j117 delivered: f863c10c Gradle lockMode STRICT (androidApis was the one unlocked config, now empty=). CI runs only assembleDebug, so lintDebug is ungated: run it locally before an Android push (it caught GestureBackNavigation, fixed 9d2e7432).
- F10 cause found + fixed 62312ee5: Android view attaching to a running facade listed pending rows as Sending (attempts 0) and the facade reports only on change, so Unconfirmed rows read Sending after each reopen until the next failed attempt (<=5 min). FacadeSide.outbound caches live rows, snapshot replays them. My REPLY 01a15308-9534 (alternation = retries) was WRONG; corrected to ux REPLY 01a15450-4812. F10 words REQUEST 01a15450-4942 to language-culture-en (10 templates + age steps). Device check of the fix owed.
- 2026-10-19: the profile-lock supply WAS delivered 06:29 (REPLY 01a152d9, for/rust-ui-dev-01/custody-recovery-api e8de15b4) and sat unfolded until owed-supply showed it; folded 1ee5ead7, used 40bca6bc (flow lock via onboarding::hold, named_identity, RecordNamesNobody, create Unsynced read-back untested). F10 words final: LC REPLY 01a15453 + 01a15455 (cite both in Supplier-Review). F7/F11/F12 specs at ux 65231264; cutout + long-status acceptance at 159bfffd. Branch past 16 work commits: open PR now, carry F7/F10-words/F11/F12/cutout to a stacked branch.
- 2026-10-19 PR #309 opened (head d87a93ce, 54 work 1 fix). Round 1 review (pullrequestreview-5573490800): P2 F1-F5 (StartService dropped with flow at Done; S1 failure leaves service held offline; FLAG_SECURE async race; flow proceeds unheld; onboard::start races identityState), P3 F6 (spec: own non-exported Activity; human-client-ui §9 = architect-cto, QUESTION 01a154c7, recommend (b) amend), F7 F8 F9. Fixed 1ecd5196 0cc3a508 31360777 (Effect::Hold, IdentityFlow::held/not_created, PresenceGate::holding, View::after_next_frame, SECURE only cleared after a word-free frame, Kotlin sets it before words). Answers comment issuecomment-6270928062. Re-review r2 dispatched on d87a93ce..31360777. Stacked F10 branch feat/j111-waiting-status eb80f10e (pushed, no PR). Second Slint Activity IS possible (GLOBAL_CONTEXT thread-local; android proxy replaceable).
- 2026-10-19 later: round 2 (pullrequestreview-5574443743) P1 N1 (start() wait deadlocked with onStart: super.onStart waits on android_main) -> afb74a49 asks identityState BEFORE super.onStart; N2-N4 + risks fixed. Answers issuecomment-6271796750. Folded architect 80594810 as 5a04689b (brought main incl. check_android_no_play_services.sh; f2a3bbe8 names it). Round 3 re-review dispatched on 31360777..afb74a49. Acceptance 10 NOT MET (cancel -> Status not known + false "will connect" notice; diagnostic has no UI) carried by name; architect INFO 01a154f7: Stage 17 CLOSE WAITS on launch-prompt piece (gate (b) user-presence restart diagnostic) = next branch first piece: state "start refused, authentication needed" (offline, diagnostic notice, re-prompt action, cancel+launch), device test, setRecentsScreenshotEnabled. Tell ux if it slips past close. No person reaches user-presence in #309 build (stand_in omits key_unlock_policy, default background-compatible). Stacked branch feat/j111-waiting-status: F10 eb80f10e; F12 built uncommitted (AppOptions "App options" final, menu with Trust, TrustSettings text removed); F7 words final (short subtitle texts as accessible name, long as description; banner host-split), F11 words final (PeerId / Channel ID, Copy PeerId, Close, route line, snackbar incl desktop).
- 2026-10-19 #309 ARMED (57 work, 6 fix) at 21e2a785, queued 1; rounds 3-5 clean (N5 fixed 45e47494; CI DejaVu overflow fixed 21e2a785). Next branch feat/j111-waiting-status (F10 eb80f10e, F12 859e326b, merge a22fa3fc) has the launch-prompt piece uncommitted: SessionNotice::AuthenticationNeeded + Intent::Authenticate + Opener::authenticate, hub ToView::AuthenticationNeeded, onboard AFTER_PROMPT/await_authentication/authenticated, state AUTHENTICATION_NEEDED=3, ask::UNLOCK=8 with title Unlock InterWeave (LC 01a15524-e91b); words LC 01a15511-9c0a. Still to do there: setRecentsScreenshotEnabled, F7, F11, device test; PR after #309 merges.
- 2026-10-19 #309 MERGED 5fb0589f. Next PR #313 (feat/j111-waiting-status, worktree ~/projects/interweave-j111-f10, CARGO_TARGET_DIR shared with ~/projects/interweave-j111-android-recovery/target): F10 eb80f10e, F12 859e326b, launch prompt + Recents d350cb6a, main merged adc51f82. Open, unarmed. Still to add: F7 (Chats subtitle short texts as name + long as description; offline banner host-split; LC 01a154ca-d723 + 01a154cd-c3d0), F11 (Details dialog: PeerId label, route line, Copy PeerId/Close, channel variant Channel ID/Copy channel ID, snackbar "PeerId copied" below API 13 AND desktop; ux 11b07651). Device test of the launch prompt owed (close waits; needs user-presence via adb run-as on the debug build). Told architect REPLY 01a15559-df5e, ux 01a15559-e7cb.
- 2026-10-19 cleanup (owner broadcast 01a15576): swept merged j111-android-recovery (remote auto-deleted), deleted local test/phone-run-276; j111 worktree kept DETACHED as the shared CARGO_TARGET_DIR (target 111G after removing debug/incremental 45G); a scratch path removed. j97 worktree kept (unpushed prototype). #313 now has F7 c8ac15c2 and F11 8fdac376 too (5 work).
- 2026-10-19: j116's build head is now d47e02fb on develop-qzapp/p2p-network-dev-02/test/j36-android-e2e-values-as-text (INFO 01a15675). It replaces 99d66031, which is retired. #313 stays unarmed: architect-cto-01 holds it until the device run (REPLY 01a15632).
- 2026-10-20: j116 now builds origin/main after #316 merges (or #316's head at that moment, if my turn comes first). d47e02fb is retired (INFO 01a15815).
- 2026-10-20: #316 merged as 8a0c8d64. j116 builds origin/main at or after it, and I send p2p-network-dev-02 the commit I built (INFO 01a15840). Launch-prompt test moved to the Samsung A40 at the owner's word (Pixel: nothing destructive). APK 3e12fc5a from 04605e70 installed fresh there, user-presence config seeded with run-as.
- 2026-10-20 Samsung A40 (API 30) run of #313, done at the owner's word: the Pixel is non-destructive only.
  - Setup: fresh install; user-presence config seeded with run-as at interweave/config/profiles/human-android/config.yaml.
  - Verified at the target-platform level with the owner's hands:
    - Onboarding seals with a prompt.
    - A cold launch past 60 s prompts "Unlock InterWeave" before binding.
    - A grant binds.
    - Back cancels (API 30 has no Cancel button with DEVICE_CREDENTIAL). Leaving for Recents is onError 10, counted as a cancel.
    - After a cancel the subtitle reads Offline.
  - DEFECT found: no session notice on a phone's Chats; it was drawn only in the conversation pane. Fixed in c3a3c8ea (NoticeCard, has-session-notice).
  - Re-run on c3a3c8ea (APK 501225b2):
    - Return via Recents gives no prompt.
    - The yellow notice with Unlock shows.
    - Unlock prompts, a PIN grant starts the runtime, and the subtitle reads "Online, limited".
  - FLAG_SECURE blocks screencap, so the owner reads the screen.
  - The default availability is foreground-only, so leaving stops the runtime; that is correct.
  - Owner finding: the PeerId sits beside the 24 words. ux split it in design 2d102b4c/4cf3deed; LC words are final (INFO 01a1589d, 01a1589f; replay 376848/376883). To build next.
- 2026-10-20 window: Pixel API 37 got 35c71169 (APK 2faa357a) by install -r. p2p ran 6 cases; ux round 2 found F4 (fontique Android generic empty -> ea91c966 Droid Sans Mono) and F16 (composer no focus; bisect 80fe2281..ebeccf2c; b010afb9 clear-focus removed in ff981b5e, unproven on device). Gate (b) run record 53c34f7a. Pushed ff981b5e. Pending: Recents thumbnail, capture-with-words measurement, notification line, network callbacks, the Slint issue (owner OK), j124 human_chat close.
- 2026-10-20 22:50 Pixel API 37, build 6f8162e3 (APK 07f2bd0f), install -r: F16 fixed (keyboard opens in ...x6gY6Eri, owner); F4 fixed (Trust PeerIds monospace, screencap read by me, deleted).

*Observed 2026-10-19 (rust-ui-dev)*
