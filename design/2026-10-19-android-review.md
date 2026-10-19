# Android app review: look and feel against the messengers people use

Status: frame written 2026-10-19; the device run waits for the phone
link. No finding below is recorded until it has been seen on the
device. The ranked findings and the re-review are added to this file.

Owner: ux-designer. It is the review that plan §20's exit gate, clause
(j), asks for (architect-cto-01's REQUEST of 2026-10-19). Findings go to
rust-ui-dev-01, who builds the app.

## 1. What the owner asked, and how it is read

The owner, on 2026-10-19, relayed by architect-cto-01: the Android app's
look and feel are far below standard. It is to be brought to the level
of the mainstream Android messengers a person already uses. The owner
named Telegram and WhatsApp as the reference.

Read as: a person who uses those apps every day should meet the same
conventions here. That covers layout, density, type, colour, motion,
feedback, the gestures and keys, the empty and failed states, and how
the app behaves when it is left and resumed. They should find nothing
that looks unfinished. The reference is those conventions. Their
feature set is not the reference. Where a feature of theirs meets this
product's rules, the gap is named as a product question, never as a
look finding. Examples are read receipts (`human-client-ui.md` §6: read
state is never sent in v1), reactions (not in v1; they need their own
record) and typing indicators.

## 2. The standard each finding is measured against

A finding names one of these, or the line of a spec under `design/`
that it misses:

- **S1. Material 3** (m3.material.io): the top app bar, lists and list
  items, the message composer as a text field with its trailing
  action, menus, snackbars, dialogs, state layers, motion durations and
  easing, and the type scale.
- **S2. Android's platform behaviour** (developer.android.com):
  edge-to-edge drawing under the system bars with insets kept clear;
  the IME resizing the screen so the composer stays above the
  keyboard; predictive and gesture Back; the app's state surviving
  rotation, background and process death; notifications; and system
  settings the app follows (font scale, dark theme, 12/24-hour, Remove
  animations).
- **S3. The messenger conventions** a person brings from the reference
  apps, where they agree with each other and with S1:
  - the Chats list ordered by latest activity, with name, preview,
    time and an unread count;
  - the conversation newest at the bottom, opening at the first unread
    message or the end;
  - own messages on the end side;
  - a clear status on every outgoing message;
  - a composer pinned at the bottom whose Send appears active only with
    text;
  - long-press for the message's actions;
  - a way back to the newest message after scrolling up.
- **S4. Usability heuristics** (Nielsen's ten), named per finding:
  visibility of status; match with the real world; user control and
  freedom; consistency and standards; error prevention; recognition
  rather than recall; flexibility; aesthetic and minimalist design;
  recovery from errors; help.
- **S5. Accessibility** (WCAG 2.2 AA and Android's own guidance):
  touch targets of 48 dp, contrast, text at 200 %, focus, reduced
  motion. TalkBack itself is carried to the release gate (plan §20,
  "Decided 2026-10-18"), so it yields no finding here.
- **The specs**: `design/2026-10-16-android-look.md` §11 and
  `design/2026-10-16-message-swipe.md` §11, read on main once #300 has
  merged.

## 3. Method, and what it is not

A heuristic and specification review of the app as built on the Pixel
9a. It is run by the designer through `.agent-fabric/bin/phone-run`
(input, screenshots, screen recordings, `dumpsys`). It is walked as a
person who has used the reference apps and has none of the team's
knowledge.

It is not a test with people. Two things stay a person's check, and
this review does not rank them from a script:
- the feel of a gesture (both remits);
- TalkBack (carried).
If the owner wants people to try the tasks in §4, those tasks and their
success criteria are written for that.

Each finding records:
- what was seen;
- the build (commit and APK sha);
- the steps to reproduce;
- the standard or spec line it misses;
- its rank;
- for rust-ui-dev, what has to change, as an observable result and
  never as an implementation.

What was seen is kept apart from what it is judged to mean.

Every task runs:
- in dark and light;
- at 100 % and 200 % font;
- with real content: long names, a long multi-line message, a link, a
  quote, a conversation of more than one screen, and several days of
  history;
- under an RTL locale for layout;
- with Remove animations on for motion.

## 4. The tasks walked

Each task starts from the app's own state on the phone and ends when the
person would say it is done.

1. Open the app cold and understand where you are: the first screen,
   what is loading, what is empty.
2. Find a conversation with new messages and read them.
3. Reply, and follow the message until it is sent. Then send while the
   peer is unreachable, see the failure, and retry or cancel.
4. Keep a message, dismiss one, and undo each, by swipe and by button.
5. Hide a conversation, and undo it.
6. Start a conversation with someone new, and find where trust is
   managed.
7. See whether you are online, and what to do when you are not.
8. Leave the app mid-draft and come back. Then rotate the phone, open
   the keyboard, and press Back from each screen.
9. Scroll a long conversation up, and get back to the newest message.
10. Open a link, see a quote, and read the source of a message.

A task the app cannot do at all (6 may be one) is a finding of its own,
under §6. It is never folded into the look findings.

## 5. Ranking

- **Rank 1:** what a person notices first, or what makes the app look
  unfinished or untrustworthy, in a task above. Clause (j) binds these
  to the close: rust-ui-dev builds them and this review re-checks them.
- **Rank 2:** noticed in use, but the task still succeeds as expected.
- **Rank 3:** polish, or a rare state.

Ranks 2 and 3 are built, or carried by name to the owner's release
gate.

## 6. Findings

Run 1, 2026-10-19, on the Pixel 9a:
- the build was main ffce28df's debug build, APK sha256 beginning
  0d1bcbec, installed by rust-ui-dev-01 with the app's data kept;
- the phone was at 1080 x 2424, 420 dpi, font scale 1.0, dark theme.

The run stopped partway through. A scripted tap of mine most likely
switched the phone's Wireless debugging off (§3, method). Tasks 1, 2,
8 and part of 6 were walked; the rest wait for the next run.

The ranks are provisional until every task in §4 is walked. What was
seen is kept apart from what it is judged against.

### Rank 1 (provisional)

- **F1. Back leaves the app from an open conversation and from the
  Trust screen.**
  - Seen: one Back from an open conversation shows the launcher. The
    focused window was `com.android.launcher3`, reproduced in a clean
    run. The same holds from Trust: the previous task came up instead
    of Chats.
  - Against: S2 (Back returns through the app's own screens), and S4
    (consistency and standards). A person who presses Back to return
    to the list leaves the app instead.
- **F2. Every message shows its actions as permanent outlined buttons
  under the bubble.**
  - Seen: "Stop keeping" and "Show source" under each kept message;
    "Send again", "Cancel" and "Show source" under each pending one.
    On a screen of four messages the buttons take more room than the
    messages do.
  - Against: S3 (actions behind long-press, and the swipe the swipe
    spec already gives), and S4 (aesthetic and minimalist design).
  - Not yet checked: what long-press does today.
- **F3. The composer draws wrongly.**
  - Seen: unfocused, it is a tall pill about 250 px high, with a
    lighter inner pill drawn over the "Message" placeholder. After Back
    hides the keyboard, the field stays focused and tall, with a teal
    ring and a text-selection handle showing.
  - Against: S1 (text field), and `design/2026-10-16-android-look.md`
    §6 (the composer).
- **F4. The Trust screen has none of the Android look.**
  - Seen: a boxed "Back to conversations" text button instead of an app
    bar, plain square buttons, and PeerIds clipped mid-string with no
    ellipsis. A "Remove trust" button sits on every row.
  - Against: S1, and S4 (consistency: it looks like another app).
  - Not yet checked: what "Remove trust" asks before acting
    (`human-client-ui.md` §8 requires a confirmation showing the exact
    PeerId).

### Rank 2 (provisional)

- **F5. The focus ring is drawn on touch focus.**
  - Seen: tapping the composer draws the 3 px teal ring.
  - Against: S1, where the focus indicator is for keyboard focus only.
    The look spec's §7 ring was meant for keyboard and switch access.
- **F6. Conversation titles are a PeerId fragment with "/ human".**
  - Seen: "...x6gY6Eri / human", in the Chats rows and in the app bar.
  - Against: S4 (match with the real world). "human" is an endpoint
    name, an implementation concept. The names a person would give
    (contacts, `human-client-ui.md` §3) are a missing flow; see below.
- **F7. The app bar's subtitle shows the network state where
  messengers show the contact's.**
  - Seen: "Online, some peers might not reach you" under the peer's
    name.
  - Against: S4, because it reads as the peer's state.

### Not the app's

- **N1.** A system toast, "App has unexpected permission:
  OTHER_SENSORS, ACCESS_LOCAL_NETWORK", shows over the app when it
  opens. It is likely the debug build's. rust-ui-dev to confirm that a
  release build does not show it.

### Flows the reference apps have and this app lacks

Each such flow is named here, with the product question it raises and
whose that question is. The owner decides whether it belongs to this
stage or a later one.

- **M1. Starting a conversation.** The Chats screen has no visible way
  to start one: no button, no menu entry. Whether one exists elsewhere
  is not yet checked. If none does, the product question (how a person
  reaches a new peer) is architect-cto's.
- **M2. Contact names.** Conversations are titled by a PeerId fragment
  (F6). `human-client-ui.md` §3 has contacts as named application
  metadata; the app has no screen for them.

## 7. Re-review

To be written after rust-ui-dev's fixes, on the build that carries them:
what passed, and what is carried.
