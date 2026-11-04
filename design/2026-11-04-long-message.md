# A very long message: a bounded part in the conversation, the whole in a reader — desktop and Android

Status: proposal, 2026-11-04. Asked by rust-ui-dev (QUESTION 01a1a896,
their job j92, my job j90), read against `origin/main` ed46059ff. It
answers their three questions (§4, §5, §6) and declines their
alternative (§2). Where the bound sits in the contracts is
architect-cto's to decide, on rust-ui-dev's OBSERVATION of the same
subject; this note gives what the person needs from it, and its
figures are proposals that architect-cto's decision may move within
§6's conditions. Copy of a message (§5.3) is a new action, asked of
architect-cto in §11.

Owner of the design: ux-designer. The words are language-culture's: §8
names each string and what it must say. Colours are roles of
`design/2026-10-16-android-look.md` §3. Both clients render the same
Slint UI (`crates/human/ui-slint`) from the same `ui-model`; "narrow"
and "wide" are the layouts `app.slint` already has. Built by
rust-ui-dev.

This note replaces the look spec's edge case "A very long message"
(`design/2026-10-16-android-look.md` §8) for a body past §6's bound;
below it that line stands as written.

## 1. What is already decided

- **HUMAN-CHAT.md §Text is markdown**: a body is up to 196 608 bytes
  decompressed; render cost must stay linear in its size; the raw
  source is always available to the receiver, however it rendered.
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
drawn in bounded pieces, which fixes the panic alone). The panic is
one cost; the person's is another. At the ceiling, a body is some
49 000 drawn lines. In place, the messages after it are reachable only
by scrolling through all of it, on a phone tens of thousands of
flings, and the conversation's own position is lost in it. That fails
user control and freedom and flexibility and efficiency of use
(Nielsen's heuristics 3 and 7): the person cannot step past content
they did not ask for. Messaging apps on both platforms cut a long
message in the conversation and open the whole in a view of its own
(the Android Signal client's "Read more", Gmail's "Message clipped —
View entire message"). That is platform habit, not proof; the
hypothesis this note makes, and how it is checked, is §10's H1.

## 3. What the person sees — the states

A message's body is **cut** when it passes §6's bound, **whole**
otherwise. Nothing changes for a whole body.

| state | in the conversation | in the reader (§5) |
|---|---|---|
| whole | the body, as today | not offered |
| cut, formatted | the preview (§4.1), the cut line (§4.2), Read the whole message (§4.3) | the whole body, formatted |
| cut, source | the source's preview, the cut line, Read the whole message | the whole source |
| reader opening | — | its first screen within §9's figure; no spinner below 0.5 s, then an indeterminate progress bar under the app bar until the first screen is drawn |
| row gone while the reader is open | — | the reader closes (§5.4) |

A sent message is treated as a received one: the person may have
pasted something long themselves. Its delivery state stays under the
bubble, after the cut line and the button.

## 4. Where the body stops (question 1)

### 4.1 The preview

- The preview is the start of the body as the whole body's parse
  renders it — never a parse of a cut source, so emphasis, a list's
  numbering or a code block that runs past the cut look in the preview
  as they do in the reader.
- It ends at the last block boundary (a paragraph, a list item, a table
  row, a code line, a heading) inside the bound when that boundary is in
  the bound's last quarter; otherwise at the last word boundary inside
  it, then an ellipsis (`…`) at the end of the cut line. Never inside a
  grapheme cluster. A table cut between rows keeps its header row.
- A link whose text is cut is not in the preview, and its control
  (the one after the text) is not drawn; it is in the reader.
- In source mode (Show source, as today) the preview is the source's
  start, cut by the same rule over its lines.

### 4.2 The cut line

Under the preview, inside the bubble, in `on-surface-variant` on the
bubble's colour (contrast §9): one line saying the message continues
and **how much of it is shown**, as a whole percentage —
`LongMessageShown` (§8). A percentage, not lines or words, because
it is the one measure that does not depend on how the text wraps, which
script it is in or how the person counts: drawn lines change with the
width and text size, words do not exist in a script written without
spaces, and characters mean nothing to most people. It is the share of
the body's characters in the preview, rounded down, at least 1 %.

### 4.3 Read the whole message

A text button, `ReadWholeMessage` (§8), under the cut line at the
bubble's start edge, with the 48 dp target every control keeps. It
opens the reader (§5) at the start of the body, in the mode the row
is in (formatted or source). It is also an item in the row's menu,
after Show source, and an accessibility action of the row, so it is
reachable from the menu, the keyboard and TalkBack's actions as well
as by a press. No gesture opens it.

Keep, Hide and Show source stay on the row and act on the whole
message, as today.

## 5. The reader, and the source past the bound (question 2)

### 5.1 The page

A page of its own: on a narrow layout it covers the app, on a wide one
it takes the conversation's pane with the list beside it, as the
contact page does. Its top app bar: Back (the navigation icon), the
title `ReaderTitle` (§8, naming the sender, and the time as the row
shows it), and two actions — Show source / Show formatted (the same
labels as the row's), and Copy (§5.3). Below the bar, the whole body
in one continuous scroll, drawn as the person scrolls so that only
what is near the screen is laid out. Its look is the bubble's text
on `surface`, without the bubble.

**Not pages.** The whole body is one scroll, not pages of N lines:
paging breaks a log's line or a paragraph at an edge the content did
not choose, puts a control between the person and the next line, and
no app on either platform reads text that way. Scrolling is the
platform's convention; the scroll bar (desktop) and the fast-scroll
thumb (Android, shown once the body is taller than four screens) give
the position and a way to cross it.

### 5.2 Source in the reader

Show source in the reader shows the whole raw source, in the code font,
in the same scroll, wrapping as the row's source does. A source line
longer than the screen wraps over as many lines as it needs; however
it is cut into pieces to be drawn, no seam, gap or repeated character
shows where one piece meets the next. The switch keeps the position
as nearly as the two forms allow: the first block on screen before the
switch is on screen after it.

### 5.3 Copy

Copy puts the **whole raw source, byte for byte as received**, on the
clipboard, in one action, whatever part the reader shows and whichever
mode it is in. It is also an item of the row's menu for a cut message
(`CopyMessage`). Feedback: on Android 13 and later the system shows
its own clipboard confirmation and the app adds none (the platform's
guidance); before 13, and on the desktop, a snackbar `MessageCopied`.
If the platform refuses the copy, a snackbar `MessageNotCopied` says so;
nothing else changes. Copy is not undoable and needs no undo: it
changes nothing in the app.

Whether Copy is offered on a whole message too is §11's Q1's; this note
needs it only where the body is cut, since there the reader is the only
other way to the whole.

### 5.4 Leaving the reader

Back (Android's system Back and the app bar's), Escape on the desktop:
the reader closes and the conversation is where it was. The reader also
closes, with no message of its own, when its row leaves the
conversation (the conversation hidden from the list beside it, on a
wide layout) — the person did that themselves.

## 6. The bound (question 3)

**Both, characters and lines.** A body is cut when it has more than
**2 400 characters** or more than **48 lines**, and the preview is then
at most **1 600 characters** and **32 lines**. A character is a
grapheme cluster; a line is one of the model's lines (a paragraph, a
list item, a table row, a code line, a heading, a rule), however it
wraps. The gap between the two figures is so that a body just over the
bound is not cut by a sentence.

Why both: characters bound a body that is one long paragraph, which
wraps into many drawn lines without a line of the model; lines bound a
list of one-letter items, which has many lines and few characters.

Why these figures, and the conditions any other figures must keep:

- **The person's**: at 100 % text on a phone the preview is about one
  and a half screens — enough to tell what the message is, never so
  much that the next message is far.
- **At 200 % text**: the preview is taller but bounded, and every
  whole body at the bound, on the narrowest layout, at the highest
  display scale either client supports, draws under the renderer's
  ceiling. My estimate on the Pixel 9a (2.625 dp per px, a 32 sp line
  about 126 px, about 17 characters per line in a narrow bubble):
  2 400 / 17 + 48 ≈ 190 drawn lines, about 24 000 px. That is an
  estimate, not a measurement; §10's A4 is the measurement, and if it
  does not hold the figures go down, not the text size.

## 7. Read, Keep and what a cut changes nothing of

- A cut message is read when its conversation is shown, as any message
  is (§1): opening the reader does not read it, and not opening it
  does not keep it unread.
- Keep is on the row and keeps the whole message. A person who wants
  to come back to a long message after the app closes keeps it; the
  cut line is what tells them there is more to come back to.
- The chat list's one-line subtitle and the notification's text are
  unchanged.

## 8. Strings

Names and what each must say; the words are language-culture-en's, and
every locale's theirs.

| key | where | must say |
|---|---|---|
| `LongMessageShown` | the cut line (§4.2) | that the message continues past what is shown, and the share shown as a percentage (one value, `{percent}`, formatted by the locale) |
| `ReadWholeMessage` | the button and menu item (§4.3) | that it opens the whole message to read |
| `ReaderTitle` | the reader's app bar (§5.1) | the sender's name (`{sender}`) and when it was sent, as the row shows it (`{time}`) |
| `CopyMessage` | the reader's action and the row's menu (§5.3) | that it copies the whole message, as it was written |
| `MessageCopied` | snackbar (§5.3) | that the whole message is on the clipboard |
| `MessageNotCopied` | snackbar (§5.3) | that the message could not be copied |

Show source and Show formatted reuse the row's existing labels.

## 9. Accessibility

**Focus table** — every case, and its check (§10).

| case | focus goes to | check |
|---|---|---|
| Read the whole message pressed (button, menu or action) | the reader's first content element: on Android, TalkBack's focus on the app bar's title; on the desktop, the scroll area, so arrows and Page keys scroll at once | A6 |
| Reader closed (Back, Escape) | the row's Read the whole message button | A6 |
| Reader closed because its row left | the conversation's composer; on a wide layout with the conversation gone too, the list | A7 |
| Show source / Show formatted in the reader | stays on the action pressed | A6 |
| Copy pressed | stays on Copy | A6 |
| The app sent to the background and back with the reader open | the reader, at its position, focus where it was | A7 |

- **Reading order** in a cut row: the preview, the cut line, Read the
  whole message, then the preview's link controls, then the row's
  delivery state. TalkBack reads the cut line as text, never as part
  of the button's label; the button's label is its own string.
- **Keyboard** (desktop): Tab reaches Read the whole message after the
  row's text; in the reader, Tab goes Back, Show source, Copy, the
  scroll area; Up, Down, Page Up, Page Down, Home and End scroll;
  Escape closes.
- **TalkBack in the reader**: swiping forward reads the body block by
  block, scrolling as it goes, to the end of the body, though only
  what is near the screen is laid out. If the build's accessibility
  tree cannot expose blocks that are not laid out, that is a
  precondition A8 states: blocked, never passed.
- **Contrast**: the cut line's `on-surface-variant` holds 4.5:1 on
  both bubble colours (received, sent) in both schemes, and the button's
  `primary` 4.5:1 on both; the focus ring 3:1 on both bubbles and on
  the reader's `surface`.
- **Motion**: the reader's arrival is the platform's page transition;
  under Remove animations it appears without one. Nothing else moves.
- **RTL**: the cut line and the button sit at the bubble's start edge,
  on the right; the ellipsis ends the cut line on its left; the
  percentage is the locale's (`٪` and Arabic-Indic digits where the
  locale writes them). The reader mirrors as every page does.

## 10. Acceptance

Run on the Pixel 9a and the A40 for Android, the desktop build at
scale 1 and 2, with a body at the 196 608-byte ceiling of each shape:
one long paragraph, a list of one-letter items, a code block, a table
of 256 rows. Each is checked in formatted and source mode, at 100 % and
200 % text, left to right and right to left.

- **A1** A body of 2 400 characters and 48 lines is whole; one more
  character, or one more line, and it is cut (§6).
- **A2** A cut row shows the preview, the cut line with a percentage
  equal to the preview's characters over the body's, rounded down and
  at least 1, and Read the whole message; its preview ends as §4.1
  says, and a table cut keeps its header.
- **A3** With a cut message in the middle of a conversation, the next
  message is on screen after at most two screens' scrolling past its
  start.
- **A4** At 200 % text, the narrowest layout and the highest scale, no
  row is taller than the renderer's ceiling: the build measures the
  tallest row of A1's whole body and reports it, and nothing panics
  for any shape.
- **A5** The reader opens on each shape with its first screen drawn
  within 0.5 s on the A40, optimised build; End reaches the body's last
  line within 1 s; scrolling shows no blank screen and no seam (§5.2).
  The build measures and reports both times; a figure it cannot reach
  comes back to ux-designer, not changed quietly.
- **A6** Every row of §9's focus table but the last two, by keyboard on
  the desktop and TalkBack on the Pixel.
- **A7** The reader's row removed while it is open, and the app sent to
  the background and back, as §9 says.
- **A8** TalkBack reads a cut message's reader from its start to its
  last block by swiping forward alone. Precondition: an accessibility
  tree that exposes the unlaid blocks (§9); without it, blocked.
- **A9** Copy, from the reader in each mode and from the row's menu,
  puts on the clipboard bytes equal to the received source (compared
  with the stored source), and the feedback of §5.3 shows.
- **A10** A cut message is read when its conversation is shown, and
  Keep on it keeps the whole message: after a restart the reader opens
  on all of it.

**H1** (the hypothesis of §2, unvalidated): a person given a
conversation with a long pasted log in its middle, asked to find the
message after the log and then the log's last line, does both without
backtracking. It is falsified if, in a walkthrough with a person who
did not see this note, they scroll the preview looking for the rest,
miss the cut line, or look for the next message inside the reader.

## 11. Questions, and what is left out

- **Q1 — architect-cto**: Copy of a message's raw source (§5.3) is an
  action the client does not have: a peer's text leaves the app on the
  person's press. Is it allowed, on a cut message only or on every
  message? Until it is answered, §5.3 and A9 are not built; the reader
  is.
- **Q2 — architect-cto**: whether the bound is a contract's (as
  HUMAN-CHAT.md's markdown bounds are) or the renderer's, on
  rust-ui-dev's OBSERVATION. This note needs only §6's conditions kept.
- **Q3 — rust-ui-dev**: does the composer draw a draft at the ceiling
  without the same panic? A long draft is not designed here.

Left out on purpose: search within a message; selecting part of a
message; a "show more" in steps inside the conversation (it grows the
row back to the ceiling); opening the reader at a position other than
the start.
