# Contact cards, contacts and discarding the store — desktop and Android

Status: proposal for plan §29 steps 2–4, 2026-10-21, at architect-cto's
request (REQUEST 01a15d2a); revised the same day for plan §30 (Stage 22)
at their request (REQUEST 01a15efd), when the owner made a user identity
an EndpointId of the node with its own card (ADR-0061, ADR-0059 A
2026-10-21 (ii)). rust-ui-dev builds the card flows when the owner opens
Stage 21 and the identity flows (§5.7) when the owner opens Stage 22.
The rules it answers to: ADR-0059, ADR-0060 decision 6, ADR-0061,
HUMAN-CHAT.md §Contact card, STATE.md (`own_cards` keyed by EndpointId,
`conversation_index.local_endpoint_id` and `card_shared_at`,
`contacts.local_endpoint_id`, §Migrations) and `human-client-ui.md`
§3–§6, §10–§13. This note decides how a person meets those rules, and
changes none of them. Where it needs one changed or settled, §12 asks.

Owner of the design: ux-designer. The words are language-culture's: §10
names each string, where it shows and what it must say; its text is a
proposal for language-culture-en to write. Colours are named as roles of
`design/2026-10-16-android-look.md` §3; a new role is named with the
contrast it must hold, and rust-ui-dev adds it to `app.slint`.

Both clients render the same Slint UI (`crates/human/ui-slint`) from the
same `ui-model`. A screen below is one component; where Android and the
desktop differ, the difference is stated beside it. "Narrow" is the
layout `app.slint` already uses for a phone (one pane); "wide" is the
desktop's two panes.

## 1. What the owner asked, and how it is read

The owner, 2026-10-21, as ADR-0059 quotes:

> create a contact system, internal, absolutely separated from the
> Android contact system … load its own contact data, then send it to a
> counterparty by app with some tag … associated to the id from which
> the contact details are received

> a user should define more than one identity and choose what to send

The same day the owner said what an identity is, and how the app must
treat two of them (ADR-0061's status line):

> An EndpointId should match a user identity. One user of the app can
> have many user identities on the same phone. So for any EndpointId he
> can add a display_name and a photo

> two endpointid are two identities even if they belong to one user.
> Avoid any ux that can let the user think that is operating with one
> while operating instead with the other

Read here as: the person holds one or more **identities** on one device,
each a configured EndpointId with one card of its own (a name, an
optional family name and an optional photo); every conversation speaks
as one identity for life, and the app shows one identity at a time,
names it wherever the person can act, and changes it only when the
person switches (ADR-0061 decisions 1, 3 and 9). The card the person
shares in a conversation is that conversation's identity's card; there
is nothing to choose. What they receive becomes a contact only when they
say so, under the identity that received it (ADR-0059 decision 3).

The word "identity" is now the person's word for one of these. The app
used it until now for the PeerId and its recovery phrase
(`design/2026-10-19-android-identity.md`), and some of this note's own
strings did (§7, §10). Two things one word cannot both mean on one
screen; §12 L1 asks language-culture for the person-facing words of the
two, and every string here that named the device's identity is marked
for that pass.

And, 2026-10-16, for the message swipe (`design/2026-10-16-message-swipe.md` §1):

> Swipe right → Keep / Like / Accept
> Swipe left → Discard / Reject / Skip

At the time nothing could be accepted. A received card can be, so §5.4
gives it that swipe, with two narrowings the owner agreed to (§12 O1,
O2).

## 2. The person's goals

- **Identities:** "Be the right person to the right people, and never
  mix them up." Most people will have one identity and never see the
  word. Some want a work identity and a private one. They need to know,
  at every moment they can send or act, which one they are, and that
  nothing done as one is done as the other. An EndpointId means
  nothing to them; they think of a name and a face.
- **Share:** "Tell this person who I am." They are in a conversation
  that shows the other side as `…x6gY6Eri` and want it to show their
  name instead. They need to see what goes before it goes, and that only
  this person gets it.
- **Receive:** "Is this really Anna? Then save her." They have no way
  to know who is behind a PeerId except what they already know of it.
  Accepting must be one step, but a stranger calling themselves "Anna"
  must not slip into the place of the real Anna.
- **Recognise:** "Find Anna's conversation." Names and faces in the
  list and the title, and a way to tell two Annas apart.
- **Recover the store:** "Get the app working again." The app cannot
  read its own stored data. The person did nothing wrong. They need to
  know what they lose before they lose it, and that their PeerId and
  trust are not part of it.

## 3. Rules every screen here keeps

From the contracts, listed so a reviewer can check each screen:

- **A card is never sent without the person's Share** on one direct
  conversation. No screen sends a card on first contact, on opening a
  conversation, on receiving a card, or to a channel (ADR-0059 d. 2).
  A channel conversation offers no Share at all.
- **Nothing reaches `contacts` or `contact_routes` before Accept**
  (d. 3). Decline and an unanswered proposal leave nothing in the store.
- **The photo of a received card is not decoded before Accept.** Until
  then the proposal shows a neutral placeholder and the words "includes
  a photo" (d. 5).
- **The PeerId is always beside a claimed name** on a proposal, in
  full, in the exact canonical form, copyable (`human-client-ui.md` §3,
  §11).
- **No screen and no message tells a counterparty about the person's
  other identities** (d. 2): a card carries one identity's name and
  photo and nothing else. What the node itself shows is said, not
  hidden: a counterparty who holds two of the person's identities sees
  one PeerId beside both (ADR-0061 d. 5), and the identity pages say so
  (§5.1).
- **One identity in front, named wherever the person can act** (ADR-0061
  d. 9, §5.7): every screen shows only the identity in front's
  conversations, contacts and card; every screen that can send, share,
  accept, create or delete names it, and the send control names it
  beside itself; it changes only on one of the person's acts that §5.7.3
  lists, and is named right after; no list,
  search, chooser or warning draws on another identity's data. While
  the person holds one identity, nothing asks and the identity is shown
  only where asked for (ADR-0061 d. 3).
- **A local edit is never overwritten by a card** (d. 3).
- **A name is plain text.** A received name is never rendered as
  markdown, never a link, and is shown inside a bidirectional isolate
  (Unicode FSI…PDI, or the toolkit's equivalent), so a right-to-left or
  mixed name cannot reorder the words around it. This holds in every
  place a received name appears: the proposal, the same-name note, the
  title, the list, Details, the contact page, an announcement.
- **No contacts permission and no platform contact picker** (d. 6). The
  photo of one's own card comes from Android's Photo Picker, which needs
  no permission, or from the desktop's file dialog.
- **Labels and avatars are not trust** (`human-client-ui.md` §3). A
  photo or initials avatar is decoration; the trust indicator stays tied
  to the PeerId and is shown in words, on the contact page (§6.3).
- **Discarding the store is confirmed, and the confirmation lists what
  goes** (ADR-0060 decision 6, Operational implications). Nothing is
  deleted without the person's word, and the loss is listed once and
  taken once.

## 4. Where things are

The Chats screen's top app bar is look spec §5's: one overflow button
(⋮, labelled `AppOptions`, never "More options", Android review F12)
whose menu holds "Trust". On main the shield of an earlier revision is
still drawn (`app.slint`); this spec builds on the look spec, not on
that. The menu gains two items after "Trust", in order:

1. **Trust** (as today) → the Trust page
2. **Contacts** (`ContactsTitle`) → §6.2
3. **Your identities** (`IdentitiesTitle`) → §5.1; with one identity
   held, the item reads **Your card** (`OwnCardTitle`) and opens that
   identity's card (§5.2) directly, since a list of one is a step for
   nothing and the word "identity" is not needed yet (ADR-0061 d. 3)

While two or more identities are held, the same bar carries the
**identity switch** (§5.7.1) between the title and the ⋮.

The menu's focus and closing rules stay look spec §5's. The Chats ⋮
keeps its own accessible name, so that on a wide layout, where it and a
conversation's "More options" are on screen together, a screen-reader
user can tell them apart (WCAG 2.4.6, the reason F12 gave). On the
desktop the same bar heads the list
pane, so the same menu is there. Each opens a full-screen page on a
narrow layout and takes the conversation pane's place on a wide one,
with the same back arrow the Trust page uses ("Back to conversations",
`BackToConversations`); Back (Android) and Escape (desktop) do the same.

`human-client-ui.md` §1 lists Contacts as an area beside Chats. A
navigation bar with two destinations on a phone would cost a permanent
80 dp for a page opened rarely, so it is reached from the menu. §11
names this as a choice left open for later.

A direct conversation's overflow (⋮) gains **Share a card**
(`ShareCard`), above Details. A channel's overflow does not.

## 5. Flows

### 5.1 Your identities (list)

Shown while two or more identities are held; with one, the menu opens
§5.2 for it (§4). What is visible, top to bottom:

- the app bar: back arrow, title "Your identities";
- the shared-device statement (`IdentitiesShareDevice`), always, not
  behind a link (ADR-0061 d. 5): your identities use this device's one
  PeerId and its network addresses, so someone who knows two of them
  can tell they are on the same device, as can the network; to stop the
  people they talk to from linking two identities, the person needs a
  separate profile, which the desktop can run and this phone cannot,
  and even then the network sees one computer (ADR-0061 d. 5; the
  Android text ends with what is true there); and, in the same block, that an identity's
  label is no secret from the peers this device trusts: one that learns
  or guesses it can write to that identity, and no setting narrows
  that today: an identity has the device's trust (ADR-0061 d. 5);
- the identities, one row each, in the order they were created: a 40 dp
  avatar (the card's photo, or its initials, §6.1, on the hue of the
  identity's EndpointId, so two identities of one PeerId do not look
  alike), the card's name and family name in title medium, and under it
  in body medium the route label (the EndpointId as `DirectTitle`
  shows a route, `IdentityRouteLabel`) and, on one row only, **Default**
  (`IdentityDefault`); the identity in front carries a check mark and
  `IdentityInFront` in its accessible description. An identity whose
  card has no name yet shows its route label in the name's place;
- **Add an identity** (`AddIdentity`), a filled button at the bottom,
  above the gesture inset, the one primary action.

States:

- **Loading**: the store read is local and fast; nothing is drawn until
  it answers.
- **Store degraded** (`StorageDegraded` already shown by the app): the
  list shows what it read; Add, Save and Remove are disabled with the
  existing degraded notice above them, since a card that cannot be
  stored cannot be saved.
- **At the bound**: when the profile holds 64 endpoints, the
  configuration's bound (ENDPOINTS.md), authored and added counted
  together, agent endpoints included (§12 Q7–Q10), Add an identity opens
  nothing and `IdentityLimit` shows
  under it, focused.

A row opens that identity's page (§5.2). The list is the one place where
identities are shown side by side, and it shows nothing of their
conversations or contacts: it is a list of who the person can be, not of
what each holds. Opening an identity's page does not switch to it; a
page for an identity not in front says so in its app bar (§5.7.2).

**Add an identity** opens §5.2 empty, with one field more: the route
label (`IdentityRouteField`), required, under the name. The label is the
EndpointId; every person this identity talks to sees it beside its
PeerId (`human-client-ui.md` §4: two identities of one device are told
apart by it), so its helper (`IdentityRouteHelp`) says so, and it is
fixed once saved (§12 Q1). It takes ENDPOINTS.md's grammar: lowercase
letters, digits, `.`, `_` and `-`, starting with a letter, at most 64;
typed capitals become lowercase as typed, and a space becomes `-`, so
the person never meets an error for a key they could not avoid. Errors,
on Save, inline: `IdentityRouteNeeded`, `IdentityRouteBadCharacters`,
`IdentityRouteTooLong`, `IdentityRouteTaken` (any endpoint the profile
has uses it: an identity's, or a route's such as an agent's, which Add
never takes over, ADR-0061 d. 1). A label freed by Remove is no
endpoint any more, so it is accepted like a new one; what reusing it
means for the old identity's contacts was said in Remove's dialog
(§5.2), the one moment the client knows it. Save creates the endpoint and its card together
(ADR-0061 d. 2: through the admin connection on the desktop, the
embedded host's configuration path on Android; rust-ui-dev's), the
endpoint with `advertise: false`, as every identity's endpoint has, the
first one's (`human`) included, so the endpoint directory lists none of
the person's identities to a trusted peer; no control changes it
(ADR-0061 d. 5). Save runs with a
progress indicator on Save while it runs; a refusal writes nothing and
shows `IdentityAddFailed` above Save, except a refusal for the limit,
which shows `IdentityLimit` there instead, since "try again" could
never succeed. A new identity is not put in front:
the list returns with the new row focused, and switching is the
person's act (§5.7.3).

### 5.2 An identity's card

Opened from a row of §5.1, from Add an identity (empty, §5.1), from the
menu's Your card while one identity is held (§4), or from Share when the
identity's card has no name yet (§5.3). App bar: back arrow, title
`CardEdit` ("Your card"), and, while two or more identities are held,
the identity's name as the subtitle, so an edit is never made to one
identity while the person believes it is another's. The overflow, with
two or more identities held, has **Make default** (`MakeDefault`) and
**Remove this identity** (`RemoveIdentity`); with one, it has **Add
another identity** (`AddAnotherIdentity`), the way to a second, and,
only while the profile has no default, **Make default** (below).

Fields, in order:

1. **Photo**: a 96 dp circle (accessible label `CardPhotoLabel`, or
   `CardPhotoNone` without a photo) showing exactly what will be sent (the
   prepared photo, below), or the initials of the name as typed so far
   on the identity's hue (§5.1); under it **Add a photo** (`AddPhoto`) or, with
   one, **Change photo** (`ChangePhoto`) and **Remove photo**
   (`RemovePhoto`).
2. **Name** (`CardName`), required.
3. **Family name** (`CardSurname`), optional, marked as optional in its
   label.
4. With two or more identities, the **route label**, read-only, with
   `IdentityRouteHelp`; and the default line: `IdentityIsDefault` on the
   default identity, else nothing.

Then the note `CardSharedStays` (a card already shared stays with the
person who has it; a change here reaches them only when the person
shares it again), and **Save** (`Save`), filled, at the bottom.

Input rules, checked on Save and shown inline:

- Name and surname: after NFC, 1 to 64 Unicode scalar values, no control,
  format or line/paragraph separator characters (HUMAN-CHAT.md). A
  counter `{n} / 64` shows under a field once it holds 56 or more. A
  pasted value with a line break has it replaced by a space on paste,
  since the person cannot see why it would be refused.
- A name of only spaces counts as empty.

On Save with an invalid field: nothing is written, each invalid field
shows its error (`CardNameNeeded`, `CardNameTooLong`,
`CardNameBadCharacters`) under it in `error`, and focus goes to the
first invalid field. Save is never disabled: a disabled button does not
say what is missing.

On a valid Save: the `own_cards` row of this identity is written, the
page closes back to where it was opened from (the list, the Chats
screen, or the share sheet, §5.3), and focus goes to the identity's row
in the list, else to the control that opened the page. No snackbar: the
row, or the sheet's preview, is the confirmation. A card has no Delete
of its own: it is the identity's, and goes only with the identity.

**Leaving with unsaved changes.** While the editor holds unsaved
changes, every way of leaving it opens one dialog first,
`CardDiscardChanges`, whichever control was used (rust-ui-dev-02's
QUESTION 01a1757e, 2026-10-26). Whether a change is unsaved is judged
as the last item of the photo step says (below). The ways of leaving
are:

- Back (Android), Escape (desktop) and the page's back arrow;
- an identity switch (§5.7.3);
- a notification's conversation opening over the editor, on either
  layout;
- on a wide layout, where the conversation list stays beside the
  editor (§4): a conversation picked in the list, and a page opened
  from the app options (Trust, Contacts or Your identities). With one
  identity the menu's Your card opens this same editor (§4), so
  choosing it there leaves nothing: the editor stays as it is, with
  its changes, and nothing is asked.

The dialog asks about the loss of changes in the same way, whatever
causes it. A change lost without warning, because the person picked a
row beside the editor, is the slip that heuristic 5, error prevention,
asks a design to prevent. The actions are **Close without saving**
(`DiscardChanges`, text button) and **Continue editing** (`KeepEditing`, filled, default
focus), stacked as the proposal's are, for the same reasons (§5.4):
each on a line of its own, the full width of the dialog, Close without
saving above Continue editing, at every width and text scale
(rust-ui-dev-02's INFO 01a17517 on #453, built so).

- **Close without saving** drops the changes, and the action that
  opened the dialog continues: Back closes the editor, the switch
  switches, the picked conversation or page opens. Focus then goes
  where that action puts it with no unsaved change: after Back, where
  a saved card's page puts it (the identity's row in the list, else
  the control that opened the page, item 1's table); after a switch,
  as §5.7.3 says; on the opened page, as its own row in item 1's
  table says.
- **Continue editing** closes the dialog and changes nothing: the
  editor keeps its changes, and the list's selection stays where it
  was before the pick. Focus goes back to the control that had focus
  when the dialog opened: an editor field, or the back arrow when it
  was used; the switch; the picked row. If that control no longer exists, as with an item of a
  menu that has closed, focus goes to the control that opened the
  menu: the app options button.

The dialog has no body line and no Save. The title says what Close
without saving does, so the person knows what happens to the changes
(a question from language-culture-en's cold read, OBSERVATION
01a17503). Save stays in the editor, because Save can find an invalid
field, whose error belongs under that field (above), where a dialog
cannot show it. With no unsaved change, each of these ways of leaving
acts at once.
What the build keeps of unsaved changes when the system ends the app
in the background is not specified here.

**Two rules every action on an identity keeps**, on both platforms: when
there is a default, it is an identity that is turned on, so peer-only
sends reach one that can receive them, and a profile may have none
(below) (ADR-0028's endpoint overlay and
ENDPOINTS.md: the default references an enabled endpoint or there is
none); and no action leaves the profile with no identity turned on,
which the runtime needs (ADR-0061 d. 7). Where the overlay would clear
the default when it is turned off, the note instead does not offer Turn
off on the default, an interaction choice of its own: a cleared default
silently leaves peer-only senders reaching no one, while asking the
person to choose another default first keeps that decision theirs. Make
default, its Undo, Turn off, Remove and their states below each obey
both; an action or an Undo that would break one is not offered, or is
withdrawn when it would.

**Make default** is offered only on an identity that is turned on (on
one that is off, the page says `IdentityDefaultNeedsOn`: turn it on
first). It runs at once: the default moves to this identity, the
line `IdentityIsDefault` appears, and a snackbar `ConfirmDefaultSet`
with Undo (moves it back), withdrawn — its snackbar closes — as soon as
the previous default is turned off or removed. From **no default**
there is no Undo: no default is the state the action repairs, and no
record names a way to write it back, so the snackbar is
`ConfirmDefaultSet` without its action. A kill inside the
undo window leaves the new default, as the snackbar said. **No
default**: a profile can arrive without one (a configuration edited
outside the app, or the overlay clearing it); then no row in §5.1
carries Default, the list shows `IdentityNoDefault` above the rows
(people who write to this device without using a label reach none of
your identities), and Make default on any identity that is on sets
one. With one identity held there is no list: the identity's card page
(§4's Your card) shows `IdentityNoDefault`'s one-identity form under
its fields, and its overflow holds **Make default** beside Add another
identity, until a default is set; the word "identity" stays out of it,
as everywhere with one (ADR-0061 d. 3): its result is
`ConfirmDefaultSetOne` (people who write to the PeerId now reach the
person), no Undo, the no-default line gone and no default line in its
place (§5.2 field 4 shows only with two or more), and a refusal shows
`IdentityChangeFailed`'s one-identity form. With two or more
identities held, its helper on the page says what the default
is in the person's terms (`IdentityDefaultHelp`: people who write to
this device's PeerId without using one of the person's labels reach the
default; ADR-0061
Operational implications). It does not put the identity in front.

**Which identities can be removed.** An identity is an endpoint the
person holds as one: the first identity, the profile's authored `human`
endpoint (`human-client-ui.md` §2 step 4), or one the client added (the
overlay's `added` member, which only Add an identity writes); any other
configured endpoint, an agent's included, is a route and never an
identity (ADR-0061 d. 1). Only an added identity can be removed. The
first is never removed; it can be turned off instead (below).

**Remove this identity**, on an added identity, is not offered on the
default identity (the page says `IdentityRemoveDefault`: make another
identity the default first), nor when removing it would leave no
identity turned on, on either platform (the rules above; the embedded
runtime needs one to lease, and the desktop's composed profile asks for
one, ADR-0061 d. 7). Elsewhere it opens a confirmation
dialog, `IdentityRemoveTitle` naming the identity, listing what goes
(`IdentityRemoveLoses`: its conversations and their messages, unread,
kept and waiting to be sent; its contacts; its card; and that people
who write to this identity are no longer reached). Messages waiting to
be sent are deleted and tried no further; when any of them was already
tried, so that it may have reached its recipient (RETENTION.md's
`may_have_reached`), the list says so with their number
(`IdentityRemoveMayHaveReached`), since deleting it does not take it
back. The endpoint is deleted from the overlay, so its label is free:
the dialog's last line, `IdentityRemoveLabel`, says that a new identity
may take that label later, and that whoever had this identity as a
contact would then reach the new one, under the contact they saved for
this one, until they accept its card — the one moment the person can
weigh that, since Add an identity cannot tell a freed label from a new
one. A removed identity is gone: it is no longer listed, counted among
the identities held or against the limit, offered in the switcher or
able to come in front. Then **Cancel**
(default focus) and **Remove** (`IdentityRemoveConfirm`, filled in
`error`). Enter on the initial focus cancels; Escape and Back cancel.
Remove has no Undo (ADR-0061 d. 7): the endpoint and every row under it
go together. Where it lands, by what is left:

- **Two or more identities remain**: the list returns, focus on the
  next row, else the previous, and the snackbar `ConfirmIdentityRemoved`
  ("{identity} removed", no Undo). When the removed identity was in
  front, the default comes in front (§5.7.3's act 4) and the one
  snackbar is `ConfirmIdentityRemovedSwitched`, naming both the removed
  identity and the one now in use, in place of the switch's own, which
  could offer Switch back to nothing; focus follows the list's rule,
  not the switch's.
- **One identity remains**: there is no list any more (§5.1), so the
  app shows Chats of the remaining identity, without the switch or any
  "As" line (§5.7), focus on Chats' title, and `ConfirmIdentityRemoved`
  names the removed identity; nothing names the remaining one, since
  with one identity it is shown only where asked for (ADR-0061 d. 3).

**Turn off this identity** (`TurnOffIdentity`), in the overflow of the
first identity's page in place of Remove, with two or more identities
held, offered once another identity is turned on and not on the default
(the rules above) (ADR-0061 d. 7). It runs at once,
nothing is deleted: the endpoint is disabled, the identity stays
listed as off (§5.7.6), its conversations, contacts and card stay, and
people who write to it are not reached while it is off. A snackbar
`ConfirmIdentityOff` with Undo (turns it on again); the overflow then
holds **Turn on this identity** (`TurnOnIdentity`), which runs at once
with `ConfirmIdentityOn`. **Turn on** is in the overflow of the page
of any identity that is off, with two or more held, however it came
to be off — the first identity by Turn off, an added one in the
configuration (§5.7.6) — beside Remove on an added one, so no state
leaves the person without it and `IdentityDefaultNeedsOn` always has a
control to follow. One identity held and off is not a state the app's
actions reach (the two rules above), and a profile edited into it
outside the app cannot start the runtime, which needs one identity on
to lease (ADR-0061 d. 7): the app then shows its existing start-up
refusal, not this page, so no string here needs a one-identity form
for Turn on. A
kill inside the undo window leaves it off, as said. While an identity
is off, its messages waiting to be sent stay waiting, shown as any
pending message is (`human-client-ui.md` §5), and are tried when it is
turned on again; nothing is sent as another identity. Turning off is
not a switch: when the first identity is in front it stays in front,
readable, its composer saying it cannot send (§5.7.6). Focus stays on
the overflow button.

**When the device refuses.** Make default, Remove, Turn off and Turn on
write the profile's configuration (ADR-0061 d. 2). A refusal or a
failure changes nothing the person sees as done: no snackbar of
success, the page as it was, and `IdentityChangeFailed` in a snackbar
with the text for that action, naming the identity while two or more
are held (with one, Make default's form names none, as nothing shown
with one identity does), and no Undo, since nothing happened. A refused Remove closes its dialog first; after any
refusal focus is on the overflow button of the identity's page.

**The photo.** Add a photo opens Android's Photo Picker (images only,
one item) or the desktop's file dialog filtered to PNG and JPEG. The
client prepares what will be sent from the picked image:

- first turns the pixels upright by the image's own orientation (an
  EXIF Orientation tag, as a camera writes it), since the tag goes with
  the rest of the metadata below and a photo stripped of it unturned
  would arrive sideways;
- then crops the largest centred square and scales it to exactly
  256 × 256 pixels, HUMAN-CHAT.md's bound, which covers the photo's
  largest use, the 96 dp circle on a contact page and in this editor,
  at common densities (252 px on a 420 dpi phone, 192 px on a desktop
  at 2×; a denser screen scales it up a little); every smaller use (the
  40 dp list avatar, the 36 dp app bar) is drawn from it on the
  receiving device. A picture smaller than 256 px a side is sent at its
  own size, never enlarged. (The owner, 2026-10-21: "my advice for the
  contact image is to resize exactly to the use we do before sending
  it over transport base64 encoded", read as one fixed size and
  confirmed; until then this scaled to at most 256 and stepped down to
  192 and 128 px.) It lays any transparency over plain white and
  encodes it as JPEG, always, with the encoder's chroma subsampling, at
  the highest quality from 90 down to 20 that fits 24,576 bytes
  (HUMAN-CHAT.md). Subsampling is not a rule of this design: every
  decoder reads every subsampling, so the choice is the encoder's.
  Today the image crate's encoder uses 4:4:4, the only subsampling it
  offers; an encoder that offers 4:2:0, if one is adopted later, would
  use 4:2:0 (architect-cto's DECISION 01a173ac, 2026-10-25). At 4:4:4,
  with the image crate 0.25.10 and this preparation, rust-ui-dev-02
  measured seven photographs (from 850 × 772 to 3024 × 4032 pixels
  before preparation) on 2026-10-25: the preparation chose quality 85
  to 90 for them, and the prepared photographs were 13,123 to 24,555
  bytes (recorded in `crates/human/app-core/src/card_photo.rs` at
  4df2fa26, which #449 brought to main;
  `card_photo.rs` names neither the seven photographs nor the generator
  of the noise square that it also measures, below). So with
  today's encoder a camera photograph fits far above quality 50. An
  earlier run at 4:2:0, which recorded neither the photographs nor the
  encoder, found that all 15 photographs at 256 px fit at quality 50,
  and that the largest of them was 17,405 bytes. The lower qualities
  are for a picture of fine texture or noise, which at 256 px can
  exceed the bound at quality 50. The noise is a 256 × 256 RGB square
  whose every channel byte is random. Measured at 4:2:0 with
  libjpeg-turbo through Pillow, with Huffman optimisation off, the
  square was 27,478 bytes at quality 50, 23,260 at 40 and 13,620 at 20;
  #447's review reproduced this measurement byte for byte, with the
  square's bytes from Python's `random.seed(1)`. Measured at 4:4:4 with
  the image crate 0.25.10, with the square's bytes from xorshift64
  (shifts 13, 7 and 17, seed 0x9E3779B97F4A7C15, the state stepped
  before each byte, each byte taken from bits 24 to 31 of the state,
  in row-major RGB order), the square was 56,257 bytes at quality 50,
  47,920 at 40, 24,967 at 22, 23,340 at 21 and 21,626 at 20, so at
  4:4:4 such a picture fits only at quality 21 or below. In
  rust-ui-dev-02's record in `card_photo.rs`, also at 4:4:4 with the
  image crate, the preparation chooses quality 21 for their own noise
  square too (55,940 bytes at quality 50, 47,781 at 40 and
  21,616 at 20). No record says that all the squares come from one generator, so
  these sets of figures cannot be read as one picture measured at two
  subsamplings. So the picture keeps its pixel size (256 × 256, or
  its own smaller square), and the quality is what goes down. An image
  that still does not fit at 20 is unusable (below). JPEG always,
  because a lossless PNG of a photograph seldom fits and has no
  quality to lower.
  The bytes go into the card as HUMAN-CHAT.md's base64, which the
  client never re-encodes in transit;
- the preview shows the prepared bytes, decoded, not the original, so
  the person sees exactly what a counterparty will see;
- the prepared image carries nothing but the picture: no EXIF, no
  location, no camera data, no thumbnail, no file name: the sender
  strips every metadata block before the photo enters the card
  (architect-cto's answer to §12 A3, a rule of HUMAN-CHAT.md §Contact
  card from their next records PR);
- an image that cannot be read or prepared leaves the card as it was
  and shows `PhotoUnusable` under the circle.

Preparing can take a moment on a phone: the circle shows a progress
indicator, and Save waits for the preparation to end, unless Remove
photo cancels the preparation (below). Choosing the crop is deliberately left
out (§11).

**Focus and speech in the photo step** (rust-ui-dev-02's QUESTION
01a17307, 2026-10-25), on both platforms:

- When the Photo Picker or the file dialog closes, whether a picture
  was chosen or not, focus is on the button that opened it (Add a
  photo, or Change photo). A cancel changes nothing and announces
  nothing.
- While a picture is being prepared, the circle's accessible label is
  `CardPhotoPreparing`, and the progress indicator inside it is not
  a separate stop. The start is not announced: the person has just
  chosen, and focus is on the button. The buttons stay as they were
  before the choice and stay usable; a picture chosen while another
  is being prepared replaces the one being prepared, and only the
  last one chosen is shown. During the preparation, Remove photo is
  offered whenever the button was offered before the person chose the
  picture, that is, whenever the circle showed a photo before that
  choice, whether that photo was saved or not. (During the
  preparation, the circle shows the progress indicator instead.)
  Remove photo cancels the preparation: the picture being prepared
  is dropped, nothing is shown or announced when it would have been
  ready, and Save does not wait for it; then Remove photo does what
  the item **Remove photo in this editor** below says.
- When the photo is prepared, the circle shows the photo, and the
  circle's label is `CardPhotoLabel`; focus stays on the same button, which now reads
  Change photo, with Remove photo after it. `PhotoAdded` is announced
  once through the live region, because the result changes a part of
  the page that does not have focus (WCAG 2.2 SC 4.1.3, status messages).
- When the picture cannot be used, `PhotoUnusable` shows under the
  circle and is announced once through the live region; focus stays
  on the button, so the person can choose another at once. The line
  is read after the circle in the reading order and is not a stop of
  its own. It goes when another picture is chosen or the photo is
  removed.
- **Remove photo in this editor is an unsaved change**, like a chosen
  photo: nothing is written until Save, so there is no snackbar
  and no Undo. To undo the removal, the person uses the `CardDiscardChanges` dialog
  that Back opens, or chooses a photo again. The circle shows the initials, its label
  becomes `CardPhotoNone`, and focus goes to Add a photo, which
  replaces Change photo and Remove photo; nothing is announced,
  since the button that takes focus says there is no photo. The
  contact page's Remove photo (§6.3) is different: that page has no
  Save, so Remove photo there removes the photo at once, with an
  Undo.
- Whether the editor holds unsaved changes is judged against the saved
  card: a photo added and then removed before Save, on a card saved
  with no photo, is no change, and Back closes the editor without the
  `CardDiscardChanges` dialog.

### 5.3 Share a card

Entry: **Share a card** in a direct conversation's overflow, or the
offer in an empty conversation (below).

There is nothing to choose: the card sent is the card of the identity
the conversation speaks as (ADR-0059 d. 2 (ii), ADR-0061 d. 3), which is
the identity in front (§5.7). But a disclosure has no undo, so Share
first shows what goes. The share sheet is one component: a modal bottom
sheet on a narrow layout (Material's, with a drag handle and a scrim;
Back and a tap on the scrim close it), a dialog on a wide one (Escape
closes it). It holds:

- title `ShareCardTitle`, with the conversation's title in it ("Share
  your card with {title}"), so the person sees who gets it;
- with two or more identities held, the line `ShareCardAs` ("as
  {identity}"), the identity's avatar beside it, so the person sees which
  identity's card this is before it goes;
- **What {title} will see** (`ShareCardPreview`): the card's photo and
  name and family name, as the counterparty's client will show them
  after they accept;
- the line `ShareCardNote`: the counterparty will see this name and
  photo, and can save them; a card cannot be removed from their device once it arrives;
- actions: **Cancel** (`Cancel`), **Edit your card** (`EditCard`, a text
  button, which opens §5.2 for this identity and returns to the sheet)
  and **Share** (`ShareCardAction`), filled, at the end.

**No name yet** (the identity's card has none): the preview is replaced
by `ShareCardNoName` and the primary becomes **Add your name**
(`CreateCard`), which opens §5.2 for this identity. Its Save returns to
the sheet with the preview filled; its Back returns to the sheet
unchanged.

**Too large for this profile.** When the card with its photo is
over the profile's effective payload limit (HUMAN-CHAT.md: the client
offers to send it without the photo), the sheet knows it before any
send: the preview shows the initials instead of the photo, the line
`ShareCardNoPhoto` says the photo cannot be sent in this conversation,
and the primary reads **Share without the photo**
(`ShareCardWithoutPhoto`). If even that does not fit (only a profile
limit below a name's size could do this), the primary is not offered and
`ShareCardTooLarge` says so; Cancel stays.

**After Share.** The sheet closes, focus returns to the control that
opened it (the overflow button, or the offer's Share a card button,
which the new row replaces, so then the conversation's header title),
never to the composer, which would open the keyboard unasked
(message-swipe §7's reason); and the card appears
at the end of the conversation as **one's own card row** (§5.5) with
the status of any outbound message: Sending, then Accepted by remote
transport, or Not sent with Retry and Cancel (`human-client-ui.md` §5,
§6). It leaves from the conversation's identity's lease, as everything
the conversation sends does (ADR-0061 d. 3). It is pending outbound
under RETENTION.md like a text message: it
survives a restart until terminal, and Cancel with its Undo behaves as
for text (§6's cancel rule, the same record restored). The
conversation's `card_shared_at` is set when the card's row reaches
Accepted by remote transport — the strongest evidence the client has
that the counterparty's client holds it (`human-client-ui.md` §3: "that
the card was shared and when"; §5: never "read"). A card that is
cancelled or never accepted leaves the previous value; Details follows
the stored value only. No
snackbar: the row is the confirmation, and there is no undo of a
disclosure, which is why the sheet shows the preview first.

**The offer in an empty conversation.** While a direct conversation
holds no message, the message area shows,
centred where the empty list is: `ShareCardOffer` (let {title} see your
name instead of a PeerId) and a text button **Share a card**, which opens
the share sheet. It disappears with the first item either way, a shared
card's row included. It is never a dialog, sends nothing, and stores
no "dismissed" state: there is nothing to dismiss.

### 5.4 A received card: the proposal

A received `contact_card` is a row of its own in the conversation, at
its place in time, distinct from a text bubble so that the peer's claim
never looks like a message the app wrote:

- full width of the message column (not a bubble), on
  `surface-container`, 12 dp corners, a 1 dp `outline-variant` border
  (decoration only; the heading says what it is in words);
- a 40 dp neutral avatar: a person glyph on `surface-variant`, the same
  for every proposal, never initials or a colour derived from the claim;
- the heading `CardProposal`: "This peer says they are {name}", the name
  in title medium, isolated (§3), wrapping to at most three lines, then
  ellipsised; the whole name is in the row's accessible description and
  on Details;
- `CardIncludesPhoto` when the card carries one that Accept would
  write: it includes a photo, shown after you accept. It is not shown
  when the contact's photo is a local choice, or the contact has no
  photo by a local choice (§5.4.1), since the person never sees the
  card's photo; the kept line (`CardKeepsPhoto` or `CardKeepsNoPhoto`)
  says so instead;
- **PeerId** (a label of its own, `CardPeerIdLabel`, beside the app's
  existing `OwnPeerId`) and the
  full PeerId under it in the code font, wrapping, selectable, with
  **Copy PeerId** (`CopyPeerId`);
- the **context line**, exactly one of (§5.4.1);
- `CardGoesAtClose`, in body small: if you do not accept it, it is removed
  when InterWeave stops;
- actions at the end: **Decline** (`Decline`, text button) and **Accept**
  (`Accept`, filled; its label changes by case, §5.4.1), **stacked**:
  each on a line of its own, the full width of the content column,
  Decline above Accept, at every width and every text scale; in case
  S, only Close is shown. A label that does not fit wraps inside its
  button, which grows in height and is never under 48 dp. The buttons are
  stacked at every scale rather than only when they do not fit side by
  side, because the toolkit (Slint 1.18) cannot choose a layout from the
  width of a label's text (rust-ui-dev-02's finding while building the row,
  QUESTION 01a170cb, 2026-10-25), and a translated case label ("Accept
  as a new contact") may not fit beside Decline even at 100 %; one
  layout everywhere also keeps the reading and focus order the same in
  every locale.

The row carries the time and the status words of any received item
(Unread, Read) under `human-client-ui.md` §5–§6. It has no Keep: Accept
is how a card is saved, and saving an unanswered claim would add a state
the person cannot use (architect-cto confirmed, §12 A2).

#### 5.4.1 What Accept will do, said before it is done

The client compares the card with the store and shows one context line
and one Accept label.

**Same name.** What is compared is the name the card would give the
contact — its `name`, then a space and its `surname` when it has one
(ADR-0059 d. 3's `display_name`) — against each contact's
`display_name`. Three keys are made from each name. Here `skeleton` is
the confusable skeleton of Unicode TS #39 (TS #39's §4), "fold" is full
case folding, and "spaces" means every run of whitespace (any `Zs`
character, a no-break space included, and tabs) collapsed to one space
and trimmed:

- the **case key**: NFKC, then spaces, then fold, then `skeleton`;
- the **shape key**: spaces, then `skeleton`, then fold, then
  `skeleton` again;
- the **compatibility shape key**: NFKC, then the shape key's steps.

Two names are the same when their case keys are equal, **or** their
shape keys are equal, **or** their compatibility shape keys are equal.
Each key is compared only with the same key of the other name. The keys decide only whether the warning shows
and whether titles carry the PeerId (§6.1); they are never displayed
and never stored in place of the name. The reason is the threat
ADR-0059 makes visible: a peer naming itself "Аnna" with a Cyrillic А,
"Anna Rossi" with a no-break space, "Ａnna" in fullwidth letters,
"Βob" with a Greek capital Beta, "ꓮda" with a Lisu letter or "Alſie"
with a long s — each allowed by HUMAN-CHAT.md's name rules — must meet
the same warning as "Anna", "Anna Rossi", "Bob", "Ada" or "Alfie".

Why these keys. TS #39's prototypes are case-sensitive: a capital
lookalike maps to a capital Latin letter (Greek "Β" and Lisu "ꓮ" to
"B" and "A"). Folding first turns such a letter into one that no
longer looks Latin (Greek "Β" folds to "β"), or leaves the letter as it
is, so that its prototype is still a capital (Lisu "ꓮ" has no case).
So the case key misses them; the shape key reads the name as written
and catches them. The shape key alone would miss what the case key
catches: "Ian" and "ian" differ only in case, yet their shape keys
differ, because a capital I looks like a small l and a small i does
not. A match on any one key is enough for the warning, because a missed
lookalike lets a peer pass as a contact, while an extra warning costs
the person one comparison of PeerIds. The rule warns on every pair
that the case key alone warns on. There are two shape keys, one without
NFKC and one with it, because each catches lookalikes that the other
misses.
NFKC rewrites some lookalikes before `skeleton` sees them: long "ſ"
becomes "s", so "Alſie" would be compared as "Alsie"; the shape key
reads "Alſie" as written, and `skeleton` maps "ſ" to "f". Other characters look Latin only after NFKC: a fullwidth digit zero
becomes "0", whose skeleton is "O", so only the compatibility shape
key joins "J０hn" and "John". The rule is not transitive: "Ian" is
the same name as "ian" and as "lan", but "ian" and "lan" are not the
same name. Nothing here needs the rule to be transitive, because the
warning and
§6.1's titles compare one pair of names at a time.

The cost of the shape keys is a few false warnings. Each shape key
folds after `skeleton`, so each joins a capital I, which `skeleton` turns into a
small l, with a capital L, which the fold turns into a small l. "Ian"
and "Lan" are the same name
by this rule though they do not look alike. Such a warning costs the
person one comparison of PeerIds, and the contact is still saved on
Accept.

Evidence (`confusables.txt` of Unicode 16.0.0, SHA-256
95bd0aad6dced5ebc63436f459c06ab21a8d107cd842fb57f5c3a1e91bca8611; the
script and its name list are in PR #438's comment
https://github.com/BlueTeam-OU/InterWeave/pull/438#issuecomment-6395097730,
and the script prints every figure below). A pair is a
source character of the file against one form of its prototype. A
source character counts when its prototype has at least one letter
and is made only of code points that are below U+0250 or are
combining marks. Its forms are the distinct ones among four: the
prototype as listed, in lower case, in upper case, and with its first
letter capital. That gives 3,765 pairs. The case key
alone missed 949 of them. The first fix proposed for this gap
(rust-ui-dev-02's observation), fold, then `skeleton`, then fold,
missed 435, among them "ꓟax" against "Max" (a small m's skeleton is
"rn") and "Βob" against "Bob". The rule above misses 17, each a pair
that `skeleton` itself does not join (such as "ȧ" and "å"). Without
the compatibility shape key, the rule would also miss 45 pairs of a
compatibility character against a Latin form, found by comparing every
code point that NFKC changes with the Latin forms of that code point's
compatibility shape key. In 15 of them the character's NFKC form looks
like the Latin form with its case as written (the fullwidth digit zero
against "O", a circled number twenty against "2O"); the other 30 join
only after the fold, and some of the 30 are false warnings of the I
and L kind above (a circled capital I against "L"). Among 106
common given names, the rule joins only one pair, "Ian" and "Lan", the
false warning above.

| case | context line | Accept label | Accept does |
|---|---|---|---|
| **P** the route is under no contact; another route of the same PeerId is under contact X | `CardSharesDevice`: "This card comes from the same device as your contact {X}." (or `CardSharesDeviceMany`, naming X and the number of other contacts on that PeerId) and, under it, `CardSharesDeviceNote`: a device can hold several identities; you can save this one as a contact of its own | **Accept as a new contact** (`AcceptAsNew`) | creates a contact {name [surname]}, its photo, and the route; X is unchanged, and never gains a second route of one PeerId (ADR-0059 d. 4 (ii), ADR-0061 d. 4) |
| **N** the route is under no contact; no route of its PeerId is under one; no contact has this name | none | **Accept** (`Accept`) | creates a contact {name [surname]}, its photo, and the route |
| **D** the route is under no contact; no route of its PeerId is under one; a contact on another PeerId has the same name | `CardSameName`, in `error`, with a warning icon: "One of your contacts is named {name} and has a different PeerId. Compare the two PeerIds before you accept.", `{name}` being the existing contact's name as stored, so the person sees both spellings; and **Open your contact {name}** (`ShowContact`), which opens that contact's page | **Accept as a new contact** (`AcceptAsNew`) | creates a separate contact; never merges (ADR-0059 d. 4) |
| **U** the route is under contact X; the card differs from X's values and neither is a local edit | `CardUpdates`: "If you accept, your contact {X} is updated." with the values that change named (`CardUpdatesName`, `CardUpdatesPhoto`, `CardUpdatesPhotoRemoved`) | **Update contact** (`UpdateContact`) | writes the card's values to X |
| **E** as U, but a value that differs is a local edit | `CardKeepsEdit`: "You named this contact {X} yourself. That name stays." (and/or a photo line: `CardKeepsPhoto` when X holds a photo the person chose, `CardKeepsNoPhoto` when the card carries a photo and the person removed X's photo), and, for each value that Accept writes, that value's change line as in case U (`CardUpdatesName`, `CardUpdatesPhoto`, `CardUpdatesPhotoRemoved`) | **Update contact** if any value would change, else see S | writes only the values not locally edited |
| **S** the route is under X and the card changes nothing | `CardSame`: "This card matches your contact {X}." when the card's name is X's name, character for character (not by the same-name rule), and neither the card nor X has a photo; otherwise, when the card differs only in values that are local choices, the kept lines of case E in its place (`CardKeepsEdit`, `CardKeepsPhoto`, `CardKeepsNoPhoto`), never `CardSame` | **Close** (`Close`) in place of Accept; Decline is not shown | removes the row from the session, as Decline does, without a snackbar |

Every contact compared here is a contact of the identity the
conversation speaks as, which is the identity in front; another
identity's contacts are never read for a case, a warning or a link
(ADR-0061 d. 9), and the same counterparty saved under two identities
is two contacts (d. 4).

The cases are tested in this order: the route under X (S, E, U), then P,
then D, then N. E applies per value (name, photo). A card whose route
is already under X updates X rather than creating a contact, since X is
that identity of that peer; but X being that identity says nothing about
whether it is naming itself as someone else. So **whenever Accept would
write a name** — in case P, and in cases U and E when the name changes
and is not a local edit — and that name is the same name, by the rule
above, as the name of a contact on another PeerId, `CardSameName` is shown too, in `error`, above the
case's own line, naming that contact, with its Open your contact link
(ADR-0059 Security implications: a second contact with an existing name
is shown as such before acceptance). The Accept label stays the case's.
A contact on the same PeerId with the same name is not a lookalike
warning: it is the same device, which case P's line already says, and
their titles are told apart by the route label (§6.1). A photo is a
local choice when the store records that the person, not a card,
decided it on this device (Remove photo, §6.3, records one). Every
other photo is not a local choice, including one whose source the
store did not record. The absence of a photo is a local choice too
when the person made it: Remove photo leaves the contact with no photo
and records that the person chose to have no photo. When the card
carries a photo, and X's photo is a local choice or X has no photo by
a local choice, the card's photo is not used: X keeps the
photo the person chose (`CardKeepsPhoto`) or stays without one
(`CardKeepsNoPhoto`), and `CardIncludesPhoto` is not shown (§5.4).
When the card carries no photo and X holds a photo the person chose,
X keeps that photo in this case too, and the proposal says so with
`CardKeepsPhoto`, so that a case S row shows a line that is true.
(When the card has no photo and X has no photo by a local choice, the
card differs in nothing, and no photo line shows.) `CardKeepsPhoto` is never shown for a contact that has no photo,
since it says that the contact has one (rust-ui-dev-02's finding,
OBSERVATION 01a17304, 2026-10-25). A photo is a
"value that changes" whenever the card carries one and X's photo is not
a local choice, since the client cannot compare a photo it has not
decoded (`CardUpdatesPhoto`). It is one too when the card carries no
photo and X holds a photo that is not a local choice: Accept writes the
card's photo or its absence, so X's photo is removed, and the proposal says so before Accept, with `CardUpdatesPhotoRemoved`.
The later card has no photo, and the person sees a removal before
Accept, as they see a new photo (rust-ui-dev-02's finding on
#433, QUESTION 01a171a0, 2026-10-25). If the card has no photo, and X has no photo or has a photo that the
person chose, no photo changes.

#### 5.4.2 Accept

Runs at once: the contact (and route) is written, and the photo is
decoded only now, with the memory-safe decoder, to the fixed display
size. The proposal row is replaced in place by a short confirmation row
(`CardAccepted`: "You saved {name} as a contact." in cases N, D and P;
`CardUpdated` in cases U and E; each with the name as saved and the new
avatar), so the conversation holds a trace of what
happened in this session. The title and the list take the name (§6.1).

Snackbar with Undo: `ConfirmContactSaved` in cases N, D and P,
`ConfirmContactUpdated` in U and E. Undo is the reverse: a created
contact is deleted with its route; an updated one gets back the values
it had. The proposal row returns,
read, as it was. A kill inside the window leaves the contact saved,
which is what the snackbar said.

Accept is not announced through the live region (§8 Announcements):
focus moves to the confirmation row, whose text is the result. So that
a person who hears the row can still find the Undo, these hold
while the Accept's snackbar is open:

- the confirmation row's accessible description is `CardUndoHint`,
  read after the row's text; it goes when the snackbar closes;
- the snackbar's timeout does not run while focus is anywhere from
  the confirmation row to the snackbar in the reading order (the row,
  any message or control after it, the composer, the snackbar), so the
  snackbar does not time out while the person moves focus from the row
  to the Undo that the hint names (within the cap below); it starts, in full (message-swipe §6's
  6 s, or the platform's longer recommendation), when focus leaves that
  span: to something before the row, or out of the conversation's
  content; and it stops again if focus comes back into the span;
- whatever focus does, the snackbar is never shown for more than
  message-swipe §6's 2 minutes, counted from Accept, and then closes
  as by its timeout; so a person who accepts with a pointer or a touch
  and moves no focus sees it close by then;
- its other closers (message-swipe §5) are unchanged.

A photo that fails to decode, or decodes larger than 256 × 256, is
discarded and the contact is saved without it (HUMAN-CHAT.md);
`CardPhotoNotUsed` is added to the confirmation row. The accept is not
refused for a photo. In cases U and E, when X held a photo that was
not a local choice, that photo is removed too, because the card's unusable photo is saved
as no photo. The row then says `CardPhotoNotUsedRemoved`
in place of `CardPhotoNotUsed`, and the snackbar's Undo gives the old
photo back, as it gives back every other value.

A store refusal (degraded store): nothing is written, the proposal stays,
and the app's existing `StorageDegraded` notice explains; Accept stays
available for a retry.

#### 5.4.3 Decline

Removes the proposal from the session at once and writes nothing (its
durable unread copy went at read, as for any item). Snackbar
`ConfirmCardDeclined` with Undo; Undo returns the row to the session.
Once the window closes it does not come back; the peer can send the card
again. A kill inside the window leaves it gone, as said.

#### 5.4.4 Gestures

The proposal row takes the owner's swipe (§1): **swipe right (start→end)
= Accept** with the label the case gives it, **swipe left = Decline**,
with the message-swipe spec's region, thresholds, reveal, haptics, RTL
mirroring, reduced-motion rules and Undo snackbar (its §3–§8), as a new
row kind E there. The reveal's label is the button's label of the case,
never a bare "Accept". **Whenever the row shows `CardSameName`, the
right swipe is withheld.** It is withheld in case D, and in cases P, U and E
when Accept would write a name that clashes with a contact on another
PeerId (§5.4.1). These are the owner's answers: §12 O1 for case D, and
O2 for cases P, U and E. The row does not move to the right and shows
no accept reveal. On such a row, only a deliberate choice of
Accept with its case label saves the card: the button, the menu's
Accept item (below) or the screen reader's custom action (§8), never a
swipe. This is the owner's answer, §12 O3. The reason is that a swipe can happen without reading, while
each of the three names what Accept will do before it is chosen. If an
accidental swipe saves a stranger under an existing contact's name, or
renames a contact to that name, that is the impersonation ADR-0059
makes visible, and Undo helps only a person who noticed. Whether the
swipe is withheld is decided by what the row shows when the drag
starts, after any recomputation of §5.4.5. So a row never accepts by
swipe while it shows the warning, and never withholds a swipe that
starts without the warning. If the warning appears during a right drag, the drag is
cancelled: the row returns to rest as it does after a release below
the threshold, and nothing is accepted. If the warning disappears during a
right drag that started withheld, the row stays still until the
release, which accepts nothing. A left drag is not affected when the
warning appears or disappears, since Decline is the same with or
without it. The left
swipe still declines. In case S neither direction moves (Close is not a
swipe action). The buttons are
the non-gesture equivalent, always present.

**A difference from message-swipe §7, on purpose.** There, as revised by
Android review F2, a row at rest shows no buttons and puts its actions
in a menu. A proposal shows Accept and Decline at rest, because
it exists to be answered: a claim whose answer is hidden in a menu reads
as a message, and the case's label (§5.4.1) is what tells the person what
Accept will do before they do it. The row also has §7's menu.
The menu holds the actions that the row shows, with the case's labels:
Accept and Decline, or in case S, only Close. Copy PeerId comes after
them. A row that shows `CardSameName` has the same menu, because only
the right swipe is withheld there. The menu has no Show source, because
a card has no markdown source to show. The menu opens with a long press
on the row, on the desktop by a secondary click on the row, or by §7's
Menu key or Shift+F10 while focus is on any control that the row has
(Copy PeerId, Open your contact, Decline, Accept or Close). It does not open with Enter: unlike a message row, whose bubble is its focus stop, the
proposal has no focus stop of its own, and Enter on each of its
controls is that control's action. A focus stop for the
card would add one more focus stop to every proposal, and that stop would
only repeat what the visible buttons already do (rust-ui-dev-02's finding while building the menu,
QUESTION 01a1718c, 2026-10-25). For the same reason, no accessible
default action opens the menu: each of the row's controls keeps its own default action, and
a screen reader reaches the menu's actions as the row's custom actions
(§8). Once open, the menu behaves as §7's and §4's (focus on its first
item; Escape, Back or a touch outside closes it; a press released on
the item it started on chooses it), with three differences that follow
from the missing bubble and the warning:

- when the menu closes with nothing chosen, focus goes to one of two
  places. If a key opened the menu, focus goes back to the control
  where that key was pressed. If a key did not open it (a long press or
  a secondary click did), focus goes to the row's first control, Copy
  PeerId;
- when a chosen item leaves the row in place (Copy PeerId, or an Accept
  that the store refuses, §5.4.2), focus goes to the same place as when
  the menu closes with nothing chosen;
- on a row showing `CardSameName`, a press-drag-release onto Accept
  chooses nothing (message-swipe §4's release that ends the long press
  already chooses nothing on every row). On that row, Accept is chosen
  only by a press that starts and ends on Accept after the menu has
  opened (a tap, Enter or Space). Onto any other item of that row's
  menu, and onto any item on every other row, a press-drag-release
  chooses the item the finger ends on. The menu is anchored to the
  card, as message-swipe §7 anchors a message's menu to its bubble. It
  opens under the card when it fits in the visible area there; else
  above the card when it fits there; else over the card, at the top of
  the visible area (when neither the space below the card nor the
  space above it holds the menu, as with a tall card at a large text
  size). The visible area is the
  window inside the system bars; the menu may cover the app bar or the
  composer. The menu's place depends on the card alone, never on
  where the press was made. So a
  long press that slides from the card onto the menu can end on
  Accept, the menu's first item, before the person has read the card. Without
  this rule, a warned card could be saved that way unread. This rule
  is the designer's reading of §12 O3's reason (a deliberate choice of
  Accept, never one that can happen without reading), not a separate
  answer from the owner.

§9's focus table holds each of these.

**Two differences from message-swipe §6 and §7 for Accept, on
purpose.** Accept's result is not announced through the live region,
as §7's Announcement would have it, because focus moves to the
confirmation row and reads the result there (§8 Announcements); and its
snackbar's timeout waits while focus is between that row and the
snackbar, for at most message-swipe §6's 2 minutes, with the row's description pointing to the Undo (§5.4.2), so
the person can still reach the Undo, though the snackbar is not announced.
Decline and every Undo (with "Action undone", message-swipe §9) are
announced as §7 says.

#### 5.4.5 Several proposals, and a proposal while away

- Each card is its own row. A later card from the same route does not
  replace an earlier one (d. 3); each row's context line is computed
  when it is shown, and again after **any action or Undo that changes
  which contact holds a route or what a contact's values are** — an
  Accept and its Undo, Details' Add as a contact, a Move and its Undo,
  a Delete and its Undo, a name edit, Remove photo — so an older card under a just-accepted newer one reads as
  case U, E or S against the new values, and a row never shows a label
  whose case no longer holds.
- A card that arrives while the conversation is not shown is unread
  content: it counts in the unread badge and survives a restart as any
  unread item does.
- **List preview** of a conversation whose last item is a card:
  `CardPreviewReceived` ("Contact card") or, for one's own,
  `CardPreviewSent` ("You shared a card"). Never the claimed name: the
  list has no PeerId beside it to frame the claim.
- **Notification** (`human-client-ui.md` §10): the conversation's title
  (the short PeerId until Accept) and `CardNotification` ("You received
  a contact card."), never the claimed name or photo, for the same
  reason, and the identity it arrived on (§5.7.5). A tap opens the
  conversation, under that identity.

### 5.5 One's own card row

An outbound card is shown as a row at the end side in the sent bubble's
colour (`primary-container`), holding the card as sent (avatar or photo,
name and surname) and the line `CardSentLabel`: "You shared your card",
then the status and time as for any sent message. The card shown is the
one that was sent, not the identity's card as it is now: an edit after
the share does not rewrite the row (`CardSharedStays`). Its actions are those of a sent
message in its state (Retry, Cancel), with the same swipes (kind C).

### 5.6 Naming a contact by hand

The person can also type a name (`human-client-ui.md` §3). Details
(§6.4) of a conversation whose route is under no contact of its
identity offers **Add as a contact** (`AddAsContact`), which opens the
contact page's name field empty, for that route; Save creates the
contact under the conversation's identity. The name is a local edit
from the start (it is never overwritten by a card). When another route
of the same PeerId is under a contact X, Details says so above the
action (`DetailsSharesDevice`, as case P's line, §5.4.1), and Add as a
contact still makes a contact of its own: a contact holds at most one
route per PeerId (ADR-0061 d. 4), so there is no "add to X".

### 5.7 The identity in front, and switching

ADR-0061 decision 9 and the owner's rule (§1): the app operates as one
identity at a time and says which, everywhere the person can act. The
failure this section exists to prevent is one the person does not
notice: a message, a card, an Accept or a deletion done as one identity
while they believe they are the other. So the design rests on three
things, each checkable: what is shown is only the identity in front's
(no screen mixes two); every place that acts names it in words, not only
by a face or a colour; and it changes only on one act of the person's
that is named as such and confirmed by what follows.

Everything here applies while **two or more identities** are held. With
one, nothing in this section is shown: no switch, no "as {identity}"
lines, no question when a conversation starts (ADR-0061 d. 3); the
identity's card is reached as Your card (§4).

#### 5.7.1 The switch

On the Chats screen's top app bar, between the title and the ⋮ (§4):
the **identity switch**, a control showing the identity in front's
24 dp avatar, its name (the card's name, else its route label), and a
▾. It is the only control whose purpose is to change the identity in
front; §5.7.3 lists the other acts of the person's that change it as a
consequence. Its accessible name is `IdentitySwitch` ("Identity: {identity}.
Switch identity"); its target is at least 48 dp high; its name takes
what width is left after "Chats" and the ⋮ and ellipsises at the end,
never below the avatar and three graphemes, the full name in the
accessible name. On the desktop it is in the list pane's bar, where the
same menu is.

When another identity has unread items or a message that was not sent,
the switch carries a badge: a dot with the count of unread items in the
other identities (`IdentityOthersUnread`, read as part of its accessible
name). With nothing unread there but a message not sent, the dot has no
count and the accessible name carries `IdentityOthersNotSent`; with
both, the count, and the name carries both strings. It counts and shows
nothing else — no name, no preview — so the person knows to look
without the screen showing another identity's content. (A suggestion
resting on visibility of system status, Nielsen's first heuristic: a
person in front of one identity must not lose messages to the other;
falsified if people in a test switch to see why the badge is there and
find it says too little.)

Activating it opens the **switcher**: a modal bottom sheet on a narrow
layout, a menu-anchored dialog on a wide one; Back, Escape and a tap on
the scrim close it with nothing changed. It holds:

- title `IdentitySwitchTitle` ("Switch identity");
- the identities as single-choice rows (56 dp): avatar, name, route
  label, and at the end the row's unread count and, when any message of
  that identity was not sent, `IdentityNotSent`; the identity in front
  is the chosen row;
- at the bottom, **Your identities** (`IdentitiesTitle`, a text
  button), which opens §5.1 without switching.

Choosing another row switches at once (§5.7.3) and closes the sheet;
choosing the row already chosen closes it with nothing changed.

#### 5.7.2 Where the identity in front is named

| screen | what it can do | how it names the identity |
|---|---|---|
| Chats | open, start or delete a conversation | the switch (§5.7.1) |
| a conversation | send, share a card, accept or decline a card, delete messages | `ComposerAs` ("As {identity}"), one line in body small at the end side directly above the send control, with the 24 dp avatar; the send control's accessible name is `SendAs` ("Send as {identity}"); Details' `DetailsIdentity` (§6.4). Where the conversation has no composer, the line stands where the composer would |
| the share sheet | share | `ShareCardAs` (§5.3) |
| Contacts, a contact | delete, move, rename, remove a photo | the identity as the app bar's subtitle (§6.2, §6.3) |
| an identity's card | save, make default, remove | that identity as the subtitle (§5.2); when it is not the identity in front, `IdentityNotInFront` under the app bar ("You are using {front}. What you change on this page applies to {identity}.") |
| Trust | trust, untrust | not an identity: trust is the device's, held for every identity (ADR-0061 d. 1); `TrustAllIdentities` says so under the title, so no one trusts a peer for "this identity only" by mistake |
| recovery, discard (§7) | delete stored data | not an identity: the dialog says it is every identity's data (§7.2) |

The "As {identity}" line is text, not a control: a conversation never
changes identity (ADR-0061 d. 3), and a switch reached from inside one
would read as changing who speaks in it. It is always the identity in
front, because only its conversations are shown.

#### 5.7.3 Switching

**What changes the identity in front**, all of it and nothing else,
each an act of the person's and each named to them right after:

1. choosing another identity in the switcher (§5.7.1);
2. **Switch back** on the snackbar a switch shows (below);
3. opening a notification of another identity (§5.7.5), named by the
   transition before anything is shown;
4. removing the identity in front (§5.2), named by Remove's snackbar
   while two or more identities remain; with one left there is no
   identity in front to name (ADR-0061 d. 3), and the snackbar names
   the removed one;
5. at the app's start, the default comes in front only when the
   identity last in front is gone (below), and the switch names it.

Starting the app otherwise, opening a contact, Your identities or an
identity's page, adding an identity and Make default change nothing.
A switch, by any of these, changes everything shown together, at once: the Chats list becomes the
new identity's conversations, the switch shows its name, and on a wide
layout, where the switch stays on screen beside the second pane (§4),
whatever that pane holds of one identity closes — a conversation,
Contacts, a contact page, an identity's page — and the pane shows the
new identity's empty-pane placeholder. Two pages there belong to no
identity and stay: Trust (the device's) and Your identities, whose check
mark moves. On a narrow layout those pages cover Chats, so the switcher
is reached only from Chats; a sheet or a dialog covers the switch on
either layout. **An unsaved card edit** (an identity's page with changes,
§5.2) is asked about before any switch: `CardDiscardChanges` opens
first; Close without saving lets the switch go on, and Continue editing cancels it,
leaving everything as it was — for a notification's switch too, whose
conversation the person then finds in Chats once they switch. What
the person had in hand stays with its
identity: a draft stays in its conversation's composer, sends in
progress go on under their own lease, and nothing of the old identity's
is carried into the new one's screens.

A snackbar shown before the switch closes with it: its Undo belonged to
a screen of the old identity, and what it confirmed stays done, as a
kill inside its window would leave it.

Feedback: the list cross-fades (under reduced motion it changes at
once); the snackbar `ConfirmIdentitySwitched` ("You are now
using {identity}") with **Switch back** (`SwitchBack`), which switches to the
previous identity by the same rule; its text is announced. Focus goes to
the switch, which now names the new identity. A switch is not a
destructive act, so it is not confirmed beforehand; its result is named
right after, and undone by the same act.

**When the app starts**, the identity in front is the one that was in
front when it last closed, named on the switch as always; if that
identity is gone or the setting cannot be read, the default identity.
Where that setting is kept is rust-ui-dev's (it holds no content).

**When the identities change**: a second identity added shows the switch
and the "as" lines, and switches nothing; going back to one removes
them. Removing the identity in front (§5.2) brings the default in front,
with Remove's own snackbar and focus (§5.2), not this section's.

#### 5.7.4 Starting a conversation, and what never changes identity

A conversation started by the person is the identity in front's
(ADR-0061 d. 9), and a conversation started by a peer is the identity
whose endpoint it reached. Neither ever changes identity, and nothing
moves a conversation, a contact or a card from one identity to another.
The new-conversation flow is plan §18 carry 11's, not designed here
(§11); what it must do, while two or more identities are held, is name
the identity in front at its top before anything can be sent
(`NewConversationAs`, "You are starting this conversation as
{identity}"),
and ask nothing: no choice of identity, since a chooser that could pick
another identity while this one is in front is the confusion the
owner's rule forbids (ADR-0061 d. 3). To start as another identity, the
person switches first, on Chats (§5.7.1); nothing on this screen
switches.

#### 5.7.5 Notifications

A notification names the identity it arrived on, in its visible text
and not only by an icon: on Android the identity's name is the
notification's sub-text (shown in its header), on the desktop it leads
the notification's title ("{identity} · {title}") (`human-client-ui.md`
§10). Notifications of different identities are never folded into one
summary that hides which is which: on Android each identity is its own
notification group, its summary naming the identity. On the lock screen,
when the person has chosen to hide notification content, the redacted
form shows no identity name either, since an identity's name can be what
the person wants hidden; the switch happens on opening, after the
unlock (ADR-0061 d. 9). The service's persistent
notification names no identity: there is one runtime (ADR-0061 d. 9).

Opening a notification of the identity in front opens its conversation,
as today. Opening one of **another identity** switches first, visibly,
before anything of that identity is shown: a full-screen transition
with that identity's 96 dp avatar and `SwitchingTo` ("Opening as
{identity}"), held for at least one second and until the conversation
is ready, announced; then the conversation opens, and the snackbar
`ConfirmIdentitySwitched` with Switch back shows in it. Switch back
there closes the conversation and shows the previous identity's Chats.
Back from the conversation goes to the new identity's Chats. Under
reduced motion the transition has no movement and keeps its duration.
(The one second is a hypothesis: long enough to read a name and a face,
short enough not to be a wait; falsified if people in a test reply as
the identity they were in before, or do not say which identity they are
in when asked right after.) A notification offers no action that sends
(an inline reply) unless its field names the identity it replies as;
none is specified here (§11).

#### 5.7.6 States of an identity

- **Cannot send or receive** (its session holds no lease, or its
  endpoint is turned off, by the first identity's Turn off, §5.2, or in
  the configuration): its row in the switcher and
  §5.1 shows `IdentityUnavailable`; it can still come in front, its
  conversations readable, and its composer line reads
  `IdentityUnavailable` in place of "As {identity}", the send control
  disabled with that as its reason. Turning an added identity off is
  not designed here (§11).
- **No card yet**: the first identity (`human`) before the person
  writes its card; every further identity is made with its card (§5.1).
  It is named by its route label everywhere a name would be, and Share
  asks for the name first (§5.3). **Named by its label**: after a
  discard every identity's card is made again with its label as its
  name (ADR-0061 d. 1); it shows as the label, and Share shows that name
  in the preview, so the person sees what goes and can edit first.
- **Key cannot be used**: its identity key file was refused at open,
  or is missing after it shared its card. It sends and receives as
  before; Share and the card page show the state and the way to a new
  key, which `design/2026-10-24-identity-key.md` defines.

## 6. Showing contacts

### 6.1 Name and avatar

- **Title** (conversation app bar, list row): the contact's
  `display_name` in place of the short PeerId. `human-client-ui.md` §4's
  route rule is unchanged: when two conversations with the same peer on
  different routes exist, the title is `DirectTitle` with the name in
  `{peer}`.
- **Two contacts with the same name** (of the identity in front, by
  §5.4.1's rule, so a lookalike spelling counts): each title carries
  what tells it apart from every other contact of that name, both when
  both are needed (`human-client-ui.md` §4, §13):
  - another contact of that name on **another PeerId** → the short
    PeerId, never a route label alone, which another device can forge;
  - another contact of that name on **the same PeerId** (another
    identity of that device) → the route label;
  - both at once (three or more contacts) → both, PeerId first.
  So `ContactTitleTwin` is "{name} ({peer})", "{name} ({route})" or
  `ContactTitleTwinBoth` "{name} ({peer}, {route})", and no two titles
  are one line twice. Never told apart by colour alone.
- **Long names**: one line, ellipsised at the end (the logical end, so
  RTL ellipsises on the left); the full name is in the accessible label
  and on Details.
- **Avatar**: the accepted photo, circle-cropped, at the fixed sizes the
  look spec uses (40 dp list, 36 dp app bar, 96 dp contact page); else
  initials: the first grapheme of the first word and of the last word of
  the name (one grapheme for a one-word name), uppercased where the
  script has case, on the hue the PeerId already picks (look spec §5.1),
  so a contact keeps its colour when it gains a name. A photo is never a
  trust signal, as the look spec says of the hue.
- **Screen reader**: the row's label is the title text; the avatar is
  not read.

### 6.2 Contacts (list)

App bar: back arrow, title "Contacts" and, while two or more identities
are held, the identity in front as the subtitle (§5.7.2): the list is
that identity's contacts and no other's. Rows sorted by name in the
locale's collation: avatar, name, and under it `ContactRoutes` ("{n}
conversations") only when there is more than one. A row opens the contact page.

- **Empty**: `ContactsEmpty`: no contacts yet; a contact is added when
  someone shares their card and you accept it, or when you name a
  conversation from its Details.
- Nothing here creates a contact without a route: a contact always
  starts from a conversation.
- **Snackbars on these pages** (Delete contact, Remove photo, Move, and
  an identity page's Make default): a snackbar shown on the Contacts
  list or a contact page lives while the person moves between those
  two, and one shown on an identity's page lives while they stay on it;
  each closes when the person leaves its pages or switches identity
  (§5.7.3, which closes the pages too on a wide layout), as a conversation's
  closes when they leave it (message-swipe §6: one at a time, a new one
  replaces the old).

### 6.3 A contact

App bar: back arrow, the name as title, the identity in front as the
subtitle while two or more are held, overflow with **Delete contact**
(`DeleteContact`).

- the 96 dp avatar, and **Remove photo** when there is one;
- **Name** (`ContactName`), an editable field with **Save** beside it on
  a wide layout and under it on a narrow one; saving marks the name a
  local edit; the same input rules as §5.2 (the name is local only, but
  one rule is easier to learn and keeps a later card comparable);
- the provenance line `ContactNameFromCard` ("Name from their card,
  {date}") or `ContactNameTyped` ("Name you entered");
- **Routes**, one row each: the short PeerId and the route label as
  `DirectTitle` builds it, the trust state in words
  (`ContactRouteTrusted` "Trusted" / `ContactRouteNotTrusted` "Not
  trusted", from the PeerId's trust, never inherited from another route,
  ADR-0043), and **Open conversation**, which opens that route's direct
  conversation — empty, with §5.3's offer, when nothing of it is held
  any more (a conversation whose messages were read and not kept is
  gone after a restart, look spec §5.2, while the contact stays,
  ADR-0059 d. 3); such a conversation is listed in Chats only once it
  holds a message, as any conversation is; a row expands to the full PeerId
  with Copy PeerId;
- **Shares a device**: under a route whose PeerId also holds a route
  of another contact of this identity, `ContactSharesDevice`: "Same device
  as your contact {name}", one line for each other contact, each a
  link to its page,
  and once on the page the plain statement `ContactSharesDeviceNote`:
  one device can hold several identities, so these contacts may be one
  person; InterWeave never combines them (ADR-0061
  d. 4 and d. 5). Only the identity in front's contacts are named.
- **Your side of it**, with two or more identities held, once on the
  page under the routes: `ContactYourIdentitiesNote`, that this contact
  sees the PeerId all of the person's identities share, so a contact
  who knows another of them can tell both are on this device
  (`human-client-ui.md` §4; ADR-0061 d. 5). It names no other identity.
- **Move to another contact** on a route row (`MoveRoute`) when there
  are other contacts that hold no route of this route's PeerId. Move
  takes this one route (ADR-0061 d. 4, ADR-0059 d. 4 (ii)): a contact
  holds at most one route per PeerId, so a contact that already holds
  one of this PeerId is not offered, and the picker says why when that
  leaves it shorter (`MoveRouteNoneSameDevice`). First a picker of
  contacts (a dialog titled `MoveRoutePickerTitle`, the eligible
  contacts as single-choice rows, with Cancel), then a confirmation
  dialog `MoveRouteConfirm` naming both contacts and the route's
  conversation in its sentence, saying that trust does not carry
  across, with the full PeerId under it in the code font and Copy
  PeerId. This is the only way two PeerIds come under one contact
  (ADR-0059 d. 4: one person's several devices, ADR-0043); no screen
  suggests it from a name or a photo. When the route is all the
  contact has, moving it deletes that contact, and the dialog says so
  (`MoveRouteLast`). Move runs at once and shows `ConfirmRouteMoved`
  with Undo, which moves the route back and, when the move deleted the
  contact, writes it back with its values and their sources; a kill
  inside the window leaves the move done, as said. The Undo is
  withdrawn — its snackbar closes — as soon as its reverse would
  break a rule or lose a write: a card on the moved route is accepted
  (the Accept may have written its name and photo to the target, which
  no Move undo can tell apart from the target's own; the Accept's own
  Undo is then the way back), or another route of the same PeerId comes
  under the source contact (§6.2 on how long a snackbar on these pages
  lasts). After Move the moved-to contact's page opens, and its Back
  goes to the Contacts list, since the source page may no longer exist.
  Undo opens the source contact's page again, its Back to the Contacts
  list. A Move and its Undo change which contact holds a route, so the
  proposals in the session are recomputed (§5.4.5), as after a delete.
  It is in v1 on both graphical clients and the command line
  (architect-cto, §12 A4). A route never moves between identities: a
  contact belongs to the identity that accepted it (ADR-0061 d. 4), and
  the picker lists the identity in front's contacts only.

**Remove photo**: at once, snackbar `ConfirmPhotoRemoved` with Undo
(writes the bytes back). After Remove photo, the contact has no photo by a local
choice, so a later card's photo is not used and the proposal says
`CardKeepsNoPhoto` (case E, or case S when nothing else changes,
§5.4.1).

**Delete contact**: at once (contact and its routes deleted), snackbar
`ConfirmContactDeleted` with Undo (writes them back with the same
values and provenance). The Undo is withdrawn — its snackbar closes —
as soon as a route the deleted contact held comes under another contact
of this identity (an Accept, or Details' Add as a contact), so it can
never put one route under two contacts. The page closes to the list. Conversations stay;
their titles return to the short PeerId. The proposals in the session
are recomputed (§5.4.5).

### 6.4 Details

The conversation's Details dialog (look spec §6) gains, above the
PeerId:

- with two or more identities held, `DetailsIdentity`: "You write as
  {identity} in this conversation", with its avatar and route label: the conversation's
  identity, fixed for its life (ADR-0061 d. 3);
- the name and its provenance (§6.3), or for a route under no contact
  **Add as a contact**, with `DetailsSharesDevice` above it by §5.6's
  rule; with a contact, **Open contact** (`OpenContact`);
- `DetailsCardShared`: "You shared your card here on {date}", from
  `card_shared_at`, or `DetailsNoCard` ("You have not shared your card
  here."). This is the person's own record and is never sent.

## 7. Discard and recreate

### 7.1 Where it is offered

| condition (ADR-0060 d. 6 order) | screen | Discard and recreate is |
|---|---|---|
| key invalidated (Android only) | **K**, key no longer accepted | the primary action, still confirmed |
| a key that does not open the file, no key file, an unlisted combination of names, a failed migration | **R**, the recovery screen | a third action, on the desktop on its own line at the start side, above Close and Try again at the end |

**Desktop**: R is today's `RefusalWindow` page for
`RefusalStoreRecovery`. Its actions become, from the start: **Delete
stored data…** (`DiscardStore`, an outlined button in `error`) on a
line of its own at the start side, then, on the line below it, at the
end, **Close** (text) and **Try again** (filled, the initial focus). The
two lines hold at every width: the three do not fit one line of the
window's 320 px minimum, and a layout that moved the button only when
narrow would have to read the window's width, which Slint 1.18 refuses
as a binding loop (rust-ui-dev-01, 2026-10-22). Above rather than below
keeps Close and Try again where a desktop dialog's actions sit, at the
bottom end, and keeps the destructive action apart from the primary one
by its line and its side; a press on it only opens the confirmation
(§7.2), so its place cannot lose data. Tab follows the reading order:
the file path, then Delete stored data…, Close, Try again. The ellipsis says a further step
follows.
`RefusalStoreRecovery`'s text ("Nothing was changed…") stays
true until the person confirms; its sentence on
replacing the file with a backup copy no longer fits an encrypted
store; §10 gives what the new text must say (architect-cto, §12 A5).

**Android**: there is no store-recovery screen today (`jni.rs` maps the
refusal to code −5; nothing presents it as a page). This spec gives it
one, built on the identity screens' frame
(`design/2026-10-19-android-identity.md` §3): heading `StoreRecoveryTitle`,
the reason (`StoreRecoveryBody`), and at the bottom **Try again** (filled), then **Delete stored
data…** (outlined, `error`), then **Close** (text; closes the app's
task). No file path: the person cannot reach it. **K** has heading
`StoreKeyLostTitle`, body `StoreKeyLostBody` (the phone no longer accepts
the key that protects the app's stored data, which happens after some
changes to the screen lock; the stored data cannot be read again; the
device's PeerId and recovery phrase are not affected; and the way forward: Delete stored
data, which shows what is lost before anything is deleted), primary **Delete stored data…**, then
**Close**. K has no Try again: the platform has said the key is gone.

### 7.2 Building the confirmation

Pressing **Delete stored data…** first tries `human.sqlite.encrypted`
read-only under the key when that file exists (ADR-0060 d. 6). While it
tries, the button shows a progress indicator and is inert; past 300 ms
the screen also shows `StoreChecking`. Then one of two dialogs opens:

**Full discard** (no copy, or a copy that cannot be opened for any
reason):

- title `DiscardStoreTitle`: "Delete all stored data?";
- `DiscardStoreLoses`, a list, each item its own line, for **every
  identity** on this device and not only the one in front, which the
  list's lead-in says (ADR-0061 d. 9: no action whose scope the person
  cannot see): your contacts and their routes; messages waiting to be
  sent; unread and kept messages; your conversation list; the names and
  photos of your identities; the app's settings other than your network
  settings; and the record of the
  messages you already received (without it, one of them might show
  again; it is an item of the list because the lead-in says what is
  deleted);
- `DiscardStoreKeeps`: this device's PeerId and recovery phrase, the
  peers you trust, and your network settings are not affected; your
  identities stay, since they are the profile's endpoints (ADR-0061
  d. 1), and their cards are made again at the next open with each
  identity's label as its name, until the person names it again
  (§5.7.6). With one identity
  held, neither list speaks of identities: the lead-in is the plain one
  and the item reads "the name and photo on your card" (ADR-0061 d. 3);
- `DiscardStoreRemains`: copies the system made earlier may stay on the
  disk (RETENTION.md's disclaimer, ADR-0060 d. 6);
- actions **Cancel** (default focus) and **Delete stored data**
  (`DiscardStoreConfirm`, filled in `error`).

**Unfinished file only** (a copy exists and opened):

- title `DiscardPartialTitle`: "Delete the unfinished file?";
- `DiscardPartialBody`: an update to how your data is protected was
  interrupted; a complete copy of your data exists; this removes only
  the unfinished file, and the app opens with your data;
- actions **Cancel** (default focus) and **Delete the unfinished file**
  (`DiscardPartialConfirm`, filled, not in `error`: nothing the person
  has is lost).

Escape, Back and a tap outside are Cancel. Enter activates the focused
button, which starts on Cancel, so Enter alone never discards. There is
no second confirmation after either dialog (the loss is listed once).

### 7.3 Doing it

Confirm: the dialog closes, the screen shows `StoreDiscarding` with an
indeterminate indicator, every action inert, Back ignored (a half-done
discard cannot be left). It is short; it is not cancellable.

- **Done**: the app opens as on any start. Chats shows a notice banner
  (the existing notice slot) `StoreDiscarded` ("The stored data
  was deleted. This device's PeerId and the peers you trust did not
  change.") after a full
  discard, or `StoreRestored` ("Your data was restored from the complete
  copy.") after the partial one, with **Close** to dismiss it; it is
  shown once and not stored.
- **Failed** (a file could not be removed): back on R or K with the line
  `StoreDiscardFailed` under the heading and the same actions; what was
  removed is not re-listed, since the next press rebuilds the dialog
  from the files as they then are.

## 8. Accessibility

TalkBack does not reach the Android app yet (Slint 1.18.1's Android
backend has no accessibility bridge; look spec §7). Every Android
screen-reader check below is a **blocked precondition**, not a pass,
until it does; the desktop's screen readers (through Slint's
accessibility tree) are checked now.

**Reading order.**

- Proposal (§5.4): one group, read as heading (the claim, with the full
  name), "includes a photo" (when shown, §5.4), "PeerId" and the PeerId, the context line,
  the "goes at close" line, time and status; then Copy PeerId, Open
  your contact {name} (when the row shows `CardSameName`), Decline,
  Accept (its case label) as controls. The PeerId is read character by
  character only on request (the platform's own); the label says what
  it is. The row's custom actions (Android) are Accept and Decline with
  the case labels, as message-swipe §7 sets for its rows. Accept is also a custom action on a row that shows `CardSameName`
  (§5.4.4, §12 O3). In case
  S, the only custom action is Close, as in the row's menu.
- Confirmation row (§5.4.2): one focusable group, read as
  `CardAccepted` or `CardUpdated` (with the name as saved), then
  `CardPhotoNotUsed` or `CardPhotoNotUsedRemoved` when one shows, then, while the Accept's snackbar
  is open, `CardUndoHint` as its description.
- Own card row: "You shared your card", the name, status, time.
- Share sheet: title, the "as {identity}" line when shown, then the
  preview as one item (`ShareCardPreviewSpoken`, or
  `ShareCardPreviewSpokenPhoto` when the photo goes), the note, Cancel,
  Edit your card, Share.
- The switch: one control, its name `IdentitySwitch` with the badge's
  count in it. The switcher: title, then each identity as "{name},
  {route label}, {unread} (`IdentityRowUnread`), selected/not selected,
  n of m", then Your
  identities.
- A conversation, with two or more identities: the "As {identity}"
  line is read just before the send control, and the send control's
  name says the identity again (`SendAs`), so a person who moves
  straight to it hears who sends.
- Discard dialog: title, each lost item as a list item, the keeps line,
  the remains line, Cancel, Delete stored data.

**Announcements** (the app's two alternating live strings): a card
arriving in the shown conversation is announced as `AnnounceCard` ("New
contact card"), never with the name; Decline, photo removed,
contact deleted, a Move, Make default and a switch announce their
snackbar text (the photo removed snackbar shows on the contact page only, §6.3; the
editor's Remove photo announces nothing, §5.2); in the card editor,
a prepared photo announces `PhotoAdded` and an unusable one
`PhotoUnusable`, and the start of preparing announces nothing (§5.2); Accept announces nothing of its own. Focus moves to the confirmation
row (§9 item 1's focus table), and the screen reader reads that row:
`CardAccepted` or `CardUpdated`, and `CardPhotoNotUsed` or
`CardPhotoNotUsedRemoved` when one shows. So the person hears the result once, and does not hear it again
as "Contact saved" or "Contact updated". The snackbar with its Undo is
shown without an announcement and is reached last in the reading order
(language-culture-en's observation 01a16df9, 2026-10-24); the transition of §5.7.5 announces `SwitchingTo`; the
sheet's Share announces nothing extra (focus moves, and the row
appears with "Sending"). Nothing that arrives for an identity not in
front is announced in the app: its notification is the platform's.

**Targets**: every control at least 48 × 48 dp (Android) and keyboard
reachable (desktop and a hardware keyboard on Android); radio rows 56 dp
high. **Text scaling** to 200 % reflows: the sheets and the proposal
scroll rather than clip; the proposal's actions, stacked at every
scale (§5.4), grow in height as their labels wrap;
the switch keeps its avatar and three graphemes and the "As {identity}"
line wraps to two lines before it ellipsises, since it is the one
naming that must stay readable where the person sends.

**New colour roles.** Four roles here are not in look spec §3's table,
and rust-ui-dev adds them to `app.slint` with these needs (Material 3's
names, so a reader recognises them):

- `outline-variant`: the proposal's 1 dp border; decoration only, with no
  contrast need, since the heading says in words what the row is;
- `surface-variant`: the neutral avatar's fill; its glyph needs 3:1
  against it;
- `surface-container-low`: the share sheet's surface; text on it 4.5:1,
  the focus ring 3:1;
- `on-error`: text on an `error` fill (the full discard's confirm);
  4.5:1.

**Contrast** (WCAG 2.2; one row per surface a mark can sit on, both
schemes; rust-ui-dev measures and reports the figures):

| mark | on | needs |
|---|---|---|
| proposal heading and body text | `surface-container` | 4.5:1 |
| `CardSameName` in `error` and its icon | `surface-container` | 4.5:1 text, 3:1 icon |
| neutral avatar glyph | `surface-variant` | 3:1 |
| focus ring | `surface-container` (proposal), `primary-container` (own card row), the sheet's `surface-container-low`, the dialog's `surface-container-high` | 3:1 each |
| Discard outlined button text and outline in `error` | `surface` (R, K) | 4.5:1 text, 3:1 outline |
| Discard confirm, `on-error` on `error` | the button | 4.5:1 |
| the code-font PeerId | `surface-container`, dialog | 4.5:1 |
| the switch's name and ▾, and its focus ring | the top app bar's `surface` | 4.5:1 text, 3:1 ▾ and ring |
| the switch's badge: count on its dot, and the dot | the dot's fill; the app bar's `surface` | 4.5:1 count, 3:1 dot |
| "As {identity}" line | the composer's surface | 4.5:1 |
| `SwitchingTo` text | the transition's surface | 4.5:1 |

**Motion**: the sheets slide in, the proposal's in-place replacement
and a switch's list cross-fade, and §5.7.5's transition fades; under
reduced motion each appears without movement, the transition keeping
its duration.

**Never colour alone**: an identity's hue (§5.1) helps recognition and
tells nothing by itself: every place that names an identity names it in
words (§5.7.2).

**Focus**: §9's table, item 1 of the acceptance.

## 9. Acceptance

Each item is observable in the running build on both clients unless it
says otherwise. "Checked by keyboard" means hardware or `adb shell input
keyevent` with uiautomator's `focused="true"` on Android, the keyboard
on the desktop. "Two identities" means a profile with endpoints `human`
and `work`, each with a card ("Anna Rossi" and "A. Rossi").

1. **Focus**, every case in one table:

   | case | focus goes to | check |
   |---|---|---|
   | Your identities / Your card / Contacts opened from the menu | the page title (focusable heading) | keyboard |
   | a card saved | the identity's row in the list, else the control that opened the page (the sheet's Edit your card or Add your name) | keyboard |
   | a card's Save with an invalid field | the first invalid field | keyboard |
   | the editor's Photo Picker or file dialog closes, a picture chosen or cancelled (§5.2) | the button that opened it (Add a photo, or Change photo) | keyboard on desktop: open the file dialog from Add a photo, cancel it, focus on Add a photo; choose a picture, focus on the same button. Keyboard on Android: open the Photo Picker from Add a photo, press Back (`adb shell input keyevent KEYCODE_BACK`), focus on Add a photo; choose a picture, focus on the same button |
   | the editor's photo prepared | stays on that button, which now reads Change photo; `PhotoAdded` announced once | keyboard; desktop screen reader: `PhotoAdded` heard once, and the circle reads `CardPhotoPreparing` while the picture is being prepared, then `CardPhotoLabel`; TalkBack blocked (§8) |
   | the editor's picture unusable | stays on that button; `PhotoUnusable` announced once | keyboard, choosing a file named `.png` that holds no image; desktop screen reader: heard once; TalkBack blocked (§8) |
   | the editor's Remove photo, also while a picture is being prepared (§5.2) | Add a photo; nothing is announced, also when the dropped picture would have been ready | keyboard; on a card with a saved photo, choose a large picture and press Remove photo before it is ready: focus on Add a photo, the circle shows the initials, and Save leaves the card with no photo; the same steps and results on a card saved with no photo, after a first picture is prepared and the person chooses a second one with Change photo |
   | unsaved-changes dialog opens / closes on Continue editing (whichever control asked it, §5.2) | Continue editing / the control that had focus when it opened (an editor field, or the back arrow when it was used; the switch; the picked row), else the app options button when a menu item asked; on Close without saving, where the action that asked puts it with no unsaved change (§5.2) | keyboard: from an editor field press Escape (desktop) or Back; on a wide window move focus to a conversation row and press Enter; open the app options and choose Contacts; each time Continue editing returns focus as this row says and the editor keeps its change |
   | an identity added | its new row in the list | keyboard |
   | Add an identity at the bound | `IdentityLimit` | keyboard |
   | Make default, and its Undo | stays on the overflow button | keyboard |
   | Remove this identity: the dialog opens / is cancelled | Cancel / the overflow button | keyboard |
   | an identity removed, two or more left (in front or not) | the next row, else the previous | keyboard |
   | an identity removed, one left | Chats' title | keyboard |
   | Turn off / Turn on this identity, and the Undo | stays on the overflow button | keyboard |
   | a change refused (Make default, Remove, Turn off, Turn on) | the identity page's overflow button, Remove's dialog closed first | keyboard |
   | the switch activated | the switcher's chosen row | keyboard |
   | the switcher closed with nothing changed | the switch | keyboard |
   | a switch, from the switcher or Switch back | the switch, naming the new identity | keyboard |
   | a notification of another identity opened | the transition's text, then the conversation's header title | keyboard; TalkBack blocked (§8) |
   | Switch back inside that conversation | the switch on the previous identity's Chats | keyboard |
   | the share sheet opens | Share, else Add your name | keyboard |
   | the sheet closes after Share | the control that opened it; the header title when that was the empty conversation's offer | keyboard |
   | the sheet cancelled | what opened it (overflow button or the offer's button) | keyboard |
   | Accept (button, menu item, custom action or swipe) | the confirmation row (focusable), read once with `CardUndoHint`: no live announcement of `ConfirmContactSaved` or `ConfirmContactUpdated` follows (§8 Announcements); the snackbar's timeout waits while focus is between the row and the snackbar, for at most 2 minutes (§5.4.2) | keyboard: with three or more rows after the confirmation row, wait past 6 s on the row, then move one stop at a time, more than 6 s apart, to Undo, and Undo is still reached; move focus to something before the row, then back to the row within 6 s, wait past 6 s, and Undo is still reached; pointer Accept with no further input: the snackbar closes by 2 minutes; desktop screen reader: in a case N and a case U accept, the row is read with `CardUndoHint` and neither "Contact saved" nor "Contact updated" follows; TalkBack blocked (§8); swipe with TalkBack cannot occur |
   | Decline, by button, menu item or keyboard | message-swipe §7's removal rule: only if focus was in the row, to the next row's first control, else the previous row's, else the conversation's header title; never the composer | keyboard |
   | Decline by a touch swipe | stays where it was (message-swipe §7) | keyboard focus set elsewhere first, then a swipe |
   | Undo of Accept or Decline | the proposal row's Accept | keyboard |
   | case S Close | as Decline | keyboard |
   | the proposal's menu opens (Menu key or Shift+F10 on a row control, long press, secondary click) | its first item | keyboard: Menu key and Shift+F10 from each of Copy PeerId, Open your contact (a warned row), Decline, Accept, and Close (case S); long press (Android) and secondary click (desktop) |
   | the proposal's menu closes with nothing chosen (Escape, Back, outside) | the control where the key that opened it was pressed; after a long press or secondary click, Copy PeerId | keyboard: opened from Accept, Escape returns focus to Accept; after a secondary click, Escape puts focus on Copy PeerId |
   | a menu item that leaves the row in place: Copy PeerId, or Accept refused by a degraded store | as the row above | keyboard: Shift+F10 on Decline, choose Copy PeerId, focus on Decline; with the store degraded, Shift+F10 on Accept, choose Accept, focus on Accept |
   | snackbar appears (except after Accept, above) | stays where it was; the snackbar is reachable last in order | keyboard |
   | Open your contact {name} on a row showing `CardSameName`, or a shares-a-device link | the contact page title; Back returns to the control that opened it | keyboard |
   | contact deleted | the Contacts list's first row, else its empty text | keyboard |
   | Move: the picker opens | the first eligible contact | keyboard |
   | Move: the picker cancelled | the route row's Move | keyboard |
   | Move route dialog | Cancel; after Move, the moved-to contact's page title | keyboard |
   | Undo of a Move | the moved route's row on the source contact's page, recreated if needed | keyboard |
   | Discard pressed (R or K) | the dialog's Cancel | keyboard |
   | dialog cancelled | the Discard button | keyboard |
   | discard failed | the failure line (focusable) | keyboard |
   | after a discard, Chats opens | the notice banner | keyboard |

2. Sharing sends only on Share: opening a conversation, receiving a card,
   connecting and the offer itself send nothing (check: the peer's
   receiving client shows no card; the sender's conversation shows no
   own card row).
3. A channel's overflow has no Share a card; no share sheet is
   reachable from a channel.
4. The share sheet shows the conversation's identity's card and no
   choice; with two identities, a share from a `work` conversation
   arrives at the counterparty as "A. Rossi" with `source_endpoint`
   `work`, and from a `human` conversation as "Anna Rossi" with
   `human`, measured at the counterparty.
5. Two identities sharing their cards to one counterparty: the
   counterparty's proposals each show only their own card's name, the
   second is case P naming the first contact, and no screen of theirs
   names an identity of the sender that did not write to them.
6. A proposal shows the claim, the full PeerId (copy gives the exact
   canonical form), "includes a photo" when Accept would write the
   card's photo (§5.4), and the
   "goes at close" line; before Accept no decoded photo appears anywhere,
   and a test pins that the decoder is not called (rust-ui-dev).
7. Each case P, N, D, U, E and S of §5.4.1 shows its context line and Accept
   label, and Accept does what the table says. On every row that shows
   `CardSameName` (case D, and the case P, U and E fixtures below that
   show it), a right swipe past the commit threshold does not move the
   row, shows no accept reveal and does not accept, and the store is
   unchanged; a left swipe declines. On the case P fixture without
   `CardSameName` ("Carol"), the right swipe accepts (§5.4.4). In case
   E, a locally edited name is unchanged after Accept. A later card with no photo
   arrives on a route of a contact whose photo came from an earlier card: the proposal is case U and shows `CardUpdatesPhotoRemoved`;
   after Accept the contact has no photo, and Undo gives it back. When the
   name matches, the same later card, against a contact whose photo
   the person chose, is case S: it shows `CardKeepsPhoto` and no
   `CardSame`, and Accept is not offered. A later
   card whose photo fails to decode, against a contact whose photo came
   from a card, shows `CardPhotoNotUsedRemoved` on the confirmation row and
   leaves no photo; against a contact with none, `CardPhotoNotUsed`.
   Case E: a contact the person named themselves, whose photo came from
   an earlier card, meets a later card with another name and no photo:
   the proposal shows `CardKeepsEdit` and `CardUpdatesPhotoRemoved`,
   and after Accept the name is unchanged and the photo is gone. The
   same contact, as it was before that Accept, meets a later card with
   another name and a photo that fails to decode: the proposal shows
   `CardKeepsEdit` and `CardUpdatesPhoto`, and the confirmation row
   shows `CardPhotoNotUsedRemoved`.
   After Remove photo (§6.3) on a contact whose name came from a card,
   a later card with the same name and a photo is case S: it shows
   `CardKeepsNoPhoto` and Close, and shows none of `CardSame`,
   `CardKeepsPhoto` and `CardIncludesPhoto`; after Close the contact
   still has no photo. A later card with the same name and no photo
   is case S too: it shows `CardSame` and no photo line. When a later card with another name and a photo
   arrives for the same contact, the proposal is case E and shows
   `CardUpdatesName` and `CardKeepsNoPhoto`, and neither
   `CardKeepsPhoto` nor `CardIncludesPhoto`; after Accept the name is
   the card's and the contact still has no photo. Control: the same
   card against the contact as it was before Remove photo is case U
   and shows `CardIncludesPhoto` and `CardUpdatesPhoto`.
   Fixtures for D against a contact "Anna Rossi": a card "Anna" +
   "Rossi"; "Аnna" (Cyrillic А) + "Rossi"; a name "Anna Rossi" with
   U+00A0 between the words and no surname; "Ａnna" (fullwidth) +
   "Rossi" — each shows `CardSameName`. Fixtures for D that the case key does
   not join, each card against a contact of the plain name, on
   another PeerId: "ꓮda" (Lisu U+A4EE) against "Ada"; "Ꭰave"
   (Cherokee U+13A0) against "Dave"; "ꓟax" (Lisu U+A4DF) against
   "Max"; "Βob" (Greek U+0392) against "Bob"; "Alſie" (long s,
   U+017F) against "Alfie"; "J０hn" (fullwidth digit zero U+FF10)
   against "John" — each shows `CardSameName`. "Alſie" is joined only
   by the shape key and "J０hn" only by the compatibility shape key, so
   a build that leaves out either shape key fails one of these fixtures. A card "Ian" against a contact "ian", a pair that only
   the case key joins, also shows `CardSameName`. Controls: a card "Mia" against a
   contact "Max", and "Hanna" + "Rossi" against "Anna Rossi", are case
   N and show no `CardSameName`. Each pair is also checked by §5.4.1's
   three keys directly, as a unit test of the store's comparison
   (rust-ui-dev). A card from a second endpoint
   of contact Carol's PeerId is case P: Accept leaves two contacts, each
   holding one route, Carol unchanged, and each contact page naming the
   other as sharing its device. A card naming itself "Anna Rossi" from
   a new route of Carol's PeerId (case P), and one from Carol's own route
   (case U), each show `CardSameName` naming the "Anna Rossi" on another
   PeerId before Accept; a card naming itself "Carol" on another route
   of Carol's PeerId shows case P's line and no `CardSameName`. A
   contact named Dan has a photo the person chose locally and a name
   that is not a local edit. A card naming itself "Anna Rossi", with a
   photo, from Dan's route is case E: the name changes and the photo
   stays. It shows `CardSameName` and
   `CardKeepsPhoto`, and no `CardIncludesPhoto`, and its right swipe is withheld. While a right
   drag is held past the threshold and not yet released, an action of
   §5.4.5 adds the warning to the row: the row returns to rest, and
   the later release accepts nothing. On desktop, with no contact
   named "Anna Rossi", the person drags the row of a card naming itself
   "Anna Rossi" right with the pointer past the threshold and holds it
   there, and then, from the keyboard, accepts another proposal that saves a contact "Anna
   Rossi" on another PeerId. While a withheld right
   drag is held, an action of §5.4.5 removes the warning: the row
   stays still, and the release accepts nothing. On desktop, again
   with no contact named "Anna Rossi" at the start, the person first
   accepts, from the keyboard, the proposal that saves a contact
   "Anna Rossi" on another PeerId, so that the row shows the warning.
   Then the person starts a right drag on the row with the pointer,
   moves the pointer past the threshold and holds it there, and then
   chooses, from the keyboard, the Undo of that Accept. A left drag held past the
   threshold while the warning appears or disappears still declines on
   release. The right swipe is withheld on the case D row and on case
   E's "Anna Rossi" row. On each of them, the menu holds Accept with
   the case label ("Accept as a new contact" for case D, "Update
   contact" for case E), Decline and Copy PeerId. Choosing that
   Accept saves the card exactly as the button does, with the same
   confirmation row and snackbar. The row's Accept custom action on
   Android does the same (TalkBack blocked, §8). The menu opens by the
   Menu key and by Shift+F10 with focus on each of Copy PeerId, Open
   your contact, Decline and Accept, by a long press, and on the
   desktop by a secondary click. Enter on each of those controls does
   that control's action and opens no menu. On Android,
   on the case D row and on case E's "Anna Rossi" row, a long press
   that slides onto Accept and lifts there leaves the menu open with
   nothing saved. Then a tap on Accept saves the card. On a case N row,
   the same slide and lift accepts the card. On each of these rows the
   menu opens under the card when there is room below it, above the
   card when there is not, and over the card at the top of the visible
   area when there is room on neither side (a case D card at 200 %
   text). In each of the three places, two long presses at different points
   of the card open the menu at the same place.
8. Delete contact Carol, then accept a card on one of Carol's routes
   (case N): Delete's Undo is gone. Move a route, then accept a card on
   it: the Move's Undo is gone. No action and no Undo leaves a contact
   holding two routes of one PeerId (the Move picker omits a contact
   that holds one). Accept then Undo leaves the store as before the
   Accept; Decline then restart leaves nothing of the card in the store;
   an unanswered, shown proposal is gone after a restart; an unshown one
   survives it.
9. The title and the list show the accepted name; two contacts with one
   name on two PeerIds each show `{name} ({peer})`, two on one PeerId
   `{name} ({route})`; three named "Anna Rossi", two on PeerId A (routes
   `x` and `y`) and one on B, show A's two as `{name} ({peer}, {route})`
   with different routes and B's as `{name} ({peer})`; the list preview and the notification of a card
   never contain the claimed name.
10. A right-to-left name inside the proposal heading and the same-name
    note does not reorder the surrounding words (a fixture with an
    Arabic name and a name containing U+202E, which must be refused as a
    format character, HUMAN-CHAT.md).
11. The photo sent is exactly 256 × 256 (or the picture's own smaller
    square, never enlarged), at most 24,576 bytes, and carries no
    metadata block (EXIF, XMP, text chunks), checked on the bytes a test
    captures before send; a JPEG fixture stored with Orientation 6
    arrives with its marker in the corner where a viewer that honours the
    tag shows it. In the card editor, Remove photo writes nothing: with
    a saved photo, Remove photo then Back shows `CardDiscardChanges`,
    and Close without saving leaves the saved photo; Remove photo then Save leaves
    the card with no photo; no snackbar shows in either.
12. A card too large for a low-limit profile is offered "Share without
    the photo" and nothing is sent before the person presses it.
13. R shows Delete stored data… as a third action on both clients (on
    the desktop on its own line at the start side, above Close and Try
    again at the end side, at every width, Try again focused at open,
    Tab running the file path, Delete stored data…, Close, Try again);
    K (Android, key invalidated) shows it as the primary with no Try
    again; both confirm before deleting.
14. The full-discard dialog's two lists are §10's `DiscardStoreLoses`
    and `DiscardStoreKeeps` in the form for the number of identities
    held, and nothing more: with two or more, the lead-in says "for all
    your identities" and the keeps list says every identity stays,
    named by its label until renamed; with one, neither list names
    identities (the lead-in without "for all your identities", the item
    "the name and photo on your card", the keeps list only that this
    device's PeerId and recovery phrase, the peers you trust and your
    network settings are not affected); with a complete
    `human.sqlite.encrypted` the partial dialog shows instead and the
    data is present after the next open; with that copy unopenable the
    full dialog shows and no second dialog follows.
15. In every confirmation dialog of this spec Enter with the initial
    focus cancels; Escape/Back cancels.
16. After a discard, the PeerId, trust and every identity are
    unchanged, each identity's card named by its label, and Chats shows
    `StoreDiscarded` once.
17. Every control in this spec is labelled in the accessibility tree
    (desktop now; Android blocked on the bridge), and each swipe has its
    buttons. The proposal's actions are stacked (§5.4): at 100 % and
    at 200 % text, in English and with a case label long enough to
    wrap, on the narrow layout and the wide one, Decline and Accept
    each fill the content column's width on a line of their own,
    Decline above Accept (Close alone in case S); a wrapped label
    shows whole inside its button, each button at least 48 dp high;
    keyboard focus moves in this order: Copy PeerId, Open your contact
    {name} (when the row shows `CardSameName`), Decline, Accept
    (in case S: Copy PeerId, Close).
18. §8's contrast table measured, every row at or above its threshold,
    in both schemes.
19. **One identity in front.** With two identities, `human` in front:
    Chats, Contacts, a contact page and the Move picker show none of
    `work`'s conversations or contacts; a same-name contact under
    `work` raises no `CardSameName` under `human`; the same counterparty
    accepted under both is two contacts, each seen only under its own
    identity. Every screen of §5.7.2's table shows its naming, checked
    screen by screen; with one identity none of it shows and no
    question is asked.
20. **Only §5.7.3's five acts switch.** Starting the app with the
    identity last in front still held, opening a contact, opening Your
    identities or an identity's page, adding an identity and Make
    default each leave the identity in front unchanged; each of the
    five acts switches and names the identity right after, except act 4
    with one identity left, where `ConfirmIdentityRemoved` names the
    removed one and nothing else; choosing
    another identity in the switcher changes the
    list, the switch's name and every "As" line at once and shows
    `ConfirmIdentitySwitched`; Switch back returns. On a wide layout a
    switch closes an open conversation, Contacts, a contact page and an
    identity's page to the placeholder, and leaves Trust and Your
    identities open; with a card's unsaved changes, the
    unsaved-changes dialog comes first and Continue editing cancels the
    switch.
21. **A conversation never changes identity.** Messages and a card sent
    from a `work` conversation all carry `source_endpoint` `work`,
    measured at the counterparty, before and after any switch; nothing
    in the app moves a conversation, contact or card to another
    identity.
22. **Notifications.** A message to `work` while `human` is in front
    posts a notification naming `work`; opening it shows the transition
    naming `work` for at least a second before the conversation, then
    the conversation under `work` with its snackbar; with content hidden
    on the lock screen, the redacted notification names no identity.
    The switch's badge counts the unread item, and names nothing.
23. **Removing an identity** is offered on added identities only,
    never on the first (`human`) or the default; it is confirmed by a
    dialog listing §5.2's losses and the freed label's line with focus
    on Cancel, and lands by §5.2's cases: with two or more left,
    the list and `ConfirmIdentityRemoved`, or
    `ConfirmIdentityRemovedSwitched` naming the identity now in use when
    the removed one was in front; with one left, Chats with no switch
    and no "As" line, and `ConfirmIdentityRemoved`.
24. **No directory lists an identity.** Every identity's endpoint, the
    `human` one included, has `advertise: false`, and a trusted peer's
    directory query lists none of the person's identities; no screen
    offers a control for it, or one that narrows who can write to an
    identity. With a pending message already tried, Remove's dialog
    names it as one that may have reached. Add an identity refuses an
    agent's label with `IdentityRouteTaken`.
25. **Turning the first identity off** is offered only once another
    identity is turned on and not on the default; Make default is not
    offered on an identity that is off; on both platforms no sequence of
    Make default, Turn off, Remove and their Undos leaves a default that
    is off or no identity on (the sequence: Make default X, Turn off
    `human`, then Make default `human` is not offered and Remove X is
    not offered); a refused call shows `IdentityChangeFailed` and
    changes nothing; with one identity and no default, its card page
    shows `IdentityNoDefault` and Make default, which then has no Undo,
    and its snackbar and any refusal use their one-identity
    forms, without the word "identity"; it deletes nothing,
    lists the identity as off, leaves it in front if it was, refuses
    messages to it while off, and its Undo and Turn on bring it back.
26. **Every way off the card editor with unsaved changes asks**
    (§5.2). With a name changed and not saved, each of these opens
    `CardDiscardChanges`, its two actions stacked, Close without saving
    above Continue editing, each the dialog's full width at 100 % and
    200 % text on a 360 dp phone: Back; Escape on the desktop; an
    identity switch; a notification's conversation; and on a wide
    window, a conversation picked in the list and Trust, Contacts and
    Your identities from the app options (with one identity, Your card
    from the app options keeps the editor open with the change and asks
    nothing). For each, Continue editing
    leaves the editor open with the changed name, the list's selection
    as it was, and focus as item 1's table says; Close without saving opens
    what was asked, and the card reopened shows the saved name. With no
    change, each opens what was asked at once, with no dialog.

### 9.1 `human-client-ui.md` §13 criteria

architect-cto ruled on this section's first proposal on #335
(REPLY 01a15d98): §13 holds the criteria for contacts, cards and the
store's discard, read with ADR-0061 and ADR-0059 A 2026-10-21 (ii), and
they are the ones a build answers to. This section's earlier list is
not repeated here, so that there is one source. Two of §13's items
already carry ADR-0061 decision 9 (the identity in front named on every
acting screen; no screen in front of one identity showing the other's
data). Offered for the Stage 22 list, which plan §30 step 4 has
rust-ui-dev propose and architect-cto rule, the person-facing ones this
note adds:

- the identity in front changes only on the person's acts of §5.7.3
  (the switcher, Switch back, opening a notification of another
  identity, removing the identity in front, and at start the default
  when the last one is gone), and the change is named right after it;
- a notification of another identity is opened only after a transition
  naming that identity;
- no notification summary, badge or announcement mixes the content of
  two identities;
- Trust and the store's discard say that they cover every identity;
- an identity is removed only after a confirmation listing what goes,
  and only an added one: never the first or the default;
- when there is a default it is an identity that is turned on, and no
  action leaves no identity turned on.

## 10. Strings

Every string a person reads here, for language-culture-en to write.
The Text column is a proposal that shows what the string must say;
the third column is what it must communicate and where. Key names are
suggestions; rust-ui-dev names them. Placeholders: `{name}` (a received
or saved name, plain text, isolated), `{title}` (a conversation's title),
`{identity}` and `{front}` (an identity's card name, else its route
label; `{front}` the identity in front), `{route}` (a route label),
`{peer}` (a short PeerId),
`{n}` (a number in digits), `{date}` (a date in the app's date format).
No string uses "keep" as an ordinary verb (Keep is a message action).
"Identity" is the person's word for a user identity (§1); a string that
used it for the device's PeerId and recovery phrase is marked **L1**
(§12) for language-culture's word for the device. Rows marked "new
text, proposal" or "revised, proposal" replace language-culture-en's
final text of the first round where the records changed what it says,
and are for them to write.

| Key | Text (proposal) | Where, and what it must say |
|---|---|---|
| `ContactsTitle` | Contacts | menu item and page title |
| `CardEdit` | Your card | an identity's card page title (§5.2) |
| `AddPhoto` / `ChangePhoto` / `RemovePhoto` | Add a photo / Change photo / Remove photo | editor and contact page |
| `CardName` | Name | editor field |
| `CardSurname` | Family name (optional) | editor field; must say optional |
| `CardSharedStays` | People who already have this card still see it as it was. To show them your changes, share the card with them again. | editor note |
| `Save` | Save | editor, contact page |
| `CardNameNeeded` | Enter a name. | field error |
| `CardNameTooLong` | Use 64 characters or fewer. | field error; "characters" for scalar values |
| `CardNameBadCharacters` | Remove line breaks and invisible characters. | field error for Cc/Cf/separators |
| `CharCount` | {n} / 64 | field counter |
| `CardDiscardChanges` | Close without saving your changes? | unsaved-changes dialog |
| `KeepEditing` / `DiscardChanges` | Continue editing / Close without saving | its actions; stacked, Close without saving above (§5.2) |
| `PhotoUnusable` | This photo cannot be used. Choose another one. | editor, under the circle; announced once (§5.2) |
| `CardPhotoPreparing` | Preparing the photo… | new text, proposal; the editor's photo circle, accessible label while a chosen picture is being prepared (§5.2); must say the photo is not ready yet |
| `PhotoAdded` | Photo ready | new text, proposal; announced once in the editor when a chosen picture has been prepared (§5.2); must not say it is saved or sent, since that happens only on Save and Share |
| `ShareCard` | Share a card | conversation overflow, empty-conversation offer |
| `ShareCardTitle` | Share your card with {title} | share sheet title |
| `ShareCardPreview` | What {title} will see | share sheet preview heading |
| `ShareCardNote` | They will see this name and photo, and they can save them. You cannot remove a card from their device after it arrives. | share sheet note |
| `ShareCardAction` | Share | share sheet primary |
| `CreateCard` | Add your name | share sheet primary when the identity's card has no name |
| `ShareCardNoPhoto` | The photo is too large to send in this conversation. | share sheet, over the limit |
| `ShareCardWithoutPhoto` | Share without the photo | share sheet primary, over the limit |
| `ShareCardTooLarge` | This card is too large to send in this conversation. | share sheet, nothing fits |
| `ShareCardOffer` | Let {title} see your name instead of a PeerId. | empty conversation |
| `CardProposal` | This peer says they are {name} | proposal heading; must keep "says": a claim, not a fact |
| `CardIncludesPhoto` | This card has a photo. You see it after you accept. | proposal, when Accept would write the card's photo; not shown when the contact's photo is a local choice, or the contact has no photo by a local choice (§5.4) |
| `CardPeerIdLabel` | PeerId | proposal, above the PeerId |
| `CopyPeerId` | Copy PeerId | existing (#313) |
| `CardGoesAtClose` | If you do not accept this card, it is removed when InterWeave stops. | proposal; Android's "closes" includes the system ending the app: language-culture to find the word that is true there |
| `Accept` / `Decline` | Accept / Decline | proposal actions, case N |
| `AcceptAsNew` | Accept as a new contact | cases D and P |
| `UpdateContact` | Update contact | cases U, E |
| `ShareCardPreviewSpoken` | {title} will see the name {name}. | share sheet preview, accessible description, no photo |
| `ShareCardPreviewSpokenPhoto` | {title} will see the name {name} and your photo. | the same, with the photo |
| `MoveRoutePickerTitle` | Choose a contact for this conversation | the move-route picker's title. One route moves, this conversation's (ADR-0061 d. 4); the text must say what the person picks a contact for |
| `CardPhotoLabel` | Photo for this card | editor photo circle, accessible label |
| `CardPhotoNone` | No photo for this card | the same, with no photo |
| `CardSameName` | One of your contacts is named {name} and has a different PeerId. Compare the two PeerIds before you accept. | case D, and cases P, U and E when Accept would write a name that clashes with a contact on another PeerId (§5.4.1); `{name}` the other contact's name; must name both facts |
| `ShowContact` | Open your contact {name} | the link beside `CardSameName`: case D, and cases P, U and E when `CardSameName` shows |
| `CardUpdates` | If you accept, your contact {name} is updated. | case U |
| `CardUpdatesName` / `CardUpdatesPhoto` | New name: {name} / New photo | cases U and E, the changes written |
| `CardKeepsEdit` | You named this contact {name} yourself. That name stays. | case E, name |
| `CardKeepsPhoto` | You chose this contact's photo yourself. That photo stays. | cases E and S, when the contact holds a photo the person chose, whether or not the card carries a photo (§5.4.1); never for a contact with no photo |
| `CardKeepsNoPhoto` | You removed this contact's photo yourself. The contact stays without a photo, and the photo on this card is not used. | new text, proposal; cases E and S, when the card carries a photo and the person removed the contact's photo (§5.4.1, §6.3); must say that the contact stays without a photo by the person's own choice and that the card's photo is not used, and must not name a photo the contact has |
| `CardSame` | This card matches your contact {name}. | case S, only when the card's name is the contact's, character for character, and neither the card nor the contact has a photo (§5.4.1); must not show when the card differs in a value the person chose |
| `CardAccepted` | You saved {name} as a contact. | confirmation row, cases N, D and P |
| `CardUpdated` | You updated your contact {name}. | confirmation row, cases U and E (language-culture-en's text) |
| `CardUpdatesPhotoRemoved` | Your contact's photo will be removed. | cases U and E, the change when the card carries no photo and the contact's photo is not a local choice (§5.4.1); must say that the contact's current photo goes, not that the card's is missing, and that the contact's photo has not gone yet: the line shows before Update contact (language-culture-en's final English, 2026-10-25) |
| `CardPhotoNotUsedRemoved` | The card's photo could not be used, so the contact no longer has a photo. | confirmation row addition in place of `CardPhotoNotUsed`, cases U and E, when the contact's previous photo was not a local choice (§5.4.2); must say the previous photo is gone too |
| `CardPhotoNotUsed` | The card's photo could not be used, so the contact was saved without it. | confirmation row addition, in both cases of §5.4.2 (the photo fails to decode, or decodes larger than 256 × 256); must not say the photo was shown, since it is decoded only at Accept (language-culture-en's final English, 2026-10-24) |
| `ConfirmContactSaved` / `ConfirmContactUpdated` | Contact saved / Contact updated | snackbars, with Undo; not announced (§8 Announcements) |
| `CardUndoHint` | You can undo this change. Undo comes after all the messages in this conversation. | accessible description of the confirmation row while the Accept's snackbar is open (§5.4.2), never shown as text; must say that the save can be undone and where the Undo is, in a phrase read after `CardAccepted` or `CardUpdated` |
| `ConfirmCardDeclined` | Card declined | snackbar, with Undo |
| `CardPreviewReceived` / `CardPreviewSent` | Contact card / You shared a card | list previews |
| `CardNotification` | You received a contact card. | notification body; never the name |
| `AnnounceCard` | New contact card | live announcement |
| `CardSentLabel` | You shared your card | own card row |
| `AddAsContact` / `OpenContact` | Add as a contact / Open contact | Details |
| `DetailsNoCard` | You have not shared your card here. | Details |
| `ContactsEmpty` | One identity: You have no contacts yet. Someone becomes a contact when you accept a card they share, or when you use Add as a contact in a conversation's Details. Two or more: {front} has no contacts yet. Someone becomes a contact when you accept a card they share, or when you use Add as a contact in a conversation's Details. | §6.2 empty; with two or more identities it is this identity's list that is empty, and the text must not suggest the person has no contacts at all (language-culture to judge whether it needs saying) |
| `ContactRoutes` | {n} conversations | contact row, n ≥ 2: the number of the contact's conversations, called conversations as every other string does (plural rules language-culture's) |
| `ContactTitleTwinBoth` | {name} ({peer}, {route}) | title of a contact sharing a name both with a contact on another PeerId and with one on its own (§6.1) |
| `ContactTitleTwin` | {name} ({peer}) | title of a contact sharing a name: `{peer}` the short PeerId across devices, the route label within one device (§6.1) |
| `ContactName` | Name | contact page field |
| `ContactNameFromCard` | Name from their card, {date} | provenance |
| `ContactNameTyped` | Name you entered | provenance |
| `ContactRouteTrusted` / `ContactRouteNotTrusted` | Trusted / Not trusted | route trust in words |
| `OpenConversation` | Open conversation | route row |
| `MoveRoute` | Move to another contact | route row |
| `MoveRouteConfirm` | Move this conversation from {name} to {other}? Moving does not change whether you trust {peer}. | dialog; `{other}` the target contact's name; must say one conversation moves and trust does not follow |
| `MoveRouteLast` | {name} has no other conversations, so moving this one deletes the contact {name}. | dialog addition, when the route is all {name} has; must say moving it deletes the contact |
| `ConfirmRouteMoved` | Conversation moved to {other} | snackbar after Move, with Undo |
| `Move` | Move | dialog primary |
| `DeleteContact` | Delete contact | contact overflow |
| `ConfirmContactDeleted` / `ConfirmPhotoRemoved` | Contact deleted / Photo removed | snackbars, with Undo; `ConfirmPhotoRemoved` on the contact page only, never in the card editor (§5.2) |
| `IdentitiesTitle` | Your identities | menu item, page title, the switcher's link |
| `OwnCardTitle` | Your card | menu item with one identity |
| `IdentitiesShareDevice` | Desktop: Your identities use this device's PeerId and network addresses. Someone who talks to two of your identities can see that these identities are on the same device. Anyone who sees your network traffic can also see this. To stop the people you talk to from linking two identities, use a separate InterWeave profile for each. The network can still see that both profiles are on this device. Any peer you trust who knows an identity's label can write to that identity. There is no setting to limit this. Android: Your identities use this device's PeerId and network addresses. Someone who talks to two of your identities can see that these identities are on the same device. Anyone who sees your network traffic can also see this. On this phone, any two of your identities can be linked in this way. Any peer you trust who knows an identity's label can write to that identity. There is no setting to limit this. | §5.1; Android ends instead: "This phone cannot keep two identities fully apart." Must state the shared PeerId and addresses, who can see it, the separation that exists, and that a label is no secret from a trusted peer (ADR-0061 d. 5) The last sentence was changed by ux-designer when the reach control was withdrawn |
| `IdentityRouteLabel` | Seen by others as {route} | §5.1 row, under the name |
| `IdentityDefault` | Default | §5.1 row |
| `IdentityInFront` | In use | §5.1 row's accessible description; the check mark's meaning |
| `AddIdentity` | Add an identity | §5.1 primary |
| `AddAnotherIdentity` | Add another identity | §5.2 overflow while one identity is held (language-culture-en's text) |
| `IdentityLimit` | You cannot add more identities on this device. | §5.1 at the profile's endpoint limit, and on a Save refused for it |
| `IdentityRouteField` | Label that others see | §5.1 new identity's field |
| `IdentityRouteHelp` | Everyone you talk to as this identity sees this label next to your PeerId. You cannot change the label after the identity is created. | under that field and on §5.2; must say who sees it and that it is fixed |
| `IdentityRouteTooLong` | Use 64 characters or fewer. | route label over 64 (language-culture-en's text, as `CardNameTooLong`) |
| `IdentityRouteNeeded` / `IdentityRouteBadCharacters` / `IdentityRouteTaken` | Enter a label. / Use only lowercase letters, digits, dots (.), hyphens (-) and underscores (_), and start with a letter. / Another identity on this device already uses this label. | field errors |
| `IdentityAddFailed` | The identity could not be added. Try again. | §5.1 above Save |
| `MakeDefault` | Make default | §5.2 overflow |
| `IdentityIsDefault` | This is your default identity. | §5.2 line |
| `IdentityDefaultHelp` | People who write to your PeerId without using one of your labels reach your default identity. | §5.2 helper, with two or more identities only; ADR-0061: "which identity a stranger reaches" |
| `ConfirmDefaultSet` | {identity} is now your default identity | snackbar, with Undo; without Undo when there was no default before |
| `ConfirmDefaultSetOne` | People who write to your PeerId without using your label now reach you. | snackbar after Make default with one identity held, no Undo; must not use the word "identity" |
| `RemoveIdentity` | Remove this identity | §5.2 overflow |
| `IdentityRemoveDefault` | To remove this identity, first make another identity the default. | §5.2, on the default identity |
| `IdentityRemoveTitle` | Remove {identity}? | dialog title |
| `IdentityRemoveLoses` | This deletes from this device: the conversations of {identity}, with their unread and kept messages and the messages waiting to be sent; the contacts of {identity}; the card of {identity}, with its name and photo. People who write to {identity} will no longer reach you. This cannot be undone. | dialog list, one item per line; must list exactly §5.2's items and say there is no undo |
| `IdentityRemoveMayHaveReached` | One: 1 of the messages waiting to be sent may already have arrived. Deleting it does not take it back. Other: {n} of the messages waiting to be sent may already have arrived. Deleting them does not take them back. | Remove dialog, when pending rows were already tried (RETENTION.md `may_have_reached`); plural forms language-culture's; must say they may have arrived and that deleting does not recall them |
| `ConfirmIdentityRemoved` | {identity} removed | snackbar after Remove, no Undo |
| `ConfirmIdentityRemovedSwitched` | {identity} removed. You are now using {front}. | snackbar after Remove of the identity in front with two or more left; must name both |
| `IdentityRemoveLabel` | The label {route} can then be given to a new identity. If that happens, people who have {identity} as a contact will reach the new identity through the contact they saved for {identity}, until they accept the new identity's card. | Remove dialog, last line; must say the label is freed and what reusing it would mean for this identity's contacts |
| `TurnOffIdentity` / `TurnOnIdentity` | Turn off this identity / Turn on this identity | Turn off: the first identity's overflow; Turn on: the overflow of any identity that is off (§5.2) |
| `ConfirmIdentityOff` / `ConfirmIdentityOn` | {identity} turned off / {identity} turned on | snackbars; the first with Undo |
| `IdentityDefaultNeedsOn` | To make this identity the default, first turn it on. | §5.2, on an identity that is off |
| `IdentityChangeFailed` | Make default, one identity: Could not set the default. Nothing was changed. Make default, two or more identities: Could not make {identity} the default. Nothing was changed. Remove: Could not remove {identity}. Nothing was changed. Turn off: Could not turn off {identity}. Nothing was changed. Turn on: Could not turn on {identity}. Nothing was changed. | snackbar when the device refuses Make default, Remove, Turn off or Turn on; one whole text per action, each saying nothing was changed and, with two or more identities, naming the identity (language-culture-en: a button's words placed into a sentence cannot be translated or reordered); with one identity, Make default's form names no identity and does not use the word; the one-identity Make default form added in round 5 |
| `IdentityNoDefault` | One identity: People who write to your PeerId without using your label do not reach you. To let them reach you, choose Make default in the menu on this page. Two or more identities: You have no default identity. People who write to your PeerId without using one of your labels do not reach any of your identities. To set a default, open an identity that is turned on and choose Make default. | §5.1, when the profile has no default; must say who is not reached; with one identity, a one-identity form on the card page, naming no identity, saying people who write to the PeerId without the label do not reach the person and that Make default in the page's menu fixes it; the one-identity form added in round 4 |
| `IdentityRemoveConfirm` | Remove | dialog primary, in `error` |
| `IdentitySwitch` | Identity: {identity}. Switch identity | the switch's accessible name |
| `IdentityOthersUnread` | One other identity: 1 unread message in your other identity / {n} unread messages in your other identity. Two or more other identities: 1 unread message in your other identities / {n} unread messages in your other identities. | the switch's badge, accessible; plural rules language-culture's |
| `IdentitySwitchTitle` | Switch identity | switcher title |
| `IdentityOthersNotSent` | A message in another identity was not sent. | the switch's accessible name when the badge has no count (language-culture-en's text) |
| `IdentityRowUnread` | 1 unread message / {n} unread messages | a switcher row's unread count, spoken (language-culture-en's text) |
| `IdentityNotSent` | A message was not sent | switcher row, when a message of that identity was not sent |
| `ConfirmIdentitySwitched` | You are now using {identity} | snackbar after a switch; must say which identity is now in use, not only that it changed |
| `SwitchBack` | Switch back | that snackbar's action |
| `SwitchingTo` | Opening as {identity} | §5.7.5 transition |
| `ComposerAs` | As {identity} | the line above the send control |
| `SendAs` | Send as {identity} | the send control's accessible name with two or more identities |
| `ShareCardAs` | as {identity} | share sheet, under the title |
| `EditCard` | Edit your card | share sheet |
| `ShareCardNoName` | Your card has no name yet. Add your name, then share the card. | share sheet, no name |
| `IdentityNotInFront` | You are using {front}. What you change on this page applies to {identity}. | §5.2 when the page's identity is not in front |
| `TrustAllIdentities` | When you trust a peer, you trust them for all your identities on this device. | Trust page with two or more identities |
| `NewConversationAs` | You are starting this conversation as {identity} | new-conversation flow, when built |
| `IdentityUnavailable` | {identity} cannot send or receive right now | §5.7.6 |
| `CardSharesDevice` | This card comes from the same device as your contact {name}. | case P |
| `CardSharesDeviceMany` | This card comes from the same device as your contact {name} and 1 other contact. / This card comes from the same device as your contact {name} and {n} other contacts. | case P, several |
| `CardSharesDeviceNote` | One device can hold several identities. You can save this card as a separate contact. | case P, under it |
| `DetailsSharesDevice` | The person in this conversation uses the same device as your contact {name}. | Details, a route under no contact |
| `DetailsIdentity` | You write as {identity} in this conversation. | Details |
| `DetailsCardShared` | You shared your card here on {date}. | Details, from `card_shared_at` |
| `ContactSharesDevice` | Same device as your contact {name} | contact page, under a route |
| `ContactSharesDeviceNote` | One device can hold several identities, so these contacts may be one person. InterWeave does not combine them. | contact page |
| `ContactYourIdentitiesNote` | This contact sees your PeerId, which all your identities share. If this contact also talks to another of your identities, this contact can see that both identities are on the same device. | contact page with two or more identities; must say the PeerId is shared by all the person's identities and what a contact who knows two can tell; names no identity |
| `MoveRouteNoneSameDevice` | Contacts that already have a conversation with this conversation's device are not listed. A contact can have only one conversation with each device. | Move picker |
| `StoreRecoveryTitle` | InterWeave cannot read its stored data | Android R heading |
| `RefusalStoreRecovery` (revised) | InterWeave cannot read its stored data, which includes your messages and contacts. Nothing was changed. The data opens only with its key on this device, so there is no copy of it that you can put back. You can try again, close InterWeave, or choose Delete stored data. InterWeave shows what you lose before it deletes anything. If a newer version of InterWeave wrote this data, install and start the newest version that you have used, or a newer one. Restoring this device's PeerId from its recovery phrase does not bring back this data. | desktop R body (Android R has its own, `StoreRecoveryBody`): its backup-copy sentence assumes a plaintext file a person can swap; the store is now encrypted and a copy opens only with its key. Must still say the file was not changed until the person chooses. Since architect-cto's A5 (§12) it must also say: the stored data opens only with its key on this device, and a copy without its key opens nothing, so there is no backup copy to put back; the way back is Try again, Close, or Delete stored data, which loses what it lists; the recovery phrase restores the identity alone, never this data. It must not say to replace the file with a backup, and must not suggest moving the key or the data between devices (ADR-0060 d. 8). **L1**: "this device's PeerId" is the device's identity, which the recovery phrase restores; not a contact-card identity |
| `StoreRecoveryBody` | InterWeave cannot read its stored data, which includes your messages and contacts. Nothing was changed. The data opens only with its key on this phone, so there is no copy of it that you can put back. You can try again, close InterWeave, or choose Delete stored data. InterWeave shows what you lose before it deletes anything. Restoring this device's PeerId from its recovery phrase does not bring back this data. | Android R body (new key): Android cannot install an older version over a newer one, so the desktop's newer-version advice is not true here, and Android shows no path. Since architect-cto's A5 (§12) it must also say: the stored data opens only with its key on this device, and a copy without its key opens nothing, so there is no backup copy to put back; the way back is Try again, Close, or Delete stored data, which loses what it lists; the recovery phrase restores the identity alone, never this data. It must not say to replace the file with a backup, and must not suggest moving the key or the data between devices (ADR-0060 d. 8). **L1**: "this device's PeerId" is the device's identity, which the recovery phrase restores; not a contact-card identity |
| `StoreKeyLostTitle` | The key to your stored data no longer works | K heading |
| `StoreKeyLostBody` | This phone no longer accepts the key that protects InterWeave's stored data. This can happen after some changes to the screen lock. The data cannot be read again. This device's PeerId and recovery phrase are not affected. To use InterWeave again, choose Delete stored data. InterWeave shows what you lose before it deletes anything. | K body; **L1**: "This device's PeerId" is the device's identity, not a contact-card identity |
| `DiscardStore` | Delete stored data… | R, K action |
| `StoreChecking` | Checking the stored data… | §7.2 progress |
| `DiscardStoreTitle` | Delete all stored data? | full dialog title |
| `DiscardStoreLoses` | Two or more identities: This deletes from this device, for all your identities: your contacts and their routes; messages waiting to be sent; unread and kept messages; your conversation list; the names and photos of your identities; the app's settings, other than your network settings; the record of the messages you already received (one of them might show again). One identity: This deletes from this device: your contacts and their routes; messages waiting to be sent; unread and kept messages; your conversation list; the name and photo on your card; the app's settings, other than your network settings; the record of the messages you already received (one of them might show again). | full dialog list; one item per line; must list exactly these, and say it is every identity's (§7.2); with one identity, no "for all your identities", and the item reads "the name and photo on your card" |
| `DiscardStoreKeeps` | One identity: This device's PeerId and recovery phrase, the peers you trust and your network settings are not affected. Two or more identities: This device's PeerId and recovery phrase, the peers you trust and your network settings are not affected. All your identities are kept. Each one shows its label as its name until you give it a new name. | full dialog; with two or more, must say every identity stays and is named by its label until renamed; the one-identity form is unchanged; ADR-0061 at #335's 74d05203 withdrew the rule the second-round text followed |
| `DiscardStoreRemains` | Earlier copies that your device made outside InterWeave might still be on it. | full dialog |
| `DiscardStoreConfirm` | Delete stored data | full dialog primary |
| `DiscardPartialTitle` | Delete the unfinished file? | partial dialog title |
| `DiscardPartialBody` | An update to how your data is protected was interrupted. A complete copy of your data exists. This deletes only the unfinished file. Then InterWeave opens with your data. | partial dialog |
| `DiscardPartialConfirm` | Delete the unfinished file | partial dialog primary |
| `StoreDiscarding` | Deleting… | §7.3 progress |
| `StoreDiscarded` | The stored data was deleted. This device's PeerId and the peers you trust did not change. | Chats notice after a full discard; **L1** |
| `StoreRestored` | Your data was restored from the complete copy. | Chats notice after a partial discard |
| `StoreDiscardFailed` | The stored data could not be deleted. Try again. | R/K after a failure |

Existing keys reused unchanged: `AppOptions` (look spec §9; the Chats
⋮), `MoreOptions` (the conversation's ⋮ only), `BackToConversations`,
`Cancel`, `Close`, `Undo`, `TryAgain`, `Details`, `DirectTitle`, `OwnPeerId`,
`ShortPeer`, `StorageDegraded`.

## 11. Deliberately left out

- **Choosing the photo's crop**: a centred square is taken. A crop step
  is a later addition if people's photos are cut badly (hypothesis: most
  are portraits whose face is centred; falsified by a test where people
  reject the preview).
- **Contacts as a navigation destination** (bar or rail): reached from
  the menu until it is used often enough to earn the space.
- **Search or sort in Contacts and Your identities**, a contact's notes
  and device labels: not asked for; the store has the columns. A search,
  when one comes, searches the identity in front only (ADR-0061 d. 9):
  a search across identities is excluded by the record, not left out.
- **Turning an added identity off**, and per-identity notification
  settings (an Android channel per identity, to mute one): the overlay
  and the platform allow them; not asked for. The first identity's Turn
  off exists because it cannot be removed (§5.2); §5.7.6 says what a
  turned-off identity shows.
- **A shortcut that switches identity** (a key, a gesture on the
  switch): left out on purpose, since a switch made without looking is
  the failure §5.7 exists to prevent.
- **An inline reply from a notification**: not specified; if it comes,
  its field names the identity it replies as (§5.7.5).
- **Starting a conversation with a PeerId no contact or conversation
  holds**: plan §18 carry 11's own flow, not designed here. A contact
  starts from a conversation and may outlive it; for its routes, §6.3's
  Open conversation is the way back.
- **The command-line client's words and output**: its implementer's
  (ADR-0059 d. 9). Its discard needs an explicit confirmation flag and
  refuses without it; this spec's lists are what it should print.
- **Anything about a third party's card** (introductions): ADR-0059
  rejects it for v1.
- Required versus suggested: everything in §3, §5.4.1's table, §7 and
  §9 is required; the dp sizes, colours and the ordering of menu items
  are suggestions rust-ui-dev may change within the look spec and §8's
  contrasts.

## 12. Questions

For the owner:

- **O1** — §1's swipe: right on a proposal accepts. In case D (a contact
  of the same name already exists on another PeerId), should the right
  swipe be withheld, so that only the button "Accept as a new contact"
  saves it? Proposed: withheld, because a second "Anna" accepted by a
  stray thumb is the impersonation ADR-0059 makes visible, and Undo
  only helps if the person noticed. **Answered 2026-10-21**, the owner:
  "i agree with your reccomandation" — withheld in case D (§5.4.4); the
  left swipe still declines, and every other case keeps the owner's
  swipe as given.
- **O2** — `CardSameName` also shows in cases P, U and E when Accept
  would write a name that clashes with a contact on another PeerId
  (§5.4.1), and O1's answer covered case D only. Should the right swipe
  be withheld there too? Proposed: withheld whenever the warning shows,
  judged when the swipe starts. **Answered 2026-10-25**, the owner, who
  chose "Whenever warned" over "Case D only": withheld whenever the row
  shows `CardSameName` (§5.4.4); a row without it keeps the swipe.
- **O3** — on a row showing `CardSameName`, should the proposal's menu
  and the screen reader's custom action keep Accept, under the case's
  label, or should only the button save the card, as O1's question
  had worded it before the row had a menu? Proposed: keep them, since
  choosing "Accept as a new contact" from a menu is as deliberate as
  the button and only a swipe can happen without reading.
  **Answered 2026-10-25**, the owner, who chose "Keep menu + TalkBack"
  over "Button only": the menu's Accept and the custom action stay on
  a warned row (§5.4.4, §8).

For architect-cto, asked with this revision (the shape of ADR-0061
decision 9 and the records around it; the note is built on the reading
given until they answer):

- **Q1–Q3**, settled by the records on #335 at 6a2896be before they
  were sent: renaming an identity renames its card, never the EndpointId
  (ADR-0061 d. 7; §5.1 fixes the route label); the identity controls
  appear with the second identity, none with one (d. 3 and d. 9;
  §5.7); a new conversation speaks as the identity in front, named
  before the first send, and nothing asks (d. 3; §5.7.4).
- **Q4–Q6**, ruled by architect-cto as read here (REPLY 01a15f0e, on
  #335 at 9746baa4): the redacted lock-screen notification names no
  identity (d. 9; §5.7.5); Remove is always confirmed with the list of
  what goes, deletes pending outbound and tries it no further, has no Undo, is absent on
  the only identity and on the default (d. 7; §5.2); its "only turns
  off an endpoint the configuration file authors" was withdrawn at
  74d05203 (Q7–Q10 below: Remove exists for added identities only); an identity's endpoint
  is created with `advertise: false` and no control, its label is no
  secret from a trusted peer (d. 5; §5.1). Corrected at #335's
  1f9fdbb2 (INFO 01a15f17): the authored `human` endpoint has
  `advertise: false` too; no control narrows who can reach an identity
  in Stage 22, a reach subset being a revisit condition of ADR-0061, so
  the note's "who can reach this identity" control is withdrawn; and
  pending outbound on Remove is "tried no further", one that may have
  reached named as such.

- **Q7–Q10**, ruled by architect-cto (REPLY 01a15f29, then REPLY
  01a15f2d on #335 at 74d05203, which withdrew the first answer's
  card-row rule): an identity is the first, authored `human` endpoint or
  an endpoint the client added (the overlay's `added` member); any other
  configured endpoint, an agent's included, is a route, offered to no
  one; the card is the presentation, made again after a discard with the
  label as its name (d. 1; §5.7.6, §7.2). Remove exists for added
  identities only, deletes the endpoint from `added` and frees its label;
  the first identity is never removed, only turned off once another is
  enabled (d. 7; §5.2). Add never takes a configured endpoint's label
  (§5.1). The limit is the configuration's bound of 64 endpoints, authored and
  added counted together, refused by name (d. 2; §5.1; corrected at
  #335's 29ae9c38, INFO 01a15f30: ENDPOINTS.md's default of 16 was a
  value nothing applied).

For language-culture:

- **L1** — two meanings of "identity". The owner's word is now the
  person's for a user identity (§1). The app used "identity" for the
  PeerId and its recovery phrase: the recovery and discard strings
  marked L1 in §10, and `design/2026-10-19-android-identity.md`'s
  screens. One word cannot mean both on one screen. Asked: the
  person-facing word for the device's identity, and the text of every
  L1 string with it. Proposed: "this device's PeerId and recovery
  phrase" where the restore is meant, as `human-client-ui.md` §2 already
  calls the PeerId a device identity, never a person's.

For architect-cto, answered 2026-10-21 (REPLY 01a15d51); each answer
lands in architect-cto's next records PR, not on #332:

- **A1** — where a value came from. STATE.md's `contacts` gains, for
  the name and for the photo, its source (card or person) and when,
  added by plan §29 step 2's migration. §6.3 and §6.4 are built as
  written.
- **A2** — a proposal has no Keep. Confirmed: Accept is the only way a
  card outlives the session; `human-client-ui.md` §3 will say so.
- **A3** — the photo carries pixels and nothing else. A rule of
  HUMAN-CHAT.md §Contact card: the sender re-encodes and strips every
  metadata block (EXIF, XMP, GPS, thumbnails, file name); a receiver
  treats any that arrives as absent and never shows it; a fixture
  case with a GPS block the sender strips. §5.2 and acceptance 11.
- **A4** — moving a route between contacts is in v1, on both
  graphical clients and the command line. §6.3 keeps Move, of one route
  (A6).
- **A5** — `RefusalStoreRecovery`'s new content is given in §10's row.
- **A6** (asked after round 2's review, QUESTION 01a15d68; answered
  in REPLY 01a15d6a, then withdrawn by ADR-0061 the same day) — the
  first answer was that Move takes every route of a PeerId together and
  a PeerId is never under two contacts. ADR-0061 decision 4 and
  ADR-0059 A 2026-10-21 (ii) replaced it: a contact is one remote
  identity, holds at most one route per PeerId, and Move moves one
  route (§5.4.1 case P, §5.6, §6.3).
