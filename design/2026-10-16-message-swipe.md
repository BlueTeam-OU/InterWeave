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
`UiModel::actions(key)`. Hide for now is offered on every kind-A row,
including one whose session copy is gone (`UiModel::copy_gone`). There
`actions` is empty, so Keep is not offered, but hiding needs no copy:
such a row offers the left swipe and the Hide button, and no right
swipe.
The swipe never decides legality itself. When the model's answer
changes, the swipe changes with it.

- A direction with no action **does not move**. The row does not
  follow the finger that way and nothing is revealed. A row that
  slides and then does nothing tells the person something happened.
- Kind D, and any other row whose actions are empty, does not move
  either way. A vertical scroll that starts on it scrolls as usual.
- **In a right-to-left layout the directions mirror**, the platform
  convention. The owner ruled this on 2026-10-16: "mirror, the platform
  convention". In an RTL locale Keep and Retry are a swipe to the
  **left**, and Hide, Unkeep and Cancel a swipe to the **right**. Every
  "right" and "left" in this file is the left-to-right case. The
  icons, the side they show on, and the snackbar follow the layout
  direction.

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
| commit | release while armed; **or** a fling in the allowed direction at ≥ 1000 dp/s that has moved at least 48 dp, armed or not. The fling's speed is the finger's average over the last 100 ms before release (at least three position samples); with fewer samples it is not a fling | the row's result, §5 |
| cancel | release while not armed (and not a qualifying fling); a second finger down; the touch cancelled by the system; the list scrolled under the finger | the row springs back to rest (200 ms, decelerate), nothing happens, no snackbar |

Edge rules:

- If vertical movement passes the slop first, the touch is a scroll for
  the rest of its life. It never becomes a swipe.
- The tracking rule is this one, not the toolkit's. Slint 1.18.1's
  `SwipeGestureHandler` starts at 8 px when dx > |dy| / 2, so a drag
  45–63° off horizontal would swipe. Here it scrolls, since this rule
  tracks only below 45°. A row tracks
  its own touch with this rule and the system slop (ruled 2026-10-16
  on rust-ui-dev-01's question).
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
| B left, Unkeep | `Intent::Unkeep(row)`, at once | slides back to rest. Its status reads not kept, and it becomes kind A | "No longer kept" | the Keep `actions` now offers on it (`from` is the kept row the Unkeep released, whose content this session holds). The row reads Kept again |
| C right, Retry now | `Intent::Retry(row)` | slides back to rest. Its status changes as the facade reports it (Sending) | none (nothing to undo) | — |
| C left, Cancel send | `Intent::Cancel(row)`, at once | slides out, then the space closes | "Send cancelled" | restores the cancelled pending record itself: everything §8 lists unchanged (both ids, destination, media type, stored payload bytes, attempt count), never a re-serialised copy. It is shown in its old place and sent as a retry of that record (§8) |

**The rule: after any interruption, the state is what the person last
saw confirmed.** Every action runs on commit. Undo is the reverse
action, not a cancelled delay. If the app is killed while a snackbar is
up, the message is as the snackbar said: no longer kept, or cancelled
and not sent again. An earlier draft of this spec deferred Unkeep and
Cancel until the snackbar closed. That fails the rule: a kill left an
unkept message kept, and let a cancelled one be sent after restart,
with nothing saying so (found by rust-ui-dev-01, 2026-10-16).

**What closes the snackbar:**

- its timeout;
- the person swiping it away;
- another snackbar replacing it;
- leaving the conversation;
- the app going to the background or being killed.

Once it closes, the Undo is gone. The action itself has already
happened.

## 6. The snackbar

- At the bottom of the conversation, above the composer, never
  covering the composer. One at a time. A new swipe's snackbar replaces
  the old one, and the old one's Undo goes with it.
- Shown for 6 s, or for `AccessibilityManager.getRecommendedTimeoutMillis(6000,
  FLAG_CONTENT_TEXT | FLAG_CONTENT_CONTROLS)` if that is longer (TalkBack
  users get the time their setting asks for). It is never shown for more
  than 2 minutes, the longest the platform setting offers.
- Its Undo is a button with a touch target of at least 48 × 48 dp,
  reachable by TalkBack and by a hardware keyboard.
- An Undo restores exactly the state before the swipe. Two undos in a
  row are not possible, since one snackbar holds one action.

## 7. Equivalents for anyone who cannot swipe

Every swipe is a shortcut. Nothing is reachable only by a gesture.

- **Row buttons.** Each row keeps its action buttons, which are the
  primary control: kind A shows Keep and **Hide** (new). Kind B shows
  Unkeep. Kind C shows Retry and Cancel. A button press gives the same
  result, the same snackbar and the same Undo as the swipe.
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
  can be read.
- **Colour and icon roles.** These are new entries that rust-ui-dev
  adds to `global Palette` in `crates/human/ui-slint/ui/app.slint`. The
  palette and the icons are rust-ui-dev's until a brand-comms holder is
  placed in this repository (the remits, InterWeave#282). Each must
  meet its contrast in every theme the app draws:

  | role | what it is | contrast it must meet |
  |---|---|---|
  | `swipe-neutral` | the fill behind a row before arming | ≥ 3:1 against the row's own background, so the uncovered area reads as one (WCAG 1.4.11) |
  | `swipe-keep` | the armed fill for Keep | — |
  | `swipe-remove` | the armed fill for Hide, Unkeep, Cancel and the conversation hide | — |
  | `swipe-retry` | the armed fill for Retry | — |
  | `swipe-on-fill` | the icon and label drawn on any of the four fills | icon ≥ 3:1 (WCAG 1.4.11) and label ≥ 4.5:1 (WCAG 1.4.3) against each fill |

  The armed fills need no contrast against each other or against
  `swipe-neutral`, because colour is not the only signal of arming
  (above).

  Icons, one per action: Keep, Hide, Unkeep, Retry, Cancel, and the
  conversation hide (Hide's icon may serve). The shapes are
  rust-ui-dev's to choose, from a source whose licence the repository
  accepts. Each is decorative beside its label, so it carries no
  accessible name of its own.

## 8. Edge cases

- **The row's state changes under the finger.** For example, a pending
  row is accepted by the remote while it is tracking. The swipe is
  re-checked against `actions` at commit. If the action is no longer
  legal, the row springs back and nothing happens.
- **Cancel of an attempt already in flight.** The client cannot know
  whether that attempt reached the receiver. The facade reports a
  cancelled row as `Cancelled { may_have_reached }` and nothing after
  it (TRANSPORT.md, Cancellation). So:
  - the snackbar text follows the existing status label: "Send
    cancelled" when `may_have_reached` is false, and the
    `CancelledMayHaveBeenReceived` label's words when it is true;
  - there is no "accepted while cancelling" case. An earlier draft
    had one, but nothing reports it (architect-cto, InterWeave#284).

  The status words are never "delivered", "read" or "seen"
  (`human-client-ui.md` §5).
- **Undo of Cancel restores the record (a new row id, the same message).** architect-cto
  ruled this on 2026-10-16, and writes it into `human-client-ui.md` §6
  and §13 in InterWeave#284:
  - the undone cancel restores the pending record unchanged: its app
    message id, its transport message id, its destination selector, its
    media type, its stored payload bytes and its attempt count. It is
    never a re-serialised or re-identified copy. A new row id is
    expected. The attempt count matters because after a restart the
    client rebuilds `may_have_reached` from it (RETENTION.md §2,
    conformance case 16, InterWeave#284). A restore with no attempts
    would wrongly say the cancelled attempt never reached the
    receiver;
  - the receiver dedups a direct message by its transport message id.
    If an attempt was admitted at the receiver before Cancel ran and
    only its answer was lost, the undone send is answered from the
    receiver's dedup entry while that entry exists. The entry is bounded
    by its TTL and by eviction, and never survives the receiver's
    restart (ADR-0019). The restored record may go out long after the
    snackbar, and once the entry is gone the receiver may present the
    message again, as for any retry. The client never claims
    exactly-once (RETENTION.md §2);
  - what the session holds for that Undo is the record (both ids and
    the bytes), not the rendered text.
  If the facade or store cannot restore a cancelled record under its
  old ids, rust-ui-dev returns this point to ux-designer before building
  the Cancel swipe.

  **The Undo of Cancel waits on InterWeave#284 landing on main.** That
  PR writes the restore into RETENTION.md §2 (architect-cto, 28efec49):
  the undone cancel enters the diagram again at Send for the same
  message, with the cancelled record's identities, durable before any
  attempt. It adds no new edge or state, and conformance case 16 tests
  it. The restore is offered only while the session still holds that
  record. When it does not, Cancel's snackbar shows no Undo, as for
  Unkeep after `copy_gone`. The cancelled row itself stays terminal
  (`OutboundStatus::is_terminal`): the restore is a new row, so
  nothing about the old one changes. The Cancel swipe and button
  themselves, without an Undo, do not wait.
- **Unkeep when the session no longer holds the content.** If the
  session's copy of the message is past its bound (`UiModel::copy_gone`),
  Keep is not offered after the Unkeep, so there is nothing for an Undo
  to restore. The snackbar then shows "No longer kept" with **no** Undo,
  never an Undo that fails. A missing Undo is honest. A failing one
  breaks the promise the button makes (ruled 2026-10-16 on
  rust-ui-dev-01's question).
- **Cancel refused by the store** (storage degraded or full;
  `TransportClient::cancel` answers `StorageUnavailable`, and the row
  stays pending):
  - the row springs back to its place, still pending, and no snackbar
    is shown;
  - the existing storage notice and refusal announcement apply;
  - focus stays where it was.
- **Undo of Cancel when the store refuses the restore** (storage
  degraded or full, RETENTION.md §4):
  - the row stays cancelled, and nothing is sent: a send never goes out
    without its durable pending record (RETENTION.md §2);
  - the snackbar closes, and the existing storage notice and refusal
    announcement apply;

  If Cancel succeeds while storage is already degraded, its snackbar
  shows no Undo, since a restore could not be written. A missing Undo
  is honest; a failing one is not (as for Unkeep above).
- **Undo of Unkeep when Keep is refused** (storage degraded): the row
  stays not kept, the snackbar closes, and the existing storage notice
  and refusal announcement apply.
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
  with a snackbar up closes the snackbar (§5).

## 9. Deliberately left out

- **Like, Accept, Reject.** Reactions and contact requests are features
  the app does not have. The owner sent them to architect-cto as a
  separate product question, and architect-cto answered on 2026-10-16:
  - **Reactions** are not in the product now. They would need their
    own record, as an explicit message the person sends, never a
    receipt sent on read. Until that record exists, no swipe or button
    sends one.
  - **Contact requests** are refused. A remote message may never ask
    for a trust change (`human-client-ui.md` §4, §8, §10).
- **Desktop.** The Hide button and the snackbar live in the shared views
  and will appear on desktop. This spec is reviewed on Android only.
- **The words.** Every label above is a proposal. language-culture
  writes them in `labels.rs`.
- **The colour values and icon shapes.** rust-ui-dev chooses them,
  against the roles and contrasts named in §7, until a brand-comms
  holder is placed here.

## 10. Swipe on a conversation row (Chats list)

The owner's ruling, 2026-10-16: "Left = hide the conversation for now,
offered only when it holds nothing unread, pending or kept, with a
snackbar Undo; right = nothing."

- **Offered** only when every message in the conversation is a read,
  unkept inbound message (kind A) or an accepted outbound one (kind D).
  If any row is unread, kept or pending, the row does not move either
  way. A right swipe never moves the row.
- **Gesture** as §4: the same slop, angle rule, threshold, fling, haptic
  tick and edge-inset rule. The revealed icon and label say "Hide".
- **On commit** the conversation leaves the list and the space closes.
  Its messages are dropped from the session as Hide for now drops one
  (§8): nothing is written to the store, and none of them can be kept
  any more. The snackbar reads "Conversation hidden" (proposed), with
  Undo.
- **Undo** puts the conversation back in its place with its messages.
- **A new message** into a hidden conversation brings it back to the
  list, holding only that message.
- **A new message while the hide's snackbar is still up.** The
  arrival wins, and nothing is lost: the conversation comes back in its
  place with all its messages and the new one, as an Undo would bring
  it. The snackbar closes, and the arrival is announced as any new
  message is. A new message is a reason to see the conversation, and
  merging a later Undo into a list that already holds the new message
  would risk a duplicate row.
- **Equivalent without the gesture.** The conversation's own screen has
  a "Hide conversation" control in its header, shown only when the
  conversation is offered. It returns to the list and shows the same
  snackbar there. The Chats row itself gains no button. TalkBack users
  reach the control by opening the conversation. Opening it reads its
  unread rows, but a conversation offered for hiding has none.
- **Focus** after a hide goes to the next conversation row, or the
  previous one if it was the last. After Undo, it goes to the restored
  row.
- Announced as §7, with the snackbar's words.

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
   - Undo within the timeout makes it kept again. Check by restarting
     the app: the message is still there.
6. On a kind-B row, a drag left followed by killing the app from Recents
   before the snackbar closes: after restart the message is gone, as
   "No longer kept" said.
7. On a kind-B row, a drag left, then Undo, then killing the app: after
   restart the message is still kept.
8. On a kind-C row (no daemon or network, so it stays pending):
   - a drag right changes its status to Sending and shows no snackbar;
   - a drag left removes it and shows "Send cancelled";
   - Undo restores it in place, pending, with the same app message id
     and transport message id (the diagnostics view shows both).
9. On a kind-C row, a drag left followed by killing the app before the
   snackbar closes: after restart the message is not pending, and this
   client never sends it again, even once a network is available.
10. On a kind-D row, neither direction moves the row.
11. A vertical scroll that starts on any row, drifting sideways less
    than it moves vertically, scrolls and never swipes. That includes
    a drag 55–60° off horizontal, which Slint's own swipe handler
    would take.
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
16. In an RTL locale (Settings › Developer options › Force RTL layout
    direction, or an RTL language), a swipe to the left on a kind-A row
    commits Keep, and a swipe to the right hides it.
17. On the Chats list, a conversation holding only read, unkept or
    accepted messages:
    - a drag left hides it and shows "Conversation hidden";
    - Undo puts it back;
    - a drag right does not move it.

    A conversation holding an unread, kept or pending message moves in
    neither direction. Its screen's "Hide conversation" control does
    the same as the swipe and is shown only when the swipe is offered.
18. The ui-model tests show:
    - which swipes are offered is computed from `actions` and the
      Hide rule, for kinds A–D, and for a conversation from §10's rule;
    - Unkeep and Cancel raise their intent at commit, not when the
      snackbar closes;
    - the Undo of Keep raises Unkeep, the Undo of Unkeep raises Keep
      with the released row as `from`, and the Undo of Cancel restores
      the cancelled record with its ids, destination, media type,
      stored bytes and attempt count unchanged;
    - the Undo of Hide raises nothing.
