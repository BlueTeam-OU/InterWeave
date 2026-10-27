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
- **A read row that is not kept lives in this session only**
  (`human-client-ui.md` §6), within the session bound that already
  exists (`UiModel::copy_gone`). A row loaded by the window and then
  read becomes such a row.

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
- **Each Chats row's last message, time and unread badge come from the
  whole store**, not from the window: the conversation's newest row and
  its true unread count (a). Look §5.2's rules for the preview hold
  unchanged, above all that showing a preview never reads a message.
- **Each conversation holds its own newest rows**, up to the row and
  byte budget, and always at least its newest row. One conversation's
  backlog never pushes another conversation's rows out.
- The total unread count and every notification count every unread
  row in the store (a).

**Rejected: one global budget**, newest first across all
conversations. A peer flooding one conversation would push every
other conversation's rows out of the window: those conversations would
vanish from Chats, or show a badge with nothing behind it, while the
total badge still counts them.

**The builder's**: the budget figures; whether a conversation's window
is loaded at start or when it is opened; how the window is held in
memory. One condition: if opening a conversation can take a time a
person sees (more than about 100 ms on the Pixel 9a with a full
budget), say so and I add a loading state to this note; none is
designed now.

## 4. Opening a conversation

- It opens at its newest row, as today.
- The loaded rows are read under §1's rule. Unread rows beyond the
  window stay unread.
- If unread rows remain beyond the window, the line of §6 shows.

## 5. The edge: earlier rows load at the top

**Trigger.** While a conversation has rows in the store older than the
oldest one it holds, the next page of older rows is fetched when the
oldest loaded row:

- scrolls into view (a scroll, a fling, a drag of the scroll bar on
  the desktop), or
- receives focus by any means (Tab or the arrow keys on the desktop,
  TalkBack's swipe, switch access).

The second trigger is the non-gesture equivalent of the first: a
person who never scrolls reaches the top by moving focus, and the
older rows load. No button, and so no new control and no new label for
one.

**While it loads.** A progress row shows above the oldest loaded row:
an indeterminate circular progress indicator (Material), centred, with
`EarlierLoading` as its accessible name. It is not focusable and is
not announced. It shows only if the page has not arrived within
250 ms, so a fast load does not flicker.

**When it arrives.** The page's rows are added above, in their order.
**Neither the scroll position nor focus moves**: the row the person
was looking at, or the row that has focus, stays where it was on
screen. The new rows are reached by scrolling or moving focus further
up, and the next page loads when the new oldest row is reached. The
loaded rows are read under §1's rule.

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
the row is replaced, goes to the oldest loaded row (§7).

**A new message arriving** while the person is up in the history
behaves as today; the window does not change it.

## 6. Unread rows beyond the window

While the conversation is open and the store holds unread rows of it
that the window has not loaded, a line shows at the top of the loaded
rows, above the oldest one and below the progress or failure row:
`EarlierUnread`, with the count of those rows (messages and card
proposals together, since both are counted on the badge).

- It is text, not a control. It is focusable so that TalkBack and the
  keyboard can reach it, and focus reaching it triggers the load of §5
  as the oldest row does.
- Its count follows the store: as pages load and their unread rows
  are read, the count goes down. At zero the line goes. Its count and
  the conversation's badge on Chats always agree.
- It is announced when the conversation opens, as part of the
  conversation's opening announcement, after the title (§8).
- It does not show when every unread row fits in the window, which is
  the ordinary case; nothing changes then.

**Rejected: jumping to the oldest unread**, §2's hypothesis.

## 7. Focus

Every case where focus can be on something the window removes or
replaces, with its target and check. Look's rules for the conversation
screen hold for every other case.

| Focus was on | It disappears when | Focus goes to | Check |
|---|---|---|---|
| any row | a page is added above it | stays on that row; nothing moves | §10 item 4 |
| the `EarlierUnread` line | its count reaches zero | the oldest loaded row | §10 item 6 |
| the failure row's Try again | it is pressed and the row becomes the progress row, or the page arrives | the oldest loaded row | §10 item 7 |
| a loaded row | the session bound drops it (`copy_gone`, §1) | as look's rule for a message that leaves the session | unchanged |

On Android every TalkBack check above is a **blocked precondition**,
not a pass, until TalkBack reaches the app (Slint 1.18.1's Android
backend has no accessibility bridge; look §7). The desktop's checks run
now, through Slint's accessibility tree and the keyboard.

## 8. Accessibility

- **Reading order** at the top of a conversation: the failure row
  (`EarlierLoadFailed`, then Try again) or nothing (the progress row is
  not focusable), then `EarlierUnread` when shown, then the oldest
  loaded row.
- **Opening announcement**: when `EarlierUnread` shows as the
  conversation opens, it is read after the title, once.
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
  | `EarlierUnread` | `on-surface-variant` on `surface` | 4.5 | 9.9 | 7.6 |
  | `EarlierLoadFailed` | `error` on `surface` | 4.5 | 10.6 | 6.2 |
  | Try again, its focus ring, the progress indicator | `primary` on `surface` | 4.5 (3 for the ring and indicator) | 10.8 | 6.2 |

## 9. Strings

The words are language-culture's; these say what each must do.

| Key | Proposed English | Shows | Must say |
|---|---|---|---|
| `EarlierUnread` | {count} earlier unread | the line of §6 | how many unread items are above, not yet shown; a plural by count (language-culture's forms); must not say "messages" if it counts cards too, or must cover both |
| `EarlierLoading` | Loading earlier messages | the progress row's accessible name (§5); never visible text | that older rows are on their way |
| `EarlierLoadFailed` | Couldn't load earlier messages. | the failure row (§5), and its announcement | that the older rows could not be read; must not suggest they are lost |

`TryAgain` exists and is reused.

## 10. Acceptance

Run on the desktop and on the Pixel 9a, with a store a test fills: one
conversation "A" with more rows than the window's row budget, 10 of
them unread, followed by more unread rows than the window holds; a
second conversation "B" with two read-and-kept rows and one unread row.

1. **List complete under flood**: start the app. Chats lists A and B.
   A's badge shows A's true unread count; B's shows 1; B's preview is
   its newest row. The total badge is their sum.
2. **Preview reads nothing**: with Chats shown, restart the app. Both
   badges are unchanged (look §5.2).
3. **Opening**: open A. It opens at its newest row. `EarlierUnread`
   shows at the top of the loaded rows with the count of unread rows
   not loaded; A's badge on Chats equals it.
4. **Load by scrolling, position kept**: scroll A to its top. The next
   page loads above (the progress row only if it takes longer than
   250 ms); the row that was at the top stays where it was on screen;
   `EarlierUnread`'s count drops by the unread rows in the page.
5. **Load by focus**: reopen A. Without scrolling, move focus to the
   top (Tab or Shift+Tab on the desktop; TalkBack's swipe on the
   phone — blocked precondition). The page loads when focus reaches
   the oldest row or the `EarlierUnread` line; focus stays where it
   was.
6. **Line goes at zero**: keep loading until every unread row is
   loaded. The line goes; if it had focus, focus is on the oldest
   loaded row. A's badge on Chats is 0.
7. **Failure and Try again**: make the next page's read fail (a test
   hook). The failure row shows with Try again and its text is
   announced once. Scrolling to the top again retries nothing. Remove
   the failure; Try again loads the page; focus, if it was on Try
   again, is on the oldest loaded row.
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
- **How the window is held, its budget figures and what is evicted**:
  the builder's.

## 13. Questions

- **Q1, rust-ui-dev**: are pending outbound rows bounded in count? The
  person's own rows must stay reachable when they fail (Retry and
  Cancel), so a pending row should always be held, whatever the
  window; that is safe only with a ceiling. Asked in REPLY 01a17bd0.
- **Q2, rust-ui-dev**: rows loaded far up, once read, are session-only
  (§1). Does the session bound then drop the newest read rows first, or
  the oldest? A person who paged up and comes back down must not find
  the newest messages gone; if the bound can do that, §7's last row
  needs a rule of its own.
