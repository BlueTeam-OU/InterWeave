# android-e2e on a Samsung A40: the cluster bound, release profile, API 30 — 2026-11-13

TM-58's owed device run (`tests/SECURITY-MATRIX.md`; architect-cto's
REQUEST 01a1d12d-47dc): a peer's character, one grapheme cluster, at the
32-code-point bound and past it, drawn wherever the Android client draws
a peer's text, on a release-profile build. The case is
`tests/android-e2e/tests/cluster_bound.rs::on_a_device_a_character_at_the_cluster_bound_and_past_it_is_drawn_everywhere`.
A build with the replacement taken out is the control. Every time is UTC;
the phone's clock read UTC+4, and its log lines and screens show that.

## What ran

| | |
|---|---|
| Device | Samsung Galaxy A40 SM-A405FN, `samsung/a40eea/a40:11/RP1A.200720.012/A405FNXXU4CWC3:user/release-keys`, Android 11, **API 30** |
| Link | Wireless adb, on this login's own adb server under the `samsung-a40` lease; no adb process of this login remained after any run (`pgrep -u "$(id -u)" -a adb` empty). `stay_on_while_plugged_in` read 7 before and after every run and was not changed. |
| Before | The phone held rust-ui-dev-02's debug build (app `d0209344`, test `4db8c691`, another signer). rust-ui-dev-02 and p2p-network-dev-02 released the phone (REPLY 01a1d3ba-2170, 01a1d3ba-45bf), and both packages were uninstalled on the owner's word. |
| Build under test | The app at main `41bb631b9`; the branch then added only `tests/android-e2e`, so it was the same app. Its later commits change the app (the owed Connect's launch rule, j268; a proposal's row unclipped at rest, j270; every elided label clipped to its box, j271, two of them drawing a peer's text), and this run is not of them. `./gradlew --rerun-tasks assembleDebug assembleDebugAndroidTest`, with the native library compiled at the root `Cargo.toml`'s `[profile.release]` settings passed as `CARGO_PROFILE_DEV_*` (opt-level 3, thin LTO, one codegen unit, debug 1, overflow checks **on**, debug assertions off, no incremental). This is the 2026-11-12 A40 record's method: a release APK carries no trace and no case entry, so the case cannot drive one. rustc's command lines were not read; the control's overflow panic below shows the overflow checks were on. App sha256 `077e4183462eb1fd2f1636055d716f3a4c6512d8a9e37309e9b953e9580eb163`, androidTest `1efb4b1552d428095f93774ee0bc7a24bce9fb38369b6e431d236227f9b9af2b`, signer certificate SHA-256 `18f39ab0…`. |
| Control: the build is the one installed | After each install, `sha256sum` of each package's `base.apk` on the phone (its `pm path`) was the built APK's, both hashes above, for every run counted here. |
| Control build | The same source and settings, with `cluster.rs`'s `too_long` returning false, so no character is ever replaced. It is a probe, not in the tree. App sha256 `da17702a63a5e27fea4b97b66af348a5b3a95a2f971821170553488fc1bea5e4`, same signer, installed with `install -r` over the build above; its `base.apk` read the same on the phone. |
| Host side | `tests/android-e2e` on this branch, D a desktop `transport-daemon` on this host's loopback. Command: `cargo test -p interweave-android-e2e-tests --test cluster_bound -- --include-ignored --test-threads 1 --nocapture`, with `ANDROID_SERIAL`, `INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/org.interweave.human.test.E2eCase` and `INTERWEAVE_ANDROID_SHOTS` set. |

## What was sent and drawn

D's person sent one body, `at <a + 31 U+0301> one past <a + 32 U+0301>
far past <a + 255 U+0301> end`: characters of 32, 33 and 256 code
points. Then two contact cards: the first named one character of
32 code points (`x` + 31 U+0301), the second one of 64 (`x` + 63 U+0301),
the most a card name holds. The case accepted each card by key. After
every step it read the app's log for a panic and the app's pid, then
saved the screen.

**The recorded run, 08:19:51 to 08:21:11 (fresh install): pass**, one
process (pid 25009) and no panic line across all seven steps.

| step | where | at the bound (32) | past it (33, 64, 256) |
|---|---|---|---|
| 1 | Chats, the body's line | not replaced; its marks rose above the line to the row's clip | each one U+FFFD |
| 2 | the conversation, the body | not replaced; about two of its 31 marks showed | each one U+FFFD |
| 3 | the first card's proposal ("This peer says they are …") | not replaced; about two marks showed | — |
| 4 | the conversation's title and the notice "You saved … as a contact.", after Accept | not replaced; in the title its marks ran off the top of the screen, in the notice about two showed | — |
| 5 | the second card's proposal ("This peer says they are …", "New name:") | the current contact's name not replaced ("your contact … is updated"): about two marks showed there, and the title's ran off the screen as in step 4 | one U+FFFD |
| 6 | the title and the notice "You updated your contact …", after Update contact | — | one U+FFFD |
| 7 | Chats, the conversation's name | — | one U+FFFD |

A first run at 08:09:26 to 08:11:13 drew the same on every step. It
passed under an earlier form of the case, which read the pid only and
had an eighth step: a relaunch drawing Chats from the store. That step
was dropped: the body, read and not kept, and the accepted card's row
are gone from the store by then, so Chats was drawn empty.

**At the bound, in a one-line label, the marks spilled out of the
line.** In the Chats row (step 1) the 31 stacked accents rose from the
preview over the row's own name and stopped at the row's top edge,
which the row's clip cut. From the conversation's title (steps 4 and
5), which nothing clipped, they ran up through the status bar and off
the top of the screen. In wrapped text (the body, the proposal, the
notices) the same character showed about two of its marks, the first
just above the letter and a second above that. No more were visible.
That is where the Text's own box ends, and Slint clips wrapped text
to its box (below), so the rest were most likely drawn and cut there;
the screens show where they stop, not why. So "not replaced" in the
table means the client drew the cluster without replacing it, not
that all 31 marks could be seen. Nothing crashed and nothing past the bound was drawn, but a peer's name or
text at the bound painted outside its line in those two places, and
outside the screen's own area in the title. The cause was every elided
label: Slint 1.18.1 clips a Text to its box only under
`overflow: clip`, which wrapped text has. Since this run every elided
Text is drawn in a clipping box (ui-slint's `OneLine` and
`FocusHeading`), `tests/compile.rs` holds each one to it and
`tests/one_line_clip.rs` measures the desktop's Chats preview staying
in its line; no phone has run it since.

## The control: no replacement

**08:11 to 08:13:52, the control build: fail, as it should.** Step 1
took the body, and the app's log then had, at 08:12:23:

```text
panic on thread unnamed at …/parley-0.11.1/src/shape/mod.rs:270: attempt to add with overflow
```

This is the same shaper and the same overflow the desktop's real-window
test meets (`crates/human/ui-slint/tests/cluster_draw.rs`): the phone
draws with its own renderer, but it shapes text with parley 0.11.1 too.
The app's process lived on with the same pid. The window was gone, and
the screen showed the system settings page behind it. The case as it
ran then read the pid only, so it passed step 1 and failed at the next
key: 40 TABs and no focus line. That is why the committed case also
fails on a `panic` line in the app's log.

A second run on the control build (08:14 to 08:15, the case as
committed) failed at its launch: the store still held the first run's
unread body, Chats drew its line at the start, and the same panic came
at 08:14:56, before any key.

## Not counted

A run at 08:15:25 to 08:18:00 on the build under test, reinstalled with
`install -r` over the control (data kept), drew steps 1 to 5 as above
and then failed. The store held the first run's contact, of the same
name and key, so the second card's proposal carried a note and an
"Open your contact" link before its Update contact. The case's TABs went
`proposal-copy:1`, `proposal-contact:1:1`, `composer`, ... and never
reached `proposal-decline` or `proposal-accept`, two times round.
**That is a keyboard-reach finding**: with that note, the proposal's
Decline and Update contact are not reachable by TAB on the phone. The
case now says it needs an app with no contact, and the recorded run was
on a fresh install.

## What this does NOT show

- The release APK itself (its Java side and signing): the native
  library is built the same way.
- Any other phone, the Pixel 9a included.
- Show source and the reader, which the host tests hold to the same
  replacement; a name in a notification (a card is never announced);
  the identity and add-a-peer screens.
- The scripts the shaper clusters otherwise than UAX #29.
- TalkBack.
- That the glyphs are right beyond what a person reads off the screens.
  The trace names kinds, not text. The screens were read by this
  session, and they are not committed (they carry this host's test peer
  ids).
