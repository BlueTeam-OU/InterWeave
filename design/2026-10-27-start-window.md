# Opening with the newest rows: the conversation window — desktop and Android

Status: proposal, 2026-10-27. Its product rules are architect-cto's
REPLY 01a17bcd (§1); this note designs only what a person sees and does.

Asked by rust-ui-dev-02 (QUESTION 01a17bce, job j68, the owner's
finding P2-1 judged to hold at main f2295901): at start the client
loads and decodes every stored row — pending, unread, kept, unread
cards, all contacts (`crates/human/app-core/src/listing.rs`,
`list_all`) — and a trusted peer's unread backlog, up to 48 KiB a row
with no lifetime count, can exhaust memory before the app is usable.
The fix is a bounded window: the newest rows up to a row and byte
budget, the rest left in the store until fetched. The four questions
asked were: the window's order and grain; what the person sees at its
edge; what opening a conversation does when unread rows lie beyond
it; and whether contacts and proposals ride the same window.

A provisional answer went to rust-ui-dev-02 as REPLY 01a17bd0; this
note is the binding one once reviewed, and where they differ the note
wins.

Owner of the design: ux-designer. The words are language-culture's:
§9 names each string, where it shows and what it must say. Built by
rust-ui-dev in `crates/human/` and the apps. The Chats list and the
conversation screen are those of `design/2026-10-16-android-look.md`
("look" below); its rules apply unchanged unless this note says
otherwise.

## 1. What is already decided

architect-cto ruled the window consistent with `RETENTION.md` and
`HUMAN-CHAT.md` (REPLY 01a17bcd) under four constraints, quoted as
the builder's message gives them:

- (a) the unread badge and notifications count every unread row, not
  the window;
- (b) pending rows are retried whatever the window;
- (c) a proposal not yet shown survives exit;
- (d) Keep, Dismiss, Cancel and Undo keep their rules in the window.

Unchanged by this note, and relied on by it:

- **Read is what it is today.** A row is read when it is held in a
  conversation that is open in an app window that has focus
  (`UiModel::conversation_viewed`): read is a retention act, never
  raised on receipt, from a notification or from a preview (look
  §5.2). The window changes which rows are held, so it changes which
  rows an open conversation reads, and nothing else: a row the window
  has not loaded is not read, keeps its stored copy and keeps its
  badge count.
- **A card proposal is read when it is shown** in an open, focused
  conversation (`Cards::proposals_viewed`), never through
  `conversation_viewed` (`crates/human/ui-model/src/model/cards.rs`).
  The window changes which proposals can be shown, nothing else.
- **A read row that is not kept lives in this session only**
  (`human-client-ui.md` §6), as does the person's own message once the
  transport accepted it. A row loaded by the window and then read
  becomes such a row. The session holds at most `SESSION_ITEM_CAP`
  (1,024) of them; past it, `UiModel::evict` drops the one that became
  session-only first, and never one an open snackbar's Undo needs
  (`crates/human/ui-model/src/model.rs`, `release` and `evict`). The
  bound is shared by every conversation. §7.1 keeps that order and
  changes only when paging may push rows past it. (`copy_gone` is a
  different bound: it withdraws Keep from a read row whose copy the
  root no longer holds, and removes nothing.)

## 2. The person's goal

Open the app and see every conversation they have, with the right
unread counts, at once and whatever a peer has sent; open one and read
what is new; go back through older messages when they want to; and
never be told something is waiting that they cannot find.

Evidence and hypotheses behind the choices below:

- *Visibility of system status* and *match between system and the
  real world* (Nielsen 1 and 2): a badge counting rows the person
  cannot reach is a status they cannot act on. This decides §3's
  grain.
- Platform convention (*consistency and standards*, Nielsen 4): the
  messaging apps a person uses on Android load older messages when the
  list reaches its top, with no button, and keep the reading position
  while they do. This decides §5.
- **Hypothesis, not evidence**: that opening at the newest rows with a
  count of the earlier unread ones (§6) serves a person returning to a
  flooded conversation better than jumping to the oldest unread, which
  is the usual messaging convention. It would be falsified by a person
  who, opening such a conversation, looks for the first unread message
  and does not find the way to it from the line; §10 says how to test
  it. The reason for taking it anyway is the case the window exists
  for: the oldest unread can be thousands of rows back, and reaching
  it means paging in both directions.

## 3. The grain: per conversation, never one budget for all

**Required.**

- **The Chats list is complete.** A conversation is listed if the
  store holds a row of it that the app can show (§6.1): pending, unread
  or kept. A conversation is also listed if the session holds read rows
  of it (look §5.2: listed while the app holds one of its messages). No budget leaves a
  conversation out. A conversation with no row the app can show,
  stored or held in the session, is not listed, as today; its
  undecodable rows are counted by §6.1's line.
- **Each Chats row's last message and time are those of the newest
  message the conversation has, held in the session or stored**. It
  is the newer of two rows: the newest row the session holds (a message
  read or sent and accepted this session, which has no stored copy),
  and the newest stored row the app can show (§6.1). That is look §5.2's rule, "the last
  message the conversation holds in this session, received or sent",
  extended to the rows the window has not loaded; it holds unchanged
  otherwise, above all that showing a preview never reads a message.
- **Each Chats row's unread badge counts every unread row of the
  conversation in the store** (a), loaded or not, with §6.1's
  exception for rows the app cannot show. Card proposals count in it:
  contact-cards §5.4.5 says so; the badge counts message items only
  today (`UiModel::conversations`), so this is a change for the
  builder.
- **A Chats row needs a summary, not the conversation's rows.** The
  rows of a conversation are loaded when it is opened, its newest rows
  up to the row and byte budget; a closed conversation holds none of
  its stored rows in memory, only its summary (a preview line, a time,
  a count). Starting the app loads summaries and the pending rows, and
  no conversation's window. One conversation's backlog never pushes
  another conversation's stored rows out of the list or its window.
  The session bound on read rows is shared (§1), and paging may push
  another conversation's read rows past it only by the person's
  choice (§7.1).
- **The number of conversations is bounded, and the person sees
  nothing at the bound.** A direct conversation is keyed by the remote
  EndpointId (`human-client-ui.md` §4), which the sending peer asserts;
  no conforming profile speaks from more than 64 endpoints
  (`architecture/contracts/ENDPOINTS.md`), and architect-cto rules that
  an inbound from a peer's 65th distinct source endpoint is treated as
  a malformed envelope: nothing stored, nothing shown, counted
  (REPLY 01a17be2, Q3; the rule is theirs to write into `HUMAN-CHAT.md`
  and `RETENTION.md` with j68's record). So the summaries held are
  bounded by the trust list times 64. Nothing is designed for the
  bound.
- Every notification counts as (a) says; a notification is raised on
  receipt, which the window does not change.
- **Every pending row is held, whatever the window**, so a failed
  send's Retry and Cancel stay reachable, and it shows in its
  conversation in its place. Pending rows have no count ceiling, but
  only the person's own Send, Share or Undo of a Cancel makes one; no
  peer can (rust-ui-dev-02, REPLY 01a17bd8, read at main f2295901).
  Whether a ceiling belongs here is §13 Q1.

**Rejected: one global budget**, newest first across all
conversations. A peer flooding one conversation would push every
other conversation's rows out of the window: those conversations would
vanish from Chats, or show a badge with nothing behind it.

**The builder's**: the budget figures; how a summary and a window are
held in memory, and whether a closed conversation's window is dropped
at once or kept for a while (it must be reloadable, and the person
sees no difference). One condition: if opening a conversation can take long enough for a
person to notice (more than about 100 ms on the Pixel 9a with a full
budget), say so and I add a loading state to this note; none is
designed now.

## 4. Opening a conversation

- It opens at its newest row, as today.
- The loaded rows are read under §1's rule. Unread rows beyond the
  window stay unread.
- If unread rows remain beyond the window, the line of §6 shows.

## 5. The edge: earlier rows load at the top

**The top edge** is the `EarlierUnread` line when it shows (§6),
otherwise the oldest loaded row.

**Trigger.** While the store holds rows of the conversation that the
app can show (§6.1) and that are older than the oldest loaded row, the
next page of older rows is fetched when the top edge:

- scrolls into view (a scroll, a fling, a drag of the scroll bar on
  the desktop), or
- receives focus moved by the person (Tab, Shift+Tab or the arrow keys
  on the desktop, TalkBack's swipe, switch access).

At the session bound, the limit row of §7.1 shows in place of a
fetch. Focus the app places (§7) triggers nothing, and no page is requested
while one is in flight. A page is requested once each time the top edge is reached: focus or
the view must leave the top edge and come back before the next page is
fetched, except that a page arriving below a focused line (below) makes
its rows reachable by moving focus down, without reaching the top edge
again.

The second trigger is the non-gesture equivalent of the first: a
person who never scrolls reaches the top by moving focus, and the
older rows load. No button, and so no new control and no new label for
one.

**While it loads.** A progress row shows above the oldest loaded row:
an indeterminate circular progress indicator (Material), centred, with
`EarlierLoading` as its accessible name. It is not focusable and is
not announced. It shows only if the page has not arrived within
250 ms, so a fast load does not flicker.

**When it arrives.** The page's rows are added in their order between
the `EarlierUnread` line, when it shows, and the row that was oldest:
the line stays at the very top. **Neither the scroll position nor
focus moves**: the row the person was looking at, or the item that has
focus, stays where it was on screen. With no line, the new rows lie
above the person's position and are reached by scrolling or moving
focus up; the new oldest row is then the top edge. With focus on the
line, the new rows lie below it in reading order and are reached by
moving focus down; the line remains the top edge. The loaded rows are
read under §1's rule.

**When the store holds nothing older that the app can show**, nothing
is shown at the top:
no "start of the conversation" marker. Read rows that were not kept
are gone after a restart, so such a marker would claim a beginning
that is not one.

**When it fails** (the store refuses the read): the progress row, if
shown, is replaced by a failure row: `EarlierLoadFailed`, then a Try
again button (`TryAgain`, which exists). The failure is announced once,
politely (`EarlierLoadFailed`). Scrolling or moving focus to the top
again does not retry by itself; Try again does, and the row becomes the
progress row again (subject to the 250 ms rule). Closing and reopening
the conversation also starts over. Focus, if it was on Try again when
the row is replaced, goes to the item just below it, as app-placed
focus (§7). The announcement is checked on the desktop now and is a
blocked precondition on Android (§7).

**A new message arriving** while the person is up in the history
behaves as today; the window does not change it.

## 6. Unread rows beyond the window

While the conversation is open and the store holds unread rows of it
that the window has not loaded, a line shows at the top of the loaded
rows, above the oldest one and below the progress or failure row:
`EarlierUnread`, with the count of those rows (messages and card
proposals together, as the badge counts both, §3).

- It is text, not a control. It is focusable so that TalkBack and the
  keyboard can reach it; as the top edge, focus moved onto it by the
  person triggers the load of §5.
- Its count follows the store: as pages load and their unread rows
  are read, the count goes down. At zero the line goes.
- **It agrees with the conversation's badge on Chats while the app's
  window has focus**, when every loaded unread row has been read. While
  the app's window does not have focus, loaded rows stay unread (§1), so the
  badge counts them and the line does not: the line counts only what
  has not been loaded, the badge every unread row.
- It is not announced: opening a conversation announces nothing today
  (`crates/human/ui-slint/README.md`), and this note adds no
  announcement. A screen-reader user meets it as the first item when
  they move focus to the top.
- It does not show when every unread row fits in the window, which is
  the ordinary case; nothing changes then.

### 6.1 Unread rows the app cannot show

A stored unread row that does not decode never becomes an item, so no
load shows it and no view reads it (`listing.rs`, `undecodable_unread`;
`model_side.rs`, `hidden_rows`). Neither the line nor the
conversation's badge counts it, as today's badge does not: (a) means
every unread row this build can decode, and the hidden-rows count is
the surface for the rest (architect-cto, REPLY 01a17be2, Q4). So the
line reaches zero, and goes, once every unread row the app can show is
loaded.

architect-cto's condition: the hidden count stays reachable when the
badge and the line read zero. Today only the command-line client
shows it (`apps/human-cli`); the desktop and Android apps show it
nowhere. A conversation can hold only such rows, which gives it no
Chats row to open, and a row can lack an origin the app can attribute
to any conversation (`listing.rs`, `inbound`), so the count is not
shown inside a conversation. Required: while
`ModelSide::hidden_rows` is above zero, the Chats list ends with a
line, `RowsNotShown`, with that count. The count is every stored row
that does not decode, not only unread ones: the person's own pending
sends and kept rows too (`hidden_rows` is `hidden_unread` plus
`hidden_other`). It is text, not a control,
focusable, not announced, and styled as the `EarlierUnread` line
(§8). It says that these messages are still stored and that this
version of the app cannot show them; it claims nothing about who sent
them or why. It is the same count the command-line client prints; no
per-conversation count is needed.

## 7. Focus

Every case where focus can be on something the window removes or
replaces, with its target and check. Look's rules for the conversation
screen hold for every other case.

| Focus was on | It disappears when | Focus goes to | Check |
|---|---|---|---|
| any row, or the `EarlierUnread` line | a page is added above it or below the line | stays where it was; nothing moves | §10 items 4 and 5 |
| the `EarlierUnread` line | its count reaches zero | the row that was just below the line, as app-placed focus (no fetch, §5) | §10 item 6 |
| the failure row's Try again | it is pressed and the row becomes the progress row, or the page arrives | the row that was just below the failure row (the line, else the oldest loaded row), as app-placed focus | §10 item 7 |
| the limit row's `EarlierShowAnyway` | it is pressed and the row becomes the progress row, or the page arrives | the item that was just below the limit row, as app-placed focus | §10 item 12 |
| a row of the open conversation | the session bound drops it | cannot happen: the focused row is never dropped (§7.1) | §10 item 12 |
| a `ReadNotShown` line | a new drop adds to its count | stays on the line, whose count changes | §10 item 12 |

### 7.1 The session bound while a conversation is open

The bound and its order stay as today (§1): it is shared by every
conversation, and past it the row that became session-only first is
dropped, never one an open Undo needs. Paging up through a flood reads
each page as it loads, so it fills the bound; past it, today's order
drops first the read rows of other conversations that were read
earlier (a conversation listed only because of such a row leaves
Chats, look §5.2), then the open conversation's newest rows, which
were read first on opening. Those rows have no stored copy: they are
gone for good. No order avoids losing something read; what this note
requires is that **paging never drops a read row on its own**.

**Required.**

- When fetching the next page would take the session past the bound
  (the page's rows that would become session-only, counted by the
  builder), no page is fetched by scrolling or focus. The top edge
  shows instead a **limit row**: `EarlierAtLimit`, then a button,
  `EarlierShowAnyway`. The text says, before anything is committed,
  that showing earlier messages removes from the screen the messages
  read or sent first in this session, here and in other conversations, that
  they cannot come back, and that a message the person keeps stays.
  Nothing is dropped while the row only shows.
- Pressing `EarlierShowAnyway` fetches the next page; the bound drops
  rows in today's order. From then until the conversation is closed,
  paging is automatic again (§5) and drops as it needs: the person
  chose that once for this opening. Closing and reopening the
  conversation asks again.
- The unread rows not loaded stay unread and stored: the badge and
  `EarlierUnread` stay true, and they can be read later (after other
  read rows leave the session, or after a restart).
- Wherever rows of the open conversation were dropped, a line stands
  in their place: `ReadNotShown`, with the count dropped there, one
  line for each run of consecutive dropped rows. Today's order does not
  drop them as one block: rows read in one batch, messages that
  arrived while the conversation was open and kept rows (never
  dropped) can leave several runs, with remaining rows between them.
  A run that reaches the top edge has its line below the top edge's
  items; paging past it brings rows older than the run. It looks and behaves as the
  `EarlierUnread` line (§8): text, focusable, not announced. The rows
  it stands for have no stored copy and cannot be loaded again.
- Rows of other conversations are dropped as today, with nothing said
  in them; their summaries follow §3.
- **Opening a conversation is not covered by the limit row.** It reads
  its window at once (§4), as today, and near the bound that can drop
  other conversations' read rows, and a conversation listed only
  because of one leaves Chats, with nothing said, as today. This note
  changes that only for paging; asking before opening is left out
  (§12).
- The focused row is never dropped while focused; if today's order
  reaches it, the next row in that order is dropped instead.

## 8. Accessibility

- **Reading order** at the top of a conversation: the failure row
  (`EarlierLoadFailed`, then Try again), the limit row
  (`EarlierAtLimit`, then `EarlierShowAnyway`), or nothing (the
  progress row is not focusable); then `EarlierUnread` when shown; then
  the loaded rows. A `ReadNotShown` line is read where it stands. On
  Chats, `RowsNotShown` is read after the last conversation.
- **Look**: the `EarlierUnread`, `ReadNotShown` and `RowsNotShown`
  lines are styled
  as the day separator (look §6): centred, body small,
  `on-surface-variant`, one item each for a screen reader. The
  failure row is centred: `EarlierLoadFailed` in body medium in
  `error`, and under it a Material text button, Try again. The
  progress row is the indicator alone, 24 dp, centred, with 8 dp
  above and below. The limit row is laid out as the failure row:
  `EarlierAtLimit` in body medium in `on-surface-variant`, and under
  it a Material text button, `EarlierShowAnyway`.
- **Every gesture has a non-gesture equivalent**: §5's focus trigger.
- **Reduced motion**: the progress indicator follows the platform's
  setting as Material's does; no row animates in.
- **Text size**: at the largest font and display size, the
  `EarlierUnread` line and the failure row wrap and are never cut;
  the Try again target is at least 48 by 48 dp.
- **Contrast**: everything here sits on the conversation's background,
  `surface`, except `RowsNotShown`, which sits on Chats' background:
  `surface`, or `surface-container` in the wide-screen pane. Each pair
  is one look §3 already holds:

  | Mark | Pair | Needs | Dark | Light |
  |---|---|---|---|---|
  | `EarlierUnread`, `ReadNotShown`, `RowsNotShown`, `EarlierAtLimit` | `on-surface-variant` on `surface` | 4.5 | 9.9 | 7.6 |
  | `RowsNotShown` in the wide-screen pane | `on-surface-variant` on `surface-container` | 4.5 | 9.2 | 7.0 |
  | `EarlierLoadFailed` | `error` on `surface` | 4.5 | 10.6 | 6.2 |
  | Try again, `EarlierShowAnyway`, their focus ring, the progress indicator | `primary` on `surface` | 4.5 (3 for the ring and indicator) | 10.8 | 6.2 |

## 9. Strings

The words are language-culture's; these say what each must do.

| Key | Proposed English | Shows | Must say |
|---|---|---|---|
| `EarlierUnread` | Earlier unread messages and contact cards: {count} | the line of §6 | how many unread items are above, not yet shown; the count after a colon, one text for every count (below); must not say "messages" if it counts cards too, or must cover both |
| `ReadNotShown` | one: 1 message that you read or sent is no longer shown. It cannot be shown again. other: {count} messages that you read or sent are no longer shown. They cannot be shown again. | the line of §7.1 | that messages the person already read or sent were removed from this screen and cannot come back; a plural by count; must not suggest they were deleted by anyone else |
| `RowsNotShown` | one: 1 message is saved on this device, but this version of InterWeave cannot show it. other: {count} messages are saved on this device, but this version of InterWeave cannot show them. | the line at the end of Chats (§6.1) | that these messages (received unread, kept, or the person's own still waiting to send) are still stored and this app version cannot show them; must not use "kept" in the sense of Keep; a plural by count; must not blame a sender or suggest tampering |
| `EarlierAtLimit` | If you choose Show earlier messages, InterWeave makes room for the earlier messages by no longer showing some messages that you read or sent in this session. It begins with the ones you read or sent first, in any conversation, including this one. It never stops showing kept messages. The messages that it no longer shows cannot be shown again. | the limit row (§7.1) | before anything happens: what showing earlier messages removes (messages read, and the person's own once accepted, first in this session, in every conversation), that it cannot be undone, and that kept messages stay |
| `EarlierShowAnyway` | Show earlier messages | the limit row's button (§7.1) | the action, which the text above it qualifies |
| `EarlierLoading` | Loading earlier messages | the progress row's accessible name (§5); never visible text | that older rows are on their way |
| `EarlierLoadFailed` | Could not load earlier messages. | the failure row (§5), and its announcement | that the older rows could not be read; must not suggest they are lost |
| `ContactsLoadFailed` | Could not load all contacts. | the Contacts list's failure row, when a page of a paged list cannot be read (§11), and its announcement | that some contacts could not be read just now; must not suggest they are lost |

`TryAgain` exists and is reused.

`ReadNotShown` and `RowsNotShown` have one text for a count of 1 and
one for every other count: two templates, never a word joined to a
number. `EarlierUnread` puts the count after a colon, so it needs one
text only. A conversation can hold card proposals as well as messages,
which is why it names both.

## 10. Acceptance

Run on the desktop and on the Pixel 9a, with a store a test fills: one
conversation "A" with more rows than the window's row budget, 10 of
them unread, followed by more unread rows than the window holds; a
second conversation "B" with two read-and-kept rows and one unread row
newer than both.
Every TalkBack check below is a blocked precondition on Android (§7);
its desktop counterpart, through Slint's accessibility tree, runs now.

1. **List complete under flood**: start the app. Chats lists A and B.
   A's badge shows A's unread count, card proposals included; B's
   shows 1; B's preview is its newest row.
2. **Preview reads nothing**: with Chats shown, restart the app. Both
   badges are unchanged (look §5.2).
3. **Opening**: open A in an app window that has focus. It opens at its newest
   row, and nothing is announced. `EarlierUnread` shows at the top of
   the loaded rows with the count of unread rows not loaded; A's badge
   on Chats equals it.
3a. **Preview of a session-only row**: open B (its unread row is read,
   its stored copy gone) and go back to Chats. B's preview and time are
   that row's, not the older kept row's. Send a message in B and let it
   be accepted: B's preview is "You: " and that message.
4. **Load by scrolling, position kept**: scroll A to its top. The next
   page loads above (the progress row only if it takes longer than
   250 ms); the row that was at the top stays where it was on screen;
   `EarlierUnread`'s count drops by the unread rows in the page.
5. **Load by focus**: reopen A. Without scrolling, move focus to the
   top (Shift+Tab on the desktop; TalkBack's swipe on the phone). The
   page loads when focus reaches the `EarlierUnread` line; focus stays
   on the line, no second page loads, and the next item down is the
   oldest row of the new page. Move focus down into the page and back
   up to the line: the next page loads.
6. **Line goes at zero**: keep loading until every unread row is
   loaded. The line goes; if it had focus, focus is on the row that
   was just below it, and no page is fetched by that move. A's badge
   on Chats is 0.
7. **Failure and Try again**: make the next page's read fail (a test
   hook). The failure row shows with Try again and its text is
   announced once (the desktop's screen reader now; TalkBack blocked).
   Scrolling to the top again retries nothing. Remove the failure; Try
   again loads the page; focus, if it was on Try again, is on the item
   that was just below the failure row.
8. **Nothing at the start**: load until the store has nothing older.
   No marker shows at the top.
9. **Unloaded unread survive**: open A, read only the newest window,
   close the app. Restart: A's badge shows the unread rows that were
   never loaded.
10. **Constraints held**: a pending row that was never loaded is
    retried (b); a card proposal never loaded is listed after a
    restart (c); Keep, Dismiss, Cancel and Undo on a loaded row
    behave as their specs say (d).
11. **Contacts**: with more contacts than any budget, the Contacts list
    shows every contact, with no edge or progress row, and every
    conversation title on Chats uses its contact's name.
12. **The bound while open**: with a test bound small enough to reach
    (the builder's hook), read B's unread row, then open A and page up
    until the next page would pass the bound. No page is fetched; the
    limit row shows with `EarlierAtLimit` and `EarlierShowAnyway`;
    nothing has been dropped (A's newest rows and B are as they were).
    Press `EarlierShowAnyway`: the page loads, focus is on the item
    that was below the limit row, B's read row goes first (B leaves
    Chats if it held nothing else), then A's rows in today's order,
    and each run of dropped rows has one `ReadNotShown` line with its
    count, where the run was. Paging goes on without asking until A is closed; reopening
    A and reaching the bound shows the limit row again. Keep a row
    before pressing: it stays. A row with an open snackbar is not
    dropped; the focused row is not dropped.
13. **An undecodable unread row**: add one to A's store, as its oldest
    row. It is not counted on the line or on A's badge; the line goes
    once every other unread row is loaded, and once the store holds
    nothing older that the app can show, paging stops (no further
    fetch at the top); with A's badge at 0, Chats ends with
    `RowsNotShown` showing 1. Add a second undecodable row to a third
    conversation with no other row: it has no Chats row, and
    `RowsNotShown` shows 2. Add a third undecodable row as B's newest stored
    row, with the app restarted so the session holds none of B's
    rows: B's preview and time are those of its newest row that
    decodes, and `RowsNotShown` shows 3.

**Test of §2's hypothesis** (with people, when there is a session to
run): give a participant the flooded A and ask them to find the first
message they have not read. Success: they find it without help, by
scrolling or moving focus up, and say what the line told them.
Record hesitation, scrolling down to look, and misreadings of the
line. Two of five failing is enough to reopen §6.

## 11. Contacts and card proposals

- **Card proposals are conversation items** (`human-client-ui.md`
  §3), so they ride their conversation's window like messages, and are
  counted by `EarlierUnread`. A trusted peer can send many cards, so
  they are not always few. A proposal not yet loaded is unread and
  survives exit (c).
- **What a proposal shows that depends on other rows** — the
  same-name note, the contact it will update, a key change — comes
  from the store, never from what the window holds.
- **The Contacts list has no visible edge.** It grows only by the
  person's own Accepts. If the builder pages it, the paging is not
  seen: no progress row, no failure row unless a read fails (then
  `EarlierLoadFailed`'s pattern, with `ContactsLoadFailed`). Every contact's name must be known for
  conversation titles and for proposals' notes whatever is loaded.

## 12. Deliberately left out

- **Read by what is on screen** instead of by what is held. Today a
  conversation reads every row it holds, on screen or not; this note
  keeps that, and a page loaded above the person's position is read
  before they scroll to it.
- **A jump to the first unread, or to the newest from far up.** Not
  designed; §10's test decides whether the first is needed.
- **A warning before opening a conversation near the session bound**
  (§7.1): opening drops read rows as it does today.
- **A loading state for opening a conversation** (§3), until a
  measurement says it is visible.
- **How the window is held and its budget figures**: the builder's.
  When paging may push read rows past the session bound is §7.1's.

## 13. Questions

- **Q1, architect-cto** (put to them by rust-ui-dev-02 with REPLY
  01a17bd8): pending rows have no count ceiling, so holding every one
  (§3) is safe from a peer but not bounded. A ceiling would be a
  product rule (Send refused past a count, with a reason); whether it
  belongs to this fix or a later one is theirs. If one comes, this
  note adds its refusal state.
- **Q2, rust-ui-dev**: answered by the code. The session bound drops
  the row that became session-only first (`UiModel::evict`), so it
  would drop the newest rows of a flood first; §7.1 keeps that order
  and makes paging past the bound the person's informed choice.
- **Q3, architect-cto**: answered (REPLY 01a17be2). At most 64 direct
  conversations per peer; a 65th source endpoint is malformed,
  counted, and shows nothing (§3).
- **Q4, architect-cto**: answered (REPLY 01a17be2). (a) is every
  decodable unread row; the hidden count is the rest, reachable at
  zero (§6.1).
