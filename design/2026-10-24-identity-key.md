# An identity whose key cannot be used, and a new key — desktop and Android

Status: proposal, 2026-10-24. Its two product rules, what happens to
the refused file and when a missing file is a loss, are architect-cto's
DECISION 01a16d8a (§12), which amends ADR-0063 decision 1.

Revised 2026-11-22 for ADR-0067 decision 5 (architect-cto's REQUEST
01a202c7, job j159), in two parts. **Part 1, this revision, is the
interim for the first Stage 22 batch**: a key is the identity, since
the EndpointId derives from it, so no key can be made under an
identity that exists, and the build's store answers every new key
with a refusal. From the Stage 22 release no kind of §3's table offers
Make a new key; each keeps its explanation, and the screen says in
its place that this identity cannot get a new key (`KeyNoNewKey`). A
missing key file is always the "missing" state: the silent first key
is withdrawn (§7). The key folder is `endpoint-keys/` (ADR-0067
decision 3). **Part 2**, the replacement of such an identity by a new
one (plan §30 steps 1–3), is the new §5; its four questions are
ruled by architect-cto (REPLY 01a20525, §12), and two bounds remain
to propose with rust-ui-dev-02 (§12, Q-bound); it is built once an
identity can be created and removed, and until then part 1 stands.

What part 1 adds, changes and retires (§10):

- added: `KeyNoNewKey`;
- changed: `KeyNewerVersion` (no "if you make a new key"),
  `KeyCannotShare` and `KeyCannotShareHere` (no time when the key
  works again; language-culture-en's pass, 6257df7c8), `KeyDirUnsafe`
  (it asks for a folder that gives no permissions at all to any group
  or any other user, and it says InterWeave does not use any key kept there,
  not "the key", since the folder may hold none; 2026-11-23), `KeyNotNowAgain`
  (not "the key", for the same reason; 2026-11-23), and, in
  `design/2026-10-19-android-identity.md`, `IdR1NewKeys` (a new
  identity, not a changed key);
- retired from the build, with §5 (withdrawn): `KeyMakeNew`,
  `KeyNewTitle`, `KeyNewBody`, `KeyNewOld`, `KeyNewOldNewer`,
  `KeyNewPending`, `KeyNewConfirm`, `KeyMaking`, `KeyMade`,
  `KeyShareAgainTitle`, `KeyNewFailed`, `KeyNewFailedGone`.

What part 2 adds and changes (§10): added `KeyReplace`,
`KeyReplaceTitle`, `KeyReplaceBody`, `KeyReplaceRemoves`,
`KeyReplaceCarries`, `KeyReplaceKeepsList`, `KeyReplaceTurnsOff`,
`KeyReplaceOldNewer`, `KeyReplacePending`, `KeyReplaceConfirm`,
`KeyReplacing`, `KeyReplaced`, `KeyReplacedBy`, `KeyReplaceFailed`,
`KeyReplaceUnfinished`, `KeyReplaceTrustFull`, `KeyReplaceFinish`,
`KeyShareAgainDone`; brought back changed: `KeyShareAgainTitle`; reused
unchanged from `design/2026-11-04-identities.md` §7: `IdentityRemoveLoses`,
`IdentityRemoveKey`, `IdentityRemoveMayHaveReached`,
`IdentityRemoveOthersKeep`. The other retired keys stay retired.

Asked by rust-ui-dev-02 (REQUEST 01a16d84, job j51), for the two
criteria of `human-client-ui.md` §13 that have no person-facing design:

> a key file whose custody field says `filesystem-only`, opened where a
> wrapping key exists, is refused at open as a downgrade, named, the
> client offering to create a new key; a sealed record on a desktop is
> refused as unreadable here […]

> a device row: with the wrapping key removed (a Keystore
> invalidation), the client says the identity has no key and offers to
> create one, and after creation the counterparty sees a key change
> (ADR-0063 decisions 2 and 5)

Read here as: whenever an identity's key file is refused at open, for
any of ADR-0063 decision 2's reasons, the person is told which kind of
refusal it is, what it stops, and what they can do; and making a new
key is one confirmed action that says, before it is taken, that their
contacts will see a key change. (That second half held until the
Stage 22 release; from it no new key is made under an identity, see the
revision above.)

Owner of the design: ux-designer. The words are language-culture's:
§10 names each string, where it shows and what it must say, its text
language-culture-en's English. Built by rust-ui-dev
in `crates/human/` and the apps. It extends
`design/2026-10-21-contact-cards.md` (called "cards" below): the
identity's card page (cards §5.2), the list of identities (cards §5.1),
the share sheet (cards §5.3) and that note's §8 conventions (focus,
reading order, TalkBack's blocked precondition) apply unchanged unless
this note says otherwise.

## 1. What the key does today, and so what its loss stops

The identity key signs the identity's card and nothing else (ADR-0063
decision 5): it encrypts nothing, authenticates no message and grants
no trust. So an identity without a usable key can still send, receive
and read every conversation; what it cannot do is **share its card**.
That is the only thing this state blocks, and the design says so,
plainly and without alarm: a person who reads "your key cannot be used"
must not think their messages stopped or became unsafe.

A new key matters to other people. Until the Stage 22 release, a
contact who accepted this identity's card held its old key; the next
card they received carried the new one, and their client showed it as
a key change — "this might be someone else" (`CardKeyChange`).

From the Stage 22 release (ADR-0067 decision 5) the key is the
identity: the identity's EndpointId derives from its public key, so a
different key is a different identity, which a contact sees as another
identity of this person's device, never as a changed key. A key that
cannot be used is therefore never replaced under the same identity.
The way out is a new identity, whose card the person shares again
(§5, part 2); until that is built, the state offers nothing beyond
Try again where Try again can help (§3).

## 2. The person's goal

To share their card again — or to understand why they cannot, and
whether doing something about it costs anything. They did not cause
the state and may not know a key exists; most never will, until this.

## 3. The state: a key that cannot be used

The client checks an identity's key when it opens it: at start for the
identity in front, and for every held identity when the list of
identities (cards §5.1) is shown or the identity comes in front.
Opening asks the person nothing (ADR-0063 d. 2: the Android wrapping
key is `background-compatible`). A refusal puts that identity in this
state until a key opens. Nothing is shown before a
refusal: an identity whose key opens has no line about it.

Each refusal is named by its kind, in the person's terms, never by the
error's name. A refusal comes from one of two places: the file read
(`IdentityKeyFileError::Persist`, from `read_private_within`, before
the record is parsed) or the record (`IdentityKeyError`); both are
mapped here, and nothing else refuses at open:

| kind | refusal | platform | reason line | actions |
|---|---|---|---|---|
| no longer usable here | `Cipher(KeyInvalidated)`, `Cipher(KeyUnavailable)` | Android | `KeyLostDevice` | none; `KeyNoNewKey` |
| protected less than this device requires | `CustodyDowngrade` | Android (any platform with a wrapping key) | `KeyBelowDevice` | none; `KeyNoNewKey` |
| sealed by another device | `SealedUnreadableHere` | desktop | `KeyOtherDevice` | none; `KeyNoNewKey` |
| possibly a newer version's | `UnsupportedVersion(v)` with `v` above this build's version; `UnknownCustody` | both | `KeyNewerVersion` | none; `KeyNoNewKey` (the reason line names the update) |
| damaged | `NotARecord`, `Truncated`, `UnsupportedVersion(v)` with `v` below this build's version, `EndpointMismatch`, `KeyMismatch`, `Cipher(Unauthentic)`; the read's `TooLarge`, and its `Read` with `InvalidData` (bytes that are not text) or a permission refusal while `endpoint-keys/` is a directory this user owns (a file its owner cannot read); and any refusal of a key path that holds neither a regular file nor a link (a directory, a FIFO, a socket or a device file in the key file's place), whoever owns it (Q-kind, §12). Which refusal that is depends on the entry: one that this user can open is refused by its kind as `NotPrivate`, before the read looks at the owner or the mode; one whose own mode refuses this user is a permission refusal, as above; and a socket, or a device file with no driver behind it, fails the open itself (`ENXIO` or `ENODEV`), which the client must tell apart from the I/O errors of "temporarily unusable" | both | `KeyDamaged` | none; `KeyNoNewKey` |
| not protected | the read's `NotPrivate` for the file (a regular file readable or writable by another user, a link, another user's regular file), and the read reaches the file only after `endpoint-keys/` is private; while the folder is loose, the kind is "key folder not safe" (below) | both (in practice the desktop) | `KeyExposed` | none; `KeyNoNewKey` |
| key folder not safe | the read's `NotPrivate` for an `endpoint-keys/` that is a link or not a directory at all; and any read refusal while `endpoint-keys/` is a directory another user owns, which the client checks itself, since the read judges the directory's mode and not its owner (a root-owned 0700 directory reads as a permission refusal); and the read's `NotPrivate` for an `endpoint-keys/` directory that this user owns and whose mode is loose (any permission for its group or for other users, 0711 and 0701 among them, though no other user can list a folder at either mode), whatever files it holds. The read stops at the directory and never reaches the file, so the person fixes the folder first. Try again then shows the file's own state: the key opens, or the page shows the file's kind ("not protected", "missing" or any other) (Q-loose, REPLY 01a207c7: a private key opens again once the folder is private, and nothing outlasts the fix) | desktop (Android's is the app's own) | `KeyDirUnsafe` | Try again, after the person fixes the folder |
| temporarily unusable | `Cipher(Platform)`; the read's `Read` for any I/O error but those in the "damaged" row and an other-owner directory | both | `KeyNotNow` | Try again |
| missing | no file (§7) | both | `KeyMissing` | none; `KeyNoNewKey` |

The client must tell these read refusals apart: today `open` turns
every read error into one string (`IdentityKeyFileError::Persist`),
which cannot carry this mapping; the error's shape is rust-ui-dev's.
A refusal that persists on every attempt is never "temporarily
unusable", since Try again would repeat it for ever.

`Random` is not an open refusal: it can only fail the making of a key
(§5.4, failed). `UnknownCustody` carries no byte, so the client cannot tell a
future custody from a damaged one; the "newer version" line says only
that a newer version *may* read it, which is true of both. A version
below this build's was never written by any release, so it is damage.
"Not protected" offers no way to tighten the file's permissions: the
key may already have been read, so it is not used again (architect-cto's,
§12); from the Stage 22 release only a new identity (§5) shares
a card again. "Not used again" applies when the client refuses the
key file itself, not its folder: a loose directory this user owns,
over a private file, never lets another user read the file. It lets
other users list the folder's entries (each file's name, kind and
inode number), if it gives them the read bit, and see what `stat`
says of each file whose name they know (its size, times, owner, mode
and the rest of its metadata, never its contents), if it gives them
the search bit. With the search bit alone (0711, 0701), they can list
nothing, but the names are EndpointIds, which are public, so they can
still see each file's metadata; none of it is key material. If the directory gives another user both the write bit and
the search bit, a file that was swapped is refused at open by the
checks on the file itself, and a file that was removed is "missing";
the write bit without the search bit (0720, 0702) lets another user
neither swap a file nor remove one. The person sees
each result only after the folder is private (REPLY 01a207c7;
ADR-0063, second amendment of 2026-11-23). A private key
file in a loose directory opens after the folder is made private,
never while the folder is loose. When the folder is private, a key
file that is not private is refused by its own check, and is not
used again (Q-loose, REPLY 01a207c7).

Every reason line is followed by the same consequence line,
`KeyCannotShare`: messages are not affected, and its card cannot be
shared; it names no time when the key works again, since for most
kinds it never does.

For every kind but "temporarily unusable" and "key folder not safe",
whose Try again can still open the key, `KeyNoNewKey` follows
`KeyCannotShare`: InterWeave cannot give this identity a new key, so
the person is not left looking for a control that does not exist.
It promises no later version and names no implementation term (no
"EndpointId", no "derived"). For "possibly a newer version's" the
reason line has already named the one way the key may come back, an
update; `KeyNoNewKey` follows it all the same, since nothing else
here can help. Part 2 adds the offer of a new identity after it
(§5.1).

## 4. Where the state is shown

Three places, and no others: not on Chats, not as a notification, not
in a conversation. The one thing it blocks is reached through these.

### 4.1 The identity's card page (cards §5.2)

Where the page shows the identity's fingerprint (`human-client-ui.md`
§13: beside the PeerId), it shows instead a block:

- the heading `KeyStateTitle` ("This identity's key cannot be used"),
  with the warning icon before it, in `error`;
- the reason line of §3's table, then `KeyCannotShare`, then, for
  every kind §3 gives it, `KeyNoNewKey`;
- for "temporarily unusable": **Try again** (`TryAgain`, filled
  tonal) alone;
- for "key folder not safe": the reason line is `KeyDirUnsafe`, and
  under it, on a line of its own, `KeyDirUnsafePath` ("Folder
  location: {path}"), the path isolated (left to right, never translated),
  selectable and copyable as `human-client-ui.md` §11 asks of an exact
  identifier; the path appears nowhere but under `KeyDirUnsafe`,
  wherever that line shows (here and in the share sheet, §4.3). Then **Try again** (`TryAgain`,
  filled tonal) alone. Try again opens the key once more, as in §6, and whatever
  the folder then holds decides the state: the old file opens, or is
  refused by its own kind, or is missing (and §7 applies); the person
  is not told to keep or move the files;
- for every other kind: no action. The block ends with `KeyNoNewKey`,
  and the page's own controls (Save, Back) follow it. Until the
  Stage 22 release these kinds offered **Make a new key**
  (`KeyMakeNew`); from it none does, on either platform (ADR-0067
  decision 5).

The fields stay editable and **Save works**: the card is signed when it
is shared, not when it is saved, so editing it needs no key. (The build
of 2026-10-24 refuses Save with `NoIdentityKey`; that refusal goes.)

### 4.2 The list of identities (cards §5.1)

The identity's row gains a third line, `KeyStateRow` ("Key cannot be
used"), after the warning icon, in `error`. The icon and the words
carry it; the colour only repeats them. The row opens §4.1 as before.

With one identity there is no list: the menu's Your card opens §4.1
directly, so the state is one tap from the menu either way.

### 4.3 The share sheet (cards §5.3)

Opened while the conversation's identity is in this state, the sheet
keeps its title and its "as {identity}" line, and replaces the preview
and `ShareCardNote` with the reason line (for "key folder not safe",
`KeyDirUnsafe` with `KeyDirUnsafePath` under it, as in §4.1, since
the reason refers to the location shown) and `KeyCannotShareHere`
("You cannot share your card without this identity's key").
The primary becomes **Open your card** (`KeyOpenCard`), which opens
§4.1 for this identity; Cancel stays; Edit your card is not offered
(it is the same page). Share is never shown disabled, as cards §5.3
keeps for every state.

This state wins over cards §5.3's "no name yet": with neither a key
nor a name, the sheet shows this state, since Open your card leads to
the page where the name is added too, and a name alone would not make
the card shareable.

From §4.1, after Try again opened the key (§6), Back returns to the
sheet, which then shows the preview and Share as usual; in every other
case it returns to the sheet in this state. The sheet does not show
`KeyNoNewKey`: Open your card leads to the page that says it. The offer in an empty
conversation (cards §5.3) stays: it opens this sheet.

## 5. Replace with a new identity (part 2)

Status: part 2, 2026-11-22. Q-link, Q-list, Q-trust and Q-carry (§12)
are ruled by architect-cto (REPLY 01a20525), all four yes with
conditions, written below; the store's text for the link and the
list, and the ADR amendments, are architect-cto's supply on the j185
batch, not this note's. The list's bound and what the step does at
the trust overlay's row bound wait for Q-bound (§12), which names the
acceptance items it blocks. Part 2 is built with plan §30 steps 1–3,
once an identity can be created and removed through the administrative boundary (ADR-0061
decisions 2 and 7). Until then, part 1's interim stands: `KeyNoNewKey`
and no action (§3, §4.1). From the Stage 22 release this replaces the
Stage 21 "Make a new key", which git history keeps (ADR-0067
decision 5: a new key is a new identity).

The person's goal is unchanged (§2): to share a card again. Under
ADR-0067 that means a new identity, which contacts see as a new
contact, so the flow has to say four things before it is confirmed:
who gets what, that messages to the old identity stop reaching them,
what the new identity takes from the old one (the people it trusts
and a copy of its contacts), and what happens to the old identity.
The rulings' common reason (REPLY 01a20525): a replacement is the
person's own confirmed act saying the new identity takes the place of
the old one, so what the old one held about other people may follow where the
confirmation says so; nothing about the old identity itself follows,
not its key, its EndpointId, its route or its messages (ADR-0061
decision 9).

### 5.1 Where it is offered

On §4.1's block, after `KeyNoNewKey`: **Replace with a new identity**
(`KeyReplace`), filled tonal (Save stays the page's one filled
button), for every kind that shows `KeyNoNewKey`. For "temporarily
unusable" it is a text button after Try again, and `KeyNoNewKey` is
not shown: Try again comes first, as the Stage 21 design had it. It
is never offered for "key folder not safe", where the person fixes the
folder and the key can open again (§4.1); a loose `endpoint-keys/`
that this user owns is of that kind, whatever files it holds
(Q-loose, §3). A new key cannot be minted into a loose folder
(Q-dir), and Replace does not change the folder's mode: the page
names the folder and the person makes it private, as for every other
folder that is not safe. When the person then chooses Try again, the
page shows the file's own state, and for every kind that shows
`KeyNoNewKey` ("not protected" and "missing" among them) it offers
Replace.

It is offered nowhere else: not in the share sheet (§4.3, whose Open
your card leads here), not in the list of identities (§4.2's row opens
this page). It replaces part 1's no-action state and changes nothing
else in §3 or §4.

### 5.2 The confirmation

**Replace with a new identity** opens a dialog (Material's basic
dialog; on Android `AlertDialog`'s placement). In order:

- title `KeyReplaceTitle`: "Replace {identity} with a new identity?";
- `KeyReplaceBody`, one paragraph, every case: InterWeave makes a new
  identity with this card's name and photo; to your contacts the new
  identity is a different identity from {identity}, and they get the new identity's card only when
  you share that card with them; messages they send to {identity}
  will no longer reach you.
  It says nothing of when contacts can see the new identity itself: a
  trusted peer's directory query shows the new endpoint once it is
  advertised, before any card is shared (REPLY 01a20525, Q-trust), so
  "only after a share" is true of the card and of nothing else;
- `KeyReplaceCarries`, every case: the new identity trusts the people
  that {identity} trusts and gets a copy of the contacts of
  {identity}; nothing is sent to these people (Q-trust, Q-carry). The copy admits nobody new to the node:
  those peers are already in its admission set;
- what happens to the old identity, one of:
  - an added identity: `KeyReplaceRemoves` ("{identity} is then
    removed:"), followed by identities §7's Remove list as it stands
    there — `IdentityRemoveLoses`, `IdentityRemoveKey`,
    `IdentityRemoveMayHaveReached` when its count is not zero, and
    `IdentityRemoveOthersKeep` — so a removal by replacement says
    exactly what a removal by Remove says; then
    `KeyReplaceKeepsList`: what stays, the list of whom to share the
    new card with (§5.4). That list is the one thing of a removed
    identity that outlives its Remove (Q-list), so it is said beside
    what goes;
  - the first identity: `KeyReplaceTurnsOff`: it is turned off; its
    conversations and contacts stay, to read; it cannot send or
    receive (identities §7's Turn off);
- `KeyReplaceOldNewer`, only for "possibly a newer version's": a newer
  version may be able to use this identity's key; to keep this
  identity, update InterWeave first;
- `KeyReplacePending`, only when the old identity has own-card rows not
  yet Accepted by remote transport: those still held are cancelled,
  and one already on its way may still arrive, as the old identity's
  card (§5.3, step 3);
- actions **Cancel** (`Cancel`) and **Replace** (`KeyReplaceConfirm`).
  Replace is filled in `error` for an added identity (its
  conversations and contacts are deleted, with no Undo) and for
  "possibly a newer version's" (the person may lose an identity an
  update could keep); otherwise filled, not in `error`, since nothing
  is deleted and the cost is said above.

Focus opens on Cancel, so Enter alone never replaces. Back, Escape
and a tap on the scrim cancel. There is no Undo after Replace: the
removal has none (identities §7), and a new identity whose card may
already have been shared cannot be taken back.

### 5.3 Doing it

Confirm: the dialog closes, and the block shows `KeyReplacing`
("Replacing {identity}…") with an indeterminate progress indicator in
place of its actions, for as long as it takes. It is not cancellable:
a half-done replacement is the one outcome the step must not leave
unsaid. The rest of the page stays usable, but Remove and Turn off are
not offered while it runs.

The step, in this order:

1. **The new identity.** Created as plan §30 step 1 creates one (on
   Android, step 3): a new key, a new EndpointId, a new alias chosen as
   Add an identity chooses one, and a card with the old card's name
   and photo, not yet shared with anyone. The record that the new
   identity replaces the old one (Q-link, the store's) is written
   **before** the administrative add, not with it: the client mints
   the key, so it knows the new EndpointId first, and a kill between
   the add and a later write would leave an added identity with no
   record, which the next Replace would turn into a third identity.
   Nothing of the old identity is touched before this step has stored
   the new one, so a failure here leaves the old identity exactly as
   it was.
2. **What the new identity takes.** After the add and before step 5,
   in this order: the share-again list (§5.4) is stored; the old
   identity's contacts are copied as rows of the new identity, routes,
   name and photo as accepted (Q-carry); and the old identity's
   trusted set is written as the new identity's, as trust acts of the
   person under the new identity, through the endpoint dimension
   (Q-trust; the add itself still writes the set empty, ADR-0067
   decision 6). None of these writes sends anything. The old identity's own rows then go or stay
   with it in step 5. The trust overlay's row bound can refuse a write
   while both sets exist; the step then stops there, the list and the
   contacts written and the trusted set partly copied, and the person
   finishes it with Finish replacing (below, and Q-bound for what is
   said when it refuses again).
3. **The old identity's pending cards.** As in the Stage 21 design,
   and for the same reason (a card of an identity about to go): the
   step closes a Cancel snackbar of any of the old identity's card
   rows, if one is open, and its Undo with it, then withdraws every
   own-card row of the old identity that the client still holds (Not
   sent, and Sending not yet handed to the transport; where the client
   cannot tell the two Sending kinds apart, it treats every Sending row
   as taken), with no Undo. A row the transport took may still arrive;
   it carries the old identity's card, whose key still matches the old
   identity, so the counterparty sees the old contact, and its
   conversation is on the share-again list (§5.4). A Cancel of one of
   these rows answered while the step runs is announced once, with
   message-swipe §8's words and no snackbar or Undo, after the step's
   own announcement and never over it; a refusal of a row the step
   withdrew is not announced and does not return the row, since the
   row is already gone. A refusal of a row the step left applies as
   message-swipe §8 says, its announcement also after the step's own.
4. **The default.** The new identity becomes the default when the old
   one was the default, and always when the old one is the first
   identity (ADR-0067 decision 5).
5. **The old identity.** An added identity is removed, as identities
   §7's Remove removes one, and the record of step 1 is deleted with
   it, so no record of a removal survives (ADR-0061 decision 7, A
   2026-11-04); only the share-again list, owned by the new identity,
   outlives it (§5.4). The first identity is turned off, as identities
   §7's Turn off turns it off; the record is kept while both exist, and
   goes if the new identity is ever removed.

**Every point the step can stop at resolves to a state the person
already knows**:

- stopped before step 1 has stored the new identity: the old identity
  is in this state as before, unchanged; after a failure (not a kill)
  `KeyReplaceFailed` shows under its actions, and Replace is offered
  again. A record of step 1 whose new identity was never added names
  no identity the node holds; the client discards it at the next open
  and shows this case, so a record alone never shows as unfinished.
- stopped after step 1, before step 5 is done: both identities exist.
  The old identity's page shows `KeyReplaceUnfinished` ("{new} was
  made to replace this identity, but the replacement did not
  finish.") and,
  in place of Replace, **Finish replacing** (`KeyReplaceFinish`),
  which runs steps 2 to 5 only, without a dialog (the person
  confirmed them), and never creates another identity. This needs the
  record of step 1 (Q-link). Without that record, a second Replace
  would make a third identity, which this design does not accept.
  A refusal anywhere in steps 2 to 5 leaves this state too, never
  `KeyReplaceFailed`, whose words are true only before step 1 has
  stored the new identity. Step 2 resumes where it stopped: a trusted
  peer or a contact already copied is not written twice. When step 2
  stopped at the trust overlay's row bound, the page also shows `KeyReplaceTrustFull`
  under `KeyReplaceUnfinished`, and Finish replacing is offered again
  (Q-bound).
- after step 5: the result (§5.4).

The client never shows "replaced" before step 5 is done, and never
shows the new identity before it is stored where the next start will
open it.

### 5.4 The result

**Replaced.** The client switches to the new identity visibly, as
cards §5.7.5's transition does ("Opening as {identity}"), and shows
the new identity's card page: its fingerprint row, and under it
`KeyReplaced` ("{new} has replaced {old}. Your contacts get the card
of {new} only when you share that card with them."). The line is that session's: it is
not stored, and after an added identity's step 5 nothing stored names
the old one (REPLY 01a20525, Q-link), so once the page is left, or
the app restarts, it does not show again; the list below does.

Then, when any conversation of the old identity holds or may hold its
card — `card_shared_at` set, an own-card row not yet Accepted by remote
transport when the step ran (withdrawn or on its way), or a card that
may have left this device (ADR-0063 decision 1, A 2026-10-27), read
over the old EndpointId's conversations (plan §30 step 2) —
`KeyShareAgainTitle` ("Share your new card in these conversations")
and one row per such conversation, most recently shared first. A row's
title is the new identity's contact for that route (copied in step 2),
else the route as the client shows a route; the list stores no title
and no message. A row opens the new identity's conversation
with that conversation's remote route (a new one if it has none) with
the share sheet open (cards §5.3). The list answers "who should hear
from me": those, and nobody else.

The list is read over the old identity's conversations and stored
in step 2, before step 5 deletes them: remote routes only, `{peer,
endpoint}` with their order, owned by the new identity (Q-list, the
store's). A row leaves when it has been shared from; all leave when
the person closes the list with **Done** (`KeyShareAgainDone`); all go
with the new identity's Remove. Until then it is shown on the new
identity's card page, across leaving the page and a restart. It cannot
be a list shown only until the page is left: for an added identity the
conversations it is read from are deleted in step 5, so leaving the
page would lose it for good. The list's bound is Q-bound (§12).

Because the new identity trusts the old one's peers and holds a copy
of the old one's contacts (step 2), a row opens a conversation with a
named contact, and the counterparty's reply to the shared card is
admitted. The counterparty still sees a new identity, and still has
their contact for the old identity until they accept the new card.

**Failed** (step 1 refused — a key that cannot be minted or sealed, the
add refused): `KeyReplaceFailed` ("InterWeave could not make the new
identity. {identity} has not changed. Choose Replace with a new
identity to try again."), in `error`, under the actions, and Replace
offered again.

The old identity, after a replacement:

- an added identity: gone, from every list and every screen, as after
  Remove;
- the first identity: listed as off (identities §7), its conversations
  and contacts readable under it, its page still showing this state's
  block (§4.1) without Replace, since it has been replaced: its
  `KeyReplaced` line reads from the old side as `KeyReplacedBy` ("{new}
  has replaced this identity."), read from the record of step 1, which is
  kept while both exist. If the new identity is later removed, the
  record goes with it, and the first identity's page shows this
  state's block with Replace again and no `KeyReplacedBy`.

## 6. Try again ("temporarily unusable", "key folder not safe")

Try again opens the key once more. Opened: the block is replaced by the
fingerprint row, nothing announced beyond §8. Refused again with the
same kind: `KeyNotNowAgain` ("InterWeave still could not use this identity's
key.", §10) under the
actions. The actions stay as they were: for "temporarily unusable"
with part 2 built, Try again and Replace (§5.1), and in every other
case Try again alone. Refused again with another kind: that kind's
block, with `KeyNoNewKey` and Replace where §3 and §5.1 give them, and
focus on its reason line (§8, "Try again, refused with another kind").
The same holds for Try again under "key folder not safe" (§4.1).

## 7. Next to other states

- **The store's key** (cards §7): when the store cannot be opened, its
  screen comes first and nothing of this note shows until the store is
  open. A Keystore change that invalidates the store's key may
  invalidate identity keys too; after the store's recovery, each such
  identity is in this state, with its own line.
- **A missing key file** is always the "missing" state, for every
  identity, from the Stage 22 release (ADR-0067 decision 5): a key
  made at open would name an id the configuration does not, so the
  silent first key (Q-missing, architect-cto's; ADR-0063 A 2026-10-24,
  2026-10-27) is withdrawn. An identity's key is made only where the
  identity is created: the desktop's by `transportctl endpoint init`
  or `adopt`, Android's at onboarding (ADR-0067 decision 4 as amended
  on architect-cto's branch, 840747b85, not yet on main). The same
  holds when the whole `endpoint-keys/` directory is missing outside
  the recovery flow. Until the Stage 22 release a key was made
  silently for an identity whose card no conversation held or might
  hold; §5.4's share-again predicate was the test.
- **Restore on fresh state** (identity note `IdR1NewKeys`): the person
  is told before the phrase that the words restore the PeerId, that
  each identity's key is gone, and that each identity is replaced by a
  new one, which contacts see as a new identity. On Android the first
  identity's key is minted at Restore (ADR-0067 A 2026-11-21, on
  architect-cto's branch, 840747b85, not yet on main), so it
  is a new identity and shows no state here. On a desktop state root
  whose configuration still names the key files, each such identity is
  in the "missing" state with `KeyNoNewKey` until it is replaced (§5)
  (`human-client-ui.md` §13).
- **Remove an identity** (cards §5.2) is unchanged and needs no key; its
  confirmation still names the key as going with it.
- **The command line**: the flag for a key change (ADR-0063 d. 6) is
  withdrawn with the key change itself (ADR-0067 decision 5); the
  command line makes no key under an existing identity either.

## 8. Accessibility

**Reading order**, §4.1's block: the heading (read with "warning"), the
reason line (for "key folder not safe", `KeyDirUnsafe`, then
`KeyDirUnsafePath` read as one item), `KeyCannotShare`, `KeyNoNewKey`
where §3 gives it, then the actions, if any; it sits where the
fingerprint row would, so it is read in the page's order. §4.2's row
reads its third line with the row: "{name}, {route label}, key cannot
be used". §4.3's sheet: title, "as {identity}", the reason line (with
`KeyDirUnsafePath` after it, as one item, for "key folder not safe"),
`KeyCannotShareHere`, Cancel, Open your card.

**Focus.** Every case, with its check:

| case | focus goes to | check |
|---|---|---|
| §4.1 opened | the page title (focusable heading), as cards §9's table for Your card | keyboard |
| the share sheet opens in this state | Open your card (cards §9's "Share, else Add your name" has neither here) | keyboard |
| §4.1, a kind with `KeyNoNewKey` | the page title, as above; `KeyNoNewKey` is text, read in order, not a stop of its own; Tab from the block goes to Replace with a new identity where §5.1 offers it, otherwise to the page's next control | keyboard; desktop screen reader reads it once, after `KeyCannotShare` |
| Replace with a new identity pressed | the dialog's Cancel | keyboard; screen reader reads the title, then the body in §5.2's order |
| the dialog cancelled (Cancel, Back, Escape, scrim) | Replace with a new identity | keyboard |
| Replace confirmed, while `KeyReplacing` shows | the block's progress line (the button that held focus is gone) | keyboard; screen reader announces `KeyReplacing` once |
| replaced | after the switch to the new identity (cards §5.7.5), the `KeyReplaced` line on its card page, which is announced by being focused, not again | keyboard; screen reader |
| the replacement failed | `KeyReplaceFailed`, announced by being focused, not again | keyboard; screen reader |
| the old identity's page with `KeyReplaceUnfinished` opened | the page title, as §4.1; Tab reaches Finish replacing after `KeyReplaceUnfinished` | keyboard |
| Finish replacing pressed | the progress line, then as "replaced", or as one of the two refusals below | keyboard; screen reader |
| steps 2 to 5 refused for another reason, under Replace or Finish replacing | `KeyReplaceUnfinished`, announced by being focused, not again; Tab reaches Finish replacing after it | keyboard; screen reader |
| step 2 stopped at the trust overlay's row bound (§5.3), under Replace or Finish replacing | `KeyReplaceTrustFull`, announced by being focused, not again; Tab reaches Finish replacing after it | keyboard; screen reader |
| a row under `KeyShareAgainTitle` | the conversation's share sheet, as cards §5.3 opens it | keyboard |
| back on the new identity's card page from a row's conversation, the card shared | the next row of the list, else Done | keyboard |
| back on the new identity's card page from a row's conversation, not shared | the row that opened it | keyboard |
| Done under the list | the new identity's fingerprint row (the list and Done are gone) | keyboard |
| the page goes to the background during `KeyReplacing` | on return, as "replaced", "failed" or unfinished by what the step left; the announcement is made on return, once | keyboard; screen reader |
| a card row's Cancel answered while the step runs (§5.3 step 3) | stays where it is; a success is announced once after the step's own announcement | keyboard; desktop screen reader |
| Try again, opened | the fingerprint row | keyboard |
| the `KeyDirUnsafePath` line ("key folder not safe"), on the page | reached by keyboard in reading order before Try again, so the path can be selected and copied; it opens nothing | keyboard; screen reader reads it once |
| the `KeyDirUnsafePath` line, in the share sheet | reached by keyboard in reading order before `KeyCannotShareHere` and the actions, so backwards from Open your card, where the sheet's focus opens; selectable and copyable, it opens nothing | keyboard; screen reader reads it once, with the reason line |
| Try again, refused again | `KeyNotNowAgain` | keyboard; screen reader |
| Try again, refused with another kind (§6; the Try again button may be gone) | the new kind's reason line (for "key folder not safe", `KeyDirUnsafe`, with `KeyDirUnsafePath` next in reading order), announced by being focused, not again; Tab then reaches the kind's action (for "key folder not safe", after the `KeyDirUnsafePath` line): Try again where §3 keeps it, else Replace with a new identity where §5.1 offers it, else the page's next control | keyboard; screen reader |
| Open your card from the sheet | as cards §5.2 | keyboard |
| Back from §4.1 to the sheet, Try again opened the key | the sheet's Share | keyboard |
| Back from §4.1 to the sheet, no key | Open your card | keyboard |

The Stage 21 rows for Make a new key, its dialog, `KeyMaking` and the
made and failed results are withdrawn with it; the rows above for
Replace take their place.

On Android every screen-reader check is a blocked precondition while
TalkBack does not reach the app (cards §8); the keyboard checks run.

**Not by colour alone**: the warning icon and the words carry the
state; `error` repeats them. The icon's colour and the heading in
`error` hold 4.5:1 on every surface the block can sit on — the page's
`surface` and, in the sheet, the sheet's container — in light and
dark.

**Targets**: Try again, Replace with a new identity, Finish replacing,
each row under `KeyShareAgainTitle` and Done are at least 48 dp tall
on Android and 24 px on the desktop.

**Motion**: the progress indicator (§5.3) and the switch's transition
(cards §5.7.5) are the platform's; with reduced motion each is the
platform's reduced form.

## 9. Acceptance

On both clients unless said; Android's screen-reader checks blocked as
§8 says.

1. Android, a device row (`human-client-ui.md` §13): remove the
   identity's wrapping key from the Keystore, start the app: the
   identity's page shows `KeyStateTitle`, `KeyLostDevice` and
   `KeyCannotShare` and `KeyNoNewKey`, and no action; conversations
   send and receive as before.
2. Android: a `filesystem-only` key file in the identity's place shows
   `KeyBelowDevice`; desktop: a Keystore-sealed file shows
   `KeyOtherDevice`; on both, a file of another identity, a version
   byte below this build's, and a file over the record bound each show
   `KeyDamaged`; a version byte above this build's, and an unknown
   custody byte, show `KeyNewerVersion`; desktop: the key file made
   readable by the group shows `KeyExposed`.
3. With two identities, the list's row of the one in this state shows
   `KeyStateRow` with the icon, and only that row.
4. Share in a conversation of that identity shows the sheet with the
   reason line, `KeyCannotShareHere` and Open your card, focus on Open
   your card; no Share control; nothing is sent. With the card's name
   also empty, the sheet shows the same, not Add your name.
5. On the page, Save of a changed name works in this state.
6. Withdrawn (ADR-0067 decision 5): Make a new key and its dialog.
   In its place: for every kind but "temporarily unusable" and "key
   folder not safe", the page shows `KeyNoNewKey` after
   `KeyCannotShare` and no control in the block, on both clients.
7. Withdrawn (ADR-0067 decision 5): the counterparty's key change. A
   different key is another identity; item 25 checks what the
   counterparty sees.
8. Withdrawn with the Stage 21 step (its cut points); item 24 checks
   the replacement's.
9. Withdrawn with §5.3 (a failed new key).
10. A key that cannot be opened transiently (a test seam returning a
    platform cipher error once): `KeyNotNow` and Try again; Try again
    opens it and shows the fingerprint.
11. An identity's key file removed starts in the "missing" state
    with `KeyNoNewKey`, and no key is made by itself, whether or not
    the identity ever shared its card, has an own-card row pending or
    has neither; with the whole `endpoint-keys/` directory removed,
    every identity does the same. A restart shows the same state, and
    no file appears in `endpoint-keys/`.
12. Focus follows §8's table, row by row, by keyboard; on the desktop,
    the screen reader reads §8's order and each announcement once.
13. Withdrawn with the Stage 21 step (pending own cards and the Cancel
    snackbar at a new key); item 26 checks the replacement's.
14. A key file of non-text bytes, and one of mode 0000, each show
    `KeyDamaged`, not `KeyNotNow`. Desktop: in a private
    `endpoint-keys/`, a directory of mode 0700 in the key file's place,
    a FIFO of mode 0600 in the same place, and a Unix socket of mode
    0600 there, each owned by this user, each show `KeyDamaged`, not
    `KeyExposed` and not `KeyNotNow`; the open does not wait for the
    FIFO. A device file is not checked: making one needs a privilege
    or a mount that allows devices, which a test run does not have.
15. Desktop, in three runs: `endpoint-keys/` at mode 0755 over a 0600
    key file, over a 0644 key file, and with no key file. Each run
    shows `KeyDirUnsafe` with the path and Try again, no `KeyNoNewKey`
    and no Replace, and the directory's mode is not changed. In the
    run with no key file, Try again pressed before the person makes
    the directory 0700 shows `KeyNotNowAgain` under Try again, and
    focus is on `KeyNotNowAgain` (§8, "Try again, refused again"); the
    directory's mode is still not changed. After the person makes the
    directory 0700, Try again has these results: in the 0600 run, the
    key opens and the page shows no block; in the 0644 run, the page
    shows `KeyExposed` and `KeyNoNewKey` (and Replace, with part 2
    built), and the key does not open; in the run with no key file,
    the page shows `KeyMissing` and `KeyNoNewKey` (and Replace, with
    part 2 built). In the 0644 run and the run with no key file, focus
    after Try again is on the new reason line, and Tab reaches Replace
    with part 2 built, else the page's next control (§8, "Try again,
    refused with another kind"). A further run at mode 0711 over a
    0600 key file shows `KeyDirUnsafe` as the 0755 run does, though no
    other user can list that folder.
16. Desktop: `endpoint-keys/` replaced by a link to a private directory
    shows `KeyDirUnsafe` with the path and Try again, and no
    `KeyNoNewKey`; the link is not followed, changed or replaced; after
    the person puts a private directory there, Try again shows the
    state the open then finds.
17. Desktop: `endpoint-keys/` owned by root with mode 0700 shows
    `KeyDirUnsafe`, not `KeyDamaged`, with Try again alone.
18. Desktop: `endpoint-keys/` replaced by a regular file shows
    `KeyDirUnsafe` with `KeyDirUnsafePath` and Try again alone; Share
    in a conversation of that identity shows the same two lines in the
    sheet.
19. No kind of §3's table shows a Make a new key control, on either
    client, on the page, in the share sheet or in a menu; `KeyNotNow`
    shows Try again alone, and refused again (item 10's seam returning
    the platform error twice) shows `KeyNotNowAgain` with Try again
    alone. The store's new-key refusal (`KeyIsTheIdentity`) is never
    reached from the interface.
20. With every kind of item 2 and `KeyMissing` in turn, the sheet of
    §4.3 shows the reason line and `KeyCannotShareHere` and not
    `KeyNoNewKey`; Open your card opens the page, which shows it.

Items 1, 6 and 19's "no action" describe part 1's interim. With
part 2 built, every kind that shows `KeyNoNewKey` also shows Replace
with a new identity after it, and item 21 replaces "no action" in
items 1, 6 and 19.
Items 21–35 are part 2's. Q-link, Q-list, Q-trust and Q-carry are
ruled (REPLY 01a20525); items 33 and 35 rest on Q-bound and are
blocked, not passed, until it is answered. A cut "at a point" in
items 24 and 31 is a test seam that ends the process there,
rust-ui-dev's to provide beside item 26's holding seam; until it
exists those runs are blocked.

21. With part 2 built: every kind that shows `KeyNoNewKey` shows
    Replace with a new identity after it; "temporarily unusable"
    shows Try again, then Replace as a text button, and no
    `KeyNoNewKey`; "key folder not safe" shows Try again alone. Replace
    is not in the share sheet or the list of identities.
22. Replace opens the dialog with focus on Cancel, and Enter cancels.
    For an added identity it shows `KeyReplaceBody`,
    `KeyReplaceCarries`, `KeyReplaceRemoves` and the Remove list of
    identities §7 with the same counts that identity's Remove would
    show, then `KeyReplaceKeepsList`, and Replace in `error`; for the
    first identity, `KeyReplaceBody`, `KeyReplaceCarries` and
    `KeyReplaceTurnsOff`, no `KeyReplaceKeepsList`, and Replace not in
    `error`; for "possibly a
    newer version's", `KeyReplaceOldNewer` too and Replace in `error`.
    With an own-card row Not sent, `KeyReplacePending` shows.
23. Replace confirmed on an added identity that had shared its card in
    conversations A and B: the client switches to a new identity with
    the old card's name and photo, a fingerprint different from any
    before and a new EndpointId; `KeyReplaced` is focused; the list
    shows A and B and no other; the old identity is in no list, and
    its conversations and contacts are gone. On the first identity:
    the same, and the first identity is listed as off, its
    conversations readable, its page showing `KeyReplacedBy` and no
    Replace; the new identity is the default. After leaving the page,
    and after a restart, `KeyReplaced` is not shown and the list is.
24. Killed during `KeyReplacing`, at each point the step can be cut:
    before the record of step 1 is written, and after it is written
    but before the administrative add, the next start shows the old
    identity in this state, unchanged (its card rows not withdrawn),
    with no new identity, no `KeyReplaceUnfinished`, and Replace,
    which then makes exactly one new identity; after the add and
    before the old one is removed or turned off (inside step 2
    included), the old identity's page shows `KeyReplaceUnfinished`
    and Finish replacing, which completes the replacement and makes no
    third identity (Q-link); after, as item 23.
25. After 23, A's row opens a conversation of the new identity with
    A's route and the share sheet; Share sends the new card, and A's
    counterparty's client shows it as a card of an identity it does
    not hold (cards' proposal for a new route), not a key change and
    not an update of the old contact. B's counterparty receives no
    card until B is shared from the list; a directory query of theirs
    may show the new endpoint once it is advertised, which is not a
    card and changes nothing in their contacts.
26. The old identity has a card row Not sent in C and one Sending in D
    that the transport has not taken (a test seam holding it), and a
    Cancel snackbar open on a third row in E: after Replace, C's and
    D's rows are withdrawn with no Undo and never sent (a restart
    included), E's snackbar is closed with no Undo, and C, D and E's
    conversations are on the list where the predicate (§5.4) holds
    for them. With step 1 failed by item 28's seam, C's and D's rows
    are not withdrawn and E's snackbar is still open.
27. The list survives leaving the page and a restart, until every row
    has been shared from or Done is pressed (Q-list); for an added
    identity, it is still there after its conversations were deleted.
28. Creating the new identity refused (a test seam): `KeyReplaceFailed`
    under the actions, the old identity unchanged, Replace offered
    again, and no second identity listed.
29. Q-trust and Q-carry: the old identity, added, trusts peers P and
    Q and holds contacts A (name, photo) and B. After Replace, the
    endpoint dimension of `admin.trust.list` for the new EndpointId
    lists P and Q as `administered`, and for the old one nothing (it
    is removed); the new identity's contacts are A and B with the same
    names, photos and routes; no counterparty receives a message or a
    card from the replacement (their clients show nothing new); and
    A's reply to the card shared from A's row is admitted and shown.
    On the first identity, the same for the new identity, and the
    first identity's own trusted set and contacts are unchanged.
30. A desktop restore whose configuration names two identities' key
    files that are gone: each shows "missing" with `KeyNoNewKey` and
    Replace, and is replaced from its own page; the first replacement
    changes nothing on the other's page.
31. Cut inside step 2 (item 24's seam) after some of P, Q and the
    contacts are written: Finish replacing writes the rest, and the
    new identity then lists P and Q once each and A and B once each.
    A seam refusing step 5 (the removal) shows `KeyReplaceUnfinished`
    focused, not `KeyReplaceFailed`, and Finish replacing offered.
32. The stored list holds remote routes and their order only: the
    store's row for it carries no title and no message text (read by
    the store's shape guard). A row's title is A's contact name; after
    A's contact is renamed under the new identity, the row shows the
    new name; a row whose route has no contact shows the route as the
    client shows a route. Removing the new identity removes the list.
33. Q-bound: with the trust overlay filled by a test seam so that the
    copy of Q is refused, the page shows `KeyReplaceUnfinished` and
    `KeyReplaceTrustFull`, focused; once a row is freed, Finish
    replacing copies Q and completes. Blocked until Q-bound states
    the bound and the freeing act.
34. On the first identity replaced, then the new identity removed: the
    record goes with it, and the first identity's page shows its block
    with Replace again and no `KeyReplacedBy`.
35. Q-bound: the list at its bound, with more conversations matching
    the predicate than the bound holds, behaves as Q-bound states;
    blocked until then.

## 10. Strings

The Text column is language-culture-en's final English (2026-10-24;
part 1's rows, `KeyNoNewKey`, `KeyNewerVersion`, `KeyCannotShare` and
`KeyCannotShareHere`, 2026-11-23; `KeyDirUnsafe`, changed for Q-loose
and for folders at 0711 and 0701, 2026-11-23; `KeyNotNowAgain`,
2026-11-23; part 2's `KeyReplaceBody`,
`KeyReplaceCarries`, `KeyReplaceKeepsList`, `KeyReplaced`,
`KeyReplacedBy` and `KeyReplaceUnfinished`, 2026-11-23); the third
column is what it must say. A row marked *retired* is not in the build from the Stage 22
release; it stays here as the record. Part 2's rows (§5) are
ux-designer's proposals for language-culture-en to finalise, marked
*proposed*. `{identity}` is the identity's card name, or its route
label without one (cards §5.7.6); `{path}` is the folder's full path
as the platform shows it, isolated as a path, never translated;
`{old}` is the replaced identity's name and `{new}` the new
identity's, each as `{identity}` is.

| Key | Text | Where; must say |
|---|---|---|
| `KeyStateTitle` | This identity's key cannot be used | §4.1 heading |
| `KeyLostDevice` | This phone no longer allows InterWeave to use this identity's key. | Android, invalidated or gone; must not claim a cause the client cannot know |
| `KeyBelowDevice` | This identity's key has weaker protection than this phone requires, so InterWeave does not use it. | a downgrade; must say it is refused on purpose, not broken |
| `KeyOtherDevice` | This identity's key was saved on another device and works only there, not on this computer. | desktop, a sealed record |
| `KeyNewerVersion` | This version of InterWeave cannot use this identity's key, because the key file has a format this version does not know. A newer version may be able to use it. To find out, update InterWeave. | must name the update as the way to keep the key, and say only that a newer version *may* read it (§3: the client cannot know); names no new key, which is not offered (until the Stage 22 release it ended: "If you make a new key, you lose this key.") |
| `KeyDamaged` | This identity's key file is not valid, so the key cannot be used. | every damaged or foreign record, and anything but a regular file or a link in the key file's place |
| `KeyNotNow` | InterWeave could not use this identity's key. The problem may be temporary. Try again. | a platform error; must suggest trying again, not a loss |
| `KeyMissing` | This identity's key is missing from this device. | a lost file (§7); must not suggest the person deleted it |
| `KeyCannotShare` | You can still send and receive messages, but you cannot share this card. | after every reason; must say what is blocked and what is not, and name no time when the key works again, since for most kinds it never does (until 2026-11-23: "...until the key can be used.") |
| `KeyMakeNew` | *retired* Make a new key | §4.1 action |
| `KeyStateRow` | Key cannot be used | §4.2 row line |
| `KeyNoNewKey` | InterWeave cannot give this identity a new key. | §3, §4.1, after `KeyCannotShare`, for every kind but `KeyNotNow` and `KeyDirUnsafe`; must say no new key can be made for this identity, so the person does not look for one; must not promise a later version, blame the person, or name an implementation term (EndpointId, derived); never in the share sheet (§4.3) |
| `KeyCannotShareHere` | You cannot share your card without this identity's key. | §4.3 sheet; as `KeyCannotShare`, no time when the key works again |
| `KeyOpenCard` | Open your card | §4.3 primary |
| `KeyNewTitle` | *retired* Make a new key for {identity}? | §5.1 title |
| `KeyNewBody` | *retired* The next time you share your card with a contact who has it, their app will show a warning that the card might be from someone else. Tell them it was you. Until then, nothing changes for them. | §5.1; must name the change, the warning they will see, and that nothing happens before a share |
| `KeyNewOld` | *retired* The old key will be deleted and cannot be used again. | §5.1, every kind but missing |
| `KeyNewOldNewer` | *retired* A newer version of InterWeave may be able to use the old key. To try to keep it, update InterWeave first. If you make a new key, you lose the old key. Updating later will not bring it back. | §5.1, newer version; must say the choice is the update or the loss, and claim no more than *may* |
| `KeyNewConfirm` | *retired* Make a new key | §5.1 confirm |
| `KeyMaking` | *retired* Making a new key… | §5.2 |
| `KeyMade` | *retired* This identity has a new key. Your contacts have the new key only after you share your card with them again. | §5.3 |
| `KeyShareAgainTitle` | *proposed (brought back, changed):* Share your new card in these conversations | §5.4 list heading; must say the card to share is the new identity's (Stage 21: "Share your card again in these conversations") |
| `KeyNewFailed` | *retired* InterWeave could not make a new key. Choose Make a new key to try again. | §5.3, failed before the old file was removed; says nothing about what changed |
| `KeyExposed` | Other users of this device may be able to read or change this identity's key file. To be safe, InterWeave does not use this key. | §3 "not protected"; must say why it is refused, and not suggest fixing the file |
| `KeyNewFailedGone` | *retired* The old key was deleted, but InterWeave could not make a new key. Choose Make a new key to try again. | §5.3, failed after the old file was removed; must say the saved key is gone and the new one was not made (architect-cto, Q-order) |
| `KeyDirUnsafe` (changed 2026-11-23) | This identity's key must be kept in a regular folder that your user account owns and that gives no permissions at all to any group or any other user. The location shown is not a folder like that, so InterWeave does not use any key kept there. Make the location shown a regular folder that your user account owns and that gives no permissions at all to any group or any other user, then choose Try again. | §3, desktop only; must be true for a symbolic link, a folder owned by another user account, a file where the folder should be, and (Q-loose, 2026-11-23) a folder that this user owns and that gives any permission to a group or other users, for which "owns" alone was already true, so the text also says that the folder must give no permissions to any group or any other user. Not "only your user account owns and can open": "only" may be read as covering the owner alone, and then the second sentence is false for that last folder (language-culture-en, j409); must be true whatever files the folder holds, no key file included, so it says "any key kept there" and not "the key" (language-culture-en, j410); must be true of a folder at 0711 or 0701, which the read refuses (any permission for another user) though no other user can list it, so "open" must not read as "list" to a person who checks their folder (review 6019805428; the words are language-culture-en's): it says "gives no permissions at all to any group or any other user", which the person can check against the folder's permissions, and not "can open" (language-culture-en, j411); must say what to make of it, since the client never changes what it does not own |
| `KeyDirUnsafePath` | Folder location: {path} | §4.1 and §4.3, desktop only, the line under `KeyDirUnsafe` wherever it shows; a label and the path, `{path}` last so no punctuation touches it |
| `KeyNewPending` | *retired* In some conversations, your card has not reached the contact yet. If InterWeave has not sent the card yet, making a new key cancels it. If InterWeave has already sent the card, it cannot be canceled, and the contact may still receive it with the old key. | §5.1, only when this identity has own-card rows not yet Accepted by remote transport; must not promise a cancellation the client cannot make (review round 3, F11) |
| `KeyNotNowAgain` (changed 2026-11-23) | InterWeave still could not use this identity's key. | §6, after Try again under "temporarily unusable" or "key folder not safe" refused with the same kind; must be true when no key file is there (a loose folder that holds none, a file where the folder should be), so not "the key", as if one were found; must not repeat the folder's state, which the reason line above says |
| `KeyReplace` | *proposed:* Replace with a new identity | §5.1 action; must name the result, a new identity, not a new key |
| `KeyReplaceTitle` | *proposed:* Replace {identity} with a new identity? | §5.2 title |
| `KeyReplaceBody` | InterWeave makes a new identity with this card's name and photo. To your contacts, the new identity is a different identity from {identity}, and they get the new identity's card only when you share that card with them. Messages they send to {identity} will no longer reach you. | §5.2, every case; must say: a new identity with the same name and photo; contacts see a different identity, and get its card only after a share; messages to the old identity stop reaching the person. Never "key change"; never that contacts cannot see the new identity before a share, since a trusted peer's directory shows it once advertised (REPLY 01a20525) |
| `KeyReplaceCarries` | The new identity trusts the people that {identity} trusts and gets a copy of the contacts of {identity}. Nothing is sent to these people. | §5.2, every case, after `KeyReplaceBody`; must say both copies, that they are the new identity's, and that nobody is told (Q-trust, Q-carry) |
| `KeyReplaceRemoves` | *proposed:* {identity} is then removed: | §5.2, an added identity, before identities §7's Remove list; must say the removal follows the new identity |
| `KeyReplaceKeepsList` | Only one thing of {identity} stays: a list of the conversations where you shared its card or tried to share it, so that you can share the new card with the same people. The list holds no messages. | §5.2, an added identity, after the Remove list; must say what stays (the list, Q-list) beside what goes, that it is the only thing kept, and that it holds no messages |
| `KeyReplaceTurnsOff` | *proposed:* {identity} is then turned off. You can still read its conversations and contacts, but it cannot send or receive messages. | §5.2, the first identity; must say what stays (to read) and what stops (send and receive) |
| `KeyReplaceOldNewer` | *proposed:* A newer version of InterWeave may be able to use this identity's key. To try to keep this identity, update InterWeave first. | §5.2, "possibly a newer version's"; must say the choice is the update or the replacement, and claim no more than *may* |
| `KeyReplacePending` | *proposed:* In some conversations, this identity's card has not reached the contact yet. If InterWeave has not sent the card yet, replacing the identity cancels it. If InterWeave has already sent the card, it cannot be canceled, and the contact may still receive it. | §5.2, only when the old identity has own-card rows not yet Accepted by remote transport; must not promise a cancellation the client cannot make |
| `KeyReplaceConfirm` | *proposed:* Replace | §5.2 confirm |
| `KeyReplacing` | *proposed:* Replacing {identity}… | §5.3 |
| `KeyReplaced` | {new} has replaced {old}. Your contacts get the card of {new} only when you share that card with them. | §5.4, the new identity's page, that session only; must name both identities and that contacts get the new card only after a share; as `KeyReplaceBody`, never that they cannot see the identity before it |
| `KeyReplacedBy` | {new} has replaced this identity. | §5.4, the first identity's page after it was replaced |
| `KeyReplaceFailed` | *proposed:* InterWeave could not make the new identity. {identity} has not changed. Choose Replace with a new identity to try again. | §5.4, step 1 failed; true only because nothing of the old identity is touched before step 1 (§5.3) |
| `KeyReplaceUnfinished` | {new} was made to replace this identity, but the replacement did not finish. | §5.3, the old identity's page after a stop between steps 1 and 5 |
| `KeyReplaceTrustFull` | *proposed, first sentence final:* InterWeave stopped before {new} trusted all the people that this identity trusts, because this device cannot hold more trusted people. | §5.3, under `KeyReplaceUnfinished` when step 2 stopped at the trust overlay's row bound; must say what did not finish and why; what the person can do, and the bound's name, wait for Q-bound |
| `KeyReplaceFinish` | *proposed:* Finish replacing | §5.3 action, in place of Replace |
| `KeyShareAgainDone` | *proposed:* Done | §5.4, closes the share-again list |

`TryAgain` and `Cancel` are the existing keys.

## 11. Deliberately left out

- **Why** a Keystore key was invalidated: the client cannot know, so it
  names no cause (`KeyLostDevice`).
- **Moving or exporting a key**, and any way to bring an old key back:
  ADR-0064's.
- **A notice on Chats or a notification** for this state: it blocks
  only Share, which leads here.
- **Replacing several identities at once** (a desktop restore whose
  every identity is "missing", §7): each is replaced from its own page,
  one at a time; a "replace all" would be one confirmation for several
  deletions.
- **Replacing an identity whose key opens**: a person who wants a new
  identity adds one (identities note); Replace is offered only in this
  state.
- **Telling contacts automatically** after a replacement: a share is
  the person's act (ADR-0059), so part 2's list offers it and sends
  nothing.
- **The fingerprint row's own design** (the key-change proposal is
  withdrawn, ADR-0067 decision 5):
  built from `human-client-ui.md` §3 and §13 directly; their
  interaction design is owed to cards (Stage 21 backlog), and this note
  only names where it replaces the row.

## 12. Questions

Q-file, Q-missing, Q-order and Q-dir's tightening describe the new key
and the silent first key of Stage 21, withdrawn from the Stage 22
release (§5, §7); they stay as the record. Replace does not tighten
the folder: a loose one is shown as "key folder not safe" for the
person to fix (§3, §5.1).
The answers on refusals and
on "key folder not safe" still hold.

For architect-cto (QUESTION 01a16d88), answered by DECISION 01a16d8a
the same day, as read here:

- **Q-file**: the refused file goes only on the person's confirmation
  of a new key, never before and never on its own; the new key is
  written first, then the old file removed and its alias retired, in
  one step. For a newer version's record the state says to update
  first, the new key stays offered, and its confirmation says the saved
  key is deleted and an update will not bring it back; nothing is kept
  aside, since a second sealed file with no reader is a secret at rest
  for nothing. A platform error: Try again first, the new key beside
  it.
- **Q-missing**: a missing file is not a first key unless the identity
  never shared its card (§7); otherwise the "missing" state and a
  confirmed new key. ADR-0063 decision 1 is amended with that on
  architect-cto's branch.

Asked after review round 1 (QUESTION 01a16d92), answered by
architect-cto (REPLY 01a16d93) the same day:

- **Q-order**: "new key first" withdrawn, since one alias per identity
  makes it impossible on Android. The order on both platforms: confirm,
  remove the old file, create the new key; a stop after the removal is
  the "missing" state at the next open, confirmed again, never a silent
  key; the failure line then says the saved key is gone and the new one
  was not made (§5.2, §5.3).
- **Q-exposed**: confirmed as read: refused, a new key offered, no
  offer to tighten the file; the reason is one sentence (§3).
- **Q-route**: the share record alone, `card_shared_at` on any
  conversation that speaks as the identity; the accepted-card clause is
  gone from the ADR amendment (§7). Revised 2026-10-27 (REPLY
  01a17abe) once cards §5.3 set `card_shared_at` at Accepted: a
  pending own-card row counts beside the record, since a card on its
  way may still arrive.

Asked after review round 2, the note built on the reading given:

- **Q-dir**: a non-private `identity-keys/` makes every removal and
  creation in it refuse. Answered by architect-cto (REPLY 01a16d9b):
  a real directory this user owns with a loose mode is made owner-only
  by the client as the step's first part, asking nothing; a link, or a
  directory another user owns, is refused with no new key and a line of
  its own (`KeyDirUnsafe`, desktop only) naming what to make private
  and where, with Try again; the client never follows or replaces a
  link there and never changes what it does not own. `KeyNewFailed`
  stays for a tightening or a create the platform refuses. A file
  standing where the directory should be is the same class, confirmed
  by architect-cto (REPLY 01a16dad): the client replaces nothing it did
  not make.

Asked by architect-cto for ADR-0067 decision 5 (REQUEST 01a202c7,
2026-11-22, after rust-ui-dev-02's report 01a202c6 that the store's key
replace was removed), as read here:

- **Q-interim**: every kind of §3's table keeps its explanation and
  offers no Make a new key; Try again stays for the temporary kind;
  the screen says what it says in its place (`KeyNoNewKey`, §3). Read
  here also as: Try again stays for "key folder not safe", which is
  also fixed by the person and opened again, not by a new key; and the
  silent first key goes with the new key, since ADR-0067 decision 5
  withdraws it in the same sentence (§7).
- **Q-restore**: `IdR1NewKeys` says the words restore the PeerId, each
  identity's key is gone, and each identity is replaced by a new one;
  never that contacts will see each key change: they see a new
  identity (identity note §10).

Asked of architect-cto with part 2 (QUESTION 01a204e4, 2026-11-22),
answered by REPLY 01a20525 the same day: all four yes, with
conditions, and the new identity taking the old card's name and photo
confirmed as drawn. The common reason: ADR-0061 decision 9 keeps
identities apart because the person keeps them apart, and a
replacement is the person's own confirmed act saying the new identity
stands for the old one, so what the old one held about other people
may follow where the confirmation says so; nothing about the old
identity itself follows. The store's text (STATE.md, the link and the
list), the amendments of ADR-0067 decision 5 and ADR-0061 decision 7,
and `human-client-ui.md` §13 are architect-cto's supply on the j185
batch. As read here:

- **Q-link**: yes, the store's. Written **before** the administrative
  add, since the client mints the key and knows the new EndpointId
  first (§5.3 step 1). Kept until step 5 for an added identity and
  deleted with its Remove, so no record of a removal survives; kept
  while both exist for the first identity, and gone if the new one is
  ever removed (§5.4). After an added identity's step 5 nothing stored
  names the old one: `KeyReplaced`'s {old} is that session's.
- **Q-list**: yes, as the one thing of a removed identity that
  outlives its Remove, so `KeyReplaceKeepsList` says it is kept beside
  what goes. Stored as remote routes only, `{peer, endpoint}` with the
  order, owned by the new identity; a row's title is read from the new
  identity's contacts, else the route as the client shows a route; no
  stored title, no message. A row leaves when shared from, all leave
  at Done, all go with the new identity's Remove (§5.4).
- **Q-trust**: yes. The copy admits nobody new to the node, since the
  peers are already in its admission set; it is the person's trust act
  under the new identity, written after the add through the endpoint
  dimension and before step 5, the add itself still writing the set
  empty (ADR-0067 decision 6). The confirmation says it
  (`KeyReplaceCarries`). Two consequences are designed: a partly
  copied set when the overlay's row bound refuses (§5.3, finished by
  Finish replacing), and `KeyReplaceBody`'s "only after a share" made
  true of the card and not of the identity, which a trusted peer's
  directory query shows once it is advertised.
- **Q-carry**: yes, a copy before step 5, as rows of the new identity;
  the old identity's rows then go or stay as ADR-0067 decision 5 says.
  Nothing is sent by it. A contact is the counterparty's accepted card,
  and their key has not changed, so the row is as true under the new
  identity as under the old.

Open, for rust-ui-dev-02 (the store and the overlay), asked with this
revision:

- **Q-bound**: two bounds the rulings ask for. (a) The share-again
  list needs a stated bound (REPLY 01a20525, Q-list). Proposed: the
  bound the old identity's own conversations already have, so that
  every conversation the predicate selects fits and no row is dropped;
  if the store has no such bound, a number of rows with what the step
  does above it, which, since a dropped row loses for good who should
  hear about the new identity, is asked of the person before the
  replacement rather than dropped silently. (b) The trust overlay's row
  bound: its name and size, whether a replacement can reach it in
  practice, and what frees a row, so that `KeyReplaceTrustFull` can say
  what the person can do. Acceptance 33 and 35 wait for it.

Asked of architect-cto after review round 4 on #738 (F18), and
answered with option (a) by REPLY 01a207c7 the same day: "not used
again" is the key file's own refusal; a refusal that comes only from
a loose directory this user owns, over a private file, ends when the
folder is private, and no mark outlasts the fix; a file that is not
private stays "not protected" and is not used again. §3 places every
loose folder that this user owns under "key folder not safe", since
the read stops at the folder: the person fixes the folder first, and
Try again then shows the file's own state, so the rule for the file
applies from the first open that reaches the file (review F23, F24
on #738); §5.1 and
acceptance 15 follow. ADR-0063 records the ruling (A 2026-11-23,
decision 2): exposed is the file's own state; the directory is judged
first, and a file's own state is judged only once the directory is
private. The mapping in §3 follows that ruling. The question, as asked:

- **Q-loose**: on the desktop, "not protected" can come from the
  `endpoint-keys/` directory alone (this user's, loose mode, each key
  file private). A new key cannot be minted into it (Q-dir), and
  making it owner-only first, as Q-dir's answer did for Stage 21's
  new key, lets every key in it open again at its next open, the
  replaced identity's and the turned-off first identity's included,
  against §3's "not used again". The same reopening already follows
  when the person makes the folder private themselves (§3: the state
  lasts until a key opens). Asked: (a) may a key refused only for the
  folder's mode be used again once the folder is private (then this
  case is "key folder not safe": Try again after the person fixes the
  folder, and §3's sentence is narrowed to the file), or (b) must the
  refusal outlast the folder's fix (a stored mark, the store's), and
  Replace then tighten the folder first?

Asked of architect-cto with job j166 (review round 1 on #746), and
answered yes the same day, 2026-11-23, by REPLY 01a208d0-28c6: a
directory, a FIFO, a socket or a device file at the key path is a
record that is not
valid ("damaged", §3), whoever owns it, since no record is there.
ADR-0063 records the ruling (second amendment of 2026-11-23, decision
2: its three causes of "exposed" say "regular file"); §3 and
acceptance 14 follow. Review round 1 on #748 found that the read does
not refuse every such entry by its kind: one whose own mode refuses
this user fails the open with a permission refusal, and a socket or a
device with no driver fails it with `ENXIO` or `ENODEV`. §3's
"damaged" row names all three. Still open, architect-cto's to rule
(asked with F4 of that round), and not designed here: a
directory in the key file's place of an identity that was added
cannot be removed by Remove (it deletes a file), so Replace's step 5
(§5.3) stops there and Finish replacing repeats the refusal. A FIFO,
a socket or a device file is removed as a file is, and the first
identity is turned off, not removed.
The question, as asked:

- **Q-kind**: the read refuses anything at the key path that is
  neither a regular file nor a link (a directory, a FIFO) as
  `NotPrivate`, before it looks at owner or mode
  (`crates/config/profile-config/src/private_read.rs`,
  `open_private_as`). ADR-0063 decision 2's "exposed" names three
  causes, none of them this, and `KeyExposed` ("Other users … may be
  able to read or change …") is not true of a directory at 0700 that
  this user owns. Until architect-cto answers, this note reads it this
  way: such a key path is "damaged" (§3), whoever owns it, since no
  record is there and the read never looks at the owner; acceptance 14
  checks this reading. What happens to a directory in the key file's
  place when Replace removes the old identity is not designed here.
