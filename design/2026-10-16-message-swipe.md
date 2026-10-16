# Swipe actions on a message row — Android human client

Status: specification for rust-ui-dev to build (`crates/human/ui-model`,
`crates/human/ui-slint`, `apps/human-android`). Written by ux-designer,
2026-10-16. It answers to `architecture/docs/architecture/human-client-ui.md`;
where this file and that one disagree, that one wins and this file is
wrong.

## 1. What the owner asked, and how it was read

The owner, in their words (2026-10-16):

> I want swipe at message
> Swipe right → Keep / Like / Accept
> Swipe left → Discard / Reject / Skip

The app has no reactions and no requests, so "Like", "Accept" and
"Reject" have nothing to act on here (§9). The owner confirmed this
reading on 2026-10-16, through fabric-coordinator:

| kind | the row is | swipe right | swipe left |
|---|---|---|---|
| A | received, read, not kept | Keep | Hide for now (new) |
| B | received, kept | — | Unkeep |
| C | sent, pending or failed | Retry now | Cancel send |
| D | sent, accepted by remote transport | — | — |

The owner also ruled that an Undo on a snackbar is enough, with no
confirmation before Unkeep or Cancel.

## 2. The person's goal

A person reading a conversation on a phone wants to deal with each
message where it is, with one thumb: keep what matters, clear what
doesn't, and retry or take back something stuck on its way out. They
don't want to aim at small buttons. And a stray thumb must never cost
them a message they can't get back.

## 3. Which rows can be swiped, and in which direction

**Rule:** a direction is offered on a row exactly when its action is
legal on that row now. For Keep, Unkeep, Retry and Cancel that means
`UiModel::actions(key)`. Hide for now is offered on kind A only.
The swipe never decides legality itself. When the model's answer
changes, the swipe changes with it.

- A direction with no action **does not move**. The row does not
  follow the finger that way and nothing is revealed. A row that
  slides and then does nothing tells the person something happened.
- Kind D, and any row whose actions are empty, does not move either
  way. A vertical scroll that starts on it scrolls as usual.
- In a right-to-left layout the directions are **not** mirrored in
  this version: right is Keep or Retry in every locale (§10, open
  question 1).

## 4. The gesture

Distances are in dp. Sizes in the view are logical pixels, which
Slint maps 1:1 to dp on Android. Two words used below:

- **Slop:** the system's touch slop, `ViewConfiguration.getScaledTouchSlop`
  (about 8 dp).
- **Commit:** releasing the finger in a way that carries out the action.

| phase | what starts it | what the person sees and feels |
|---|---|---|
| idle | — | the row at rest |
| tracking | horizontal movement passes the slop, and is larger than the vertical movement at that moment | the row follows the finger 1:1 in an allowed direction. The list stops scrolling for this touch. Under the row, on the side it uncovered, the action's icon and its label appear in the neutral colour |
| armed | the row has moved ≥ 40 % of its width (at least 96 dp) | the action colour fills the uncovered area, and the icon scales up once (100 → 120 %). One haptic tick plays (`HapticFeedbackConstants.GESTURE_THRESHOLD_ACTIVATE`, or `CONTEXT_CLICK` below API 34), once per crossing. Dragging back below the threshold disarms it, with no second tick |
| commit | release while armed; **or** a fling in the allowed direction at ≥ 1000 dp/s that has moved at least 48 dp, armed or not | the row's result, §5 |
| cancel | release while not armed (and not a qualifying fling); a second finger down; the touch cancelled by the system; the list scrolled under the finger | the row springs back to rest (200 ms, decelerate), nothing happens, no snackbar |

Edge rules:

- If vertical movement passes the slop first, the touch is a scroll for
  the rest of its life. It never becomes a swipe.
- Only one row tracks at a time.
- A press that stays within the slop on a link, or on one of the row's
  buttons, is that control's tap. A drag that starts on one passes the
  slop and is the row's swipe, and that control does not fire.
- Long-press and text selection inside the row are not changed by this
  spec.
- A swipe that starts inside the system's gesture inset
  (`WindowInsets.systemGestures`, the back-gesture zone at each edge) is
  the system's and does not track.

## 5. Results

Each result is visible on the row itself and is announced (§7). Every
committed swipe except Retry shows the snackbar (§6).

| kind, direction | on commit | the row afterwards | snackbar text (proposed; words are language-culture's) | Undo does |
|---|---|---|---|---|
| A right, Keep | `Intent::Keep { item, from }` as `actions` offers it | slides back to rest. Its status reads Kept and it becomes kind B | "Kept" | the Unkeep of that kept row |
| A left, Hide for now | removes the row from this session's list (new; §8) | slides out, then the space closes (150 ms) | "Hidden" | puts the row back in the same place |
| B left, Unkeep | **nothing yet**. The Unkeep is raised when the snackbar closes, not on release | slides back to rest. Its status already reads as not kept, so the person sees the result | "No longer kept" | cancels the pending Unkeep. The row reads Kept again. No store call was made |
| C right, Retry now | `Intent::Retry(row)` | slides back to rest. Its status changes as the facade reports it (Sending) | none (nothing to undo) | — |
| C left, Cancel send | **nothing yet**. The Cancel is raised when the snackbar closes | slides out, then the space closes | "Send cancelled" | cancels the pending Cancel. The row is back in place with its status as it now is |

**Why Unkeep and Cancel are deferred.** Both are destructive, and their
undo must survive whatever happens while the snackbar is up. If Unkeep
ran at once and the app were killed, the kept copy would be gone with
no way back. Deferring makes the undo free, and makes a kill fail
safe: the row is still kept, or still pending.

**What closes the snackbar.** Any of these commits a deferred action:

- its timeout;
- the person swiping it away;
- another snackbar replacing it;
- leaving the conversation;
- the app going to the background (`onStop`).

The app going away without `onStop` (killed) commits nothing.

## 6. The snackbar

- At the bottom of the conversation, above the composer, never
  covering the composer. One at a time. A new swipe's snackbar replaces
  the old one, and the old one's deferred action commits as it goes.
- Shown for 6 s, or for `AccessibilityManager.getRecommendedTimeoutMillis(6000,
  FLAG_CONTENT_TEXT | FLAG_CONTENT_CONTROLS)` if that is longer (TalkBack
  users get the time their setting asks for).
- Its Undo is a button with a touch target of at least 48 × 48 dp,
  reachable by TalkBack and by a hardware keyboard.
- An Undo restores exactly the state before the swipe. Two undos in a
  row are not possible, since one snackbar holds one action.

## 7. Equivalents for anyone who cannot swipe

Every swipe is a shortcut. Nothing is reachable only by a gesture.

- **Row buttons.** Each row keeps its action buttons, which are the
  primary control: kind A shows Keep and **Hide** (new). Kind B shows
  Unkeep. Kind C shows Retry and Cancel. A button press gives the same
  result, the same snackbar and the same Undo as the swipe, including
  the deferral of Unkeep and Cancel.
- **Touch targets.** Every row button and the snackbar's Undo have a
  target of at least 48 × 48 dp (Android accessibility, WCAG 2.5.8).
  Today `ActionButton` has a minimum height of 24 px, which fails this.
  This spec raises it on message rows.
- **TalkBack.** A one-finger swipe with TalkBack on is TalkBack's
  navigation and never reaches the row, which is correct. The person
  uses the buttons. Each button's label names the action. Its
  description names the message, its author and time, as the row's
  existing labels do. If the toolkit later offers custom accessibility
  actions, Keep, Hide, Unkeep, Retry and Cancel go on the row as
  well. That is optional, not required for acceptance.
- **Announcement.** Every result in §5, by swipe or button, and every
  Undo, is announced through the existing polite live region, with the
  same words as the snackbar ("Kept", "Hidden", "No longer kept",
  "Send cancelled", and "Restored" for an Undo).
- **Focus.**
  - After Hide or Cancel removes a row, focus goes to the next row's
    first control, or the previous row's if it was the last. Never to
    the top of the list, never lost.
  - After Undo, focus goes to the restored row's first control.
  - After Keep, Unkeep or Retry, focus stays where it was.
- **Reduced motion.** When the system's animator duration scale is 0
  ("Remove animations"):
  - the row still follows the finger, since direct manipulation is not
    animation;
  - spring-back, slide-out, space-closing and icon scaling are instant;
  - the haptic tick still plays.
- **Colour is never the only signal.** The armed state is shown by the
  colour **and** the icon's scale **and** the haptic tick. Before
  arming, the icon has a label beside it, so the direction's meaning
  can be read. Contrast: the icon against its fill ≥ 3:1, and the label
  ≥ 4.5:1 (WCAG 1.4.11, 1.4.3). The colours are brand-comms'. This spec
  names only their roles: neutral, keep, hide or remove, retry.

## 8. Edge cases

- **The row's state changes under the finger.** For example, a pending
  row is accepted by the remote while it is tracking. The swipe is
  re-checked against `actions` at commit. If the action is no longer
  legal, the row springs back and nothing happens.
- **A cancel is pending and the row is accepted.** The row is accepted
  by the remote transport while its Cancel waits on the snackbar:
  - the Cancel is dropped;
  - the row comes back in place with its accepted status;
  - the snackbar is replaced by "Not cancelled: accepted by remote
    transport".

  Never "delivered", "read" or "seen" (`human-client-ui.md` §5).
- **No new send attempt while a Cancel is pending.** The client starts
  no new send attempt of that row while its Cancel waits. An attempt
  already in flight may finish, and then the case above applies. If the
  facade cannot hold back a scheduled retry, rust-ui-dev returns this
  point to ux-designer before building §5's deferral.
- **Hide for now is session-only and changes no retention.**
  - The hidden row is read and not kept, so its durable copy is already
    gone (`human-client-ui.md` §6). Hide removes it from what the view
    shows and from the session's memory once the snackbar closes.
  - It does not come back in this session. A restart would have
    removed it anyway.
  - A message that replies to a hidden one shows the neutral
    unavailable-reply placeholder (§15), as for any message not held.
- **Keep is refused because storage is degraded.** The row stays kind
  A, no snackbar is shown, and the existing storage notice and refusal
  announcement apply.
- **Network denied.** Keep, Hide and Unkeep work. Retry is offered only
  if `actions` offers it, and its result is what the facade reports.
- **Leaving the conversation mid-swipe** cancels the swipe. Leaving it
  with a snackbar up commits that snackbar's deferred action (§5).

## 9. Deliberately left out

- **Like, Accept, Reject.** Reactions and contact requests are features
  the app does not have. The owner asked for them to go to architect-cto
  as a separate product question. A reaction would tell the sender a
  message was read, which `human-client-ui.md` §6 says is never sent in
  v1. A contact request would need a trust flow, and trust changes are
  settings-only (§8).
- **Swipes on the Chats list rows.** These are waiting on the owner. A
  proposal has gone to them, and that part gets its own section or file
  once they answer.
- **Desktop.** The Hide button and the snackbar live in the shared views
  and will appear on desktop. This spec is reviewed on Android only.
- **The words.** Every label above is a proposal. language-culture
  writes them in `labels.rs`.
- **The colours and icons.** These are brand-comms'. The roles are
  named in §7.

## 10. Open questions

1. **Right-to-left mirroring.** Android convention mirrors start and
   end in an RTL layout. The owner's words name physical right and left.
   This version keeps physical directions until the owner rules (asked
   2026-10-16).

## 11. Acceptance — what a reviewer checks on a device

Run on an Android phone or emulator with a debug build, once with
TalkBack off and once with it on, and once with "Remove animations" on.

1. On a kind-A row:
   - a slow drag right past 40 % of its width shows the Keep icon and
     label, changes the fill and ticks once;
   - releasing makes the row read Kept, with a "Kept" snackbar;
   - Undo makes it unkept again.
2. On a kind-A row, a drag right to 30 % and back gives no tick on the
   way back and nothing on release. A drag to 30 % and released also
   does nothing.
3. A quick flick right of about 60 dp on a kind-A row commits Keep.
4. On a kind-A row:
   - a drag left past the threshold hides it and shows "Hidden";
   - Undo puts it back in the same place;
   - without Undo, after the snackbar closes, it does not come back in
     this session.
5. On a kind-B row:
   - a drag right does not move the row;
   - a drag left commits and shows "No longer kept";
   - Undo within the timeout leaves it kept. Check by restarting the
     app: the message is still there.
6. On a kind-B row, a drag left followed by killing the app from Recents
   before the snackbar closes leaves the message kept after restart.
7. On a kind-B row, a drag left followed by letting the snackbar time
   out, then restarting the app: the message is gone.
8. On a kind-C row (no daemon or network, so it stays pending):
   - a drag right changes its status to Sending and shows no snackbar;
   - a drag left removes it and shows "Send cancelled";
   - Undo restores it.
9. On a kind-C row, letting the snackbar time out, then restarting: the
   message is not pending and is never sent.
10. On a kind-D row, neither direction moves the row.
11. A vertical scroll that starts on any row, drifting sideways less
    than it moves vertically, scrolls and never swipes.
12. With TalkBack on:
    - every action in 1–9 can be done with the row's buttons alone;
    - each result is announced;
    - after Hide or Cancel, focus lands on the next row.
13. Every row button and the snackbar's Undo measure at least 48 × 48 dp
    (Layout Inspector or the Accessibility Scanner).
14. With "Remove animations" on, the row still follows the finger, but
    spring-back and collapse are instant.
15. A swipe started at a screen edge, inside the back-gesture zone,
    performs the system back gesture and does not move the row.
16. The ui-model tests show:
    - which swipes are offered is computed from `actions` and the
      Hide rule, for kinds A–D;
    - a deferred Unkeep or Cancel raises its intent only when the
      snackbar closes;
    - an Undo raises nothing.
