# The screen-lock setting not yet in force, and the restart diagnostic — Android

Status: proposal, 2026-11-05. Asked by rust-ui-dev (REQUEST 01a1abf3,
their job j213, my job j93), read against `origin/main` 8859f6366 and
their branch `develop-qzapp/rust-ui-dev-01/feat/j213-android-start-never-calls-custody-reseal`
at 76ac4a188. It answers where a person sees that a changed
`key_unlock_policy` is not yet in force and why (ADR-0042 A 2026-11-05),
and puts `human-client-ui.md` §14's
`background_restart_requires_user_authentication` diagnostic on the
same surface, since both say what happens to InterWeave after a
restart. Two product questions are architect-cto's (§10, Q1 and Q2);
one input is rust-ui-dev's (Q3).

Owner of the design: ux-designer. The words are language-culture's: §7
names each string and what it must say. No new colour: the line is body
text on the surface and the notice is the existing `NoticeCard`
(`design/2026-10-16-android-look.md` §3). Built by rust-ui-dev. Android
only: the desktop has no unlock policy.

## 1. What is already decided

- ADR-0042 A 2026-11-05 and `android-key-custody.md` (Reseal and the
  policy byte): a reseal that fails otherwise than `UserNotAuthenticated`
  never refuses the start. The record opens under its own policy, the
  start goes on, and the reseal is tried again at every start. While the
  record's policy differs from the configured one, or the other
  policy's key remains (`OldKeyRemains`), the status model exposes the
  pending change by name, with the refusal's reason.
- rust-ui-dev's branch classes the refusal as `ResealFailure`:
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
  which no control calls. So the person never meets this state just
  after pressing something. They meet it on a later look, often long
  after someone else changed the file.
- The app's one settings surface is the Trust page (App options →
  Trust). It already shows this device's PeerId and, under it, "Show
  recovery phrase" (`design/2026-10-19-android-identity.md` §3.5). That
  is the identity's section, and the unlock policy guards that identity.

## 2. The person's goal

The person wants to know **whether InterWeave needs their screen lock
after the phone or the app restarts**, because that decides two things:
whether someone holding the phone after a restart can open their
identity, and whether InterWeave comes back online by itself.
When the answer differs from what the setting says, they want to know
that, what the answer is now, why, and whether they must do anything.
Usually they need to do nothing: the next start tries again. Freeing
storage is the one thing that can help.

Evidence: a heuristic reading, not a test with people. Nielsen's
visibility of system status (the state is invisible otherwise, and in
one direction it is weaker than the person was told). The match between
system and real world (say what happens after a restart, not "policy").
Aesthetic and minimalist design (a fault the person cannot act on does
not stay on the main screen forever). Hypothesis H1, with what would
falsify it, is in §4.

## 3. States

The view reads four facts from the status member (Q3): the policy the
record opens under (**in force**), the **configured** policy, the
reseal's pending class if any, and the effective availability mode.

| State | Condition | Trust page line (§5) | Chats notice (§4) |
|---|---|---|---|
| N | nothing pending; not (stay-reachable and user-presence) | none | none |
| D | nothing pending; effective stay-reachable and user-presence in force | `UnlockRestartNeedsUnlock` | none |
| P-up(r) | configured user-presence, in force background-compatible, reason r | `UnlockPendingNeedsLock` + reason r | `UnlockPendingNotice`, per §4 |
| P-down(r) | configured background-compatible, in force user-presence, reason r | `UnlockPendingNoLock` + reason r | none |
| P-unknown | pending, configured policy unknown (the profile could not be read at the reseal) | `UnlockPendingUnknown` | none |
| K | in force = configured; the other policy's key remains (`OldKeyRemains`) | `UnlockOldKeyRemains`, and D's line under it when D's condition holds | none |

Reasons r: `Storage`, `Keystore` and `Other`. These are rust-ui-dev's
classes, and the person is told about each one differently:

- **Storage**: the phone could not save the change. Freeing space may
  help, and it is the only class where the person can do something.
- **Keystore**: the phone's secure key storage did not accept the
  change. Nothing the person does in InterWeave helps. Saying so
  stops the person looking for a fix that does not exist.
- **Other**: the change could not be made. No cause is offered.

`OldKeyRemains` is a state (K), not a reason r: the record is already
under the configured policy.

**While a change is pending (P-\*), D's line is not shown.** D's
predicate reads the configured policy, so in P-up it would say a
restart needs the screen lock when it does not. In P-down it would stay
silent when a restart does need it. Each P line says what happens after
a restart **now**, which covers what D would have said. Whether §14's
predicate itself should follow the policy in force is architect-cto's
question, Q1.

Shown and cleared: the state is read again whenever the status member
changes. A start whose reseal succeeds leaves N or D. Nothing is
shown "for a while" after it clears: the line goes and, on the Trust
page only, the change is announced (§6).

## 4. The Chats notice — P-up only

Only P-up reaches Chats. It is the one direction where the phone is
**less protected than the setting says**: after a restart, someone
holding the phone can open InterWeave without the screen lock. A
person who looks only at Chats would otherwise never learn of it.
P-down only costs an Unlock prompt the person already sees and
answers, and its line on the Trust page explains why. K and D are not
faults.

- **Place and priority**: the session notice's slot, where the app's
  notices already show (under the app bar of Chats on a narrow window,
  beside the list on a wide one). It has the **lowest priority** of all
  session notices. Every existing `SessionNotice` (no daemon,
  reconnecting, storage, refused, network denied, locked) replaces it
  while it holds, and it returns when they clear.
- **Weight**: informational, the same card as the others, one action:
  `UnlockPendingDetails`. It opens the Trust page with focus on the
  line (§6). It has no Try again: none exists, and adding one is Q2.
- **How long**: from the start whose reseal left P-up, **until the
  person has seen the Trust page's line** (by Details, or by opening
  Trust from App options while P-up holds), or until P-up clears. It is
  remembered in the view only, not stored. If P-up still holds, it shows
  again after the next start, because that start retried and failed
  again. This puts it in front of the person once per failed retry,
  not forever.
- **H1** (hypothesis): once per failed start is often enough for the
  person to learn of the weaker state and rare enough not to be
  dismissed unread. What would show H1 wrong: in a walkthrough or
  test, a person who has seen the notice once cannot later say whether
  their screen lock is needed after a restart, or a person ignores the
  card by its second appearance. Until it is tested, this rests on the
  heuristics in §2.

## 5. The Trust page line

- **Place**: in the identity section, **after this device's PeerId and
  before "Show recovery phrase"**, in reading and focus order. One
  block of body text (`FocusNote`): it wraps over as many lines as it
  needs, never elided, at every font scale.
- In K with D, two blocks, K first, each its own focus stop.
- **Weight**: plain body text in `on-surface`, no icon, no colour, no
  action. It describes the state; it is not an alarm. Its contrast is
  the body text's on the page surface, already measured.
- **No action**. A P line ends by saying InterWeave tries again each
  time it starts. Only the Storage reason adds what the person may do.

## 6. Focus and TalkBack — every case in one table

| # | Case | Focus goes to | Announced | Check |
|---|---|---|---|---|
| F1 | P-up's notice appears (at a start, or when a higher notice clears) | stays where it was; **the notice never takes focus**, unlike the locked notice | nothing; TalkBack reaches it in reading order after the app bar, before the list | A3 |
| F2 | Details pressed | the Trust page opens; focus on the P-up line | TalkBack reads the focused line | A4 |
| F3 | Back from the Trust page opened by Details | Chats; the notice is gone (seen), so focus goes to the list heading | — | A5 |
| F4 | the notice goes (P-up clears, or another notice replaces it) while it or Details has focus | the list heading | nothing | A6 |
| F5 | Trust page opened from App options | as today (unchanged); the line sits between PeerId and Show recovery phrase | — | A7 |
| F6 | a line appears while the Trust page is open (a start under Stay reachable) | stays where it was | its text, politely, once | A8 |
| F7 | a line clears while the Trust page is open | if it had focus: this device's PeerId field; otherwise unchanged | `UnlockApplied`, politely, once, for a P line or K clearing; nothing when D clears (the availability changed, not the lock) | A9 |
| F8 | rotation, or the window resized, with focus on a line | the same line | nothing | A10 |

Keyboard: every line is a Tab stop in the order of §5, and Details is
the notice's action button as on every other notice. Both lines and the
notice have non-gesture paths: there is no gesture here.

## 7. Strings — what each must say

The words are language-culture-en's; the keys are placeholders that
rust-ui-dev may rename. Use the glossary: "unlock with your screen
lock", "After a restart, … until you unlock". Never "policy", "Keystore",
"seal", "key" (except in K, where it is the fact), "automatically" or
"always reachable".

| Key | Must say |
|---|---|
| `UnlockRestartNeedsUnlock` | (D) While InterWeave keeps running it can stay reachable; after the phone or the app restarts, it stays offline until you unlock it with your screen lock. Never claims it comes back by itself. |
| `UnlockPendingNeedsLock` | (P-up) InterWeave's setting was changed to need your screen lock after a restart, and the change is not in force yet: until it is, InterWeave opens after a restart without your screen lock, as before. It tries again each time it starts. |
| `UnlockPendingNoLock` | (P-down) The setting was changed so that InterWeave no longer needs your screen lock after a restart, and the change is not in force yet: until it is, after a restart it stays offline until you unlock it, as before. It tries again each time it starts. |
| `UnlockPendingUnknown` | (P-unknown) InterWeave could not check its screen-lock setting after a restart; it works as before and tries again each time it starts. |
| `UnlockReasonStorage` | This phone's storage could not save the change; freeing some space may help. |
| `UnlockReasonKeystore` | This phone's secure key storage did not accept the change; there is nothing to do in InterWeave. |
| `UnlockReasonOther` | none, or a sentence saying only that the change could not be made; no cause. language-culture decides. |
| `UnlockOldKeyRemains` | (K) The screen-lock setting is in force; the key that the setting before it used could not be removed from this phone yet, and InterWeave tries again each time it starts. Nothing for the person to do. |
| `UnlockPendingNotice` | (Chats, P-up) One sentence: the screen-lock setting is not in force yet, so InterWeave does not need your screen lock after a restart. |
| `UnlockPendingDetails` | The notice's action, opening the line: "Details" or the like. |
| `UnlockApplied` | (announcement) The screen-lock setting is now in force. |

A reason is its own sentence after the line's first, never a fragment
inside it, so no locale has to make it agree grammatically. If a
locale needs one whole string per pair, language-culture may write the
six P × reason combinations instead. That is their call.

## 8. Acceptance

The P and K states cannot be produced on a device on demand (a
Keystore fault, a full disk at the start). They are checked by view
tests (`crates/human/ui-slint/tests/views.rs`) with the status member
set. **Precondition for all of them: the status member carries the four
facts of §3 (Q3).** Without it, A1–A10 are blocked, not passed.

- **A1** For each row of §3's table, the Trust page shows exactly the
  line(s) named, in that order, between the PeerId and "Show recovery
  phrase". D's line is absent in every P row.
- **A2** For each reason r, the P line carries that reason's sentence
  and no other.
- **A3** P-up with no other session notice: the notice shows and focus
  does not move to it. With any other `SessionNotice` present, that
  notice shows instead. When it clears, P-up's returns. No notice in
  any other row.
- **A4** Details opens the Trust page with focus on the P-up line.
- **A5** Back from there: Chats with no P-up notice and focus on the
  list heading. After a simulated next start with P-up still held, the
  notice shows again.
- **A6** P-up clears while Details has focus: the notice goes and focus
  is on the list heading.
- **A7** Trust from App options in P-up: the focus order is PeerId,
  line, Show recovery phrase. Back to Chats: no notice (seen).
- **A8** A line appears while the Trust page is open: one polite
  announcement of its text, and focus unchanged.
- **A9** A P or K line clears with focus on it: focus is on the PeerId
  field and `UnlockApplied` is announced once. D clearing announces
  nothing.
- **A10** Rotation with focus on a line keeps it there.
- **A11** On the Pixel 9a, by me, at the largest font scale and with
  TalkBack: D, the one state a device can reach (Stay reachable and
  user-presence authored in the profile). The line wraps whole, is
  read in the order of A7, and nothing on the page is elided. For a P
  line's layout, a view test at the same scale stands in.

## 9. Deliberately left out

- **A control to change the unlock policy or Stay reachable.** Neither
  exists. When one is designed, its confirmation is where a pending
  change is first shown, and D's sentence goes beside the Stay-reachable
  switch. This note's surfaces stay as they are.
- **A Try again** on the line or the notice (Q2).
- **The desktop**: it has no unlock policy.
- **The locked notice in P-down**: it keeps its text. The person who
  wonders why they were asked to unlock finds the reason on the Trust
  page.
- **The raw refusal** (`detail`): the log's, as ADR-0042 says. It is
  never shown.
- **The foreground-service notification**: its text already promises
  nothing across a restart.

## 10. Questions

- **Q1 — architect-cto.** §14's predicate reads the *configured*
  policy. While a change is pending, that tells the person the opposite
  of what happens. This note suppresses D's line in P-\* and lets the P
  line say what happens now (§3). Should §14 (and `android-key-custody.md`)
  say that the diagnostic follows the policy **in force** while a change
  is pending, so that the status model and the UI agree by rule rather
  than by this note?
- **Q2 — architect-cto.** A Try again that restarts the service, so the
  person can retry once they have freed space, is not specified here:
  it would add a product action. Is one wanted? If it is, it belongs on
  the Storage line only.
- **Q3 — rust-ui-dev.** The status member on `ServiceHost` needs to
  carry: the policy in force (the record's), the configured policy or
  "unknown", the pending class (`Keystore`, `Storage`, `Other`) or
  `OldKeyRemains`, and the effective availability mode. Can it? Your
  branch at 76ac4a188 has the class but not yet the member.
