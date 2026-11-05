# A very long message: a bounded part in the conversation, the whole in a reader — desktop and Android

Status: proposal, 2026-11-04. Asked by rust-ui-dev (QUESTION 01a1a896,
their job j92, my job j90), read against `origin/main` ed46059ff. It
answers their three questions (§4, §5, §6) and declines their
alternative (§2). architect-cto ruled on its four product questions
(§11): Copy is allowed on every message (Q1, REPLY 01a1a89e-9df5,
§5.3); the bound is the contract's, in HUMAN-CHAT.md's Bounds bullet,
with §6's figures and conditions (Q2, same reply); and what is counted
is what is drawn (Q4, REPLY 01a1a8dd-a73d, §6), written on #562;
and a cluster past 32 code points is drawn as one replacement mark
(Q5, DECISION 01a1a928-0d26, §6). Until #562's amendment is on
`main`, the build waits for it (Q5's rule lands with rust-ui-dev's fix
and does not hold the build; aligning the contract's "raw source
viewable" to it is architect-cto's), and it must carry §6's current figures (2 000
and 40, lowered after the ruling: §11 Q2). Where the two ever differ,
the contract governs only with figures that keep §6's ceiling
condition; a contract figure that does not is raised with architect-cto
before anything is built to it.

Owner of the design: ux-designer. The words are language-culture's: §8
names each string and what it must say. Colours are roles of
`design/2026-10-16-android-look.md` §3. Both clients render the same
Slint UI (`crates/human/ui-slint`) from the same `ui-model`; "narrow"
and "wide" are the layouts `app.slint` already has. Built by
rust-ui-dev.

**What this note revises.** For a body past §6's bound, it replaces
the look spec's edge case "A very long message"
(`design/2026-10-16-android-look.md` §8); for a body within the bound,
that line stands as written. It revises one rule of
`design/2026-10-16-message-swipe.md` §7: a cut row has a second focus
stop besides its bubble and its links, Read the whole message (§4.3,
§9). And it revises message-swipe §7's list of a row's menu items, and
its §11 item 23 that checks that list: every text row, cut or whole,
gains Copy message source, and a cut row Read the whole message too,
both before Show source, which stays last (§4.3, §5.3). A card row,
which has no Copy, is unchanged. And it revises the look spec's §6
Composer, a 48 dp field (the build grows it without a limit): it
grows from 48 dp up to six lines, then scrolls inside itself (§11 Q3,
A11). And, for a grapheme cluster of more than 32 code points (§6,
§11 Q5), it revises what `design/2026-10-27-start-window.md` §3 draws
in the Chats list's subtitle and what the look spec's Show source
shows: one replacement mark in place of the cluster.

## 1. What is already decided

- **HUMAN-CHAT.md**: an envelope is at most 196 608 bytes decompressed,
  so a body is somewhat less; render cost must stay linear in its size;
  the raw source is always available to the receiver, however it
  rendered.
- **RETENTION.md §3**: read is a local UI state; a read message not
  kept is gone when the process exits; Keep is offered only after read.
- **`ui-model` `conversation_viewed`**: every unread row of a
  conversation is read when the conversation is shown in a focused
  window. This note does not change that: a cut message is read when
  its conversation is shown, as an uncut one is (§7).
- **rust-ui-dev's measurement** (QUESTION 01a1a896): the renderer
  panics once one message is taller than 32 767 physical pixels (about
  1 400 lines at scale 1, 700 at 2×); a body of about 900 lines costs
  about 0.2 s at first draw on the A40, growing with its lines.

## 2. The person's goal, and why the whole body is not drawn in place

**Who and what.** Someone opens a conversation in which one message is
far longer than a screen: a pasted log, a document, a long list. They
want, in order: to see what it is and how long it is; to go on through
the conversation past it; to read any part of it, start to end; and to
take all of it elsewhere (into a file, an editor, a ticket).

**Why not draw it whole** (rust-ui-dev's alternative: the whole body
drawn in bounded pieces, which fixes only the panic). The panic is
one cost; the person's is another. At the ceiling, a body is some
49 000 drawn lines. In place, the messages after it are reachable only
by scrolling through all of it, on a phone more than a thousand
screens, and the conversation's own position is lost in it. That fails
two of Nielsen's heuristics, 3 (user control and freedom) and 7
(flexibility and efficiency of use): the person cannot step past
content they did not ask for. Messaging apps on both platforms cut a
long message in the conversation and open the whole in a view of its
own (the Android Signal client's "Read more", Gmail's "Message clipped —
View entire message"). That is platform habit, not proof; the
hypothesis this note makes, and how it is checked, is §10's H1.

## 3. What the person sees — the states

A message's body is **cut** when what the row draws passes §6's
bound, **whole** otherwise, decided **in each mode** by what that mode
draws (§6): formatted mode by the formatted body, source mode by the
source. So one message can be whole in one mode and cut in the other —
a body of reference definitions, which draw nothing formatted, or of
links whose labels pass 2 000 characters and so are drawn inert (§6),
is short formatted and long as source; §6's count decides every other
body. The row always shows the state of the mode it is in,
and switching mode (Show source, Show formatted) switches it: the
preview, the cut line and the button appear or go with the mode that
needs them; focus stays on the bubble, which the menu returns it to
(message-swipe §7). Nothing changes for a body that is whole in the
mode shown.

| state | in the conversation | in the reader (§5) |
|---|---|---|
| whole in the mode shown | the body in that mode, as today | not offered from this mode |
| cut, formatted | the preview (§4.1), the cut line (§4.2), Read the whole message (§4.3) | the whole body, formatted |
| cut, source | the source's preview, the cut line, Read the whole message | the whole source |
| reader opening | — | its first screen within A5's figure (§10); nothing below 0.5 s, then an indeterminate progress bar under the app bar, labelled `ReaderOpening` (§8), until the first screen is drawn |
| row gone while the reader is open | — | the reader closes (§5.4) |

A sent message is treated as a received one: the person may have
pasted something long themselves. Its delivery state stays inside the
bubble, under the cut line and the button, as the look spec puts it
under the text.

## 4. Where the body stops (question 1)

### 4.1 The preview

- The preview is the start of the body as the whole body's parse
  renders it — never a parse of a cut source, so emphasis, a list's
  numbering or a code block that runs past the cut look in the preview
  as they do in the reader.
- It ends at the last block boundary (a paragraph, a list item, a table
  row, a code line, a heading) inside the preview's limit (§6) when
  that boundary is in the last quarter of whichever limit cuts
  (characters or lines); otherwise at the last
  word boundary inside the limit, and then an ellipsis (`…`) follows the
  preview's last word. The cut never falls inside a grapheme cluster. A
  table cut between rows keeps its header row.
- A link's control (the one after the text) is in the preview only
  when its text is and the whole of its label, as shown, fits inside
  the preview's figures with the rest of the preview (§6's count);
  otherwise it is not drawn there, and the link is in the reader.
- In source mode (Show source, as today), when the source is cut, the
  preview is the source's start, cut by the same rule over its lines.

### 4.2 The cut line

Under the preview, inside the bubble, in `on-surface-variant` on the
bubble's colour (contrast §9): a short sentence saying the message
continues and **how much of it is shown**, as a whole percentage —
`LongMessageShown` (§8). It wraps like the text: at 200 % text it takes
two or three lines. A percentage, not lines or words, because it is the
one measure that does not depend on how the text wraps, which script it
is in or how the person counts: drawn lines change with the width and
text size, a script written without spaces does not mark where words
end, and characters mean nothing to most people. It is the share of the
body's characters, counted as §6 counts them, that the preview holds,
rounded down, at least 1 %.

### 4.3 Read the whole message

A text button, `ReadWholeMessage` (§8), under the cut line at the
bubble's start edge, with the 48 dp target every control keeps. It
opens the reader (§5) at the start of the body, in the mode the row
is in (formatted or source). It is a control at rest on purpose,
unlike the row's actions, which are in its menu (message-swipe §7):
the cut line says there is more, and the way to it must be in sight
beside it (visibility of system status, Nielsen's heuristic 1).

It is also an item of the row's menu. A cut row's menu, in order: the
row's actions as message-swipe §7 gives them (Keep and Dismiss, or
Unkeep, or Retry and Cancel), then Read the whole message, then
`CopyMessage` (§5.3), then Show source last. Where the toolkit offers
custom accessibility actions (message-swipe §7: optional, not there
today), it is one of the bubble's as well. No gesture opens it.

Keep, Dismiss and Show source stay on the row and act on the whole
message, as today.

## 5. The reader, and the source past the bound (question 2)

### 5.1 The page

A page of its own: on a narrow layout it covers the app, on a wide one
it takes the conversation's pane with the list beside it, as the
contact page does. Its top app bar: Back (the navigation icon), the
title: the sender's name as the conversation shows it (`You` for one's
own), one line cut with an ellipsis as every app bar title is (look
spec §8), and two actions — Show source / Show
formatted (the same labels as the row's), and Copy (§5.3). Below the
bar, the first line of the content, `ReaderSent`, gives the day and the
time the message reached this app, as the row's time does (`local_at`,
look spec §5.2), wrapping like the text, so neither is ever cut off;
its forms on screen and spoken are §8's.
Under it, the whole body in one continuous scroll, drawn as the person
scrolls so that only what is near the screen is laid out. Its look is
the bubble's text on `surface`, without the bubble.

**Not pages.** The whole body is one scroll, not pages of N lines:
paging breaks a log's line or a paragraph at an edge the content did
not choose, puts a control between the person and the next line, and
no app on either platform shows text that way. Scrolling is the
platform's convention; the scroll bar (desktop) and the fast-scroll
thumb (Android, shown once the body is taller than four screens) show
where the person is in the body and let them move through it quickly.
Whether the toolkit offers both is a precondition of A5 (§10).

**Drawn in pieces**, in both modes, because the toolkit cannot draw
one block taller than its ceiling. A piece ends where the text is drawn
broken anyway, and there nothing shows: in source mode at a newline;
in formatted mode at a block boundary (a paragraph, a list item, a
table row, a code line, a heading), since a single newline inside a
paragraph is not drawn as a break. Only a stretch with no such
boundary that is longer than a piece — one long source line, or one
long paragraph, hard-wrapped or not — is split elsewhere: at a word
boundary, or at a grapheme where there is none, so that no character
is lost, repeated or cut, and no gap opens. The one thing that shows is
a short, ragged line at that split, where the text itself does not
break. That is accepted (rust-ui-dev's j92 note on #561: the toolkit
does not say where it wraps): it changes no character, Copy takes the
source whole, and it comes about once per piece of such a stretch — a
few thousand characters, rust-ui-dev's estimate of a piece — never
between blocks or lines that break anyway.

**Across a resize.** A window that changes between narrow and wide
(a rotation, a fold, a desktop resize) keeps the reader open, at the
same place in the body, with focus where it was: on narrow it covers
the app, on wide it takes the conversation's pane.

### 5.2 Source in the reader

Show source in the reader shows the whole raw source, in the code font
(a grapheme cluster past 32 code points drawn as one replacement mark,
§6; its exact bytes are Copy's, §5.3),
in the same scroll, wrapping as the row's source does. A source line
longer than the screen wraps over as many lines as it needs, drawn in
pieces as §5.1 says. The switch keeps the position
as nearly as the two forms allow: the first block on screen before the
switch is on screen after it.

### 5.3 Copy

Copy puts the **message's whole markdown source** — the envelope's
`text` as the sender wrote it, byte for byte — on the clipboard, in one
action, whatever part the reader shows and whichever mode it is in.
It is also an item of every text message's row menu, cut or whole
(`CopyMessage`): on a cut row as §4.3 orders it, on a whole one just
before Show source; a card has none. Feedback: on Android 13 and later
the system shows its own clipboard confirmation and the app adds none
(the platform's guidance); on Android 12 and earlier, and on the
desktop, a snackbar `MessageCopied`. If the platform refuses the copy,
a snackbar `MessageNotCopied` says so; nothing else changes. Copy is
not undoable and needs no undo: it changes nothing in the app.

The conditions architect-cto's ruling carries (§11 Q1), each a check
of A9: Copy acts on the person's press only, never on receipt, read or
open; the clipboard gets the source and nothing else — no sender,
time, id, route or formatting of the app's; one write per press,
always the whole body, never a part. Because the source carries the
markdown, the label says so: Copy message source (§8).

On Android 13 and later the clip is **not** marked sensitive. Marking
a clip sensitive hides the system's preview of it; the message is on
the person's screen at the moment they copy it, so the preview shows
nothing that is not already shown, and without it the person loses the
one sign of what was copied. (Recovery words, which are marked so, have
no clipboard path at all: `design/2026-10-19-android-identity.md`.)

### 5.4 Leaving the reader

Back (Android's system Back or the app bar's) or, on the desktop,
Escape closes the reader, and the conversation is where it was: the
row in the mode it was in when the reader opened, whatever mode the
reader was switched to, so its Read the whole message is there for
focus to return to (§9).

The reader also closes, with no message of its own, when its row
leaves the conversation. Only the person's own act can cause it: on a
wide layout, hiding the conversation from the list beside the reader.
Nothing else removes the row while the reader is open: the session's
item ceiling (`ui-model`'s eviction of read, unkept items) never
evicts the item open in the reader, as it never evicts the focused
item, so the text cannot go from under the person while they read it.

## 6. The bound (question 3)

**Both, characters and lines.** A body is cut when it has more than
**2 000 characters** or more than **40 lines**, and the preview is then
at most **1 600 characters** and **32 lines**. A character is a
grapheme cluster; a line is one of the model's lines (a paragraph, a
list item, a table row, a code line, a heading, a rule), however it
wraps, and each link's control, drawn as a line of its own (§4.1),
counts as one. **What is counted is what the app draws**, each time it
draws it (architect-cto's ruling of Q4, §11, in HUMAN-CHAT.md's Bounds
on #562, not yet on `main`): the drawn text; every link control's label
as shown — its own words and the destination made visible, a hidden
character spelled as a code — once per link in the body, so an
autolink counts its text and its label, and a reference link used
many times counts its label each time; every image placeholder as
shown. Every other figure in this note — the preview's limit, the
cut line's percentage, A1 and A2 — counts the same way.

**In source mode what is drawn is the source** (HUMAN-CHAT.md on #562:
"the plain-text fallback and Show source are cut by the same rule over
the source's lines"): its characters are the source's, markup and
destinations included, and its lines the source's lines, blank lines
included. The plain-text fallback, for a body whose markdown is past a
nesting or table bound, counts the same way. The cut line in source
mode gives the share of the source shown.

**A grapheme cluster of more than 32 code points is drawn as one
replacement mark** (architect-cto's DECISION 01a1a928-0d26, on
rust-ui-dev's measurement that one cluster of 256 code points stops the
desktop client): U+FFFD in place of the whole cluster, never cut inside
it, in every place the client draws the body — the row, the preview,
the Chats list's subtitle, Show source, the plain-text fallback and the
reader, and any later view that draws a body (the ruling also names a
reply quote, which the build does not draw today: its reply line is
fixed words) — and counted as one character. The notification's text
is drawn by the platform, not the client, and the ruling does not name
it; it is unchanged. A link whose control's label, as shown — hidden
code points already spelled as codes — would still hold such a cluster
gets no control, as a label past 2 000 characters does. How it looks is this note's: the font's own
replacement glyph, as the platform draws U+FFFD everywhere else, with
no words on screen and nothing added to the cut line. Ordinary writing
puts a few marks on a character at most; a cluster past 32 is a stack
of marks, not text a person reads, and the platform's sign for "this
could not be shown" says that without a sentence. TalkBack reads it as
the platform reads U+FFFD. Nothing is lost: Copy message source
carries the cluster byte for byte (§5.3).

The gap between the two figures is there so that a body just over the
bound is never cut to hide only its last sentence or two.

**A link whose label would be too long gets no control.** Where one
control's label as shown would pass 2 000 characters, the link has no
control at all: its text is drawn as plain, inert text, as a link of a
scheme outside the allow-list already is, and the whole destination
stays in the source (Show source, Copy message source; Copy alone for
a cluster past 32 code points, above). The label is
never cut: a cut label could show one host and open another, which is
the spoof the full label is there to prevent (architect-cto, #562's
review). What the person sees is the link's text, which they cannot
press (nothing, if the link has no text), and the way to it is the
source; nothing explains it on screen, as nothing does for a link of a
disallowed scheme today. That is accepted for a link no honest sender
writes; a hint is left out (below, §11).

Why both: characters bound a body that is one long paragraph, which
wraps into many drawn lines without a line of the model; lines bound a
list of one-letter items, which has many lines and few characters.

Why these figures, and the conditions any other figures must keep:

- **For the person**: at 100 % text on a phone the preview is about
  one and a half screens — enough to tell what the message is, never
  so much that the next message is far.
- **At 200 % text**: the preview is taller but bounded, and every
  whole body at the bound, in its tallest shape, on the narrowest
  layout, at the highest display scale either client supports, draws
  under the renderer's ceiling. My estimate on the Pixel 9a (2.625 px
  per dp; body text 32 sp on a 48 sp line, about 126 px, about 17
  characters per line in a narrow bubble; a first-level heading 40 sp
  on a 56 sp line, about 147 px, about 13.6 characters per line): a
  paragraph body, 2 000 / 17 + 40 ≈ 158 drawn lines, about 19 900 px;
  a body of 40 headings, 2 000 / 13.6 + 40 ≈ 187 lines, about
  27 500 px. That is an estimate, not a measurement; §10's A4 is the
  measurement, and if it does not hold the figures go down, not the
  text size. (The first figures, 2 400 and 48, gave about 33 000 px for
  the heading body, over the ceiling: review round 1, F9.)

## 7. Read, Keep and what a cut does not change

- A cut message is read when its conversation is shown, as any message
  is (§1): opening the reader does not make it read, and not opening
  it does not keep it unread.
- While the reader is open, its conversation is not shown, as with the
  trust settings that cover it (`crates/human/ui-slint/README.md`): a
  message that arrives in that conversation meanwhile stays unread,
  keeps its stored copy, and is read when the reader closes and the
  conversation is shown again.
- Keep is on the row and keeps the whole message. A person who wants
  to come back to a long message after the app closes keeps it; the
  cut line is what tells them there is more to come back to.
- The chat list's one-line subtitle is unchanged but for §6's
  replacement mark; the notification's text is unchanged.

## 8. Strings

Names, where each shows and what it must say; the words are
language-culture-en's (proposed on #561), and each other locale's words
are that locale's holder's.

| key | where | must say | en (proposed) |
|---|---|---|---|
| `LongMessageShown` | the cut line (§4.2) | that the message continues past what is shown, and the share shown (`{percent}`, the whole locale-formatted value, sign included) | The first {percent} of the message is shown. |
| `ReadWholeMessage` | the button and menu item (§4.3) | that it opens the whole message to read | Read the whole message |
| `ReaderSent` | the reader's first content line (§5.1) | when the message reached this app, as the row's time does (`local_at`): the day (`{day}`) and the time as the row shows it (`{time}`). On screen `{day}` is `Today`, `Yesterday`, else the Chats list rows' short date (`DateRowThisYear`, `DateRowOtherYear`); in its accessible label it is `Today`, `Yesterday`, else `DateFull`, never the short form (look spec §5.2, §9) | {day}, {time} |
| `ReaderOpening` | the reader's progress bar (§3) | that the message is being opened | Opening the whole message |
| `CopyMessage` | the reader's action and every text message's row menu (§5.3) | that it copies the whole message's source, markdown included | Copy message source |
| `MessageCopied` | snackbar (§5.3) | that the whole message's source is on the clipboard | Message source copied |
| `MessageNotCopied` | snackbar (§5.3) | that the message could not be copied | Could not copy the message |

Show source and Show formatted reuse the row's existing labels.

## 9. Accessibility

**Focus table** — every case, and its check (§10).

| case | focus goes to | check |
|---|---|---|
| Read the whole message pressed (button or menu) | the reader's first content element: on Android, TalkBack's focus on the app bar's title (the sender), then `ReaderSent` next; on the desktop, the scroll area, so arrows and Page keys scroll at once | A6 |
| Reader closed (Back, Escape) | the row's Read the whole message button | A6 |
| Reader closed because its row left (§5.4) | stays where the person's act in the list left it | A7 |
| Show source / Show formatted in the reader | stays on the action pressed | A6 |
| Copy pressed in the reader's app bar | stays on Copy | A6 |
| Copy message source chosen from a row's menu (cut or whole) | the row's bubble, as when the menu closes with nothing done (message-swipe §7) | A6 |
| The app sent to the background and back with the reader open | the reader, at its position, focus where it was | A7 |
| The window resized between narrow and wide with the reader open | where it was in the reader (§5.1) | A7 |

- **What is read, and in what order**, in a cut row: the bubble is read
  as one item, as the look spec's `Item` label gives it, with `{body}`
  the preview's text followed by the cut line's sentence — never the
  whole body. Then, as their own stops, Read the whole message, the
  preview's link controls, and the row's next element. The button's
  label is its own string.
- **Keyboard** (desktop): Tab reaches Read the whole message after the
  row's bubble and before its links; in the reader, Tab goes to Back,
  then Show source (or Show formatted), Copy and the scroll area; Up,
  Down, Page Up, Page Down, Home and End scroll; Escape closes.
- **TalkBack in the reader**: swiping forward reads the body block by
  block, scrolling as it goes, to the end of the body, though only
  what is near the screen is laid out. A8 states this as a
  precondition: if the build's accessibility tree cannot expose blocks
  that are not laid out, A8 is blocked, never passed.
- **Contrast**: the cut line's `on-surface-variant` holds 4.5:1 on
  both bubble colours (received, sent) in both schemes, and the button's
  `primary` 4.5:1 on both; the focus ring 3:1 on both bubbles and on
  the reader's `surface`.
- **Motion**: the reader's arrival is the platform's page transition;
  under Remove animations it appears without one. Nothing else moves.
- **RTL**: the cut line and the button sit at the bubble's start edge,
  on the right; the ellipsis ends the preview's last line on its left;
  the percentage is the locale's (`٪` and Arabic-Indic digits where the
  locale writes them). The reader mirrors as every page does.

## 10. Acceptance

Run on the Pixel 9a and the A40 for Android, the desktop build at
scale 1 and 2, with a body of each of these shapes, the largest whose
envelope fits the 196 608-byte ceiling: one long paragraph, a list of
one-letter items, a code block, a table of 256 rows, first-level
headings, headings of autolinks, one reference link used many times,
images with no description, one link whose destination is padded
with hidden characters (§6); and A12's four cluster bodies, each of the
size A12 gives it. Each is checked in formatted and source mode, at 100 % and
200 % text, left to right and right to left.

**Precondition on Android** for every TalkBack check below (A6's
Android half, A8): an accessibility adapter that lets TalkBack reach
the app. The Android build has none today
(`apps/human-android/device-runs/2026-11-04-samsung-a40.md`); without
it, those checks are blocked, never passed.

- **A1** A body of 2 000 characters and 40 lines is whole; one more
  character, or one more line, and it is cut (§6), in each mode by what
  that mode draws. A body whole formatted and past the bound as source
  (links whose labels pass 2 000 characters, drawn inert; a body of
  reference definitions; about 2 000 blank lines) is whole formatted, and in source mode shows the
  source's preview, the cut line with the source's share and Read the
  whole message, which opens the reader in source mode; Show formatted
  takes them away again.
- **A2** A cut row shows the preview, the cut line with a percentage
  equal to the preview's characters over the body's, rounded down and
  at least 1, and Read the whole message; its preview ends as §4.1
  says, and a table cut keeps its header. Its menu is in §4.3's order.
- **A3** With a cut message in the middle of a conversation, the cut
  row — preview, cut line, button and status — is at most as tall as
  the figures below, counted in the conversation's viewport (the space
  between its app bar and its composer, about 1 960 px on the Pixel 9a):
  for the paragraph, list, code and table shapes, 2 viewports at 100 %
  text and 7 at 200 %; for the headings shape, 4 and 12. The figures
  are §6's estimates with the cut line, the button and the status
  added (a paragraph at 200 %, about 12 500 px; headings at 200 %,
  about 22 600 px); the build measures and reports each, and one over
  its figure comes back to ux-designer. Why the cut is not bounded in
  drawn height instead, which would hold one figure for every shape:
  the cut would then move with the width and the text size, and the
  same message would show a different part after a rotation. That is
  a judgement, kept for H1's walkthrough to test at 200 % text. The
  2 400-character paragraph of A12 (C2) takes the paragraph figures,
  the mark being one character of body text. The
  four link and image shapes take the headings figures, 4 and 12:
  with what is drawn counted (§6), none of their drawn characters is
  taller than a first-level heading's — a control's label is in body
  medium, a placeholder in body text — so the headings estimate bounds
  them. That is an estimate too, measured and reported like the rest.
- **A4** At 200 % text, on the narrowest layout and at the highest
  scale, a whole body at the bound in each of the shapes above, the
  four link and image shapes and A12's C1 included, draws without a
  panic (the
  padded link drawn as inert text with no control, §6), in formatted
  and in source mode, and so do A1's whole-formatted, cut-as-source
  bodies in source mode,
  and the build measures and reports the tallest row's height against
  the 32 767 px ceiling.
- **A5** The reader opens on each shape with its first screen drawn
  within 0.5 s on the A40, optimised build; End reaches the body's last
  line within 1 s; scrolling shows no blank screen, and no seam but §5.1's ragged line at
  a split inside a stretch with no break, in either mode.
  The build measures and reports both times; a figure it cannot reach
  comes back to ux-designer, not changed quietly. Precondition: the
  toolkit offers the scroll bar and the fast-scroll thumb of §5.1; a
  missing one is reported, not built around.
- **A6** Every row of §9's focus table whose check is A6, by keyboard
  on the desktop and TalkBack on the Pixel.
- **A7** The reader's row removed while it is open, the app sent to
  the background and back, and the window resized, as §9 says; a reader
  opened from a row cut in one mode only, switched to the other mode,
  then closed, leaves the row in the mode it was opened from, with
  focus on its Read the whole message (§5.4); and with
  more than the session's item ceiling of read, unkept messages, a new
  one arriving leaves the reader open on its message (§5.4).
- **A8** TalkBack reads a cut message's reader from its start to its
  last block by swiping forward alone, `ReaderSent` first, with the
  full date for a message from before yesterday (§8). Precondition: an accessibility
  tree that exposes the blocks that are not laid out (§9); without it,
  blocked.
- **A9** Copy, from the reader in each mode and from the row's menu of
  a cut and of a whole message, puts on the clipboard bytes equal to
  the message's `text` as the build holds it (the source Show source
  shows, but for §6's replacement mark) and nothing else, once per press, and the feedback of §5.3
  shows — a cluster drawn as one replacement mark (§6) arrives on the
  clipboard whole; on Android 13 and later the system's confirmation shows its
  preview (the clip not marked sensitive). Receiving, reading and
  opening a message write nothing to the clipboard.
- **A10** A cut message is read when its conversation is shown, and
  Keep on it keeps the whole message: after a restart the reader opens
  on all of it. A message arriving in the conversation while the reader
  is open stays unread until the reader closes (§7).
- **A11** The composer, given a draft of many lines up to the
  envelope's ceiling (pasted, and typed past six lines), grows to six
  lines at 100 % and at 200 % text, then keeps that height and scrolls
  inside itself with the caret in view; nothing panics; the message
  sent carries every character of the draft (§11 Q3).
- **A12** Four bodies, each holding one cluster of 256 code points:
  **C1**, a paragraph of 2 000 characters whose first character is the
  cluster; **C2**, the same paragraph at 2 400 characters; **C3**, a
  message that is one link with a 20-character text to an `https`
  destination of 30 characters whose last character is the cluster,
  so that its label as shown holds the cluster, is far under 2 000,
  and only the cluster rule can take its control away; **C4**, the cluster inside 17 nested block quotes, past the
  nesting bound, so that the body is drawn as the plain-text fallback.
  In each, the cluster is drawn as one replacement mark, never part of
  it: in the row, the Chats list's subtitle (C1, C2: the cluster opens
  the line), the preview (C2), Show source, the plain-text fallback
  (C4) and the reader (C2). The mark counts as one character: C1 is
  whole (A1), C2 is cut and its cut line's percentage counts the mark
  as one. C3 has no control, its text drawn inert, and is whole, so A3 does not apply to it. Copy message source carries
  the cluster whole in each (A9). TalkBack's reading of the mark is
  reported, under §10's Android precondition.

**H1** (the hypothesis of §2, unvalidated): a person given a
conversation with a long pasted log in its middle, asked to find the
message after the log and then the log's last line, does both without
backtracking. It is falsified if, in a walkthrough with a person who
did not see this note, they scroll the preview looking for the rest,
miss the cut line, or look for the next message inside the reader.

## 11. Questions, and what is left out

- **Q1 — architect-cto**: Copy of a message's raw source (§5.3) is an
  action the client did not have. Is it allowed, on a cut message only
  or on every message? **Ruled** (REPLY 01a1a89e-9df5): on every
  message, under the conditions §5.3 lists; whether the clip is marked
  sensitive was left to this note (§5.3: it is not).
- **Q2 — architect-cto**: whether the bound is a contract's or the
  renderer's. **Ruled** (same reply): the contract's, in HUMAN-CHAT.md's
  Bounds bullet beside nesting and tables, with §6's figures and
  conditions, the plain-text fallback and Show source cut by §4.1's
  rule over source lines; architect-cto writes the amendment on their
  branch, and it corrects their earlier DECISION 01a1a899 to
  rust-ui-dev (a line-box cap with paged source), so both read this
  rule. The ruling took the first figures, 2 400 and 48; §6 lowered
  them to 2 000 and 40 after it, and architect-cto was told (INFO
  01a1a8a6-fc67). The build waits for the amendment carrying them.
- **Q3 — rust-ui-dev**: does the composer draw a draft at the ceiling
  without the same panic? **Answered** (#561, rust-ui-dev's j92 note):
  no, a draft of many lines panics the desktop at scale 1 and 2. What
  the person sees once it is fixed, the whole of what this note asks
  of the composer: the field grows with the draft up to six lines at
  the current text size, then keeps that height and scrolls inside
  itself, the caret kept in view; every character of the draft is
  kept and sent as today — nothing is cut, refused or
  dropped for its length below the envelope's ceiling. How the build
  bounds it is rust-ui-dev's. A long draft's other questions are not
  designed here.
- **Q4 — architect-cto** (OBSERVATION 01a1a8d3, #561 review round 9,
  F23): the contract's count, over the source text, missed what links
  and images draw. Will the count cover everything drawn, once per
  drawing, and how is one control's label kept under the ceiling beside
  the rule that it shows the full destination? **Ruled** (REPLY
  01a1a8dd-a73d; #562 at c0d2778a5, ADR-0050 rule 2 A 2026-11-04 (ii)):
  the count is of what is drawn, once per drawing; a link whose label
  would pass the figures gets no control and is drawn inert, its label
  never cut; the preview carries a control only with its whole label
  (§4.1, §6). The build waits for #562 on `main`.
- **Q5 — architect-cto** (asked of them by rust-ui-dev's measurement,
  OBSERVATION 01a1a923-4ca1): a grapheme cluster of hundreds of code
  points stops the desktop client wherever it is drawn. **Ruled**
  (DECISION 01a1a928-0d26 to rust-ui-dev; INFO 01a1a928-e1a9 to this
  note): a cluster past 32 code points is drawn as one U+FFFD in every
  place, Show source included, and counts as one. HUMAN-CHAT.md's "raw
  source viewable" is then met for such a cluster by Copy message
  source, which carries it byte for byte, and not by Show source; the
  contract's text is architect-cto's to align. How the mark looks is
  this note's (§6).

Left out on purpose: a hint on screen for a link drawn inert (§6);
search within a message; selecting part of a
message; a "show more" in steps inside the conversation (it grows the
row back to the ceiling); opening the reader at a position other than
the start.
