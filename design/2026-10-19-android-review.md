# Android app review: look and feel against the messengers people use

Status: 2026-10-19, two device runs on main ffce28df; ranked findings
in §6, for rust-ui-dev-01 to build. Tasks 3 (failure), 7 (offline), 9
and 10 wait for what §6 names. The re-review is added in §7.

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

Both runs used the same build and phone:
- main ffce28df's debug build, APK sha256 beginning 0d1bcbec, installed
  by rust-ui-dev-01 with the app's data kept (re-read from the phone
  before run 2);
- a Pixel 9a at 1080 x 2424, 420 dpi, font scale 1.0, dark theme,
  unless a finding names another setting.

Run 1 (2026-10-19, morning) stopped partway: a scripted tap of mine
most likely switched the phone's Wireless debugging off. From run 2
(2026-10-19, 10:50-11:15 phone time) every scripted input first checks
that InterWeave has the window focus and stops otherwise; every
setting a run changed (theme, font scale, rotation, the app's locale)
was put back and read back at the end.

Walked: tasks 1, 2, 4 (by button and by swipe, each with Undo), 5
(the header's "Hide conversation" and its Undo; the row's left swipe
where it is not offered), 6 (as far as the app goes, see M1), 8
(rotation with a draft, the keyboard, the app's own back arrow), and
part of 3 and 7 (the pending states and the connectivity line as they
stand). Light theme, 200 % font and an RTL locale were checked on
Chats and a conversation.

Not walked, and why:
- 3's failure path and 7's offline state: making the peer or the phone
  unreachable also cuts the phone link this review runs over. They
  need a person at the phone, or a peer the team can stop.
- 9 and 10: the phone holds two conversations of one and four short
  messages, with no link, quote or multi-day history. They need seeded
  content (asked of rust-ui-dev with the findings).
- Remove animations: the motion the specs add is the swipe and the
  snackbar, whose feel is a person's check (§3).

What was seen is kept apart from what it is judged against. The ranks
are final for the tasks walked; a task walked later can add findings
but does not re-rank these.

### Rank 1

- **F1. Back leaves the app from an open conversation and from the
  Trust screen.**
  - Seen: one system Back from an open conversation shows the
    launcher, or the task that was open before the app (focused window
    `com.android.launcher3`, reproduced in a clean run). The same holds
    from Trust. The app's own back arrow in the conversation's app bar
    does return to Chats (run 2).
  - Against: S2 (Back returns through the app's own screens), and S4
    (consistency and standards). A person who presses Back to return
    to the list leaves the app instead.
- **F2. Every message shows its actions as permanent outlined buttons
  under the bubble, and long-press does nothing.**
  - Seen: "Stop keeping" and "Show source" under each kept message;
    "Keep", "Dismiss" and "Show source" under each read, unkept one;
    "Send again", "Cancel" and "Show source" under each pending one. On
    a screen of four messages the buttons take more room than the
    messages do; at 200 % font each button takes a line of its own.
    A 900 ms press on a bubble shows no menu, no selection and no
    change (run 2).
  - Against: S3 (actions behind long-press, and the swipe the swipe
    spec already gives), and S4 (aesthetic and minimalist design).
  - The swipe itself behaves as specified (run 2, swipe spec §4 and
    §5): the row tracks the finger over the neutral fill with its
    label, turns to the armed fill past the threshold, and on release
    commits with the snackbar; Undo restores the row.
  - What has to change, as seen: a message at rest shows its bubble
    and its status line only. Its actions are reached by long-press (a
    menu or an action bar, as the reference apps do) and by the swipe;
    the swipe spec §7's equivalents for anyone who cannot swipe stay
    reachable from that long-press, without a gesture beyond it.
  - Specified 2026-10-19 on rust-ui-dev-01's proposal (QUESTION
    01a1533b-4905): `design/2026-10-16-message-swipe.md` §4 and §7 and
    `design/2026-10-16-android-look.md` §6 are revised in this PR. A
    Material menu anchored to the bubble, opened by long-press (the
    platform's timeout, one haptic tick), Enter, the Menu key,
    Shift+F10, a secondary click on the desktop, and the bubble's
    accessible default action; the bubble is the row's one focus stop
    besides its links; the desktop shares the interaction.
- **F3. The composer draws wrongly, and its height follows the free
  space on the screen.**
  - Seen: unfocused, it is a tall pill about 250 px high, with a
    lighter inner pill drawn over the "Message" placeholder. In a
    conversation of one message it swells to a shape about 700 px tall
    (run 2). When a snackbar appears it shrinks by the snackbar's
    height, and grows back when the snackbar goes. After Back hides the
    keyboard, the field stays focused and tall, with a teal ring and a
    text-selection handle showing. At 200 % font it is one line high,
    as it should be at every size.
  - Against: S1 (text field), S3 (a composer pinned at the bottom), and
    `design/2026-10-16-android-look.md` §6 (the composer).
- **F4. The Trust screen has none of the Android look, and its
  confirmation pushes the screen past its right edge.**
  - Seen: a boxed "Back to conversations" text button instead of an app
    bar, plain square buttons, and PeerIds clipped mid-string with no
    ellipsis. A "Remove trust" button sits on every row. Remove trust
    does ask first, as `human-client-ui.md` §8 requires, with "Do not
    change" holding focus, and "Do not change" leaves the list as it
    was (run 2). But while the confirmation is up, the whole screen is
    laid out wider than the phone: the explanation, the Remove trust
    buttons and the confirmation's own text are cut at the right edge,
    including the PeerId it asks about, so the person cannot read
    which peer they are about to remove.
  - Against: S1, S4 (consistency: it looks like another app; error
    prevention: the consequence is not readable before commitment),
    and `human-client-ui.md` §8 (the confirmation shows the exact
    PeerId).
  - The overflow is fixed on rust-ui-dev-01's branch (4f784a1e): the
    PeerId breaks whole at the edge. The look below is the Trust
    screen's spec, since `design/2026-10-16-android-look.md` covers
    only Chats and a conversation; it takes rust-ui-dev-01's proposal
    (QUESTION 01a15315-4f61) with the changes marked.
  - What has to change, as seen:
    1. A top app bar as a conversation's: the back arrow (labelled as
       "Back to conversations" is today) and the screen's title. It
       replaces the boxed button and the "Trust" heading. The title's
       words are language-culture's; until they say otherwise it is
       today's "Trust".
    2. Every PeerId (this profile's and each row's) wraps whole over
       as many lines as it needs, never cut or ellipsized, and stays
       selectable to copy. Monospace, so that characters a person
       compares line up. No spaces or grouping are inserted: the
       copied text is the canonical form. This differs from the
       identity flow's groups of four (identity design §5) on
       purpose: there the PeerId is not selectable and its "Copy
       PeerId" button gives the canonical string (identity §5), so
       the spacing never reaches the clipboard; here the
       person copies by selecting the text itself, and any spacing
       would be copied with it.
    3. The field is a Material outlined text field labelled "PeerId to
       trust", with its error ("not a PeerId") under it, not beside
       it. "Trust this peer" is a filled button beside the field when
       both fit at the current font size, and under it when they do
       not (at 200 % they do not).
    4. Each trusted peer is a list item: the PeerId, then its origin
       ("Added here", "From this profile's configuration") as the
       supporting line. "Remove trust" stays on every row as a text
       button in the error colour, with a 48 dp target. It is not
       moved behind a long-press: this is a settings list with one
       action per row, the one place trust is taken back, and F2's
       long-press is for the density of a conversation.
    5. Changed from the proposal: the confirmation is a Material
       dialog over a scrim, not a notice at the foot of the screen,
       which today opens far from the row that was tapped. It holds
       the question, the exact PeerId (2 above) and what follows
       (connections close now), and two text buttons: "Do not change"
       and the confirming action, which names the act ("Remove trust")
       rather than "Confirm"; the words are language-culture's.
       "Do not change" has focus when it opens; Back and a tap on the
       scrim are "Do not change". After either, focus returns to the
       row's Remove trust button, or, after a removal, to the next
       row, else the previous, else the "Trusted peers" heading.
       If Slint makes a dialog costly, the notice may stay, opened
       directly under the row it is about, with the same contents,
       focus and Back; say which before building.
  - Acceptance on the phone: at 100 % and 200 % font, in both themes,
    no text on Trust reaches past the screen's edge, with and without
    the confirmation up; every PeerId on the screen can be read whole;
    Back from Trust returns to Chats.
- **F8. In light theme the status bar's icons are white on the app's
  white top.**
  - Seen: with the phone in light theme, the clock, signal, Wi-Fi and
    battery icons are drawn white over the app's light background and
    cannot be read, on Chats and in a conversation (run 2).
  - Against: S2 (edge-to-edge: the app sets the system bars' icon
    appearance to match the surface under them), and S5 (contrast).
- **F9. In landscape the open keyboard covers the composer and every
  message.**
  - Seen: rotated with a draft in the composer, the app shows the list
    and the conversation side by side; the conversation keeps the
    draft (good). With the keyboard open, only the conversation's app
    bar and the composer's top edge are above it: no message and none
    of the typed text can be seen (run 2).
  - Against: S2 (the IME resizes the screen so the composer stays
    above the keyboard; in landscape Android's own answer is the
    full-screen editor when the space is too small).
  - The full-screen editor is not available: Slint 1.18.1's Android
    backend sets `IME_FLAG_NO_EXTRACT_UI` whatever the app asks
    (rust-ui-dev-01, QUESTION 01a15310-3d0c). A patched backend would
    be architect-cto's decision and would trade the app's look for
    Android's editor, so the answer is the app's own layout.
  - What has to change, as seen, while the keyboard is up. Let *H* be
    the height above the keyboard, *C* the composer's height at the
    current font size, *B* the app bar's height, and *M* = 120 dp, the
    least message area worth showing (about two one-line bubbles at
    100 % font):
    1. if the window shows the list beside the conversation, and the
       conversation pane would leave the messages less than *M*, the
       list steps aside and the conversation takes the whole width;
    2. with *N* the network notice's height (0 when none shows): if
       *H* − *C* − *B* − *N* is still less than *M*, the network notice
       steps aside; if *H* − *C* − *B* is then still less than *M*,
       the app bar steps aside too;
    3. the composer is never covered by the keyboard, and the typed
       text is always in view;
    4. when the keyboard closes, everything that stepped aside comes
       back, at once, with no other change to the screen.
    Nothing moves focus: it stays in the composer throughout. The app
    bar stepping aside is acceptable because the person opened this
    conversation and is typing in it; the peer's name comes back the
    moment the keyboard closes. Back with the keyboard up closes the
    keyboard first, as the platform does.
    5. Whatever space the messages keep, the list stays anchored at
       its newest message: shrinking it never shows older messages or
       the date line in place of the newest.
  - Acceptance on the phone: in landscape with the keyboard up, at
    100 % and 200 % font, the composer's whole height and the draft
    are above the keyboard, and the space left to the messages shows
    the newest message's end. Where that space is less than *M* (a
    phone in landscape can leave less, since nothing more can step
    aside), it shows as much of the newest message as fits.
    (Revised 2026-10-19 at the re-review: the first acceptance
    promised the whole newest message, which the Pixel's landscape
    height cannot give; step 5 was implied and is now written.)

### Rank 2

- **F5. The focus ring is drawn on touch.**
  - Seen: the 3 px teal ring after a tap on the composer, on the ⋮
    button after its menu closes, on the menu's first item, on the
    Chats row a person came back from, on the button that replaced the
    one tapped (Keep after Stop keeping), on the "Conversations"
    heading when tapped, and on the Details field.
  - Against: S1, where the focus indicator is for keyboard focus only.
    The look spec's §7 ring was meant for keyboard and switch access.
    Where the specs move focus after an action, they move it for
    keyboard and TalkBack users; a touch user sees no ring.
- **F6. Conversation titles are a PeerId fragment with "/ human".**
  - Seen: "...x6gY6Eri / human", in the Chats rows and in the app bar.
    At 200 % font the title is cut at both ends ("...x6gY6Eri /..."),
    so nothing that tells two conversations apart is left.
  - Against: S4 (match with the real world). "human" is an endpoint
    name, an implementation concept. The names a person would give
    (contacts, `human-client-ui.md` §3) are a missing flow; see M2.
- **F7. The app bar's subtitle shows the network state where
  messengers show the contact's.**
  - Seen: "Online, some peers might not reach you" under the peer's
    name.
  - Against: S4, because it reads as the peer's state.
  - What has to change, as seen (the five states and their meaning are
    `human-client-ui.md` §7's; the client has no peer presence, so
    nothing takes the peer's place):
    - the conversation's app bar shows the peer's name alone, centred
      in the bar's height, with no subtitle;
    - the state is the device's, so it moves to Chats: a one-line
      subtitle under "Conversations", in body medium and
      `on-surface-variant`, with a leading 18 dp icon (a filled cloud
      when online in any form, a crossed-out cloud when offline, none
      when unknown), so it is not told by colour alone. It takes one
      line, so its texts are short ones written for it, one per state,
      that fit at 320 dp and 200 % without an ellipsis
      (language-culture's; rust-ui-dev-01, QUESTION 01a154cb-b3fe).
      TalkBack reads it after the title: the short text as shown as
      its name (WCAG 2.5.3, the name contains the visible text), and
      the state's full text as its description. It updates in place,
      with no snackbar and no motion;
    - in a conversation, only "offline" shows anything: a notice
      banner under the app bar (look spec §6, `notice-background`)
      that says the device is offline and that messages written now
      wait and go when it is back. It closes by itself when the state
      leaves offline; it has no dismiss control, so it cannot hide a
      state that still holds. "Unknown" shows nothing there, never as
      offline (`human-client-ui.md` §7);
    - in the side-by-side layout the Chats subtitle is the only place,
      with the banner over the conversation pane when offline;
    - deliberately left out: the route indicator (`direct` or
      `relayed`, `human-client-ui.md` §7's option), which waits on a decision of what a
      person does with it.
    The words are language-culture's; the state names above are
    `human-client-ui.md` §7's, not labels.
- **F10. A pending message's status alternates for hours and its time
  never moves.**
  - Seen: two messages sent at 01:14 read "Sending" and "Not
    confirmed, trying again" in turn, nine hours later, each still
    showing 01:14 and offering both "Send again" and "Cancel". Nothing
    says how long it has been or that the peer may be offline.
  - Against: S4 (visibility of status) and S3 (a clear status on every
    outgoing message).
  - The rule (architect-cto-01, REPLY 01a15308-4fa0): the app never
    gives a pending message up. It is retried, 1 s doubling to a 5 min
    cap, until the transport accepts it or the person cancels it
    (ADR-0044, `human-client-ui.md` §6). The alternation seen was not
    the retries: each time the window attached to the running service
    it showed every waiting message as "Sending" until the next failed
    attempt reported it again (rust-ui-dev-01, REPLY 01a15450-4812,
    correcting their REPLY 01a15308-9534; fixed in 62312ee5, not yet
    seen on the phone).
  - What has to change, as seen:
    - a waiting message shows one status while it waits, not two in
      turn; whether an earlier attempt may have reached the peer
      (swipe spec §5) stays visible in that one status, never as a
      flicker;
    - the status says how long it has been waiting, in the coarse
      relative form the reference apps use, and the last problem the
      client has (no network path, peer not reachable, busy, service
      unavailable) when it has one;
    - the time beside it stays the time the person wrote it;
    - "Send again" (retry now, same message) and "Cancel" (the only
      way out) stay as the swipe spec gives them, reached as F2 says.
    The words are language-culture's; this says what each must
    communicate.
- **F11. "Details" is a bare floating field.**
  - Seen: ⋮ → Details opens a panel holding only "Identifier, select
    to copy" over a text field with the full PeerId, its focus ring and
    a selection handle. There is no title, no Copy button and no close
    control; a tap outside closes it.
  - Against: S1 (a dialog or a screen with its title and actions), S4
    (recognition rather than recall: copying is an action, not a text
    selection), and `design/2026-10-16-android-look.md` §6 ("copyable
    in its exact canonical form").
  - What has to change, as seen:
    - ⋮ → Details opens a Material basic dialog: the conversation's
      title as its headline; under it the label "PeerId" and the whole
      PeerId in the code font, wrapped, never cut (as F4); when the
      conversation has an endpoint, a second line with it, marked as
      named by the peer and not verified (`human-client-ui.md` §3,
      "remote-advertised routing labels");
    - two text buttons at the end: "Copy PeerId" ("Copy channel ID"
      for a channel) and "Close". The copy puts the
      exact PeerId on the clipboard, closes the dialog and returns
      focus to ⋮. On Android 13 and later the platform shows its own
      clipboard confirmation, so the app shows none (Android's copy
      and paste guidance: no duplicate confirmation); below 13 a
      "PeerId copied" snackbar, without Undo. The desktop has no platform
      confirmation, so it shows the same snackbar there (S4,
      visibility of status). Close, Back and a tap outside
      close it with nothing copied, focus to ⋮;
    - the text is not a selectable field, so no handle or ring shows;
      TalkBack reads the headline, then the PeerId, then each button.

### Rank 3

- **F12. The Trust screen is reached only by an unlabelled shield
  icon.**
  - Seen: the shield at the end of the Chats app bar is the only way
    in; nothing names it on screen.
  - Against: S4 (recognition rather than recall). A tooltip on long
    press and a content description are the platform's minimum; the
    owner's reference apps put such settings in a menu entry with a
    name.
  - What has to change, as seen:
    - the shield goes; Chats' app bar ends with a ⋮ overflow, as a
      conversation's does. Its accessible name is its own, never the
      conversation's "More options": where the list and a
      conversation show side by side, both buttons are on screen, and
      a screen reader user must tell which is which from the name
      alone (WCAG 2.4.6, labels that describe; rust-ui-dev-01,
      QUESTION 01a154c9-e84c). The name says these are the app's
      options, not a conversation's, and it is the same in every
      layout (WCAG 3.2.4, consistent identification). The conversation's
      button keeps "More options", the platform's own words. A
      tooltip on long press shows the same name;
    - its menu has a named "Trust" entry that opens the Trust screen,
      and nothing else for now: the screens it could hold beside
      Trust (settings, the recovery flow's entry) are steps 7-8's and
      architect-cto's to place, and are added there when they exist;
    - Back from Trust returns to Chats with focus on ⋮;
    - evidence: Material 3's top app bar keeps action icons for
      frequent actions and puts the rest, named, in the overflow; the
      shield is not a symbol a person recognises as "trust" (S4,
      recognition rather than recall).
- **F13. Changing the font scale while a conversation is open drops
  the person on Chats.** (rust-ui-dev-01: the manifest's
  `configChanges` omits `fontScale`, so the Activity is recreated.)
  - Seen: in a conversation, font scale 2.0 then back to 1.0: the app
    showed Chats afterwards (run 2). Rotation, in contrast, kept the
    conversation and its draft.
  - Against: S2 (state survives a configuration change).

### Carried, not ranked here

- **The app is invisible to Android's accessibility services.** A
  `uiautomator` dump of Chats and of a conversation finds one node,
  "InterWeave", at the title's place, and nothing else. This is the
  known gap of Slint 1.18.1's Android backend, which has no
  accessibility bridge (rust-ui-dev-01's finding of 2026-10-16, look
  spec §7). TalkBack is carried to the release gate (§2, S5), so this
  review only records that the run saw it unchanged.

### Not the app's

- **N1.** A system toast, "App has unexpected permission:
  OTHER_SENSORS, ACCESS_LOCAL_NETWORK", shows over the app when it
  opens. It is likely the debug build's. rust-ui-dev to confirm that a
  release build does not show it. rust-ui-dev-01 (REPLY 01a15308-9534):
  neither permission is declared, main or merged debug manifest; their
  reading is that the platform implies both for an app targeting SDK
  36 on an API 37 phone, so a release build would show it too. Not run:
  a release APK means a reinstall that wipes the phone's data. Raising
  the target SDK is architect-cto's. Carried, not ranked.
- **N2.** The app's strings stay English under an Arabic app locale,
  while the layout mirrors correctly (run 2). The translations are
  language-culture's; this review only notes that the RTL layout held.

### What works as specified

Kept apart, so that a fix does not undo it (run 2):
- Stop keeping and Keep by button, each with its snackbar and Undo;
  focus lands on the replacing button.
- The swipe on a message row (above, F2).
- "Hide conversation" is offered in ⋮ only when the conversation holds
  nothing kept, unread or pending; it returns to Chats with
  "Conversation hidden" and Undo, and Undo puts the row back in place.
  The row's left swipe does not move a conversation that is not
  offered (swipe spec §10).
- Remove trust asks first, with the safe choice focused.
- Rotation keeps the open conversation and its draft.
- An RTL locale mirrors Chats: the shield to the start, avatars to the
  end, times to the start.

### Flows the reference apps have and this app lacks

Each such flow is named here, with the product question it raises and
whose that question is. The owner decides whether it belongs to this
stage or a later one.

- **M1. Starting a conversation.** Chats has no way to start one: no
  button, no menu entry. Trust lets a person add a PeerId, but trusting
  a peer opens no conversation with it, and nothing on Trust says how
  to write to the peer just trusted. How a person reaches a new peer
  is architect-cto's product question. Answered (architect-cto-01,
  REPLY 01a15308-4fa0): carried by name from Stage 15's record
  ("conversations exist from traffic"; plan §18), so not a §20
  finding; the owner is asked whether the smallest form, a peer
  trusted on Trust opening its conversation, comes in Stage 18 or 19.
- **M2. Contact names.** Conversations are titled by a PeerId fragment
  (F6). `human-client-ui.md` §3 has contacts as named application
  metadata; the app has no screen for them. Carried by name with M1,
  together with the owner's privacy review (same answer).
- **M3. Recovery.** `human-client-ui.md` §9 requires both platforms to
  state prominently that recovering a profile needs the recovery phrase
  and separately backed-up configuration, trust and endpoint settings.
  The app shows no such statement and has no recovery or settings
  screen to put it on (Chats, a conversation, Details and Trust are the
  whole app). Where that flow lives, and whether the app offers any way
  to back the settings up, is architect-cto's question. Answered: §9
  was amended on 2026-10-19 (2f3b0587): Android asks for no backup it
  has no export for, and the statement lives on the recovery flow,
  plan §20 steps 7-8 (rust-ui-dev-01's recovery branch). M3 closes
  with it and is re-checked in §7.

## 7. Re-review

Round 1, 2026-10-19 (13:00-13:05 phone time), of rank 1 on
rust-ui-dev-01's `feat/j111-android-recovery` at 80fe2281, debug build,
APK sha256 beginning c85eb889 (read from the phone), installed with
the app's data kept. Same phone and method as §6; dark theme and font
1.0 unless named.

| finding | result |
|---|---|
| F1 | Pass. Back hides the keyboard first, then closes the bubble's menu, the Remove trust dialog, a conversation and Trust in turn, each returning to the screen under it; from Chats it leaves the app. |
| F2 | Pass. Nothing under a bubble at rest. A long-press opens a menu anchored to the bubble (Stop keeping, Show source on a kept message) with the bubble in its pressed state; Back closes it and focus goes to the bubble. |
| F3 | Partly. The composer is one line high at rest, with the keyboard and at 200 %, and the inner pill is gone. Still seen: after Back hides the keyboard, the empty field keeps its text-insertion handle, 6 s later as at once. |
| F4 | Partly. The app bar, whole PeerIds, the error-coloured Remove trust, the filled Trust this peer, and the confirmation as a dialog over a scrim (whole PeerId, "Do not change" focused, Back = "Do not change", focus back to the row's button) all pass; nothing passes the edge. Still seen: the PeerIds are in the body font, not the monospace F4's spec item 2 asks for. |
| F8 | Pass. In light theme the status bar's icons are dark on the light top. |
| F9 | Partly. In landscape with the keyboard up the list and the app bar step aside and the composer and its draft are wholly above the keyboard. Still seen: the strip left to the messages shows the top of the conversation (the date line, cut), not the newest message (F9 step 5). |
| F13 | Pass. Font scale 2.0 and back to 1.0 keep the open conversation. |

Rank 2 (F5) still shows, as expected: the ring after touch on the
bubble, the menu's first item, the composer and the row's Remove trust,
and the Chats row a person came back from stays filled.

New, rank 2:
- **F14. "Show recovery phrase" on Trust is a boxed text button** in
  the style F4 removed, under this profile's PeerId. It comes with the
  recovery flow (plan §20 steps 7-8) on the same branch. Against: S1,
  S4 (consistency). What has to change: a Material button of the
  screen's own style; where the recovery entry belongs (Trust, or a
  settings or profile screen of its own) is `design/` work with the
  recovery flow's note, not settled here.

Not judged here: whether the landscape strip left of the conversation
(about 125 px, with a thin vertical line) is the display cutout's
inset or a remnant of the list pane.

Owed for the close: F3's handle, F4's monospace and F9's anchoring,
then round 2 on the build that carries them.

Round 2, 2026-10-20 (18:56-19:18 phone time, the owner's Pixel window;
the F9 landscape row and F16's isolation came after
p2p-network-dev-02's run, from 19:13),
on #313 at 35c71169 (rust-ui-dev-01's, origin/main folded in), debug
build, APK sha256 beginning 2faa357a (read from the phone; installed
16:03 phone time with the app's data kept), after rust-ui-dev-01's and
p2p-network-dev-02's runs. Same phone and method; dark theme, font 1.0.
Before it, a partial run on 2026-10-19 on ebeccf2c (APK 8a3e871e)
stopped when a person took the phone; what it saw is folded in below
where this build shows the same.

| finding | result |
|---|---|
| F3 | Pass as ranked: the composer is one line high at rest and above the keyboard, with no inner pill. The handle after Back is the toolkit's (Slint 1.18.1's Android backend hides it only on an input-method update): not met, carried by name to the owner's release gate (architect-cto-01, 01a15914-65f3); its acceptance is unchanged. Seen again: a focused Trust field keeps its handle with the keyboard down. |
| F4 | Not met. Trust's PeerIds (this profile's and every trusted peer's) and the Details dialog's PeerId are still in the proportional body font: lines of the same width hold 30, 31 and 32 characters, with narrow i, l and 1. #313's 62fad28a does not reach the phone's drawing. |
| F7 | Pass. Chats shows "Online, limited" under "Conversations" with a cloud icon; a conversation's app bar shows the name alone. |
| F9 | Partly (checked at 19:13-19:14 in …U1KnLSeN, after p2p-network-dev-02 released the phone). In landscape with the keyboard up, the list and the app bar step aside and the composer is wholly above the keyboard. Still seen: the strip left above the composer (about 50 dp) shows only the bottom edge of the newest bubble and empty space under it, not the newest message's end with its status and time (F9 step 5). Turning the phone with the keyboard up closes the keyboard and keeps the draft; a tap opens it again. In portrait with the keyboard up (…kX45pbRP), the newest message's end is in view. The strip left of the panes in landscape (the cutout's inset) still shows the window's own background, not the adjacent pane's surface (look spec acceptance 21, specified after this build's fixes). |
| F11 | Pass. ⋮ → Details is a dialog: the conversation's title, "PeerId" and the whole PeerId, "Route named by the peer, not verified: "human"", Close and Copy PeerId. Copy closes it, and Android's own clipboard preview shows the exact PeerId with no spaces (API 37, no app snackbar). Not checked: pasting it into Trust's field; neither an injected paste key nor the keyboard's Paste button put text into the Slint field from a script. A person's check. |
| F12 | Pass. Chats ends with ⋮; its menu holds "Trust", which opens Trust. No shield. Focus with a keyboard not checked (no hardware keyboard on the phone). |
| F14 | Pass. "Show recovery phrase" is a borderless text button. |
| F5 | Pass for what was seen: no ring after a tap on ⋮, the menu item, the composer or Trust's button. The Chats row a person came back from still takes the selected conversation's fill and bar on a phone-wide window, where look spec §5 gives that fill to the side-by-side layout only: filed as F17 below. |
| acceptance 22 (look spec) | Pass. A pending message's status wraps inside the bubble and its time ("01:14") stays whole. |

New, rank 1:
- **F16. In one conversation the composer cannot be focused, and
  nothing says so.** In …x6gY6Eri (the conversation with the two
  pending messages), a tap on the composer opens no keyboard and
  draws no cursor: one tap, two taps, a 150 ms held touch and a short
  touch swipe, each read back with `dumpsys input_method`
  (`mInputShown=false`). The composer looks exactly as it does where
  it works. Control: in …kX45pbRP, minutes later, the same tap opens
  the keyboard. Not verified: why. Ruled out: trust (nothing in
  the code ties the composer to it, rust-ui-dev-01) and Android's
  clipboard preview (at 19:18, after a relaunch and with no copy in
  that app session, the tap still fails). At the failing tap, `dumpsys
  input` shows InterWeave's window focused and nothing above it at the
  composer's height, so the touch reaches the app. The same
  conversation took the keyboard in round 1. Against:
  task 3 (reply) fails with no explanation; S4 (visibility of status,
  help users recognise and recover). The rule (architect-cto-01,
  REPLY 01a159de-729d): trust gates the send, never the composer; the
  composer is withheld only while the platform withholds network
  access, and then it is not drawn at all (`human-client-android.md`:
  "no send controls"). So F16 is a defect whatever its cause. What has to change:
  the composer takes focus in every conversation; a send to an
  untrusted peer becomes a pending row, as `RETENTION.md` §2 orders
  every send, and that row shows the not-trusted reason (look spec §6,
  Composer, and acceptance 23).

New, rank 3:
- **F17. On a phone-wide window, the Chats row a person came back
  from keeps the selected conversation's fill and bar.** Seen after
  Back from …x6gY6Eri and from …kX45pbRP. Against: look spec §5,
  which gives the fill and bar to the side-by-side layout, where a
  conversation is open beside the list; on a phone nothing is open.
  What has to change: on a phone-wide window no row is drawn as
  selected.

There is no F15: the clipped time first seen in the partial run of
2026-10-19 became look spec acceptance 22 (edge case "a message's
status line longer than the bubble"), and is judged there.

Not walked again: tasks 9 and 10. The seeded review conversation
(…DEoNJsP) was no longer on the phone in this window, so its "K8"
message (the kept message rust-ui-dev-01 seeded for task 10, in place
of the T3 row lost on 2026-10-19, §8) was not kept either. F10's new waiting status was not seen: both pending
messages read "May have reached the peer, send again or cancel" with
their written time, which is the may-have-reached state, not a retry
under way. Show recovery phrase and its Recents thumbnail need the
phone's own unlock and were left for a person.

Owed for the close: F4's monospace on the phone, F16, and F9 step 5
in landscape. The F9 rows were walked in a conversation whose
composer works; F16's conversation could not be used for them.

Round 3, 2026-10-21 (10:26-10:31 phone time), on #313 at a4b40c99
(rust-ui-dev-01's), debug build, APK sha256 beginning 338cbf0e (read
from the phone; installed 09:20 phone time with the app's data kept,
by rust-ui-dev-01). Same phone and method; dark theme, font 1.0.

An attempt on 1a8ff67e (APK fd2d2708) on 2026-10-20 sent no input:
MainActivity's window carried `FLAG_SECURE` on Chats, so every capture
came out black. rust-ui-dev-01 traced it to a closing instance taking
the ask that turns the flag off, fixed in f382d5ba (each ask carried
out on the instance in front) and a4b40c99 (one MainActivity
instance). On this build captures work on Chats, Trust and both
conversations.

| finding | result |
|---|---|
| F4 | Partly. Every PeerId on Trust, this profile's and each trusted peer's, is monospace, in dark theme at font 1.0. Not seen on this build: the Details dialog's PeerId, which round 2 found in the body font, and F4's acceptance at 200 % font and in light theme. |
| F16 | Partly. The first half passes: in …x6gY6Eri, with its two may-have-reached rows, a tap on the composer opens the keyboard (`mInputShown=true`). The second half, look spec acceptance 23 (a send to a peer Trust does not list becomes a pending row with the not-trusted reason), was not walked: no message was sent in this window, and whether …x6gY6Eri's peer is on Trust was not read. |
| F9 | Partly, as in round 2. Same in …x6gY6Eri (3 messages) and …5eQhFJG4 (1 message): in landscape with the keyboard up (capture 2424 × 1080 px), the newest bubble's fill shows only from y = 115 to y = 123 px, the composer's top is at y = 236 px, and the space between is empty. The bubble's bottom is 113 px (43 dp at the phone's 420 dpi, 2.625 px per dp) above the composer's top, and the space from the bubble's top edge in view to the composer is 121 px (46 dp), against round 2's about 50 dp read by eye; the newest message's text, status and time are not in view (F9 step 5). The app bar is not in view either. |
| F17 | Still seen: after the app bar's Back from …5eQhFJG4, its Chats row keeps the selected fill and bar on a phone-wide window. |

Not walked: Show recovery phrase (it needs the phone's own unlock), so
on this build nobody saw `FLAG_SECURE` set while the words show or the
Recents card blank, though f382d5ba and a4b40c99 changed how the flag
is set; and the paste into Trust's field (a person's check, round 2).

Owed for the close: F9 step 5; F16's second half (acceptance 23); F4
on the Details dialog, at 200 % font and in light theme; and a
person's walk of S2 on a4b40c99 with the flag read while the words
show and the Recents card after. Whether the close carries any of
them by name, as F3, is architect-cto-01's to decide.

Round 4, 2026-10-21, by script through `phone-run`, guarded on the
window in focus before every tap, in two windows of the same evening:

- **4a** (20:36–20:48 phone time) on the build round 3 used, #313 at
  a4b40c99 (APK sha256 beginning 338cbf0e), though main was the target:
  the phone had not been reinstalled. Dark theme at font 1.0, then font
  scale 2.0 in light theme for F4, set back to 1.0 and dark (checked).
- **4b** (21:12–21:14) on main 4c916aa2 (APK 5ec3da27, installed by
  rust-ui-dev-01 at 18:51 UTC, data kept), with Do Not Disturb on for
  the run and set back: in 4a a personal notification of the owner's
  arrived during a tap and took it (nothing was sent; the capture was
  deleted, so nothing of it is kept here). Rotation set to landscape
  for F9 and set back to automatic.

| finding | result |
|---|---|
| F4 | Pass on 4a for the Details dialog at 200 % font in light theme: the PeerId is monospace, wraps over three lines, nothing is clipped; the route line and both actions fit. With round 3 (Trust, dark, 1.0), F4's acceptance is met except Trust at 200 % and light, which was not walked. |
| F9 | Step 5 partly on 4b: in landscape with the keyboard up (`mInputShown=true`) in …x6gY6Eri, the newest message's status and time are in view, their bottom about 18 dp above the composer's top; the message's text above them is not, so a person sees "May have reached the peer…" without the words it belongs to. On 4a not walked. |
| F10 | Unchanged on 4a and 4b: both pending rows read "May have reached the peer, send again or cancel" at their written time, 01:14; no retry state was on screen to judge the new waiting status. |
| F16 | Second half not walked: no message was sent. |
| F17 | Still seen on 4a and 4b: after the app bar's Back from …x6gY6Eri, its Chats row keeps the selected fill and bar on a phone-wide window. In landscape's two panes the same row is selected beside its open conversation, as look spec §5 gives it. |

Also seen on 4b: the Chats menu holds Trust alone, and the identity
screens start from Trust's "Show recovery phrase", which needs the
phone's unlock and was not opened; the insertion handle stays on the
composer after the tap that opens the keyboard (F3, carried as
toolkit-limited).

**The owner, at the phone, 2026-10-21**, two changes, specified where
they live:
- dates: "my opnion, not simply "Saturday" but "Saturday, 17 October
  2026"", and, asked where, a short form on a Chats row with the full
  date in the day separator, "Today" and "Yesterday" kept (look spec
  §5.2 and §6);
- the contact photo: "my advice for the contact image is to resize
  exactly to the use we do before sending it over transport base64
  encoded", read as one fixed 256 × 256 px and confirmed
  (`design/2026-10-21-contact-cards.md` §5.2).

Not run in round 4, and why:
- the identity screens' own acceptance
  (`design/2026-10-19-android-identity.md` §10): S2 and the Recents card
  need the owner's unlock, a person's check; a first launch, create,
  restore and the process-death items need the app's data wiped, which
  this phone's identity and conversations rule out without the owner's
  word; they want a fresh install or a second device;
- F16's second half (acceptance 23): it needs a send to a peer Trust
  does not list.

**Order of what remains for the close**, by what a person notices
first (for architect-cto-01 to put to the owner; an order, not §5's
ranks):
1. F9 step 5's last part: the newest message's text out of view with the
   keyboard up in landscape, which hides what a status refers to;
2. F17: a row drawn as selected on a phone, where nothing is open;
3. the owner's date change, which a person meets on every screen;
4. F16's second half, seen only when sending to an untrusted peer;
5. the identity screens' person and fresh-install checks, which no
   script can do on this phone;
6. F10's waiting status, which needs a retry in progress to be seen;
7. F3's handle, carried as toolkit-limited.

Round 5, 2026-10-22 (10:57-11:03 phone time), the re-review for the
close, on #358's build: rust-ui-dev-01's afb4466b, merged as
fbca15e6, debug APK sha256 beginning 177170bf (read from the phone;
installed 03:50 UTC with the app's data kept). By script through
`phone-run` at the phone's new address, with Do Not Disturb on for
each run and set back; dark theme, font 1.0, rotation automatic, all
as found.

| finding | result |
|---|---|
| F9 | Pass, step 5 included: in landscape with the keyboard up (`mInputShown=true`) in …x6gY6Eri, the newest message, its text ("w2 another pending one"), status and time, is wholly in view above the composer. |
| F16 | Second half not met: a send to …x6gY6Eri, whose peer Trust does not list, becomes a pending row ("Not sent, send again or cancel", 11:00), as `RETENTION.md` §2 orders, but neither the row nor its menu (Send again, Cancel, Show source) gives the not-trusted reason look spec acceptance 23 asks for (`PeerNotTrusted`: "This peer is not trusted for this profile."). The row was cancelled at once by a drag left ("Sending canceled", the undo window let run out), so the phone holds nothing of the check. The first half passed in round 3. |
| F17 | Pass: after the app bar's Back from …x6gY6Eri, no Chats row is drawn as selected on a phone-wide window. |
| F10 | Unchanged: both earlier pending rows read "May have reached the peer, send again or cancel" at 01:14; a retry in progress, which the new waiting status is for, was not on screen. |
| F6 | Seen in passing: the titles "…x6gY6Eri" (the list and the app bar) and "…5eQhFJG4" (the list) carry no "/ human". F6's truncation at 200 % was not re-checked. |

Not re-walked on 177170bf: F1, F2, F5, F7, F8, F11, F12, F13 and F14,
which passed on earlier builds, and F3 and F4, carried as the close
table states (it cites each round). #358 changed F17's selection and F9's landscape layout, per
rust-ui-dev-01's INFO 01a161c7, and this round walked what it changed
and what was still open. The owner's date change (round 4) is
specified on this branch and not yet built: the build still reads
"Saturday" in the separator and the list, as `labels.rs` and
`clock.rs` do on main.

Also seen: one long-press on the new pending row opened its menu and
a second, minutes later, did not (the drag left then worked); not
reproduced, so not filed.

**State of every ranked finding for the close** (plan §20 clause (j)):

| rank | finding | state |
|---|---|---|
| 1 | F1 Back | built, re-reviewed: pass (round 1) |
| 1 | F2 actions as buttons | built, re-reviewed: pass (round 1) |
| 1 | F3 composer | built and passed as ranked (round 2); its insertion handle after Back is Slint 1.18.1's Android backend: carried by name to the release gate, toolkit-limited (architect-cto-01, INFO 01a15914) |
| 1 | F4 Trust's look and PeerIds | built, re-reviewed: partly. Met except Trust at 200 % font in light theme, which no round walked (Trust dark at 1.0 in round 3, Details at 200 % and light in round 4); carried by name to a person's check or the next window |
| 1 | F8 status bar in light theme | built, re-reviewed: pass (round 1) |
| 1 | F9 landscape keyboard | built, re-reviewed: pass (round 5) |
| 1 | F16 composer focus and untrusted send | built, re-reviewed: first half pass (round 3); second half pass (round 6) except acceptance 23's network-denied clause, walked in no round: carried by name to the owner's release gate (architect-cto-01, REPLY 01a163ea), walked on the owner's word or on a test device |
| 2 | F5 focus ring on touch | built: pass (round 2) |
| 2 | F6 "/ human" in titles | built: no "/ human" in the titles round 5 saw (its F6 row); F6's truncation at 200 % was not re-checked |
| 2 | F7 app bar subtitle | built: pass (round 2) |
| 2 | F10 pending status over hours | built (62312ee5); not seen on the phone, since no retry was in progress in any window: carried by name |
| 2 | F11 Details | built: pass (round 2) |
| 2 | F14 Show recovery phrase button | built: pass (round 2) |
| 3 | F12 Trust's entry | built: pass (round 2) |
| 3 | F13 font scale keeps the conversation | built: pass (round 1) |
| 3 | F17 selected row on a phone | built: pass (round 5) |
| — | the owner's dates (round 4) | specified in the look spec on this branch; not built |

Not reviewed on the phone, and why: the identity screens' own
acceptance (`design/2026-10-19-android-identity.md` §10). S2 and its
Recents card need the owner's unlock; a first launch, create, restore
and process death need the app's data wiped, which this phone's
identity and conversations rule out. They are a person's check and a
fresh-install run.

Round 6, 2026-10-22 (15:03-15:04 phone time), F16's second half alone,
on #364's build: rust-ui-dev-01's be4ca6b4, debug APK sha256 beginning
9433e6ff (read from the phone; installed 13:02 UTC with the app's data
kept), at the phone's address 192.168.100.50:33259. By script through
`phone-run`, Do Not Disturb on and set back.

| finding | result |
|---|---|
| F16 | Second half pass: a send in …x6gY6Eri, whose peer Trust does not list, becomes a pending row whose reason reads "This peer is not trusted for this profile." on a line of its own, in the error colour, above "Not sent, send again or cancel · 15:03", the time still on the bubble's last line (look spec acceptance 23's second clause). The two earlier pending rows to the same peer show the same reason above their may-have-reached status. The test row was cancelled at once by a drag left ("Sending canceled", the undo window let run out), so the phone holds nothing of the check. The reason as TalkBack reads it was not checked: TalkBack does not reach the Android app yet (look spec §7). Acceptance 23's third clause, the network-denied state (no composer and no Send drawn, nothing in their place, the `NetworkDenied` notice with its action under the app bar), was walked in no round: it needs INTERNET revoked on the owner's phone (`pm revoke … android.permission.INTERNET`), which is expected to end the app's process (`human-client-android.md`, "Revoked while running", still to verify on the device) and was not done without the owner's word. Carried by name to the owner's release gate (architect-cto-01, REPLY 01a163ea). |

With round 6, every rank-1 finding is built and re-reviewed, F3's
handle and F4's Trust at 200 % in light theme carried by name as the
close table states, and F16's acceptance 23 with the app's network
access withheld, walked in no round, is carried by name to the owner's
release gate (architect-cto-01, REPLY 01a163ea).

## 8. The swipe spec's acceptance on the same build

`design/2026-10-16-message-swipe.md` §11, walked on the same build and
phone as §6 (main ffce28df, APK 0d1bcbec), 2026-10-19, by script
through `phone-run`, gestures injected with `input motionevent` and
`input swipe`. This is j1's device review, which the phone link held
up since 2026-10-17.

| item | result |
|---|---|
| 1 | Pass: a slow right drag on a kind-A row arms the Keep fill, commits "Kept"; Undo unkeeps. |
| 2 | Pass: a right drag to 30 % and back, released: nothing, no snackbar. |
| 3 | Not settled: injected flings did not commit at 75 dp / 150 ms or 114 dp / 200 ms, and did at 95 dp / 120 ms. Injection timing is not a finger's; a person at the phone checks it. |
| 4 | Partly: the left drag on a kind-A row was not run without Undo, since it loses the phone's test message. |
| 5 | Pass: a right drag on a kind-B row does not move it; the first left drag after it commits "No longer kept" (it did not on 761d72b3); Undo, then a restart: still kept. |
| 6, 9 | Not run: each removes test data for good. |
| 7 | Pass: left drag, Undo, force-stop, restart: still kept. |
| 8 | Pass, after a restart: a right drag arms "Send again", retries, no snackbar, status "Not confirmed, trying again"; a left drag shows "Sending canceled, may have reached the peer"; Undo restores it in place. The ids were not compared (no diagnostics view on the phone). |
| 10 | Not run: no kind-D row on the phone. |
| 11 | Pass: a drag about 57° off horizontal leaves the row still. |
| 12, 13 | Blocked: no accessibility bridge (§11's precondition). |
| 14 | Not run: Remove animations is a person's check of feel. |
| 15 | Pass: a drag from the left edge shows the system Back chevron and does not move the row (Back then leaves the app: F1). |
| 16 | Pass: under an Arabic app locale a right drag on a kind-B row commits Unkeep, and the snackbar mirrors (Undo at the start). |
| 17 | Pass (§6 run 2): hide offered only on a conversation with nothing kept, unread or pending; "Conversation hidden", Undo restores; a row not offered does not move. |

During item 16 my script tapped where Undo sits in a left-to-right
layout. The snackbar closed, and the app's restart then dropped the
test message "T3 RTL test row three." as retention says for a read,
unkept message. It is gone from the phone; rust-ui-dev-01's seeding
replaces it.
