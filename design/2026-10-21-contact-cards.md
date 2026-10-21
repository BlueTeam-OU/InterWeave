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
gives it that swipe. §12 O1 asks the owner one narrowing of it.

## 2. The person's goals

- **Identities:** "Be the right person to the right people, and never
  mix them up." Most people will have one identity and never see the
  word. Some want a work identity and a private one. They need to know,
  at every moment they can send or act, which one they are, and that
  nothing done as one is done as the other. They do not think of an
  EndpointId as anything; they think of a name and a face.
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
  know what they lose before they lose it, and that their identity and
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
  beside itself; it changes only on the person's switch; no list,
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
  can tell they are on the same device, as can the network; keeping two
  identities fully apart needs a separate profile, which the desktop
  can run and this phone cannot (the Android text says only the last
  part that is true there);
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
  stored cannot be kept.
- **At the bound**: with 64 endpoints configured (ENDPOINTS.md), Add an
  identity opens nothing and `IdentityLimit` shows under it, focused.

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
`IdentityRouteTaken` (another identity, or an endpoint the profile
already has, uses it). Save creates the endpoint and its card together
(ADR-0061 d. 2: through the admin connection on the desktop, the
embedded host's configuration path on Android; rust-ui-dev's), with a
progress indicator on Save while it runs; a refusal writes nothing and
shows `IdentityAddFailed` above Save. A new identity is not put in front:
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
another identity** (`AddIdentity`), the way to a second.

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
person who has it; a change here reaches them only when you share it
again), and **Save** (`Save`), filled, at the bottom.

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

Back with unsaved changes: a dialog `CardDiscardChanges`, actions
**Continue editing** (`KeepEditing`, default focus) and **Discard**
(`DiscardChanges`). Back with no change closes at once.

**Make default** runs at once: the default moves to this identity, the
line `IdentityIsDefault` appears, and a snackbar `ConfirmDefaultSet`
with Undo (moves it back). Its helper on the page says what the default
is in the person's terms (`IdentityDefaultHelp`: people who write to
this device without naming an identity reach the default; ADR-0061
Operational implications). It does not put the identity in front.

**Remove this identity** is not offered on the default identity (the
page says `IdentityRemoveDefault`: make another identity the default
first), nor while it is the only one. Elsewhere it opens a confirmation
dialog, `IdentityRemoveTitle` naming the identity, listing what goes
(`IdentityRemoveLoses`: its conversations and their messages, unread,
kept and waiting to be sent; its contacts; its card; and that people
who write to this identity are no longer reached), with **Cancel**
(default focus) and **Remove** (`IdentityRemoveConfirm`, filled in
`error`). Enter on the initial focus cancels; Escape and Back cancel.
Remove has no Undo: the endpoint and every row under it go together,
and an Undo would have to bring back messages the transport may already
have refused for it (§12 Q5). When the removed identity was in front,
the default comes in front, by §5.7.3's switch, visibly. The list
returns with focus on the next row, else the previous.

**The photo.** Add a photo opens Android's Photo Picker (images only,
one item) or the desktop's file dialog filtered to PNG and JPEG. The
client prepares what will be sent from the picked image:

- first turns the pixels upright by the image's own orientation (an
  EXIF Orientation tag, as a camera writes it), since the tag goes with
  the rest of the metadata below and a photo stripped of it unturned
  would arrive sideways;
- then crops the largest centred square, scales it to at most 256 × 256,
  lays any transparency over plain white, and encodes it as JPEG,
  always, at the highest quality from 90 down to 50 that fits 24,576
  bytes (HUMAN-CHAT.md); if quality 50 does not fit, it scales down to
  192 and then 128 pixels a side and tries again; if nothing fits, the
  image is unusable (below). JPEG always, because a lossless PNG of a
  photograph seldom fits and has no quality to lower;
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
indicator, and Save waits for it. Choosing the crop is deliberately left
out (§11).

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
- the line `ShareCardNote`: the person will see this name and photo, and
  can save them; a card cannot be removed from their device once it arrives;
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
- `CardIncludesPhoto` when the card carries one: it includes a photo,
  shown after you accept;
- **PeerId** (a label of its own, `CardPeerIdLabel`, beside the app's
  existing `OwnPeerId`) and the
  full PeerId under it in the code font, wrapping, selectable, with
  **Copy PeerId** (`CopyPeerId`);
- the **context line**, exactly one of (§5.4.1);
- `CardGoesAtClose`, in body small: if you do not accept it, it goes when
  InterWeave closes;
- actions at the end: **Decline** (`Decline`, text button) and **Accept**
  (`Accept`, filled; its label changes by case, §5.4.1).

The row carries the time and the status words of any received item
(Unread, Read) under `human-client-ui.md` §5–§6. It has no Keep: Accept
is how a card is kept, and keeping an unanswered claim would add a state
the person cannot use (architect-cto confirmed, §12 A2).

#### 5.4.1 What Accept will do, said before it is done

The client compares the card with the store and shows one context line
and one Accept label.

**Same name.** What is compared is the name the card would give the
contact — its `name`, then a space and its `surname` when it has one
(ADR-0059 d. 3's `display_name`) — against each contact's
`display_name`. Both sides are reduced to a key: NFKC, then full case
folding, then every run of whitespace (any `Zs` character, a no-break
space included, and tabs) collapsed to one space and trimmed, then the
confusable skeleton of Unicode TS #39 (§4, `skeleton`). Two names are
the same when their keys are equal. The key decides only whether the
warning shows and whether titles carry the PeerId (§6.1); it is never
displayed and never stored in place of the name. The reason is the
threat ADR-0059 makes visible: a peer naming itself "Аnna" with a
Cyrillic А, "Anna Rossi" with a no-break space or "Ａnna" in fullwidth
letters — each allowed by HUMAN-CHAT.md's name rules — must meet the
same warning as "Anna".

| case | context line | Accept label | Accept does |
|---|---|---|---|
| **P** the route is under no contact; another route of the same PeerId is under contact X | `CardSharesDevice`: "This card comes from the same device as your contact {X}." (or `CardSharesDeviceMany`, naming X and the number of other contacts on that PeerId) and, under it, `CardSharesDeviceNote`: a device can hold several identities; you can save this one as a contact of its own | **Accept as a new contact** (`AcceptAsNew`) | creates a contact {name [surname]}, its photo, and the route; X is unchanged, and never gains a second route of one PeerId (ADR-0059 d. 4 (ii), ADR-0061 d. 4) |
| **N** the route is under no contact; no route of its PeerId is under one; no contact has this name | none | **Accept** (`Accept`) | creates a contact {name [surname]}, its photo, and the route |
| **D** the route is under no contact; no route of its PeerId is under one; a contact on another PeerId has the same name | `CardSameName`, in `error`, with a warning icon: "One of your contacts is named {name} and has a different PeerId. Compare the two PeerIds before you accept.", `{name}` being the existing contact's name as stored, so the person sees both spellings; and **Open your contact {name}** (`ShowContact`), which opens that contact's page | **Accept as a new contact** (`AcceptAsNew`) | creates a separate contact; never merges (ADR-0059 d. 4) |
| **U** the route is under contact X; the card differs from X's values and neither is a local edit | `CardUpdates`: "If you accept, your contact {X} is updated." with the values that change named (`CardUpdatesName`, `CardUpdatesPhoto`) | **Update contact** (`UpdateContact`) | writes the card's values to X |
| **E** as U, but a value that differs is a local edit | `CardKeepsEdit`: "You named this contact {X} yourself. That name stays." (and/or the photo variant `CardKeepsPhoto`) | **Update contact** if any value would change, else see S | writes only the values not locally edited |
| **S** the route is under X and the card changes nothing | `CardSame`: "This card matches your contact {X}." | **Close** (`Close`) in place of Accept; Decline is not shown | removes the row from the session, as Decline does, without a snackbar |

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
and is not a local edit — and that name has the same key as a contact on
another PeerId, `CardSameName` is shown too, in `error`, above the
case's own line, naming that contact, with its Open your contact link
(ADR-0059 Security implications: a second contact with an existing name
is shown as such before acceptance). The Accept label stays the case's.
A contact on the same PeerId with the same name is not a lookalike
warning: it is the same device, which case P's line already says, and
their titles are told apart by the route label (§6.1). A photo is a
"value that changes" whenever the card carries one and X's photo is not
a local choice, since the client cannot compare a photo it has not
decoded.

#### 5.4.2 Accept

Runs at once: the contact (and route) is written, and the photo is
decoded only now, with the memory-safe decoder, to the fixed display
size. The proposal row is replaced in place by a short confirmation row
(`CardAccepted`: "You saved {name} as a contact." in cases N, D and P;
`CardUpdated` in cases U and E; each with the name as saved and the new
avatar), so the conversation keeps a trace of what
happened in this session. The title and the list take the name (§6.1).

Snackbar with Undo: `ConfirmContactSaved` in cases N, D and P,
`ConfirmContactUpdated` in U and E. Undo is the reverse: a created
contact is deleted with its route; an updated one gets back the values
it had. The proposal row returns,
read, as it was. A kill inside the window leaves the contact saved,
which is what the snackbar said.

A photo that fails to decode, or decodes larger than 256 × 256, is
discarded and the contact is saved without it (HUMAN-CHAT.md);
`CardPhotoNotUsed` is added to the confirmation row. The accept is not
refused for a photo.

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
row kind E there. The reveal's label is the button's label of the case
("Accept as a new contact" in case D), never a bare "Accept". In case S
neither direction moves (Close is not a swipe action). The buttons are
the non-gesture equivalent, always present.

**A difference from message-swipe §7, on purpose.** There, as revised by
Android review F2, a row at rest shows no buttons and keeps its actions
in a menu. A proposal keeps Accept and Decline visible at rest, because
it exists to be answered: a claim whose answer is hidden in a menu reads
as a message, and the case's label (§5.4.1) is what tells the person what
Accept will do before they do it. The row also has §7's menu (long
press, or Enter on the focused row), holding the same actions the row shows
with the case's labels — Accept and Decline, or in case S Close alone —
then Copy PeerId; it has no Show source, since a
card has no markdown source to show. §12 O1 asks the owner
whether case D should take no right swipe.

#### 5.4.5 Several proposals, and a proposal while away

- Each card is its own row. A later card from the same route does not
  replace an earlier one (d. 3); each row's context line is computed
  when it is shown, and again after **any action or Undo that changes
  which contact holds a route or what a contact's values are** — an
  Accept and its Undo, Details' Add as a contact, a Move and its Undo,
  a Delete and its Undo, a name edit, Remove photo — so an older card under a just-accepted newer one reads as
  case U, E or S against the new values, and a row never keeps a label
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
▾. It is the only control in the app that changes the identity in
front. Its accessible name is `IdentitySwitch` ("Identity: {identity}.
Switch identity"); its target is at least 48 dp high; its name takes
what width is left after "Chats" and the ⋮ and ellipsises at the end,
never below the avatar and three graphemes, the full name in the
accessible name. On the desktop it is in the list pane's bar, where the
same menu is.

When another identity has unread items or a message that was not sent,
the switch carries a badge: a dot with the count of unread items in the
other identities (`IdentityOthersUnread`, read as part of its accessible
name: "{n} unread in your other identities"). It counts and shows
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
| an identity's card | save, make default, remove | that identity as the subtitle (§5.2); when it is not the identity in front, `IdentityNotInFront` under the app bar ("You are using {front}. This page changes {identity}.") |
| Trust | trust, untrust | not an identity: trust is the device's, held for every identity (ADR-0061 d. 1); `TrustAllIdentities` says so under the title, so no one trusts a peer for "this identity only" by mistake |
| recovery, discard (§7) | delete stored data | not an identity: the dialog says it is every identity's data (§7.2) |

The "As {identity}" line is text, not a control: a conversation never
changes identity (ADR-0061 d. 3), and a switch reached from inside one
would read as changing who speaks in it. It is always the identity in
front, because only its conversations are shown.

#### 5.7.3 Switching

Choosing another identity in the switcher, and only that, switches.
Everything shown changes together, at once: the Chats list becomes the
new identity's conversations, the switch shows its name, and on a wide
layout an open conversation closes and its pane shows the new
identity's empty-pane placeholder, since the conversation belongs to the
identity that was in front. Contacts, a contact page and the share sheet
are not open at that moment (the switch is on Chats, and a sheet or a
dialog covers it). What the person had in hand stays with its
identity: a draft stays in its conversation's composer, sends in
progress go on under their own lease, and nothing of the old identity's
is carried into the new one's screens.

A snackbar shown before the switch closes with it: its Undo belonged to
a screen of the old identity, and what it confirmed stays done, as a
kill inside its window would leave it.

Feedback: the list cross-fades (under reduced motion it changes at
once); the snackbar `ConfirmIdentitySwitched` ("You are now
{identity}") with **Switch back** (`SwitchBack`), which switches to the
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
them. Removing the identity in front (§5.2) brings the default in front
by this section's switch, with its snackbar (without Switch back, since
the previous identity is gone).

#### 5.7.4 Starting a conversation, and what never changes identity

A conversation started by the person is the identity in front's
(ADR-0061 d. 9), and a conversation started by a peer is the identity
whose endpoint it reached. Neither ever changes identity, and nothing
moves a conversation, a contact or a card from one identity to another.
The new-conversation flow is plan §18 carry 11's, not designed here
(§11); what it must do, while two or more identities are held, is ask
which identity speaks by naming the identity in front at its top
(`NewConversationAs`, "You start this conversation as {identity}") with
**Switch identity** (`IdentitySwitchAction`) beside it, which performs
§5.7.3's switch, before anything can be sent. It never offers a
per-conversation choice of identity that would leave the conversation
under an identity not in front (§12 Q3 on this reading).

#### 5.7.5 Notifications

A notification names the identity it arrived on, in its visible text
and not only by an icon: on Android the identity's name is the
notification's sub-text (shown in its header), on the desktop it leads
the notification's title ("{identity} · {title}") (`human-client-ui.md`
§10). Notifications of different identities are never folded into one
summary that hides which is which: on Android each identity is its own
notification group, its summary naming the identity. On the lock screen,
when the person has chosen to hide notification content, the redacted
form shows no identity name either (§12 Q4). The service's persistent
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
  endpoint is turned off in the overlay): its row in the switcher and
  §5.1 shows `IdentityUnavailable`; it can still come in front, its
  conversations readable, and its composer line reads
  `IdentityUnavailable` in place of "As {identity}", the send control
  disabled with that as its reason. Turning an endpoint on or off is
  not designed here (§11).
- **No card yet**: named by its route label everywhere a name would be;
  Share asks for the name first (§5.3).

## 6. Showing contacts

### 6.1 Name and avatar

- **Title** (conversation app bar, list row): the contact's
  `display_name` in place of the short PeerId. `human-client-ui.md` §4's
  route rule is unchanged: when two conversations with the same peer on
  different routes exist, the title is `DirectTitle` with the name in
  `{peer}`.
- **Two contacts with the same name** (of the identity in front): on
  different PeerIds, every title of either carries the short PeerId,
  `ContactTitleTwin`: "{name} ({peer})", never a route label alone,
  which another device can forge; on one PeerId (two identities of one
  device), every title carries the route label instead, `ContactTitleTwin`
  with the route in `{peer}`'s place (`human-client-ui.md` §4); so two
  Annas are never one line twice. Same name by §5.4.1's key, so a
  lookalike spelling counts. Never told apart by colour alone.
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
routes") only when there is more than one. A row opens the contact page.

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
  (§5.7.3), as a conversation's
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
  of another contact of this identity, `ContactSharesDevice`: "{other}
  uses the same device (PeerId)", each other contact a link to its page,
  and once on the page the plain statement `ContactSharesDeviceNote`:
  one device can hold several identities, so these contacts may be one
  person; InterWeave keeps them apart and never joins them (ADR-0061
  d. 4 and d. 5). Only the identity in front's contacts are named.
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
(writes the bytes back). Removing marks the photo a local choice, so a
later card's photo is case E (§5.4.1).

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

- with two or more identities held, `DetailsIdentity`: "You speak here
  as {identity}", with its avatar and route label: the conversation's
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
| a key that does not open the file, no key file, an unlisted combination of names, a failed migration | **R**, the recovery screen | a third action beside Close and Try again |

**Desktop**: R is today's `RefusalWindow` page for
`RefusalStoreRecovery`. Its actions become, from the start: **Delete
stored data…** (`DiscardStore`, an outlined button in `error`, at the
start side, apart from the others), then at the end **Close** and **Try
again** (filled). The ellipsis says a further step follows.
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
identity is not affected), primary **Delete stored data…**, then
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
  photos of your identities; the app's settings; and "a message you
  already received may show again";
- `DiscardStoreKeeps`: this device's PeerId and recovery phrase, the
  peers you trust, and your network settings are not affected; your
  identities stay, without their names and photos, since they are the
  profile's configuration (ADR-0061 d. 6) — each is named by its route
  label until the person names it again (§5.7.6). With one identity
  held, neither list speaks of identities: the lead-in is the plain one
  and the item reads "your name and photo" (ADR-0061 d. 3);
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
  was deleted. Your identity and the peers you trust did not change.") after a full
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
  name), "includes a photo", "PeerId" and the PeerId, the context line,
  the "goes at close" line, time and status; then Copy PeerId, Decline,
  Accept (its case label) as controls. The PeerId is read character by
  character only on request (the platform's own); the label says what
  it is. The row's custom actions (Android) are Accept and Decline with
  the case labels, as the swipe spec's §7 does for its rows.
- Own card row: "You shared your card", the name, status, time.
- Share sheet: title, the "as {identity}" line when shown, then the
  preview as one item (`ShareCardPreviewSpoken`, or
  `ShareCardPreviewSpokenPhoto` when the photo goes), the note, Cancel,
  Edit your card, Share.
- The switch: one control, its name `IdentitySwitch` with the badge's
  count in it. The switcher: title, then each identity as "{name},
  {route label}, {n} unread, selected/not selected, n of m", then Your
  identities.
- A conversation, with two or more identities: the "As {identity}"
  line is read just before the send control, and the send control's
  name says the identity again (`SendAs`), so a person who moves
  straight to it hears who sends.
- Discard dialog: title, each lost item as a list item, the keeps line,
  the remains line, Cancel, Delete stored data.

**Announcements** (the app's two alternating live strings): a card
arriving in the shown conversation is announced as `AnnounceCard` ("New
contact card"), never with the name; Accept, Decline, photo removed,
contact deleted, a Move, Make default and a switch announce their
snackbar text; the transition of §5.7.5 announces `SwitchingTo`; the
sheet's Share announces nothing extra (focus moves, and the row
appears with "Sending"). Nothing that arrives for an identity not in
front is announced in the app: its notification is the platform's.

**Targets**: every control at least 48 × 48 dp (Android) and keyboard
reachable (desktop and a hardware keyboard on Android); radio rows 56 dp
high. **Text scaling** to 200 % reflows: the sheets and the proposal
scroll rather than clip; the proposal's actions wrap to their own line;
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
on the desktop.

1. **Focus**, every case in one table:

   | case | focus goes to | check |
   |---|---|---|
   | Your cards / Contacts opened from the menu | the page title (focusable heading) | keyboard |
   | editor saved | the saved card's row in the list (or the new card in the chooser) | keyboard |
   | editor Save with an invalid field | the first invalid field | keyboard |
   | unsaved-changes dialog opens / closes on Continue editing | Continue editing / the field that had focus | keyboard |
   | card deleted from a row's menu | the next row, else the previous, else Add a card | keyboard |
   | Undo of a delete | the restored row | keyboard |
   | chooser opens | the chosen card, else the first card | keyboard |
   | Share with none chosen | the first card | keyboard |
   | chooser closes after Share | the control that opened it; the header title when that was the empty conversation's offer | keyboard |
   | chooser cancelled | what opened it (overflow button or the offer's button) | keyboard |
   | Accept (button or swipe) | the confirmation row | keyboard; swipe with TalkBack cannot occur |
   | Decline, by button or by keyboard | message-swipe §7's removal rule: only if focus was in the row, to the next row's first control, else the previous row's, else the conversation's header title; never the composer | keyboard |
   | Decline by a touch swipe | stays where it was (message-swipe §7) | keyboard focus set elsewhere first, then a swipe |
   | Undo of Accept or Decline | the proposal row's Accept | keyboard |
   | case S Close | as Decline | keyboard |
   | snackbar appears | stays where it was; the snackbar is reachable last in order | keyboard |
   | Open your contact {name} from case D | the contact page title; Back returns to the proposal's Accept | keyboard |
   | contact deleted | the Contacts list's first row, else its empty text | keyboard |
   | Move: the picker opens | the first other contact | keyboard |
   | Move: the picker cancelled | the route row's Move | keyboard |
   | Move route dialog | Cancel; after Move, the moved-to contact's page title | keyboard |
   | Details: Add this conversation to {X} | the dialog closes; the conversation's header title | keyboard |
   | Undo of Details' Add this conversation to {X} | the conversation's header title | keyboard |
   | Undo of a Move | the moved routes' row on the page focus was on before the Move, recreated if needed | keyboard |
   | Discard pressed (R or K) | the dialog's Cancel | keyboard |
   | dialog cancelled | the Discard button | keyboard |
   | discard failed | the failure line (focusable) | keyboard |
   | after a discard, Chats opens | the notice banner | keyboard |

2. Sharing sends only on Share: opening a conversation, receiving a card,
   connecting and the offer itself send nothing (check: the peer's
   receiving client shows no card; the sender's conversation shows no
   own card row).
3. A channel's overflow has no Share a card; no chooser is reachable
   from a channel.
4. The chooser lists the last shared card first and chosen; with no last
   shared and two or more cards, none is chosen.
5. Two cards shared to two conversations: each counterparty's proposal
   shows only its own card's name, and no screen of theirs shows a label
   or a count of cards.
6. A proposal shows the claim, the full PeerId (copy gives the exact
   canonical form), "includes a photo" when one is carried, and the
   "goes at close" line; before Accept no decoded photo appears anywhere,
   and a test pins that the decoder is not called (rust-ui-dev).
7. Each case P, N, D, U, E and S of §5.4.1 shows its context line and Accept
   label, and Accept does what the table says; in case E a locally
   edited name is unchanged after Accept.
   Fixtures for D against a contact "Anna Rossi": a card "Anna" +
   "Rossi"; "Аnna" (Cyrillic А) + "Rossi"; a name "Anna Rossi" with
   U+00A0 between the words and no surname; "Ａnna" (fullwidth) +
   "Rossi" — each
   shows `CardSameName`. A card from a second endpoint of a contact's
   PeerId is case P and leaves one contact holding both routes. The
   Details of a conversation on {P, "phone"}, while {P, "human"} is
   under Carol, offers Add this conversation to Carol and no Add as a
   contact; the command-line client refuses a second contact for P. A
   card
   naming itself "Anna Rossi" from a new route of contact "Carol"'s
   PeerId (case P), and one from Carol's own route (case U), each show
   `CardSameName` naming the other "Anna Rossi" before Accept.
8. Delete contact Carol, then accept a card from one of Carol's
   PeerIds on a new route (case N): Delete's Undo is gone. Move a
   PeerId, then accept a card from it: the Move's Undo is gone.
   Accept then Undo leaves the store as before the Accept; Decline then
   restart leaves nothing of the card in the store; an unanswered,
   shown proposal is gone after a restart; an unshown one survives it.
9. The title and the list show the accepted name; two contacts with one
   name each show `{name} ({peer})`; the list preview and the
   notification of a card never contain the claimed name.
10. A right-to-left name inside the proposal heading and the same-name
    note does not reorder the surrounding words (a fixture with an
    Arabic name and a name containing U+202E, which must be refused as a
    format character, HUMAN-CHAT.md).
11. The photo sent is at most 256 × 256 and 24,576 bytes, and carries no
    metadata block (EXIF, XMP, text chunks), checked on the bytes a test
    captures before send; a JPEG fixture stored with Orientation 6
    arrives with its marker in the corner where a viewer that honours the
    tag shows it.
12. A card too large for a low-limit profile is offered "Share without
    the photo" and nothing is sent before the person presses it.
13. R shows Delete stored data… as a third action on both clients; K
    (Android, key invalidated) shows it as the primary with no Try
    again; both confirm before deleting.
14. The full-discard dialog lists exactly §7.2's items; with a complete
    `human.sqlite.encrypted` the partial dialog shows instead and the
    data is present after the next open; with that copy unopenable the
    full dialog shows and no second dialog follows.
15. In both dialogs Enter with the initial focus cancels; Escape/Back
    cancels.
16. After a discard, identity (PeerId) and trust are unchanged and Chats
    shows `StoreDiscarded` once.
17. Every control in this spec is labelled in the accessibility tree
    (desktop now; Android blocked on the bridge), and each swipe has its
    buttons.
18. §8's contrast table measured, every row at or above its threshold,
    in both schemes.

### 9.1 Proposed `human-client-ui.md` §13 criteria

For architect-cto to rule on (plan §29 step 4 names rust-ui-dev as
their proposer for the code's tests; these are the person-facing ones,
offered for the same list):

- a received card's name is never shown without the full PeerId beside
  it, and never in the conversation list, a notification or an
  announcement;
- no contact or route row exists before Accept; Decline and an
  unanswered card leave none;
- a received photo is not decoded before Accept;
- Accept's label and context line state what it will create or change
  before it runs; a same-name contact on another PeerId is named before
  acceptance;
- no card is sent except on Share in the chooser of one direct
  conversation; channels offer no Share;
- nothing a counterparty receives reveals another card's existence or
  label;
- a locally edited name or photo is unchanged by any later Accept;
- two contacts with one name are told apart in every title;
- every received name is rendered as isolated plain text;
- the sent photo carries no image metadata;
- Discard and recreate deletes nothing without a confirmation that
  lists what goes, its initial focus on Cancel, and identity and trust
  are unchanged after it;
- every action of this spec has a non-gesture control and an
  accessibility-tree label.

## 10. Strings

Every string a person reads here, for language-culture-en to write.
The Text column is a proposal that shows what the string must say;
the third column is what it must communicate and where. Key names are
suggestions; rust-ui-dev names them. Placeholders: `{name}` (a received
or saved name, plain text, isolated), `{title}` (a conversation's title),
`{label}` (the person's own card label; for a card without one, the
card's name, as §5.1's list shows it), `{peer}` (a short PeerId),
`{n}` (a number in digits), `{date}` (a date in the app's date format).
No string uses "keep" as an ordinary verb (Keep is a message action),
and no string calls a card an "identity" (§1).

| Key | Text (proposal) | Where, and what it must say |
|---|---|---|
| `ContactsTitle` | Contacts | menu item and page title |
| `OwnCardsTitle` | Your cards | menu item and page title |
| `OwnCardsIntro` | A card shows your name and photo to the people you share it with. Only you see its label. | §5.1, two lines |
| `OwnCardsEmpty` | You have no cards yet. When you share a card with someone, they see your name instead of a PeerId. | §5.1 empty |
| `AddCard` | Add a card | §5.1 primary |
| `CardMenu` | Options for {label} | a card row's menu button, accessible label |
| `Edit` | Edit | card row menu |
| `Delete` | Delete | card row menu |
| `ConfirmCardDeleted` | Card deleted | snackbar, with Undo |
| `CardNew` / `CardEdit` | New card / Edit card | editor title |
| `AddPhoto` / `ChangePhoto` / `RemovePhoto` | Add a photo / Change photo / Remove photo | editor and contact page |
| `CardName` | Name | editor field |
| `CardSurname` | Family name (optional) | editor field; must say optional |
| `CardLabel` | Label (optional) | editor field |
| `CardLabelHelp` | Only you see the label. It helps you tell your cards apart. | under the label field |
| `CardSharedStays` | People who already have this card still see it as it was. To show them your changes, share the card with them again. | editor note |
| `Save` | Save | editor, contact page |
| `CardNameNeeded` | Enter a name. | field error |
| `CardNameTooLong` | Use 64 characters or fewer. | field error; "characters" for scalar values |
| `CardNameBadCharacters` | Remove line breaks and invisible characters. | field error for Cc/Cf/separators |
| `CharCount` | {n} / 64 | field counter |
| `CardDiscardChanges` | Discard your changes? | unsaved-changes dialog |
| `KeepEditing` / `DiscardChanges` | Continue editing / Discard | its actions |
| `PhotoUnusable` | This photo cannot be used. Choose another one. | editor, under the circle |
| `ShareCard` | Share a card | conversation overflow, empty-conversation offer |
| `ShareCardTitle` | Share a card with {title} | chooser title |
| `ShareCardPreview` | What {title} will see | chooser preview heading |
| `ShareCardNote` | They will see this name and photo, and they can save them. You cannot remove a card from their device after it arrives. | chooser note |
| `ShareCardAction` | Share | chooser primary |
| `ShareCardChoose` | Choose a card to share. | chooser, Share with none chosen |
| `ShareCardNoCards` | You have no cards yet. Add a card, then share it. | chooser empty |
| `CreateCard` | Add a card | chooser empty primary |
| `ShareCardNoPhoto` | The photo is too large to send in this conversation. | chooser, over the limit |
| `ShareCardWithoutPhoto` | Share without the photo | chooser primary, over the limit |
| `ShareCardTooLarge` | This card is too large to send in this conversation. | chooser, nothing fits |
| `ShareCardOffer` | Let {title} see your name instead of a PeerId. | empty conversation |
| `CardProposal` | This peer says they are {name} | proposal heading; must keep "says": a claim, not a fact |
| `CardIncludesPhoto` | This card has a photo. You see it after you accept. | proposal |
| `CardPeerIdLabel` | PeerId | proposal, above the PeerId |
| `CopyPeerId` | Copy PeerId | existing (#313) |
| `CardGoesAtClose` | If you do not accept this card, it is removed when InterWeave stops. | proposal; Android's "closes" includes the system ending the app: language-culture to find the word that is true there |
| `Accept` / `Decline` | Accept / Decline | proposal actions, case N |
| `AcceptAsNew` | Accept as a new contact | case D |
| `UpdateContact` | Update contact | cases U, E |
| `ShareCardPreviewSpoken` | {title} will see the name {name}. | chooser preview, accessible description, no photo |
| `ShareCardPreviewSpokenPhoto` | {title} will see the name {name} and your photo. | the same, with the photo |
| `MoveRoutePickerTitle` | Move this PeerId to which contact? | the move-route picker's title. Every route of this row's PeerId moves together, not only this conversation (architect-cto's ruling of 2026-10-21); the text must say what the person picks a contact for, without promising less than moves |
| `CardPhotoLabel` | Photo for this card | editor photo circle, accessible label |
| `CardPhotoNone` | No photo for this card | the same, with no photo |
| `CardSamePeer` | Your contact {name} already has this PeerId in another conversation. | case P; `{name}` the contact's name |
| `AddToContact` | Add this conversation to {name} | case P's Accept, and the Details action of §5.6 |
| `CardSameName` | One of your contacts is named {name} and has a different PeerId. Compare the two PeerIds before you accept. | case D, and cases U, E and P when Accept would write a name that clashes (§5.4.1); `{name}` the other contact's name; must name both facts |
| `ShowContact` | Open your contact {name} | case D link |
| `CardUpdates` | If you accept, your contact {name} is updated. | case U |
| `CardUpdatesName` / `CardUpdatesPhoto` | New name: {name} / New photo | case U, the changes |
| `CardKeepsEdit` | You named this contact {name} yourself. That name stays. | case E, name |
| `CardKeepsPhoto` | You chose this contact's photo yourself. That photo stays. | case E, photo |
| `CardSame` | This card matches your contact {name}. | case S |
| `CardAccepted` | You saved {name} as a contact. | confirmation row |
| `CardRouteAdded` | You added this conversation to {name}. | confirmation row, case P when no value changed |
| `CardUpdated` | You updated your contact {name}. | confirmation row, cases U and E (language-culture-en's text) |
| `CardPhotoNotUsed` | The photo could not be shown and was not saved. | confirmation row addition |
| `ConfirmContactSaved` / `ConfirmContactUpdated` | Contact saved / Contact updated | snackbars, with Undo |
| `ConfirmCardDeclined` | Card declined | snackbar, with Undo |
| `CardPreviewReceived` / `CardPreviewSent` | Contact card / You shared a card | list previews |
| `CardNotification` | You received a contact card. | notification body; never the name |
| `AnnounceCard` | New contact card | live announcement |
| `CardSentLabel` | Your card: {label} | own card row |
| `AddAsContact` / `OpenContact` | Add as a contact / Open contact | Details |
| `DetailsLastCard` | The card you last shared here: {label} | Details |
| `DetailsNoCard` | You have not shared a card here. | Details |
| `DetailsCardDeleted` | You deleted the card you last shared here. | Details |
| `ContactsEmpty` | You have no contacts yet. Someone becomes a contact when you accept a card they share, or when you use Add as a contact in a conversation's Details. | §6.2 empty |
| `ContactRoutes` | {n} routes | contact row, n ≥ 2 (plural rules language-culture's) |
| `ContactTitleTwin` | {name} ({peer}) | title of a contact sharing a name |
| `ContactName` | Name | contact page field |
| `ContactNameFromCard` | Name from their card, {date} | provenance |
| `ContactNameTyped` | Name you entered | provenance |
| `ContactRouteTrusted` / `ContactRouteNotTrusted` | Trusted / Not trusted | route trust in words |
| `OpenConversation` | Open conversation | route row |
| `MoveRoute` | Move to another contact | route row |
| `MoveRouteConfirm` | Move every conversation with {peer} from {name} to {other}? Moving does not change whether you trust {peer}. | dialog; `{other}` the target contact's name. Must say every conversation with {peer} moves, not this one alone (architect-cto, 2026-10-21) |
| `MoveRouteLast` | {name} has no other conversations, so moving them deletes the contact {name}. | dialog addition, when {peer}'s routes are all {name} has; must say moving them deletes the contact |
| `ConfirmRouteMoved` | Conversations moved to {other} | snackbar after Move, with Undo |
| `Move` | Move | dialog primary |
| `DeleteContact` | Delete contact | contact overflow |
| `ConfirmContactDeleted` / `ConfirmPhotoRemoved` | Contact deleted / Photo removed | snackbars, with Undo |
| `StoreRecoveryTitle` | InterWeave cannot read its stored data | Android R heading |
| `RefusalStoreRecovery` (revised) | InterWeave cannot read its stored data: your messages, contacts and cards. Nothing was changed. The data opens only with its key on this device, so there is no copy of it that you can put back. You can try again, close InterWeave, or choose Delete stored data. InterWeave shows what you lose before it deletes anything. If a newer version of InterWeave wrote this data, install and start the newest version that you have used, or a newer one. Restoring your identity does not bring back this data. | desktop R body (Android R has its own, `StoreRecoveryBody`): its backup-copy sentence assumes a plaintext file a person can swap; the store is now encrypted and a copy opens only with its key. Must still say the file was not changed until the person chooses. Since architect-cto's A5 (§12) it must also say: the stored data opens only with its key on this device, and a copy without its key opens nothing, so there is no backup copy to put back; the way back is Try again, Close, or Delete stored data, which loses what it lists; the recovery phrase restores the identity alone, never this data. It must not say to replace the file with a backup, and must not suggest moving the key or the data between devices (ADR-0060 d. 8). |
| `StoreRecoveryBody` | InterWeave cannot read its stored data: your messages, contacts and cards. Nothing was changed. The data opens only with its key on this phone, so there is no copy of it that you can put back. You can try again, close InterWeave, or choose Delete stored data. InterWeave shows what you lose before it deletes anything. Restoring your identity does not bring back this data. | Android R body (new key): Android cannot install an older version over a newer one, so the desktop's newer-version advice is not true here, and Android shows no path. Since architect-cto's A5 (§12) it must also say: the stored data opens only with its key on this device, and a copy without its key opens nothing, so there is no backup copy to put back; the way back is Try again, Close, or Delete stored data, which loses what it lists; the recovery phrase restores the identity alone, never this data. It must not say to replace the file with a backup, and must not suggest moving the key or the data between devices (ADR-0060 d. 8). |
| `StoreKeyLostTitle` | The key to your stored data no longer works | K heading |
| `StoreKeyLostBody` | This phone no longer accepts the key that protects InterWeave's stored data. This can happen after some changes to the screen lock. The data cannot be read again. Your identity is not affected. | K body |
| `DiscardStore` | Delete stored data… | R, K action |
| `StoreChecking` | Checking the stored data… | §7.2 progress |
| `DiscardStoreTitle` | Delete all stored data? | full dialog title |
| `DiscardStoreLoses` | This deletes from this device: your contacts and their routes; messages waiting to be sent; unread and kept messages; your conversation list; your cards; the app's settings, other than your network settings. A message you already received may show again. | full dialog list; one item per line; must list exactly these |
| `DiscardStoreKeeps` | Your identity, your PeerId, the peers you trust and your network settings are not affected. | full dialog |
| `DiscardStoreRemains` | Earlier copies that your device made outside InterWeave may still be on it. | full dialog |
| `DiscardStoreConfirm` | Delete stored data | full dialog primary |
| `DiscardPartialTitle` | Delete the unfinished file? | partial dialog title |
| `DiscardPartialBody` | An update to how your data is protected was interrupted. A complete copy of your data exists. This deletes only the unfinished file, and InterWeave opens with your data. | partial dialog |
| `DiscardPartialConfirm` | Delete the unfinished file | partial dialog primary |
| `StoreDiscarding` | Deleting… | §7.3 progress |
| `StoreDiscarded` | The stored data was deleted. Your identity and the peers you trust did not change. | Chats notice after a full discard |
| `StoreRestored` | Your data was restored from the complete copy. | Chats notice after a partial discard |
| `StoreDiscardFailed` | The stored data could not be deleted. Try again. | R/K after a failure |

Existing keys reused unchanged: `AppOptions` (look spec §9; the Chats
⋮), `MoreOptions` (the conversation's ⋮ only), `BackToConversations`,
`Cancel`, `Close`, `Undo`, `TryAgain`, `Details`, `DirectTitle`,
`ShortPeer`, `StorageDegraded`.

## 11. Deliberately left out

- **Choosing the photo's crop**: a centred square is taken. A crop step
  is a later addition if people's photos are cut badly (hypothesis: most
  are portraits whose face is centred; falsified by a test where people
  reject the preview).
- **Contacts as a navigation destination** (bar or rail): reached from
  the menu until it is used often enough to earn the space.
- **Search or sort in Contacts and Your cards**, a contact's notes and
  device labels: not asked for; the store has the columns.
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
  only helps if the person noticed. Until the owner answers, the swipe
  is built as the owner's words give it.

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
  graphical clients and the command line. §6.3 keeps Move.
- **A5** — `RefusalStoreRecovery`'s new content is given in §10's row.
- **A6** (asked after round 2's review, QUESTION 01a15d68; answered
  in REPLY 01a15d6a) — may Move split one PeerId across two contacts?
  No. ADR-0059 d. 2 constrains it, so Move takes every route of the
  PeerId together (§6.3). The sentence goes into d. 4 on #335.
