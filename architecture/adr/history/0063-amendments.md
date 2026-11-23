# ADR-0063 — amendment notes

### Amendment 2026-10-21 — Remove of an added identity deletes its key with it, and a restore on fresh state says every identity gets a new key

rust-ui-dev proposed the acceptance criteria for the card and the key
(plan §29 steps 1–2) the same evening the record landed and asked two
things it did not say: what Remove does to the key, and whether the
recovery flow tells the person about the key change. Ruled: Remove of
an added identity (ADR-0061 decision 7) deletes its key file and its
Keystore alias with the endpoint, the conversations, the contacts and
the card — the key is the identity's — while a counterparty keeps the
contact it accepted, so a later identity on the freed label is a key
change to them; and the phrase-recovery flow on fresh state says,
before it takes the phrase, that every identity gets a new key and
counterparties will see a key change for each (decision 1 now says
so, and IDENTITY-RECOVERY.md item 12 binds `transportctl` to it;
"fresh state" is a state root with no `identity-keys/` directory). The seventeen criteria, two of them
device rows, are in `human-client-ui.md` §13 with their record lines.

### Amendment 2026-10-22 — The record's custody is a floor: a filesystem-only key file is refused at open where a wrapping key exists

Plan §29 step 2 found the desktop with no wrapping key (ADR-0060
decision 3's filesystem-only floor until SPIKE-007) and asked what the
IDK1 file is there and what Android does with such a file. Ruled: the
header carries a custody field under the associated data; the five open
refusals (magic, version, a custody this build does not know, EndpointId against the file name, public key
against the secret) hold on every platform, the AEAD on top where a
wrapping key exists; a platform that holds a wrapping key refuses a
filesystem-only record at open as a downgrade, named, and the person
creates a new key (a key change, decision 5); a desktop refuses a sealed
record as unreadable here; the one re-seal is upward and at adoption —
the desktop re-sealing its own key files when it joins the SPIKE-007
envelope, as the store key does (ADR-0060 decision 3), never at open,
never downward, never on arrival from another device — the hand-copied
state root's files are the desktop's own by then and are re-sealed with
the rest.
The reason is the copy: a key file moved to a weaker platform must not
quietly become as weak as that platform. Propagation: decision 2; this
note; the log row; the digest entry.

### Amendment 2026-10-24 — A missing key file outside recovery is a loss, and a refused file goes with the confirmed new key

Building the Android identity key (rust-ui-dev-02's j53b) and the
person's side of §13 bullets 31 and 33 (ux-designer-01's QUESTION
01a16d88), two gaps: decision 1 gave a new key silently on fresh
state through the recovery flow, where the person is warned, and said
nothing of a key file missing on a root that is otherwise there,
where the build minted a key silently for an identity whose contacts
would see a key change unannounced; decision 2 said the person
"creates a new key" after a refusal and not what becomes of the
refused file. Ruled: a missing file is a silent first key only for an
identity that never shared its card — the share record alone decides,
since the identity's own public key is held nowhere once its file is
gone (rust-ui-dev-02's question; a first draft also named the
accepted-card routes, which record other identities' keys) — else a
key-missing state with a confirmed new key; a refused or exposed file is removed only on that
confirmation and the new key created after the removal (a first
draft had the new key written first, which one Keystore alias per
identity forbids: #411's review), a gap resolving to the key-missing
state, a loose directory of the client's own tightened first and a
link or another owner's directory refused with the fix named (#411's
round 2), with the update offered first for a newer build's record and
a retry first for a cipher refusal, and nothing kept aside. The
alternative of keeping a refused file beside the live one was refused
as a second secret at rest with no reader. Propagation: decision 1;
this note; the log row; the digest entry; `human-client-ui.md` §13's
bullets 31 and 33 read with it; `design/2026-10-24-identity-key.md`
(#411), whose §5.3 states the gap's outcome for an identity that
never shared.

### Amendment 2026-10-27 — The silent first key also counts an own-card row pending outbound, or one that may have left

Building contact-cards 5.3's leaving and share rules (rust-ui-dev-02's
j54, QUESTION 01a17abc), two rules met at one column: 5.3 sets a
conversation's `card_shared_at` when the own-card row reaches Accepted
by remote transport, while the build wrote it when the card was
queued, so the 2026-10-24 amendment's "share record alone" agreed with
the build and not with the design. Written as 5.3 says, a key file
lost while the first card is still Sending — it survives a restart as
pending outbound — left no record, the next start made a key silently,
and the old-key card could still arrive, making the contact's next
card a key change the person was never told of. Ruled: `card_shared_at`
stays at Accepted and nothing earlier, since the stored value is
person-facing and a Sending card can still be cancelled; the silent
key's predicate gains the pending own-card rows of the identity, the
predicate the share-again list already reads, so one store predicate
serves both sites and the two cannot part again; a failed store read
goes on counting as shared. The alternative of writing `card_shared_at`
at queue time was refused as naming a disclosure that may never happen.
The same day, from #467's review (rust-ui-dev-02's QUESTION 01a17afd):
a card cancelled after an attempt that may have reached the remote
(`Cancelled { may_have_reached: true }`, the outcome-unknown class of
the agreed amendment A1, erring toward "may") left neither a share
record nor a pending row, so the window stayed open for it. Ruled: a
conversation records that a card of the identity may have left this
device from the first such attempt, Accepted or not, cancelled or not,
cleared by nothing short of a store discard; the silent key and the
share-again list read that record beside the pending rows, and
`card_shared_at` alone stays what Details shows. The alternative of
accepting the window was refused: the rule exists for exactly the
counterparty who may hold the old key. The record is a store change,
built in rust-ui-dev-02's later batch; until it lands, #467 names the
window as a limit and the plan's §29 carries it.
Propagation: decision 1; this note; the log row; the digest entry;
`design/2026-10-24-identity-key.md` §7 and §8 (ux-designer-01, by
locator); the code on rust-ui-dev-02's branch, one PR.

### Amendment 2026-10-28 — The key is the endpoint's, its EndpointId derived from it; the key change is withdrawn

ADR-0067 makes the identity key this record introduced the key of the
endpoint itself, for every endpoint kind, and derives the EndpointId
from its public key as the PeerId is from the node's. Decision 1 names
that; decision 2's directory is `endpoint-keys/`; decision 4's key
change — "a route whose key differs from the recorded one is a key
change: the proposal says so ... the person may accept the new key,
which replaces the old one in that route" — cannot arise, since a
different key is a different EndpointId and another identity of the
node, and a card whose key does not derive its endpoint is malformed;
the proposal, `--accept-key-change` and the replace rule are withdrawn,
and decision 6's command-line form becomes the creation of a new
identity. The key-missing state (A 2026-10-24) and the silent first key
(A 2026-10-27) stand, resolved by a new identity rather than a new key
under the old name. Propagation: the Status line; decisions 1, 2, 4, 5,
6, the Security and Operational implications (each with a tombstone
in place); this note; the log row; the digest entry; STATE.md's
`contact_routes` and key-file sentences; the README row.

### Amendment 2026-11-23 — A refusal for the key directory's mode alone ends when the directory is private

Decision 2 said a new identity follows "a refused, exposed or
invalidated record" without saying what exposed covers. The desktop
read refused two different things under one name: a key file that is
itself not private, and a key file that is private inside an
`endpoint-keys/` directory this user owns at a loose mode. The design
note for the refusals treated both as "the key may already have been
read, so it is not used again", and then found that the second case
could not be resolved at all: a new key cannot be written into a
non-private directory, and making the directory private lets every key
in it open again (ux-designer's question of 2026-11-23, review of the
identity-key note).

Ruled: exposed is the file's own state. A directory this user owns at
a loose mode shows other users the entries' names where it gives them
the read bit, and where it gives them the search bit the metadata of
each entry whose name they know; the names are EndpointIds and public,
so the search bit alone, which lists nothing, is enough for that
(corrected by the second amendment of this day, below). It gives
no other user a read of an owner-only file, and no other user can
change the mode of a file they do not own. Where the loose mode gives
another user both the write bit and the search bit, that user could
have removed or swapped a file, and each such swap is refused at open
on its own: another owner's
file and a link by the file's rule, this user's other key file by the
envelope's EndpointId against the file's name; a removal is the
key-missing state. So the directory's refusal needs no memory: no key
opens while the directory is loose, each opens once it is private, and
nothing is stored to outlast the fix. This is how the node's own key
already behaves under ADR-0028's directory rule. The directory is
judged first, because the read stops at it and never sees the file: a
file that is itself not private, or missing, inside a loose directory
shows as the directory's refusal until the directory is private, and as
its own state after.

Rejected: a stored mark that keeps the refusal after the directory is
fixed. It records a state no other user could have used, and costs the
store a record whose only reader is this case.

Propagation: decision 2 (the sentence after the ADR-0067 one); this
note; the log row; the digest entry. The identity-key design note's
section 3 and its criterion for a directory at mode 0755 follow in the
note's own pull request.

### Amendment 2026-11-23 — A loose key directory is the person's to fix, and what is not a file at the key path is not a record

Three texts carried from the review of the identity-key note's pull
request (#746, ux-designer's request of 2026-11-23), and one question.
The two reviews of the pull request that carried this note (#748) then
added to it, each in place below: round 1 a correction of the second
text (the three refusals) and a second question (F4); round 2 two
corrections of F4's ruling (F5, F7) and one reading (another user's
regular file that the open refuses).

Decision 1 still said, from the 2026-10-24 amendment, that "an
`identity-keys/` directory the client's own user owns with a loose mode
it makes owner-only itself before that step". The first amendment of
this day ruled that a refusal for the directory's mode alone ends when
the directory is private, and narrowed decision 2 only. The two could
not both hold: a client that tightens the directory itself opens every
key in it again without the person having seen that the directory was
loose, the replaced identity's key among them. The clause is withdrawn.
The client refuses a loose key directory of its own user with the fix
named and does not change its mode; the person makes it private, each
key then shows its own state, and a new identity is offered on that
state alone. As this was written the store's `identity_key::replace`
still tightened the directory, as the Stage 21 text had it; the code
that conforms is rust-ui-dev's.

Decision 2's three causes of "exposed" did not cover a key path that
holds neither a regular file nor a link. Ruled (Q-kind): a directory,
a FIFO, a socket or a device at the key path is not a record and not
exposed, whoever owns it. No key was there to be read, and "other users
may have read this key" would be untrue of this user's own directory at
mode 0700. The client shows it as a record that is not valid (the
design note's "damaged"), resolved by a new identity. The three causes
now say "regular file" where they meant one. An entry of that kind that
another user placed can only have arrived while the directory gave that
user the write and search bits, and it is judged, like every file, only
once the directory is private.

As first written this paragraph said the read "refuses such an entry by
its kind, before it looks at an owner or a mode (`open_private_as`),
under the same error as a file that is not private". The review of the
design note's pull request (#748, round 1) measured three refusals, not
one, and the paragraph is corrected in place. An entry that opens
read-only and non-blocking (a directory at 0700, a FIFO at 0600, a
device with a driver behind it) reaches the kind check and is refused
as not a regular file, ahead of its owner and mode. A socket, or a
device with no driver behind it, does not open at all: the open fails
with `ENXIO` (`ENODEV` is possible for a device), ahead of the kind
check. An entry whose own mode keeps this user out (a directory at
0000, a FIFO at 0200, a socket at 0000) fails the open with `EACCES`,
so there the mode is met first. All three are the same state, a record
that is not valid: repeating the read repeats the refusal, so none of
them is offered as a key to try again. As the review found it, the
store answered the second as "temporarily unusable" (`ENXIO` fell
under the general read error in `IdentityKeyFileError::unusable`); the
code that conforms is rust-ui-dev's.

One reading from the second review of #748: the permission failure
above is about entries that are not regular files. Another user's
regular file that the open refuses with `EACCES` before any owner check
(a root-owned file at 0600) is exposed, as decision 2's third cause
says, and not an invalid record: the record is classed by what the
file is, not by which refusal the read met first, and the client finds
the case by the file's owner. Decision 2 now says so in the definition.

One question from the same review (F4): "resolved by a new identity"
did not say what happens to the entry itself. Ruled: Remove or Replace
of an added identity does not wait on it. One unlink, or one removal
of an empty directory, takes what is at the key path, whether a file,
a FIFO, a socket, a device or an empty directory. A directory that
holds anything is left where it is: the client does not delete
recursively what it did not write, and the step completes with it
there. What the person is told about an entry left behind is the
design note's.

As first written that paragraph ended "The step completes either way,
because an EndpointId is derived from its key, so no later identity
takes that file name and a leftover blocks nothing". The second review
of #748 found two gaps in the sentence, and it is corrected in place.
"Either way" named two outcomes, taken and left as a non-empty
directory, and an unlink or a removal that failed for any other reason
(`EPERM`, `EBUSY`, `EROFS`, `EIO`) was in neither: read literally,
Remove reported done with a key file still on disk (F5). Ruled: the
non-empty directory is the one failure passed over, and a path that
holds nothing has nothing to take; any other failure stops the step,
whatever the entry is. Remove takes the key path first, so a Remove
that stops there has changed nothing and says it failed; a Replace
that stops there is unfinished and offered to be finished, as at any
other refused step of it. And "no later identity takes that file name"
held for new keys only (F7): ADR-0064 decision 7 (Proposed) restores
the same key, hence the same EndpointId and the same path, where a
directory left behind would be in its way. No record in force brings a
key back, so nothing is decided here; what that restore does with the
entry is ADR-0064's to decide before it is accepted. As the first review
found it, the store removed the key path with an unlink alone, which
fails on a directory (`EISDIR`), so Replace stayed unfinished and
repeated the refusal; the code that conforms is rust-ui-dev's.

The first amendment's note gave two bounds that were wrong, and its
paragraph is corrected in place. It said a loose directory "shows
other users at most the file names (where it gives them the read bit;
a search bit alone lists nothing)". With the search bit another user
can also read the size, times, owner and mode of any file whose name
they know, and the names are EndpointIds, which are public, so a mode
of 0711 or 0701 lists nothing and still gives that much. (That list is
itself short, as the review of #748 found: the read bit gives each
entry's kind and inode number with its name, and the search bit gives
the whole of a named entry's metadata. The bound is the entries'
metadata, not a list of fields.) It said
"Where the loose mode is a write bit, another user could have removed
or swapped a file"; removing or placing an entry needs the search bit
as well, so 0720 or 0702 allows neither. Neither correction changes
the ruling: no bit of the directory gives a read of an owner-only
file's contents, and nothing in an entry's metadata is key material.

Propagation: decision 1 (the clause, with its tombstone in place);
decision 2 (the definition of exposed, the sentence after it with the
three refusals, and the Remove or Replace sentences); the
first 2026-11-23 note's second paragraph; this note; the log row; the
digest entry. The identity-key design note already reads so (its
sections 3 and 5, its section 11 for the entry left behind, which
decision 2 hands to it, and criterion 14), and the identities design
note's section 7 for Remove taking the key path first.
