# Android identity screens: create, show the phrase, recover

Status: proposal for plan §20 steps 7–8, 2026-10-19. rust-ui-dev builds
it on `develop-qzapp/rust-ui-dev-01/feat/j111-android-recovery`. The
four questions in §11 were answered by architect-cto on 2026-10-19, and
the custody doc says the first two (425ec5d0).

Owner of the design: ux-designer. The words are language-culture's: §9
gives each string's purpose and space, never its text. The rules
behind the design are fixed by `android-key-custody.md` (Recovery, and
Recovery UI hardening, at ecf20942), `IDENTITY-RECOVERY.md` (Restore
semantics) and `human-client-ui.md` §2 and §9. This note decides how a
person meets those rules, and changes none of them.

## 1. The person's goals

- **First run:** "Set the app up as me." For most people that means
  creating a new identity. Someone who already had one restores it.
- **Create:** "Get my backup down on paper, and be sure I got it
  right." They have never seen a recovery phrase, and they do not know
  that it is the whole identity.
- **Recovery required:** "Get back in." The phone has refused the
  stored key, usually after a change to its screen lock, and the person
  did nothing wrong in the app.
- **Fresh-install restore:** "Be the same identity on this phone." They
  have the 24 words and, on the same paper, the PeerId.
- **Show my phrase:** "Check or copy my backup again." The app goes
  offline while the words are shown.

Entering 24 words is the costliest step in every flow. The picker in §4
is designed for that step first.

## 2. Rules every screen here keeps

From the contracts; listed so a reviewer can check each screen:

- `FLAG_SECURE` for as long as words are shown or entered. The flows
  run in their own non-exported Activity, excluded from Recents.
- No clipboard path for words, in either direction. Long-press on a
  word does nothing. The PeerId is public and stays copyable
  (`human-client-ui.md` §11).
- Words are entered only through the in-app picker (§4): never the
  system keyboard, never a free-text field.
- No announcement, log, notification, saved-instance state or snapshot
  carries a word. §8 and §11 Q2 say what a screen reader may speak.
- The network service is stopped, and the profile lock held, while
  words are shown or entered. Creation shows its words before the
  service first starts.
- Nothing restores silently. A phrase that derives another PeerId than
  the expected one restores nothing. A fresh install restores only
  after the person confirms the derived PeerId by eye; nothing is typed
  or scanned for it.

## 3. Screens and flows

The screens follow `design/2026-10-16-android-look.md`'s palette, type
scale and app bar. Each screen has one primary action, a filled button
at the bottom, above the gesture inset. Back is the platform's Back,
and the app bar's close or back arrow does the same.

### 3.1 First run (no record)

**W, Welcome.** The app's name and one line on what it is. Two actions:
- primary: create a new identity (→ C1);
- secondary, a text button below it: restore from a recovery phrase
  (→ R1).

Back leaves the app, as on any first screen. Nothing has been created
yet, so the next launch shows W again.

**A void paper.** A creation that showed its words (C2) and never
reached C4 leaves the person holding words for an identity that was
never made. That happens on Back to W, or on process death. So C2's
first display writes one non-secret marker, "words were shown,
nothing committed". It is a flag with no word, seed or PeerId. C4
clears it.
- While the marker is set, W shows a notice above its actions
  (`IdVoidPaper`): the words shown last time were never used and are
  not this phone's identity, so keep only the paper from the creation
  that completes. An unsealed phrase is harmless, because nobody holds
  that identity (§11 Q3). The risk is a person who keeps the wrong
  paper as their backup.
- Starting a new creation, or a restore that completes, clears the
  marker.

### 3.2 Create

**C1, Before you see your words.** What the phrase is and why it
matters, in four short points:
- these 24 words are your identity;
- write them on paper and keep it safe;
- anyone who has them can be you;
- the identity exists only once creation is complete, after the check
  of two words.

Screenshots of these screens are blocked, and there is no copy. The
PeerId is written down beside the words. Primary: show my words (→ C2).
Back → W.

**C2, Your 24 words.**
- The words are numbered 1 to 24, in two columns read down the first
  column and then the second. At a font scale over 130 %, or on a
  screen narrower than 360 dp, they show as one column.
- Each word is in body large, with its number in `on-surface-variant`.
- Below the words comes the PeerId in full (§5). Its label says to
  write it down with the words.
- The list scrolls if it must. The primary action stays fixed at the
  bottom: "I've written them down" (→ C3).
- Back → C1. The same words show again on return, because the identity
  is not committed until C4. Back from C1 to W discards the uncommitted
  identity, and W then shows `IdVoidPaper`.

**C3, Check two words.** Two positions are drawn at random (for
example 7 and 19). The person enters each with the picker (§4), one
after the other. "Word 7 of 24" heads each entry.
- A right word moves on to the next one.
- After both are right: → C4.
- A wrong word shows an inline error under the entry (string
  `IdCheckWrong`). It says which position, and to compare with the
  paper. The entry clears, and two actions appear:
  - try again (stays on C3);
  - show my words again (→ C2, with the same two positions on return).
- There is no limit on tries, and no way past C3 without two right
  words.
- Back → C2.

Why C3 exists: a confirmation the person only taps is tapped without
writing. Wallet apps that hold a 24-word phrase spot-check for that
reason. It is a hypothesis for this app, falsified if people with the
build fail C3 often for reasons other than a missing paper. Two words
cost about ten taps, and the picker already exists.

**C4, Identity created.** The PeerId in full again, then one line that
the network is about to start. Primary: continue. This commits the
identity, starts the service, and opens the Chats screen. There is no
Back: the identity now exists. It is sealed here, after the person's
confirmation and before anything starts (§11 Q3). After this, the words
reach the screen only through §3.5.

### 3.3 Restore on a fresh install (no record)

**R1, Restore your identity.** What is needed: the 24 words and the
PeerId written beside them. The network stays off until the restore is
done. Primary: enter my words (→ R2). Back → W.

**R2, Enter your 24 words.** The picker over all 24 positions (§4).
When all 24 are filled, the primary action, check my words, is
enabled.
- A failed checksum stays on R2 (§6).
- A valid phrase goes to R3.
- Back → R1. The entered words are dropped; a dialog confirms first if
  any are entered (§7).

**R3, Is this your identity?** The PeerId the words derive to, in full
(§5). Above it, the question: is this the PeerId you wrote down?
- Primary: yes, restore this identity (→ R4).
- Secondary: no, check my words (→ R2, with every word kept, to find
  the one that is wrong).
- Back does the same as the secondary.
- R3 never says the PeerId is "correct". The app cannot know; the
  person's paper decides.

**R4, Restored.** The same layout as C4. Primary: continue (starts the
service, opens Chats).

### 3.4 Recovery required (a record exists and does not open)

The app opens on **Q1** instead of the Chats screen whenever the start
answers `RecoveryRequired`. It says, by cause:
- `KeyInvalidated`, `KeyMissing` (string `IdQReasonKey`): the phone's
  security changed, for example its screen lock was removed or reset,
  so the phone no longer gives the stored identity back. Nothing is
  lost if the person has their recovery phrase.
- `Record`, `Authentication`, `WrongIdentity` (string
  `IdQReasonRecord`): the stored identity cannot be read on this phone.

Q1 shows the expected PeerId from the record when the record still
names one. Primary: enter my recovery phrase (→ R2, then straight to
the check against that PeerId, with no R3). Back leaves the app. Q1
shows again on the next launch until the restore succeeds.

If the record names no readable PeerId, the flow is the fresh-install
one: R2, then R3.

The person never gets a "start over with a new identity" action here,
because a new identity would replace an established profile
(`IDENTITY-RECOVERY.md`, ADR-0042; §11 Q4). Q1 names the one way out
when the words are lost (`IdQLost`), in plain text below its primary
action, with no button: clearing the app's data in the phone's
settings starts over with a new identity, which every contact must
trust afresh.

A restore that matches goes to R4. A phrase that derives another PeerId
stays on R2, with the mismatch state (§6).

### 3.5 Show my phrase (from Settings)

Settings gains an item, show recovery phrase, under the identity's
PeerId.

**S1, Show your recovery phrase.** The network goes offline while the
words are shown and comes back after. Primary: continue. This raises
the system's device-credential or strong-biometric prompt, whatever
the unlock policy. Back → Settings.
- The prompt is denied or cancelled: back on S1 with nothing shown and
  no error. The person chose not to.
- The prompt succeeds: a progress state on S1 while the service stops
  (`IdGoingOffline`), then S2.

**S2, Your 24 words.** The same layout as C2. The primary action is
done, which drops the words, restarts the service and returns to
Settings. Back does the same. Connectivity then shows as it always
does (`human-client-ui.md` §7). This flow adds no "back online"
message of its own.

## 4. The word picker

One position is entered at a time. From the top of the screen:

1. **The position:** "Word n of 24" (`IdWordPosition`), and below it
   the letters typed so far. The letters show in a display that looks
   like a field but is not a text field. No IME ever opens.
2. **The suggestions row:** the BIP-39 words that start with the
   letters typed, as chips, alphabetical, at most four on one row.
   - While more than four match, the row shows how many still match
     instead (`IdWordMatches`). A row of four arbitrary words would
     invite a wrong tap.
   - Every word is unique by its first four letters, so the row always
     comes down to four or fewer by the fourth letter, and often
     sooner.
   - Tapping a chip fills the position and moves to the next empty
     one. Nothing ever fills itself: even a single remaining match is
     a chip the person taps, so no word is entered without being seen.
3. **The keypad:** the 26 letters in QWERTY rows (10, 9, 7). Its last
   row holds delete-a-letter and clear-the-position.
   - A letter no word can continue with is disabled: shown at 38 %
     opacity, not removed, so the keys never move.
   - Keys answer with a state layer and a haptic tick on press.
   - The layout is QWERTY because a person's hands know it from every
     keyboard (recognition over recall). Disabling dead letters makes
     the narrowing visible.
4. **The phrase so far** (R2 only):
   - a grid of the 24 positions, four columns of six, each showing its
     number and, once filled, its word;
   - the current position outlined in `primary`;
   - tapping a position edits it: its word goes back into the letters
     display, ready to delete or replace;
   - at a font scale over 130 % the grid is replaced by a "review my
     words" button opening it as a full list, so the keypad keeps its
     room.

**Sizes.** Every key is at least 48 dp tall. On a 360 dp wide phone a
QWERTY key is about 31 dp wide with 4 dp gaps, the width every
system keyboard uses. That is under Material's 48 dp but above WCAG
2.5.8's 24 px. The narrowing and the required chip tap catch a mis-hit
before it enters a word. Suggestion chips and the grid's positions are
48 dp tall. Below 340 dp of width, where a QWERTY key would fall under
28 dp, the keypad becomes alphabetical, seven columns, wider keys.

**Font scale.**
- The letters, chips and words follow the font scale. The keys grow in
  height, never in count.
- At 200 % on a 360 × 640 dp phone, from the top: the app bar,
  position, letters, suggestions, keypad and primary action fit
  without scrolling. The phrase grid moves behind its button (above).
- The longest BIP-39 word has eight letters. Chips are sized to their
  word and wrap to a second row before any word is cut.

**Landscape.** The keypad and suggestions sit on the end side, and the
position, letters and grid on the start side.

**RTL.** The screens mirror. The words, the letters display and the
keypad stay left-to-right: they are English words in Latin letters.

## 5. Showing a PeerId to be compared by eye

- The PeerId is shown in full, in a monospace face, in groups of four
  characters set apart by spacing. The spacing is display only: copying
  gives the canonical string, with no spaces.
- It wraps by groups, never mid-group.
- Its label says what it is for (`IdPeerIdLabel`): it is not a
  password, and it is safe to write down and share.

A 52-character string is compared by chunks. Chunks of four make that
compare a matter of groups, not single characters.

## 6. States

| State | Where | What shows | Way out |
|---|---|---|---|
| Checksum fails (a wrong word or a wrong order) | R2 | A banner above the grid (`IdChecksumFailed`): the words are not a valid phrase; check each word and its order against the paper. Every word stays. | Edit any position, then check again |
| Another identity (with an expected PeerId) | R2 after Q1 | A banner (`IdOtherIdentity`): these words belong to another identity, with the derived PeerId in full (§5) and the expected one under it, so the person can see which backup they used. Nothing is restored. | Edit and check again, or Back |
| Authentication needed | S1; a restore that must open a healthy user-presence record first | The system prompt. Cancelled or denied: back to the step before, nothing shown, no error | Try again by the primary action |
| Profile busy (`ProfileLocked`) | Any step that stops the service or writes | The progress state (`IdGoingOffline`) for up to 10 s. Then a notice (`IdBusy`) that the app could not take the identity offline, with Retry. Entered words stay. | Retry, or Back |
| Storage fails (`Unavailable`, a write error) | C4, R4, the Q1 restore | A notice (`IdStorageFailed`): nothing was changed. Retry. Entered words stay while the screen is open. | Retry, or Back (words dropped) |
| Success | C4, R4 | The PeerId, and continue | Continue |

Errors appear inline on the screen they belong to, never in a snackbar.
A snackbar times out, and these need the person's action. Their text
is in `error` on the screen's surface. A banner uses
`surface-container` with an `error` icon, as the look spec's notices
do.

## 7. Leaving, Back and interruptions

- **Back with words entered (R2, C3):** a dialog asks whether to
  discard the words entered (`IdDiscardTitle`). The actions are discard
  and keep entering; keep is the default focus. With nothing entered,
  Back goes straight back.
- **The app leaves the screen.** Home, a call, or another app opening.
  A stopped Activity is neither completion nor cancellation (§11 Q1):
  - on return, every screen shows what it showed, with entered words
    kept in memory only, never in saved-instance state;
  - S2 shows its words again with no new prompt while it is the same
    Activity instance; a new instance is a new display, behind the
    prompt.
- **Finish, cancellation, completion or process death** drops the
  words. After process death the flow restarts at its first screen (W,
  Q1 or Settings), with nothing entered and nothing half-done
  committed.
- **Rotation** keeps everything on screen, entered words included. A
  rotation is not leaving.

## 8. Focus and keyboard

The screens are workable without touch: a hardware keyboard or switch
access.
- Focus order follows the reading order: app bar, position, letters,
  suggestions, keypad, grid, primary action.
- A typed hardware letter acts as its key. Enter takes the only
  suggestion when exactly one is shown.
- After a chip fills a position, focus goes to the keypad's first
  enabled key.
- After an error banner appears, focus goes to the banner.
- After a dialog closes, focus returns to what opened it.
- Every disabled key keeps its place in the focus order and is marked
  disabled, not skipped, so moving through the keypad stays
  predictable.

TalkBack does not reach the app yet (plan §20, Decided 2026-10-18). The
screens are built so that it can later: every key, chip and position is
a labelled control. A screen reader may speak a focused word
control's own label; no live region, announcement, hint or error text
ever carries a word (§11 Q2).

## 9. Strings

For language-culture to write. Each entry gives the purpose, the tone
and the space. None may contain a phrase word or a placeholder that
could hold one. `{n}` and `{peer_id}` are the only placeholders.

| Key | Purpose | Space |
|---|---|---|
| `IdWelcomeTitle`, `IdWelcomeBody` | What the app is, on first launch | Title one line; body two lines |
| `IdVoidPaper` | W's notice (§3.1): the words shown last time were never used and are not this phone's identity; keep only the paper from the creation that completes. Plain, not alarming | Three lines |
| `IdCreate`, `IdRestore` | W's two actions | A button, about 24 characters |
| `IdC1Title`, `IdC1Points` (four) | Why the words matter: they are the identity, write them on paper, anyone with them can be you, the identity exists only once creation is complete. Calm, not alarming | Title one line; each point at most two lines at 100 % |
| `IdShowWords` | C1's primary action | Button |
| `IdWordsTitle`, `IdPeerIdLabel` | C2 and S2's heading; the PeerId's label (§5): write it down with the words, safe to share | Title one line; label two lines |
| `IdWrittenDown` | C2's primary: I've written them down | Button |
| `IdCheckTitle`, `IdCheckBody` | C3: check two words from your paper | Title one line; body one line |
| `IdWordPosition` | "Word {n} of 24" | One line, 20 characters |
| `IdWordMatches` | How many words still match the letters ({n}) | One line, at most 30 characters |
| `IdCheckWrong` | C3's error: that is not word {n}; compare with your paper | Two lines |
| `IdShowAgain`, `IdTryAgain` | C3's two actions after an error | Buttons |
| `IdCreatedTitle`, `IdCreatedBody`, `IdContinue` | C4 and R4 | Title one line; body two lines; button |
| `IdR1Title`, `IdR1Body`, `IdEnterWords` | R1: what is needed, the network stays off | Title one line; body three lines; button |
| `IdCheckWords` | R2's primary | Button |
| `IdR3Question`, `IdR3Yes`, `IdR3No` | R3's question and its two actions. Never claims the PeerId is right | Question two lines; buttons |
| `IdQTitle`, `IdQReasonKey`, `IdQReasonRecord`, `IdEnterPhrase` | Q1 and its two causes (§3.4): reassuring, not technical | Title one line; each reason three lines |
| `IdShowPhraseItem`, `IdS1Title`, `IdS1Body` | The Settings item; S1: the network goes offline while shown | Item one line; body two lines |
| `IdGoingOffline` | Progress while the service stops | One line |
| `IdDone` | S2's primary | Button |
| `IdChecksumFailed`, `IdOtherIdentity` | §6's banners. `IdOtherIdentity` labels the derived and expected PeerIds | Three lines each, with PeerIds below |
| `IdBusy`, `IdStorageFailed`, `IdRetry` | §6's notices | Two lines each; button |
| `IdDiscardTitle`, `IdDiscard`, `IdKeepEntering` | §7's dialog | Title two lines; buttons |
| `IdQLost` | Q1's way out when the words are lost (§3.4): clearing the app's data in the phone's settings starts a new identity that contacts must trust again. Plain text, no button | Three lines |
| `IdKeyDelete`, `IdKeyClear`, `IdReviewWords` | The keypad's two keys (their accessible labels) and the grid's button at large font | Labels; button |

## 10. Acceptance

On the Pixel 9a, dark and light, at 100 % and 200 % font, portrait and
landscape:

1. A first launch with no record shows W. Back leaves the app, and the
   next launch shows W again.
2. Create: C1 → C2 shows 24 numbered words and the PeerId in full,
   grouped by four. A screenshot (`adb exec-out screencap`) of C2, of
   C3 and of R2 is black, and Recents shows no content for the flow.
3. Long-press on any word copies nothing, and nothing in the flow
   writes the clipboard. The PeerId copies in canonical form.
4. C3 asks two positions. A wrong word shows `IdCheckWrong` and clears
   the entry. Two right words reach C4. Continue starts the service and
   opens Chats.
5. Back from C2 and returning shows the same words. Process death
   before C4 leaves no identity: the next launch shows W with
   `IdVoidPaper`. So does Back from C1 to W after C2 was shown. A
   first launch that never showed words has no notice, and after C4
   the notice never shows.
6. Picker:
   - typing "ab" disables every letter no BIP-39 word continues "ab"
     with;
   - the suggestion row shows a count until four or fewer words match;
   - a chip tap fills the position and advances;
   - no word fills itself;
   - no IME appears at any time (`dumpsys input_method` shows no
     input shown).
7. Fresh-install restore with a known fixture phrase: R2 → R3 shows
   its PeerId. "No" returns to R2 with all 24 words. "Yes" restores,
   and the PeerId on the Settings screen matches.
8. A phrase with two words swapped shows `IdChecksumFailed`, keeps
   every word, and lets any position be edited.
9. Recovery required: with the record's key invalidated (rust-ui-dev's
   test hook), launch shows Q1 with `IdQReasonKey` and the expected
   PeerId. The right phrase restores with no R3. Another identity's
   phrase shows `IdOtherIdentity` with both PeerIds and restores
   nothing.
10. Show my phrase:
    - a cancelled prompt shows nothing and no error;
    - a succeeded prompt shows `IdGoingOffline`, then the words;
    - connectivity reads offline while they are shown;
    - Done restarts the service and returns to Settings.
11. Leaving R2 with words entered (Home) and returning shows every
    word still entered. After the process is killed (`adb shell am
    kill` while in the background), the next launch starts at the
    flow's first screen with nothing entered. Rotation keeps every
    word. Leaving S2 and returning to the same instance shows the
    words with no new prompt.
12. Profile busy and storage failure, by rust-ui-dev's test hooks, show
    §6's notices. Retry works, and Back leaves with nothing changed.
13. At 200 % on the Pixel 9a, R2's keypad, suggestions and primary
    action fit without scrolling, and no word is cut.
14. Every key and chip is at least 48 dp tall. Keys are at least 28 dp
    wide.

Items 13 and 15 assume that the Android build follows the system font
scale and takes hardware-keyboard input. rust-ui-dev measures both
first on Slint 1.18.1. Where one is not met, the item is blocked on
that measurement, not passed. A build that ignores the system font
scale would also be a finding of the Android review
(`design/2026-10-19-android-review.md`, S2).
15. A hardware keyboard enters a word by its letters, with Enter on a
    single suggestion. Focus follows §8.
16. No log line, notification, crash report or test snapshot from a
    run of items 2–11 contains a fixture word. That is checked by
    `adb logcat -d` searched for the fixture's 24 words, and by the
    test harness.

## 11. Questions answered

architect-cto, 2026-10-19 (REPLY 01a15282-ac06), against
`android-key-custody.md` and ADR-0042 and ADR-0033:

- **Q1. Is a stopped Activity "leaving the screen"?** No. The words stay
  in process memory only, never in saved-instance state. They go on
  finish, cancellation, completion or process death. An on-demand
  display does not ask its prompt again on resume of the same Activity
  instance (§7). The custody doc says so (425ec5d0).
- **Q2. What may a screen reader speak?** A focused word control's own
  label, which is the secure screen read aloud at the person's request.
  No live region, announcement, hint or error text ever carries a word
  (§8). This too is in 425ec5d0.
- **Q3. Commit at C4?** Yes. The seed is generated in memory, the phrase
  is shown and verified (C3), and the seal happens on the last screen,
  straight after the person's confirmation. Back or process death
  before it leaves a first run and no record. C1 says that the identity
  exists only once creation is complete.
- **Q4. A new identity from Q1?** Not in v1. Q1's text names the way
  out instead (`IdQLost`). Whether v1.x gets an in-app start-over path
  (delete the whole profile, then provision; never mint over the
  record) is the owner's product call. architect-cto carries it by
  name in the Stage 17 closing record.

## 12. Deliberately left out

- **A creation flow that defers the backup ("remind me later").** The
  contract shows the words before the service first starts. Deferring
  is a product change.
- **Words in any language other than English.** The format is English
  BIP-39 (`IDENTITY-RECOVERY.md`).
- **Scanning or typing a PeerId.** Excluded by the decision of
  2026-10-19.
- **A recovery-phrase file, a QR code, or any export other than
  showing the words.**
- **What the Settings screen shows besides the new item.**
