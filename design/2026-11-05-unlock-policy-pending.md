# The screen-lock setting not yet in force, and the restart diagnostic — Android

Status: proposal, 2026-11-05. Asked by rust-ui-dev (REQUEST 01a1abf3,
their job j213, my job j93), read against `origin/main` 8859f6366 and
their branch `develop-qzapp/rust-ui-dev-01/feat/j213-android-start-never-calls-custody-reseal`
at 76ac4a188. This note answers where a person sees that a changed
`key_unlock_policy` is not yet in force and why (ADR-0042 A 2026-11-05),
and puts `human-client-ui.md` §14's
`background_restart_requires_user_authentication` diagnostic on the
same surface, since both say what happens to InterWeave after a
restart. The note's two product questions are answered by architect-cto
(REPLY 01a1ac8d-ea64, §10). Q1: the diagnostic follows the policy in
force. Q2: no Try again. Both are recorded in ADR-0041 A 2026-11-05 on
#571, which is not yet merged. Its one question to rust-ui-dev is
answered too (Q3).

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

The view reads four facts from the status member (Q3):
- **in force**: the authenticated policy byte of the record this start
  opened, as ADR-0041 A 2026-11-05 defines it. After any start that ran,
  it is known. It is not a claim read from the file without opening it.
- **configured**: the configured policy, or unknown when the profile
  could not be read at the reseal.
- the reseal's pending class, if any.
- the effective availability mode.

**A change is pending whenever in force and configured differ**, whatever
the start's reseal answered, `Held` included (ADR-0042 A 2026-11-05 asks
for the pending change to be shown "while the record's policy differs").
A pending change with no class from the reseal is shown with reason
`Other`. When the configured policy is unknown, no comparison can be
made. With a reason class, the state is P-unknown. With none, it is N
or D, decided by the policy in force alone.

A known limit: when the reseal wrote the new record but could not sync
it (`Storage`, custody's `Unsynced`), the next start may already open
the new record. The P line describes the record this start opened, so
for one restart it may say less protection than there is (P-up) or
more (P-down). The next start then reads it again.

| State | Condition | Trust page line (§5) | Chats notice (§4) |
|---|---|---|---|
| N | nothing pending; not (stay-reachable and user-presence) | none | none |
| D | nothing pending; effective stay-reachable and user-presence in force | `UnlockRestartNeedsUnlock` | none |
| P-up(r) | configured user-presence, in force background-compatible, reason r | `UnlockPendingNeedsLock` + reason r | `UnlockPendingNotice`, per §4 |
| P-down(r) | configured background-compatible, in force user-presence, reason r | `UnlockPendingNoLock` + reason r | none |
| P-unknown | a reason r pending and either the configured policy unknown (the profile could not be read), or in force = configured (the reseal failed otherwise than `OldKeyRemains` with the record already under the configured policy, as `Other` can), so no direction can be named | `UnlockPendingUnknown` + reason r, and D's line under it when D's condition holds (below) | none |
| K | in force = configured; the other policy's key remains (`OldKeyRemains`) | `UnlockOldKeyRemains`, and D's line under it when D's condition holds | none |

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
| F9 | at one start, a line is replaced by another or its text changes: P → D, P → K, K with D → D, reason r → r′ | if a line had focus: the first line now in that place; otherwise unchanged | when a P line ended, `UnlockApplied` first, or when the K line ended, `UnlockOldKeyRemoved` first; then the text of each line that is new or changed, politely, once each | A12 |

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

A reason is its own sentence after the line's first sentence, never a
fragment inside it, so no locale has to make the reason agree
grammatically with the line. If a locale needs one whole string for
each line and reason, language-culture may write the nine P × reason
combinations (three P lines, three reasons) instead. language-culture decides.

## 8. Acceptance

The P and K states cannot be produced on a device on demand (a
Keystore fault, a full disk at the start). They are checked by view
tests (`crates/human/ui-slint/tests/views.rs`) with the status member
set. **Precondition for all of them: the status member carries the four
facts of §3 (Q3).** Without it, A1–A10 and A12 are blocked, not passed.

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
  TalkBack: D, the only state with a line that a device can reach.
  - **Precondition:** a debuggable build, since no in-app control writes
    either setting (§1). The profile's `config.yaml` is authored with
    `key_unlock_policy: user-presence` through `run-as`, as
    `apps/human-android/device-runs/2026-10-20-gate-b.md` did. Stay
    reachable is set through its overlay file the same way.
  - Without such a build, A11 is blocked, not passed.
  - Check: the line wraps whole, is
  read in the order of A7, and nothing on the page is elided. For a P
  line's layout, a view test at the same scale stands in.
- **A12** F9's transitions, each by a view test with the status member
  changed while the Trust page is open and a line focused:
  - P-up → D;
  - P-down → K;
  - K with D → D;
  - Storage → Keystore.

  Check each against F9's focus target and its announcements in order.

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
- **The locked notice in P-down**: it keeps its text. The person who
  wonders why they were asked to unlock finds the reason on the Trust
  page.
- **The raw refusal** (`detail`): it is for the log, as ADR-0042
  says, and is never shown.
- **The foreground-service notification**: its text already promises
  nothing across a restart.

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
  **Answered yes** (REPLY 01a1ac85-a1ee, 2026-11-05): the member carries all
  four, read again at each start. Their reply read the policy in force
  from the record's unopened claim, which may be unknown. Round 1's
  review (F2) found that this disagrees with ADR-0041 A 2026-11-05, so
  §3 now asks for the policy byte of the record the start opened, and
  for "pending" to come from comparing in force with configured, `Held`
  included. Both requests went to rust-ui-dev. Until the member is
  built, A1–A10 and A12 stay blocked as §8 says.
