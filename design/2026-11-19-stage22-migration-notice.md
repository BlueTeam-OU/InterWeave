# What the Stage 22 update removed: the notice at the first open — desktop and Android

Status: proposal, 2026-11-19, my job j148. Read against `origin/main`
94ecad7f2 (#730 merged). architect-cto's OBSERVATION 01a1f387 handed
this design to ux-designer: the store migration of the Stage 22 release
counts what it discarded into one content-free `settings` row, "which
the client shows once at the first open and then deletes; the words are
ux-designer's" (STATE.md §Migrations, schema v14). This note's two
product questions (§11) were ruled by architect-cto as proposed (REPLY
01a1f3c6, 2026-11-19): the row holds eight counts, and it is deleted
on the person's acknowledgement and on nothing else. STATE.md's text
follows as architect-cto's for/ branch onto this PR; until it lands, the
merged sentence still reads "shows once at the first open and then
deletes", and this note follows the ruling.

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
  content-free `settings` row. As merged it names six counts; ruled
  2026-11-19 (§11) it holds eight, each one thing a person would call
  gone or changed: conversations, unread messages, kept messages,
  pending never attempted, pending that may have reached, contacts
  ((c)'s: deleted with their last route), contacts combined ((d)'s: a
  route moved to another contact of the same identity, its own name and
  notes gone), card proposals ((e)'s unread proposals). A second copy
  under (d) is counted nowhere: the message is still held, or was read
  and not kept.
- The pending rows are counted in two parts, "as Remove of an identity
  names them" (ADR-0061 decision 7): those never attempted were not
  sent; those with an attempt behind them "may have reached their
  counterparty under the previous release … and are said as that,
  never as not sent".
- `human-client-ui.md` §13, the Stage 22 block's first criterion: "the
  client says once, at the first open, how many conversations, unread,
  kept and contacts went and how many pending messages, those never
  attempted as not sent and those attempted before the upgrade as
  possibly received, never as not sent".
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
- messages they were sending that **may have been received**: they must
  not assume these failed, and the app will not send them again.

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
- **Why a dialog for the counts.** Up to seven lines of counts and
  explanation in the banner would fill a phone's screen at large text
  sizes and push Chats below it. A dialog holds them where they can
  scroll, and the person reads them by one press. (Heuristic: visibility
  of system status without blocking the primary task. Hypothesis, not
  tested with a person.)
- **When.** At every open that finds the `settings` row with at least
  one count above zero, until the person presses **OK** in the dialog
  (Q2 in §11, ruled). A row whose counts are all zero shows nothing, and the
  client deletes it at that open.
- **Under what.** A session notice (authentication needed, storage
  degraded, the device offline) goes first, as for the discard's
  notice; the banner shows once no session notice is shown. An identity
  screen (R, K, K1) or the reader over Chats hides the banner as it
  hides the slot.
- **Which identity.** The banner shows on Chats whichever identity is in
  front. Every row it counts was written before the update, when the
  client held one identity, and its words name none: it is about the
  update, not about an identity.
- **Never** in a conversation's notice slot alone: a narrow window that
  opens into a conversation (a notification) shows it when Chats is
  shown.

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
- **The process ends before OK** (killed, a crash, a restart, the
  update of Android's own): the row is still there, so the next open
  shows the banner again. What the person was told matches what the
  store holds: nothing says "done" until the row is gone.
- **The app goes to the background with the dialog open**: on return
  the dialog is still open, as other dialogs are; when the process ended
  in between, the banner shows (above).

## 6. Focus

| case | focus goes to | check |
|---|---|---|
| Chats shows the banner at an open (no session notice, no screen over it) | the banner's Details | keyboard: open a store with the row; Tab order starts on Details; desktop screen reader reads the banner text then "Details, button" |
| a session notice was shown first, and goes | Details, once the banner shows | keyboard: open with storage degraded, then healthy |
| an identity screen or the reader was over Chats, and goes | Details, once the banner shows | keyboard, with K1 raised at the open |
| the person moved focus before the banner showed (it showed late) | stays where it was | keyboard |
| Details pressed | the dialog's title (screen reader), OK (keyboard) | keyboard: Enter on Details, then Enter presses OK; desktop screen reader reads title, the lines in order, the explanation, then OK |
| dialog closed without OK (Back, Escape, outside) | Details | keyboard: Escape returns to Details |
| OK | the Chats list heading, as when the locked notice leaves (unlock note F14) | keyboard |
| OK on a wide window with a conversation open | the Chats pane's heading | keyboard |
| return from the background with the dialog open | OK | keyboard: Alt+Tab away and back (desktop) |

The Android screen-reader checks are a blocked precondition, not a pass,
until the Android build exposes an accessibility tree (contact-cards
§8's statement, unchanged).

## 7. Strings

language-culture writes every word; what follows is what each must say,
with a proposal in English to make the flow concrete. Plural forms and
number formats are the locale's: each count line is one message with
its number, never a number put before a fixed word.

| key | must say | proposal |
|---|---|---|
| `MigratedBanner` | the update removed some stored items; more is one press away | "This update removed some stored messages." |
| `Details` | opens the counts | "Details" |
| `MigratedTitle` | the dialog's subject | "What this update removed" |
| `MigratedNotSent` {n} | n of the person's sends were never sent and will not be | "{n} messages you were sending were not sent" |
| `MigratedMayHaveReached` {n} | n of the person's sends may have been received before the update; the app will not send them again; never "not sent" | "{n} messages you were sending may have been received before the update. They will not be sent again." |
| `MigratedCards` {n} | n contact cards the person had not answered went; the sender can share theirs again | "{n} contact cards you had not answered" |
| `MigratedUnread` {n} | n unread messages went, before they were read; never a card | "{n} unread messages" |
| `MigratedKept` {n} | n messages the person had kept went | "{n} kept messages" |
| `MigratedConversations` {n} | n conversations went, all of them with someone who never shared a contact card | "{n} conversations with no contact card" |
| `MigratedContacts` {n} | n contacts went, each with no contact card | "{n} contacts with no contact card" |
| `MigratedCombined` {n} | n contacts were joined to another contact of the same person; that person's contact stays, the joined contact's own name and notes did not | "{n} contacts were combined with another contact of the same person. Their own names and notes were not kept." |
| `MigratedWhy` | each contact is now reached by the key on their card; what had no card could not move; the app cannot show which items went | "Each contact is now reached by the key on their contact card, so what had no card could not be kept. The app cannot show which items were removed." |
| `MigratedKeptWhat` | contacts with a card, and their conversations, are still there | "Your contacts with a contact card and their conversations are still here." |
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

1. A store whose migration discarded nothing opens with no banner, and
   the `settings` row is gone after that open (store query).
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
   Details.
9. With the store degraded, OK closes the dialog and the banner for the
   session, the row stays, and the next healthy open shows the banner.
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

To architect-cto, each the product's and not a layout's. Both ruled
as proposed (REPLY 01a1f3c6, 2026-11-19), with (ii) as a separate
count; the text below is the question as asked.

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
