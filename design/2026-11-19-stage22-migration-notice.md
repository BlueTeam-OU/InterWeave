# What the Stage 22 update removed: the notice at the first open — desktop and Android

Status: proposal, 2026-11-19, my job j148. Read against `origin/main`
94ecad7f2 (#730 merged). architect-cto's OBSERVATION 01a1f387 handed
this design to ux-designer: the store migration of the Stage 22 release
counts what it discarded into one content-free `settings` row, "which
the client shows once at the first open and then deletes; the words are
ux-designer's" (STATE.md §Migrations, schema v14). This note's first two
product questions (§11) were ruled by architect-cto as proposed (REPLY
01a1f3c6, 2026-11-19): the row holds eight counts, and it is deleted
on the person's acknowledgement and on nothing else. STATE.md's schema
v14 has stated both rulings since architect-cto's commit 530def301,
which is folded into this note's PR (#738); the sentence quoted above
is the text as it stood before that commit, when this design was
handed to ux-designer.
The third, Q3, was ruled as proposed too (REPLY 01a204c2,
2026-11-22): each pending count and the card proposals count holds
every such row, whichever step discarded it.
Revised 2026-11-21, at #728's review (F29, F30): when the person moved
focus before a late banner showed, focus stays where it is, whichever
notice showed first (§6), and §5 says what the dialog does when a session notice
rises under it, which acceptance 9 now sets up. Before, two rows of §6
both applied to a late banner, and acceptance 9 needed OK with a store
degraded from the open, which hides the banner.

Revised 2026-11-22, at #738's review (F1, F3-F6, F8). Only storage
degraded refuses OK's delete; under another session notice OK deletes
the row (§5, §6, acceptance 9's second run). The late-banner row of §6
covers focus moved on Chats only. Acceptance 9's store hook is named as
missing, so its first run is blocked. The client deletes no all-zero
row, and acceptance 1 checks that no banner shows. §1 counts each
pending and card row whichever step took it, as architect-cto ruled
on Q3 (§11). Before, §5 said that any session notice refused the delete,
and §3 had the client delete an all-zero row, which STATE.md's "on
nothing else" forbids.

Owner of the design: ux-designer. The words are language-culture's: §7
names each string and what it must say. No new colour: the banner is
the existing `NoticeCard` (`design/2026-10-16-android-look.md` §3) and
the details are a Material basic dialog, as the discard dialogs of
`design/2026-10-21-contact-cards.md` §7.2 are. Built by rust-ui-dev, on
the desktop and on Android; the command-line client's form is left out
(§10).

## 1. What is already decided

- STATE.md §Migrations, schema v14 (A 2026-11-19; ADR-0067 decision 7):
  in one transaction the migration rebinds every route that recorded an
  identity key, with its conversation, and discards (c) every
  conversation whose remote endpoint recorded no key, with its unread,
  kept and pending rows, and every contact left with no route; (d) the
  second of two routes of one PeerId whose keys derive one id, its
  contact when it holds no other route, and the second copy of a message
  where two copies meet; (e) every pending row and every unread contact
  card proposal. What (c), (d) and (e) discarded is counted into one
  content-free `settings` row. As ruled on 2026-11-19 (§11; STATE.md
  530def301), the row holds eight counts, each one thing a person
  would call gone or changed: conversations, unread messages, kept
  messages, pending messages never attempted, pending messages that may
  have reached their counterparty, contacts
  ((c)'s: deleted with their last route), contacts combined ((d)'s: a
  route moved to another contact of the same identity, its own name and
  notes gone), card proposals (unread contact card proposals). A second
  copy under (d) is counted nowhere: the message is still held, or was
  read and not kept. The two pending counts and the card proposals
  count each hold every such row the migration discarded, whether (c)
  or (e) took it; the unread and kept counts hold no card row (Q3 in
  §11, ruled as proposed by architect-cto's REPLY 01a204c2, 2026-11-22;
  STATE.md states it in architect-cto's commit 840747b85, which
  reaches main with the human store's schema v14 change).
- The pending rows are counted in two parts, "as Remove of an identity
  names them" (ADR-0061 decision 7): those never attempted were not
  sent; those with an attempt behind them "may have reached their
  counterparty under the previous release … and are said as that,
  never as not sent".
- `human-client-ui.md` §13, the Stage 22 block's first criterion: "the
  client says once, at the first open, how many conversations, unread,
  kept and contacts went and how many pending messages, those never
  attempted as not sent and those attempted before the upgrade as
  possibly received, never as not sent". Written before 530def301:
  STATE.md's schema v14 now says when the notice shows (at each open
  until acknowledged) and how many counts it holds (eight); bringing
  §13 in line is architect-cto's, done in 840747b85 with Q3's
  ruling: at each open until acknowledged, each of the eight counts
  that is not zero.
- The row is content-free (RETENTION.md §7, the one exception): it holds
  counts and nothing else. The notice therefore cannot say which
  conversation, contact or message went, and it names no peer and no
  identity (OBSERVATION 01a1f387).
- The build has a notice shown once in the session-notice slot of Chats
  (`View::show_once`, used by the discard of contact-cards §7.3): it
  waits under a session notice, takes focus on its action when it shows,
  and is not stored. Its test is
  `the_discard_notice_shows_once_under_session_notices_and_goes_on_close`
  (`crates/human/ui-slint/tests/views.rs`).

## 2. The person's goal

The person installed an update and opened the app. Before they use it,
they need to know that the update removed some of what the app held,
how much, and whether they must do anything. Two facts matter most to
what they do next:

- messages they were sending and that were **not sent**: they may want
  to write them again;
- messages they were sending that **may have reached the peer**: they
  must not assume these failed, and the app will not send them again.

They cannot get any of it back, and the app cannot show which items
went. The notice must not invite a search for them, and must not alarm
someone for whom nothing was lost.

## 3. Where, and when

- **Place.** Chats, in the session-notice slot, as the discard's notice
  is: the banner `MigratedBanner` with one action, **Details**. On a
  wide window the slot is the Chats pane's, as for the discard's notice.
  The banner is one short line, so it fits the slot at every text size
  the slot already serves; the counts are in the dialog (§4), whose
  content scrolls.
- **Why a dialog for the counts.** Up to ten lines (eight counts and
  two of explanation, §4) in the banner would fill a phone's screen at
  large text sizes and push Chats below it. A dialog holds them where
  they can scroll, and the person reads them with one press. (Heuristic: visibility
  of system status without blocking the primary task. Hypothesis, not
  tested with a person.)
- **When.** At every open that finds the `settings` row with at least
  one count above zero, until the person presses **OK** in the dialog
  (Q2 in §11, ruled). A row whose counts are all zero shows nothing,
  and the client deletes no row it has not shown; architect-cto ruled
  on 2026-11-22 that the migration writes no row when every count is
  zero (STATE.md, 72434415a, not yet on main).
- **Under what.** A session notice (authentication needed, storage
  degraded, the device offline) goes first, as for the discard's
  notice; the banner shows when no session notice is shown. An identity
  screen (R, K, K1) or the reader over Chats hides the banner as it
  hides the slot.
- **Which identity.** The banner shows on Chats whichever identity is in
  front. Every row it counts was written before the update, when the
  client held one identity, and its words name none: it is about the
  update, not about an identity.
- **Never** in a conversation's notice slot alone: a narrow window that
  opens on a conversation (from a notification) shows the banner when
  Chats is shown.

## 4. The dialog

Pressing **Details** opens a basic dialog:

- title `MigratedTitle`;
- one line per count above zero, in this order, each line with its
  number in the locale's digits and its noun in the locale's plural form:
  1. `MigratedNotSent` — pending never attempted;
  2. `MigratedMayHaveReached` — pending that may have reached;
  3. `MigratedCards` — card proposals not answered;
  4. `MigratedUnread` — unread messages;
  5. `MigratedKept` — kept messages;
  6. `MigratedConversations` — conversations;
  7. `MigratedContacts` — contacts deleted;
  8. `MigratedCombined` — contacts combined.
  The two lines about the person's own sends come first, then the card
  proposals: those are the ones a person may act on (§2), by writing
  again or by asking for a card again.
- `MigratedWhy`: why it happened, and that the app cannot show which
  items;
- `MigratedKeptWhat`: what stayed;
- one action, **OK**, which acknowledges the notice (§5).

Back, Escape and a tap outside close the dialog and change nothing: the
banner stays. The dialog's content scrolls when it is taller than the
dialog; OK stays reachable.

## 5. Acknowledging it

- **OK**: the dialog closes, the banner goes, and the client deletes the
  `settings` row. The banner does not come back in this session or at a
  later open.
- **The delete is refused** (a degraded store): the dialog closes and the
  banner goes for this session, with no further message; the row stays,
  so the banner comes back at the next open. Nothing is lost by that,
  and the degraded store already has its own session notice.
- **A session notice rises while the dialog is open** (authentication
  needed, storage degraded, the device offline): the dialog stays open
  and keeps focus, and under it the notice takes the slot from the
  banner, as at any time (§3). A press on OK then closes the dialog and
  focus goes to the session notice: it rose behind the dialog, so the
  person has not seen it yet. (Heuristic: visibility of system status.)
  Only storage degraded refuses the delete, as above, so the banner
  comes back at the next open; under authentication needed or the
  device offline the store is writable, OK deletes the row, and the
  banner does not come back.
- **The process ends before OK** (killed, a crash, a restart, an
  update of Android itself): the row is still there, so the next open
  shows the banner again. What the person was told matches what the
  store holds: nothing says "done" until the row is gone.
- **The app goes to the background with the dialog open**: on return
  the dialog is still open, as other dialogs are; when the process ended
  in between, the banner shows (above).

## 6. Focus

| case | focus goes to | check |
|---|---|---|
| Chats shows the banner at an open (no session notice, no screen over it) | the banner's Details | keyboard: open a store with the row; Tab order starts on Details; desktop screen reader reads the banner text then "Details, button" |
| a session notice was shown first, and goes | Details, when the banner shows | keyboard: open with storage degraded, then healthy |
| an identity screen or the reader was over Chats, and goes | Details, when the banner shows | keyboard, with K1 raised at the open |
| the person moved focus on Chats before the banner showed (it showed late) | stays where it was; this row applies, not the session-notice or identity-screen row above, even when a session notice or an identity screen showed first; when focus was moved on an identity screen or on the reader, the identity-screen row applies instead | keyboard: open with storage degraded, Tab into the list, then healthy |
| Details pressed | the dialog's title (screen reader), OK (keyboard) | keyboard: Enter on Details, then Enter presses OK; desktop screen reader reads title, the lines in order, the explanation, then OK |
| dialog closed without OK (Back, Escape, outside) | Details | keyboard: Escape returns to Details |
| OK | the Chats list heading, as when the locked notice leaves (unlock note F14) | keyboard |
| OK with a session notice raised while the dialog was open (§5; the delete refused only under storage degraded) | that session notice | keyboard: acceptance 9 |
| OK on a wide window with a conversation open | the Chats pane's heading | keyboard |
| return from the background with the dialog open | OK | keyboard: Alt+Tab away and back (desktop) |

The Android screen-reader checks are a blocked precondition, not a pass,
until the Android build exposes an accessibility tree (contact-cards
§8's statement, unchanged).

## 7. Strings

language-culture writes every word; what follows is what each must say,
and its English words (language-culture, j386). Plural forms and
number formats are the locale's: each count line is one message with
its number, never a number put before a fixed word. For English, a
count line has two forms: "one" for 1 and "other" for every other
number.

| key | must say | words |
|---|---|---|
| `MigratedBanner` | the update removed some stored items; more is one press away | "This update removed some saved items." |
| `Details` | opens the counts | "Details" (the existing `Details` value) |
| `MigratedTitle` | the dialog's subject | "What this update removed" |
| `MigratedNotSent` {n} | n of the person's sends were never sent and will not be | one: "{n} message you were sending was not sent. InterWeave will not send it."; other: "{n} messages you were sending were not sent. InterWeave will not send them." |
| `MigratedMayHaveReached` {n} | n of the person's sends may have reached the peer before the update; the app will not send them again; never "not sent" | one: "{n} message you were sending may have reached the peer before the update. InterWeave will not send it again."; other: "{n} messages you were sending may each have reached its peer before the update. InterWeave will not send them again." |
| `MigratedCards` {n} | n contact cards the person had not answered went; the sender can share theirs again | one: "{n} contact card that you had not accepted or declined was removed. The sender can share it again."; other: "{n} contact cards that you had not accepted or declined were removed. The senders can share their cards again." |
| `MigratedUnread` {n} | n unread messages went, before they were read; never a card | one: "{n} unread message was removed."; other: "{n} unread messages were removed." |
| `MigratedKept` {n} | n messages the person had kept went | one: "{n} message you had kept was removed."; other: "{n} messages you had kept were removed." |
| `MigratedConversations` {n} | n conversations went, each with someone whose contact card the person had not accepted (unread or declined counts as not accepted) | one: "{n} conversation was removed. You had not accepted a contact card from the other person."; other: "{n} conversations were removed. In each, you had not accepted a contact card from the other person." |
| `MigratedContacts` {n} | n contacts went, each with no accepted contact card | one: "{n} contact was removed. You had not accepted a contact card from them."; other: "{n} contacts were removed. You had not accepted a contact card from any of them." |
| `MigratedCombined` {n} | n contacts were joined to another contact of the same person; that person's contact stays, the joined contact's own name and notes did not | one: "{n} contact was merged into another contact of the same person. The contact it was merged into stays, but the name and notes you had given the merged one were removed."; other: "{n} contacts were each merged into another contact of the same person. The contacts they were merged into stay, but the names and notes you had given the merged ones were removed." |
| `MigratedWhy` | each contact is now reached by the key on the card the person accepted; what had no accepted card could not move; the app cannot show which items went | "InterWeave now recognizes each contact by the key on the contact card you accepted from them. Conversations and contacts without an accepted contact card could not stay. InterWeave cannot show which items were removed." |
| `MigratedKeptWhat` | contacts with an accepted card, and their conversations, are still there | "Your contacts with an accepted contact card, and your conversations with them, are still in InterWeave." |
| `MigratedOk` | acknowledges, and the notice will not return; distinct from Close, which elsewhere dismisses with no effect | "OK" (new: no OK string exists) |

No string names a peer, an identity, a PeerId or a label.

## 8. Accessibility

- The banner is the `NoticeCard`: its live region says `MigratedBanner`
  once when it appears; focus on Details reads the banner text with it
  (the card's rule).
- The dialog is modal: focus does not leave it until it closes; its
  title is read first, then the lines in §4's order, then OK.
- Targets: Details and OK at least 48 × 48 dp (Android), the platform's
  minimum on the desktop.
- Contrast: Details's label and focus ring on `notice-background`
  (`notice-text`, `focus-ring-on-notice`, each already 3:1 or more for
  the ring and 4.5:1 for text); the dialog's text and OK on the surface
  the existing Details dialog uses (android-look §3), with that dialog's
  pairs. No new colour pair.
- Text at 200 %: the banner wraps; the dialog's content scrolls with OK
  visible.
- Right-to-left: the lines start at the reading start; the number sits
  where the locale's message puts it.
- No motion beyond the dialog's platform transition, off under reduced
  motion.

## 9. Acceptance

Each on the desktop and on Android unless it says otherwise, from a
schema-v13 store written by the release before Stage 22 holding the
cases named (the store's own migration tests build such stores).

1. A store whose migration discarded nothing opens with no banner, at
   that open and at the next (no `MigratedBanner` drawn; store query:
   no row with a count above zero).
2. A store with one of each discard opens with `MigratedBanner` on Chats
   and focus on Details; nothing else on Chats moved below the slot's
   usual place.
3. Details opens the dialog with exactly the lines whose counts are
   above zero, in §4's order, each number equal to the row's count; a
   store with only never-attempted pending shows `MigratedNotSent` and
   no other count line.
4. A store with only attempted pending shows `MigratedMayHaveReached`
   and no `MigratedNotSent`: an attempted send is never said as not
   sent.
4a. A store where two routes of one PeerId derive one id, with one
   message stored under both, shows `MigratedCombined` 1 and no
   `MigratedUnread` or `MigratedKept` line for that message; a store
   whose only discard is an unread card proposal shows `MigratedCards`
   and no `MigratedUnread`.
5. Escape (desktop) or Back (Android) closes the dialog, the banner
   stays, focus is on Details, and the row is still there.
6. OK closes the dialog, removes the banner, puts focus on the Chats
   list heading, and the row is gone (store query); a restart shows no
   banner.
7. Killing the process with the banner or the dialog showing, and
   opening again, shows the banner again with the same counts.
8. Opened with a session notice raised (storage degraded), the banner is
   not drawn; when the notice goes, the banner shows with focus on
   Details. In a second run, opened the same way, the tester moves
   focus into the list before the notice goes: when the banner shows,
   focus stays where the tester put it.
9. With the dialog open, the open store is made to refuse writes, by
   a debug or test hook on each platform: the dialog stays open with
   focus where it was, and storage degraded takes the slot under it.
   OK then closes the dialog, the banner is gone for the session,
   focus is on the storage-degraded notice, the row stays (store
   query), and the next healthy open shows the banner. No such hook
   exists today (Android's `REFUSE_STORE`, `DebugHooks.kt`, refuses
   the store at the next start; the view tests feed only the session
   state, which refuses no write), so this first run is blocked, not
   passed, until rust-ui-dev adds one. A second run, on Android, raises the
   device-offline notice under the open dialog (airplane mode on): OK
   closes the dialog, focus is on that notice, the row is gone (store
   query), and the next open shows no banner.
10. With a second identity added and in front before OK, the banner
    still shows on Chats, and no string in the banner or the dialog
    names either identity (read the strings).
11. At 200 % text (Android font scale 2.0; desktop the largest
    supported size) on a 360 dp-wide phone, the banner's text and
    Details are fully shown and the dialog's OK is reachable by
    scrolling or without it.
12. Desktop screen reader: the reading order of §6 and §8. Android
    TalkBack: blocked until the accessibility tree exists.

## 10. Deliberately left out

- The command-line client's form: its own note, if `human-cli` reports
  the migration at all.
- Which items went: the row is content-free by contract.
- A way to recover anything, or to export before the update: no such
  path exists (ADR-0067 decision 7, the owner's word that no installed
  base exists).
- A notification outside the app: the notice is for the person who
  opens it.
- How long the migration takes and what shows while it runs: the open's
  existing start state; a migration long enough for a person to notice
  is the builder's to report.

## 11. Questions

To architect-cto, each the product's and not a layout's. Q1 and Q2
were ruled as proposed (REPLY 01a1f3c6, 2026-11-19), with (ii) as a
separate count, and Q3 as proposed (REPLY 01a204c2, 2026-11-22); the
text below is each question as asked.

- **Q1. What each count holds.** STATE.md lists six counts and puts
  three different things under "discarded". (i) A second copy of a
  message under (d) is not a message the person lost: the first copy
  stays. Counting it as "unread" or "kept" tells the person a message
  went when none did. Proposal: (d)'s second copies are not counted.
  (ii) A contact deleted under (d) is merged into another contact of the
  same person, not lost; but the person's own edits on it (a name, a
  note) are lost. Proposal: a separate count, said as "{n} contacts were
  combined with another contact of the same person; their own names and
  notes were not kept", or, if no count is added, not counted as gone.
  (iii) An unread card proposal under (e) is not a message. Proposal: a
  count of its own, said as "{n} contact cards you had not answered",
  which the person can act on by asking for the card again; otherwise
  the unread line says "messages and contact cards".
- **Q2. When the row is deleted.** STATE.md says the client "shows
  [it] once at the first open and then deletes" it. Read literally, a
  process killed at that open (or a person who closes the app at once)
  never learns what went. This note deletes the row when the person
  presses OK, so the banner returns at each open until then (§5).
  Proposal: STATE.md reads "shows at each open until the person
  acknowledges it, then deletes".
- **Q3. Which step's rows each count holds** (asked 2026-11-22, at
  #738's review, F3 and F8; ruled as proposed, REPLY 01a204c2). STATE.md names the two pending counts
  "(e)'s two parts" and the card proposals "the unread rows of kind
  `card` that (e) discarded", while (c) also discards the pending and
  unread rows of every conversation it removes, and STATE.md fixes no
  order between the two steps. Applied in the order (a) to (e), (c)
  runs first and takes the sends and cards of exactly the conversations
  most likely to hold them (those with no accepted card), and the
  notice would tell the person that fewer sends were lost than really
  were.
  Proposal: the two pending counts and the card proposals count each
  hold every such row discarded, whichever of (c) and (e) took it; the unread and kept counts
  hold no card row. Also asked with it:
  `human-client-ui.md` §13's first Stage 22 criterion still says
  "once, at the first open" and names six counts; §1 above follows
  STATE.md, and §13 is architect-cto's to bring in line.
