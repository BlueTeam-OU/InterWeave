# Android identity screens: create, show the phrase, recover

Status: proposal for plan §20 steps 7–8, 2026-10-19. rust-ui-dev builds
it on `develop-qzapp/rust-ui-dev-01/feat/j111-android-recovery`. The
four questions in §11 were answered by architect-cto on 2026-10-19.
The custody doc's text behind them (ecf20942..252a6f45) is on
architect-cto's branch
`develop-qzapp/architect-cto-01/for/rust-ui-dev-01/custody-phrase-display`
and is not merged; it lands with rust-ui-dev's build PR.

Owner of the design: ux-designer. The words are language-culture's: §9
gives each string's text, where it shows and why. The rules
behind the design are fixed by `android-key-custody.md` (Recovery, and
Recovery UI hardening, at ecf20942..252a6f45), `IDENTITY-RECOVERY.md` (Restore
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
- **Show my phrase:** "Check my backup, or write it out again." The app goes
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
- The network service is stopped, and the profile exclusively locked,
  while words are shown or entered (`android-key-custody.md`, Recovery).
  The on-demand display holds the lock from the moment S1's progress
  state has stopped the service until S2's Done, which releases it
  before the restart: S2 reads the seed through
  `unlock`, which takes no lock. Create and restore take the lock once,
  before the first word is shown or picked (C2, R2), and hand it to the
  seal or restore they end in. `provision` and `restore` will accept a
  held lock (§11b Q9, asked of p2p-network-dev). Today each takes a
  fresh lock and would refuse one the flow holds, so C4 and the
  restores are not done until the crate accepts it. A refused seal or
  restore hands the lock back to the flow, which keeps it for the
  read-back and for Try again, and releases it only when the flow ends.
  A seal or restore that succeeds releases the lock before the service
  starts. The hand-back on refusal is still to be confirmed for the
  crate (§11b Q10). There is never a release and
  retake, which would let the service start the profile in between. Creation shows its words before
  the service first starts.
- Nothing restores silently. A phrase that derives a PeerId other than
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
matters, in six short points:
- these 24 words are your identity;
- they are an InterWeave identity recovery phrase, not a
  cryptocurrency wallet seed (`IDENTITY-RECOVERY.md`, Recovery format);
- write them on paper and keep it safe;
- anyone who has them can be you;
- they bring back the identity only, not the contacts, trust,
  settings or messages; on Android there is nothing else to back up
  them from, so the point asks for nothing (`human-client-ui.md` §9 as
  amended at 2f3b0587);
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
- Above the words, one line (`IdNotWalletSeed`) says that this is an
  InterWeave identity recovery phrase, not a cryptocurrency wallet
  seed. Under it, in body small and `on-surface-variant`, the format
  identifier, `interweave-ed25519-bip39-entropy-v1`, after a short label
  (`IdFormatLine`). `identity-recovery.md` (Backup UX) lists it among
  what the display must include. It is public, and not a word of the
  phrase. S2 shows both lines too.
- The PeerId is not on this page. It follows on a page of its own
  (below), so the secret words and the public PeerId are never on one
  screen. The owner read the PeerId beside the words as misleading
  and asked for it to move to a following step (2026-10-20, relayed
  by rust-ui-dev-01, REPLY 01a15899-4c7e).
- The list scrolls if it must. The primary action stays fixed at the
  bottom: "I wrote down the words" (`IdWrittenDown`, → C2's PeerId
  page).
- Back → C1. The same words show again on return, because the identity
  is not committed until C4. Back from C1 to W discards the uncommitted
  identity, and W then shows `IdVoidPaper`.

**C2's PeerId page, Your PeerId.** The second page of C2, after the
words. Title `IdPeerIdTitle`.
- The PeerId in full (§5), with `IdPeerIdLabel`: it is not a password,
  it is safe to share, and it goes on the same paper as the words,
  because a restore compares it (R3).
- Nothing of the words shows here.
- Primary, fixed at the bottom: "I wrote down the PeerId"
  (`IdPeerIdWritten`, → C3).
- Back → the words page, with the same words.

**C3, Check two words.** Two positions are drawn at random (for
example 7 and 19). The person enters each with the picker (§4), one
after the other. "Word 7 of 24" heads each entry.
- After a right word, the entry moves on to the next position.
- After both are right: → C4.
- A wrong word shows an inline error under the entry (string
  `IdCheckWrong`). It says which position, and to compare with the
  paper. The entry clears, and two actions appear:
  - try again (stays on C3);
  - show my words again (→ C2, with the same two positions on return).
- There is no limit on tries, and no way past C3 without two right
  words.
- Back → C2's PeerId page.

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

**R1, Restore your identity.** What is needed: the 24 words, and the
PeerId to compare at R3. The PeerId is the one written beside the
words, or else the one a contact has for the phone that had these
words (a contact can hold one PeerId per device, `human-client-ui.md`
§3). The
network stays off until the restore is done. One line
(`IdR1Settings`) states what `human-client-ui.md` §9 requires, as
amended at 2f3b0587: the words restore the identity only, not the
contacts, trust, settings or messages. Android has no export of
those in standard v1, so the line asks for nothing (§11a Q5). Primary: enter my words (→ R2). Back → W.

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
- A person with the words but no PeerId on paper reads a line under
  the question (`IdR3NoPeerId`): compare with the PeerId a contact
  has for the phone that had these words, and restore only after that.
  They leave by Back,
  with their words kept while the Activity lives (§7). There is no
  third button: confirming a PeerId nobody compared is exactly what R3
  exists to prevent.

**R4, Restored.** The same layout as C4. Primary: continue (starts the
service, opens Chats).

### 3.4 Recovery required (a record exists and does not open)

The app opens on **K1** instead of the Chats screen whenever the start
answers `RecoveryRequired`. It says, by cause:
- `KeyInvalidated`, `KeyMissing` (string `IdQReasonKey`): the phone's
  security changed, for example its screen lock was removed or reset,
  so the phone no longer gives the stored identity back. Nothing is
  lost if the person has their recovery phrase.
- `Record`, `Authentication`, `WrongIdentity` (string
  `IdQReasonRecord`): the stored identity cannot be read on this phone.

K1's expected PeerId is the one the record's PeerId field names, read
even when the record is otherwise refused. Custody gains a
public reader of that field for this (§11b Q8; custody doc 252a6f45,
asked of p2p-network-dev): a record refused for its shape with an
intact field names its PeerId, and K1 restores only a phrase that
derives it. Until the reader lands, K1's Record causes are not done:
without it such a record would fall to `IdQNoPath`. K1 has three
inputs: a record that opens (no K1), a record that names a PeerId
(restore to it), and a record that names none (`IdQNoPath`). When the
record names a PeerId, K1 shows it.
Primary: enter my recovery phrase (→ R2, then straight to the check
against that PeerId, with no R3). Back leaves the app. K1 shows again
on the next launch until the restore succeeds.

If the record names no readable PeerId (its field is missing or
unreadable, or the entry is not a file), K1 has no primary action. It
shows `IdQTitleNoPath`, `IdQReasonNoPath` and `IdQNoPath` in place of
its usual title, reason and `IdQLost`, because the words cannot help
there. The phrase-only path (R2 and then
R3) is never offered over a record that exists. The custody design
allows that path only where no record survives, and
`IDENTITY-RECOVERY.md` refuses to replace an established profile
without a matching old PeerId. This case gets no path of its own
(§11a Q6).

The person never gets a "start over with a new identity" action here,
because a new identity would replace an established profile
(`IDENTITY-RECOVERY.md`, ADR-0042; §11 Q4). K1 names the one way out
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

**S2, Your 24 words.** The same two pages as C2: the words, then the
PeerId on a page of its own. On the words page the primary action is
"Next" (`IdNext`, → the PeerId page), and Back is the same as Done.
The PeerId page shows the PeerId with `IdPeerIdNoteShown` (the person
has written it before); its primary action is Done, and its Back
returns to the words. Done drops the words, restarts the service and
returns to Settings. "S2's Done" below is that Done, and the time "on
S2" counts both pages. Connectivity then shows as it always
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
   - When the letters typed are themselves a whole word, that word is
     a chip at the row's start, beside the count or the other chips.
     Six words need this, because each is a whole word and also the
     start of five or more others: act, can, car, cat, man and win.
     "car" alone matches seven words, and any fourth letter leaves it
     behind, so without its own chip it could never be entered.
   - Every other word is unique by its first four letters, so the row
     comes down to four or fewer by the fourth letter, and often
     sooner.
   - Tapping a chip fills the position and moves to the next empty
     one. Nothing ever fills itself: even a single remaining match is
     a chip the person taps, so no word is entered without being seen.
3. **The keypad:** the 26 letters in QWERTY rows (10, 9, 7). Its last
   row holds two keys: one deletes the last letter, one clears the
   position.
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
   - at a font scale over 130 % the grid is replaced by a "show all my
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
comparison a matter of groups, not of single characters.

## 6. States

| State | Where | What shows | Way out |
|---|---|---|---|
| Checksum fails (a wrong word or a wrong order) | R2 | A banner above the grid (`IdChecksumFailed`): the words are not a valid phrase; check each word and its order against the paper. Every word stays. | Edit any position, then check again |
| Another identity (with an expected PeerId) | R2 after K1 | A banner (`IdOtherIdentity`): these words belong to another identity, with the derived PeerId in full (§5) and the expected one under it, so the person can see which backup they used. Nothing is restored. | Edit and check again, or Back |
| Authentication needed | S1, whatever the policy. Under `user-presence`, every key operation outside the 60 s window of the last authentication: C4's seal, a restore (R3's yes, R2's check after K1), the read-back after a refused write and each Try again of it, and the service start after S2's Done and R4's or C4's continue | The system prompt, raised by the screen before the operation, never by the service. Cancelled or denied: nothing shown and no error. C4 and the restores stay on their screen with the words kept, and nothing is committed. S2's Done drops the words and returns to Settings offline; connectivity and the profile's user-presence diagnostic say why (`android-key-custody.md`, Unlock policies) | The primary action again, or Back |
| Profile busy (`ProfileLocked`) | Any step that stops the service or writes | The progress state (`IdGoingOffline`) for up to 10 s. Then a notice (`IdBusy`) that the app could not take the identity offline, with Try again. Entered words stay. | Try again, or Back |
| A refusal that may follow the write: read back | C4's continue (the seal), R3's yes, R2's check after K1 | Only two refusals can come after the record was written: `OldKeyRemains`, and `Storage` whose inner error is `Unsynced` (a directory sync after the publish, `persist.rs`). For these two only, the screen reads back with `unlock` before it shows anything: does the record now open to the PeerId just shown or confirmed? A name alone is not enough, because after K1 the old record already names that PeerId. Every other refusal wrote nothing and shows its own row in this table (another identity, authentication needed, profile busy, or any other refusal). The builder must see `Storage`'s inner error to tell the two apart | — |
| Read-back: it opens to the PeerId just shown or confirmed | after those two refusals | Success: C4 and the restores go on as below. The app retries the retire at its next start (`reseal`), with one diagnostic line and never a word | Continue |
| Read-back fails, or the record does not open to that PeerId | after those two refusals. The record is always there after them (both follow the publish, and nothing removes it), so no row for a missing record is needed. A record opening to another PeerId cannot follow this flow's own write while it holds the profile lock (§2); should one appear anyway, the app cannot tell what happened, so it is this row | `IdStorageUnknown`: the app cannot tell whether the identity was saved. Try again reads back again before writing again. Entered words stay while the screen is open | Try again, or Back (words dropped) |
| Any other refusal: nothing was written | C4, R3's yes, R2's check after K1 | Every refusal no other row names. That is `Storage` with any inner error but `Unsynced` (an I/O error, a directory or lock file that is not private, an unsupported platform); `Cipher` other than `UserNotAuthenticated` (the Keystore unavailable, a key invalidated, missing or refused); `CipherShape`; `Identity`; and `Unreadable` (a record that cannot be read, at R3 or after K1). All come before the publish, so `IdStorageFailed`: nothing was changed, with Try again. A restore after K1 never reaches R4 this way | Try again, or Back (words dropped) |
| Another identity already on record (`AlreadyProvisioned`; at a restore, a record naming another PeerId) | C4's continue; at a restore, R3's yes and R2's check after K1 | The record was there before this flow's write. `recorded_identity` names the PeerId the phone holds. `IdAlreadyCreated`: the words just shown were not used; Continue opens that identity. At a restore, a record naming another PeerId shows `IdRestoreOther` with that PeerId | Continue |
| The record names another identity (`RecordNamesOther`) | R2's check after K1 | Not reachable while the flow holds the profile lock: K1 offers R2 only with the record's own PeerId (§3.4). If a build reaches it, the another-identity row above applies (`IdRestoreOther`) | Continue |
| Success | C4 after its seal; R4 after a restore | The PeerId, and continue | Continue |

Errors appear inline on the screen they belong to, never in a snackbar.
A snackbar times out, and these need the person's action. Their text
is in `error` on the screen's surface. A banner uses
`surface-container` with an `error` icon, as the look spec's notices
do.

## 7. Leaving, Back and interruptions

- **Back with words entered (R2, C3):** a dialog asks whether to
  delete the words entered (`IdDiscardTitle`). The actions are delete
  and continue entering; continue entering is the default focus. With
  nothing entered, Back goes straight back.
- **The app leaves the screen.** Home, a call, or another app opening.
  A stopped Activity is neither completion nor cancellation (§11 Q1):
  - on return, every screen shows what it showed, with entered words
    kept in memory only, never in saved-instance state;
  - S2 shows its words again with no new prompt while it is the same
    Activity instance; a new instance is a new display, behind the
    prompt.
- **Finish, cancellation, completion or process death** drops the
  words. After process death the flow restarts at its first screen (W,
  K1 or Settings), with nothing entered and nothing half-done
  committed.
- **Rotation** keeps everything on screen, entered words included. A
  rotation is not leaving.

## 8. Focus and keyboard

The screens are workable without touch: a hardware keyboard or switch
access.
- Focus order follows the reading order: app bar, position, letters,
  suggestions, keypad, grid, primary action.
- A typed hardware letter acts as its key. Enter takes the only
  suggestion when exactly one is shown, or the whole-word chip when the
  letters typed are a word.
- After a chip fills a position, focus goes to the keypad's first
  enabled key.
- After an error banner appears, focus goes to the banner.
- After a dialog closes, focus returns to what opened it.
- Between the two pages of C2 or S2: going forward, focus goes to the
  PeerId page's title, so the reading starts at the top; Back to the
  words page returns focus to the primary action that left it.
- Every disabled key keeps its place in the focus order and is marked
  disabled, not skipped, so moving through the keypad stays
  predictable.

TalkBack does not reach the app yet (plan §20, Decided 2026-10-18). The
screens are built so that it can later: every key, chip and position is
a labelled control. A screen reader may speak a focused word
control's own label; no live region, announcement, hint or error text
ever carries a word (§11 Q2).

## 9. Strings

The text is final (language-culture, 2026-10-19). No string contains
a word of a person's phrase, and no placeholder can hold one: `{n}`
(a number, in digits) and `{peer_id}` are the only placeholders. The
words are plain and calm. A person on K1 did nothing wrong, so no
string there blames them or names a fault code. A PeerId is called
"PeerId", as in the trust settings. No string uses "keep" as an
ordinary verb, because Keep is the name of a message action. Key
names are suggestions; rust-ui-dev names them.

| Key | Text | Where, and why this text |
|---|---|---|
| `IdWelcomeTitle` | Welcome to InterWeave | W's title |
| `IdWelcomeBody` | Exchange messages with the people you trust, from this phone. There is no account to create. | W's body, two lines. No "directly": a message may go through a relay |
| `IdVoidPaper` | The words shown last time were never used. They are not your identity, so do not use that paper as your backup. | W's notice while the void-paper marker is set (§3.1), three lines. It says what the old paper is not, and what not to do with it |
| `IdAlreadyCreated` (new) | This phone already has an identity, so the words you just saw were not used. Do not use that paper as your backup. | C4 when another first run sealed first (`AlreadyProvisioned`, §6). `IdVoidPaper` says "shown last time", which is false on C4 |
| `IdCreate` | Create a new identity | W's primary |
| `IdRestore` | Restore my identity | W's secondary |
| `IdC1Title` | About your recovery phrase | C1's title |
| `IdC1Point1` | These 24 words are the key to your identity. With them, you can move it to a new phone. | C1, first point: a move to a new phone, not a second phone in use at the same time |
| `IdC1Point2` | They are an InterWeave identity recovery phrase, not a cryptocurrency wallet seed. | C1, second point, in the order of §3.2's list (`IDENTITY-RECOVERY.md`) |
| `IdC1Point3` | Write them on paper, in order, with your PeerId. Store the paper in a safe place. | C1, third point. The PeerId is named here, because R1 asks for it later |
| `IdC1Point4` | Anyone who has these words can act as you in InterWeave. Do not show them to anyone. | C1, fourth point: "can be you", without alarm |
| `IdC1Point5` | The words bring back your identity only, not your contacts, trust, settings or messages. | C1, fifth point: `human-client-ui.md` §9 as amended (2f3b0587). It names the fact and asks for nothing, since Android has no export of the rest |
| `IdC1Point6` | Your identity exists only after you finish, by checking two of the words. | C1, sixth point (§11 Q3) |
| `IdShowWords` | Show my words | C1's primary |
| `IdNotWalletSeed` (new) | InterWeave identity recovery phrase, not a cryptocurrency wallet seed | C2 and S2, above the words. One line at 100 % only on a wide screen: 68 characters. The contract's label also says "transport"; it is left out as a word the person does not know |
| `IdFormatLine` (new) | Phrase format: | C2 and S2, before the untranslated format identifier (§3.2), one line. "Phrase format" says whose format it is; a bare "Format" could be read as the screen's layout. The label stands alone before a value, so a translation needs no template |
| `IdStorageUnknown` (new) | The app could not tell whether your identity was saved. Try again to check. | §6's notice when the read-back fails, two lines, with `TryAgain`. It never says that nothing changed. "To check" is true, because Try again reads back before it writes |
| `IdRestoreOther` (new) | Your identity was not restored. This phone has another identity, with the PeerId below. | §6's another-identity-on-record row, at a restore (a record that names a PeerId other than the one restored), two lines, the PeerId in full below it. "With the PeerId below" ties the PeerId to the other identity, so it is not read as the person's own |
| `IdWordsTitle` | Your 24 words | C2's and S2's title |
| `IdPeerIdLabel` (new value) | Write it on the same paper as your words. It is not a password, and it is safe to share. | The note under the PeerId (§5) on C2's PeerId page, where the person writes. It no longer starts with "Your PeerId": the page's title says that just above it (language-culture-en, REPLY 01a1589d-e769). The page's title is the PeerId's accessible name; this note is its description |
| `IdPeerIdNoteShown` (new) | It is the one on your paper. It is not a password, and it is safe to share. | The note under the PeerId on S2's PeerId page, whose title already says "Your PeerId"; the title is the PeerId's accessible name |
| `IdPeerIdTitle` (new) | Your PeerId | The title of C2's and S2's PeerId page |
| `IdPeerIdWritten` (new) | I wrote down the PeerId | The primary of C2's PeerId page (→ C3) |
| `IdNext` (new) | Next | The primary of S2's words page (→ its PeerId page) |
| `IdPeerIdLabelShown` (new) | Your PeerId. It is the one on your paper. It is not a password, and it is safe to share. | The PeerId's label on C4 and R4, where the person has already written it and no title above names it. One label cannot be true on both kinds of screen |
| `IdWrittenDown` (new value) | I wrote down the words | The primary of C2's words page; the PeerId has its own page now |
| `IdCheckTitle` | Check your words | C3's title |
| `IdCheckBody` | Enter two words from your paper. | C3's body, one line |
| `IdWordPosition` | Word {n} of 24 | The picker's position (§4), and C3's heading of each entry |
| `IdWordMatches` | {n} words fit these letters | The suggestions row while more than four words match (§4). {n} is then always more than four, so English needs one plural form; a translation that needs more gets them by key |
| `IdCheckWrong` | This does not match word {n}. Check word {n} on your paper. | C3's inline error. It names the position and what to do, and says nothing about the word typed |
| `IdShowAgain` | Show my words again | C3's action after an error |
| `TryAgain` (exists) | Try again | C3's other action after an error, and the retry action of §6's notices. The key exists, and a second key with the same text would fail `every_ui_text_appears_once_in_all`. One action has one name, so §6 says "Try again" too |
| `IdCreatedTitle` | Identity created | C4's title |
| `IdRestoredTitle` (new) | Identity restored | R4's title. "Identity created" would be false after a restore |
| `IdCreatedBody` | When you continue, the app connects to the network with this identity. | C4's and R4's body |
| `IdContinue` | Continue | C4's and R4's primary |
| `IdR1Title` | Restore your identity | R1's title |
| `IdR1Body` | You need your 24 words, and the PeerId that goes with them: from your paper, or from a contact who has it. The app stays offline until the restore is done. | R1's body, three lines. "The PeerId that goes with them" is true whether the person moved phones or reinstalled on the same one |
| `IdEnterWords` | Enter my words | R1's primary |
| `IdCheckWords` | Check my words | R2's primary |
| `IdR1Settings` | Your words restore your identity only. Contacts, trust, settings and messages are not part of them. | R1's line under the body, two lines: the same statement, with nothing asked |
| `IdR3NoPeerId` | If the PeerId is not on your paper, ask a contact for the one they have for the phone that had these words. Restore only if it matches. | R3, under the question: about three lines at 100 %, one over the space. A contact can hold one PeerId per device, so the text names which one |
| `IdR3Question` | Is this the PeerId that you wrote down? If it is not, a word may be wrong. | R3's question, two lines. It asks; it never says the PeerId is right, and it says what a "no" usually means |
| `IdR3Yes` | Yes, restore it | R3's primary |
| `IdR3No` | No, check my words | R3's secondary |
| `IdQTitle` | This phone needs your recovery phrase | K1's title |
| `IdQReasonKey` | This phone's screen lock or security changed, so the app cannot open your identity. Restore it with your recovery phrase. | K1 for `KeyInvalidated` and `KeyMissing`. It names the likely cause in the person's terms, and the way back |
| `IdQReasonRecord` | The app cannot read your identity on this phone. Restore it with your recovery phrase. | K1 for `Record`, `Authentication` and `WrongIdentity` |
| `IdEnterPhrase` | Enter my recovery phrase | K1's primary |
| `IdQLost` | If your words are lost, clear this app's data in the phone's settings. That starts a new identity, which each contact must trust again. | K1's plain text below its primary (§3.4), three lines |
| `IdQTitleNoPath` (new) | The app cannot open your identity | K1's title when the record names no readable PeerId (§3.4). `IdQTitle` asks for the phrase, which cannot help there |
| `IdQReasonNoPath` (new) | The app cannot read your identity on this phone, and it cannot be restored here. | K1's reason in the same case. Both other reasons end with "Restore it with your recovery phrase", which is false there |
| `IdQNoPath` (new) | To use the app again, clear this app's data in the phone's settings. That starts a new identity, which each contact must trust again. | K1's plain text in the same case, in place of `IdQLost`, whose "If your words are lost" suggests the words would help |
| `IdExpectedPeerId` (new) | PeerId this phone expects | The label of the expected PeerId, on K1 and in `IdOtherIdentity`'s banner |
| `IdDerivedPeerId` (new) | PeerId from your words | The label of the derived PeerId in `IdOtherIdentity`'s banner |
| `IdShowPhraseItem` | Show recovery phrase | The Settings item (§3.5) |
| `IdS1Title` | Show your recovery phrase | S1's title |
| `IdS1Body` | You will be asked to unlock your phone first. The app goes offline while your words are shown. | S1's body, two lines |
| `IdGoingOffline` | Going offline… | The progress state (§3.5, §6) |
| `IdDone` | Done | S2's primary |
| `IdChecksumFailed` | These words do not form a recovery phrase. A word may be wrong or in the wrong place. Check them against your paper. | §6's checksum banner |
| `IdOtherIdentity` | These words open a different identity, so nothing on this phone was changed. Compare the two PeerIds below with your paper. | §6's other-identity banner, above the two labelled PeerIds |
| `IdBusy` | The app could not go offline. Wait a moment, then try again. | §6's busy notice, with `TryAgain` |
| `IdStorageFailed` | The app could not save your identity on this phone. Nothing was changed. | §6's storage notice, with `TryAgain` |
| `IdDiscardTitle` | Delete the words you entered? | §7's dialog title |
| `IdDiscard` | Delete words | §7's dialog: the action that drops the words |
| `IdContinueEntering` | Continue entering | §7's dialog: the default action |
| `IdKeyDelete` | Delete last letter | The keypad's delete key (its accessible label) |
| `IdKeyClear` | Clear this word | The keypad's clear key (its accessible label) |
| `IdReviewWords` | Show all my words | The grid's button at large font (§4) |

Against the list the proposal gave: `IdC1Points` is six keys, one per
point in the order shown, so each wraps alone. `IdPeerIdLabelShown`, `IdRestoredTitle`,
`IdExpectedPeerId`, `IdDerivedPeerId`, `IdAlreadyCreated`,
`IdQTitleNoPath`, `IdQReasonNoPath`, `IdQNoPath`, `IdNotWalletSeed`,
`IdFormatLine`, `IdStorageUnknown` and `IdRestoreOther` are added. `IdTryAgain` and
`IdRetry` are both `TryAgain`. `IdKeepEntering` is
`IdContinueEntering`.

## 10. Acceptance

On the Pixel 9a, dark and light, at 100 % and 200 % font, portrait and
landscape:

1. A first launch with no record shows W. Back leaves the app, and the
   next launch shows W again.
2. Create: C1 → C2 shows 24 numbered words and no PeerId. Its primary
   leads to the PeerId page, which shows the PeerId in full, grouped
   by four, and no word; Back there shows the same words. S2 has the
   same two pages. A screenshot (`adb exec-out screencap`) of both C2
   pages, of C3 and of R2 is black, and Recents shows no content for
   the flow.
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
   - typing "car" shows a "car" chip beside the count of seven, and a
     tap enters "car"; the same holds for act, can, cat, man and win;
   - a chip tap fills the position and advances;
   - no word fills itself;
   - no IME appears at any time (`dumpsys input_method` shows no
     input shown).
7. Fresh-install restore with a known fixture phrase: R2 → R3 shows
   its PeerId. "No" returns to R2 with all 24 words. "Yes" restores,
   and the PeerId on the Settings screen matches.
8. A pinned fixture phrase of distinct words, with a pinned swap of two
   positions that the test first shows fails the BIP-39 checksum (an
   8-bit checksum passes about one swap in 256), shows
   `IdChecksumFailed`, keeps every word, and lets any position be
   edited. The fixture and the swap are named in the test.
9. Recovery required: with the record's key invalidated (rust-ui-dev's
   test hook), launch shows K1 with `IdQReasonKey` and the expected
   PeerId. The right phrase restores with no R3. Another identity's
   phrase shows `IdOtherIdentity` with both PeerIds and restores
   nothing. With the record's PeerId field made unreadable (a test
   hook), K1 shows `IdQTitleNoPath`, `IdQReasonNoPath` and `IdQNoPath` with no primary action, and
   no R2 or R3 is reachable. A restore that answers `OldKeyRemains`
   (a test hook) reaches R4, and the next start retries the retire.
   A K1 restore whose write fails before the publish (a hook) shows
   `IdStorageFailed` and never R4, and the next start shows K1 again.
   Once custody's PeerId-field reader lands (§11b Q8), a record refused
   for its shape (a hook damages its magic) with an intact field shows
   K1 with that PeerId and restores only to it.
   A C4 that answers `AlreadyProvisioned` with another identity on
   record shows `IdAlreadyCreated`. A C4 whose directory sync fails
   after the record is written (a test hook) reads the record back and
   reaches the Chats screen as a success, never `IdStorageFailed` or
   `IdAlreadyCreated`; so does a C4 that answers `OldKeyRemains`.
10. Show my phrase:
    - a cancelled prompt shows nothing and no error;
    - a succeeded prompt shows `IdGoingOffline`, then the words;
    - connectivity reads offline while they are shown;
    - Done restarts the service and returns to Settings;
    - on a `user-presence` profile, with more than 60 s on S2, Done
      raises the prompt before the restart. A cancelled prompt returns
      to Settings offline, with the words gone. The same profile's C4
      and restore raise the prompt outside the window, and a cancel
      there commits nothing.
    - After a cancel, the person is offline with the locked notice and
      Unlock; nothing raises the prompt again by itself. A return to
      the same Activity (Home or Recents, then back) raises no prompt:
      it binds silently if the window has opened meanwhile, and
      otherwise keeps the notice and Unlock. Leaving while the system
      prompt shows counts as a cancel. A cold launch past the window
      raises the prompt before anything binds (rust-ui-dev-01,
      QUESTION 01a155b3-448f; the person who declined is not asked
      again each time they switch in: S4, user control and freedom).
11. Leaving R2 with words entered (Home) and returning shows every
    word still entered. After the process is killed (`adb shell am
    kill` while in the background), the next launch starts at the
    flow's first screen with nothing entered. Rotation keeps every
    word. Leaving S2 and returning to the same instance shows the
    words with no new prompt.
12. Profile busy and storage failure, by rust-ui-dev's test hooks, show
    §6's notices. A refused write whose read-back fails (a hook) shows
    `IdStorageUnknown`, never "nothing was changed". Try again works, and Back leaves with nothing changed.
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
    single suggestion and on the whole-word chip ("car"). Focus follows
    §8.
16. No log line, notification, crash report or test snapshot from a
    run of items 2–11 contains a fixture word. That is checked by
    `adb logcat -d` searched for the fixture's 24 words, and by the
    test harness.
17. The profile lock is held from C2 to C4's seal and from R2 to the
    restore, and is never released in between. A service start
    attempted then (a hook starts it while each of C2, C3, R2 on the
    fresh-install path, R2 on the K1 path, and R3 is open) is refused
    as `ProfileLocked` and starts nothing, and the seal or restore that
    follows succeeds on the handed-over lock. The same holds from the
    moment S1's progress state has stopped the service until S2's Done,
    which releases the lock before the restart. After a refused seal or
    restore (a hook injects a Storage I/O error before the publish, at
    C4 and at a restore, which shows `IdStorageFailed`), a service start
    tried while the notice is open is refused as `ProfileLocked`, and
    Try again succeeds on the same lock. After a success, the service starts. This item waits on
    §11b Q9 and Q10 in the crate.

## 11. Questions answered

architect-cto, 2026-10-19 (REPLY 01a15282-ac06), against
`android-key-custody.md` and ADR-0042 and ADR-0033:

- **Q1. Is a stopped Activity "leaving the screen"?** No. The words stay
  in process memory only, never in saved-instance state. They go on
  finish, cancellation, completion or process death. An on-demand
  display does not ask its prompt again on resume of the same Activity
  instance (§7). The custody doc says both (425ec5d0, 2f3b0587).
- **Q2. What may a screen reader speak?** A focused word control's own
  label, which is the secure screen read aloud at the person's request.
  No live region, announcement, hint or error text ever carries a word
  (§8). 425ec5d0 says this in the custody doc.
- **Q3. Commit at C4?** Yes. The seed is generated in memory, the phrase
  is shown and verified (C3), and the seal happens on the last screen,
  straight after the person's confirmation. Back or process death
  before it leaves a first run and no record. C1 says that the identity
  exists only once creation is complete.
- **Q4. A new identity from K1?** Not in v1. K1's text names the way
  out instead (`IdQLost`). Whether v1.x gets an in-app start-over path
  (delete the whole profile, then provision; never mint over the
  record) is the owner's product call. architect-cto carries it by
  name in the Stage 17 closing record.

## 11a. Answered after #303's review

architect-cto, 2026-10-19 (REPLY 01a1529f-5231). The doc text is
2f3b0587 on the custody branch named in the header, not yet merged.

- **Q5. A settings backup on Android?** None in standard v1: no export
  exists, system backup and device transfer exclude the profile, and
  profile and config migration is Stage 19's. `human-client-ui.md` §9
  now says that a platform without an export states what the phrase
  restores and asks for nothing more. C1 and R1 do that.
- **Q6. A path for K1 with no readable PeerId?** None. A record that is
  present but names no readable PeerId belongs to an established
  profile, so it joins the lost-phrase case (§3.4, `IdQNoPath`). An
  in-app start-over is the owner's product item, with Q4's. The custody
  doc now says such a record is not "no record". The custody code's
  contrary branch goes to its owner as architect-cto's finding.
- **Q7. Does the wallet-seed label bind its words?** Its meaning
  (REPLY 01a152a1-54f4). The phrase restores this app's identity and is
  not a wallet seed. `IdNotWalletSeed` and `IdC1Point2` keep both
  facts without the contract's "transport". `IDENTITY-RECOVERY.md` now
  says so (497d6b42, on #301's branch).

## 11b. After #303's round 2 (Q8 and Q9 answered, Q10 open)

architect-cto, 2026-10-19 (REPLY 01a152aa-64d6), ruled into the
custody doc at 252a6f45 and asked of p2p-network-dev for the crate:

- **Q8. A public reader of the record's PeerId field.** Yes, with
  `named_peer`'s semantics, whatever the envelope's state. The cipher
  never decides whose record it is (§3.4).
- **Q9. Handing the profile lock to custody.** A handover, not a gap.
  The flow takes the lock once, before the first word, and hands it to
  the seal it ends in; `provision` and `restore` are to accept a held lock
  (§2).
- **Q10. The lock after a refusal.** Open: a refused `provision` or
  `restore` hands the lock back to the flow, for the read-back and Try
  again. Sent to architect-cto for the crate request (INFO
  01a152bf-8623), not yet confirmed. It is not part of REPLY
  01a152aa-64d6.

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
