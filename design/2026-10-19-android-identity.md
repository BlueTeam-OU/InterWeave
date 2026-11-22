# Android identity screens: create, show the phrase, recover

Status: proposal for plan §20 steps 7–8, 2026-10-19. rust-ui-dev builds
it on `develop-qzapp/rust-ui-dev-01/feat/j111-android-recovery`. The
four questions in §11 were answered by architect-cto on 2026-10-19.
The custody doc's text behind them (ecf20942..252a6f45) is on
architect-cto's branch
`develop-qzapp/architect-cto-01/for/rust-ui-dev-01/custody-phrase-display`
and is not merged; it lands with rust-ui-dev's build PR.
Revised 2026-10-23: what the phrase holds is the PeerId on every
screen and "identity" names only the person's identities (§2, §3,
§9), at architect-cto-01's REPLYs 01a16736 and 01a1673d.
Revised 2026-11-03: Back and a tap outside on the discard dialog (§7),
what Enter and Space do (§8) and the full focus order (§8), with
acceptance 15, at rust-ui-dev-01's QUESTION 01a1a324 (REPLY 01a1a331).
Revised the same day: in portrait R2's grid sits above a fixed picker
(§4, §8, acceptance 13), at rust-ui-dev-01's height figures (REPLY
01a1a333). And: what "Show all my words" opens (§4 item 4, §8,
acceptance 2 and 15), at rust-ui-dev-01's QUESTION 01a1a34d.
Revised 2026-11-12: a tap on an enabled key is typing, a tap on text
leaves the mode as it was, and a tap moves focus only where §8's
rules say so; a chip or
a position chosen by any input returns the screen to typing, and a
focused chip that the letters remove hands focus to the keypad (§4
item 2, §8, acceptance 15), at rust-ui-dev-01's QUESTION 01a1d095.
Before this revision, a tap set navigating, so a hardware Enter after
tapping t pressed the key that held focus, a key the person had not
chosen (#644).
Revised later on 2026-11-12, at the owner's ask, for two gaps that
#645's review rounds carried. First, after a wrong word on C3, focus
goes to `IdCheckWrong`, not to the keypad. Before, the chip rule and
the error-banner rule each named a target, and neither said which
won. The rules now also say where focus goes when Try again removes
the error (§3.2, §8). Second, on R2 the "next empty" position now
wraps after 24 to the lowest empty one. When a chip leaves no
position empty, focus goes to the primary action, `IdCheckWords`,
with the screen navigating. Before, §4 named no next position for
that case, and §8 named no focus (§4 item 2, §8, acceptance 4 and
15).
Revised 2026-11-13: a screen reader's activate action on a key is a press,
not a tap, so it sets navigating (§8, acceptance 15), at
rust-ui-dev-01's QUESTION 01a1d261 (REPLY 01a1d262). And, once #651
held R4 after Continue, acceptance 15 step 3 expects Connect or Chats
as the first page after R4, never Chats then Connect.
Revised 2026-11-16: while R2's check runs, the words are held, Back
does nothing until the result, and focus does not move (§6 Check
running, §7, §8, acceptance 15), at rust-ui-dev-01's QUESTION
01a1e180 (provisional REPLY 01a1e181). Before this revision, §8 said
nothing of a running check, and a build
(#682) let a chip fill a position while the check ran.
Revised 2026-11-16, after #683: "focus does not move" meant that the
check moves no focus. Tab, Shift+Tab and the arrows still move focus
while the check runs. A valid phrase puts focus on R3's question,
`IdR3Question`, because the question is what R3 asks (§3 gives R3 no title of its own). A screen reader's activate on
`IdCheckWords` is a press. If the hypothesis that only `IdCheckWords`
shows disabled is falsified, every held control shows disabled. Before
this revision, that fallback named only the keys (§8, acceptance 15). This revision answers #683's review (F12, F13) and
language-culture-en's two questions on it.
Revised 2026-11-17: when R3's yes opens R4, focus goes to R4's title,
as #698 builds it (§8, acceptance 15), at rust-ui-dev-01's QUESTION
01a1e76e (provisional REPLY 01a1e76e-e7f0). Before this revision, §8
named R4's focus only for R4 opened after K1.
Revised later on 2026-11-17, after #693's full review (F11, and the
problems it found in text that was already there). While R3's restore
runs, `IdR3Yes` and `IdR3No` show disabled, Back does nothing and the
restore moves no focus (§6 Restore running, §7, §8). Before, nothing
held R3 while its restore wrote the record, so a second press of
`IdR3Yes`, or a press of `IdR3No`, could act during the write. A §6
notice on R3 shows under the app bar, above the question. In R3's
focus order, which §8 now gives, the notice comes right after the app
bar (§6, §8). Before, §6 placed notices only on R2. Acceptance 15
gains runs for a held restore, a refusal and a cancelled prompt at
R3's yes, and checks R3's Tab order with no notice and with
`IdStorageFailed` shown; acceptance 12 verifies, on the device, where
the notice shows on R3. The typing and navigating modes are R2's and C3's only, and
"R2 and C3 open as typing" replaces "a screen opens as typing". The
hypothesis for R2's held controls now names the back arrow, drawn
inactive beside `IdCheckWords`.
Revised again on 2026-11-17, for what #700's review rounds carried. §6 says
where Back from a notice goes on each screen (Back from a notice):
on R3 it returns to R2 with every word kept, as §3.3 says. Before,
two §6 rows said "Back (words dropped)" and acceptance 12 said
"Back leaves with nothing changed". R3's Back, which keeps every word
and returns to R2, contradicted the rows. R2's Back after K1, which can
drop the words through the discard dialog, contradicted acceptance 12. A
§6 notice on C4 shows below the title, above the PeerId's label, and
§8 gives C4's focus order, with focus on the title when C4 opens, as
on R4. Before, neither was given. On C4 no notice has an action of its
own; C4's one action is its primary. While `IdBusy`,
`IdStorageFailed` or `IdStorageUnknown` shows, Try again takes
Continue's place. While `IdAlreadyCreated` shows, Continue opens the
identity on record and the unused PeerId is hidden. Acceptance 7 checks R1's order on the screen and marks
the accessibility-tree check blocked, as acceptance 12 does. Before,
it read the order from a `uiautomator dump`, which sees no node of
the app on Android. For the same reason, acceptance 15's
launch-count run now reads K1 and R2 on the phone, not from a dump.
Acceptance 15 gains four runs for the running-check rules that had
no step: focus after a check started by a tap, Try again on an R2
notice, a cancelled prompt, and "show all my words" held while the
check runs. One of its existing runs now also checks the mode after
an input that does nothing.

Revised later on 2026-11-17, for a question from the build: §5 gave
"Copy PeerId" a `PeerIdCopied` snackbar below Android 13 and said
nothing of where it shows, how long, whether it takes focus or is
announced, or on which screens. §5 now says each of these (The copy's
confirmation), §8 keeps it out of every focus order, and acceptance
18 checks it, and checks its look on the Samsung A40, at Android 11.

Revised 2026-11-18, for what #705's review rounds carried. While C4's
seal runs, C4 is held as R3 is while its restore runs: its primary
shows disabled, a second press seals nothing, `CopyPeerId` still
copies wherever it shows, and the seal moves no focus (§6 Seal running, §8). The hold
lasts until the result shows on C4 or the next screen opens, so it
also covers the service start after a success, and the service start
after Continue while `IdAlreadyCreated` shows. Before, only
`design/2026-10-22-add-a-peer.md` §5.1 held Continue, and neither that
note nor this one held Try again or said what `CopyPeerId` does while
C4 is held. Nor
did either say that Tab and the arrows still move focus while C4 is
held, that, when the result comes, focus goes as §8 says even if the
person moved it, or that a cancelled or denied prompt for the seal or
its read-back leaves focus where it was before the prompt. On R3, as on C4, a press
of the screen's primary while the snackbar shows does not close it
(§5). Before, §5 said so of C4 only. §5 now also says what a press of
`CopyPeerId` does while a §6 notice shows, and what a rotation does to the
snackbar. After `IdAlreadyCreated`, the app removes from the
clipboard the PeerId copied on C2's PeerId page or C4, if the clipboard still holds it
(§6). Before, the clipboard kept a PeerId that no phone will ever
hold. While `IdAlreadyCreated` shows, C4's title no longer says that
a PeerId was created (`IdAlreadyTitle`, §9), `IdAlreadyCreated` no
longer opens with what that title says (§9), and the notice shows
below the title, above Continue (§6). §9's `TryAgain` row now names
C4's primary. Acceptance 18 now says when Continue is pressed, so a
build that starts the snackbar's time again at Continue fails it. It
also checks R3's yes, a copy beside a notice, a rotation, and where
the scrolling area ends after the snackbar closes. Its A40 runs set
the font scale to 200 % with the `settings` command, because it is
not known whether the A40's own settings offer 200 %. Acceptance 15 gains three host-test runs for a held seal.
In the revision note of 2026-11-17 on Back from a notice, the
sentence on what Back contradicted is made exact. §5 now also says
that `ConnectStarting`, on C4 and R4, closes the snackbar, and that a
copy made while `ConnectStarting` shows keeps the snackbar and
`ConnectStarting` on screen. §8 now
says that a cancelled or denied prompt for the seal or its read-back
leaves C4 as it was, and that one for the service start ends on
Chats, as §6 says. Acceptance 3 names the clipboard removal, and
acceptance 9 names `IdAlreadyTitle` and the notice's place.
Acceptance 18 gains three host-test clipboard runs, each made below
Android 13 and again at 13 or later, and a Pixel 9a check that is blocked
until a debug-build hook exists. Its A40 run states, as a hypothesis,
that in landscape at 200 % the area above the snackbar still holds
`CopyPeerId` wholly, with what falsifies it. §12 gains two items: the
clipboard is cleared only on C4 under `IdAlreadyCreated`, not after
R3's `IdRestoreOther`, and no words say that the copy was removed.

Revised later on 2026-11-18, for what #711's review rounds carried
(PR comment 6973094962). On R4, as on C4 and R3, a press of the primary
does not close the snackbar and its time goes on (§5). Before, §5
named C4 and R3 only. A Try again that ends in the notice it was
pressed in closes the snackbar when that result shows (§5). Before,
§5 did not say whether the same notice shown again counted as
showing after the snackbar. §5 points to
`design/2026-10-22-add-a-peer.md` §5.1 for when `ConnectStarting`
shows. Before, §5 said "1 s after the person presses Continue", which
a system prompt makes false, since a prompt makes the 1 s count again
from zero.
While the progress state shows, `CopyPeerId` copies nothing (§5, §6,
§8). Before, the hold rules said that `CopyPeerId` still copies, and
did not say whether that was still true under the progress state. After
`IdRestoreOther`'s Continue, on R3 and on R2 after K1, the screen is
held until the next screen opens, as C4 is (§6, §7, §8). Before,
that Continue started the service with the screen not held. The
add-a-peer note's §5.1 now names that Continue, and C4's Try again in
Continue's place, for `ConnectStarting`. §5's snackbar rules name that Continue
and its `ConnectStarting` as they name C4's and R4's, and on R2 every
held control shows disabled during the service start that Continue
makes. That start can last up to add-a-peer §5.1's 10 s bound, and
longer while a system prompt is up (§8). §6 now says that the
clipboard removal reaches the system clipboard only, not a keyboard's
own clipboard history, and that, from Android 10, when the seal is
answered `AlreadyProvisioned` while C4 is in the background, the removal waits until C4
has the window's focus again. §12 gains both limits. Acceptance 15 gains runs
for the progress state and for `IdRestoreOther`'s Continue.
Acceptance 18 gains R4's three `ConnectStarting` runs, a Try again
ending in the same notice, and a seal answered while C4 is in the background, and
its Pixel 9a check names the platform's paste.

Revised again on 2026-11-18, for what #715's review rounds carried. §6
now says that the clipboard removal waits whenever C4's window does
not have the focus or the device is locked: in the background, in
split-screen, under the notification shade, or behind a system window.
Before, it named only the background, and only from Android 10. The
platform rule behind it, and the app op that the rule also requires,
were read in the platform's source (`ClipboardService` of Android 11
to 16 and of the platform's main branch). Before, the rule was taken
from memory. That source also shows that the platform withholds a
clip's description while the device is locked, which the note did not
say. §12 now states the
same conditions as §6, the app op included. §5 now says that the
progress state never shows for a service start: it answers custody's
`ProfileLocked`, which the start's own take of the lock never gets
(the reseal that a start retries can get it, and is then skipped,
§5), and the Profile busy row says so. §11c records architect-cto's answer to Q11 (below). §8 now says that the hold after
`IdRestoreOther`'s Continue lasts until the next screen opens, at most
add-a-peer §5.1's 10 s bound from the press, plus any time a system
prompt is up, and that the start goes on after the bound. Before, it said that the start can last up to the
bound. In acceptance 15, the run in which R2 is held after
`IdRestoreOther`'s Continue now types "d", a letter that continues
"car". Before, it named no letter, so the run could pass on a letter
that is disabled anyway. Acceptance 15 also gains a tap run on C4. In
acceptance 18, the fourth clipboard run now takes the focus away with
the app in the foreground, and a fifth run adds the device locked. §9's `IdContinue` row names `IdRestoreOther`'s Continue.
architect-cto answered Q11 (REPLY 01a1efb4-1842): a service start
can be refused because the profile is in use: the start takes its lock with no wait. §6 gains a
row for it, a notice (`IdStartBusy`, new in §9) with Try again, and
acceptance 15 gains its runs. On C4 that Try again starts the service
and seals nothing, and under `IdAlreadyCreated` the screen stays as
that notice left it. A start refused after a seal, and on R2 after
K1, has runs too. A refusal that came after the 10 s bound was not
designed then: §11c recorded it as Q12, which §11d now answers. `IdStartBusy`'s Try again is
pressed as the Continue it replaces, with the screen held from its
press.

Revised later on 2026-11-18, for what #720's review rounds carried
(PR comment 6987338123). Q12 is answered (§11d): a service start
refused with the profile in use after add-a-peer §5.1's 10 s bound
shows `IdStartBusy` as a session notice on Chats or in a
conversation, with Try again, and moves no focus (§6, A start refused
with the profile in use after the bound). A return to the app or a
recreation retries that start, as the unlock note's §11 L3 retries
any start after a refused one, and no timer retries it. Before, that
case was not designed. Acceptance 15 gains its runs. The held Try again run of
acceptance 15 now says that the hook refuses the first Try again
with `LockHeld` again before the held press. It now covers C4 under
`IdAlreadyCreated`, R4 and R2 after K1 as well as C4 after a seal and
R3, and it calls its 60 s a bounded check of a claim that has no end.
Acceptance 12 now checks where `IdStartBusy` shows on C4, R4, R3 and
R2 after K1. In acceptance 18, the fourth and fifth clipboard runs no
longer require that the app make no request for the clip's
description while it waits. §6 asks only that the removal wait, and
the platform gives no description then. §11d records two device
questions that are still open. §11c's heading is again true of what
it heads.

Revised 2026-11-19, at rust-ui-dev-01's QUESTION 01a1f2c7 on the
build of §6's read-back rows. `IdStorageUnknown`'s Try again reads
back and never writes again, on C4, R3 and R2 after K1. While it
shows, `IdR3Yes` and `IdCheckWords` do the same. Before, the row said
"reads back again before writing again", and §8 said that R3's Try
again under it started the restore again: after the two refusals that can follow the write, the record is always there, so only a read-back could answer whether it opens: a second seal would be refused, and a second restore, the row then said, would tell no more (the next revision corrects that). A cancelled or denied prompt for a read-back now shows
`IdStorageUnknown`, on C4, R3 and R2 after K1. Before, §6 and §8 left the screen as it was
before the prompt, with a primary that would write again, though the
write may already have committed the identity. A restore answered
`OldKeyRemains` reads back as a seal does: the row already said so,
and acceptance 12 now checks it with the rest. Later the same day, at
rust-ui-dev-01's QUESTION 01a1f2ff: on R2 after K1, an edit that
removes `IdStorageUnknown` does not make the next check a write. Until
a read-back opens the record to the confirmed PeerId or R2 closes, a
check of words that derive that PeerId reads back, and words of
another PeerId are checked as any words are.

Revised later on 2026-11-19, for what #723's review rounds carried
(PR comments 6989066023 to 7004460557). §6's lists of the notices
that show on R2 and R3 now name `IdStartBusy`, in `IdRestoreOther`'s
place. Before, the profile-in-use row placed it there, but the lists
did not name it. §6 now says that an edit on R2 removes
every banner and notice (An edit on R2), as the build does, and that
the read-back owed outlives the notice. Before, the read-back row
spoke of "an edit that removes the notice" while no rule said whether
an edit removed one. §6 now names the two inputs that make an edit, a
grid position chosen and a chip taken, and says that each names in §8
where focus goes, so focus never stays on a removed notice. §2 now
says that the flows run in the app's own Activity, as the custody and
human-client-ui texts have said since 2026-10-19 (rust-ui-dev-01's
REPLY 01a1f355). Before, it kept the first wording, a separate Activity
excluded from Recents. C4's start after a failed read-back now holds C4,
its own control included, from its press; takes add-a-peer §5.1's
not-owed path, writing nothing; and must open the record to C4's PeerId: the app compares the identity that `unlock` gives with that PeerId, and a mismatch fails closed on C4. Before, the text spoke of "the next screen after
Continue", left the control enabled during its own start, and took
custody's `WrongIdentity` for a comparison with C4's PeerId. After a
mismatch, `IdStartUnchecked` is not offered again while C4 is open,
and the profile-in-use and Authentication needed rows name that start:
the first says its Try again repeats the comparison, the second that
its cancelled start ends on Chats. When `IdStartUnchecked` is offered
is now said once, in §6, and every other mention points to it. The
read-back owed on R2 after K1 now lasts as long as the words do,
through a rotation, a return and a recreation (§6). §6 now says what
a read-back that keeps failing leads to: Back and a new restore on R3
and on R2 after K1, which custody's restore makes safe. On C4 the app offered no way on: only the end of the process led to a new launch; at architect-cto's ruling on Q15 (§11d), C4 now offers a
service start beside Try again after a failed read-back, which leads to
K1 when the record does not open (`IdStartUnchecked`,
`IdStorageUnknownRestore`, new in §9; acceptance 12). §12
leaves out what R3 shows for a restore of other words after a refused
one. Before, the read-back row said that a second restore
could tell the person no more than a read-back. That was wrong for a
record whose key opens nothing, which a restore seals over. §8's
reason why an early press of `IdCheckWords` is safe no longer says
that the check's result depends only on the words: after K1 it also
depends on the record, and on whether a read-back is owed.
Acceptance 15's runs of a start refused after the bound now say what
the test's store holds when a run opens a conversation or scrolls the
list. Its recreation variant now says what the recreated window shows
and what each answer does. Whether Home ends a start in
foreground-only mode is now a device run, blocked until a debug-build hook can leave a start unanswered, with what each finding requires.
Before, that variant took the end of the start from add-a-peer §5.1
and did not observe it. §6 now says when Connect's count of openings
begins for a start retried at a return or a recreation: at that return
or recreation. Before, it said so only for Try again's start. For any
later start, a conversation on screen when it is asked for now counts
as an opening (add-a-peer §5.1). Before, Connect could open over, and
close, a conversation that the person returned into or pressed Try
again in. Acceptance 15 gains runs for both. Acceptance 12 gains runs
for an edit with each banner and notice, for focus when an edit removes the notice that had focus, for the read-back owed through a
rotation, a recreation and a return (each with the notice removed
first, so the run checks the read-back owed and not the notice), and
for a read-back that keeps failing on R3 and on R2 after K1.
Add-a-peer §5.1, where it calls a foreground-only return a launch of
its own, now says that this holds for Connect, and points to §6's How
long, which counts only the Activity created anew; its paragraph on a
launch with Connect still owed names the retry at a return or a
recreation, and a conversation on screen, as above.

Revised 2026-11-21, at rust-ui-dev-01's QUESTION 01a1fd19 on #736. A
return or a recreation on C4, R4, R3 or R2 after K1 while
`IdStartBusy` shows there retries the start, as it does on Chats,
and holds the screen as Try again's press does; the 1 s before
`ConnectStarting` and the 10 s bound count from the return. A
`LockHeld` again from that start moves no focus and is not announced
(§6, A return to a screen showing `IdStartBusy`). Before, the
profile-in-use row said that the app never retries the start on its
own, and no rule covered a return to these screens. For an owed
Connect, that start and its reads must answer within the bound from
the return. A launch counts its 10 s from the start's success; a start
retried at a return does not. §8's C4 bullet, its error-banner rule and a new bullet say
that `IdStartBusy` shown again by that start takes no focus.
Acceptance 15 gains its runs, two of them for Connect on R4.

Revised 2026-11-21 again, at rust-ui-dev-01's REPLY 01a1fb77 on #736.
In foreground-only mode, Home does not cancel a start made from
`IdStartBusy` that has not answered: an answer that comes before the service stops
stands, and the return shows it; a stop that comes first ends the
start, and the return retries (§6, Foreground-only Home and a start's
answer). Before, §6 said only that leaving the app ends a running
start, and left the order of the two to a device run. Acceptance 15
gains two host variants that answer the start before Home, and a
second device run, which the debug build's `busyStarts` extra makes
runnable now; the first device run stays blocked, since `holdStartMs`
holds only a launch's first start. §6's Seal running row now says
that C4 keeps its PeerId, label, `CopyPeerId` and `IdCreatedBody` on
screen through the start after a successful seal and under
`IdStartBusy`, as #736 builds it; acceptance 15 checks it. §11c and
§11d no longer say that the build does not name this refusal: #736
built it.

Revised 2026-11-21 a third time, for what #728's review carried
(F24, F25, F35). A recreation of the window moves no focus: the
recreated window puts focus on the control that had it, on Chats, in
a conversation and on the flow screens alike (§6). Before, §12 left
focus after a recreation to the build while acceptance 15's
recreation runs checked it on both kinds of screen; §12 drops that
item, and the Chats recreation variant now checks focus too. The
retry at a return on C4 after `IdStartUnchecked` repeats the app's
comparison with C4's PeerId (§6, The hold), and acceptance 15 gains a
tenth return run for it, blocked as acceptance 12's comparison run
is. The ninth return run now says that both reads answer empty.

Revised 2026-11-22, at #738's review (F2, F7, F9). In foreground-only
mode, after a start that answered as started before the stop, the
return makes one start, as a new launch does (add-a-peer §5.1), never
a retry (§6, The start answered first); acceptance 15's two checks
of that case expect it. Before, they expected no start at the return, which a
build that follows §5.1 cannot give. §6's sentence on the order of
answer and stop names the second device run.

Revised 2026-11-22 again, at rust-ui-dev-01's INFO 01a1fde3. The
build does not yet keep a flow screen through a recreation: a change
of the system language recreates the window, and the identity flow
then starts again at its first screen (rust-ui-dev-01's job j311).
§6's rule that a recreation moves no focus remains the rule for the
flow screens, and §6 (The hold) now says what that rule depends on.
Acceptance 15's sixth return run and acceptance 12's R2 recreation
run are blocked until j311 lands. The Chats and conversation recreation runs
are not blocked.

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

- **Words:** what the phrase holds, creates or restores is this
  device's PeerId, on every screen; "identity" names only ADR-0061's
  user identities, the name and photo a person shows, which the phrase
  does not restore. The phrase keeps its contract name, "InterWeave
  identity recovery phrase" (architect-cto-01's REPLYs 01a16736 and
  01a1673d; language-culture-en's REPLYs 01a16739, 01a1673f
and 01a1674b). Where
  this note's own prose says "identity" for what the phrase holds, it
  is the PeerId in that sense.

- `FLAG_SECURE` for as long as words are shown or entered. The flows
  run in the app's own Activity, whose window sets `FLAG_SECURE` before
  the first word, and from Android 13 turns off its Recents screenshot
  (`android-key-custody.md` and `human-client-ui.md`, amended
  2026-10-19: a separate Activity excluded from Recents was the first
  wording and stays allowed). So the flow's task is listed in Recents,
  with no content. A swipe there ends the task and, with no service
  running, likely the process, so the words are lost as at process
  death (§7); that is not checked on a device.
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

**W, Welcome.** The app's name and what it is, in three lines
(`IdWelcomeBody`, which also says what a PeerId is, since W's primary
is the first place a person meets the word). Two actions:
- primary: create a new PeerId (→ C1);
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
- these 24 words are the key to your PeerId;
- they are an InterWeave identity recovery phrase, not a
  cryptocurrency wallet seed (`IDENTITY-RECOVERY.md`, Recovery format);
- write them on paper and keep it safe;
- anyone who has them can be you;
- they restore the PeerId only, not the name and photo, contacts,
  trust, settings or messages; on Android there is nothing else to back up
  them from, so the point asks for nothing (`human-client-ui.md` §9 as
  amended at 2f3b0587);
- the PeerId is created only once creation is complete, after the
  check of two words.

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
- A wrong word shows an inline error just above the entry, at the
  foot of the scrolling area over the fixed picker (§4; string
  `IdCheckWrong`). It says which position, and to compare with the
  paper. The entry clears, and two actions appear:
  - try again (stays on C3): it removes the error and both actions,
    and nothing else. The wrong word cleared the entry, and the letters
    the person typed since then stay in the letters display. Typing
    does not remove the error: it stays until the next word is taken.
    Then a right word removes it, and a wrong one shows it again,
    naming the position of the word just taken. Both actions show
    for exactly as long as the error shows: typing does not remove
    them, and whatever removes the error removes them too;
  - show my words again (→ C2, with the same two positions on return).
- There is no limit on tries, and no way past C3 without two right
  words.
- Back → C2's PeerId page.

Why C3 exists: a confirmation the person only taps is tapped without
writing. Wallet apps that hold a 24-word phrase spot-check for that
reason. It is a hypothesis for this app, falsified if people with the
build fail C3 often for reasons other than a missing paper. Two words
cost about ten taps, and the picker already exists.

**C4, PeerId created.** The PeerId in full again, then one line that
the network is about to start. Primary: continue. This commits the
identity, starts the service, and opens onboarding's third step,
Connect, when the overlay and the trust set are both empty, and the
Chats screen otherwise, by `design/2026-10-22-add-a-peer.md` §5.1's
rule, which also says what a refused service start or a failed read
does and when Connect then shows (§5.6). There is no
Back: the identity now exists. It is sealed here, after the person's
confirmation and before anything starts (§11 Q3). After this, the words
reach the screen only through §3.5.

### 3.3 Restore on a fresh install (no record)

**R1, Restore your PeerId.** What is needed: the 24 words, and the
PeerId to compare at R3. The PeerId is the one written beside the
words, or else the one a contact has for the phone that had these
words (a contact can hold one PeerId per device, `human-client-ui.md`
§3). The
network stays off until the restore is done. One line
(`IdR1Settings`) states what `human-client-ui.md` §9 requires, as
amended at 2f3b0587: the words restore the PeerId only, not the name
and photo,
contacts, trust, settings or messages. Android has no export of
those in standard v1, so the line asks for nothing (§11a Q5). Two
more statements follow it, before the primary, because
`human-client-ui.md` §9 requires both before the phrase is taken:
`IdActiveElsewhere` (an identity still running on another device would
be on two devices, which must not happen, and this phone cannot
check: ADR-0043), and `IdR1NewKeys` (a fresh install has no identity
keys, so each identity gets a new key and every contact will see a
key change for it: ADR-0063 decision 1), shown whenever the state root
has no `identity-keys/` directory, as on every fresh install, the one
condition K1 uses too. Both are statements, never
gates: nothing is asked and nothing refuses. They are on R1 and not on
R2, whose keypad, suggestions and primary must fit at 200 %
(acceptance 13), and every path into R2 passes R1 or K1
(rust-ui-dev-02's OBSERVATION 01a16711, #392). Primary: enter my words (→ R2). Back → W.

**R2, Enter your 24 words.** The picker over all 24 positions (§4).
When all 24 are filled, the primary action, check my words, is
enabled.
- A failed checksum stays on R2 (§6).
- A valid phrase goes to R3.
- Back → R1. The entered words are dropped; a dialog confirms first if
  any are entered (§7).

**R3, Is this your PeerId?** The PeerId the words derive to, in full
(§5). Above it, the question: is this the PeerId you wrote down?
- Primary: yes, restore this PeerId (→ R4).
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
service, then opens Connect or Chats by C4's rule).

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
record names a PeerId, K1 shows it, and after its reason the same
`IdActiveElsewhere` as R1 (§3.3), with `IdR1NewKeys` under it when the
state root has no `identity-keys/` directory. K1 with no primary
(below) takes no phrase and shows neither.
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
words are shown and comes back after. Primary: "Unlock and show my
words" (`IdS1Unlock`), which names the prompt it raises, as S1's body
and the locked notice's Unlock do (the owner asked for Unlock on the
device; the wording is language-culture-en's, §9). This raises
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

One position is entered at a time. In portrait R2 has two parts. At
the top, under the app bar, a scrolling area holds the banner or
notice while one shows (§6) and the phrase so far (item 4). Under it,
fixed to the bottom of the screen and never scrolling, is the picker,
items 1 to 3 below, then the screen's actions. C3 has no grid: its
title, body and, after a wrong word, `IdCheckWrong` with its two
actions sit in the scrolling area above the fixed picker.

- Why: the grid does not fit under the keypad. On a 360 × 640 dp phone
  at 100 % the column comes to about 800 dp against about 570 dp
  between the status and navigation bars (rust-ui-dev-01's figures
  from the view's own sizes, REPLY 01a1a333; computed, not measured on
  the phone). Under the keypad the grid would either push the keypad
  and the primary out of view as the person types, or shrink to a box
  of about one row. A keypad at the bottom is also where every system
  keyboard is.
- The scrolling area keeps the current position in view: when the
  position changes, it scrolls so that the position's row shows.

The picker, from its top:

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
     one. On R2 that is the next empty position by number, wrapping
     after 24 to the lowest empty one. When no position is empty, the
     position just filled stays current, its letters display is
     empty, and the primary is enabled (§3.3). Letters typed then
     start a replacement for that position's word, as on an edit: the
     word stays until a chip replaces it. Nothing ever fills itself:
     even a single remaining match is
     a chip the person taps, or takes with Enter while typing (§8), so
     no word is entered without being seen.
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
     room. The button takes the grid's place, in the scrolling area
     above the picker. "Over 130 %" is strict: at exactly 130 % the
     grid shows.
   - **The full list** (rust-ui-dev-01's QUESTION 01a1a34d). The
     button opens a full-screen page over R2, in the identity screens'
     style, titled `IdReviewWords`, with no new text. It is the grid
     in one column: 24 rows, each a 48 dp stop showing the number and
     the word, or nothing yet; the current position is outlined in
     `primary`. The page scrolls and opens with the current position
     in view. The picker and R2's actions are hidden while it shows.
     It shows words, so it is secure like R2 (acceptance 2).
   - Tapping or pressing a row does what tapping a grid position
     does: the list closes and R2 edits that position.
   - Back, Escape and the app bar's back arrow (§3) close the list
     with nothing changed. They never open the discard dialog, which
     is for Back on R2 itself. No other close control: the app bar's
     arrow is the visible one.
   - The open list is part of the flow's state: a rotation, a return
     from Home and a font change that stays over 130 % keep it open. A
     font change to 130 % or below closes it, and R2 shows the grid. Process death drops it
     with the words (§7).

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
- At 200 % on a 360 × 640 dp phone, from the top: the app bar, the
  "show all my words" button, position, letters, suggestions, keypad
  and primary action fit without scrolling. The phrase grid is behind
  its button (above). The scrolling area is never under 48 dp tall:
  while a §6 banner shows, the banner comes first and the button is
  reached by scrolling the area.
- The longest BIP-39 word has eight letters. Chips are sized to their
  word and wrap to a second row before any word is cut.

**Landscape.** The keypad and suggestions sit on the end side, and the
position, letters and grid on the start side.

**RTL.** The screens mirror. The words, the letters display and the
keypad stay left-to-right: they are English words in Latin letters.

## 5. Showing a PeerId to be compared by eye

- The PeerId is shown in full, in a monospace face, in groups of four
  characters set apart by spacing. The spacing is display only. The
  text is not selectable; under it, a text button "Copy PeerId"
  (`CopyPeerId`, the Details dialog's key, #313) puts the canonical
  string on the clipboard, with no spaces. On Android 13 and later the
  platform confirms the copy; below 13 a `PeerIdCopied` snackbar
  without Undo does (The copy's confirmation, below). Focus stays on
  the button. A copy is never of a word: the button is on no screen
  that shows words: the PeerIds in R2's and R3's banners
  (`IdOtherIdentity`, `IdRestoreOther`) are shown in full for
  comparing by eye and have no copy button.
- It wraps by groups, never mid-group.
- What is shown with it says what it is for: it is not a password,
  and it is safe to write down and share. On C2's and S2's PeerId
  pages the page title names it and a note follows (`IdPeerIdLabel`,
  `IdPeerIdNoteShown`); on C4 and R4 its label does both
  (`IdPeerIdLabelShown`).

**The copy's confirmation, below Android 13.** It is the same on
every screen with `CopyPeerId`: C2's and S2's PeerId pages, C4, R3
and R4.

- It is the look spec's snackbar (`design/2026-10-16-android-look.md`
  §7: its colours, at least 48 dp tall, 12 dp from the screen's
  sides), with the text `PeerIdCopied` and no action. It sits 12 dp
  above the screen's actions, which are fixed at the bottom (§3), and
  covers nothing: while it shows, the scrolling area above the actions
  ends 12 dp above it, and when `CopyPeerId` would then sit below that
  end, the area scrolls just far enough to keep `CopyPeerId` in
  view. When the snackbar closes, the area ends where it ended before
  the snackbar showed.
- It takes no focus and is not in the Tab order (§8): it has no
  action, and focus stays on `CopyPeerId`. A screen reader can still
  read it by touch.
- It shows for 6 s, or for longer when the platform's accessibility
  timeout asks for it (`getRecommendedTimeoutMillis(6000,
  FLAG_CONTENT_TEXT)`), and never for more than 2 minutes: the
  limits `design/2026-10-16-message-swipe.md` §6 sets for its
  snackbar. When it appears, its text is announced once, as a polite
  announcement.
- A second copy while it shows starts its time again from zero, and
  its text is announced again: the person pressed again and is told
  so.
- It closes when anything else shows on the screen, over the screen
  or in place of the screen: any §6 notice (`IdAlreadyCreated` included, a notice that also hides the
  PeerId the snackbar confirmed), the system prompt and the progress
  state; and, on C4, on R4, and on R3 after `IdRestoreOther`'s
  Continue, `design/2026-10-22-add-a-peer.md` §5.1's
  `ConnectStarting`, which shows under the control pressed (the
  primary, or that Continue inside the `IdRestoreOther` notice) once the screen
  has been held for 1 s, counted as that section counts it (a system
  prompt stops the count, and the count begins again from zero when
  the prompt succeeds). Whatever shows after the snackbar replaces
  the snackbar, because what shows later is what needs the person's
  attention. The result of a press counts as showing after the
  snackbar even when it is the notice that showed before the press: a
  Try again that ends in the same notice closes the snackbar when that
  result shows, since that notice takes focus when it shows (§8). The person pressed
  Try again and is now told how that Try again ended.
- A copy while a §6 notice already shows (C4 with `IdBusy`,
  `IdStorageFailed` or `IdStorageUnknown`; R3 with any of its
  notices) shows the snackbar, and the notice stays. The rule above
  closes the snackbar when something shows after it. Here the notice
  showed first, and it sits in the scrolling area, which the
  snackbar does not cover. The snackbar answers the press the person
  just made. Taking away the notice, which still needs the person's
  action, would hide what they must do next.
  A copy while `ConnectStarting` already shows, on a held C4 or R4,
  or on R3 held after `IdRestoreOther`'s Continue, is handled the
  same way: the snackbar shows above the actions, `ConnectStarting`
  stays under the control pressed, and only the snackbar's text is announced,
  since `ConnectStarting` was announced when it showed.
- While the progress state (`IdGoingOffline`) shows over C4 or R3,
  nothing under the progress state can be pressed, `CopyPeerId` included: Enter,
  Space, a screen reader's activate or a tap on it copies nothing and
  shows no snackbar. While the progress state shows, `CopyPeerId` is covered by the
  progress state or drawn disabled, whichever the view draws, so it
  never looks enabled while it copies nothing. The progress state
  closes the snackbar (above),
  so a copy under it would bring back what the progress state has
  just replaced, while the person is meant to read the progress
  state's text.
  The progress state never shows for a service start: not over R4,
  and not over C4 or R3 for any of these starts: the start after a
  seal or a restore succeeded, the start after C4's Continue while
  `IdAlreadyCreated` shows, and the start after `IdRestoreOther`'s
  Continue. On C4 and R3 the progress state shows only for the seal
  or the restore itself. What decides whether it shows is the answer
  that the step gets, not how long the step waits. The progress state
  answers custody's `ProfileLocked`, which the seal, the restore and a
  step that stops the service can get (§6, Profile busy). When another holder has the profile lock, the
  service start is refused with the runtime's `LockHeld` instead, and
  §6's row for a service start refused with the profile in use
  applies (§11c, Q11). The reseal that a start may retry (§6, the
  read-back) runs before the start takes its lock, under a lock that
  custody takes for the reseal alone. If another holder has the
  profile at that moment, the reseal is skipped and retried at the
  next start, and nothing shows for the skipped reseal
  (`crates/human/android-platform/src/identity.rs`, `Resealed::Held`).
  The start's own take of the lock then gives the answer, as §6's row
  for a service start refused with the profile in use says.
- A rotation while the snackbar shows keeps it shown, as §7 keeps
  everything on screen. Its time goes on from when it appeared, and
  it is not announced again. In the new layout it sits 12 dp above
  the actions, as before, and the scrolling area keeps `CopyPeerId`
  in view, as the first point of this list says.
- Leaving the screen closes it, by any way out: Back, the next C2 or
  S2 page, `IdR3No`, or a primary once the next screen opens. On C4,
  R3 and R4, a press of the screen's primary while the snackbar shows
  does not close it. The primary is, on C4, Continue or Try again
  (§6); on R3, `IdR3Yes`, a notice's Try again or `IdRestoreOther`'s
  Continue; and on R4, Continue. On C4 the press starts the seal (or, under
  `IdAlreadyCreated`, the service); on R3 it starts the restore (or,
  for `IdRestoreOther`'s Continue, the service); and on R4 it starts
  the service. In each case the screen stays shown, held (§6 Seal running,
  Restore running; `design/2026-10-22-add-a-peer.md` §5.1),
  until a result shows. What shows then closes the snackbar: anything
  that shows on the screen, by the bullet above, or the next screen,
  by this bullet. While the screen is held, the snackbar's time goes
  on from when it appeared: the press does not start it again. If
  that time ends before a result shows, the snackbar closes when that
  time ends.
  The app going to the background (Home,
  Recents, the screen turned off) closes it too, as it closes the
  message-swipe spec's snackbar. It never shows on the next screen,
  nor again when the app returns: it is the only thing on these
  screens that §7's rule "every screen shows what it showed" does not
  cover.
- On Android 13 and later the app shows no confirmation of the copy
  and announces none: the platform's own confirmation is the only
  one.

A 52-character string is compared by chunks. Chunks of four make that
comparison a matter of groups, not of single characters.

## 6. States

| State | Where | What shows | Way out |
|---|---|---|---|
| Checksum fails (a wrong word or a wrong order) | R2 | A banner above the grid (`IdChecksumFailed`): the words are not a valid phrase; check each word and its order against the paper. Every word stays. | Edit any position, then check again |
| Another identity (with an expected PeerId) | R2 after K1 | A banner (`IdOtherIdentity`): these words belong to another identity, with the derived PeerId in full (§5) and the expected one under it, so the person can see which backup they used. Nothing is restored. | Edit and check again, or Back |
| Authentication needed | S1, whatever the policy. Under `user-presence`, every key operation outside the 60 s window of the last authentication: C4's seal, a restore (R3's yes, R2's check after K1), the read-back after a refused write and each Try again of it, and the service start after S2's Done, R4's or C4's continue, `IdRestoreOther`'s Continue (R3, R2 after K1), and C4's `IdStartUnchecked` | The system prompt, raised by the screen when the operation is refused, never by the service; the operation is retried after a success. The app keeps no clock of the window: the refusal is its only reading of it (ADR-0042, A 2026-10-20). The person sees the same either way: the prompt comes before anything happens. Cancelled or denied: no error dialog and no error text. C4 and the restores stay on their screen with the words kept, and nothing is committed. A prompt for a read-back is the exception: the write before it may have committed the identity, so a cancel or denial of that prompt shows `IdStorageUnknown` (the read-back row below), with focus on it, and never returns the screen to a primary that would write again. A rotation or a return to that screen retries nothing: it stays as it was until the person uses its primary action again. Once the identity is committed, a cancelled start after C4's or R4's continue, or after `IdRestoreOther`'s Continue (the identity on record), ends offline and locked, as S2's Done does below, but on Chats: the locked notice and Unlock, with focus on the notice. A cancelled start after C4's `IdStartUnchecked` ends in the same way, whatever the record holds, because the flow has ended. A start refused after add-a-peer §5.1's 10 s bound ends the same way, but on the screen the person is on by then: Chats or a conversation (`design/2026-10-22-add-a-peer.md` §5.1). S2's Done drops the words and returns to Settings offline, with focus on "Show recovery phrase", the item that opened the flow; the locked notice shows where the app's notices show, under the app bar of Chats and of a conversation; connectivity reads offline, and the locked notice (`AuthenticationNeeded`) with its Unlock action (`Authenticate`) says why and how back (`android-key-custody.md`, Unlock policies). At a launch past the window the start is refused and the prompt answers it before the service runs; its cancel ends the same way. Unlock raises the prompt again (titled `UnlockPromptTitle`), and success starts the service. A process that keeps running answers each refused restart the same way (`design/2026-11-05-unlock-policy-pending.md` §11) | The primary action again, or Back |
| Check running | R2, from the start of the check (`IdCheckWords`, or the Try again of an R2 notice, by any input) until its result, including R2's check after K1; and after K1, from a press of `IdRestoreOther`'s Continue (or of `IdStartBusy`'s Try again in its place) until the next screen opens, which covers the service start that press makes | `IdCheckWords` disabled; after `IdRestoreOther`'s Continue, that Continue and every held control show disabled too (§8); the app bar's back arrow inactive, every word held, and focus where it was when the check began or, after `IdRestoreOther`'s Continue, where it was when that Continue was pressed: neither the check nor the service start moves focus, but Tab, Shift+Tab and the arrows still move it (§7, §8). The system prompt (Authentication needed, above) or the progress state (Profile busy, below) shows over R2 when the check needs one | The result, from one of the rows of this table, or R3, or R4 after K1, or, after `IdRestoreOther`'s Continue, the next screen (`design/2026-10-22-add-a-peer.md` §5.1) |
| Seal running | C4, from the press of its primary (Continue, or Try again in its place, by any input), or of `IdStartUnchecked`, until a result shows on C4 or the next screen opens. That covers the seal, the read-back after it, the service start after a success, the service start after Continue while `IdAlreadyCreated` shows, and the service start of `IdStartUnchecked` | The primary, and `IdStartUnchecked` where it shows, disabled, and focus where it was when the press came: the seal moves no focus, but Tab, Shift+Tab and the arrows still move it (§8). C4 has no Back (§3.2). `CopyPeerId` still copies wherever it shows (it is hidden while `IdAlreadyCreated` shows), except under the progress state (§5). Outside `IdAlreadyCreated`, C4's PeerId, its label, `CopyPeerId` and `IdCreatedBody` stay on screen through the whole hold, the service start after a successful seal included, and stay under `IdStartBusy` when that start is refused (Service start refused: profile in use): a seal that succeeded hides nothing on C4 before the next screen opens. Why: the person may still be comparing or copying the PeerId, and the start is for that PeerId; one hidden at the seal would take it away while the person waits, for a change they cannot see. A notice already showing stays until the result. The system prompt (Authentication needed, above) shows over C4 when the seal or the service start needs one. The progress state (Profile busy, below) shows over C4 for the seal only, never for the service start (§5) | The result, from one of the rows of this table, or the next screen (§3.2) |
| Restore running | R3, from the start of the restore (`IdR3Yes`, or the Try again of an R3 notice, by any input) until its result; and from a press of `IdRestoreOther`'s Continue (or of `IdStartBusy`'s Try again in its place) until the next screen opens, which covers the service start that press makes | `IdR3Yes`, `IdR3No` and the action of any notice on R3 disabled, the app bar's back arrow inactive, and focus where it was when the restore began or, after `IdRestoreOther`'s Continue, where it was when that Continue was pressed: neither the restore nor the service start moves focus, but Tab, Shift+Tab and the arrows still move it (§7, §8). `CopyPeerId` still copies, except under the progress state (§5). The system prompt (Authentication needed, above) or the progress state (Profile busy, below) shows over R3 when the restore needs one | The result, from one of the rows of this table, or R4, or, after `IdRestoreOther`'s Continue, the next screen (`design/2026-10-22-add-a-peer.md` §5.1) |
| Profile busy (`ProfileLocked`) | Any step that stops the service or writes, other than a service start and the reseal it retries (§5) | The progress state (`IdGoingOffline`) for up to 10 s. Then a notice (`IdBusy`) that the app could not take the identity offline, with Try again. Entered words stay. | Try again, or Back (Back from a notice, below the table) |
| Service start refused: profile in use (the runtime's `LockHeld`, `StartRefused::Runtime`) | The service start after C4's or R4's Continue (or C4's Try again in its place), C4's Continue under `IdAlreadyCreated`, C4's `IdStartUnchecked`, and `IdRestoreOther`'s Continue (R3, R2 after K1) | The start takes the profile lock itself with no wait, so it is refused as soon as the start takes its lock, and no progress state shows (§5). When the refusal comes before add-a-peer §5.1's 10 s bound ends, the hold ends, and a notice (`IdStartBusy`) says that the app could not go online because the profile is in use, with Try again. It never says that the identity was not created or not restored: an identity is on record (the one this flow committed, or, under `IdAlreadyCreated` and `IdRestoreOther`, the one already there). It shows where the screen's notices show: on C4 and R4 (R4 has C4's layout, §3.3) in C4's notice place, with Try again in Continue's place; on R3 and on R2 after K1, in `IdRestoreOther`'s place, with the PeerId that `IdRestoreOther` showed kept under `IdStartBusy`, and Try again in the place of `IdRestoreOther`'s Continue. Under `IdAlreadyCreated` and `IdRestoreOther` the rest of the screen stays as that notice left it (on C4, the title `IdAlreadyTitle` and the hidden PeerId, label, `CopyPeerId` and `IdCreatedBody`). Try again makes one service start and nothing else: no seal, no restore, no words. It is pressed as the Continue it replaces, and after `IdStartUnchecked` it is pressed as `IdStartUnchecked` is, the app's comparison with C4's PeerId included (A start from C4 after a failed read-back): the screen is held from its press, as after that Continue (Seal running, Restore running, Check running; §8), and the 1 s before `ConnectStarting` and the 10 s bound count from that press (`design/2026-10-22-add-a-peer.md` §5.1). The app never retries the start on its own, except at a return or a recreation (A return to a screen showing `IdStartBusy`, below the table). A refusal after the bound, with Chats already open, shows on Chats or a conversation instead (A start refused with the profile in use after the bound, below the table) | Try again (C4 and R4 have no Back; on R3 and R2 after K1, Back as it works at any other time: Back from a notice, below the table) |
| A refusal that may follow the write: read back | C4's continue (the seal), R3's yes, R2's check after K1 | Only two refusals can come after the record was written: `OldKeyRemains`, and `Storage` whose inner error is `Unsynced` (a directory sync after the publish, `persist.rs`). For these two only, the screen reads back with `unlock` before it shows anything: does the record now open to the PeerId just shown or confirmed? A name alone is not enough, because after K1 the old record already names that PeerId. Every other refusal wrote nothing and shows its own row in this table (another identity, authentication needed, profile busy, or any other refusal). The builder must see `Storage`'s inner error to tell the two apart | — |
| Read-back: it opens to the PeerId just shown or confirmed | after those two refusals | Success: C4 and the restores go on as below. The app retries the retire at its next start (`reseal`), with one diagnostic line and never a word | Continue |
| Read-back fails, or the record does not open to that PeerId | after those two refusals. The record is always there after them (both follow the publish, and nothing removes it), so no row for a missing record is needed. A record opening to another PeerId cannot follow this flow's own write while it holds the profile lock (§2); should one appear anyway, the app cannot tell what happened, so it is this row | `IdStorageUnknown`: the app cannot tell whether the identity was saved. Try again reads back again and never writes again, on C4, R3 and R2 after K1 alike: the record is there (this row's second column), so what is unknown is whether it opens, which only a read-back answers: a second seal would be refused as another identity on record, and a read-back answers it without writing. A read-back can do no more than report a record whose key opens nothing; a new restore replaces that key, and is the way on when the read-back keeps failing (A read-back that keeps failing, below the table). While the notice shows, R3's `IdR3Yes` does what its Try again does, and so does R2's `IdCheckWords` until the words change. On R2 after K1, an edit of the words removes the notice (An edit on R2, below the table) but does not end this: until a read-back opens the record to that PeerId (the success row) or R2 closes, a check whose words derive the PeerId that the refused restore confirmed reads back instead of restoring, and words that derive another PeerId are checked as any words are (the checksum banner, `IdOtherIdentity`). The record's state is what is unknown, not the notice's, so no edit can turn the next check into a second write. A read-back that opens to the PeerId shown or confirmed before the refusal is the success row above; any other answer shows `IdStorageUnknown` again. A read-back refused because authentication is needed raises the prompt (Authentication needed), and a cancel or denial of that prompt shows `IdStorageUnknown` again. Entered words stay while the screen is open. On R2 after K1, the read-back owed lasts as long as the words do: a rotation, a return to the app and a recreation of the window keep both, and process death, which drops the words, ends both with R2 (§7). rust-ui-dev-01's branch (not yet on main) holds the words and the read-back owed together, and a rotation keeps them; that a recreation keeps them is not yet verified (rust-ui-dev-01's REPLY 01a1f33c), and acceptance 12's recreation run checks it | Try again, or Back (Back from a notice, below the table); on C4, after a failed read-back, `IdStartUnchecked` too while it is offered (below the table) |
| Any other refusal: nothing was written | C4, R3's yes, R2's check after K1 | Every refusal no other row names. That is `Storage` with any inner error but `Unsynced` (an I/O error, a directory or lock file that is not private, an unsupported platform); `Cipher` other than `UserNotAuthenticated` (the Keystore unavailable, a key invalidated, missing or refused); `CipherShape`; `Identity`; and `Unreadable` (a record that cannot be read, at R3 or after K1). All come before the publish, so `IdStorageFailed`: nothing was changed, with Try again. A restore after K1 never reaches R4 this way | Try again, or Back (Back from a notice, below the table) |
| Another identity already on record (`AlreadyProvisioned`; at a restore, a record naming another PeerId) | C4's continue; at a restore, R3's yes and R2's check after K1 | The record was there before this flow's write. `recorded_identity` names the PeerId the phone holds. Not this row: on R3, a restore of other words after this flow's own restore was refused and its read-back failed, which custody also answers `RecordNamesOther`; it is not designed here (A read-back that keeps failing, below the table; §12), and until it is, a build shows `IdStorageUnknown` again for it, with focus on it, and writes nothing; its Try again and `IdR3Yes` then read back against the PeerId the record names, the earlier restore's, so a read-back that opens the record goes to R4 for that PeerId, shown on R4, and words of the new PeerId stay unused. `IdAlreadyCreated`: the words just shown were not used; Continue opens that identity. At a restore, a record naming another PeerId shows `IdRestoreOther` with that PeerId | Continue |
| The record names another identity (`RecordNamesOther`) | R2's check after K1; R3's yes after this flow's own refused restore | Not reachable on R2 after K1 while the flow holds the profile lock: K1 offers R2 only with the record's own PeerId (§3.4). If a build reaches it there, the another-identity row above applies (`IdRestoreOther`). On R3 it is reachable after this flow's own refused restore, which that row excepts | Continue |
| Success | C4 after its seal; R4 after a restore | The PeerId, and continue | Continue |

Errors appear inline on the screen they belong to, never in a snackbar.
A snackbar times out, and these need the person's action. Their text
is in `error` on the screen's surface. A banner uses
`surface-container` with an `error` icon, as the look spec's notices
do. On R2 every banner and notice of this table shows in one place,
above the grid: the checksum banner, `IdOtherIdentity`, `IdBusy`,
`IdStorageFailed`, `IdStorageUnknown` and `IdRestoreOther`, and,
after K1, `IdStartBusy`. They are
outcomes of one check, or, for `IdStartBusy`, of the service start
that `IdRestoreOther`'s Continue made, which shows in
`IdRestoreOther`'s place (the profile-in-use row), so at most one
shows. `IdBusy`, `IdStorageFailed`, `IdStorageUnknown` and
`IdStartBusy` carry `TryAgain` under their
text and `IdRestoreOther` carries Continue; the checksum banner and
`IdOtherIdentity` have no action of their own (the table's way out is
an edit of the words). On R3 a notice of this table shows in one
place too: under the app bar, above the question. These are `IdBusy`,
`IdStorageFailed`, `IdStorageUnknown` and `IdRestoreOther`, each the
outcome of one restore, and `IdStartBusy`, the outcome of the service
start after `IdRestoreOther`'s Continue, in `IdRestoreOther`'s place,
so at most one shows, with its action under
its text as on R2. R3 has no title, so the notice is read right
after the app bar, before the question. On C4 a notice of this table
shows in one place as well: below the title, before the rest of C4.
That is above the PeerId's label, or, while `IdAlreadyCreated` hides
the label, above Continue.
These are `IdBusy`, `IdStorageFailed`, `IdStorageUnknown` and
`IdAlreadyCreated`, each the outcome of one seal, and `IdStartBusy`,
the outcome of the service start after a seal or after
`IdAlreadyCreated`'s Continue, so at most one shows. On C4 no notice carries an action of its own: C4 has one
action, its primary, and, under `IdStorageUnknown` after a failed
read-back, `IdStartUnchecked` under it while it is offered (A start from C4 after a failed
read-back, below the table), which starts the service and seals
nothing. While `IdBusy`, `IdStorageFailed` or
`IdStorageUnknown` shows, the primary is Try again (`TryAgain`) in
Continue's place. Under `IdBusy` and `IdStorageFailed` it seals
again; under `IdStorageUnknown` it reads back and seals nothing (the
read-back row). While `IdStartBusy` shows, the
primary is Try again in Continue's place too, and it makes one
service start and no seal: the identity is already on record, and a
second seal would be refused as another identity on record. So C4 never shows two controls
that both seal. `design/2026-10-22-add-a-peer.md` §5.1 agrees: it
calls C4's primary "Continue (or Try again)". While
`IdAlreadyCreated` shows, the primary is Continue, which opens the
identity on record (the another-identity row). The words just shown
were not used, so C4 then hides the PeerId they made, its label,
`CopyPeerId` and `IdCreatedBody`: each would show that PeerId as the
person's own. For the same reason the title is `IdAlreadyTitle`, not
`IdCreatedTitle`: no PeerId was created. All of this stays so while
`IdStartBusy` shows after `IdAlreadyCreated`'s Continue, in that
notice's place: the words are still the ones that were not used. And if the person copied that
PeerId while creating it, on C2's PeerId page or on C4, one or more
times, the app removes it from the clipboard when
`IdAlreadyCreated` shows, but only while the clipboard still holds
that copy: whatever the person copied since, from another app or
from another screen of InterWeave, stays. This keeps the person from using the platform's paste to give a
contact, as their own, a PeerId that no phone will ever hold. The
removal reaches the system clipboard only: a keyboard's own clipboard
history (Gboard's, Samsung Keyboard's) belongs to the keyboard, which
the app cannot reach, so a paste from that history can still give the
PeerId (§12). The app tells its own copy
apart without reading the clipboard's content, so the platform shows
no notice that the app read the clipboard (how it does so is the
builder's choice: the copy's clip description, for example). On every
Android version the app supports (its minimum is Android 11, API 30),
the platform gives a clip's description only to an app whose window
has the focus, and to no app while the device is locked
(`ClipboardService.getPrimaryClipDescription`, which returns nothing
unless `clipboardAccessAllowed` finds the app's window focused and
`isDeviceLocked` is false; read in the platform source of Android 11
to 16 and of the platform's main branch). `clipboardAccessAllowed`
also requires that the clipboard read app op (`OP_READ_CLIPBOARD`)
is allowed to the app (`MODE_ALLOWED`). If that app op is not allowed
(a device's system software may let the person deny it), the
description never comes and the removal never acts: the copied PeerId stays on the clipboard, as after
process death (§12). So the removal waits whenever the seal is answered
`AlreadyProvisioned` while C4's window does not have the focus or the
device is locked. C4 can lack the focus in the background, and also
in the foreground: in split-screen while the other app has the focus,
under the notification shade, or while a system window such as a
prompt is in front. The removal waits until C4's window has the
focus with the device unlocked, and then acts only if the clipboard
still holds that copy. If the process ends first, the copy stays
(§12). The notice is read after the title, before the PeerId's
label, or, with `IdAlreadyCreated`, before Continue.

**An edit on R2.** Every position is filled when a check's result
shows, so an edit starts with one of two inputs, by any input method:
a grid position chosen to edit (or a row of the full list), or a chip
taken, which replaces the current position's word. Either removes
whatever banner or notice of this table shows: the checksum banner,
`IdOtherIdentity`, `IdBusy`, `IdStorageFailed`, `IdStorageUnknown`,
`IdRestoreOther` and, after K1, `IdStartBusy` alike. Letters typed,
delete and clear change no word (§4 item 2), so they remove nothing.
The next check's result shows what the new words give. Why: each
banner and notice speaks of the words that were checked, and once the person starts to change a word, the banner or notice would state something about words
that may no longer be on the screen (*visibility of system status*,
Nielsen 1). `IdStartBusy` speaks of the start that
`IdRestoreOther`'s Continue made, not of the words, but it stands in `IdRestoreOther`'s place and is removed as `IdRestoreOther` is; a check of the same words
shows `IdRestoreOther` again, whose Continue makes the start again.
On main, `IdentityFlow::edit` and `IdentityFlow::choose`
(`crates/human/ui-model/src/identity_flow.rs`) remove the notice on
these two inputs, and `press` does not, for the notices main has;
the read-back notices and the read-back owed are on rust-ui-dev-01's
branch, not yet on main (REPLYs 01a1f330, 01a1f33c). Removing
`IdStorageUnknown` changes nothing about the record: the read-back
owed on R2 after K1 outlives the notice, as its row says. Each of the
two inputs names where focus goes (§8: a grid position chosen goes to
the keypad, a chip that leaves no position empty to `IdCheckWords`),
so focus never stays on a removed control.

**A read-back that keeps failing.** On R3 and on R2 after K1, the
way on is Back and a new restore. On R3, Back returns to R2 with every
word kept; R2's check opens R3 again, with no notice, and `IdR3Yes`
restores. On R2 after K1, Back returns to K1 (through the discard
dialog when words are entered), and the R2 that K1 opens next owes no
read-back, so its check restores. A new restore of the same words is
safe: custody's restore first tries the record it finds. A record
sealed under the profile's policy that opens to the PeerId is kept,
and nothing is sealed; a record of that PeerId whose key is
invalidated, missing or another, or that is sealed under the other
policy, is sealed over, which is the recovery K1 exists for
(`restore_locked` in `crates/transport/embedded/src/custody.rs`). A key that cannot be used at that moment refuses the restore and writes nothing:
`UserNotAuthenticated` raises the prompt (Authentication needed), and
`Unavailable` shows `IdStorageFailed` (the any-other-refusal row), so
while the Keystore is unavailable a new restore does not recover. After K1, words of
another PeerId write nothing and show `IdOtherIdentity`. On R3,
words of another PeerId than the refused restore's are refused as
`RecordNamesOther` and write nothing, since the record that restore
wrote names its PeerId; what R3 shows then is not designed here, and
until it is, R3 shows `IdStorageUnknown` again (the another-identity
row; §12). C4 has no Back (§3.2), and Home and a return show C4 again as
it was (§7), which is not a new launch (How long, below). So on C4 the
way on in the app is a service start, offered beside Try again once a read-back has failed
(A start from C4 after a failed read-back, below; §11d, Q15). On R3 and on R2 after K1, no text on the screen tells the person that Back and a new restore is the way on (§12). Whether a person finds it is a hypothesis,
untested, falsified if a person in a test with a read-back that keeps
failing on R3 or on R2 after K1 does not press Back.

**A start from C4 after a failed read-back** (architect-cto's ruling
on Q15, REPLY 01a1f34b, §11d). Once a read-back on C4 has answered as
failing, so that `IdStorageUnknown` shows after it, C4 shows a second
control under its primary: `IdStartUnchecked` (new, §9), a text
button, with Try again staying the primary in Continue's place. The
read-back stays the first way on (Material: one filled primary, a
text button for the less likely path). It is not offered when the
notice shows after a cancelled or denied read-back prompt, since no
read-back has answered then; once one has failed, it stays offered
while the notice shows, until a start from it meets another PeerId
than C4's (Its results), after which it is not offered again while C4
is open. Every other place in this note that names
`IdStartUnchecked` as shown means "while it is offered" in this
sense. `IdStartUnchecked` shows disabled while a Try again under the notice runs and while its own start runs, as the primary does (Seal
running). Under it, a line
(`IdStorageUnknownRestore`, new, §9) says that if the app then asks for the recovery phrase, the person enters the phrase just written down, and that this phrase restores the PeerId. Neither text says
that a PeerId was created.
- **What it does.** The creation flow ends and lets go of its profile
  lock, as before any start after C4; then the start unlocks the
  record, and the app compares the identity that `unlock` gives with
  the PeerId C4 shows before the app hands that identity to the service (custody's
  `unlock` checks the seed only against the record's own PeerId field,
  `crates/transport/embedded/src/custody.rs`; this comparison is the
  app's). Nothing is sealed or written: no connect-owed marker either,
  so the start takes add-a-peer §5.1's not-owed path, as
  `IdAlreadyCreated`'s Continue does, and the flow cannot be entered
  again from here to seal. C4 is held from the press as after
  Continue (Seal running), and the 1 s before `ConnectStarting` and
  the 10 s bound count from the press
  (`design/2026-10-22-add-a-peer.md` §5.1). Why not owed: the press
  answers a record whose state is unknown, and writing a marker for it
  would be a write the ruling excludes; the person can still add a
  peer from Peers.
- **Its results**, as at a launch. The record opens to C4's PeerId:
  the service runs, and Chats opens (the not-owed path). It does not
  open (`RecoveryRequired`, whatever its cause, `WrongIdentity`
  included): K1 (§3.4), with its reason by cause and the PeerId the
  record names as the expected one, which is C4's when this flow's
  seal wrote the record; K1's own focus rule applies. It opens to
  another PeerId than C4's (the app's comparison): nothing starts, and
  C4 shows `IdStorageUnknown` again with Try again only and focus on
  the notice. `IdStartUnchecked` is not offered again while C4 is
  open, whatever Try again answers: its read-back opens the record to
  that other PeerId, so it fails as before, and the start would meet
  the same comparison. Only a new launch leads on, and only the end of the process makes one (a force stop, or a swipe of the task from
  Recents, which likely ends it, §2), and that launch's unlock runs
  whatever the record opens to, as any launch does, with no comparison
  and no word to the person (§12). A mismatch fails closed from C4, as the ruling requires: nothing runs as that identity from C4. A mismatch cannot follow
  this flow's own write unless another writer took the profile between
  the lock's release and the start, so it gets no screen of its own. A
  platform error (`Unavailable`) is "try
  again", never K1: C4 shows `IdStorageUnknown` again with both controls, and
  focus on the notice. A held lock shows `IdStartBusy` (the
  profile-in-use row), whose Try again replaces both controls and is
  pressed as `IdStartUnchecked` is, the app's comparison included. On a
  `user-presence` profile outside its window the start raises the
  prompt (Authentication needed), and a cancel or denial ends as a
  cancelled start after C4's Continue does: on Chats, offline and
  locked, since the flow has ended.
- **Focus.** The control sits after the primary in C4's order (§8).
  When it first shows, focus goes to the notice, as for any
  `IdStorageUnknown` (the error-banner rule, §8). Like a press of Continue, its press moves no focus while C4 is held: focus stays on the
  control, now disabled, so a second Enter, Space or a screen reader's
  activate starts nothing.
- **Not verified.** That the creation flow can end and start without
  a seal from this state in the build (rust-ui-dev-01's to say when
  building it), and what `Storage(Unsynced)` leaves on disk on a real
  device.

**A return to a screen showing `IdStartBusy`.** The person may leave
C4, R4, R3 or R2 after K1 while `IdStartBusy` shows there (Home or
Recents, then back), or the window may be recreated for a
configuration change that the app does not handle itself. Such a
return or recreation retries the start when no start is running, as
unlock note §11's L3 retries any start after a refused one, and as §6
does for this notice on Chats (A return or a recreation, under A
start refused with the profile in use after the bound): a refused
start is not a start that ran (its L4). Why: the person's Continue
asked for this start, and the notice asks them to try again. A
return is the moment a lock that another holder had may have gone,
and a screen that stays refused until the person presses Try again
once more keeps them from what they had already asked for. One rule for
this notice wherever it shows also keeps the app predictable
(*consistency and standards*, Nielsen 4).

- **The hold.** The retried start is made as Try again's press makes
  it, and holds the screen as that press does (the profile-in-use row;
  Seal running, Restore running, Check running; §8): Try again shows
  disabled, every control that press holds is held, and Back does
  nothing. On C4 after `IdStartUnchecked`, the start includes the
  app's comparison with C4's PeerId (A start from C4 after a failed
  read-back): a record that opens to another PeerId starts nothing,
  and C4 shows `IdStorageUnknown` again with Try again only and focus
  on the notice, as after that press, since the notice that had
  focus has gone. Why: the ruling on Q15 (§11d) lets nothing run as
  that identity from C4, and a return is no exception to it. Focus stays where it
  was at the return: the retry moves no focus. A recreation moves
  none either: the recreated window puts focus on the control that
  had it before, as on Chats (A return or a recreation, under A
  start refused with the profile in use after the bound). On the
  flow screens the build cannot yet put focus back after a
  recreation. A recreation starts the
  identity flow again at its first screen, so C4, R4, R3 and R2 after
  K1 are not kept, and neither is the focus on them (rust-ui-dev-01's
  INFO 01a1fde3, their job j311). The rule that a recreation moves no
  focus is still the target on these screens, and the two runs that
  check it there, acceptance 15's sixth return run and acceptance
  12's R2 recreation run, are blocked until j311 lands. The 1 s before
  `ConnectStarting` and add-a-peer §5.1's 10 s bound count from the
  return, never from when `IdStartBusy` first showed. Why: the
  bound limits how long the person waits on a held screen, and while
  they were away nothing ran and nothing was held. Counted from the
  first showing, a return after more than 10 s away would open Chats
  at once, before the retried start answers anything, and take the
  person off the screen they came back to. At the bound, Chats opens
  and the start goes on behind it, as after Try again's press; its
  refusal after the bound then shows as §6 says (A start refused with
  the profile in use after the bound).
- **Its results** are Try again's (the profile-in-use row; §8),
  except that this start answers no press of the person's. A success
  opens the next screen, whose focus is add-a-peer §5.1's, as after
  the Continue that Try again replaces. A `LockHeld` again ends the
  hold and shows `IdStartBusy` again, with Try again enabled; focus
  stays where it was and the notice is not announced again: its text
  has not changed, and the person did not press anything. A prompt
  that this start needs rises by itself (L3); a cancel or denial ends
  as a cancelled start after that Continue does (Authentication
  needed): on Chats, offline and locked, with focus on the locked
  notice. Any other refusal shows that refusal's row.
- **Only one start.** A return or a recreation while a start runs
  (Try again's, or a retry's) starts nothing, and that start's answer
  stands. A return or a recreation that starts nothing holds nothing
  beyond what the running start already holds. In foreground-only
  mode, leaving the app stops the service, and a start that has not
  answered by then ends with the service (`design/2026-10-22-add-a-peer.md`
  §5.1, Interruptions), so a return after Home finds no start running
  and retries; a recreation keeps the service in either mode, so a
  start that runs through it goes on. Home does not cancel the start:
  a start that answers before the service stops has answered, and
  the return shows that answer, as with Stay reachable on (Foreground-only
  Home and a start's answer, below). A rotation is handled by the
  app without a recreation (§7), so it keeps the notice and retries
  nothing.
- **Connect** counts this start as it counts Try again's on these
  screens: as the start after the Continue it replaces
  (`design/2026-10-22-add-a-peer.md` §5.1): the start and its two
  reads must answer within the hold's 10 s bound from the return, and
  at the bound the start counts as deferred (that §5.1, The bound).
  No second 10 s counts from the start's success: that count is a
  launch's, which has no held screen.

**Back from a notice.** Where a row's way out is Back, Back does what
it does on that screen at any other time. On R2 after K1 it returns
to K1, through the discard dialog when words are entered, and only
`IdDiscard` drops the words (§7). On R3 it returns to R2 with every
word kept, as `IdR3No` does (§3.3). C4 has no Back (§3.2), so there
the way out is Try again, or, while `IdAlreadyCreated` shows, Continue,
or, under `IdStorageUnknown` after a failed read-back, `IdStartUnchecked` while it is offered.

**A start refused with the profile in use after the bound.** The
start takes its lock only after the key is unwrapped and after any
reseal that the start retries (§5). So, on a slow Keystore,
`LockHeld` can come after add-a-peer §5.1's 10 s bound. By then
Chats has opened and the screen whose button made the start is gone.
The person may be on Chats, in a conversation opened since, or on a
page opened from Chats (Peers, Settings, a card page). rust-ui-dev-01
read the build and said that the parts §11d lists can be built (§11d,
Q12).

- **What shows.** `IdStartBusy`, as a session notice, in the place
  and at the priority where Chats and a conversation show the notice
  of any other refused start: under the app bar, before the list or
  the messages, where the locked notice shows on either
  (`design/2026-11-05-unlock-policy-pending.md` §4 for Chats, F12 for
  a conversation).
  Its one action is Try again (`TryAgain`). It shows on Chats or in a
  conversation, whichever is on screen. A page with no place for
  session notices does not show it; the notice shows when the person
  comes back to Chats or a conversation. The string is the one C4
  shows: the app could not go online because the profile is in use,
  never that an identity was not created or not restored.
  Connectivity reads offline, as after any refused start.
- **Focus does not move.** The notice is announced once, politely,
  when it first shows on screen, because every notice's message is a polite live region (unlock note F1). It sits in the reading order
  and the Tab order after the app bar. Why: the refusal answers
  nothing the person did. The bound has already given them Chats,
  and they may be reading or typing in a conversation. A screen that
  moves focus by itself while the person acts on it breaks *user
  control and freedom* (Nielsen 3), and a status message is
  announced without taking focus (WCAG 2.2, 4.1.3 Status Messages).
  The locked notice after the bound is different: it takes focus
  because it answers the person's own cancel or denial of the
  start's prompt (`design/2026-10-22-add-a-peer.md` §5.1, a refused
  service start).
- **Try again** makes one service start and nothing else: no seal,
  no restore, no words. From its press until that start answers, Try
  again shows disabled, and Enter, Space, a screen reader's activate
  or a tap on it starts nothing more. A second start while the first
  runs would itself be refused with `LockHeld` (§11c, Q11). The press
  moves no focus: after a key or a screen reader's activate, focus
  stays on Try again, and after a tap it stays where it was. Nothing
  else on the screen is held, no `ConnectStarting` shows, and neither the hold nor the 10 s bound of a held screen applies: the person is on
  Chats or a conversation, not on a held screen, and connectivity
  shows this start as it shows any other. For Connect, this start is
  a later start that the person asked for
  (`design/2026-10-22-add-a-peer.md` §5.1, A launch with Connect
  still owed): its count of openings begins at the press, a
  conversation on screen at the press counts as an opening (that
  §5.1), and the 10 s within which the reads must answer for Connect
  to open counts from that start's success, as for a launch.
- **Its results.** A start that succeeds removes the notice. If
  focus was on the notice or on Try again, it goes as unlock note F14
  says: on Chats to the list heading, in a conversation to its
  composer. A start refused with `LockHeld` again shows the notice
  again, with Try again enabled. When focus was on the notice or on
  Try again, it goes to the notice, since this result answers the
  person's press; otherwise it stays where it was, since the screen
  was not held and the person may be acting elsewhere by then.
  A start that needs the system prompt raises it over the screen.
  If the prompt is cancelled or denied, the locked notice takes
  `IdStartBusy`'s place, with focus on it, as unlock note §11's L1
  answers a network grant that leaves the process running (F11). Any
  other refusal shows what that refusal shows on Chats today, which
  this note does not design.
- **A return or a recreation.** A return to the app (Home or Recents,
  then back) or a recreation of the window for a configuration
  change that the app does not handle itself retries the start when
  no start is running, as unlock note §11's L3 retries any start
  after a refused one: a refused start is not a start that ran (its
  L4). A return or a recreation while Try again's start runs starts
  nothing, and that start's answer stands, so only one start is ever
  in flight (Try again, above). In foreground-only mode, leaving the
  app stops the service, and a start that has not answered by then
  ends when the service stops (`design/2026-10-22-add-a-peer.md` §5.1,
  Interruptions), so a return after Home finds no start running and
  retries; a recreation keeps the service in either mode, so a start
  that runs through it goes on. Home does not cancel the start: one
  that answers before the service stops has answered (Foreground-only
  Home and a start's answer, below). The notice stays while the retried
  start runs, with Try again shown disabled. Its results
  are Try again's, except that this start answers no press of the
  person's: a `LockHeld` again leaves the notice and focus as they
  were and is not announced again, and a success moves focus only as
  above. A prompt that this start needs shows with no press, and a
  cancel or denial shows the locked notice with focus on it (L3,
  F12). A recreation moves no focus of its own: the recreated window
  puts focus on the control that had it before, a disabled Try again
  included, and an answer then moves it as above. Why: Android keeps
  a focused view through a configuration change, so a keyboard user
  who changes the system language expects to be where they were
  (*consistency and standards*, Nielsen 4). A rotation is handled by
  the app without a recreation (the Activity's `configChanges` in
  `AndroidManifest.xml`), so it keeps the notice and retries nothing
  (§7).
- **How long.** Until a start succeeds, until the notice of another
  refused start takes its place, or until the process ends. A new
  launch (the Activity created anew, as the unlock note counts it; a
  foreground-only return to a live Activity is not one here) makes a
  start of its own and shows what that start answers,
  never this notice. Apart from the return and the recreation above,
  the app never retries the start on its own: no timer retries it.
- **Connect.** At the bound this start counted as deferred
  (`design/2026-10-22-add-a-peer.md` §5.1, the bound). So Connect
  stays owed when it was owed, and stays not owed after
  `IdAlreadyCreated`'s or `IdRestoreOther`'s Continue, which write no
  connect-owed marker. A start retried at a return or a recreation
  (above) is a later start too, as Try again's is: for an owed
  Connect, its count of openings begins at the return or the
  recreation that made it, a conversation on screen then counts as
  an opening, and the 10 s are counted from that start's success
  (`design/2026-10-22-add-a-peer.md` §5.1, A launch with Connect
  still owed). Why: the person is on Chats or a conversation again,
  and what they opened before leaving was opened before this start
  was made.

**Foreground-only Home and a start's answer.** In foreground-only
mode, Home or Recents while a start of this section has not answered
(Try again's, or one retried at a return) lets go of the service
(`onStop`, `MainActivity.kt`), and the service stops. The start can
still answer before the service has stopped (rust-ui-dev-01, REPLY
01a1fb77, on #736). Home does not cancel the start, and the app never
drops an answer that comes before the stop. Whichever comes first,
the start's answer or the service's stop, decides what the return
shows:

- **The start answered first.** Its answer stands, as with Stay
  reachable on, and the return shows what it decided. On C4, R4, R3
  and R2 after K1, a success has opened the next screen
  (`design/2026-10-22-add-a-peer.md` §5.1), and the return shows that
  screen, never `IdStartBusy`; on Chats or a conversation, a success
  has removed the notice. The service stopped at Home all the same, so
  the return then makes one start, as a new launch does (add-a-peer
  §5.1: the return is a new launch, counted as one for Connect), never
  a retry, and no `IdStartBusy` shows for it. A `LockHeld` again has
  shown `IdStartBusy` with Try again enabled, so the return finds no
  start running and retries, as above. Any other answer shows its row.
- **The stop came first.** The start ended unanswered, and nothing it
  asked for happened, so the return finds no start running and
  retries, as above.

Either way, at most one start is in flight, and the return shows a
state this section already designs. Why not cancel at Home: a cancel
would drop the answer of a start that succeeded, so the person would
come back to `IdStartBusy` for a start that had worked, and the return
would make the start again. That retry, made while the stopping
service still holds the lock, would be refused with `LockHeld`
(§11c, Q11), so the person would see "profile in use" caused by the
app's own start. The answer that came is the one true status to show
(*visibility of system status*, Nielsen 1), and an unanswered start
needs no cancel: the stop already ends it. Not verified on a phone:
which comes first, and that an answer received while the Activity is
stopped shows at the return. Acceptance 15's second device run in
foreground-only mode (`busyStarts` 1) records which comes first; the
run sees an answer received while the Activity is stopped only when
the order of events on that phone produces one, and the run says
which it saw.

## 7. Leaving, Back and interruptions

- **Escape acts as Back** on R2 and C3, wherever Back is named below.
- **Back with words entered (R2, C3):** a dialog asks whether to
  delete the words entered (`IdDiscardTitle`). The actions are delete
  and continue entering; continue entering is the default focus. With
  nothing entered, Back goes straight back.
  - Only `IdDiscard` drops the words. Back again (the system Back or
    Escape) and a tap outside the dialog mean `IdContinueEntering`:
    the dialog closes and every word stays. A second Back must never
    lose 24 words typed from paper.
  - Continue entering, however chosen, returns focus to the control
    that held it when Back was pressed. Back is a key, not a control
    on the screen, so that control stands for "what opened it" (§8).
  - `IdDiscard` drops the words and goes back.
  - Back from R2 or C3, with the dialog or with nothing entered: on
    the screen Back reaches, focus goes to the control that opened
    the screen just left, as for C2's pages (§8).
- **Back while R2's check runs** (§6, Check running) does nothing
  until the result: Back and Escape do nothing, no discard dialog
  opens, and the app bar's back arrow is drawn inactive until the
  result. After K1 the check can write the record: a person who left
  R2 while it wrote would not know whether the restore happened. A
  system prompt over R2 keeps its own Back, which cancels the prompt
  (§6, Authentication needed).
- **Back while R3's restore runs** (§6, Restore running) does nothing
  until the result, for the same reason: the restore writes the
  record. The app bar's back arrow is drawn inactive until the
  result. `IdR3No` is disabled for the same time, since it too would
  return to R2, where the words could change while the restore writes
  the record from the words it read. A system prompt
  over R3 keeps its own Back, which cancels the prompt.
- **Back after `IdRestoreOther`'s Continue** (§6, Restore running,
  Check running) does nothing until the next screen opens, on R3 and
  on R2 after K1, with the back arrow drawn inactive: that Continue
  starts the service, and a screen left during the start would not
  say whether the service started. A system prompt keeps its own Back, which
  cancels the prompt (§8).
- **The app leaves the screen.** Home, a call, or another app opening.
  A stopped Activity is neither completion nor cancellation (§11 Q1):
  - on return, every screen shows what it showed, except the
    `PeerIdCopied` snackbar below Android 13, which closes when the
    app goes to the background (§5); entered words are kept in memory
    only, never in saved-instance state;
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
- Focus order follows the reading order of §4 and §6:
  - **R2 in portrait**, top to bottom (§4): app bar, the banner or
    notice while one shows and then its action (§6: above the grid),
    the grid's positions by number (or `IdReviewWords` over 130 %
    font), position, letters, the suggestion chips, the letter keys
    row by row, the delete key, the clear key, then the screen's
    actions, primary first.
  - **R2 in landscape**, start side, then end side (§4, Landscape):
    app bar, position, letters, the banner or notice and its action,
    the grid (or `IdReviewWords`), then the chips, the letter keys,
    delete and clear, then the screen's actions, primary first.
  - **C3**: app bar, title, body, and while it shows `IdCheckWrong`
    (above the entry, §4) with `TryAgain` then `IdShowAgain`, then
    position, letters, the chips, the letter keys, delete and clear.
    In landscape, the end side's chips and keys follow the same way.
  - **R3**: app bar, the notice while one shows and then its action
    (§6: under the app bar), the question (`IdR3Question`),
    `CopyPeerId` (§5), then `IdR3Yes` and `IdR3No`. The PeerId and
    `IdR3NoPeerId` are text and take no focus.
  - **C4**: the title (`IdCreatedTitle`, or `IdAlreadyTitle` while
    `IdAlreadyCreated` shows), the notice while one shows
    (§6: below the title), `CopyPeerId` (§5; not while `IdAlreadyCreated`
    shows), then the primary: Continue (`IdContinue`), or Try again in
    its place (§6), then, under `IdStorageUnknown` after a failed
    read-back and while it is offered, `IdStartUnchecked` (§6, A start from C4 after a failed
    read-back). C4 has no Back (§3.2), so its app bar has no back
    arrow. The PeerId, its label and `IdCreatedBody` are text and take
    no focus. When C4 opens, by any input, focus goes to its title,
    which is made focusable for this, as R4's is (below): Continue
    seals the PeerId and starts the service, so it is reached by Tab,
    not by a second Enter on a screen the person has not read.
  - The `PeerIdCopied` snackbar below Android 13 (§5) takes no focus
    and is in none of these orders.
- A typed hardware letter acts as its key, and Backspace as the delete
  key.
- An arrow moves focus to the nearest control in its direction, a
  disabled key included (below), as the platform's directional focus
  does: across a keypad row, between rows, and between the grid's
  positions. Tab and Shift+Tab follow the order above.
- On every screen of this note other than R2 and C3, Enter and Space
  press the focused control, and nothing below about typing and
  navigating applies. On R2 and C3, the screens with a keypad, what
  Enter and Space do depends on the last input:
  - **Typing** — the last input was a hardware letter, Backspace, an
    Enter that acted under this rule, or a tap on an enabled letter
    key, the delete key or the clear key. A tap on one of these keys
    counts as typing: the person is entering letters by finger instead
    of by keyboard, and an Enter that follows means the word, not a
    key they never chose.
    Enter takes the only suggestion when exactly one is shown, or the
    whole-word chip when the letters typed are a word; otherwise it
    does nothing. Space
    does nothing: no word holds a space, and a Space typed between
    words must not press whichever key holds focus.
  - **Navigating** — the last input was Tab, an arrow, a tap on any
    other control that acts, or a press of a control by Enter or Space,
    a key included. A screen reader's activate action (TalkBack's
    double tap, Orca's activate) is a press, on any control. On a
    letter key, the delete key or the clear key, it sets navigating,
    though a finger's tap on the same key counts as typing. A finger's
    tap chooses a key without moving focus to it first. A screen-reader
    user moves focus to the key and then activates it, as a keyboard
    user does with Enter. Orca does not keep Enter for itself: the app
    receives it, so the next Enter must press that key again. Enter and
    Space press the focused control, as on every other screen. After a
    key is pressed by Enter, the screen stays navigating, so a person
    who moves through the keypad can press one key twice ("pp"). A
    chip, a grid position and Try again keep the exceptions below
    however they are activated.
  - Every other input leaves the mode as it was: Back, Escape, a Space
    or an Enter that did nothing, a screen reader's activate action on
    a disabled key (it does nothing, as a tap on it does), and a tap on a
    disabled key, on text
    that does nothing when tapped (the position, the letters, the
    match count, C3's title and body, and the text of a banner, a
    notice or `IdCheckWrong` but not their actions), or on no control
    at all. Such a tap acts on nothing, so it must not turn the next
    Enter into a press of a key the person did not choose. Closing the
    discard dialog (§7), however it is closed, returns the screen to
    the mode it was in when Back was pressed.
  - The full list (§4 item 4) has no keypad and is always navigating:
    Enter and Space press the focused row or arrow, and letters and
    Backspace do nothing. Choosing a row, by any input, returns R2 in
    typing mode, an exception to "a press sets navigating", as closing
    the discard dialog is: the person is back to entering letters.
  - The same exception holds on R2 and C3 for a chip that fills a
    position, for a wrong word on C3 (a chip that fills nothing), for
    a grid position chosen to edit and for C3's Try again, by a tap or
    by Enter or Space: the screen is in typing mode afterwards, with
    focus as the focus rules below say. If the screen
    were navigating instead, the next Enter would press the key that
    focus went to and add a letter the person did not choose.
  - There is one exception to that exception: a chip that leaves no
    R2 position empty sets navigating, with focus on `IdCheckWords`
    (below). No word is left to type, so the next Enter can only mean
    "check".
  - R2 and C3 open as typing. Either way the focus ring is drawn on
    the focused control, so the person sees what Enter would press
    once they move.
- The full list (§4 item 4): when it opens, focus is on the current
  position's row; Tab goes app bar arrow, then rows 1 to 24. When it
  closes with nothing changed, focus returns to `IdReviewWords`; when
  a row is chosen, to the keypad's first enabled key (the delete key
  when the word put back leaves no letter enabled), in typing mode;
  when a font change closes it, to the grid's current position.
- A tap does not move focus. Focus stays on the control that held it and
  the focus ring stays drawn there, so a person who goes back to the
  keyboard continues from where they were and sees what Enter would
  press once they navigate. This is the app's own rule: Android's
  View system would instead clear focus from a button when it is
  touched, and
  the app draws its own focus. Where a tap
  opens, closes or changes something, the rules below say where focus
  goes, as for any other input.
- When R2 or C3 opens, and after a chip fills a position, by any
  input, focus goes to the keypad's first enabled key. There are two
  exceptions, each in a rule of its own below: a wrong word on C3,
  and a chip that leaves no R2 position empty.
- After a wrong word on C3, focus goes to `IdCheckWrong`, which is
  focusable for this, and the scrolling area brings `IdCheckWrong`
  into view. A second wrong word puts focus on it again. This is the
  error-banner rule below, and it wins over the chip rule. Why: the
  error is the one thing the person must read before the next try.
  With focus on the keypad, the error is outside the person's
  attention, and, once TalkBack reaches the app, outside TalkBack's
  too: TalkBack would say nothing of it, because no live region
  announces it (Nielsen's
  heuristic 9, help users recognise and recover from errors; WCAG
  3.3.1, error identification). Typing loses nothing: the screen is
  in typing mode afterwards, however the word was taken (above). A
  typed hardware letter acts as its key wherever focus is, and with
  the entry clear, Enter and Space do nothing. From `IdCheckWrong`,
  Tab reaches `TryAgain`, then `IdShowAgain`, in the C3 order above.
- When C3's Try again removes the error, focus goes to the keypad's
  first enabled key, in typing mode. The keypad is what the person
  uses next. The error's text and both its actions are gone, so focus
  cannot stay on them. When a right word removes the error, the chip
  rule above already puts focus on the first enabled key.
- When a chip fills R2's only remaining empty position, by any input,
  focus goes to the primary action, `IdCheckWords`, now enabled, and
  the screen is navigating, so Enter or Space presses it. The same
  holds when a chip replaces a word while no position is empty, after
  an edit or after letters were typed with every position filled (§4
  item 2). There is no next position to type into, and checking is
  the person's next step. Why an early press is safe: the check acts only on the words entered, on the record already there and on whether a read-back is owed, never on when it was pressed. A failed check keeps every word (§6). A
  valid phrase goes to R3, which asks whether the PeerId is the one
  the person wrote down. After K1, a valid phrase restores only the
  PeerId the record names (§3.4), and while a read-back is owed (§6,
  the read-back row) the check only reads back. So a second Enter pressed before
  the person meant to check loses nothing they would get by waiting. This is a
  hypothesis for the device run, falsified if a person who presses
  Enter twice does not expect the check.
- **While R2's check runs** (§6, Check running), the words are held,
  as they are behind the discard dialog: a letter key, a typed letter,
  delete, clear, Backspace, a chip, Enter, a grid position, "show
  all my words", and the action of a notice still showing (`TryAgain`,
  or `IdRestoreOther`'s Continue) do nothing, by any input. A notice
  that is showing when the check starts stays on the screen while the
  check runs. A Try again on an R2 notice (`IdBusy`, `IdStorageFailed`,
  `IdStorageUnknown`) also starts a check (after K1, under `IdBusy`
  and `IdStorageFailed` that check writes, and under
  `IdStorageUnknown` it only reads back), and the words are held in
  the same way while it runs. So no chip can be taken
  while the check runs, and no focus rule for a chip applies. The
  check moves no focus: focus stays on the control that had focus when
  the check began. After a press by Enter, Space or a screen reader's
  activate, that is the control pressed (`IdCheckWords`, or a
  notice's `TryAgain`). After a tap, it is whatever had focus when
  the person tapped that control, since a tap does not move focus
  (above). Tab, Shift+Tab and the arrows still move focus, in the
  order and by the rules above, and set navigating, as they do at any
  other time; `IdCheckWords`, shown disabled, keeps its place in that
  order, as a disabled key does. Moving focus changes no word, so
  stopping focus from moving would protect nothing. Where the grid
  shows (at 130 % or below, §4 item 4), a keyboard or screen-reader
  user who waits for the result can move through the grid and read
  each word (WCAG 2.1.1, keyboard). Over 130 %, "show all my words" is
  held, so the full list cannot be opened, and the words cannot be
  read, until the check ends.
  Enter, Space or a screen reader's activate on a held control then
  does nothing, as on a
  disabled key. Every other input that does nothing while the check
  runs leaves the mode (typing or navigating) as it was, as a tap on
  a disabled key leaves it. When the result comes, focus goes as
  follows, wherever the person moved it. A §6 banner or notice takes focus (the error-banner
  rule below). A valid phrase opens R3, with focus on R3's question
  (`IdR3Question`), which is made focusable for this, because the
  question says what R3 asks. After
  K1, a phrase from which the app derives the PeerId that the record
  names opens R4, with focus on its title, "PeerId
  restored", as on C2's PeerId page (below). On R3 and on R4, a screen
  reader starts at the text that has focus. A cancelled or denied prompt leaves R2 as it was, with
  `IdCheckWords` enabled again and focus where it was before the
  prompt, except a prompt for a read-back after K1: its cancel or
  denial shows `IdStorageUnknown`, which takes focus (§6,
  Authentication needed). Why the words
  are held: a check reports on the words it read. A word changed while
  it runs would leave R3 showing the PeerId of words no longer on the
  screen, or, after K1, restore words other than the ones checked.
  The held keys and chips keep their look while the check runs
  (after `IdRestoreOther`'s Continue, they show disabled; see below). Of R2's
  controls, only `IdCheckWords` shows disabled, with the app bar's back arrow drawn
  inactive (§7), because a check that waits on no key operation ends
  before a person can type again, and keys that went grey for that
  moment would look like a fault. This is a hypothesis for the device
  run, falsified if a check after K1, the only one that waits on a key
  operation, lasts long enough that a person types into the held
  keypad and expects a letter. If it is falsified, every held control
  shows disabled while the check runs: the letter
  keys, delete and clear, the chips, the grid positions, "show all my
  words" and the action of a notice still showing. Each keeps its
  place in the focus order and is marked disabled, as a disabled key
  is. A control that
  looks enabled but does nothing is the fault the hypothesis is
  about, whichever control it is.
- When R3's "Yes, restore it" succeeds and opens R4, by any input,
  focus goes to R4's title, "PeerId restored", as when R4 opens after
  K1 (above). Reading starts at the
  title, which says what happened. An Enter pressed right after R4
  opens presses nothing. Continue starts the service, so Continue is
  reached by Tab, not by a second Enter on a screen the person has
  not read. Where R3's yes ends in a §6 row instead (a banner, a
  notice, or a cancelled or denied prompt), that row's focus rule
  holds: a banner or notice takes focus (the error-banner
  rule below), and a cancelled or denied prompt leaves R3 as it was,
  with `IdR3Yes` and `IdR3No` enabled again and focus where it was
  before the prompt, except a prompt for a read-back: its cancel or
  denial shows `IdStorageUnknown`, which takes focus (§6,
  Authentication needed).
- **While R3's restore runs** (§6, Restore running), `IdR3Yes` and
  `IdR3No` are disabled, and Enter, Space or a screen reader's
  activate on either does nothing. The action of a notice still
  showing (`TryAgain`, or `IdRestoreOther`'s Continue) is disabled
  too, and Enter, Space or a screen reader's activate on it does
  nothing. The notice stays on the screen while the restore runs. A
  Try again on an R3 notice (`IdBusy`, `IdStorageFailed`) starts the
  restore again; `IdStorageUnknown`'s Try again, and `IdR3Yes` while
  it shows, read back instead (§6, the read-back row). R3 is held in
  the same way while either runs. The restore moves no focus: focus stays on
  the control that had focus when it began, which after a press is
  the control pressed. Tab, Shift+Tab and the arrows still move focus,
  in R3's order above; `IdR3Yes`, `IdR3No` and a notice's action,
  shown disabled, keep their places in it. `CopyPeerId` still copies, since a copy changes
  nothing the restore reads, except under the progress state (§5). When the result comes, focus goes as the
  rule above says, wherever the person moved it. Why: the restore
  writes the record. A second press of `IdR3Yes` would start a second
  restore over the first, and an Enter pressed twice by a person who
  meant one press would do it: the rule that an Enter pressed right
  after R4 opens presses nothing protects that person only after R4 is
  open. `IdR3No` would return to R2 and let the words change while the
  restore was writing the record from the words it read. `IdR3Yes`,
  `IdR3No` and a notice's action show disabled, unlike R2's held keypad
  keys, because a restore always waits on a key operation, so it can
  last long enough for a person to try to press one of them, and R3's
  only other action is `CopyPeerId`, which still copies (except
  while the progress state shows, when `CopyPeerId` is covered or
  drawn disabled, §5): a control
  that looks enabled and does nothing is the fault that §8's
  hypothesis for R2 is about.
- **While C4's seal runs** (§6, Seal running), C4 is held as R3 is
  while its restore runs (above). Its primary, Continue or Try again,
  and `IdStartUnchecked` where it shows, show disabled, and Enter,
  Space or a screen reader's activate on either does nothing. The hold
  also begins at a press of `IdStartUnchecked`. The seal moves no focus: focus stays on the control
  that had focus when it began, which after a press is the control pressed.
  Tab, Shift+Tab and the arrows still move focus in C4's order above,
  where the disabled primary and `IdStartUnchecked` keep their places. `CopyPeerId` still
  copies wherever it shows (§6), since a copy changes nothing the seal reads, except under
  the progress state (§5). When the result
  comes, focus goes as follows, wherever the person moved it. A §6
  notice takes focus (the error-banner rule below), except
  `IdStartBusy` shown again by a start retried at a return: no press
  made that start (A start retried at a return, below). A cancelled or
  denied prompt for the seal leaves C4 as it was,
  with its primary enabled again and focus where it was before the
  prompt. One for its read-back shows `IdStorageUnknown`, which takes
  focus as any §6 notice does (§6, Authentication needed). A cancelled or denied prompt for the service start, after a
  success or after Continue while `IdAlreadyCreated` shows, ends the
  flow on Chats, offline and locked, with focus on the locked notice (§6,
  Authentication needed); after the 10 s bound, the flow ends on the screen
  the person is on by then (`design/2026-10-22-add-a-peer.md` §5.1). A success, and Continue
  while `IdAlreadyCreated` shows, open the next screen, whose focus is
  `design/2026-10-22-add-a-peer.md` §5.1's. Why: a second press would
  start a second seal over the first, or start the service a second
  time, and a person who meant to press Enter once can press it twice.
  The primary shows disabled for the reason R3's controls do: the seal
  waits on a key operation and can last long enough for a person to
  press again.
- **After `IdRestoreOther`'s Continue**, or `IdStartBusy`'s Try again in its place (on R3, and on R2 after K1),
  the screen is held until the next screen opens, as R3 is while its
  restore runs and R2 while its check runs (above; §6 Restore running,
  Check running). That Continue, or that Try again, shows disabled, and
  Enter, Space or a screen reader's activate on it does nothing. Every control that the
  R3 and R2 bullets above hold is held here too, and Back does
  nothing (§7). On R2, every held control shows disabled until the
  next screen opens, as that Continue does: the letter keys, delete
  and clear, the chips, the grid positions, "show all my words" and
  `IdCheckWords`. Each keeps its place in the focus order. For a
  check, the device run decides whether a check lasts long enough to
  need the disabled look (R2's held-look hypothesis, above). For the
  service start that this Continue makes, the look is decided, not
  measured: the hold lasts until the next screen opens. That is at
  most add-a-peer §5.1's 10 s bound from the press, plus any time a
  system prompt is up, since the bound does not count that time. At
  the bound Chats opens and the start goes on behind it (add-a-peer
  §5.1). Such a hold is long enough
  for a person to type into a keypad that looks enabled, and a
  control that looks enabled and does nothing is the fault that the
  hypothesis is about. The start moves no focus, and
  Tab, Shift+Tab and the arrows still move it. A cancelled or denied
  prompt for the service start ends on Chats, offline and locked,
  with focus on the locked notice, as after C4's Continue while
  `IdAlreadyCreated` shows (§6, Authentication needed); after the
  10 s bound it ends on the screen the person is on by then
  (`design/2026-10-22-add-a-peer.md` §5.1, a refused service start). The next
  screen and its focus are `design/2026-10-22-add-a-peer.md` §5.1's.
  Why: this Continue starts the service, as C4's does under
  `IdAlreadyCreated`, and a second press would start the service a
  second time; while the service starts, `IdR3Yes` or a chip would act on a screen
  that is about to close.
- **A start refused with the profile in use after the bound** moves
  no focus, on Chats or in a conversation. Its Try again moves none
  at its press, and its results move focus as §6 says (A start
  refused with the profile in use after the bound). Each case is a
  row of `design/2026-10-22-add-a-peer.md` §8 item 1's table.
- **A start retried at a return** to C4, R4, R3 or R2 after K1 while
  `IdStartBusy` shows there (§6, A return to a screen showing
  `IdStartBusy`) moves no focus while it holds the screen. Refused
  with the profile in use again, it leaves focus where it was and
  `IdStartBusy` takes no focus, as an exception to the error-banner
  rule: this result answers no press. Its other results move focus as after the
  Continue it replaces (the bullets above). It is a row of
  `design/2026-10-22-add-a-peer.md` §8 item 1's table. Try again's
  start, if it answered before a foreground-only Home stopped the
  service (§6, Foreground-only Home and a start's answer), moved focus
  at its answer as Try again's result does, and the return keeps that
  focus: on the next screen, add-a-peer §5.1's; on `IdStartBusy`
  again, the notice; the retry at the return then moves no focus.
- When a grid position is chosen to edit, by any input, focus goes as
  for a row chosen from the full list: to the keypad's first enabled
  key, or to the delete key when the word put back leaves no letter
  enabled.
- When the letters disable the focused key, focus stays on it: a
  disabled key keeps its place in the focus order (below), so neither
  focus nor the keys move while the person enters letters.
- When the focused chip leaves the suggestions row because the letters
  changed, by a tap, a hardware letter or Backspace, focus goes to the
  keypad's first enabled key. The mode is the one the input set.
- When a refused start raises a screen in place of the one the person
  was on or was opening, focus goes to that screen's heading, which
  is focusable for this. The screens are K1 (§3.4) and the store's R
  and K (`design/2026-10-21-contact-cards.md` §7.1). This holds at a
  launch and over a screen already showing, such as Chats, a
  conversation or Settings, whatever had focus there. Any on-screen
  keyboard closes, since none of these screens has a text field. Tab
  then follows the reading order. On R it reaches Try again, Delete
  stored data… and Close; on K, Delete stored data… and Close; on K1,
  its primary action when it has one. The heading comes first, not the
  primary action, because the person expected another screen. Read
  from the top, the screen says what happened before what to do, and
  the first Tab reaches the primary action. The desktop's R is a
  dialog window and puts its initial focus on Try again (contact-cards
  §7.1). Where a start made while R is showing is refused again and R
  stays, R was not raised, so this rule does not apply. That holds for
  the start that Try again makes, and also for a restart the platform
  runs. Focus stays where it was, so after Try again it stays on Try
  again, as unlock note F11 says for a cancelled prompt
  (`design/2026-11-05-unlock-policy-pending.md`).
- After an error banner appears, focus goes to the banner. C3's
  `IdCheckWrong` counts as one (above). `IdStartBusy` shown again by
  a start retried at a return takes no focus (A start retried at a
  return, above).
- After a dialog closes, focus returns to what opened it. The discard
  dialog's `IdDiscard` leaves the screen instead, and §7 names where
  focus goes then.
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

The text is final (language-culture, 2026-10-19; the keys added on
2026-10-20 for the PeerId page, language-culture-en, REPLYs
01a1589d-e769 and 01a1589f-7882; `IdS1Unlock` on 2026-10-21,
language-culture-en, REPLY 01a15d22-6abd, at the owner's asking for
Unlock; `IdActiveElsewhere` and `IdR1NewKeys` on 2026-10-23,
language-culture-en, REPLY 01a16717; the PeerId rewording of W, C1,
C4, R1, R4, K1 and the storage and other-PeerId states on 2026-10-23,
language-culture-en, REPLYs 01a16739, 01a1673f
and 01a1674b). No string contains
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
| `IdWelcomeBody` | Exchange messages with the people you trust, from this phone. There is no account. Your contacts know this phone by its PeerId, a public code. | W's body, three lines; its last sentence says what a PeerId is, before the primary names it. No "directly": a message may go through a relay |
| `IdVoidPaper` | The words shown last time were never used, so they do not restore this phone's PeerId. Do not use the paper where you wrote those words as your backup. | W's notice while the void-paper marker is set (§3.1), three lines. It says what the old paper is not, and what not to do with it |
| `IdAlreadyCreated` (new) | The words you just saw were not used. Do not use the paper where you wrote those words as your backup. | C4 when another first run sealed first (`AlreadyProvisioned`, §6). `IdVoidPaper` says "shown last time", which is false on C4. The notice shows under `IdAlreadyTitle`, which is read first and says that this phone already has a PeerId, so the notice does not say that again |
| `IdCreate` | Create a new PeerId | W's primary |
| `IdRestore` | Restore my PeerId | W's secondary |
| `IdC1Title` | About your recovery phrase | C1's title |
| `IdC1Point1` | These 24 words are the key to your PeerId. With them, you can move your PeerId to a new phone. | C1, first point: a move to a new phone, not a second phone in use at the same time |
| `IdC1Point2` | They are an InterWeave identity recovery phrase, not a cryptocurrency wallet seed. | C1, second point, in the order of §3.2's list (`IDENTITY-RECOVERY.md`) |
| `IdC1Point3` | Write them on paper, in order, with your PeerId. Store the paper in a safe place. | C1, third point. The PeerId is named here, because R1 asks for it later |
| `IdC1Point4` | Anyone who has these words can act as you in InterWeave. Do not show them to anyone. | C1, fourth point: "can be you", without alarm |
| `IdC1Point5` | The words restore your PeerId only, not your name and photo, contacts, trust, settings or messages. | C1, fifth point: `human-client-ui.md` §9 as amended (2f3b0587). It names the fact and asks for nothing, since Android has no export of the rest |
| `IdC1Point6` | Your PeerId is created only after you check two of the words. | C1, sixth point (§11 Q3) |
| `IdShowWords` | Show my words | C1's primary |
| `IdNotWalletSeed` (new) | InterWeave identity recovery phrase, not a cryptocurrency wallet seed | C2 and S2, above the words. One line at 100 % only on a wide screen: 68 characters. The contract's label also says "transport"; it is left out as a word the person does not know |
| `IdFormatLine` (new) | Phrase format: | C2 and S2, before the untranslated format identifier (§3.2), one line. "Phrase format" says whose format it is; a bare "Format" could be read as the screen's layout. The label stands alone before a value, so a translation needs no template |
| `AuthenticationNeeded` (the app's, #313, not yet on main) | InterWeave is offline because it is locked. To unlock it, tap Unlock and enter your phone's PIN, pattern or password. Your conversations are saved on this phone and appear again after you unlock InterWeave. | The locked notice: offline after a cancelled unlock at S2's Done, at a launch, or after C4's or R4's continue (§6) |
| `Authenticate` (the app's, #313, not yet on main) | Unlock | The locked notice's action; it raises the prompt (§6) |
| `UnlockPromptTitle` (the app's, #313, not yet on main) | Unlock InterWeave | The system prompt's title for a key operation or a launch; S1's prompt keeps its own |
| `CopyPeerId`, `PeerIdCopied` (the app's, #313, not yet on main) | Copy PeerId · PeerId copied | The button under a PeerId and its confirmation below Android 13 (§5) |
| `IdStartUnchecked` (new) | Go online with this PeerId | C4's text button under Try again while `IdStorageUnknown` shows after a failed read-back and it is offered (§6, A start from C4 after a failed read-back). It must say that the app starts with the PeerId shown, and never that a PeerId was created or saved. "Go online" is `IdStartBusy`'s verb for the same start |
| `IdStorageUnknownRestore` (new) | If the app then asks for your recovery phrase, enter the phrase you just wrote down. It restores this PeerId. | C4's line under `IdStartUnchecked` (§6). It must say what the person does if the start leads to K1, and that the phrase restores the PeerId (§11d, Q15). It names K1 by what K1 asks for (`IdQTitle`), so it promises nothing where K1 cannot restore (`IdQTitleNoPath`). "Then" is the press of `IdStartUnchecked` above it |
| `IdStorageUnknown` (new) | The app could not tell whether your PeerId was saved. Try again to check. | §6's notice when the read-back fails, two lines, with `TryAgain`. It never says that nothing changed. "To check" is true, because Try again reads back and writes nothing |
| `IdRestoreOther` (new) | Your PeerId was not restored. This phone already has another PeerId saved, shown below. | §6's another-identity-on-record row, at a restore (a record that names a PeerId other than the one restored), two lines, the PeerId in full below it. "Another PeerId saved, shown below" ties the shown PeerId to the phone's record, so it is not read as the person's own |
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
| `TryAgain` (exists) | Try again | C3's other action after an error, the retry action of §6's notices on R2 and R3, and C4's primary in Continue's place while `IdBusy`, `IdStorageFailed` or `IdStorageUnknown` shows (§6), and the action of `IdStartBusy`: in Continue's place on C4 and R4, in `IdRestoreOther`'s Continue's place on R3 and R2 after K1, and on its session notice after the bound (§6). The key exists, and a second key with the same text would fail `every_ui_text_appears_once_in_all`. One action has one name, so §6 says "Try again" too |
| `IdCreatedTitle` | PeerId created | C4's title, except while `IdAlreadyCreated` shows |
| `IdAlreadyTitle` (new) | This phone already has a PeerId | C4's title while `IdAlreadyCreated` shows (§6): no PeerId was created, so `IdCreatedTitle` would be false. It takes C4's first focus, so it says what happened. The notice under it then says what that means for the paper, and does not repeat the title |
| `IdRestoredTitle` (new) | PeerId restored | R4's title. "PeerId created" would be false after a restore |
| `IdCreatedBody` | When you continue, the app connects to the network with this PeerId. | C4's and R4's body |
| `IdContinue` | Continue | C4's and R4's primary, and `IdRestoreOther`'s Continue (R3, and R2 after K1; §6) |
| `IdR1Title` | Restore your PeerId | R1's title |
| `IdR1Body` | You need your 24 words, and the PeerId that goes with them: from your paper, or from a contact who has it. The app stays offline until the restore is done. | R1's body, three lines. "The PeerId that goes with them" is true whether the person moved phones or reinstalled on the same one |
| `IdEnterWords` | Enter my words | R1's primary |
| `IdActiveElsewhere` (new) | If another device still uses this PeerId, a restore here puts the same PeerId on two devices. One PeerId must never be on two devices, and this phone cannot check whether another device uses it. | R1, and K1 when it offers a restore, before the primary (§3.3, §3.4); `human-client-ui.md` §9, ADR-0043; must say the phone cannot check, and ask nothing |
| `IdR1NewKeys` (new) | Your words restore your PeerId, but not the keys of your identities. Each identity gets a new key, and your contacts will see a new key for each identity. | R1 and K1, each when the state root has no `identity-keys/` (always on R1 after a fresh install), before the primary; `human-client-ui.md` §9, ADR-0063 decision 1; must say the phrase restores the device, not the identities' keys, and that contacts see a key change |
| `IdCheckWords` | Check my words | R2's primary |
| `IdR1Settings` | Your words restore your PeerId only. Your name and photo, contacts, trust, settings and messages are not part of them. | R1's line under the body, three lines: the same statement, with nothing asked |
| `IdR3NoPeerId` | If the PeerId is not on your paper, ask a contact for the one they have for the phone that had these words. Restore only if it matches. | R3, under the question: about three lines at 100 %, one over the space. A contact can hold one PeerId per device, so the text names which one |
| `IdR3Question` | Is this the PeerId that you wrote down? If it is not, a word may be wrong. | R3's question, two lines. It asks; it never says the PeerId is right, and it says what a "no" usually means |
| `IdR3Yes` | Yes, restore it | R3's primary |
| `IdR3No` | No, check my words | R3's secondary |
| `IdQTitle` | This phone needs your recovery phrase | K1's title |
| `IdQReasonKey` | This phone's screen lock or security changed, so the app cannot use your PeerId. Restore it with your recovery phrase. | K1 for `KeyInvalidated` and `KeyMissing`. It names the likely cause in the person's terms, and the way back |
| `IdQReasonRecord` | The app cannot use the PeerId saved on this phone. Restore it with your recovery phrase. | K1 for `Record`, `Authentication` and `WrongIdentity` |
| `IdEnterPhrase` | Enter my recovery phrase | K1's primary |
| `IdQLost` | If your words are lost, clear this app's data in the phone's settings. That gives this phone a new PeerId, which each contact must trust again. | K1's plain text below its primary (§3.4), three lines |
| `IdQTitleNoPath` (new) | The app cannot use your PeerId | K1's title when the record names no readable PeerId (§3.4). `IdQTitle` asks for the phrase, which cannot help there |
| `IdQReasonNoPath` (new) | The app cannot use the PeerId saved on this phone, and it cannot be restored here. | K1's reason in the same case. Both other reasons end with "Restore it with your recovery phrase", which is false there |
| `IdQNoPath` (new) | To use the app again, clear this app's data in the phone's settings. That gives this phone a new PeerId, which each contact must trust again. | K1's plain text in the same case, in place of `IdQLost`, whose "If your words are lost" suggests the words would help |
| `IdExpectedPeerId` (new) | PeerId this phone expects | The label of the expected PeerId, on K1 and in `IdOtherIdentity`'s banner |
| `IdDerivedPeerId` (new) | PeerId from your words | The label of the derived PeerId in `IdOtherIdentity`'s banner |
| `IdShowPhraseItem` | Show recovery phrase | The Settings item (§3.5) |
| `IdS1Title` | Show your recovery phrase | S1's title |
| `IdS1Unlock` (new) | Unlock and show my words | S1's primary; it raises the system prompt |
| `IdS1Body` | You will be asked to unlock your phone first. The app goes offline while your words are shown. | S1's body, two lines |
| `IdGoingOffline` | Going offline… | The progress state (§3.5, §6) |
| `IdDone` | Done | The primary of S2's PeerId page, its last page |
| `IdChecksumFailed` | These words do not form a recovery phrase. A word may be wrong or in the wrong place. Check them against your paper. | §6's checksum banner |
| `IdOtherIdentity` | These words belong to a different PeerId, so nothing on this phone was changed. Compare the two PeerIds below with your paper. | §6's other-identity banner, above the two labelled PeerIds |
| `IdBusy` | The app could not go offline. Wait a moment, then try again. | §6's busy notice, with `TryAgain` |
| `IdStartBusy` (new) | The app could not go online because this profile is in use. Wait a moment, then try again. | §6's row for a service start refused with the profile in use, with `TryAgain`; after the bound, a session notice on Chats or in a conversation, with `TryAgain` as its action (§6, A start refused with the profile in use after the bound), so the same words must read right there, where no flow screen is in view. It must say that the start failed; it must not say that the PeerId was not created or not restored, since the record already holds the identity (after `IdRestoreOther`, the phone's other PeerId) |
| `IdStorageFailed` | The app could not save your PeerId on this phone. Nothing was changed. | §6's storage notice, with `TryAgain` |
| `IdDiscardTitle` | Delete the words you entered? | §7's dialog title |
| `IdDiscard` | Delete words | §7's dialog: the action that drops the words |
| `IdContinueEntering` | Continue entering | §7's dialog: the default action |
| `IdKeyDelete` | Delete last letter | The keypad's delete key (its accessible label) |
| `IdKeyClear` | Clear this word | The keypad's clear key (its accessible label) |
| `IdReviewWords` | Show all my words | The grid's button at large font (§4), and the title of the full list it opens (§4 item 4) |
| `AddClose` (exists) | Back | The accessible label of the app bar's back arrow (§3), first on the full list (§4 item 4). It is the add page's back arrow: one act, going back, under one name, so the key is reused as `TryAgain` is; a second key with the same meaning would need an exception in `every_ui_text_appears_once_in_all` the test does not allow for |

Against the list the proposal gave: `IdC1Points` is six keys, one per
point in the order shown, so each wraps alone. `IdPeerIdLabelShown`, `IdRestoredTitle`,
`IdExpectedPeerId`, `IdDerivedPeerId`, `IdAlreadyCreated`,
`IdQTitleNoPath`, `IdQReasonNoPath`, `IdQNoPath`, `IdNotWalletSeed`,
`IdFormatLine`, `IdStorageUnknown` and `IdRestoreOther` are added, and
on 2026-10-20 `IdPeerIdTitle`, `IdPeerIdWritten`, `IdNext` and
`IdPeerIdNoteShown`, on 2026-10-21 `IdS1Unlock`, and on 2026-11-18 `IdStartBusy`. The locked notice and its prompt
(`AuthenticationNeeded`, `Authenticate`, `UnlockPromptTitle`) are the
app's keys, not this flow's, added to `labels.rs` by #313 and not yet
on main; so are `CopyPeerId` and `PeerIdCopied` (§5). Their rows are
in the table above. `IdTryAgain` and
`IdRetry` are both `TryAgain`. `IdKeepEntering` is
`IdContinueEntering`. `IdBack` is `AddClose`.

## 10. Acceptance

On the Pixel 9a, dark and light, at 100 % and 200 % font, portrait and
landscape:

1. A first launch with no record shows W. Back leaves the app, and the
   next launch shows W again.
2. Create: C1 → C2 shows 24 numbered words and no PeerId. Its primary
   leads to the PeerId page, which shows the PeerId in full, grouped
   by four, and no word; Back there shows the same words. S2 has the
   same two pages. A screenshot (`adb exec-out screencap`) of both C2
   pages, of C3, of R2 (also when reached from K1), of R2's full list
   (§4 item 4) and of both S2 pages is black, and Recents shows no content for the flow.
3. Long-press on any word copies nothing, and no word ever reaches the
   clipboard. The flow writes to the clipboard in two ways only:
   "Copy PeerId", which gives the canonical PeerId with no spaces,
   checked by pasting it into Trust's field; and the removal of the
   PeerId that "Copy PeerId" put there, when `IdAlreadyCreated` shows
   on C4 (§6).
4. C3 asks two positions. A wrong word shows `IdCheckWrong` and clears
   the entry. Try again removes the error and both actions, and so
   does a right word. Two right words reach C4. Continue starts the
   service and opens Connect when both reads succeed with no address and no trusted
   peer, Chats when either list has an entry, and Chats with no Connect
   when either read fails (a test hook).
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
   and the PeerId on the Settings screen matches. R1 shows
   `IdActiveElsewhere` and `IdR1NewKeys` before its primary, in that
   order on the screen, with no control between them and nothing to
   accept; R2 shows neither. The tester checks that order by looking
   at the screen. The check of the same order in the accessibility
   tree is blocked, not passed, on Android until the build exposes an
   accessibility tree, as in item 12 (§8). TalkBack reads them once
   TalkBack reaches the app.
8. A pinned fixture phrase of distinct words, with a pinned swap of two
   positions that the test first shows fails the BIP-39 checksum (an
   8-bit checksum passes about one swap in 256), shows
   `IdChecksumFailed`, keeps every word, and lets any position be
   edited. The fixture and the swap are named in the test.
9. Recovery required: with the record's key invalidated (rust-ui-dev's
   test hook), launch shows K1 with `IdQReasonKey` and the expected
   PeerId, and `IdActiveElsewhere` after its reason (with
   `IdR1NewKeys` only when the hook also removes `identity-keys/`).
   The right phrase restores with no R3. Another identity's
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
   record shows `IdAlreadyCreated`, with `IdAlreadyTitle` as C4's
   title in place of `IdCreatedTitle`, the notice right above
   Continue, and no PeerId, no PeerId label, no `CopyPeerId` and no
   `IdCreatedBody` (§6), and its Continue opens the identity on
   record. A C4 whose directory sync fails
   after the record is written (a test hook) reads the record back and
   reaches Connect or the Chats screen (C4's rule) as a success, never `IdStorageFailed` or
   `IdAlreadyCreated`; so does a C4 that answers `OldKeyRemains`.
10. Show my phrase:
    - a cancelled prompt shows nothing and no error;
    - a succeeded prompt shows `IdGoingOffline`, then the words;
    - connectivity reads offline while they are shown;
    - Done restarts the service and returns to Settings;
    - on a `user-presence` profile, with more than 60 s on S2, Done
      raises the prompt before the service runs (the restart is refused
      and the prompt answers it). A cancelled prompt returns
      to Settings offline, with the words gone. The same profile's C4
      and restore raise the prompt outside the window, and a cancel
      there commits nothing.
    - After a cancel of the unlock prompt at S2's Done, the app
      shows Settings with focus on "Show recovery phrase"; at a
      launch, or at the start after C4's or R4's continue (not at
      C4's seal or a restore's seal, which stay on their screen as §6
      says), it shows Chats with focus on the locked notice. Either
      way connectivity reads offline, and on Chats the notice
      (`AuthenticationNeeded`) shows with its Unlock action
      (`Authenticate`); nothing raises the prompt again by itself. A return to
      the same Activity (Home or Recents, then back) raises no prompt:
      it binds silently if the window has opened meanwhile, and
      otherwise keeps the notice and Unlock. Leaving while the system
      prompt shows counts as a cancel. A cold launch past the window
      (also the first after the system killed the process) raises the
      prompt before the service runs (rust-ui-dev-01,
      QUESTION 01a155b3-448f; the person who declined is not asked
      again each time they switch in: the Android review's S4, user
      control and freedom).
    - On a `user-presence` profile past the window, a C4 whose seal
      succeeds and whose start's prompt is cancelled ends on Chats,
      offline, with the locked notice and focus on it; the identity
      exists (a relaunch does not show W).
11. Leaving R2 with words entered (Home) and returning shows every
    word still entered. After the process is killed (`adb shell am
    kill` while in the background), the next launch starts at the
    flow's first screen with nothing entered. Rotation keeps every
    word. Leaving S2 and returning to the same instance shows the
    words with no new prompt.
12. Profile busy and storage failure, by rust-ui-dev's test hooks, show
    §6's notices. A refused write whose read-back fails (a hook) shows
    `IdStorageUnknown`, never "nothing was changed". Try again works.
    In a host test, on C4, on R3 and on R2 after K1, with
    `IdStorageUnknown` shown: its Try again, and on R3 `IdR3Yes` and
    on R2 `IdCheckWords`, read back and write nothing (the test's
    custody records no seal and no restore). A read-back answered as
    opening to the PeerId shown or confirmed goes on as a success; one
    that fails shows `IdStorageUnknown` again, with focus on it. On a
    `user-presence` profile, a read-back refused because
    authentication is needed raises the prompt; granted, the read-back
    runs again; cancelled, `IdStorageUnknown` shows with focus on it,
    and C4's primary is Try again, not Continue. A restore answered
    `OldKeyRemains` on R3 and on R2 after K1 reads back before R4
    opens, as a seal answered `OldKeyRemains` does on C4 (the test's
    custody counts its `unlock` calls and records one; rust-ui-dev adds
    that count if the custody double lacks it, and until then this step
    is blocked, not passed). On R2 after K1 with `IdStorageUnknown`
    shown, the test first presses Try again and answers that read-back
    as failing, so `IdStorageUnknown` shows again. Then an edit of one word: `IdStorageUnknown` is removed (§6, An edit on R2). Then the word is edited back and `IdCheckWords` is pressed: the custody records
    one more read-back and no restore; answered as failing,
    `IdStorageUnknown` shows with focus on it; in a second run,
    answered as opening to the confirmed PeerId, R4 opens. An edit to
    words of another PeerId, then `IdCheckWords`, shows
    `IdOtherIdentity` and records no read-back and no restore. These
    runs need the same `unlock` count, and are blocked until it
    exists. With the checksum banner, and in a second run with
    `IdOtherIdentity`, and in further runs with `IdBusy`,
    `IdStorageFailed`, `IdRestoreOther` and `IdStartBusy` after K1, a
    tap on a grid position removes the banner or notice, and typing a
    letter before that removes nothing. With `IdStorageUnknown` shown
    and focus on its Try again (moved there by Tab), a tap on a grid
    position removes the notice and puts focus on the keypad's first
    enabled key, in typing mode; in a second run, a tap on the
    position's chip that replaces its word removes it and puts focus
    on `IdCheckWords`, navigating. On R2 after K1 with
    `IdStorageUnknown` shown, a rotation, then an edit of one word
    (the notice is removed), then the word edited back and `IdCheckWords` pressed: the
    custody records one more read-back and no restore. In a second
    run, a recreation of the window, as a change of the system language makes it, takes the rotation's place, and in a third
    run, the app sent to the background and back. Each is blocked on
    the same count, and the second run is also blocked until the
    build keeps R2 through a recreation (rust-ui-dev-01's j311; §6,
    The hold). Removing the notice first is what makes each run
    check the read-back owed and not the notice: with the notice
    shown, `IdCheckWords` reads back by the notice's rule alone. A read-back that keeps
    failing, in host tests: on R2 after K1, Back, `IdDiscard`, K1's
    action to R2 again, the same 24 words and `IdCheckWords`: the
    custody records one restore; on R3, Back, `IdCheckWords` on R2,
    then `IdR3Yes`: R3 shows no notice before the press, and the
    custody records one restore. The custody double's record, in a
    second run of each, has a key answered as invalidated, and the
    restore seals over it, so R4 opens. In a third run on R3, the
    words are edited to another valid phrase before `IdCheckWords`,
    and the restore is answered `RecordNamesOther`: R3 shows
    `IdStorageUnknown`, with focus on it, and the custody records no
    write. Then Try again: the custody records one read-back, against
    the PeerId the record names; answered as opening, R4 opens with
    that PeerId. On C4, in host tests (§6, A
    start from C4 after a failed read-back): with `IdStorageUnknown`
    shown after a read-back answered as failing, `IdStartUnchecked`
    and `IdStorageUnknownRestore` show under Try again, and Tab from
    Try again reaches `IdStartUnchecked`; after a cancelled read-back
    prompt instead, neither shows. Its press: the custody records no
    seal and no restore, the flow's lock is let go before the start,
    and the app compares the identity that the start unlocks with C4's PeerId. Answered as opening: Chats.
    Answered `RecoveryRequired`, and in a second run with its
    `WrongIdentity` cause: K1, with the record's PeerId, C4's, as the
    expected one. Answered `Unavailable`: `IdStorageUnknown` again with
    both controls and focus on the notice. Answered `LockHeld`:
    `IdStartBusy` with Try again only. On a `user-presence` profile,
    the start's prompt cancelled: Chats, offline, with the locked
    notice focused. While a Try again under the notice runs,
    `IdStartUnchecked` shows disabled. With focus on `IdStartUnchecked`,
    Enter, with the start left unanswered: the control shows disabled,
    focus stays on it, and a second Enter, a Space and a screen
    reader's activate start nothing (the hook records one start).
    Answered as opening, with Connect's condition holding (empty
    reads): Chats opens, not Connect. With the double's record
    naming and opening to another PeerId than C4's: nothing starts
    (the hook records no service start), and `IdStorageUnknown` shows
    with Try again only and focus on the notice. Then Try again,
    answered by the read-back as opening to that other PeerId:
    `IdStorageUnknown` shows again, and `IdStartUnchecked` stays
    hidden. This run needs the app's comparison before the start, and
    is blocked until it exists.
    Back goes where §6 (Back from a notice) says: on R2 after K1, to
    K1, through the discard dialog, with the words dropped only by
    `IdDiscard`; on R3, to R2 with all 24 words kept. On C4, Back
    does nothing.
    On R3, a hook's `IdStorageFailed` shows under the app bar, above
    the question (§6), and the tester checks that place by
    looking at the screen. The check of its place in the accessibility tree (right
    after the app bar, before `IdR3Question`) is blocked, not passed,
    on Android until the build exposes an accessibility tree (§8;
    acceptance 15's screen-reader check). Until then, acceptance 15's
    Tab runs check R3's Tab order.
    On C4, a hook's `IdStorageFailed` shows below the title, above the
    PeerId's label (§6), checked the same way; the check of its place
    in the accessibility tree is blocked in the same way.
    A hook's `IdStartBusy` (§6, the row for a service start refused
    with the profile in use) shows on C4 after a seal and on R4
    below the title, above the PeerId's label, with Try again in
    Continue's place; on C4 under `IdAlreadyCreated`, below
    `IdAlreadyTitle`, above Try again, with the PeerId, its label,
    `CopyPeerId` and `IdCreatedBody` hidden; on R3, under the app
    bar, above the question; and on R2 after K1, above the grid. On
    R3 and on R2 after K1 the PeerId that `IdRestoreOther` showed is
    kept under it, and Try again is in the place of
    `IdRestoreOther`'s Continue. The tester checks each place by
    looking at the screen, and the check of its place in the
    accessibility tree is blocked in the same way. rust-ui-dev adds
    the hook, as it adds this item's other hooks; until it exists,
    these checks are blocked, not passed.
13. Run twice: at the Pixel 9a's own size, and set to 360 × 640 dp
    (`adb shell wm size 945x1680` at its 420 dpi, through
    `phone-run`, then `wm size reset`), the size §4's figures are for.
    - At 200 %, R2's "show all my words" button, position, letters,
      suggestions, keypad and primary action fit without scrolling,
      and no word is cut. While a §6 banner shows, the scrolling area
      is at least 48 dp tall and scrolling it reaches the button.
    - At 100 % in portrait the grid is above the picker in a
      scrolling area, the keypad, suggestions and primary stay in view
      while all 24 words are entered, and the current position's row
      is always visible.
    - On C3 at 200 %, after a wrong word, `IdCheckWrong`, `TryAgain`
      and `IdShowAgain` can be reached, scrolling the area above the
      picker if they must, with the picker in view.
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
    §8:
    - Tab from the app bar visits §8's order for R2 in portrait and in
      landscape, with the checksum banner shown and, by a test hook,
      with `IdStorageFailed` shown; for C3 with `IdCheckWrong`
      shown; for R3, with no notice and, by a test hook, with
      `IdStorageFailed` shown; and for C4, from its title, in two runs: by
      a test hook with `IdStorageFailed` shown, the notice,
      `CopyPeerId`, then Try again in Continue's place; and by a test
      hook with `IdAlreadyCreated` shown, the notice, then Continue.
      A newly opened C4 has focus on its title, and Enter then
      presses nothing.
    - On C4, held while the seal runs (§6 Seal running, §8): Tab to
      Continue, then Enter, and the test leaves the seal unanswered.
      Meanwhile a second Enter, a Space and a screen reader's activate
      on Continue start no second seal, focus stays on Continue,
      Continue shows disabled, and Shift+Tab to `CopyPeerId` then
      Enter puts the PeerId's canonical string on the clipboard. When
      the test answers the seal with a refusal that wrote nothing,
      `IdStorageFailed` shows with focus on it, and Try again is
      enabled in Continue's place. Enter on Try again holds C4 the
      same way until the test answers again. In a second run, the test
      answers the seal as authentication needed and then the prompt
      as cancelled: C4 stays as it was, with Continue enabled and
      focus on it. In a third run, by a test hook with
      `IdAlreadyCreated` shown, Enter on Continue leaves C4 open until
      the test answers the service start. Meanwhile a second Enter, a
      Space and a screen reader's activate on Continue do not start
      the service a second time, focus stays on Continue, and Continue
      shows disabled. These three runs are host tests: no device hook
      holds the seal. In a fourth host-test run, with the platform
      version below 13, the seal is answered `ProfileLocked`, and while the progress state (`IdGoingOffline`)
      shows, Enter on `CopyPeerId`, reached by Shift+Tab before the
      answer, leaves the clipboard as it was and shows no snackbar
      (§5). In a fifth host-test run, focus is on `CopyPeerId` and
      Continue is pressed by a tap, with the seal left unanswered:
      Continue shows disabled, focus stays on `CopyPeerId` (a tap
      moves no focus), and Enter then puts the PeerId's canonical
      string on the clipboard.
    - On R3, and on R2 after K1, held after `IdRestoreOther`'s
      Continue (§6 Restore running, Check running, §7, §8): by a test
      hook, the restore (on R2, the check) is answered with a record
      naming another PeerId, and `IdRestoreOther` shows with focus on
      it. On R2, the test first types "car", so the "car" chip shows.
      Tab to that Continue, then Enter, and the test leaves the
      service start unanswered. Meanwhile a second Enter, a Space and
      a screen reader's activate on that Continue do not start the
      service a second time; that Continue shows disabled, and on R3
      so do `IdR3Yes` and `IdR3No`; Back does nothing and the back
      arrow is inactive; on R2 every held control shows disabled; a
      press of the "d" key leaves "car" as it was ("d" continues "car",
      as in "card", so its key is enabled when the screen is not held);
      Enter on the "car" chip, reached by Tab, fills no position; focus stays on that Continue, and Tab moves it. When the
      test answers the start's prompt as cancelled, Chats opens,
      offline and locked, with focus on the locked notice. These runs
      are host tests.
    - A service start refused with the profile in use (§6): by a test
      hook, the service start after C4's Continue with the seal
      succeeding, after R4's Continue, after C4's Continue under
      `IdAlreadyCreated`, and after `IdRestoreOther`'s Continue on R3
      and on R2 after K1 is answered `StartRefused::Runtime` with
      `LockHeld`. On each, no progress state shows, the hold ends,
      `IdStartBusy` shows with Try again in the primary's place and
      focus on the notice, and no other notice shows. Enter on Try
      again makes one service start, which the hook answers with
      `LockHeld` again: the test's custody records no seal and no
      restore, on C4 no `IdAlreadyCreated` shows, and `IdStartBusy`
      shows again with focus on it.
      On C4 after a seal, its PeerId, label, `CopyPeerId` and
      `IdCreatedBody` show from the press of Continue, through the
      held start, and under `IdStartBusy` (§6, Seal running).
      Under `IdAlreadyCreated`, C4's title stays `IdAlreadyTitle` and
      its PeerId, label, `CopyPeerId` and `IdCreatedBody` stay hidden;
      on R3 and on R2 after K1, the PeerId that `IdRestoreOther` showed stays under
      `IdStartBusy`. Then, with no input for 60 s, the hook records no
      further start. That is a bounded check of §6's "the app never
      retries the start on its own" (a return or a recreation
      apart), which has no end: 60 s is six
      times the longest timer of the start and its hold (the 10 s
      bound), so a retry driven by either would show in it. A retry
      driven by a longer timer, such as the `PeerIdCopied`
      snackbar's (acceptance 18), or later than 60 s, is not checked. Then, on
      each of the five places (C4 after a seal, C4 under
      `IdAlreadyCreated`, R4, R3, and R2 after K1, with "car" typed
      before the check as in the run above), Enter on Try again, with
      the hook leaving this start unanswered, holds the screen as after
      the Continue it replaces: a second Enter, a Space and a screen
      reader's activate start nothing, and Try again shows disabled.
      On R3, `IdR3Yes` and `IdR3No` show disabled too. On R2, every
      held control shows disabled, a press of the "d" key leaves
      "car" as it was, and Enter on the "car" chip fills no position.
      Under `IdAlreadyCreated`, C4's title stays `IdAlreadyTitle` and
      its PeerId stays hidden. Focus stays on Try again, Tab still
      moves focus, and Back does nothing. `ConnectStarting` shows 1 s
      after that Enter, and Chats opens 10 s after it, not at once.
      These runs are host tests.
    - A return to a screen showing `IdStartBusy` (§6, below the
      table): on each of the five places above, with `IdStartBusy`
      showing and focus on its text, the test waits 15 s, longer than
      the 10 s bound, then sends the app to the background and back.
      Each run is made with Stay reachable on and again in
      foreground-only mode, unless the run says otherwise. The hook records one start at the return,
      left unanswered: the screen is held as after Enter on Try again
      (the run above), with Try again disabled, and focus stays on the
      notice's text. `ConnectStarting` shows 1 s after the return, and
      Chats opens 10 s after the return, not at once. In a second run,
      the hook answers that start `LockHeld` within 0.5 s, before
      `ConnectStarting` could show: the hold
      ends, `IdStartBusy` shows with Try again enabled, focus stays on
      the notice's text, nothing is announced, and with no input for
      60 s the hook records no further start. In a third run, with
      focus on Try again before the return, answered `LockHeld`:
      focus stays on Try again. In a fourth run, answered as started:
      the next screen opens, with add-a-peer §5.1's focus. In a fifth
      run, on a `user-presence` profile, the return's start asks for
      the prompt, which rises with no input, and the test cancels it:
      Chats opens, offline and locked, with focus on the locked
      notice. In a sixth run, the test recreates the window as a
      change of the system language does: the hook records one start,
      answered as in the second run. A rotation instead records no
      start. The sixth run is blocked, not passed, until the build
      keeps a flow screen through a recreation (rust-ui-dev-01's
      j311; §6, The hold). In a seventh run, made with Stay
      reachable on only, the
      test presses Enter on Try again, the hook leaves that start
      unanswered, and the test then sends the app to the background
      and back: the hook records that one start and none at the
      return. In an eighth run, on R4 with no address recorded (so
      Connect is owed), the hook answers the return's start as
      started 8 s after the return and both reads empty 12 s after
      it: Chats opens 10 s after the return, with focus on its title,
      and Connect does not open when the reads answer. In a ninth
      run, the hook answers the start as started and both reads empty
      within 2 s of the return: Connect opens, with focus on its
      heading. In a tenth run, on C4 showing `IdStartBusy` after
      `IdStartUnchecked`'s start met `LockHeld`, the double's record
      is made to name and open to a PeerId other than C4's before the
      return: at the return nothing starts (the hook records no
      service start), `IdStorageUnknown` shows with Try again only
      and focus on the notice, and `IdStartUnchecked` stays hidden.
      Like acceptance 12's comparison run, the tenth run needs the
      app's comparison before the start, and is blocked until that
      comparison exists. These runs are host tests.
    - A service start refused with the profile in use after the
      bound (§6, A start refused with the profile in use after the
      bound). In each run below, the test records an address before
      the start, so add-a-peer §5.6's condition does not hold and
      Connect cannot open, except where a run says otherwise. Where
      a run opens a conversation or scrolls the list, the test's
      store also holds, before the start, 30 contacts, each with a
      conversation of one read inbound message, so Chats' list is
      longer than the screen at the test's window size; the
      conversation opened is the one in the list's first row. Only the human store is seeded: in the Connect runs below,
      the hook answers both of Connect's reads (add-a-peer §5.6), so
      the seeded contacts change neither answer. By a
      test hook, the service start after R4's Continue, and in a
      second run after C4's Continue with the seal succeeding, is
      left unanswered, and Chats opens at the 10 s bound with focus
      on its title. The hook then answers the start
      `StartRefused::Runtime` with `LockHeld`. `IdStartBusy` shows
      under Chats' app bar, with Try again; focus stays on Chats'
      title; the notice's text is announced once, politely; and the
      locked notice does not show. With no input for 60 s after
      that refusal, the hook records no further start (the bounded
      check of the run above).
      - In a conversation: the test opens a conversation after Chats
        opens, puts focus in its composer, and types "ab" before the
        answer and "c" after it. The notice shows under the
        conversation's app bar, focus stays in the composer, and the
        composer holds "abc". Then Tab to Try again and Enter, with
        the hook leaving the start unanswered: Try again shows
        disabled and focus stays on it. Answered as started: the
        notice goes and focus is on the composer. In a second run,
        the start is answered `LockHeld` again: the notice shows
        again and focus is on it.
      - Elsewhere: the test opens Settings before the answer.
        Settings shows no notice, and Back to Chats shows
        `IdStartBusy` there.
      - Pressed by a key, on Chats: Tab to Try again and Enter, with
        the hook leaving this start unanswered. Try again shows
        disabled; a second Enter, a Space and a screen reader's
        activate start nothing (the hook records one start); focus
        stays on Try again; no `ConnectStarting` shows. Answered as
        started: the notice goes, and focus is on the list heading.
        In a second run, answered `LockHeld` again: the notice
        shows again, Try again is enabled, and focus is on the
        notice.
      - Not held: with the start of a keyed Try again left
        unanswered, the test scrolls the list and opens a
        conversation, putting focus in its composer. Answered
        `LockHeld` again: the notice shows again under the
        conversation's app bar, and focus stays in the composer.
        In a second run, answered as started: the notice goes and
        focus stays in the composer.
      - Pressed by a tap, on Chats, with focus on Chats' title: Try
        again shows disabled and focus stays on the title. Answered
        as started: the notice goes and focus stays on the title.
      - The prompt: on a `user-presence` profile, the hook asks for
        the prompt of Try again's start and the test cancels it:
        the locked notice shows in `IdStartBusy`'s place, with focus
        on it.
      - A return: each run below is made with Stay reachable on and
        again in foreground-only mode, the mode a new profile starts
        in. With focus on Chats' title, the test sends the app to the
        background and back. The hook records one start at the
        return, the notice stays with Try again disabled until it
        answers, and focus stays on the title. Answered `LockHeld`:
        the notice stays, focus stays on the title, and nothing is
        announced. In a second run, answered as started: the notice
        goes and focus stays on the title. In a third run, with focus
        on Try again before the return: answered as started, the
        notice goes and focus is on the list heading. In a fourth
        run, on a `user-presence` profile, the return's start asks
        for the prompt, which rises with no input, and the test
        cancels it: the locked notice shows in `IdStartBusy`'s
        place, with focus on it. In a fifth run, the test recreates
        the window as a change of the system language does: the hook
        records one start, answered as in the first run. A rotation
        instead records no start. In a sixth run, made with Stay
        reachable on only, Enter on Try again, with the hook leaving
        that start unanswered, then the return: the hook records that
        one start and none at the return, and when it is answered as
        started, the notice goes. Its recreation variant, in either
        mode, recreates the window while that start runs: the hook
        records that one start and none at the recreation, and the
        recreated window shows the notice with Try again disabled
        until it answers. Answered as started, the notice goes;
        answered `LockHeld`, the notice shows with Try again enabled.
        Focus is on Try again, disabled, after the recreation, as
        before it; answered as started, it moves to the list heading,
        and answered `LockHeld`, to `IdStartBusy` (add-a-peer §8 item
        1, that notice's Try again rows).
        Its foreground-only variant, as a host test: the test feeds
        the end of Try again's start at the press of Home, as the host suite
        feeds any start's end, and the hook records a second start at
        the return, with the notice shown and Try again disabled
        until that start answers, as in the first run. Two more host
        variants answer the start before Home (§6, Foreground-only
        Home and a start's answer): answered as started, on C4 after
        a seal, the next screen shows at the return, with focus as
        add-a-peer §5.1 gives it, `IdStartBusy` does not, and the
        hook records one start at the return, as at a new launch,
        with no `IdStartBusy` and no Try again shown; answered `LockHeld`,
        `IdStartBusy` shows at the return with focus on it, and the
        hook records a second start there, with Try again disabled
        until it answers and focus still on the notice. Whether Home
        ends the start on a phone (the binding let go in `onStop`,
        `MainActivity.kt`, which a host test does not run) is a device
        run on the Pixel 9a, in foreground-only mode: Try again's
        start is left unanswered by a debug-build hook, then Home and
        a return; the run records whether the start was ended at the press of Home, and that the hook records one start at most in flight.
        Ended: a second start at the return, as above. Not ended: no
        start at the return, as with Stay reachable on, and the
        finding goes to the note's §6, whose foreground-only sentence
        then no longer holds. The debug build's `holdStartMs` holds
        only the first start after a launch, never Try again's
        (`apps/human-android/src/start_hook.rs`), so no debug-build
        hook can leave Try again's start unanswered on the device
        today, and this run is blocked, not passed, until rust-ui-dev
        adds one. A second device run, on C4 after a seal, refuses
        only the first start with `LockHeld` (the debug build's
        `busyStarts` launch extra at 1, `DebugHooks.kt`), so Try
        again's start runs as it would without the hook; Home is
        pressed as soon as Try again is. The run records, from the
        app's log, which came first, the start's answer or the
        service's stop, and passes when the return shows what that
        order requires (§6, Foreground-only Home and a start's
        answer): after an answer of started, the next screen, no
        `IdStartBusy`, and one start at the return, as at a new
        launch; after the stop, `IdStartBusy` and a second start,
        with Try again disabled until it answers. This run needs no
        new hook.
      - Connect: with no address recorded, after R4's Continue (so
        Connect is owed and was deferred at the bound), Try again's
        start is answered as started and both reads answer empty
        within 10 s of that success, with nothing opened from Chats
        since the press: Connect opens over Chats, with focus on its
        heading. In a second run, with Stay reachable on, the test
        opens a conversation, goes back to Chats, then sends the app
        to the background and back; the start retried at the return
        is answered as started and both reads answer empty within
        10 s of that success, with nothing opened since the return:
        Connect opens over Chats. In a third run, the test opens a
        conversation after the return and before the reads answer:
        Connect does not open, and stays owed. In a fourth run, the
        test leaves a conversation open, with "ab" typed in its
        composer, sends the app to the background and back, and the
        retried start and the reads answer as in the second run:
        Connect does not open, the conversation stays with "ab" in
        its composer, and Connect stays owed. In a fifth run,
        `IdStartBusy` shows in a conversation and Try again is
        pressed there, with the start and the reads answered as in
        the first run: Connect does not open, and the conversation
        stays.
      These runs are host tests, except two of the return runs: the two device runs in foreground-only mode.
    - A newly opened R2 or C3 has focus on the keypad's first enabled
      key, and Enter does nothing. After typing "ab",
      Enter does nothing and Space does nothing, and after each the
      next Enter still does nothing. After "car", Enter takes the
      "car" chip, and Space then does nothing. While typing, the focus
      ring shows on the focused control.
    - After a Tab to a key, and after an arrow to a key, Enter and
      Space each press that key, and a second Enter presses it again.
    - On a newly opened R2, a tap on t leaves focus on q, the key that
      held it, now disabled, and Enter and Space then do nothing.
      After taps on c, a and r, Enter takes the "car" chip. After a
      Tab to a key and a tap on another key, Enter does not press the
      focused key.
      After taps on c, a, r and g, a tap on the delete key, then
      Enter, takes the "car" chip; a tap on the clear key, then
      Enter, does nothing. After taps on c, a and r and a tap on the
      letters display, Enter takes the "car" chip. Every check in this
      bullet also holds when the last tap and the Enter arrive in one
      burst.
    - After a chip fills a position, whether by a tap or by Enter on
      the focused chip, focus is on the first enabled key and Enter does
      nothing, except as the next two bullets say.
    - On C3, a wrong word taken by Enter puts focus on
      `IdCheckWrong`, in view. A hardware letter then enters a letter,
      and Enter and Space do nothing. Tab from it reaches `TryAgain`,
      then `IdShowAgain`. Try again pressed by Enter removes the error
      and puts focus on the first enabled key, and the next Enter does
      nothing. After a wrong word, typing "c" and then pressing Try
      again leaves "c" in the letters display. After a wrong word, a right word puts
      focus on the first enabled key.
    - On R2, with positions 1 to 23 filled and 24 current, a chip
      taken by Enter fills 24 and puts focus on `IdCheckWords`, now
      enabled, and the next Enter runs the check. While a host test
      leaves that check unanswered, a typed "a", a tap on position
      5, Escape and another Enter change no word and open nothing,
      and after each of them the screen is still navigating;
      focus stays on `IdCheckWords`, which shows disabled; and the
      back arrow is inactive. In portrait, a Shift+Tab then moves
      focus to the clear key and a Tab brings it back to
      `IdCheckWords`; Up moves focus from `IdCheckWords` to a key in
      the keypad's last row, and Down brings it back. A Shift+Tab
      then puts focus on the clear key again, and two more Shift+Tabs
      move it to the delete key and then to a letter key. With focus
      there, Enter, Space and a screen reader's activate each leave the
      letters display empty and change no word, although a press on an
      enabled letter key adds a letter when the words are not held. After each step from the first Shift+Tab on, the screen is navigating. When the test answers the check as a failed checksum,
      the banner shows, with focus on it, and all 24 words are as
      they were. In a second run, the test answers the check as a valid phrase, and R3 opens with
      focus on `IdR3Question`. In a third run, the check is started by
      a screen reader's activate on `IdCheckWords`: focus stays on
      `IdCheckWords` while it runs. In a run after K1, a phrase from
      which the app derives the PeerId that the record names opens R4 with focus on its title, "PeerId restored". In the second run, the test answers the restore as a success. On R3, Tab moves focus from `IdR3Question` to `CopyPeerId` (§5), and a second Tab moves it to `IdR3Yes`. An Enter there opens R4 with focus on its title. The next Enter presses nothing, and the service does not start. On R4, Tab then moves focus to `CopyPeerId`, and a second Tab moves it to Continue. In a fourth run, the steps of the second run are repeated as far as the Enter on `IdR3Yes`, and the test leaves the restore unanswered. Meanwhile, Back, a second Enter on `IdR3Yes`, a Space on `IdR3Yes` and a screen reader's activate on `IdR3Yes` start no second restore and leave R3 open, with focus on `IdR3Yes`; `IdR3Yes` and `IdR3No` show disabled; and the back arrow is inactive. A Tab moves focus to `IdR3No`, and Enter there leaves R3 open, with no word changed. When the test then answers the restore as a success, R4 opens with focus on its title, although focus was on `IdR3No`. In a fifth run, from R3 as in the second run, the test answers the restore with a refusal that wrote nothing (`IdStorageFailed`). The notice shows, with focus on it. Its place, under the app bar and above the question, is decided by the view, and item 12 verifies it on the device; this run does not. R3 still shows the same PeerId, Tab moves focus to `TryAgain`, and a second Tab moves it to `IdR3Question`. A Shift+Tab returns focus to `TryAgain`, and Enter there starts the restore again, and the test leaves that restore unanswered. While it runs, `IdR3Yes`, `IdR3No` and `TryAgain` show disabled; focus stays on `TryAgain`; a second Enter, a Space and a screen reader's activate on `TryAgain` start no second restore; and Enter on `CopyPeerId`, reached by Tab, puts the PeerId's canonical string on the clipboard. In a sixth run, from R3 as in the second run, the test answers the restore as authentication needed and then the prompt as cancelled. R3 stays as it was, with no notice, `IdR3Yes` and `IdR3No` enabled again and focus on `IdR3Yes`, and the flow asks for no further restore until `IdR3Yes` is pressed again. These steps run in a host test, not on the
      phone: no device hook holds the check, and in a host test the
      flow's check is an effect that the test answers itself, as the
      identity flow's tests in `crates/human/ui-model` do. With
      position 3
      empty and 24 current, a chip taken by Enter fills 24 and makes
      3 current, with focus on the first enabled key.
    - Four more host-test runs check the running-check rules of §8
      that the runs above do not reach. Each starts with all 24
      positions filled and focus on `IdCheckWords`, and Enter there
      starts the check unless the run says otherwise. In the first of the four, a Shift+Tab moves focus from
      `IdCheckWords` to the clear key, and a tap on `IdCheckWords`
      starts the check: while the test leaves it unanswered, focus
      stays on the clear key. In the second, after K1, the test
      answers the check with a refusal that wrote nothing
      (`IdStorageFailed`), and the notice shows with focus on it. A
      Tab moves focus to its `TryAgain`, and Enter there starts the
      check again, which the test leaves unanswered. Meanwhile a
      second Enter, a Space and a screen reader's activate on
      `TryAgain` start no second check, a typed "a" changes no word,
      focus stays on `TryAgain`, `IdCheckWords` shows disabled and
      the back arrow is inactive. In the third, after K1, the test
      answers the check as authentication needed and then the prompt
      as cancelled. R2 stays as it was, with every word, no notice,
      `IdCheckWords` enabled again and focus on it, and no further
      check starts until `IdCheckWords` is pressed again. In the
      fourth, at a font scale over 130 %, the check is started by Enter
      on `IdCheckWords` and left unanswered. Then a tap on "show all
      my words" (`IdReviewWords`), and an Enter on it once Shift+Tab
      has moved focus there, open no list.
    - With position 5 filled, a tap on it puts focus on the first
      enabled key (the delete key when none is enabled), and Enter
      then takes the whole-word chip, so position 5 keeps its word.
    - With focus on a chip after a Tab, a tap on a letter key that removes
      that chip from the row moves focus to the first enabled key.
    - Backspace deletes the last letter.
    - With a screen reader on a newly opened R2: "a" typed on the
      hardware keyboard, focus moved to the p key and p activated,
      the letters show "ap", and the next Enter presses p again
      ("app"). In typing mode, that Enter would do nothing, since many
      words begin "ap". On a device this check needs the platform's
      accessibility tree, and it is blocked, not passed, on both
      clients: on Android until the build exposes one to TalkBack (the
      plan's TalkBack carry to the release gate), and on the desktop
      until the desktop client shows the identity screens, which it
      does not today. Until then, the check is a host test on Slint's
      testing backend that invokes the p key's accessible default
      action, the way `crates/human/ui-slint/tests/cards.rs` drives
      its controls. That
      test is rust-ui-dev's to write, and it is a dependency of this
      check.
    - At 200 %, `IdReviewWords` opens the full list with focus on the
      current position's row. Back, Escape and the app bar arrow each
      close it with nothing changed, no discard dialog and focus on
      `IdReviewWords`. Tab from the app bar arrow visits rows 1 to 24
      in order. With position 20 current, it opens with row 20 in
      view. On the list, Enter and Space each press the focused row,
      and a letter or Backspace changes nothing.
    - Choosing filled row 5 by Enter closes it with position 5 being
      edited and focus as §8 says; a second Enter at once takes the
      whole-word chip (typing mode), so position 5 keeps its word and
      the entry moves on.
    - A rotation, Home and back, and a font change from 200 % to
      150 % each keep it open. Setting the font to 130 % closes it,
      with the grid shown and focus on the current position.
    - After "ab", an arrow from a key beside a disabled key lands on
      the disabled key; Down from the first keypad row lands on the
      key below; in landscape, an arrow toward the end side from the
      start side reaches the end side's nearest control; Shift+Tab
      visits §8's order in reverse.
    - Escape, like Back, opens the dialog on R2 and C3 with a word
      entered, and with nothing entered leaves with focus as for Back
      (below).
    - On R2 and C3 with a word entered, Back opens the dialog. Back
      again, Escape, a tap outside and `IdContinueEntering` each close
      it with every word kept, focus on the control focused before the
      first Back, and, after typing "ab", Enter still doing nothing.
      After a Tab to a key, Back, then `IdContinueEntering`, Enter
      presses that key.
      Only `IdDiscard` leaves, with the words gone.
    - Back from C3, with `IdDiscard` or with nothing entered, puts
      focus on `IdPeerIdWritten`; from R2 opened by R1, on
      `IdEnterWords`.
    - R raised by a refused start (§8) has focus on its heading.
      This is checked by the debug build's focus trace
      (`ui-trace focus=`). The trace must name the heading's own
      focus key, which the build adds for each of R, K and K1:
      `r-heading`, `k-heading` and `k1-heading` (rust-ui-dev-01,
      2026-11-10). Any other value fails,
      `focus=none` and `focus=?` included. One exception: a launch
      makes a new window, whose focus is empty until the heading takes
      focus. So at a launch, a `focus=none` logged in the same
      millisecond, just before the trace names the heading's focus key,
      is that empty focus and does not fail. The
      trace reads the focus property, not frames (rust-ui-dev-01's
      REPLY 01a1ce5b). Run it three times, on a profile whose
      `key_unlock_policy` is `background-compatible`, so that no
      start raises an unlock prompt. Each
      of the first two runs fires the debug hook `REFUSE_STORE` and
      then `RESTART` (`apps/human-android/android/README.md`). The first
      is over Chats, with focus on a conversation row. The second is
      over an open conversation, with the composer focused and the
      keyboard up; there the keyboard closes too. The third run is at
      a launch. The hook's refusal lasts only while the process lives,
      and a launch runs a start only after the Service has ended. So
      use a profile that is not Stay reachable: fire `REFUSE_STORE`
      on Chats and note the pid that
      `adb shell pidof org.interweave.human` shows. Then press Back,
      so that the Activity finishes (Home would leave it alive, and
      a return to it is not a launch), and confirm that the same
      command shows the same pid (the process stays). Then open
      InterWeave from the launcher. In each run the first Tab
      reaches Try again. Then, in the run over Chats, with focus on
      Try again, fire `REFUSE_STORE` and then `RESTART` again. The start
      is refused again and R stays, and the trace still shows Try again's
      focus key (`focus=identity-primary`), not `focus=r-heading`
      (§8: R was not raised). Then fire `REFUSE_STORE` once more and
      press Enter on Try again, so that the start that Try again makes
      reaches the store. That start is refused again and R stays, and
      focus stays on Try again, as it did after the restart. Then
      press Shift+Tab to put focus back on R's heading. Where adb cannot
      send Shift+Tab, as on Android 11, press Tab instead, again and again,
      until focus comes back to R's heading.
      The trace shows `focus=r-heading`. Then fire `RESTART` again. The hook refuses
      only once, so this start runs, and R leaves the screen while
      focus is on its heading. The trace then names Chats' list
      heading (`focus=conversations-heading`), as unlock note F15
      says for focus inside R. K1 is checked at the launch in item 9,
      with item 9's hook. K1 raised over a screen already showing, and
      K, are checked as R's first two runs are, over Chats and over an
      open conversation: with the debug hook `REFUSE_IDENTITY` (K1) or
      `REFUSE_STORE_KEY` (K) in place of `REFUSE_STORE`, followed by
      `RESTART`. The trace must name `focus=k1-heading` or
      `focus=k-heading`, and the first Tab reaches K1's primary action,
      or on K Delete stored data…. R's steps from Try again on do not
      apply to either screen. Firing the same hook and `RESTART` again
      while K1 shows leaves focus where it was, since K1 is not raised
      again.
    - A K1 that was raised over a running window stays until its
      restore ends (#611; rust-ui-dev-01's REQUEST 01a1c551, which
      carries these checks from architect-cto's review). Use a profile
      whose recovery phrase the tester has, whose `key_unlock_policy`
      is `background-compatible` (as for R above), and that is not Stay
      reachable.
      First note whether the Peers page lists nothing and whether the
      Trust page lists nothing: step 3 uses both notes. Then, on
      Chats, fire `REFUSE_IDENTITY` and then `RESTART`, so that K1
      rises.
      1. Before anything on K1 is touched, `adb shell dumpsys window`
         shows `FLAG_SECURE` (`SECURE`) in the flags of InterWeave's
         window. Read it again on R2, opened by K1's primary, before a
         word is entered; then press Back, which with nothing entered
         returns to K1 (§7). Read it a third time on K1 at the launch
         in step 2. If the flag is missing at any of the three
         readings, the step fails (§2). Report it to rust-ui-dev-01 as
         a P1: on this path nothing ensures that the flag is set
         before K1's first frame, so only the reading can show it.
      2. Note the pid that `adb shell pidof org.interweave.human`
         shows. On K1, press Back, so that the Activity finishes before
         the restore, and confirm that the same command shows the same
         pid. Then open InterWeave from the launcher. K1 opens again,
         not Chats, with the same PeerId. If Chats opens, or K1 shows
         a different PeerId, the step fails. If the pid changed before
         the launch, the run is void, not failed: K1 is kept only while
         the process lives, and the hook left the record able to open,
         so a new process opens Chats. Run it again.
      3. Clear the log (`adb logcat -c`). Then, from that K1, enter
         the phrase on R2. The restore matches and
         goes to R4. After R4's continue, the client runs. R4 stays on
         screen, with Continue disabled, while the start reads the peer
         list and the trust set. If the phone knows no address and no
         peer, the first page after R4 is Connect to someone:
         onboarding's third step (`design/2026-10-22-add-a-peer.md`
         §5.1). Otherwise, the first page after R4 is Chats, and Chats
         stays. Chats, then Connect to someone over it, fails the
         step. In
         both cases, `adb shell dumpsys activity services
         org.interweave.human` lists the Service, and `logcat -s
         interweave` shows the start succeeding. No second K1 rises,
         then or at the next launch. Which case applies is read from
         the trace, not from the screen. The phone's screen and keys
         are not touched until the trace has been read, so that every
         line after R4's continue is the client's own and none comes
         from a page the tester opened. After R4's continue, the
         trace writes `connect=owed` while the start reads the peer
         list and the trust set. While R4 is on screen, the trace
         writes no `page=` line. The trace writes `identity=closed`
         when R4 leaves the screen, and then a `page=` line only if the
         page under R4 differs from the page of the last `page=` line
         written. Step 3 runs in the app window that step 2's launch
         opened. No `page=` line was written in that window before K1
         covered it, so when Chats shows after R4, the trace writes
         `page=conversations` after `identity=closed`. When K1 is
         raised over an app window that had shown Chats, the trace
         writes no new `page=` line. In both cases the page is Chats,
         so the tester reads Chats from the absence of a
         `page=connect` line after `identity=closed`. Wait, for at most 30 seconds, until the trace
         has written `identity=closed` and the last `connect=` line is
         no longer `connect=owed` (on the A40 this took under a
         second). Then read the last `connect=` line, and whether a
         `page=connect` line follows `identity=closed`.
         `connect=open`, with `page=connect` after `identity=closed`,
         is Connect opening. `connect=settled` with no `page=connect`
         line after `identity=closed` is Chats staying, and so is
         `connect=not-owed` with no such line. A start writes
         `connect=not-owed` when the restore did not write the marker
         that says Connect is owed. `connect=deferred` with no such
         line means that the start could not read the peer list or
         the trust set within the 10 s bound, so
         Chats opens without Connect (`design/2026-10-22-add-a-peer.md`
         §5.1); then only the Connect
         part of the step is void, not passed, and the step is run
         again. The step fails if any of these holds: the Service is
         not listed; logcat does not show the start succeeding; a
         second K1 rises, then or at the next launch; after 30
         seconds the trace has not written `identity=closed`, or the
         last `connect=` line is still `connect=owed`; the last
         `connect=` line written before `identity=closed` is
         `connect=owed` and a `connect=open` follows it (R4 left the
         screen before the start decided: Chats, then Connect over it);
         the result is none of the four results above (`connect=open` with
         `page=connect` after `identity=closed`, and
         `connect=settled`, `connect=not-owed` or `connect=deferred`
         with no `page=connect` line after it); the result is Connect opening
         on a phone whose Peers page or Trust page,
         or both, listed something before the hooks; or the result is
         Chats staying on a phone whose Peers page and Trust page
         both listed nothing before the hooks. There is one
         exception to that last case. If, on such a phone, the result
         is `connect=not-owed` with no `page=connect` line after
         `identity=closed`, and
         `logcat -s interweave` shows a line that contains
         `writing the connect-owed marker:` and that was logged between
         the clear of the log at the start of this step and R4's
         continue, the marker write failed. A line with that text
         from any other time does not count, because such a line
         comes from another creation or restore. A line that contains
         `clearing the connect-owed marker:` does not count either:
         the app logs that text when it fails to remove the marker,
         not when a write fails. The
         build accepts a failed marker write and opens Chats without
         Connect
         (`apps/human-android/src/onboard.rs`, `connect_owed`). The
         Connect part of the step is then void, not passed, as for
         `connect=deferred`.
      Each result is reported with how it was seen: by the focus
      trace, by the flag read through `dumpsys`, or rendered. K1 and
      R2 are secured (step 1), so a screenshot of them is black, as
      item 2 says of R2: what
      they show is read on the phone. A `uiautomator dump` sees no
      node of the app on Android until the build exposes an
      accessibility tree (§8), and a screenshot is evidence only for
      Chats or Connect to someone after R4.
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
18. Below Android 13, "Copy PeerId" is confirmed as §5 says (The
    copy's confirmation). In a host test with the platform version set
    below 13, on each of C2's PeerId page, S2's PeerId page, C4, R3
    and R4: after Enter on `CopyPeerId`, `PeerIdCopied` shows with no
    action, focus stays on `CopyPeerId`, Tab and Shift+Tab move focus
    to the same controls as on the same screen with no snackbar and
    never to the snackbar, and one polite announcement of its text is
    made. A second Enter within 6 s keeps it shown 6 s from that
    Enter and makes a second announcement. With no input it closes 6 s
    after it appeared; with the accessibility timeout set to 30 s,
    30 s after; and with that timeout set to 5 minutes, 2 minutes
    after. On R3, after Enter on `IdR3Yes` 3 s after the snackbar
    appeared, with the restore left unanswered, the snackbar stays
    shown until its time ends, 6 s after it appeared (3 s after the
    Enter), and closes then: it is still shown 2 s after the Enter and
    gone 4 s after it. On C4, `ConnectStarting` shows 1 s after the
    person presses Continue (no system prompt is raised in these
    runs), and three runs, each with the seal left
    unanswered, check the snackbar beside `ConnectStarting`. In the first, the test presses Enter on
    Continue 5.5 s after the snackbar appeared: the snackbar
    closes when its time ends, 6 s after it appeared and before
    `ConnectStarting` shows; it is still shown 0.2 s after the Enter
    and gone 0.7 s after it. In the second, the test presses Enter on
    Continue 2 s after the snackbar appeared: the snackbar closes when
    `ConnectStarting` shows, 1 s after the Enter, and only one polite
    announcement, `ConnectStarting`'s, is made then. In the third,
    nothing is copied before Continue, and the test copies 1.5 s after
    the Enter on Continue, while `ConnectStarting` shows: the snackbar
    shows, `ConnectStarting` stays shown, and one polite announcement is
    made, the snackbar's. The same three runs are made on R4, with
    Connect owed. In those runs, R4's Continue is pressed in place of
    C4's, and the service start's reads are left unanswered in place
    of the seal (the hold of `design/2026-10-22-add-a-peer.md` §5.1).
    The times and the results are the same, so a build that restarts the snackbar's time
    at R4's Continue, or keeps the snackbar once
    `ConnectStarting` shows, fails. On R3, by a test hook with `IdRestoreOther`
    shown, the second and third runs are made again with
    `IdRestoreOther`'s Continue in place of C4's and with the service
    start left unanswered: `ConnectStarting` shows inside the notice, under
    that Continue, with the same times and the same results. On C4, by a test
    hook with `IdStorageFailed` shown, a copy shows the snackbar and
    the notice stays shown; after Enter on Try again 5.5 s after the
    snackbar appeared, with the seal left unanswered, the snackbar is still shown 0.2 s after the
    Enter and gone 0.7 s after it, as for Continue. In a second run
    with `IdStorageFailed` shown, the test copies, presses Enter on
    Try again 1 s later, and answers that seal with `IdStorageFailed`
    again 1 s after the Enter: the snackbar closes when that result
    shows, 2 s after it appeared, and focus goes to the notice
    (§5, §8). A rotation 3 s after the snackbar appeared (in the host
    test, the window's size swapped from portrait to landscape) keeps
    it shown, 12 dp above the actions, with no second announcement,
    and it closes 6 s after it appeared. Before a copy, the test
    records where the scrolling area ends; while the snackbar shows,
    the area ends 12 dp above it; after it closes, the area ends where
    the test recorded, on each of the screens above. On C4, when a
    seal is answered `AlreadyProvisioned` while
    the snackbar shows, the snackbar closes when `IdAlreadyCreated`
    shows; on R3, when a restore is refused with `IdStorageFailed`,
    the snackbar closes when that notice shows. On C4, under
    `user-presence`, when the seal is refused as authentication needed
    while the snackbar shows, the snackbar closes when the system
    prompt shows. On C4, after a copy, when the seal is answered
    `AlreadyProvisioned`, the clipboard no longer holds the PeerId's
    string. In a second run, the copy is made on C2's PeerId page and
    none on C4, and the clipboard no longer holds the string either.
    In a third run, the test copies another text to the clipboard
    after the copy and before the answer, and that text is still there
    after the answer. These three runs are made with the platform
    version below 13 and again at 13 or later, and in every run the
    test's clipboard records no read of its content by the app. In a
    fourth run, the test takes C4's window's focus away after the
    copy, with the app still in the foreground (as split-screen or the
    notification shade does), and answers the seal
    `AlreadyProvisioned`: the clipboard still holds the string until
    C4's window has the focus again; then the clipboard no longer
    holds it. A fifth run is the fourth with the device reported
    locked when C4's window has the focus again: the clipboard still
    holds the string until the device is reported unlocked; then the
    clipboard no longer holds it. Neither run checks whether the app asks for the clip's description while the removal waits: §6 asks only
    that the removal wait, and the test's clipboard, like the
    platform, gives no description then. The host
    test reports the lock state through a seam that rust-ui-dev adds for
    it (fed on the phone by `KeyguardManager.isDeviceLocked` and the
    unlock broadcast `ACTION_USER_PRESENT`, for example); until that
    seam exists, the fifth run is blocked, not passed. On the
    Pixel 9a (Android 13 or later), a person copies the PeerId on C4, a debug-build hook
    answers the seal `AlreadyProvisioned`, and the person verifies
    that pasting into another app by the platform's paste (a long
    press in a text field, then Paste, not the keyboard's clipboard
    history, §12) no longer gives the PeerId, and that
    the platform showed no notice that InterWeave read the clipboard.
    rust-ui-dev adds that hook, as it adds acceptance 12's hooks;
    until it exists, this Pixel 9a check is blocked, not passed. On C4, whatever the policy, when the seal is
    answered `ProfileLocked` while the snackbar shows, the snackbar
    closes when the progress state (`IdGoingOffline`) shows. Leaving
    the screen closes it, and the next screen shows none. The app going to the background
    closes it too, and when the app returns, the screen shows none.
    With the platform version 13 or later, the same Enter shows no
    snackbar and makes no announcement. On the Samsung A40 (Android
    11, the project's second test phone,
    `packaging/android/evidence/2026-11-05-samsung-a40-update-and-downgrade.md`),
    at 200 % font in portrait and in landscape (set with `adb -s <the
    A40's address> shell settings put system font_scale 2.0`, and set
    back to the phone's own scale after the run, since whether the
    A40's own settings offer 200 % is not known; the command runs
    under the `samsung-a40` lease, on the running login's own adb
    server, since `.agent-fabric/bin/phone-run` holds only the Pixel
    9a's lease; the run records the scale the A40's settings
    offer), in the dark and the
    light scheme, on C2's PeerId page, C4 and R3, a person looks at
    the screen and verifies that the snackbar has the look spec's
    snackbar colours, is at least 48 dp tall and 12 dp from the
    screen's sides, sits 12 dp above the actions, and covers none of
    them, nor the PeerId, and that `CopyPeerId` is wholly in view,
    also when the person scrolled down to `CopyPeerId` before the
    copy. In landscape at 200 %, the hypothesis is that the scrolling
    area left above the snackbar still holds `CopyPeerId` wholly. It
    is falsified if, on the A40, `CopyPeerId` cannot be wholly in view
    while the snackbar shows. That is a finding on this note, not on
    the build, and it comes back to ux-designer with the screen's
    figures. The rest of this item runs in the host test, except the
    Pixel 9a's clipboard check above. The other items
    of this list run on the Pixel 9a, which shows no snackbar.

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

## 11c. After #715 (Q11 answered)

- **Q11. A service start refused with the profile in use.**
  Answered by architect-cto, 2026-11-18 (REPLY 01a1efb4-1842). The
  service start takes the profile lock itself with a wait of zero
  (`crates/transport/embedded/src/lib.rs`, `LOCK_WAIT`), and a held
  lock refuses the start as the runtime's `LockHeld`, which reaches
  the client as a refused start (`StartRefused::Runtime`). That is not
  custody's `ProfileLocked`, which the seal, the restore and the reseal
  answer. Once the flow has released its lock, only a service host in
  the same process that has not finished stopping, or a second start made while the
  first is running, can hold it (`crates/transport/embedded/tests/host.rs`,
  `a_second_start_is_refused_while_the_first_serves`); the flow still
  holding it at Continue would be a defect. So: no progress state
  (§5); the person is told that the start was refused, with Try again, and
  never that the identity was not created or restored; no retry by the
  app on its own (§6, the row for a service start refused with the
  profile in use).
  architect-cto suggested §8's treatment of a refused start, a screen
  raised in place; this note instead shows a §6 notice on the
  screen whose Continue or Try again made the start, since no screen of §8 is for this refusal
  (K1, R and K are the record's and the store's) and §6 shows every
  other refusal of these screens there. Which string the client
  showed for this refusal was rust-ui-dev's to say when it built it
  (§11d, Q12).

## 11d. After #720 (Q12 and Q15 answered, Q13 and Q14 open)

- **Q12. A start refused with the profile in use after the bound.**
  Answered by ux-designer-01, 2026-11-18, in §6 (A start refused with
  the profile in use after the bound). The start takes its lock only
  after the key is unwrapped and after any reseal that the start
  retries (§5; `crates/human/android-platform/src/identity.rs`), so,
  on a slow Keystore, `LockHeld` can come after add-a-peer §5.1's
  10 s bound, when Chats or a conversation already shows. Then
  `IdStartBusy` shows there as a session notice, with Try again, and
  takes no focus. rust-ui-dev-01 answered where the build can show it
  (REPLY 01a1f275-b775, to QUESTIONs 01a1f00b-9926 and 01a1f029-c24c,
  read on main at 24b088224). When Q12 was answered, the build did
  not name this refusal: every refused runtime start other than
  network denied reached the view as one answer, and Chats or a
  conversation then showed the notice that no service runs
  (`NoDaemon`), which has no action and says the app will connect by
  itself. That was untrue here: nothing retries the start while the
  person stays in the app, and only a return or a recreation does
  (§6). rust-ui-dev-01 said the notice of §6 could be built as a
  session notice of its own, with an action that makes one service
  start by the path a granted Unlock uses, once `LockHeld` was told
  apart from the other refused runtime starts, with the notice's state
  kept by the process, not by the runtime that ends, so that the notice survives a
  foreground-only Home and shows at the return. #736 (merged
  b0bf4eb49) built it so: `IdStartBusy` before the bound on C4, R4,
  R3 and R2 after K1, and after the bound as a session notice on
  Chats or in a conversation (rust-ui-dev-01, REPLY 01a1fb77, host
  tests only). Its REPLY also said that in foreground-only mode a
  start can answer before the service stops at Home; §6 rules on it
  (Foreground-only Home and a start's answer).
- **Q13. Can the person deny the clipboard read app op on a target
  phone?** Open, ux-designer's to check on the devices. §6 and §12
  read the app op (`OP_READ_CLIPBOARD`) in the platform source.
  Whether the system software of the Pixel 9a or of the Samsung A40
  offers the person a way to deny it was not checked. If one does,
  §12's "the copy stays with no word to the person" is a case that a
  person can reach, not only a platform possibility, and comes back
  to this note. The check: the app's settings page on each phone,
  and `adb shell appops get org.interweave.human READ_CLIPBOARD`
  (through `phone-run` on the Pixel 9a).
- **Q14. The unlock signal on a device.** Open, rust-ui-dev's to
  choose and ux-designer's to check on the devices. Acceptance 18's
  fifth run is a host test through a lock-state seam. On a phone,
  whether the signal that the build chooses (`ACTION_USER_PRESENT`,
  for example) comes at every unlock was not checked: with no secure
  lock screen, and with a fingerprint or face unlock. If it misses
  one, the removal waits until the next change of the window's
  focus, and acceptance 18 gains a device run.

- **Q15. A way on from C4 when the read-back keeps failing.**
  Answered by architect-cto, REPLY 01a1f34b-95bb: a service start from
  C4 under `IdStorageUnknown` is allowed, and a recovery-required
  answer from it is what a launch does. The conditions are rules: it
  is offered only after a read-back has failed, beside Try again; the
  flow ends and lets go of its lock first and nothing seals after; the
  start must open the record to the PeerId C4 shows, and leads to K1 when the record
  does not open, and fails closed when it opens to another PeerId; a
  platform error is "try again" and a held lock `IdStartBusy`; the words
  never say a PeerId was created, and say that the phrase restores it.
  §6 designs it (A start from C4 after a failed read-back). The
  question, as asked: On C4 the seal's write may have committed the
  identity, Try again only reads back, and C4 has no Back (§6, A
  read-back that keeps failing). Only the end of the process led on. Does a service start from C4 under
  `IdStorageUnknown`, whose unlock would answer whether the record
  opens and, if it does not, lead to K1 (§3.4) as a launch does, fit
  the custody rules?

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
- **Removing a copied PeerId from the clipboard anywhere but C4
  under `IdAlreadyCreated`.** On R3, a restore refused with
  `IdRestoreOther` leaves the PeerId the words make on the clipboard.
  That PeerId belongs to an identity a phone held, and the paper names
  it; it is not a PeerId no phone will hold. Whether the person meant
  to keep it is theirs.
- **Removing the PeerId from a keyboard's own clipboard history**
  (§6). Gboard and Samsung Keyboard keep recent copies in a history of
  their own, which no app can reach. The note promises removal from the
  system clipboard only, and the Pixel 9a check of acceptance 18
  pastes with the platform's paste.
- **Removing the copy when the process ends** before the removal can
  act (§6). When the seal is answered `AlreadyProvisioned`
  while C4's window does not have the focus or the device is locked,
  the removal waits until C4's window has the focus with the device
  unlocked, on every version the app supports (§6). After process death the
  flow restarts at its first screen with nothing half-done kept (§7).
  So `IdAlreadyCreated` does not show again, and the copy stays (§6). Removing a copy at a later launch would act on the
  clipboard at a moment when nothing on the screen tells the person
  why. The same holds when the clipboard read app op is not allowed
  to the app: the description never comes, the removal never acts,
  and the copy stays with no word to the person (§6).
- **Words saying that the copy was removed** (§6). `IdAlreadyCreated`
  already says that the words just seen were not used, and the
  removal takes only the app's own copy.
- **What R3 shows when a restore of other words follows a refused
  restore** (§6, A read-back that keeps failing). Custody refuses it
  as `RecordNamesOther`; the another-identity row's `IdRestoreOther`
  assumes a record that was there before the flow's write, and its
  Continue would open a record whose read-back kept failing. The case
  needs Back from `IdStorageUnknown` and a new phrase, so it is left
  to a later revision. Until then R3 shows `IdStorageUnknown` again
  and writes nothing (§6, the another-identity row).
- **Words for a read-back that keeps failing** (§6). On R3 and on R2
  after K1, `IdStorageUnknown` offers Try again only; the way on is Back and a new restore on R3
  and on R2 after K1, and no words say so. On C4 the start is the way
  on, and its words say it (§6, Q15), except after a start from it met
  another PeerId than C4's: then only the end of the process leads on,
  the next launch runs the record's identity, and no words say either.
  That case needs another writer between the flow's release of the
  lock and the start (§6), so it gets none. A read-back that keeps failing can come only after the two refusals that can come after the write, and no device has shown
  it yet; words for it are language-culture's, if a test shows people
  stuck there.
- **What the Settings screen shows besides the new item.**
