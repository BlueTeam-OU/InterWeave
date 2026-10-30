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

Revised for rust-ui-dev-02's four further questions (QUESTION
01a17dc9, U1 to U4, asked while building this note): the held-row
ceiling and gaps (§5.1), the limit row judged at the bound and not a
page early (§7.1), photos loaded on demand and the one contact a
proposal names (§11, and contact-cards §5.4.1). Revised again for
rust-ui-dev-02's QUESTION 01a17e5d (U-a, U-b): the preview's bound
(§3) and the Contacts list when its read fails, with what that
withholds (§11). Revised again for rust-ui-dev-02's
QUESTION 01a180cb: what one conversation's backlog may do to another
(§3). And for its QUESTION 01a18b25 (B3's views): the progress row
at the very top, held proposals and gaps, which side of a gap
fetches, when a conversation is closed, and the Decline-only
proposal (§5, §5.1, §7.1, §10, §11).

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
- **The preview text is bounded** (rust-ui-dev-02's QUESTION
  01a17e5d, U-a). The text is look §5.2's: the first line, with
  markdown marks removed, and a link showing its text. It is computed
  first and then cut, so a cut never leaves half a mark showing. It
  holds at most 160 characters as a person counts them (grapheme
  clusters) and at most 1 KiB of UTF-8, whichever comes first, cut
  only between two whole characters. A cut text ends in an ellipsis
  at its logical end, even when the row has room for more, so it never
  passes for the whole first line. The ellipsis is added after the cut
  and counts in neither bound. A first character larger than 1 KiB on
  its own (a base with hundreds of combining marks) leaves nothing
  that fits: the preview is the ellipsis alone. Rows the window holds and rows it
  does not are cut the same way, so a preview does not change when its
  row loads. The row's accessible description (look §9,
  `ConversationDescription`) carries the same cut text. 160 is the
  builder's proposal: it is more than a one-line subtitle shows on the
  widest Chats pane, and it is short enough to keep the start bounded.
  It is accepted on that reasoning, unmeasured.
- **Each Chats row's unread badge counts every unread row of the
  conversation in the store** (a), loaded or not, with §6.1's
  exception for rows the app cannot show. Card proposals count in it:
  contact-cards §5.4.5 says so; the badge counts message items only
  today (`UiModel::conversations`), so this is a change for the
  builder.
- **A Chats row needs a summary, not the conversation's rows.** The
  rows of a conversation are loaded when it is opened, its newest rows
  up to the row and byte budget; a closed conversation needs none of
  its stored rows in memory, only its summary (a preview line, a time,
  a count), and whether the builder keeps them a while is its choice
  (below), which the person cannot tell. Starting the app loads summaries and the pending rows, and
  no conversation's window. One conversation's backlog never pushes
  another conversation's stored rows out of the list, its badge or
  its preview: the held-row ceiling may release another conversation's
  held stored rows (§5.1), and they come back when the person reaches
  them, nothing of them lost (rust-ui-dev-02's QUESTION 01a180cb, which
  read this sentence so). Read rows that have no stored copy are the
  session bound's, not this rule's: past it another conversation's read
  rows are dropped for good, which can change its preview or take it
  off Chats (§7.1): when paging after the person's choice, when a
  conversation is opened near the bound (§7.1, §12), and when rows
  arriving in the open conversation are read as they come (§1), as
  today.
  The session bound on read rows is shared (§1), and paging may push
  another conversation's read rows past it only by the person's
  choice (§7.1).
- **The number of conversations is bounded, and the person sees
  nothing at the bound from a peer.** A direct conversation is keyed by the remote
  EndpointId (`human-client-ui.md` §4), which the sending peer asserts;
  no conforming profile speaks from more than 64 endpoints
  (`architecture/contracts/ENDPOINTS.md`), and architect-cto rules that
  an inbound from a peer's 65th distinct source endpoint is treated as
  a malformed envelope: nothing stored, nothing shown, counted
  (REPLY 01a17be2, Q3; the rule is theirs to write into `HUMAN-CHAT.md`
  and `RETENTION.md` with j68's record). So the summaries held are
  bounded by the trust list times 64. Nothing is designed for an
  inbound at the bound.
- **The person's own send at the bound has words of its own**
  (rust-ui-dev-02's QUESTION 01a17e74). architect-cto ruled (A2) that
  the person's own direct send counts toward the same 64: a send that
  would open a 65th endpoint of one peer is refused before its pending
  row is written, and shown as any refused send is (Not sent, with
  the reason). The 64 are what the store holds for that peer
  (`HUMAN-CHAT.md`, `STATE.md`): the endpoints of its conversations,
  of every local identity, together with the routes kept under
  contacts, an endpoint forgotten only when both are gone. So the
  identity in front may have fewer than 64 conversations with that
  peer, or none, and the reason must not send the person to
  conversations that are not there, or that belong to another
  identity (ADR-0061: the client never leads a person to act as one
  identity while believing they act as another). The reason is not
  `NotConfigured`'s "This route or channel is not configured here",
  which points at a setting, and no setting changes the bound
  (*help users recognize, diagnose and recover from errors*, Nielsen
  9). Since a conforming peer speaks from at most 64 endpoints, a
  65th typed one may well be mistyped, so the reason may suggest
  checking the route, without blaming the person. Its key is `NotSentEndpointLimit` (§9). Nothing
  else differs from any refused send: focus and the composer's
  contents stay as that rule leaves them.
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
fetch, where §7.1 asks (not when the rows still to load hold no unread
row). Focus the app places (§7) triggers nothing, and no page is requested
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
250 ms, so a fast load does not flicker. **At the very top** (rust-ui-dev-02's
QUESTION 01a18b25, U1): while the view is at its very top, each row of
the top edge (the progress row, the failure row, the limit row) shows
in view when it appears, so the person sees it; and each time one
appears, is replaced by another (a failure in the progress row's
place, the progress row in Try again's) or goes, what is below it
moves by the change in height, once each time and without animation,
and focus stays on its item. A failure or limit row whose button was
pressed stays where it is, its button doing nothing more while the
read is in flight, until the progress row takes its place (after
250 ms) or the page arrives, so pressing never moves the view twice.
One case keeps the person's place instead: when the page arrives with
no `EarlierUnread` line, the page's rows take the place the top edge's
row leaves, whichever row it was, so the oldest row the person was
looking at does not move back. With the line, which no message row
goes above, the line moves back up when the top edge's row goes. If
the top edge is reached by focus with the view elsewhere, the row
shows where it stands and the view does not move to it.

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

### 5.1 The held-row ceiling: rows released, never lost

Asked by rust-ui-dev-02 (QUESTION 01a17dc9, U1). The stored rows the
app holds in memory (unread and kept rows, which keep their stored
copy) need a ceiling of their own beside the session bound of §7.1,
across every conversation's window; the figure is the builder's (about
1,024). Two cases reach it: paging up through many kept rows, and a
flood arriving in the open conversation while its app window has no
focus, so that nothing is read (§1).

**Required.**

- **At the ceiling, held stored rows are released**: the app stops
  holding them and they stay in the store unchanged. An unread row
  stays unread (a row not held is not read, §1) and keeps its badge
  count; a kept row stays kept. Nothing is announced and nothing asks.
- **Rejected: the limit row here** (U1's option (b)). The limit row
  exists because what the session bound drops cannot come back, and
  its text says so (§7.1, `EarlierAtLimit`); a released row comes back
  when the person reaches it, so the text would be false and the choice
  would be no choice.
- **What is released first**: the rows of conversations that are not
  open (§3 lets the builder keep a closed conversation's window; the
  person sees none of it). Then the open conversation's held stored
  rows farthest from the person's place: the item that has focus, if
  the person moved focus there, else the item at the middle of the
  view; distance is counted in items along the conversation.
- **Never released**: a pending row (§3), the conversation's newest row,
  the focused item, a row any part of which is in view, and a row an
  open snackbar's Undo needs. A session-only row has no stored copy, so
  this ceiling never touches it; the session bound (§7.1) is the only
  rule that drops one. The figure is the builder's on one condition: it
  is larger than the rows one view shows at the smallest text size on
  the largest window, so that what is never released always fits.

**Where released rows go.** Released rows at the top end move the top
edge down: §5 loads them again as older rows, and an unread one among
them is counted by `EarlierUnread` (§6). Released rows anywhere else
leave a **gap**: rows still held on both sides, with stored rows
between them that the app does not hold. Since the newest row is never
released, a gap always has a held row below it, and the conversation
has no bottom edge.

A card proposal the app holds is a held row like any other (§11),
wherever it stands in time: a gap is bounded by the nearest held rows
of either kind, so a held proposal inside a run of released rows
splits it into two gaps, each with its own sides and its own
`EarlierUnread` line, and nothing is shown out of its order. A
proposal has no focus stop of its own (contact-cards §5.4.4), so where
it bounds a gap, "the row" in this section and in §7's targets means
its first control in reading order, and focus moved by the person onto
any of its controls counts as reaching that side. At the
top edge, the marks of §5 and §6 (failure, limit, progress,
`EarlierUnread`) stand above everything held, proposals included
(QUESTION 01a18b25, U2).

A gap behaves as the top edge does (§5), from either side, with
nothing new for the person to learn:

- **Trigger**: the row just above the gap or the row just below it
  scrolls into view, or receives focus moved by the person. The gap's
  rows nearest that side are fetched, a page at a time, once each time
  that side is reached; no page of a gap is requested while another
  page of the same gap is in flight. When both sides are in view or
  reached together, the side the person reached first fetches first:
  moving up (scrolling up, or focus moving up), the lower side; moving
  down, the upper side; with no movement (opening, or a page's arrival
  bringing a gap into view), the lower side. This agrees with focus
  moved onto a gap's line fetching from below (below).
  A side still in view when the other side's page arrives counts as
  reached again, so a gap wholly in view fills page by page without
  the person moving (QUESTION 01a18b25, U3).
- **While it loads, when it fails, at the session bound**: §5's
  progress row, §5's failure row with Try again, and §7.1's limit row
  (where §7.1 asks) show in the gap, on the side that fetched. The upper side's row
  shows directly below the row above the gap. The lower side's row
  shows directly above the gap's `EarlierUnread` line (below), or, when
  there is no line, directly above the row below the gap, as at the top
  edge.
- **When rows arrive**: they are added at their place in the gap.
  Neither the scroll position nor focus moves, as in §5. A gap whose
  rows have all loaded is gone; nothing marks where it was.
- **Unread rows in a gap**: an `EarlierUnread` line stands in the gap,
  directly above the held row below it, with the count of the gap's
  unread rows. It is §6's line in every respect, and the rows it counts
  are always above it: rows fetched from below the gap are added
  between the line and the row below it, rows fetched from above are
  added above the line. Focus moved onto the line by the person
  fetches from below, as at the top edge.

A gap is not a `ReadNotShown` run: a run's rows cannot come back
(§7.1); a gap's rows are stored and come back when it is reached.

## 6. Unread rows beyond the window

While the conversation is open and the store holds unread rows of it
older than the oldest loaded row, a line shows at the top of the loaded
rows, above the oldest one and below the progress or failure row:
`EarlierUnread`, with the count of those rows (messages and card
proposals together, as the badge counts both, §3). Unread rows in a
gap are not among them: the gap's own line counts those (§5.1).

- It is text, not a control. It is focusable so that TalkBack and the
  keyboard can reach it; as the top edge, focus moved onto it by the
  person triggers the load of §5.
- Its count follows the store: as pages load and their unread rows
  are read, the count goes down. At zero the line goes.
- **It agrees with the conversation's badge on Chats while the app's
  window has focus**, when every loaded unread row has been read; where
  gaps hold unread rows (§5.1), it is this line and each gap's line
  together that agree with the badge. While
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
| the `EarlierUnread` line or the oldest row, at the view's very top | a row of the top edge appears above it, is replaced, or goes (§5, "At the very top") | stays on it; it moves with each change in height, except that with no line the oldest row stays put when the page arrives | §10 items 3b and 7 |
| the row just above a gap, or any row above it | a page of the gap is added below it, from the gap's upper side (§5.1) | stays where it was; nothing moves | §10 item 14 |
| the `EarlierUnread` line, at the top or in a gap (§5.1) | its count reaches zero | the row that was just below the line, as app-placed focus (no fetch, §5) | §10 items 6 and 14 |
| the failure row's Try again | it is pressed and the row becomes the progress row, or the page arrives | the item that was just below the failure row, as app-placed focus: at the top edge the line, else the oldest loaded row; in a gap, for the upper side's row, the lower side's row if it shows, else the gap's line, else the row below the gap, and for the lower side's row, the gap's line, else the row below the gap (§5.1) | §10 items 7 and 14 |
| the limit row's `EarlierShowAnyway` | it is pressed and the row becomes the progress row, or the page arrives | the item that was just below the limit row, as app-placed focus, at the top edge or in a gap as for Try again | §10 items 12 and 14 |
| a row of the open conversation | the session bound drops it | cannot happen: the focused row is never dropped (§7.1) | §10 item 12 |
| a row of the open conversation | the held-row ceiling releases it | cannot happen: the focused item is never released (§5.1) | §10 item 14 |
| a `ReadNotShown` line | a new drop adds to its count | stays on the line, whose count changes | §10 item 12 |
| the Contacts failure row's Try again (§11) | it is pressed and the row goes | the Contacts page title, as app-placed focus | §10 item 11 |
| the Contacts page title, placed there by Try again | the read answers | success: the first contact row, else `ContactsEmpty`; failure: the new failure row's Try again | §10 item 11 |

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

- When the session already holds as many session-only rows as the
  bound allows, so that the next row read would drop one, no page is
  fetched by scrolling or focus, unless the rows still to load on that
  side hold no unread row: older than the top edge with no
  `EarlierUnread` line at the top, or a gap with no line of its own
  (§5.1, §6). Such a page makes no row session-only and drops
  nothing, so it is fetched as below the bound. Below that, a page brings at most as
  many rows as can become session-only without dropping one, so near
  the bound a page may be shorter than the builder's page size; a kept
  row in it does not count, since it keeps its stored copy. (Asked by
  rust-ui-dev-02, QUESTION 01a17dc9, U4: judging the bound by a full
  page, which shows the limit row up to a page early, is rejected: it
  would ask the person to accept a loss that would not happen, and
  hold back rows they could see without one.) The top edge, or the side
  of a gap that fetched (§5.1), shows instead a **limit row**:
  `EarlierAtLimit`, then a button,
  `EarlierShowAnyway`. The text says, before anything is committed,
  that showing more messages removes from the screen the messages
  read or sent first in this session, here and in other conversations, that
  they cannot come back, and that a message the person keeps stays.
  Nothing is dropped while the row only shows.
- Pressing `EarlierShowAnyway` fetches the next page; the bound drops
  rows in today's order. From then until the conversation is closed,
  paging is automatic again (§5) and drops as it needs: the person
  chose that once for this opening. Closing and reopening the
  conversation asks again. The conversation is closed when the person
  leaves it for Chats (narrow layout), for another conversation, or
  for one of these pages shown in its place, by whatever route (the
  Chats screen's menu, the identity switch, Share): Trust, Peers,
  Contacts, Your identities, Your card; coming back asks
  again, since a choice about losing rows is asked again rather than
  assumed (QUESTION 01a18b25, U5). It is not closed by a page opened
  from inside it and left with Back (its Details, a contact page
  reached from one of its proposals), nor by its app window losing
  focus or the app going to the background: it is still the open
  conversation, only not read meanwhile (§1). Whether its held rows are kept meanwhile
  is the builder's (§3); the person sees no difference.
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
  the loaded rows. A `ReadNotShown` line is read where it stands. In a
  gap (§5.1), top to bottom: the row above the gap, the upper side's
  failure or limit row, the lower side's failure or limit row, the
  gap's `EarlierUnread` line when shown, the row below the gap. On
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
| `EarlierUnread` | Unread messages and contact cards not shown yet: {count} | the line of §6, and in a gap (§5.1) | how many unread items are above, not yet shown; the count after a colon, one text for every count (below); must not say "messages" if it counts cards too, or must cover both |
| `ReadNotShown` | one: 1 message that you read or sent is no longer shown. It cannot be shown again. other: {count} messages that you read or sent are no longer shown. They cannot be shown again. | the line of §7.1 | that messages the person already read or sent were removed from this screen and cannot come back; a plural by count; must not suggest they were deleted by anyone else |
| `RowsNotShown` | one: 1 message is saved on this device, but this version of InterWeave cannot show it. other: {count} messages are saved on this device, but this version of InterWeave cannot show them. | the line at the end of Chats (§6.1) | that these messages (received unread, kept, or the person's own still waiting to send) are still stored and this app version cannot show them; must not use "kept" in the sense of Keep; a plural by count; must not blame a sender or suggest tampering |
| `EarlierAtLimit` | If you choose Show more messages, InterWeave makes room for more messages by no longer showing some messages that you read or sent in this session. It begins with the ones you read or sent first, in any conversation, including this one. It never stops showing kept messages. The messages that it no longer shows cannot be shown again. | the limit row (§7.1), at the top edge or in a gap (§5.1) | before anything happens: what showing more messages removes (messages read, and the person's own once accepted, first in this session, in every conversation), that it cannot be undone, and that kept messages stay |
| `EarlierShowAnyway` | Show more messages | the limit row's button (§7.1) | the action, which the text above it qualifies |
| `EarlierLoading` | Loading more messages | the progress row's accessible name (§5), at the top edge or in a gap (§5.1); never visible text; not on Contacts, whose progress row is named `ContactsLoading` | that more rows are on their way |
| `EarlierLoadFailed` | Could not load more messages. | the failure row (§5), at the top edge or in a gap (§5.1), and its announcement | that the rows could not be read; must not suggest they are lost |
| `CardContactsUnread` | InterWeave could not read your contacts to compare this card with them, so you cannot accept it yet. To read your contacts again, choose App options on the Conversations screen, then Contacts, then Try again. | a proposal's case line while the contacts are unread (§11) | that the comparison with the person's contacts could not be made and why Accept is missing, and where to try again, by the controls' own names: the Chats screen's menu (`AppOptions`), then Contacts (`ContactsTitle`), then `TryAgain` (contact-cards §4, §11), never the conversation's own menu; must not suggest the card is unsafe or the contacts lost |
| `NamingContactsUnread` | InterWeave could not read your contacts, so you cannot add this person as a contact yet. | Details, in place of naming by hand, while the contacts are unread (§11) | that the contacts could not be read and that adding a contact waits for that; names no card; must not suggest the contacts are lost |
| `NotSentEndpointLimit` | This device already keeps 64 routes of this peer, the limit for all its identities and contacts together, so it cannot open a new route; check that this is the route you want, or send in a conversation with this peer if you have one. | the reason of a refused send that would open a peer's 65th endpoint (§3), after the "Not sent" that every refused send shows, which it does not repeat | that this device keeps at most 64 routes of one peer, over all its identities and contacts, and already has them, so it cannot open another; that an existing conversation with that peer still works, without claiming one exists for the identity in front; may suggest checking the route; must not blame the person, suggest that a setting can change it, or say anything about the peer's intent |
| `ContactsLoading` | Loading your contacts | the accessible name of the Contacts list's progress row during Try again (§11); never visible text | that the contacts are being read |
| `ContactsLoadFailed` | Could not load your contacts. | the Contacts list's failure row, when the read of every contact fails (§11), and its announcement | that the contacts could not be read just now; must not suggest they are lost or that there are none |

`TryAgain` exists and is reused.

The `Earlier…` strings show in a gap too (§5.1), which the person can
reach scrolling down as well as up. A gap reached from above holds
rows later than the row the person has just read, so the texts name no
direction: "more" and "not shown yet" hold on either side of a gap and
at the top edge. The keys keep their names.

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
3b. **Placement** (§5, §5.1, §7.1, §11; QUESTION 01a18b25): with the
   view at A's very top and the next page held (a test hook), the
   progress row shows in view after 250 ms and the line and the rows
   below move down by its height; release the page: the progress row
   goes, the line moves back to the top once, and focus, if on the
   line, stays on it through both. Again with no line and focus on the
   oldest row: it moves down when the progress row shows and stays
   there when the page arrives, the page's rows above it. Again with
   the page's read failing: the failure row replaces the progress row
   in view, what is below moves by the difference in height once, and
   Try again is in view; pressing it leaves the failure row in place,
   pressing again doing nothing, until the progress row replaces it
   after 250 ms, what is below moving by the difference once; with the
   page let through within 250 ms and no line, the page's rows take the
   failure row's place and the oldest row does not move. With a held proposal standing among
   released rows, two gaps show around it, each loading on its own,
   and at the top edge every mark stands above a leading proposal.
   With a gap wholly in view and no movement, its lower side fetches
   first, then the upper, until it is full. After `EarlierShowAnyway`,
   open Contacts in the conversation's place on a wide layout and come
   back: reaching the bound asks again. While contacts are unread, a
   proposal shows one button, Decline, with nothing below it where
   Accept would stand, and a left swipe declines it; Decline, then
   Undo on its snackbar: focus is on the row's Decline.
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
    conversation title on Chats uses its contact's name. Make the
    contacts' read fail at start (a test hook): Chats titles read as
    for a conversation under no contact; Contacts shows the failure row
    alone at the top, with Try again and no `ContactsEmpty`, and
    announces nothing on opening. Press Try again with the read held
    (a test hook): the row goes at once, focus is on the page title,
    the progress row shows after 250 ms, its accessible name
    `ContactsLoading` (the desktop's accessibility tree now; TalkBack
    blocked), and nothing else can be pressed there. Let it fail again: focus on Try again,
    `ContactsLoadFailed` announced once. Remove the failure and press
    Try again: the list shows, focus on its first row, and the titles
    on Chats take the contacts' names. With a store holding no contact,
    the same success shows `ContactsEmpty`, with focus on it. With the
    read failed, a card "Anna Rossi" arriving while a contact "Anna
    Rossi" exists shows `CardContactsUnread`, no Accept and no
    `CardSameName`, and a right swipe on it does nothing, while a left
    swipe declines it as on any proposal, and its menu and its screen
    reader actions offer Decline and no Accept; Details shows
    `NamingContactsUnread` and no naming. After Try again succeeds, the row shows
    `CardSameName` and Accept as a new contact, with nothing announced.
11a. **Preview bound** (§3): a conversation whose newest row's first
    line is 300 characters long previews its first 160 characters and
    an ellipsis, on a window wide enough to show more, and identically
    before and after the row loads. A first line that holds a link
    across the 160th character shows the link's text, never its
    syntax. A first line of combining marks or emoji sequences is cut
    between whole characters, within 1 KiB; one whose first character
    alone exceeds 1 KiB previews as the ellipsis alone. A first line
    under 160 characters and under 1 KiB shows whole, with no ellipsis;
    100 emoji sequences of about 25 bytes each are cut at 1 KiB, with
    the ellipsis. A 300-character
    Hebrew first line previews its first 160 characters with the
    ellipsis on the left, and one whose cut falls in a Latin word at
    the end of a Hebrew line keeps the ellipsis on the left too.
11b. **A send at the endpoint bound** (§3): with a test store
    holding 64 endpoints of one peer, a send to a 65th endpoint of that
    peer is refused with `NotSentEndpointLimit`, nothing is pending and
    nothing is sent; a send in one of the 64 conversations goes as
    usual. Again with the 64 made of 40 conversations of the identity
    in front, 20 of another identity and 4 routes kept only under
    contacts: the same refusal and the same text, which names no
    conversation of the identity in front that is not there. Again with
    the identity in front holding no conversation with that peer, the
    64 being 60 conversations of another identity and 4 routes kept
    only under contacts: the same refusal, and its text directs the
    person to no conversation as one they have; it offers the
    conversation only as a condition ("if you have one"). (Where no
    surface in the apps addresses a new endpoint yet, a test hook makes
    the send.)
12. **The bound while open**: with a test bound small enough to reach
    (the builder's hook), read B's unread row, then open A and page up.
    With room left for fewer session-only rows than a page holds, the
    next page loads without the limit row and brings no more rows that
    become session-only than that room (kept rows in it may be more);
    no read row is dropped. Page up again, with the session
    holding as many session-only rows as the bound allows: no page is
    fetched; the
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
14. **The held-row ceiling** (§5.1), with a test ceiling small enough
    to reach (the builder's hook) and the session bound out of reach.
    (a) Open A in a focused app window, at its newest row; take focus
    away from the app window and deliver more unread rows to A than
    the ceiling. Nothing is announced and no limit row shows. Give
    focus back: the rows held are read; A's badge counts the unread
    rows released; `EarlierUnread` lines, at the top and in each gap,
    show counts that together equal A's badge. The newest row, the
    rows in view and the focused item were never released. Scroll up
    to a gap: its rows load where they belong, the view does not move,
    and its line's count drops; when the count reaches zero with focus
    on the line, focus is on the row that was just below it. (b) Fill
    A with kept rows between read ones, note the order of every row,
    then page up past the ceiling and come back down, once by
    scrolling and once by moving focus only (Tab on the desktop,
    TalkBack's swipe on the phone): every gap loads as it is reached,
    and the rows are in the noted order, none missing and none twice.
    Restart the app: the store is as it was before (a) and (b), less
    the rows read. (c) In a gap, coming from above: make the gap's
    next read fail (a test hook). The failure row shows directly below
    the row above the gap and its text is announced once; focus on
    that row stays where it was. Remove the failure, move focus to Try
    again and press it: the page loads below the row above the gap,
    and focus is on the item that was just below the failure row. With
    the session bound brought within reach (the hook of item 12), the
    gap's next page shows the limit row on the side that fetched,
    with nothing dropped. Press `EarlierShowAnyway` with focus on it:
    the page loads below the row above the gap, focus is on the item
    that was just below the limit row, and the bound drops rows as in
    item 12. With focus on the row above the gap, let a further page of
    the gap arrive from the upper side: focus stays on that row and the
    view does not move. (d) Set the session bound out of reach again (the hook of
    item 12). Close A and reopen it, so that paging asks again (§7.1),
    and make a gap holding more unread rows than two pages bring, as in
    (a). Make the gap's next read fail, and scroll up until the row
    below the gap comes into view. The failure row shows directly above
    the gap's line. Remove the failure, move focus to Try again and
    press it: focus is on the gap's line, and stays there once the page
    arrives, since the gap still holds unread rows. Now set the bound
    so that the session holds as many session-only rows as it allows,
    as in item 12, and move focus down from the line to the row below
    the gap and back up to the line. The limit row shows directly above
    the line, with nothing dropped. Press `EarlierShowAnyway` with
    focus on it: focus is on the line. (e) With the session bound out of reach, close A and reopen it, so
    that paging asks again (§7.1), and make a gap holding more kept
    rows than two pages bring and no unread row, as in (b), so that
    the person is above it. Make the gap's next read fail and
    scroll down until the row above the gap comes into view: the
    failure row shows directly below that row, with no line in the
    gap. Remove the failure, move focus to Try again and press it:
    focus is on the row below the gap, and stays there once the page
    arrives. Now set the bound so that the session holds as many
    session-only rows as it allows, as in item 12, and scroll down
    until the gap's upper side is reached again: the next page loads,
    with no limit row and nothing dropped. Then the top edge: with the
    bound still full and the store holding only kept rows older than
    A's top edge (a test hook that makes them so), so that no
    `EarlierUnread` line shows at the top, scroll to the top edge: the
    next page loads,
    with no limit row and nothing dropped. Control: give A one unread
    row older than the top edge (a test hook), so that the line shows,
    and reach the top edge again: the limit row shows.
15. **Photos on demand** (§11), with photo loads slowed by a test hook.
    The Contacts list, and Chats and the app bar once they draw contact
    avatars, show a contact's initials
    on its hue, at the photo's size, then the photo in its place:
    nothing else in the row moves, nothing animates and nothing is
    announced. A photo that cannot be read leaves the initials. A
    contact page opens with its photo, never the initials first; with
    the photo made unreadable (a test hook), it opens with the initials
    and Remove photo, and Remove photo removes the stored photo. A
    proposal shows its neutral avatar, as contact-cards §5.4 says. With
    a contact's avatar in view, Remove photo on its page (wide layout)
    replaces the photo with the initials at once.
16. **One contact named** (§11): with two contacts "Anna Rossi" on
    other PeerIds, a card "Anna Rossi" from a third PeerId shows
    `CardSameName` naming the one whose PeerId sorts first by code
    point (contact-cards §6.2's tie-break; the fixture's two PeerIds
    first differ at an uppercase and a lowercase letter, such as
    `…KooWB…` and `…KooWa…`, and the one with the uppercase letter is
    expected), and Open your contact opens that
    one, not the other; with three contacts
    on one PeerId, a card from a fourth route of that PeerId shows
    `CardSharesDeviceMany` naming the first of them in that order and
    2 others. Both hold with the Contacts list scrolled to its end and
    with it never opened.

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
- **No list on a proposal to cap** (U3 of QUESTION 01a17dc9). A
  proposal names one contact: `CardSameName` and its Open your contact
  link name one, and case P's `CardSharesDeviceMany` names one and the
  number of the others (contact-cards §5.4.1). Which one, when several
  match, is the first in the Contacts list's order, read from the
  store; contact-cards §5.4.1 says so once. On a contact page,
  `ContactSharesDevice` has one line for each other contact on that
  PeerId (contact-cards §6.3): all are shown, and they are at most 63,
  since a 65th distinct source endpoint of a peer is malformed and
  stores nothing (§3, architect-cto's REPLY 01a17be2).
- **Photos may load on demand** (U2 of QUESTION 01a17dc9). Wherever
  a contact's avatar is drawn at contact-cards §6.1's sizes (the
  Contacts list, Chats, the app bar, the contact page), a contact whose
  photo has not arrived shows its initials avatar (contact-cards §6.1), at
  the photo's size, and the photo replaces it in place when it
  arrives: nothing else moves, nothing animates, nothing is announced
  (the avatar is not read, contact-cards §6.1), and the accessible
  label does not change. A photo that cannot be read leaves the
  initials. A photo shown in a row that stays in view is not released
  back to the initials; a photo the contact no longer has (Remove
  photo, an accepted card that removes it) goes at once, as today.
  Two places are not covered: a
  proposal shows no contact's photo at all (its neutral avatar,
  contact-cards §5.4), so nothing loads there; and a contact page, where the
  person acts on the photo (Remove photo, contact-cards §6.3), opens
  with its photo: opening the page waits for its one photo, with no
  loading state, so that it never shows initials for a contact whose
  photo can be read. If that wait can be long enough for a person to
  notice (more than about 100 ms on the Pixel 9a), the builder says so
  and a loading state is added here, as §3 says of opening a
  conversation. A photo that cannot be read shows the initials there
  too, and Remove photo stays, since the contact still has a stored
  photo the person can remove. A third place needs nothing: the
  confirmation row that replaces an accepted proposal (contact-cards
  §5.4.2) shows the photo decoded by that Accept; reloaded later, it
  follows the lists' rule above. Evidence: the avatar is decoration, never
  a trust signal (contact-cards §3), so a moment of initials misleads
  no one about who a contact is; on the contact page the photo is what
  the person acts on. Chats and the app bar draw the PeerId's
  letters in today's build, not yet §6.1's contact avatars (QUESTION
  01a18b25, U4); this rule acts there from when they do, and item 15's
  checks there wait for it. When that is built is outside this note.
- **The Contacts list has no visible edge.** It grows only by the
  person's own Accepts. If the builder pages it, the paging is not
  seen: no progress row, no failure row unless a read fails. Every contact's name must be known for
  conversation titles and for proposals' notes whatever is loaded.
- **When the contacts cannot be read** (U-b of QUESTION 01a17e5d).
  All contacts are read at start, page by page, with no paging the
  person sees, so a failure is the whole read failing. The Contacts
  list then shows no partial list: one failure row stands alone at
  the top, under the app bar, in place of the rows and of
  `ContactsEmpty`, which would claim there are no contacts. The row
  is `EarlierLoadFailed`'s pattern (§8): `ContactsLoadFailed`, then a
  Try again button (`TryAgain`). Pressing Try again reads every contact
  again. The failure row goes at once, and focus goes to the page's
  title as app-placed focus (the title is where Contacts puts focus on
  opening, contact-cards §9 item 1). Its place stays empty until §5's
  progress row, named `ContactsLoading` here, shows there after 250 ms, if the read has not answered
  by then. With the button gone, no second read can start while one
  is in flight. Success shows the list, with focus on its first row,
  else on `ContactsEmpty` when the store holds no contact (contact-cards
  §9 item 1's rule), if focus was still on the title. A failure shows
  the failure row again, with focus on Try again if it was still on
  the title. `ContactsLoadFailed` is announced
  once, politely, when the failure arises while Contacts is open, a
  failed Try again included. A Contacts list that opens already
  showing it announces nothing, because the row is the first item read
  there. While the contacts are unread, a conversation title on Chats
  and in the app bar has no contact name to use, so it is the title of
  a conversation under no contact: the short PeerId, with the route
  label only where `human-client-ui.md` §4 asks for it (`DirectTitle`). It takes the contact's
  name once a read succeeds, from Try again or the next start.
- **Nothing writes a contact while the contacts are unread.** A card
  proposal's case, its same-name warning (`CardSameName`) and its
  Accept label are decided by comparing the card with the contacts
  (contact-cards §5.4.1). That comparison is a security surface
  (ADR-0059: a second contact with an existing name is shown as such
  before acceptance), so it is never made against contacts that could
  not be read. While they are unread, a proposal shows its heading,
  PeerId and Decline as usual, and in place of its case line
  `CardContactsUnread`: "InterWeave could not read your contacts to
  compare this card with them, so you cannot accept it yet. To read
  your contacts again, choose App options on the Conversations screen,
  then Contacts, then Try again." It offers no Accept,
  so its right swipe does nothing (contact-cards §5.4.4, a row with no
  Accept); a left swipe still declines it, and its menu and its screen
  reader actions (contact-cards §8) offer no Accept. Its one button is
  Decline, first in contact-cards §5.4's stacked layout as always
  (Decline above Accept), with no space kept below it for Accept; it is
  never the one-button Close row of case S, which declines nothing
  (QUESTION 01a18b25, U6). When the contacts are read and Accept comes
  back, the lines above the buttons change too (the case line, and
  `CardSameName` with its link where it applies), so the row may move;
  nothing is announced (above). After an Undo of its Decline, focus
  goes to its Decline, since the row has no Accept (contact-cards §9's
  row for that Undo names Accept). Details' naming by hand (contact-cards §5.6) is withheld in
  the same way, with `NamingContactsUnread` in its place, because it writes a
  contact without knowing whether the route is already under one.
  When a read succeeds, from Contacts' Try again or the next start,
  every such proposal shows its case and its Accept. That is a change
  the person did not ask for: nothing is announced and focus does
  not move, as for a proposal's other lines. A notification's title
  falls back as the conversation's title does (contact-cards §5.7.5
  uses it). Since nothing is written meanwhile, the Contacts list
  never has to show a contact while its read has failed.

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
- **An exact limit row at the bound** (§7.1). With the session at the
  bound and unread rows still to load on that side, the limit row
  shows before the next page is read, so it shows even when every row
  of that page is kept and showing it would drop nothing; pressing `EarlierShowAnyway` then drops nothing. Accepted:
  knowing it beforehand needs the page's rows read first, and the cost
  is one question the person did not need.

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
- **U1 to U4, rust-ui-dev-02** (QUESTION 01a17dc9): answered here. U1,
  the held-row ceiling, releases rows silently and never shows the
  limit row (§5.1). U2, initials until the photo arrives, holds
  wherever contact avatars are drawn, not on the contact page (§11). U3, no list on
  a proposal to cap; which contact is named is contact-cards §5.4.1's
  rule (§11). U4, the limit row a page early, is rejected: it shows at
  the bound, and a page near it is shortened (§7.1).
- **U-a and U-b, rust-ui-dev-02** (QUESTION 01a17e5d): answered here.
  U-a, the preview holds at most 160 characters and 1 KiB, cut after
  it is computed (§3). U-b, a failed contacts read shows its row alone
  with Try again, and nothing writes a contact until a read succeeds
  (§11).
