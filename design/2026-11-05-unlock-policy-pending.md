# The screen-lock setting not yet in force, and the restart diagnostic — Android

Status: proposal, 2026-11-05. Asked by rust-ui-dev (REQUEST 01a1abf3,
their job j213, my job j93), read against `origin/main` 8859f6366 and
their branch `develop-qzapp/rust-ui-dev-01/feat/j213-android-start-never-calls-custody-reseal`
at 76ac4a188. The locked-state revision (#573) was read against that
branch at 6464c3cf0. This note answers where a person sees that a changed
`key_unlock_policy` is not yet in force and why (ADR-0042 A 2026-11-05),
and puts `human-client-ui.md` §14's
`background_restart_requires_user_authentication` diagnostic on the
same surface, since both say what happens to InterWeave after a
restart. The note's two product questions are answered by architect-cto
(REPLY 01a1ac8d-ea64, §10). Q1: the diagnostic follows the policy in
force. Q2: no Try again. Both are recorded in ADR-0041 A 2026-11-05 on
#571, which is not yet merged. Its one question to rust-ui-dev is
answered too (Q3). §11, added for my job j96, specifies a restart that
the screen lock refuses while the app process keeps running. rust-ui-dev
traced that path in REPLY 01a1ada5, from their branch
`develop-qzapp/rust-ui-dev-01/feat/j216-577-follow-up-the-activity-s`
at cf55ad5a6. That path has not been run on a device. Revised
2026-11-13 (my job j126), from the Pixel review of §11: on GrapheneOS
a network grant ends the app's process, so §11 now treats the grant as
a restart in a running process (L1) only on a platform where the grant
leaves the process running (§11, "A network grant that ends the
process").

Owner of the design: ux-designer. The words are language-culture's: §7
names each string and what it must say. No new colour: the line is body
text on the surface and the notice is the existing `NoticeCard`
(`design/2026-10-16-android-look.md` §3). Built by rust-ui-dev. Android
only: the desktop has no unlock policy.

## 1. What is already decided

- ADR-0042 A 2026-11-05 and `android-key-custody.md` (Reseal and the
  policy byte): a reseal that fails for any reason other than
  `UserNotAuthenticated` never refuses the start. The record opens
  under its own policy, the start continues, and the reseal is tried again
  at every start. While the
  record's policy differs from the configured one, or the other
  policy's key remains (`OldKeyRemains`), the status model exposes the
  pending change by name, with the refusal's reason.
- rust-ui-dev's branch classifies the refusal as `ResealFailure`:
  `Keystore`, `Storage`, `OldKeyRemains`, `Other`.
- `human-client-ui.md` §14 and `android-key-custody.md`: with
  `availability_mode=stay-reachable` and
  `key_unlock_policy=user-presence`, the UI exposes
  `background_restart_requires_user_authentication` and never describes
  the configuration as reachable across a restart. Glossary:
  "After a restart, … until you unlock".
- Neither setting can be changed from inside the app today.
  `key_unlock_policy` is authored in `config.yaml`, and the Stay-reachable
  overlay is written only through `ServiceHost::set_availability`,
  which no control calls. So the person never sees this state just
  after pressing something. They see it when they look later, often
  long after someone else changed the file.
- The app's only settings surface is the Trust page (App options →
  Trust). It already shows this device's PeerId and, under the PeerId, "Show
  recovery phrase" (`design/2026-10-19-android-identity.md` §3.5). That
  is the identity's section, and the unlock policy guards that identity.

## 2. The person's goal

The person wants to know **whether InterWeave needs their screen lock
after the phone or the app restarts**, because that decides two things:
whether someone holding the phone after a restart can open the person's
identity, and whether InterWeave comes back online by itself.
When the answer differs from what the setting says, they want to know
that, what the answer is now, why, and whether they must do anything.
Usually they need to do nothing: the next start tries again. Freeing
storage is the one thing that can help.

Evidence: a heuristic reading, not a test with people. Nielsen's
visibility of system status (the state is invisible otherwise, and in
one direction (P-up) the protection is weaker than the person was
told). The match between system and real world (say what happens after
a restart, not "policy"). Aesthetic and minimalist design (a fault the
person cannot act on does not stay on Chats forever). Hypothesis H1,
with what would show it wrong, is in §4.

## 3. States

The view reads four facts (Q3). The status member (`UnlockStatus`)
carries the first three. The host gives the fourth, or, while no host
runs, the profile on disk does:
- **in force**: the policy of the record that is authenticated for this
  start. It is never a claim read from the file without opening it,
  except while locked (below), where ADR-0041 A 2026-11-05 takes the
  header's byte. After any start that ran it is known.
  - When the start's reseal wrote the new record, it is the configured
    policy: the record now on disk was sealed under it, under the
    profile's lock. That covers three outcomes:
    - the reseal succeeded;
    - it answered `OldKeyRemains`;
    - it failed with custody's `Storage(Unsynced)` and the record reads
      back under the configured policy (so it is classed
      `OldKeyRemains`). `Unsynced` arrives only after the new record has
      replaced the old one, with the old policy's key not retired
      (`custody::reseal`'s documentation). An `Unsynced` that does not
      read back that way is the exception below.
  - Otherwise (`Keystore`, `Storage` before the write, `Other`,
    `Held`), it is the policy byte of the record the start opened
    (ADR-0041 A 2026-11-05).
  - This is ADR-0041 A 2026-11-05 as architect-cto ruled it on #571
    (089b8554f): the policy byte of the record on disk when the start's
    custody step ends. That is the record the reseal wrote when it
    wrote one, `Storage(Unsynced)` included, and otherwise the record
    the start opened. The ruling does not say which record is on disk
    when the written one cannot be read back (the exception below); the
    note shows the opened policy there, as rust-ui-dev's code does.
- **configured**: the configured policy, or unknown when the profile
  could not be read at the reseal.
- the reseal's pending class, if any.
- the effective availability mode: the host's while it runs, and
  otherwise profile-config's `effective_availability_mode`, which
  reads it from disk with no host running (ADR-0041 A 2026-10-10: one
  effective mode, overlay else config, for everything derived from
  it). It is unknown only when the profile cannot be read, and then D's
  line is not shown, since D cannot be claimed.

**A change is pending whenever in force and configured differ**, whatever
the start's reseal answered, `Held` included (ADR-0042 A 2026-11-05 asks
for the pending change to be shown "while the record's policy differs").
A pending change with no class from the reseal is shown with reason
`Other`. When the configured policy is unknown, no comparison can be
made. With a reason class, the state is P-unknown. With none, it is N
or D, decided by the policy in force alone.

`Unsynced` is shown as K: the record is under the configured policy and
the other policy's key remains, which are K's facts. **The status member
is to report `Unsynced` as the class `OldKeyRemains`**, never as
`Storage`. rust-ui-dev accepted this in REPLY 01a1aca2-ea97, with a test
promised. It is on their j213 branch at 15632b734, not yet on `main`.
There, `Unsynced` is classed `OldKeyRemains` only when the record on
disk is under the configured policy, and the test
`an_unsynced_reseal_is_old_key_remains_and_a_failed_write_is_storage`
pins two cases: an unsynced write is `OldKeyRemains`, and an unwritable
directory is `Storage`. So the view needs no fifth fact, and a `Storage`
class means a failure before the write, with one exception. When an
`Unsynced` record cannot be read back, or names another policy, it
stays `Storage`; in force is then the opened policy, and the line shows
P. It is a precondition of §8
beside the member until it reaches `main`. One limit remains.
If a power cut loses the unsynced write, the next start opens the old
record again, and the line shows P for that start.

| State | Condition | Trust page line (§5) | Chats notice (§4) |
|---|---|---|---|
| N | nothing pending; not (stay-reachable and user-presence) | none | none |
| D | nothing pending; effective stay-reachable and user-presence in force | `UnlockRestartNeedsUnlock` (`UnlockRestartNeedsUnlockLocked` while locked, below) | none |
| P-up(r) | configured user-presence, in force background-compatible, reason r | `UnlockPendingNeedsLock` + reason r | `UnlockPendingNotice`, per §4 |
| P-down(r) | configured background-compatible, in force user-presence, reason r | `UnlockPendingNoLock` + reason r | none |
| P-unknown | a reason r pending and either the configured policy unknown (the profile could not be read), or in force = configured (the reseal failed otherwise than `OldKeyRemains` with the record already under the configured policy, as `Other` can; never `Unsynced`, which arrives as `OldKeyRemains`), so no direction can be named | `UnlockPendingUnknown` + reason r, and D's line under it when D's condition holds (below) | none |
| K | in force = configured; the other policy's key remains (`OldKeyRemains`, or `Unsynced`, above) | `UnlockOldKeyRemains`, and D's line under it when D's condition holds | none |

Reasons r: `Storage`, `Keystore` and `Other`. These are rust-ui-dev's
classes, and the person is told about each one differently:

- **Storage**: the phone could not save the change. Freeing space may
  help. Storage is the only class where the person can do something.
- **Keystore**: the phone's secure key storage did not accept the
  change. Nothing the person does in InterWeave helps. Saying so
  stops the person from looking for a fix that does not exist.
- **Other**: the change could not be made. No cause is offered.

`OldKeyRemains` is a state (K), not a reason r: the record is already
under the configured policy.

**D follows the policy in force, never the configured one** (ADR-0041
A 2026-11-05, Q1). In P-up the diagnostic is therefore false. In P-down
it is true, and the P-down line already says that a restart needs the
screen lock, so **in P-up and P-down D's line is not shown**: the P
line stands in for it. In P-unknown, D's line shows under the P-unknown
line when D's condition holds (the policy in force is always known),
because the P-unknown line does not say what a restart does.

**While InterWeave is locked** (the start stopped at
`UserNotAuthenticated`, and the locked notice shows), this section
applies unchanged:
- In force is the policy byte the record's header carries (ADR-0041
  A 2026-11-05). Either the start opened nothing, or it opened the
  record and the reseal was refused; then the header byte is the
  opened policy.
- The availability mode is read from disk, as above.
- So D's line shows when its condition holds, as `human-client-ui.md`
  §14 requires for `stay-reachable` with `user-presence` after a
  restart. While locked it is `UnlockRestartNeedsUnlockLocked` (§7), since
  "while InterWeave keeps running" is not true then. The locked notice
  says InterWeave is offline now; D says this will happen after every
  restart. The two do not repeat each other.
- P and K lines show as the member holds them, since a locked start
  changes no record.
- P-up's Chats notice is below the locked notice in priority (§4), so
  it does not show while locked. A locked P-up arises when a
  background-compatible record's reseal toward user-presence is
  refused `UserNotAuthenticated`.

Shown and cleared: the state is read again whenever the status member
changes. A start whose reseal succeeds results in N or D. Nothing is
shown "for a while" after a state clears: its line is removed and, on
the Trust page only, the change is announced (§6).

## 4. The Chats notice — P-up only

Only P-up reaches Chats. It is the one direction where the phone is
**less protected than the setting says**: after a restart, someone
holding the phone can open InterWeave without the screen lock. A
person who looks only at Chats would otherwise never learn of it.
P-down costs only an Unlock prompt, which the person already sees
and answers, and its line on the Trust page explains why. K and D are not
faults.

- **Place and priority**: the session notice's slot, **only while the
  conversation list is on screen**. On a narrow window that means
  Chats, never an open conversation. On a wide one, the slot where it
  already shows beside the list. It is not about a conversation, so the
  conversation pane on a phone never carries it. On Chats it sits where
  session notices already sit: under the app bar, after the no-reach
  notice when that shows, and before the list. It has the **lowest priority** of all
  session notices. Every existing `SessionNotice` (no daemon,
  reconnecting, storage, refused, network denied, locked) replaces it
  while that notice is shown, and the P-up notice returns when
  they clear.
- **Weight**: informational, the same card as the others, one action:
  `UnlockPendingDetails`. The action opens the Trust page with focus on the
  line (§6). It has no Try again: none exists, and adding one was Q2,
  answered no (§9).
- **How long**: from the start whose reseal resulted in P-up, **until the
  person has seen the Trust page's line** (by Details, or by opening
  Trust from App options while P-up holds), or until P-up clears.
  "Seen" lasts for **the start that set it**. It is kept in memory, never
  stored, so an Activity recreated or relaunched while the service keeps
  running still counts as seen, and a new start clears it. If P-up still holds, it shows
  again after the next start, because that start retried and failed
  again. This puts it in front of the person once per failed retry,
  not forever.
- **H1** (hypothesis): once per failed start is often enough for the
  person to learn of the weaker state and rare enough not to be
  dismissed unread. What would show H1 wrong: in a walkthrough or
  test, a person who has seen the notice once cannot later say whether
  their screen lock is needed after a restart, or a person ignores the
  card from its second appearance on. Until H1 is tested, this rests on the
  heuristics in §2.

## 5. The Trust page line

- **Place**: in the identity section, **after this device's PeerId and
  before "Show recovery phrase"**, in reading and focus order. One
  block of body text (`FocusNote`): it wraps over as many lines as it
  needs, never elided, at every font scale.
- Where §3 names two lines (K with D, P-unknown with D), there are
  two blocks, in the order §3's table lists them, and each is its own
  focus stop.
- **Weight**: plain body text in `on-surface`, no icon, no colour, no
  action. It describes the state; it is not an alarm. Its contrast is
  the body text's on the page surface, already measured.
- **No action**. A P line is three sentences, in this order:
  1. What is not in force, and what a restart does now.
  2. The reason.
  3. That InterWeave tries again each time it starts.

  Only the Storage reason adds what the person may do.

## 6. Focus and TalkBack — every case in one table

| # | Case | Focus goes to | Announced | Check |
|---|---|---|---|---|
| F1 | P-up's notice appears (at a start, or when a higher notice clears) | stays where it was; **the notice never takes focus**, unlike the locked notice | its text, politely, once, as every `NoticeCard`'s message is a polite live region; it sits in reading order after the app bar, before the list | A3 |
| F2 | Details pressed | the Trust page opens; focus on the P-up line | TalkBack reads the focused line | A4 |
| F3 | Back from the Trust page opened by Details | Chats; the notice is gone (seen), so focus goes to the list heading | — | A5 |
| F4 | the notice goes (P-up clears, or another notice replaces it) while it or Details has focus | the list heading | nothing for P-up's leaving; a replacing notice announces itself as it does today | A6 |
| F5 | Trust page opened from App options | as today (unchanged); the line sits between PeerId and Show recovery phrase | — | A7 |
| F6 | a line appears while the Trust page is open (a start under Stay reachable) | stays where it was | its text, politely, once | A8 |
| F7 | a line clears while the Trust page is open, and none takes its place | if it had focus: this device's PeerId field; otherwise unchanged | politely, once: `UnlockApplied` when a P line clears or when D clears because the policy in force changed, `UnlockOldKeyRemoved` when the K line clears; nothing when D clears because the availability mode changed | A9 |
| F8 | rotation, or the window resized, with focus on a line | the same line | nothing | A10 |
| F9 | at one start, a line is replaced by another or its text changes: P → D, P → K, K with D → D, reason r → r′, D's locked text → D's running text at the unlock | if a line had focus: the first line now in that place; otherwise unchanged | when a P line ended, `UnlockApplied` first, or when the K line ended, `UnlockOldKeyRemoved` first; then the text of each line that is new or changed, politely, once each | A12 |
| F10 | §11, L1 and L3: the system prompt rises | the system prompt (the platform's) | the prompt's title, `UnlockPromptTitle` | A14, A16, A17, A18, A19 |
| F11 | §11, L1: that prompt cancelled | depending on the restart: on R, its Try again; for S2's Done, "Show recovery phrase"; for a discard's restart, the locked notice on Chats; for a network grant that leaves the process running, the locked notice on the screen the grant was made on (Chats, or a conversation, under its app bar). A grant that ends the process is not an L1 case, and no prompt rises after it: the next launch is F10 (L3), then F12 if that prompt is cancelled | the focused control, as TalkBack reads it | A15, A16, identity note §10 item 10 |
| F12 | §11, L2, and L3's prompt cancelled: the locked notice shows at a return after a cancel, or after the prompt of a launch or of another return is cancelled | the locked notice, on the screen the person is on (Chats, or a conversation, under its app bar) | the notice's text, as focus reads it | A16, A17, A18, A19 |
| F13 | §11, L4: the start refused while no window shows | nothing is shown then; at the next launch, F10 (L3); at the next return, F10 (L3), or F12 (L2) after a cancel | — | A19 |
| F14 | §11: Unlock granted and the client runs again, while the notice has focus | on Chats, the list heading (as F4); in a conversation, its composer | nothing for the notice's leaving | A20 |
| F15 | §11, L1: R's prompt granted, and R goes with focus inside it | Chats' list heading (as F3, F4); in an open narrow conversation, its header title, never the composer: the person was on R, not writing. A locked notice the start asks for comes first (F11) | the focused control, as TalkBack reads it | A14 |

Keyboard: every line is a Tab stop in the order of §5, and Details is
the notice's action button as on every other notice. Each line, and the
notice, has a non-gesture path: there is no gesture here.

## 7. Strings — what each must say

The words are language-culture-en's; the keys are placeholders that
rust-ui-dev may rename. Use the glossary: "unlock with your screen
lock", "After a restart, … until you unlock". Never "policy", "Keystore",
"seal", "automatically" or "always reachable". Never "key" either, except
in K and in the Keystore reason, where the phone's key storage is the
fact.

| Key | Must say |
|---|---|
| `UnlockRestartNeedsUnlock` | (D) While InterWeave keeps running it can stay reachable; after the phone or the app restarts, it stays offline until you unlock it with your screen lock. Never claims it comes back by itself. |
| `UnlockRestartNeedsUnlockLocked` | (D while locked) After every restart of the phone or the app, InterWeave stays offline until you unlock it with your screen lock. Without "while it keeps running". Never claims it comes back by itself. |
| `UnlockPendingNeedsLock` | (P-up) InterWeave's setting was changed to need your screen lock after a restart, and the change is not in force yet: until it is, InterWeave opens after a restart without your screen lock, as before. |
| `UnlockPendingNoLock` | (P-down) The setting was changed so that InterWeave no longer needs your screen lock after a restart, and the change is not in force yet: until it is, after a restart it stays offline until you unlock it, as before. |
| `UnlockPendingUnknown` | (P-unknown) InterWeave could not apply or check a change to its screen-lock setting; it works as before. |
| `UnlockTriesAgain` | (every P line's last sentence) InterWeave tries again each time it starts. |
| `UnlockReasonStorage` | This phone's storage could not save the change; freeing some space may help. |
| `UnlockReasonKeystore` | This phone's secure key storage did not accept the change; there is nothing to do in InterWeave. |
| `UnlockReasonOther` | A sentence saying only that the change could not be made; no cause. |
| `UnlockOldKeyRemains` | (K) The screen-lock setting is in force; the key used by the previous screen-lock setting could not be removed from this phone yet, and InterWeave tries again each time it starts. Nothing for the person to do. |
| `UnlockPendingNotice` | (Chats, P-up) One sentence: the screen-lock setting is not in force yet, so InterWeave does not need your screen lock after a restart. |
| `UnlockPendingDetails` | The notice's action, opening the line: "Details" or the like. |
| `UnlockApplied` | (announcement, a P line ending) The screen-lock setting is now in force. |
| `UnlockOldKeyRemoved` | (announcement, the K line ending) The key used by the previous screen-lock setting has been removed. |

§11 needs no new string. It uses the locked notice's three
(`AuthenticationNeeded`, `Authenticate`, `UnlockPromptTitle`), which
belong to `design/2026-10-19-android-identity.md` §9.

A reason is its own sentence after the line's first sentence, never a
fragment inside it, so no locale has to make the reason agree
grammatically with the line. If a locale needs one whole string for
each line and reason, language-culture may write the nine P × reason
combinations (three P lines, three reasons) instead. language-culture decides.

## 8. Acceptance

The P and K states cannot be produced on a device on demand (a
Keystore fault, a full disk at the start). The exception is a locked P
line, P-up or P-down (A13(b), A13(d)): A11's configuration reaches it by
authoring one policy through `run-as` over a record of the other, then
A11's locked steps. They are checked by view
tests (`crates/human/ui-slint/tests/views.rs`) with the status member
set. **Precondition for all of them: the four facts of §3 reach the
view (the member's three and the mode, Q3), with `Unsynced` reported as
`OldKeyRemains` (§3).**
Without both, A1–A10, A12 and A13 are blocked, not passed.

- **A1** For each row of §3's table, the Trust page shows exactly the
  line(s) named, in that order, between the PeerId and "Show recovery
  phrase". D's line is absent in P-up and P-down whatever the status
  member says. In P-unknown it shows exactly when D's condition holds.
  A mismatch of in force and configured with no reseal class (`Held`)
  shows the P line with reason `Other`.
- **A2** For each of the three P lines and each reason r, the line
  carries that reason's sentence and no other, between its first
  sentence and `UnlockTriesAgain`.
- **A3** P-up with no other session notice: the notice shows on Chats,
  focus does not move to it, and its text is announced politely once.
  It never shows in an open conversation on a narrow window. With any
  other `SessionNotice` present, that notice shows instead. When that
  notice clears, the P-up notice returns. This note's notice shows in
  no other row.
- **A4** Details opens the Trust page with focus on the P-up line.
- **A5** Back from the Trust page: Chats with no P-up notice and focus
  on the list heading. After a simulated next start with P-up still
  holding, the notice shows again. Recreating the Activity without a
  new start does not bring it back.
- **A6** P-up clears while Details has focus: the notice goes and focus
  is on the list heading. The same holds when another `SessionNotice`
  replaces P-up while Details has focus.
- **A7** Trust from App options in P-up: the focus order is PeerId,
  line, Show recovery phrase. Back to Chats: no notice (seen).
- **A8** A line appears while the Trust page is open: one polite
  announcement of its text, and focus unchanged.
- **A9** A P or K line clears with focus on it: focus is on the PeerId
  field, and the line's announcement is made once: `UnlockApplied` for
  a P line, `UnlockOldKeyRemoved` for K. D clearing because the
  policy in force changed announces `UnlockApplied` once; D clearing
  because the availability mode changed announces nothing.
- **A10** Rotation with focus on a line keeps it there.
- **A11** On the Pixel 9a, by me, at the largest font scale and with
  TalkBack: D, and D's locked text. A locked P line, either direction,
  is reachable on a device too (§8's exception). It is laid out as every
  P line is, so A13(b)'s and A13(d)'s view tests stand in for it here.
  - **Precondition:** a debuggable build, since no in-app control writes
    either setting (§1). The profile's `config.yaml` is authored with
    `key_unlock_policy: user-presence` through `run-as`, as
    `apps/human-android/device-runs/2026-10-20-gate-b.md` did. Stay
    reachable is set the same way, by writing
    `<state>/availability-overlay.json` holding
    `{"availability_mode": "stay-reachable"}` (human-client-android.md,
    "Where the choice lives").
  - The build already installed on the Pixel must be the debuggable one.
    Installing it over a differently signed build wipes the app's data,
    which the phone's runs keep, so A11 never installs one for this.
  - Without such a build, A11 is blocked, not passed.
  - Check: the line wraps whole, is read in the order of A7, and nothing
    on the page is elided.
  - Then reach the locked state (`failure-model.md`'s
    restart-without-authentication case):
    1. Restart the phone, and unlock it to reach the launcher.
    2. Wait more than 60 s. Unlocking the phone opens the key's
       user-presence window (ADR-0042), and inside it the start
       unwraps without a prompt and is not locked.
    3. Open InterWeave, and cancel the screen-lock prompt, so the
       locked notice shows.
    4. Open Trust: D's locked text (`UnlockRestartNeedsUnlockLocked`)
       shows, and passes the same check. This step depends on Q4. If the Trust page does not
    open while locked, it is blocked, not passed.
  - For a P line's layout, a view test at the same scale stands in.
- **A12** F9's transitions, each by a view test with the status member
  changed while the Trust page is open and a line focused:
  - P-up → D;
  - P-down → K;
  - K with D → D;
  - Storage → Keystore;
  - locked D → D (the person unlocks with the Trust page open).

  Check each against F9's focus target and its announcements in order.
- **A13** Locked: the locked notice is shown (the start stopped at
  `UserNotAuthenticated`), each case by a view test.
  - (a) Header user-presence, with the effective mode on disk
    stay-reachable: the Trust page shows D's line, in its locked text
    `UnlockRestartNeedsUnlockLocked`. With the mode
    foreground-only, it shows none.
  - (b) Header background-compatible, configured user-presence (a
    reseal refused `UserNotAuthenticated`): the Trust page shows the
    P-up line, and Chats shows only the locked notice.
  - (c) The profile unreadable, so the mode is unknown: no D line.
  - (d) Header user-presence, configured background-compatible, the
    effective mode on disk stay-reachable: the Trust page shows the
    P-down line and no D line (the P line stands in for it, as when
    running), and Chats shows only
    the locked notice.

**§11's acceptance.** These run on the Pixel 9a, by me, with a
`user-presence` profile authored as A11's precondition says, the
availability mode foreground-only unless the item says otherwise. Each
starts with InterWeave running and online in a process that is already
running. Wait more than 60 s with the phone unlocked and **not**
authenticating again, so that the 60 s window has closed. Where an item
needs a refusal the phone cannot produce on demand (a store refusal,
withheld network access, a background restart), a test hook of
rust-ui-dev's stands in for it, named in the run's record. Without one,
that item is blocked, not passed. A16's network-grant case is the
exception to two of these rules, the running process and the test hook:
that case withholds network access with `adb shell pm revoke` rather
than a test hook, and the command ends the process, so the case starts
in a new process. In every item,
connectivity reads offline while the locked notice shows, and §3's locked rules hold on the
Trust page (A13).
- **A14** L1, R's Try again: the system prompt rises by itself, titled
  `UnlockPromptTitle`, before anything else on the screen changes.
  Granting it runs the restart again, and R then follows the start's answer as
  `design/2026-10-21-contact-cards.md` §7 says. Where R goes, focus is on
  Chats' list heading, on a wide layout too; in an open narrow
  conversation, on its header title; where the start asks for the locked
  notice, on that notice first (F15, F11).
- **A15** L1, R's Try again with the prompt cancelled: R stays, with focus
  on Try again and no other change. No prompt rises again until Try
  again is pressed again. Pressing it raises the prompt once.
- **A16** L1, the prompt cancelled after one of the restarts below.
  Where a network grant ends the process, its case is L3 instead, at
  the next launch:
  - a discard's restart: Chats, offline, with the locked notice focused,
    and neither `StoreDiscarded` nor `StoreRestored` shows. Unlock,
    granted: the locked notice goes, and the discard's notice shows once;
  - a network grant. On the Pixel 9a the grant ends the process, so
    this case is a launch (L3), not L1 (§11, "A network grant that ends
    the process"). With InterWeave online, have a peer send two
    messages and leave both unread. Withhold network access (`adb shell
    pm revoke` of the permission, which ends the process). Open
    InterWeave and deny the network-access dialog: the `NetworkDenied`
    notice shows. If the prompt rises, grant it. Open the conversation
    and read both messages, then keep the second. Wait past the 60 s
    window, then press the `NetworkDenied` notice's action, and Allow in
    the system's dialog. The launcher shows, and `adb shell pidof`
    prints no process id. Open InterWeave: the prompt rises before Chats
    is online. Cancelled: Chats, offline, with the locked notice
    focused, and no conversations. Unlock, granted: Chats, online. Open
    the conversation: the kept message shows, and the message read and
    not kept does not. On a platform where the grant leaves the process
    running, the case is L1. When the prompt is cancelled, the screen
    the grant was made on stays, offline, with the locked notice
    focused in place of the `NetworkDenied` notice, under the
    conversation's app bar if a conversation was open.
- **A17** L2 and L3, a return: open a conversation and type a draft.
  Press Home, wait past the 60 s window, and return from Recents. The prompt
  rises (no prompt was cancelled since the last start that ran). Cancel
  it: the conversation is still open with its messages, the draft and
  its composer, and the locked notice is under its app bar, with focus
  on it. Press Home and return again past the 60 s window: no prompt rises,
  and the notice keeps focus. Unlock, granted: the client runs, and
  focus is on the composer with the draft (F14). A return within 60 s
  of unlocking the phone shows no notice and no prompt, and the bind runs.
- **A18** L3, a launch in a live process: press Back on Chats, so that
  the Activity finishes and the process stays (`adb shell pidof` shows
  the same pid). Wait past the 60 s window, then open InterWeave from the
  launcher. The prompt rises before Chats is online. Cancelled: Chats,
  offline, with the locked notice focused.
- **A19** L4 and L3, under Stay reachable, by a hook:
  - The restart is refused with the window in front: the prompt rises
    (L3). Cancelled: the locked notice on the screen in front, focused.
  - The restart is refused with no window shown: no prompt rises and
    no notification is added. At the next return the prompt rises (L3).
  - A prompt is cancelled, then Home, then the restart is refused with
    no window shown: at the next return no prompt rises, and the locked
    notice shows, focused (L2).
- **A20** After A16 to A19, Unlock, granted, runs the refused restart
  again, once. The notice goes, and focus follows F14.

## 9. Deliberately left out

- **A control to change the unlock policy or Stay reachable.** Neither
  exists. When one is designed, its confirmation is where a pending
  change is first shown, and applying it is the retry (ADR-0041
  A 2026-11-05's revisit condition). D's sentence goes beside the
  Stay-reachable switch. This note's surfaces stay as they are.
- **A Try again** on the line or the notice. Ruled out by ADR-0041
  A 2026-11-05 (Q2): every start retries, and a retry while the service
  runs would be a restart that, under user-presence, costs an Unlock
  prompt.
- **The desktop**: it has no unlock policy.
- **The Trust page itself while locked.** Its PeerId field is blank
  and its trust list may not finish reading, because both come from the
  runtime (Q4's answer). That gap belongs to the Trust page, not to this
  note. Showing the record's PeerId without opening the record would
  put an unverified identity in front of a person who compares it by
  eye; whether that is allowed is architect-cto's to rule. This note's
  lines keep their place after the PeerId field whether it is filled
  or blank.
- **The locked notice in P-down**: it keeps its text. The person who
  wonders why they were asked to unlock finds the reason on the Trust
  page.
- **The raw refusal** (`detail`): it is for the log, as ADR-0042
  says, and is never shown.
- **The foreground-service notification**: its text already promises
  nothing across a restart.
- **A notification when a background restart is locked** (§11, L4). A
  person away from the window learns of it only at their next return or
  launch, from the prompt. Telling them sooner would add a notification to the product,
  which is architect-cto's to decide. Until it is asked for, Stay
  reachable's limit is the one `human-client-ui.md` §14 already states.
- **The discard's notice lost with the process** (§11, L1). If the
  process ends before Unlock, `StoreDiscarded` or `StoreRestored` never
  shows. The person confirmed the deletion in the discard's dialog, and the notice
  is not stored, by `design/2026-10-21-contact-cards.md` §7.3.
- **A word at the next launch about a grant that ended the process**
  (§11). The app cannot speak before the process ends. At the next
  launch, InterWeave shows where it now stands: connectivity reads
  online, or the locked notice shows. This note adds no notice saying
  why the app closed. **H2** (hypothesis): what the next launch shows
  is enough, and the person needs no notice of why the app closed.
  What would show H2 wrong: in a walkthrough or test on a platform
  where the grant ends the process, a person who allowed network
  access says that the app crashed, or does not notice that the message
  they read and did not keep is gone. A notice would then be a new
  state of the product, so architect-cto would decide on such a notice
  first.

## 10. Questions

- **Q1 — architect-cto.** `human-client-ui.md` §14's predicate reads the
  *configured* policy. While a change is pending, that tells the person
  the opposite of what happens. This note suppresses D's line in P-\* and lets the P
  line say what happens now (§3). Should §14 (and `android-key-custody.md`)
  say that the diagnostic follows the policy **in force** while a change
  is pending, so that the status model and the UI agree by rule rather
  than by this note?
  **Answered yes** (REPLY 01a1ac8d-ea64, ADR-0041 A 2026-11-05, #571):
  the diagnostic is derived from the effective availability mode and
  the record's policy in force. The configured policy reaches the
  status model only as the pending change. §3 follows that answer.
- **Q2 — architect-cto.** A Try again that restarts the service, so the
  person can retry once they have freed space, is not specified here:
  it would add a product action. Is one wanted? If it is, it belongs on
  the Storage line only.
  **Answered no**, for now (same reply and amendment): see §9.
- **Q3 — rust-ui-dev.** The status member on `ServiceHost` needs to
  carry: the policy in force (the record's), the configured policy or
  "unknown", the pending class (`Keystore`, `Storage`, `Other`) or
  `OldKeyRemains`, and the effective availability mode. Can it?
  **Answered yes** (REPLY 01a1ac85-a1ee, 2026-11-05), and since built
  otherwise: the member (`UnlockStatus`) carries three of §3's facts,
  read again at each start, and the mode comes from the host or from
  disk (§3). §3 defines all four; they are not restated here. rust-ui-dev accepted
  each change §3 made after their first reply:
  - in force from the opened record, or from the record the reseal
    wrote when it wrote one (REPLY 01a1ac9e-52ad);
  - "pending" from the comparison, `Held` included (same reply);
  - `Unsynced` reported as `OldKeyRemains` (REPLY 01a1aca2-ea97).

  Both are built on rust-ui-dev's j213 branch: `UnlockStatus` and
  `ServiceHost::unlock_status()` at 77d337267, and the classing at
  15632b734. At 77d337267, `ServiceHost::restart_requires_user_authentication()`
  answers `None` while no runtime runs, so the view cannot take the mode
  from it then. It reads §3's fourth fact from disk instead. Until they
  reach `main`, A1–A10, A12 and A13 stay blocked, as §8 says.
- **Q4 — rust-ui-dev.** Does the Trust page open, with its identity
  section, while InterWeave is locked (no runtime, the locked notice
  showing)? The App options button has no locked condition in
  `app.slint`, but what the page can read without the runtime is your
  code's. If it does not open, the locked text of D has no surface on
  Android today. Then this note needs a place for it, and A11's locked
  step and A13(a) are blocked until it has one.
  **Answered yes** (REPLY 01a1ace2-1ffe, read from the code at
  6464c3cf0, not run): the page opens while locked, but its PeerId field
  is blank and its list may stay reading (§9). rust-ui-dev is also
  making `restart_requires_user_authentication()` read the mode from
  disk while no host runs, and keeping the last start's pending class
  while locked if the record's policy is unchanged.
- **Q5 — architect-cto.** §11 sorts each refused restart in a running
  process by the rule `android-key-custody.md` (failure list) and
  ADR-0042 A 2026-10-20 already state. The prompt answers a refusal,
  with one exception: the start retried when a live Activity continues
  after a cancelled or denied prompt, which the notice and Unlock
  answer. The Service never prompts. The rule's list of
  refusals that the prompt answers does not name three restarts: R's Try
  again, a discard's restart, and a restart after network access is
  granted. §11 treats them as the person's own action (L1). Is that the
  rule's reading? The network-access grant arrives as the Activity
  resumes, so it could also be read as a return (L2 in this note's
  first revision, which answered every return with the notice).
  **Answered: L1 for all three** (REPLY 01a1adae-487d, 2026-11-05).
  A case is sorted by what started the restart, never by the lifecycle
  callback that carries it. A network grant is the person's answer to a
  dialog over the Activity, not a switch back into it; a denied grant
  restarts nothing. No ADR changes: ADR-0042 A 2026-10-20 takes out only
  the start retried when a live Activity continues. architect-cto names
  the three in the custody doc's list on their PR #575.

## 11. A restart refused for the screen lock while the app runs

**The gap.** rust-ui-dev traced this in REPLY 01a1ada5, from the code,
not on a device. A process that has once found the identity ready never
checks it again. A start refused `UserNotAuthenticated` later in that
same process gets no prompt and no locked notice. The person stays on
Chats with no client running, no Unlock and nothing to say why. This
lasts until the process ends, and reopening the app does not help. It
happens on a `user-presence` profile once the 60 s window has closed,
or after the policy changed while the app ran. The restarts that meet
it:
- R's Try again (`design/2026-10-21-contact-cards.md` §7.1);
- the restart after a discard (§7.3 there);
- the restart after S2's Done (Show my phrase,
  `design/2026-10-19-android-identity.md` §3.5);
- the restart after network access is granted, on a platform where
  the grant leaves the process running (below, "A network grant that
  ends the process");
- a new Service, bound by an Activity that already exists or that is
  created in the same process;
- a Stay-reachable restart in the background.

**The rule.** Whether the process keeps running changes nothing the
person sees. Every refusal is answered as `android-key-custody.md` (failure
list) and ADR-0042 A 2026-10-20 answer it in a fresh process:
- **The prompt answers a refusal** (the Activity raises it; the Service
  never prompts).
- **The one exception**: the start retried when a live Activity
  continues (a return from Home or Recents, or a recreation for a
  configuration change) **after a cancelled or denied prompt**. The
  locked notice and Unlock answer that, so a person who declined is not
  asked again each time they switch back in.

The four cases below apply that rule, sorted by what started the
restart (Q5). They add no rule of their own.

| Case | What started the restart | Prompt | After a cancel, or with no prompt |
|---|---|---|---|
| L1 | The person, by an action on a screen in front: R's Try again, a discard's restart, S2's Done, a network grant (Q5) | Rises by itself, before anything else changes | R: stays on R, focus on Try again, nothing else changes. S2's Done: as `design/2026-10-19-android-identity.md` §10 item 10 says (Settings, focus on "Show recovery phrase"). A discard's restart: Chats, offline, with the locked notice focused. A network grant that leaves the process running: the screen the grant was made on (Chats, or the open conversation), offline, with the locked notice focused. A grant that ends the process is not an L1 case: its next launch, in a new process, is answered as L3 answers any launch ("A network grant that ends the process", below the table) |
| L2 | A return to a live Activity (Home or Recents, then back; a recreation for a configuration change) after a prompt was cancelled or denied, with no start run since | Never by itself: ADR-0042 A 2026-10-20's exception | The locked notice and Unlock on the screen the person returns to, focused (F12). The bind runs silently if the 60 s window has opened meanwhile |
| L3 | Any other start the Activity makes or retries while its window is in front: a launch (the Activity created anew in a process still running, after Back finished it or from a notification); a return with no prompt cancelled or denied since the last start that ran; a Stay-reachable restart refused while the window is in front | Rises by itself: at a launch before the Service binds, as at a cold launch (a launch is the Activity's creation, whatever the process) | The screen in front (Chats at a launch), offline, with the locked notice focused (F12) |
| L4 | A Stay-reachable restart while no window shows | Never: the Service never prompts | Nothing is shown then. At the next launch the Activity's start is refused again, and L3 answers it. At the next return it is refused again, and L3 answers it, or L2 if a prompt was cancelled or denied since the last start that ran. A refused start, this one included, is not a start that ran |

In every case, a grant runs the refused restart again, once. Leaving
while the prompt shows counts as a cancel. After a cancel, nothing
raises the prompt again by itself: the person's next action does, or
Unlock does. Both rules come from the identity note (§6, and §10 item 10).

**A network grant that ends the process.** On the Pixel 9a under
GrapheneOS, Allow in the system's network-access dialog ends
InterWeave's process. The process end was seen during the review of
this section, in the j98 run that began on 2026-11-07, at 01:49 by the
phone's clock; the ux-designer thread on that run records it
(`.agent-fabric/memory/ux-designer/threads/j98-unlock-s11-pixel-review.md`).
The log reads "permission grant
or revoke changed gids", and the Activity is removed with no saved
state. The process that refused the restart is gone, so it never runs
the restart again after such a network grant, and L1's network-grant
case never arises there. What the person meets instead:
- **The launcher.** InterWeave shows nothing, because no process is
  left to show it. The j98 run had the availability mode Stay
  reachable, and no restart was seen in it. With the mode
  foreground-only, nothing is expected to restart; A16, which runs
  foreground-only, checks whether anything restarts.
- **The next launch is a launch in a new process**, and L3 answers it.
  Past the 60 s window, the prompt rises before the Service binds.
  Cancelled: Chats, offline, with the locked notice focused (F12).
  Granted: Chats, online. Within the 60 s window: Chats, online.
- **Chats, not the conversation the grant was made in**, since the
  Activity kept no state.
- **A message read and not kept is gone** from its conversation. The
  store removed it when it was read: with network access withheld,
  reading still works as ADR-0044 defines it (`human-client-android.md`,
  the `network_denied` state). A new process shows only what the store
  holds: unread and kept messages, and outbound still pending. Any end
  of the process has the same effect. The grant is where a person meets
  this loss without having chosen to close the app.

Where the grant leaves the process running, L1's row holds as written.
Stock Android is expected to be such a platform: it ends a process on
a revoke, but usually not on a grant. Two things have not been checked
on a device: whether stock Android leaves the process running at a
grant, and what a grant made from the app's settings page, rather than
from the dialog, does to the process.

**What Chats and a conversation show while locked, in a running
process.**
- **Connectivity** reads offline.
- **The locked notice** (`AuthenticationNeeded`, with Unlock) shows in
  the session notices' slot, under the app bar of Chats and of an open
  conversation. It is the first of the session notices; P-up's notice
  is the last (§4).
- **The conversation list, and an open conversation, stay** as they
  were. A refused restart neither empties them nor closes them, so the
  person keeps what they were reading. Nothing new arrives until the
  client runs again. A locked launch shows no conversations only
  because none has been loaded from the store yet. The notice's "your conversations …
  appear again after you unlock" is then a promise; here it describes
  the list that stays.
- **The composer stays**, with its draft, as it does offline
  (`design/2026-10-16-android-look.md` §6: it is absent only while
  network access is withheld; "what is written waits"). A send while
  locked is a send while offline. This note adds nothing to it.
- **The discard's notice** (`StoreDiscarded`, or `StoreRestored` after the
  partial discard) waits for the start that opens the store, so it shows
  after Unlock (A16). It never shows beside the locked notice: a
  "deleted" or "restored" notice next to "locked" would say the opposite
  of what the person can now do.
- **The Trust page** follows §3's locked rules, unchanged.

**Evidence.** These are heuristic readings, not tests with people:
- Nielsen's visibility of system status. Today the person is told
  nothing while the app is offline and cannot recover, and that is the
  defect.
- Consistency. The rule is the same in every process: the prompt or
  the notice comes at the same moments as at a fresh start.
- User control: the custody failure list's rule that a person who
  declined is not asked again each time they switch back in (L2).

**Built by** rust-ui-dev. They route `UserNotAuthenticated` to the locked
notice, and make a grant restart the bound Service where the grant
leaves the process running (REPLY 01a1ada5). They
also choose how the Activity tells a launch from a return, which is
theirs as long as A17 to A19 pass.
