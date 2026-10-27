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
  conversation that is open in a focused window
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
  (`human-client-ui.md` §6). A row loaded by the window and then read
  becomes such a row. The session holds at most `SESSION_ITEM_CAP`
  (1,024) of them; past it, `UiModel::evict` drops the one that became
  session-only first (`crates/human/ui-model/src/model.rs`). §7.1 changes
  which one is dropped while a conversation is open. (`copy_gone` is a
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

- **The Chats list is complete.** Every conversation the store holds
  a row of — pending, unread or kept — is listed, as are the
  conversations the session holds read rows of (look §5.2: listed
  while the app holds one of its messages). No budget leaves a
  conversation out.
- **Each Chats row's last message and time are those of the newest
  message the conversation has, held in the session or stored**:
  whichever is newer of the newest row the session holds (a message
  read or sent and accepted this session, which has no stored copy)
  and the newest stored row. That is look §5.2's rule, "the last
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
  another conversation's rows out.
- **The number of conversations is not bounded by this note.** A
  direct conversation is keyed by the remote EndpointId
  (`human-client-ui.md` §4), which the sending peer chooses
  (`architecture/contracts/ENDPOINTS.md`: peer-asserted routing
  metadata), so one trusted peer can open many conversations, limited
  only by the ingress rate. Holding one summary each keeps that cost
  small, but not bounded; whether the count needs a rule is §13 Q3.
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
sees no difference). One condition: if opening a conversation can take a time a
person sees (more than about 100 ms on the Pixel 9a with a full
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

**Trigger.** While a conversation has rows in the store older than the
oldest one it holds, the next page of older rows is fetched when the
top edge:

- scrolls into view (a scroll, a fling, a drag of the scroll bar on
  the desktop), or
- receives focus moved by the person (Tab, Shift+Tab or the arrow keys
  on the desktop, TalkBack's swipe, switch access).

Focus the app places (§7) triggers nothing, and no page is requested
while one is in flight. A page is requested once per arrival: focus or
the view must leave the top edge and come back before the next page is
fetched, except that a page arriving below a focused line (below) makes
its rows reachable by moving focus down, with no new arrival needed.

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

**When the store holds nothing older**, nothing is shown at the top:
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
- **It agrees with the conversation's badge on Chats while the window
  has focus**, when every loaded unread row has been read. While the
  window does not have focus, loaded rows stay unread (§1), so the
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
`model_side.rs`, `hidden_rows`). The line does not count it, and the
conversation's badge does not either, as today: such rows are said
by the app's existing hidden-rows report, not by a count the person
could never bring to zero. So the line reaches zero, and goes, once
every unread row the app can show is loaded. Whether (a)'s "every
unread row" means to include them is §13 Q4.

**Rejected: jumping to the oldest unread**, §2's hypothesis.

## 7. Focus

Every case where focus can be on something the window removes or
replaces, with its target and check. Look's rules for the conversation
screen hold for every other case.

| Focus was on | It disappears when | Focus goes to | Check |
|---|---|---|---|
| any row, or the `EarlierUnread` line | a page is added above it or below the line | stays where it was; nothing moves | §10 items 4 and 5 |
| the `EarlierUnread` line | its count reaches zero | the row that was just below the line, as app-placed focus (no fetch, §5) | §10 item 6 |
| the failure row's Try again | it is pressed and the row becomes the progress row, or the page arrives | the row that was just below the failure row (the line, else the oldest loaded row), as app-placed focus | §10 item 7 |
| a row of the open conversation | the session bound drops it | cannot happen: §7.1 drops the rows furthest from the person's position, never the focused one | §10 item 12 |

### 7.1 The session bound while a conversation is open

Today the bound drops the row that became session-only first (§1).
Paging up through a flood reads the newest rows first, so past the
bound it would drop the newest messages the person just read, for
good, while they are looking at older ones. Required instead, when
the bound must drop a read, unkept row while a conversation is open:

1. first, read rows of conversations that are not open, in today's
   order;
2. then, rows of the open conversation furthest from the person's
   position (the focused item, else the middle of the visible rows):
   whichever end of the loaded rows is further away.

Where rows of the open conversation were dropped, a line stands in
their place: `ReadNotShown`, with the count dropped there. It looks and
behaves as the `EarlierUnread` line (§8): text, focusable, not
announced. The rows it stands for have no stored copy and cannot be
loaded again; a page fetched past it brings rows older than them. When
the conversation closes, its read rows fall back into today's order.

On Android every TalkBack check above is a **blocked precondition**,
not a pass, until TalkBack reaches the app (Slint 1.18.1's Android
backend has no accessibility bridge; look §7). The desktop's checks run
now, through Slint's accessibility tree and the keyboard.

## 8. Accessibility

- **Reading order** at the top of a conversation: the failure row
  (`EarlierLoadFailed`, then Try again) or nothing (the progress row is
  not focusable), then `EarlierUnread` when shown, then the loaded
  rows; a `ReadNotShown` line is read where it stands.
- **Look**: the `EarlierUnread` and `ReadNotShown` lines are styled
  as the day separator (look §6): centred, body small,
  `on-surface-variant`, one item each for a screen reader. The
  failure row is centred: `EarlierLoadFailed` in body medium in
  `error`, and under it a Material text button, Try again. The
  progress row is the indicator alone, 24 dp, centred, with 8 dp
  above and below.
- **Every gesture has a non-gesture equivalent**: §5's focus trigger.
- **Reduced motion**: the progress indicator follows the platform's
  setting as Material's does; no row animates in.
- **Text size**: at the largest font and display size, the
  `EarlierUnread` line and the failure row wrap and are never cut;
  the Try again target is at least 48 by 48 dp.
- **Contrast**: everything here sits on the conversation's background,
  `surface`, and on no other surface; each pair is one look §3 already
  holds:

  | Mark | Pair | Needs | Dark | Light |
  |---|---|---|---|---|
  | `EarlierUnread`, `ReadNotShown` | `on-surface-variant` on `surface` | 4.5 | 9.9 | 7.6 |
  | `EarlierLoadFailed` | `error` on `surface` | 4.5 | 10.6 | 6.2 |
  | Try again, its focus ring, the progress indicator | `primary` on `surface` | 4.5 (3 for the ring and indicator) | 10.8 | 6.2 |

## 9. Strings

The words are language-culture's; these say what each must do.

| Key | Proposed English | Shows | Must say |
|---|---|---|---|
| `EarlierUnread` | {count} earlier unread | the line of §6 | how many unread items are above, not yet shown; a plural by count (language-culture's forms); must not say "messages" if it counts cards too, or must cover both |
| `ReadNotShown` | {count} read messages no longer shown | the line of §7.1 | that messages the person already read were removed from this screen and cannot come back; a plural by count; must not suggest they were deleted by anyone else |
| `EarlierLoading` | Loading earlier messages | the progress row's accessible name (§5); never visible text | that older rows are on their way |
| `EarlierLoadFailed` | Couldn't load earlier messages. | the failure row (§5), and its announcement | that the older rows could not be read; must not suggest they are lost |

`TryAgain` exists and is reused.

## 10. Acceptance

Run on the desktop and on the Pixel 9a, with a store a test fills: one
conversation "A" with more rows than the window's row budget, 10 of
them unread, followed by more unread rows than the window holds; a
second conversation "B" with two read-and-kept rows and one unread row.
Every TalkBack check below is a blocked precondition on Android (§7);
its desktop counterpart, through Slint's accessibility tree, runs now.

1. **List complete under flood**: start the app. Chats lists A and B.
   A's badge shows A's unread count, card proposals included; B's
   shows 1; B's preview is its newest row.
2. **Preview reads nothing**: with Chats shown, restart the app. Both
   badges are unchanged (look §5.2).
3. **Opening**: open A in a focused window. It opens at its newest
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
    (the builder's hook), open A and page up past it. The newest rows,
    read first, are still there when the person comes back down; the
    rows dropped are those furthest from the focused row, and a
    `ReadNotShown` line with their count stands where they were. Focus
    is never on a dropped row.
13. **An undecodable unread row**: add one to A's store. It is not
    counted on the line or on A's badge; the line goes once every
    other unread row is loaded; the app's hidden-rows report counts it.

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
  `EarlierLoadFailed`'s pattern, with the words language-culture
  chooses for contacts). Every contact's name must be known for
  conversation titles and for proposals' notes whatever is loaded.

## 12. Deliberately left out

- **Read by what is on screen** instead of by what is held. Today a
  conversation reads every row it holds, on screen or not; this note
  keeps that, and a page loaded above the person's position is read
  before they scroll to it.
- **A jump to the first unread, or to the newest from far up.** Not
  designed; §10's test decides whether the first is needed.
- **A loading state for opening a conversation** (§3), until a
  measurement says it is visible.
- **How the window is held and its budget figures**: the builder's.
  Which read row the session bound drops is §7.1's.

## 13. Questions

- **Q1, architect-cto** (put to them by rust-ui-dev-02 with REPLY
  01a17bd8): pending rows have no count ceiling, so holding every one
  (§3) is safe from a peer but not bounded. A ceiling would be a
  product rule (Send refused past a count, with a reason); whether it
  belongs to this fix or a later one is theirs. If one comes, this
  note adds its refusal state.
- **Q2, rust-ui-dev**: answered by the code. The session bound drops
  the row that became session-only first (`UiModel::evict`), so it
  would drop the newest rows of a flood first; §7.1 is the rule that
  replaces it while a conversation is open.
- **Q3, architect-cto**: a direct conversation is keyed by a remote
  EndpointId the sending peer chooses, so one trusted peer can open
  conversations without a count bound, limited only by the ingress
  rate. This note keeps each closed conversation to a summary (§3),
  which makes the cost small but not bounded. Is a bound on
  conversations per peer, or on summaries held, a product rule this
  fix needs? If one comes, this note adds what the person sees at it.
- **Q4, architect-cto**: (a) says the badge counts every unread row.
  §6.1 leaves out a stored unread row that does not decode, which no
  load can show and no view can read, as today's badge does. Is that
  the reading of (a) you meant?
