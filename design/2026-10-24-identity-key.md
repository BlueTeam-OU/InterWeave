# An identity whose key cannot be used, and a new key — desktop and Android

Status: proposal, 2026-10-24. Its two product rules, what happens to
the refused file and when a missing file is a loss, are architect-cto's
DECISION 01a16d8a (§12), which amends ADR-0063 decision 1.

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
contacts will see a key change.

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

A new key matters to other people. A contact who accepted this
identity's card holds its old key; the next card they receive carries
the new one, and their client shows it as a key change — "this might be
someone else" (`CardKeyChange`). Nothing reaches them before that card:
a new key changes nothing they see until the person shares the card
again.

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
state until a key opens or a new key is made. Nothing is shown before a
refusal: an identity whose key opens has no line about it.

Each refusal is named by its kind, in the person's terms, never by the
error's name. A refusal comes from one of two places: the file read
(`IdentityKeyFileError::Persist`, from `read_private_within`, before
the record is parsed) or the record (`IdentityKeyError`); both are
mapped here, and nothing else refuses at open:

| kind | refusal | platform | reason line | actions |
|---|---|---|---|---|
| no longer usable here | `Cipher(KeyInvalidated)`, `Cipher(KeyUnavailable)` | Android | `KeyLostDevice` | Make a new key |
| protected less than this device requires | `CustodyDowngrade` | Android (any platform with a wrapping key) | `KeyBelowDevice` | Make a new key |
| sealed by another device | `SealedUnreadableHere` | desktop | `KeyOtherDevice` | Make a new key |
| possibly a newer version's | `UnsupportedVersion(v)` with `v` above this build's version; `UnknownCustody` | both | `KeyNewerVersion` | Make a new key (its confirmation differs, §5.1) |
| damaged | `NotARecord`, `Truncated`, `UnsupportedVersion(v)` with `v` below this build's version, `EndpointMismatch`, `KeyMismatch`, `Cipher(Unauthentic)`; the read's `TooLarge`, and its `Read` with `InvalidData` (bytes that are not text) or a permission refusal while `identity-keys/` is a directory this user owns (a file its owner cannot read) | both | `KeyDamaged` | Make a new key |
| not protected | the read's `NotPrivate` for the file (readable or writable by another user, a link, another user's file), or for an `identity-keys/` directory this user owns with a loose mode | both (in practice the desktop) | `KeyExposed` | Make a new key (which first makes such a directory owner-only, §5.2) |
| key folder not safe | the read's `NotPrivate` for an `identity-keys/` that is a link or not a directory at all; and any read refusal while `identity-keys/` is a directory another user owns, which the client checks itself, since the read judges the directory's mode and not its owner (a root-owned 0700 directory reads as a permission refusal) | desktop (Android's is the app's own) | `KeyDirUnsafe` | Try again, after the person fixes the folder; no new key |
| temporarily unusable | `Cipher(Platform)`; the read's `Read` for any I/O error but the two in the "damaged" row and an other-owner directory | both | `KeyNotNow` | Try again, then Make a new key |
| missing | no file, for an identity that has shared its card (§7) | both | `KeyMissing` | Make a new key |

The client must tell these read refusals apart: today `open` turns
every read error into one string (`IdentityKeyFileError::Persist`),
which cannot carry this mapping; the error's shape is rust-ui-dev's.
A refusal that persists on every attempt is never "temporarily
unusable", since Try again would repeat it for ever.

`Random` is not an open refusal: it can only fail the making of a key
(§5.3). `UnknownCustody` carries no byte, so the client cannot tell a
future custody from a damaged one; the "newer version" line says only
that a newer version *may* read it, which is true of both. A version
below this build's was never written by any release, so it is damage.
"Not protected" offers no way to tighten the file's permissions: the
key may already have been read, and a new key is what makes the card
trustworthy again (architect-cto's, §12).

Every reason line is followed by the same consequence line,
`KeyCannotShare`: until this identity has a key, its card cannot be
shared; messages are not affected.

## 4. Where the state is shown

Three places, and no others: not on Chats, not as a notification, not
in a conversation. The one thing it blocks is reached through these.

### 4.1 The identity's card page (cards §5.2)

Where the page shows the identity's fingerprint (`human-client-ui.md`
§13: beside the PeerId), it shows instead a block:

- the heading `KeyStateTitle` ("This identity's key cannot be used"),
  with the warning icon before it, in `error`;
- the reason line of §3's table, then `KeyCannotShare`;
- for "temporarily unusable": **Try again** (`TryAgain`, filled
  tonal), and **Make a new key** (`KeyMakeNew`) as a text button after
  it;
- for "key folder not safe": the reason line is `KeyDirUnsafe`, and
  under it, on a line of its own, `KeyDirUnsafePath` ("Folder:
  {path}"), the path isolated (left to right, never translated),
  selectable and copyable as `human-client-ui.md` §11 asks of an exact
  identifier; the path appears nowhere else. Then **Try again** (`TryAgain`,
  filled tonal) alone; a new key cannot be made there until the folder
  is fixed. Try again opens the key once more, as in §6, and whatever
  the folder then holds decides the state: the old file opens, or is
  refused by its own kind, or is missing (and §7 applies); the person
  is not told to keep or move the files;
- for every other kind: **Make a new key** (`KeyMakeNew`), filled
  tonal (not filled: Save stays the page's one filled button).

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
and `ShareCardNote` with the reason line and `KeyCannotShareHere`
("You cannot share your card until this identity's key can be used").
The primary becomes **Open your card** (`KeyOpenCard`), which opens
§4.1 for this identity; Cancel stays; Edit your card is not offered
(it is the same page). Share is never shown disabled, as cards §5.3
keeps for every state.

This state wins over cards §5.3's "no name yet": with neither a key
nor a name, the sheet shows this state, since Open your card leads to
the page where the name is added too, and a name alone would not make
the card shareable.

From §4.1, after a new key is made, Back returns to the sheet, which
then shows the preview and Share as usual. The offer in an empty
conversation (cards §5.3) stays: it opens this sheet.

## 5. Make a new key

### 5.1 The confirmation

**Make a new key** opens a dialog (Material's basic dialog; on Android
`AlertDialog`'s placement):

- title `KeyNewTitle`: "Make a new key for {identity}?";
- `KeyNewBody`, one paragraph: your contacts who have your card will
  see that your key changed the next time you share it with them, and
  their app will tell them it might be someone else, so tell them it
  was you; nothing changes for them before that;
- `KeyNewOld`, only when a file is to be deleted (every kind but
  "missing"): the old key is deleted and cannot be used again; for
  "possibly a newer version's", `KeyNewOldNewer` instead: a newer
  version may be able to use the old key, and once it is deleted,
  updating will not bring it back (architect-cto, Q-file);
- `KeyNewPending`, only when this identity has own-card rows not yet
  Accepted by remote transport: those still held are cancelled, and one
  already on its way may still arrive with the old key (§5.2);
- actions **Cancel** (`Cancel`) and **Make a new key** (`KeyNewConfirm`),
  filled, not in `error`: what is deleted could not be used here, and
  the cost (contacts told of a change) is said above. For "possibly a
  newer version's", the confirm is filled in `error`, since something the
  person may still have is lost.

Focus opens on Cancel, so Enter alone never makes a key. Back, Escape
and a tap on the scrim cancel.

### 5.2 Doing it

Confirm: the dialog closes and the block of §4.1 shows `KeyMaking`
("Making a new key…") with an indeterminate progress indicator in place
of its actions, for as long as it takes; it is not cancellable, since a
half-made key is the one outcome the step must never leave. The rest of
the page stays usable.

The step, in architect-cto's order (Q-order, §12), on both platforms:
the person confirms; when `identity-keys/` is a real directory this
user owns with a loose mode, the client makes it owner-only first
(Q-dir, §12), since nothing can be removed or created in it otherwise
(the folder is shared: another identity whose own file is private then
opens at its next open, with no state, since only the folder was
loose);
the old file is removed (on Android the new seal
would destroy the old wrapping key anyway, one alias per identity); the
new key is created. **Every point the step can stop at resolves to a
state the person already knows**: stopped before the removal, the
identity is in this state as before; after the removal and before the
new key is stored, the next open finds no file: for an identity that
has shared its card, the share record is still there and it is the
"missing" state, never a silent new key (§7), where the person confirms
again; for one that never shared, §7's silent first key applies at the
next open, since no contact holds the old key; after, it has its new
key.
The client never shows "made" without a key it can open, and never
shows a key it holds but cannot open. It never reports success before
the new key is stored where the next start will open it.

Its own pending cards go with the old key: every own-card row of this
identity that has not reached Accepted by remote transport was signed
with the old key. The step withdraws every such row the client still
holds, Not sent ones and Sending ones not yet handed to the transport
(where the client cannot tell a Sending row it holds from one the
transport took, it treats every Sending row as taken),
as cards §5.3's Cancel does for a Not sent row but with no Undo, since
the card could only deliver a key the person no longer has. A Sending
row already handed to the transport cannot be withdrawn and may still
arrive; its conversation is in the share-again list either way, so
the next share shows the counterparty the new key. The confirmation
says so first (`KeyNewPending`), shown when any such row exists.

### 5.3 The result

**Made.** The block is replaced by the fingerprint row with the new
fingerprint, and under it `KeyMade`: "This identity has a new key.
Your contacts have the new key only after you share your card with them again."
Then, when any conversation of this identity holds or may hold the old
key — `card_shared_at` set, or an own-card row not yet Accepted by
remote transport when the step ran (withdrawn or on its way):
`KeyShareAgainTitle`
("Share your card again in these conversations") and one row per such conversation, by
the conversation's title, most recently shared first, each opening that
conversation with the share sheet open (cards §5.3). The list is the
answer to "who will see the change": those, and nobody else. It shows
on this page until the person leaves it; it is not stored.

**Failed** (the key could not be made or stored — `Random`, a cipher or
storage refusal): the client opens the key file again and the block
shows the kind that open finds, with the failure line under the
actions in `error` and **Make a new key** again:

- the old file was already removed: the kind is "missing", and the
  line is `KeyNewFailedGone` — the saved key is gone and a new one was
  not made — so the person is not told less than happened. This holds
  for this session for an identity that never shared too, since the
  person asked for a key and must learn it was not made; at its next
  start §7's silent first key applies;
- the old file is still there (the step stopped before removing it):
  the kind as before, and the line is `KeyNewFailed`.

Pending cards cancelled by the step stay cancelled; they could only
have delivered the old key.

## 6. Try again ("temporarily unusable")

Try again opens the key once more. Opened: the block is replaced by the
fingerprint row, nothing announced beyond §8. Refused again with the
same kind: `KeyNotNowAgain` ("InterWeave still could not use the key.") under the
actions; with another kind: that kind's block.

## 7. Next to other states

- **The store's key** (cards §7): when the store cannot be opened, its
  screen comes first and nothing of this note shows until the store is
  open. A Keystore change that invalidates the store's key may
  invalidate identity keys too; after the store's recovery, each such
  identity is in this state, with its own line.
- **A missing key file** (Q-missing, architect-cto's): a key is made
  silently only for an identity that has never shared its card —
  `card_shared_at` absent on every conversation that speaks as it (the
  share record alone, architect-cto's Q-route).
  Any other identity without its file is in this state as "missing",
  so its contacts are never shown a key change the person was not told
  of first. The same holds when the whole `identity-keys/` directory is
  missing outside the recovery flow.
- **Restore on fresh state** (identity note `IdR1NewKeys`): the person
  was told before the phrase that every identity gets a new key; after
  it, the keys are made without this state.
- **Remove an identity** (cards §5.2) is unchanged and needs no key; its
  confirmation still names the key as going with it.
- **The command line** (ADR-0063 d. 6: every action has a command-line
  form): making a new key there needs a flag named for the key change,
  as the discard has its confirmation flag. Its shape is rust-ui-dev's
  and architect-cto's (§11).

- **The store unreadable or degraded** (cards §5.1, §7): the step
  writes the store only to cancel this identity's pending cards (§5.2).
  Where the store cannot be written and such a card exists, the step is
  not taken and `KeyNewFailed` shows, since a key made then would leave
  a card with the old key still to be sent; with no pending card, it
  runs as usual. The share-again list needs the store's conversations;
  where they cannot be read it is not shown and `KeyMade` stands alone.

## 8. Accessibility

**Reading order**, §4.1's block: the heading (read with "warning"), the
reason line (for "key folder not safe", `KeyDirUnsafe`, then
`KeyDirUnsafePath` read as one item), `KeyCannotShare`, then the actions; it sits where the
fingerprint row would, so it is read in the page's order. §4.2's row
reads its third line with the row: "{name}, {route label}, key cannot
be used". §4.3's sheet: title, "as {identity}", the reason line,
`KeyCannotShareHere`, Cancel, Open your card.

**Focus.** Every case, with its check:

| case | focus goes to | check |
|---|---|---|
| §4.1 opened | the page title (focusable heading), as cards §9's table for Your card | keyboard |
| the share sheet opens in this state | Open your card (cards §9's "Share, else Add your name" has neither here) | keyboard |
| Make a new key pressed | the dialog's Cancel | keyboard; screen reader reads title and body |
| dialog cancelled (Cancel, Back, Escape, scrim) | Make a new key | keyboard |
| confirmed, while `KeyMaking` shows | the block's progress line (the button that held focus is gone) | keyboard; screen reader announces `KeyMaking` once |
| made | the new fingerprint row; `KeyMade` announced once | keyboard; screen reader |
| failed | the failure line (`KeyNewFailed` or `KeyNewFailedGone`), which is announced by being focused, not again | keyboard; screen reader |
| Try again, opened | the fingerprint row | keyboard |
| the `KeyDirUnsafePath` line ("key folder not safe") | reached by keyboard in reading order before Try again, so the path can be selected and copied; it opens nothing | keyboard; screen reader reads it once |
| Try again, refused again | `KeyNotNowAgain` | keyboard; screen reader |
| a row under `KeyShareAgainTitle` | the conversation's share sheet, as cards §5.3 opens it | keyboard |
| Open your card from the sheet | as cards §5.2 | keyboard |
| Back from §4.1 to the sheet, key made | the sheet's Share | keyboard |
| Back from §4.1 to the sheet, no key | Open your card | keyboard |
| the page goes to the background during `KeyMaking` | on return, as "made" or "failed" by what the step left; the announcement is made on return, once | keyboard; screen reader |

On Android every screen-reader check is a blocked precondition while
TalkBack does not reach the app (cards §8); the keyboard checks run.

**Not by colour alone**: the warning icon and the words carry the
state; `error` repeats them. The icon's colour and the heading in
`error` hold 4.5:1 on every surface the block can sit on — the page's
`surface` and, in the sheet, the sheet's container — in light and
dark.

**Targets**: Try again, Make a new key and each row under
`KeyShareAgainTitle` are at least 48 dp tall on Android and 24 px on the desktop.

**Motion**: the progress indicator is the platform's; with reduced
motion it is the platform's reduced form.

## 9. Acceptance

On both clients unless said; Android's screen-reader checks blocked as
§8 says.

1. Android, a device row (`human-client-ui.md` §13): remove the
   identity's wrapping key from the Keystore, start the app: the
   identity's page shows `KeyStateTitle`, `KeyLostDevice` and
   `KeyCannotShare`, and Make a new key; conversations send and
   receive as before.
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
6. Make a new key opens the dialog with focus on Cancel; Enter cancels;
   the dialog's Make a new key makes the key: the page shows a fingerprint different
   from any before, `KeyMade`, and, when the identity had shared its
   card in conversations A and B, those two rows and no other.
7. After 6, B's Share sends a card that the counterparty's client shows
   as a key change with the old and new fingerprints (§13's key-change
   criterion), and A's counterparty sees nothing until A shares.
8. Killed during `KeyMaking`, at each point the step can be cut:
   before the removal, the next start shows this state as before;
   after the removal and before the new key is stored, the "missing"
   state for an identity that had shared its card, with no key made
   by itself, and a new key with no state for one that never shared;
   after, the new key's fingerprint. Never a fingerprint that
   a share then contradicts, and never this state while a key opens.
9. Making the key fails after the removal (a test seam that fails the
   new key's write): "missing" with `KeyNewFailedGone` and Make a new
   key; failing before the removal (a seam that fails it):
   the previous kind with `KeyNewFailed`; for an identity that never
   shared, failing after the removal shows "missing" with
   `KeyNewFailedGone` until the app restarts, then a key with no
   state.
10. A key that cannot be opened transiently (a test seam returning a
    platform cipher error once): `KeyNotNow` and Try again; Try again
    opens it and shows the fingerprint.
11. An identity that has shared its card, its key file removed,
    starts in the "missing" state and makes no key by itself; with the
    whole `identity-keys/` directory removed, every such identity does the same;
    an identity that never shared gets a key with no state shown.
12. Focus follows §8's table, row by row, by keyboard; on the desktop,
    the screen reader reads §8's order and each announcement once.
13. The identity has a card row Not sent in conversation C, and one
    Sending in D that the transport has not taken (a test seam holding
    it), when the key becomes unusable: the confirmation shows
    `KeyNewPending`; after Make a new key, both rows are withdrawn with
    no Undo, neither is sent afterwards (a restart included), and C
    and D are in the share-again list.
    Where the client cannot tell a Sending row it holds from one the
    transport took (§5.2's fallback), D is instead left to arrive, and
    D is still in the share-again list; the build states which of the
    two it does.
14. A key file of non-text bytes, and one of mode 0000, each show
    `KeyDamaged`, not `KeyNotNow`.
15. Desktop: `identity-keys/` at mode 0755 shows `KeyExposed`; Make a
    new key makes the directory owner-only and the key, and the next
    start opens it.
16. Desktop: `identity-keys/` replaced by a link to a private directory
    shows `KeyDirUnsafe` with the path and Try again, and no Make a new
    key; the link is not followed, changed or replaced; after the
    person puts a private directory there, Try again shows the state
    the open then finds.
17. Desktop: `identity-keys/` owned by root with mode 0700 shows
    `KeyDirUnsafe`, not `KeyDamaged`, and no Make a new key.
18. Desktop: `identity-keys/` replaced by a regular file shows
    `KeyDirUnsafe` with `KeyDirUnsafePath`, and no Make a new key.

## 10. Strings

The Text column is language-culture-en's final English (2026-10-24);
the third column is what it must say. `{identity}` is the identity's card name, or its route
label without one (cards §5.7.6); `{path}` is the folder's full path
as the platform shows it, isolated as a path, never translated.

| Key | Text | Where; must say |
|---|---|---|
| `KeyStateTitle` | This identity's key cannot be used | §4.1 heading |
| `KeyLostDevice` | This phone no longer allows InterWeave to use this identity's key. | Android, invalidated or gone; must not claim a cause the client cannot know |
| `KeyBelowDevice` | This identity's key has weaker protection than this phone requires, so InterWeave does not use it. | a downgrade; must say it is refused on purpose, not broken |
| `KeyOtherDevice` | This identity's key was saved on another device and works only there, not on this computer. | desktop, a sealed record |
| `KeyNewerVersion` | This version of InterWeave cannot use this identity's key, because the key file has a format this version does not know. A newer version may be able to use it. To try to keep the key, update InterWeave first. If you make a new key, you lose this key. | must name the way to keep the key before the new-key offer, and say only that a newer version *may* read it (§3: the client cannot know) |
| `KeyDamaged` | This identity's key file is not valid, so the key cannot be used. | every damaged or foreign record |
| `KeyNotNow` | InterWeave could not use this identity's key. The problem may be temporary. Try again. | a platform error; must suggest trying again, not a loss |
| `KeyMissing` | This identity's key is missing from this device. | a lost file (§7); must not suggest the person deleted it |
| `KeyCannotShare` | You can still send and receive messages. You cannot share this card until the key can be used. | after every reason; must say what is blocked and what is not |
| `KeyMakeNew` | Make a new key | §4.1 action |
| `KeyStateRow` | Key cannot be used | §4.2 row line |
| `KeyCannotShareHere` | You cannot share your card until this identity's key can be used. | §4.3 sheet |
| `KeyOpenCard` | Open your card | §4.3 primary |
| `KeyNewTitle` | Make a new key for {identity}? | §5.1 title |
| `KeyNewBody` | The next time you share your card with a contact who has it, their app will show a warning that the card might be from someone else. Tell them it was you. Until then, nothing changes for them. | §5.1; must name the change, the warning they will see, and that nothing happens before a share |
| `KeyNewOld` | The old key will be deleted and cannot be used again. | §5.1, every kind but missing |
| `KeyNewOldNewer` | A newer version of InterWeave may be able to use the old key. To try to keep it, update InterWeave first. If you make a new key, you lose the old key. Updating later will not bring it back. | §5.1, newer version; must say the choice is the update or the loss, and claim no more than *may* |
| `KeyNewConfirm` | Make a new key | §5.1 confirm |
| `KeyMaking` | Making a new key… | §5.2 |
| `KeyMade` | This identity has a new key. Your contacts have the new key only after you share your card with them again. | §5.3 |
| `KeyShareAgainTitle` | Share your card again in these conversations | §5.3 list heading |
| `KeyNewFailed` | InterWeave could not make a new key. Choose Make a new key to try again. | §5.3, failed before the old file was removed; says nothing about what changed |
| `KeyExposed` | Other users of this device may be able to read or change this identity's key file. To be safe, InterWeave does not use this key. | §3 "not protected"; must say why it is refused, and not suggest fixing the file |
| `KeyNewFailedGone` | The old key was deleted, but InterWeave could not make a new key. Choose Make a new key to try again. | §5.3, failed after the old file was removed; must say the saved key is gone and the new one was not made (architect-cto, Q-order) |
| `KeyDirUnsafe` | placeholder: The place where this identity's key is kept is not a regular folder that only your user account owns, so InterWeave does not use the key. Make it such a folder, then choose Try again. | §3, desktop only; must be true for a symbolic link, a folder owned by another user account, and a file where the folder should be; must say what to make of it, since the client never changes what it does not own; awaiting language-culture-en (review round 5) |
| `KeyDirUnsafePath` | placeholder: Folder: {path} | §4.1, desktop only, the line under `KeyDirUnsafe`; a label and the path, `{path}` last so no punctuation touches it; awaiting language-culture-en |
| `KeyNewPending` | In some conversations, your card has not reached the contact yet. If InterWeave has not sent the card yet, making a new key cancels it. If InterWeave has already sent the card, it cannot be canceled, and the contact may still receive it with the old key. | §5.1, only when this identity has own-card rows not yet Accepted by remote transport; must not promise a cancellation the client cannot make (review round 3, F11) |
| `KeyNotNowAgain` | InterWeave still could not use the key. | §6 |

`TryAgain` and `Cancel` are the existing keys.

## 11. Deliberately left out

- **Why** a Keystore key was invalidated: the client cannot know, so it
  names no cause (`KeyLostDevice`).
- **Moving or exporting a key**, and any way to bring an old key back:
  ADR-0064's.
- **A notice on Chats or a notification** for this state: it blocks
  only Share, which leads here.
- **Telling contacts automatically** after a new key: a share is the
  person's act (ADR-0059), so the list offers it and sends nothing.
- **The command-line form's flag and output**: rust-ui-dev's, under
  ADR-0063 d. 6, with architect-cto.
- **The fingerprint row's own design and the key-change proposal**:
  built from `human-client-ui.md` §3 and §13 directly; their
  interaction design is owed to cards (Stage 21 backlog), and this note
  only names where it replaces the row.

## 12. Questions

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
  gone from the ADR amendment (§7).

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
  standing where the directory should be is read here as the same
  class: the client replaces nothing it did not make (review round 5).
