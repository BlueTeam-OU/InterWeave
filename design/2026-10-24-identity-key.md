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
one (plan §30 steps 1–3), is the new §5, a proposal with four
questions open for architect-cto (§12); it is built once an identity
can be created and removed, and until then part 1 stands.

What part 1 adds, changes and retires (§10):

- added: `KeyNoNewKey`;
- changed: `KeyNewerVersion` (no "if you make a new key"),
  `KeyCannotShare` and `KeyCannotShareHere` (no time when the key
  works again; language-culture-en's pass, 6257df7c8), and, in
  `design/2026-10-19-android-identity.md`, `IdR1NewKeys` (a new
  identity, not a changed key);
- retired from the build, with §5 (withdrawn): `KeyMakeNew`,
  `KeyNewTitle`, `KeyNewBody`, `KeyNewOld`, `KeyNewOldNewer`,
  `KeyNewPending`, `KeyNewConfirm`, `KeyMaking`, `KeyMade`,
  `KeyShareAgainTitle`, `KeyNewFailed`, `KeyNewFailedGone`.

What part 2 adds and changes (§10): added `KeyReplace`,
`KeyReplaceTitle`, `KeyReplaceBody`, `KeyReplaceRemoves`,
`KeyReplaceTurnsOff`, `KeyReplaceOldNewer`, `KeyReplacePending`,
`KeyReplaceConfirm`, `KeyReplacing`, `KeyReplaced`, `KeyReplacedBy`,
`KeyReplaceFailed`, `KeyReplaceUnfinished`, `KeyReplaceFinish`,
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
| damaged | `NotARecord`, `Truncated`, `UnsupportedVersion(v)` with `v` below this build's version, `EndpointMismatch`, `KeyMismatch`, `Cipher(Unauthentic)`; the read's `TooLarge`, and its `Read` with `InvalidData` (bytes that are not text) or a permission refusal while `endpoint-keys/` is a directory this user owns (a file its owner cannot read) | both | `KeyDamaged` | none; `KeyNoNewKey` |
| not protected | the read's `NotPrivate` for the file (readable or writable by another user, a link, another user's file), or for an `endpoint-keys/` directory this user owns with a loose mode | both (in practice the desktop) | `KeyExposed` | none; `KeyNoNewKey` |
| key folder not safe | the read's `NotPrivate` for an `endpoint-keys/` that is a link or not a directory at all; and any read refusal while `endpoint-keys/` is a directory another user owns, which the client checks itself, since the read judges the directory's mode and not its owner (a root-owned 0700 directory reads as a permission refusal) | desktop (Android's is the app's own) | `KeyDirUnsafe` | Try again, after the person fixes the folder |
| temporarily unusable | `Cipher(Platform)`; the read's `Read` for any I/O error but the two in the "damaged" row and an other-owner directory | both | `KeyNotNow` | Try again |
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
a card again.

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

Status: part 2, 2026-11-22, a proposal. Q-link, Q-list, Q-trust and
Q-carry (§12) are architect-cto's to rule, and the sections that rest
on them say so. It is built with plan §30 steps 1–3, once an identity
can be created and removed through the administrative boundary (ADR-0061
decisions 2 and 7). Until then, part 1's interim stands: `KeyNoNewKey`
and no action (§3, §4.1). From the Stage 22 release this replaces the
Stage 21 "Make a new key", which git history keeps (ADR-0067
decision 5: a new key is a new identity).

The person's goal is unchanged (§2): to share a card again. Under
ADR-0067 that means a new identity, which contacts see as a new
contact, so the flow has to say three things before it is confirmed:
who sees what, that messages to the old identity stop reaching them,
and what happens to the old identity.

### 5.1 Where it is offered

On §4.1's block, after `KeyNoNewKey`: **Replace with a new identity**
(`KeyReplace`), filled tonal (Save stays the page's one filled
button), for every kind that shows `KeyNoNewKey`. For "temporarily
unusable" it is a text button after Try again, and `KeyNoNewKey` is
not shown: Try again comes first, as the Stage 21 design had it. It
is never offered for "key folder not safe", where the person fixes the
folder and the key can open again (§4.1).

It is offered nowhere else: not in the share sheet (§4.3, whose Open
your card leads here), not in the list of identities (§4.2's row opens
this page). It replaces part 1's no-action state and changes nothing
else in §3 or §4.

### 5.2 The confirmation

**Replace with a new identity** opens a dialog (Material's basic
dialog; on Android `AlertDialog`'s placement). In order:

- title `KeyReplaceTitle`: "Replace {identity} with a new identity?";
- `KeyReplaceBody`, one paragraph, every case: InterWeave makes a new
  identity with this card's name and photo; to your contacts it is a
  different identity, which they see only after you share its card
  with them; messages they send to {identity} no longer reach you;
- what happens to the old identity, one of:
  - an added identity: `KeyReplaceRemoves` ("{identity} is then
    removed:"), followed by identities §7's Remove list as it stands
    there — `IdentityRemoveLoses`, `IdentityRemoveKey`,
    `IdentityRemoveMayHaveReached` when its count is not zero, and
    `IdentityRemoveOthersKeep` — so a removal by replacement says
    exactly what a removal by Remove says;
  - the first identity: `KeyReplaceTurnsOff`: it is turned off; its
    conversations and contacts stay, to read; it cannot send or
    receive (identities §7's Turn off);
- `KeyReplaceOldNewer`, only for "possibly a newer version's": a newer
  version may be able to use this identity's key; to keep this
  identity, update InterWeave first;
- `KeyReplacePending`, only when the old identity has own-card rows not
  yet Accepted by remote transport: those still held are cancelled,
  and one already on its way may still arrive, as the old identity's
  card (§5.3, step 2);
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
   and photo, not yet shared with anyone. The client records that the
   new identity replaces the old one (Q-link). Nothing of the old
   identity is touched before this step has stored the new one, so a
   failure here leaves the old identity exactly as it was.
2. **The old identity's pending cards.** As in the Stage 21 design,
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
3. **The default.** The new identity becomes the default when the old
   one was the default, and always when the old one is the first
   identity (ADR-0067 decision 5).
4. **The old identity.** An added identity is removed, as identities
   §7's Remove removes one. The first identity is turned off, as
   identities §7's Turn off turns it off.

**Every point the step can stop at resolves to a state the person
already knows**:

- stopped before step 1 has stored the new identity: the old identity
  is in this state as before, unchanged; after a failure (not a kill)
  `KeyReplaceFailed` shows under its actions, and Replace is offered
  again.
- stopped after step 1, before step 4 is done: both identities exist.
  The old identity's page shows `KeyReplaceUnfinished` ("{new}
  replaces this identity, but the replacement did not finish.") and,
  in place of Replace, **Finish replacing** (`KeyReplaceFinish`),
  which runs steps 2 to 4 only, without a dialog (the person
  confirmed them), and never creates another identity. This needs the
  record of step 1 (Q-link). Without that record, a second Replace
  would make a third identity, which this design does not accept.
- after step 4: the result (§5.4).

The client never shows "replaced" before step 4 is done, and never
shows the new identity before it is stored where the next start will
open it.

### 5.4 The result

**Replaced.** The client switches to the new identity visibly, as
cards §5.7.5's transition does ("Opening as {identity}"), and shows
the new identity's card page: its fingerprint row, and under it
`KeyReplaced` ("{new} replaces {old}. Your contacts see {new} as a new
identity after you share its card with them.").

Then, when any conversation of the old identity holds or may hold its
card — `card_shared_at` set, an own-card row not yet Accepted by remote
transport when the step ran (withdrawn or on its way), or a card that
may have left this device (ADR-0063 decision 1, A 2026-10-27), read
over the old EndpointId's conversations (plan §30 step 2) —
`KeyShareAgainTitle` ("Share your new card in these conversations")
and one row per such conversation, by the conversation's title, most
recently shared first. A row opens the new identity's conversation
with that conversation's remote route (a new one if it has none) with
the share sheet open (cards §5.3). The list answers "who should hear
from me": those, and nobody else.

The list is kept until each row has been shared from or the person
closes it with **Done** (`KeyShareAgainDone`), and is shown on the new
identity's card page until then (Q-list). It cannot be a list shown
only until the page is left: for an added identity the conversations
it is read from are deleted in step 4, so leaving the page would lose
it for good.

What the new identity trusts and whom it holds as contacts is Q-trust
and Q-carry (§12). Until they are ruled, the new identity starts with
no trusted peer and no contact, as Add an identity does, and a row of
the list opens a conversation whose replies the new identity does not
admit until the person trusts the peer. That is the case the questions
ask about.

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
  replaces this identity.").

## 6. Try again ("temporarily unusable")

Try again opens the key once more. Opened: the block is replaced by the
fingerprint row, nothing announced beyond §8. Refused again with the
same kind: `KeyNotNowAgain` ("InterWeave still could not use the key.") under the
actions, Try again still the only one; with another kind: that kind's
block, with `KeyNoNewKey` and no action where §3 gives it.

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
  or `adopt`, Android's at onboarding (ADR-0067 decision 4). The same
  holds when the whole `endpoint-keys/` directory is missing outside
  the recovery flow. Until the Stage 22 release a key was made
  silently for an identity whose card no conversation held or might
  hold; §5.4's share-again predicate was the test.
- **Restore on fresh state** (identity note `IdR1NewKeys`): the person
  is told before the phrase that the words restore the PeerId, that
  each identity's key is gone, and that each identity is replaced by a
  new one, which contacts see as a new identity. On Android the first
  identity's key is minted at Restore (ADR-0067 A 2026-11-21), so it
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
| §4.1, a kind with `KeyNoNewKey` | the page title, as above; `KeyNoNewKey` is text, read in order, not a stop of its own; Tab from the block goes to Replace with a new identity (part 2), or, before part 2 is built, to the page's next control | keyboard; desktop screen reader reads it once, after `KeyCannotShare` |
| Replace with a new identity pressed | the dialog's Cancel | keyboard; screen reader reads the title, then the body in §5.2's order |
| the dialog cancelled (Cancel, Back, Escape, scrim) | Replace with a new identity | keyboard |
| Replace confirmed, while `KeyReplacing` shows | the block's progress line (the button that held focus is gone) | keyboard; screen reader announces `KeyReplacing` once |
| replaced | after the switch to the new identity (cards §5.7.5), the `KeyReplaced` line on its card page, which is announced by being focused, not again | keyboard; screen reader |
| the replacement failed | `KeyReplaceFailed`, announced by being focused, not again | keyboard; screen reader |
| the old identity's page with `KeyReplaceUnfinished` opened | the page title, as §4.1; Tab reaches Finish replacing after `KeyReplaceUnfinished` | keyboard |
| Finish replacing pressed | the progress line, then as "replaced" or "failed" | keyboard; screen reader |
| a row under `KeyShareAgainTitle` | the conversation's share sheet, as cards §5.3 opens it | keyboard |
| back on the new identity's card page from a row's conversation, the card shared | the next row of the list, else Done | keyboard |
| back on the new identity's card page from a row's conversation, not shared | the row that opened it | keyboard |
| Done under the list | the new identity's fingerprint row (the list and Done are gone) | keyboard |
| the page goes to the background during `KeyReplacing` | on return, as "replaced", "failed" or unfinished by what the step left; the announcement is made on return, once | keyboard; screen reader |
| a card row's Cancel answered while the step runs (§5.3 step 2) | stays where it is; a success is announced once after the step's own announcement | keyboard; desktop screen reader |
| Try again, opened | the fingerprint row | keyboard |
| the `KeyDirUnsafePath` line ("key folder not safe"), on the page | reached by keyboard in reading order before Try again, so the path can be selected and copied; it opens nothing | keyboard; screen reader reads it once |
| the `KeyDirUnsafePath` line, in the share sheet | reached by keyboard in reading order before `KeyCannotShareHere` and the actions, so backwards from Open your card, where the sheet's focus opens; selectable and copyable, it opens nothing | keyboard; screen reader reads it once, with the reason line |
| Try again, refused again | `KeyNotNowAgain` | keyboard; screen reader |
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
    `KeyDamaged`, not `KeyNotNow`.
15. Desktop: `endpoint-keys/` at mode 0755 shows `KeyExposed` and
    `KeyNoNewKey`, and no action; the directory's mode is not changed.
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

Items 1, 6, 15 and 19's "no action" describe part 1's interim. With
part 2 built, every kind that shows `KeyNoNewKey` also shows Replace
with a new identity after it, and item 21 replaces "no action" there.
Items 21–30 are part 2's; those that rest on Q-link, Q-list, Q-trust
or Q-carry name it and wait for its ruling.

21. With part 2 built: every kind that shows `KeyNoNewKey` shows
    Replace with a new identity after it; "temporarily unusable"
    shows Try again, then Replace as a text button, and no
    `KeyNoNewKey`; "key folder not safe" shows Try again alone. Replace
    is not in the share sheet or the list of identities.
22. Replace opens the dialog with focus on Cancel, and Enter cancels.
    For an added identity it shows `KeyReplaceBody`,
    `KeyReplaceRemoves` and the Remove list of identities §7 with the
    same counts that identity's Remove would show, and Replace in
    `error`; for the first identity, `KeyReplaceBody` and
    `KeyReplaceTurnsOff`, and Replace not in `error`; for "possibly a
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
    Replace; the new identity is the default.
24. Killed during `KeyReplacing`, at each point the step can be cut:
    before the new identity is stored, the next start shows the old
    identity in this state, unchanged (its card rows not withdrawn),
    with no new identity; after it is stored
    and before the old one is removed or turned off, the old
    identity's page shows `KeyReplaceUnfinished` and Finish
    replacing, which completes the replacement and makes no third
    identity (Q-link); after, as item 23.
25. After 23, A's row opens a conversation of the new identity with
    A's route and the share sheet; Share sends the new card, and A's
    counterparty's client shows it as a card of an identity it does
    not hold (cards' proposal for a new route), not a key change and
    not an update of the old contact. B's counterparty sees
    nothing until B is shared from the list.
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
29. Q-trust and Q-carry: what the new identity trusts and holds as
    contacts after a replacement is checked as ruled; until ruled,
    this item is blocked, not passed.
30. A desktop restore whose configuration names two identities' key
    files that are gone: each shows "missing" with `KeyNoNewKey` and
    Replace, and is replaced from its own page; the first replacement
    changes nothing on the other's page.

## 10. Strings

The Text column is language-culture-en's final English (2026-10-24;
part 1's rows, `KeyNoNewKey`, `KeyNewerVersion`, `KeyCannotShare` and
`KeyCannotShareHere`, 2026-11-23); the third column is what it must
say. A row marked *retired* is not in the build from the Stage 22
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
| `KeyDamaged` | This identity's key file is not valid, so the key cannot be used. | every damaged or foreign record |
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
| `KeyDirUnsafe` | This identity's key must be kept in a regular folder that only your user account owns. The location shown is not a folder like that, so InterWeave does not use the key. Make it a regular folder that only your user account owns, then choose Try again. | §3, desktop only; must be true for a symbolic link, a folder owned by another user account, and a file where the folder should be; must say what to make of it, since the client never changes what it does not own |
| `KeyDirUnsafePath` | Folder location: {path} | §4.1 and §4.3, desktop only, the line under `KeyDirUnsafe` wherever it shows; a label and the path, `{path}` last so no punctuation touches it |
| `KeyNewPending` | *retired* In some conversations, your card has not reached the contact yet. If InterWeave has not sent the card yet, making a new key cancels it. If InterWeave has already sent the card, it cannot be canceled, and the contact may still receive it with the old key. | §5.1, only when this identity has own-card rows not yet Accepted by remote transport; must not promise a cancellation the client cannot make (review round 3, F11) |
| `KeyNotNowAgain` | InterWeave still could not use the key. | §6 |
| `KeyReplace` | *proposed:* Replace with a new identity | §5.1 action; must name the result, a new identity, not a new key |
| `KeyReplaceTitle` | *proposed:* Replace {identity} with a new identity? | §5.2 title |
| `KeyReplaceBody` | *proposed:* InterWeave makes a new identity with this card's name and photo. To your contacts, it is a different identity, and they see it only after you share its card with them. Messages they send to {identity} no longer reach you. | §5.2, every case; must say: a new identity with the same name and photo; contacts see a different identity, and only after a share; messages to the old identity stop reaching the person. Never "key change" |
| `KeyReplaceRemoves` | *proposed:* {identity} is then removed: | §5.2, an added identity, before identities §7's Remove list; must say the removal follows the new identity |
| `KeyReplaceTurnsOff` | *proposed:* {identity} is then turned off. You can still read its conversations and contacts, but it cannot send or receive messages. | §5.2, the first identity; must say what stays (to read) and what stops (send and receive) |
| `KeyReplaceOldNewer` | *proposed:* A newer version of InterWeave may be able to use this identity's key. To try to keep this identity, update InterWeave first. | §5.2, "possibly a newer version's"; must say the choice is the update or the replacement, and claim no more than *may* |
| `KeyReplacePending` | *proposed:* In some conversations, this identity's card has not reached the contact yet. If InterWeave has not sent the card yet, replacing the identity cancels it. If InterWeave has already sent the card, it cannot be canceled, and the contact may still receive it. | §5.2, only when the old identity has own-card rows not yet Accepted by remote transport; must not promise a cancellation the client cannot make |
| `KeyReplaceConfirm` | *proposed:* Replace | §5.2 confirm |
| `KeyReplacing` | *proposed:* Replacing {identity}… | §5.3 |
| `KeyReplaced` | *proposed:* {new} replaces {old}. Your contacts see {new} as a new identity after you share its card with them. | §5.4, the new identity's page; must name both identities and that contacts see the new one only after a share |
| `KeyReplacedBy` | *proposed:* {new} replaces this identity. | §5.4, the first identity's page after it was replaced |
| `KeyReplaceFailed` | *proposed:* InterWeave could not make the new identity. {identity} has not changed. Choose Replace with a new identity to try again. | §5.4, step 1 failed; true only because nothing of the old identity is touched before step 1 (§5.3) |
| `KeyReplaceUnfinished` | *proposed:* {new} replaces this identity, but the replacement did not finish. | §5.3, the old identity's page after a stop between steps 1 and 4 |
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
release (§5, §7); they stay as the record. The answers on refusals and
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

Asked of architect-cto with part 2 (2026-11-22). §5 is built on the
reading given with each, marked where it applies, until ruled:

- **Q-link**: the client records, when it creates the replacement,
  which identity it replaces, so that a replacement cut between its
  steps finishes (Finish replacing) and never makes a third identity,
  and so that the share-again list and `KeyReplacedBy` can name the
  other identity. Is that record the store's to keep (STATE.md), and
  is a client allowed to keep it, given that the client keeps no
  record of removals (ADR-0061 decision 7, A 2026-11-04)? Read here
  as yes, kept until step 4 is done for an added identity and for
  life for the first.
- **Q-list**: the share-again list is kept, beyond the page, until
  each row has been shared from or the person presses Done, because
  for an added identity its source conversations are deleted in step
  4. Read here as a stored list of {remote route, conversation title}
  owned by the new identity. The person's reason: otherwise leaving
  the page loses, for good, the only record of who should hear about
  the new identity.
- **Q-trust**: does the new identity take the old identity's trusted
  peers? Trust is per endpoint (ADR-0067 decision 6), and Add an
  identity starts with none. Without them, every reply to a card the
  person shares from the list is refused until they trust that peer
  again, one by one, for peers they already chose to trust. Proposed:
  the replacement takes the old identity's trusted set; a replacement
  is the same person's way out of a lost key, not a new acquaintance.
  Not built until ruled (acceptance 29).
- **Q-carry**: does the new identity take the old identity's
  contacts (name, photo, routes) as its own? Your rule removes an
  added identity's contacts with it; the question is only whether
  the new identity gets a copy first. Proposed: yes, so that the list's
  rows open a conversation with a named contact rather than a bare
  route. The counterparty side is unchanged: they still see a new
  identity, and hold their old contact until they accept the new
  card. Not built until ruled (acceptance 29).
