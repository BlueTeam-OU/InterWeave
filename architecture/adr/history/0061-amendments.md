# ADR-0061 — amendment notes

### Amendment 2026-10-21 — An identity has a signing key of its own

ADR-0063, the same day, gives every user identity an Ed25519 key pair
of its own — generated with the identity from the CSPRNG, never derived
from the node's transport seed, sealed as the store key is (ADR-0060
decision 3), never through the seed's custody seam — carried in
the identity's card and proving it. Decision 1 names it beside the
card. Nothing else moves: the key encrypts nothing and grants no trust
until the layer ADR-0064 proposes is accepted; the node's phrase
restores the node and not the identities, which is why the key is not
derived from the seed.

### Amendment 2026-10-21 — Remove deletes the identity key with the identity (ii)

With ADR-0063 A 2026-10-21: Remove of an added identity deletes
its key file and its Keystore alias beside the endpoint, the
conversations, the contacts and the card, since the key is the
identity's; a counterparty keeps the contact it accepted.

### Amendment 2026-10-28 — An identity's EndpointId is derived from its key, and trust is per identity

ADR-0067, the owner's direction of 2026-10-28: a user identity is an
endpoint whose EndpointId is derived from its own Ed25519 key pair as
the PeerId is from the node's, the same for every endpoint kind, and
trust is per identity. Decision 1 read "a configured EndpointId of the
profile together with the card" and "one trust set"; it now reads a key
pair with a derived id and an admission set the transport derives
from the operator's list and every endpoint's overlay set (its
identities', or any endpoint the operator gave one). Decision 2's "configuring an endpoint" is
generating its key. Decision 7's rename changes the card and the local
alias, never the id, and a new identity on a freed alias has a new key
and id. Decision 8's automated endpoint is a key the deployment mints.
Decision 5's trust sentence — "an identity inherits the node's
trust" — is replaced by the per-identity set; decision 6's export
carries key references and no key; decision 9 (one identity at a
time) is unchanged and the Trust screen is the identity in front's
under it. Propagation: the Status line; decisions 1, 2, 5, 6, 7, 8
and the Security implications; this note; the log row; the digest
entry.

### Amendment 2026-11-04 — The old-contact warning is Remove's, not the add's

Decision 7 read "the client saying before the add that counterparties
may still hold a contact on the old route with the old card's name and
photo". That placement was written when a label was the EndpointId and
a new identity on a freed label took the old route. Since ADR-0067 a
new identity has a new key and a new EndpointId, so nothing the add
does can be confused with a removed identity by a counterparty; what
remains true is a consequence of the removal, and the client keeps no
record of removals from which an add could say it. Decision 7 now
places the sentence in Remove's confirmation and the add carries no
such line (the Stage 22 identities design's §16 Q7 (b), ruled by
architect-cto). Propagation: `human-client-ui.md` §13's Remove and Add
bullets, the plan's §30 exit gate, this note, the log row, the digest
entry.

### Amendment 2026-11-04 — The desktop's one notification form is the redacted one

Decision 9 made the redacted form the lock screen's exception to a
notification that names its identity, written for a phone whose system
redacts on the lock screen. The desktop's freedesktop notification
interface gives the client no lock state, and a notification shown
before the screen locks persists on it, so a desktop notification that
named the identity would name it on a locked screen. Decision 9 now
says that where the platform cannot tell the client a locked screen
apart, the redacted form is the only form — a count, no name, no
content — and the identity is shown after the open (the Stage 22
identities design's §16 Q6, ruled by architect-cto; PR #553's review,
F2). Propagation: `human-client-ui.md` §10 and its preview bullet,
its §13 precedence sentence over the contact-cards design's §5.7.5,
this note, the log row, the digest entry.

### Amendment 2026-11-04 — Opening a notification that has no one identity

Decision 9 said a notification names its identity and opening it
switches to that identity; the lock screen's redacted form named none
and the switch followed the unlock. The Stage 22 identities design
(#552 §9, §16 Q7 (a)) showed the lock screen one entry for every
identity — one group per identity would tell someone holding the
locked phone how many identities have news — and found that such an
entry, the desktop's redacted form (the amendment of the same day) and
a group's summary have no one identity to switch to. Decision 9 now
says: with news in one identity, the switch to it, visibly; with news
in two or more, the switcher over the identity in front's
conversations, naming the identities with news, nothing switched until
the person picks — the alternative, switching to the identity with the
newest message, is an action whose identity is implied by state the
person cannot see, which the decision forbids. Ruled by architect-cto
as the design proposed. Propagation: `human-client-ui.md` §10 and
§13's notification and lock-screen bullets, the plan's §30 exit gate,
this note, the log row, the digest entry.

### Amendment 2026-11-21 — On the phone the first identity is the entry the app writes after its key is sealed

Decision 1 named the first identity as the profile's "authored human endpoint (`human-client-ui.md` §2 step 4; `runtime.android.endpoint` on the phone)". Under ADR-0067 that endpoint is named by its key, and on the phone no key exists when the app first writes its profile document, so the document holds no identity until onboarding seals the first key (ADR-0067 A 2026-11-21 has the ruling and what it refused). The parenthesis now says what the phone's first identity is: the entry `runtime.android.endpoint` names in the app's own document, an entry of `config.yaml` as the desktop's is and never one of the overlay's `added`. Nothing of the decision changes; making the first identity an added one was the alternative refused, since decision 7's rule for it (never removed, only turned off) rests on its standing.

### Amendment 2026-11-22 — A removal that is a replacement keeps the list of routes to share the new card with

Decision 7's Remove deletes an added identity's conversations, contacts and card, and (A 2026-11-04) the client keeps no record of removals. ADR-0067 decision 5 removes an added identity whose key is lost once its replacement exists, and the replacement's design needs two records that sentence seemed to forbid: which identity the new one replaces, so that a replacement cut between its steps is finished and never repeated into a third identity, and the list of conversations to share the new card in, whose source rows the Remove deletes. Both are allowed as bounded: the replaces record exists only while both identities do and is deleted with the removed one; the list is remote routes alone, held by the new identity, and the Remove's confirmation says it is kept, since the confirmation is the list of what goes. See ADR-0067's note of the same day for the reason.

### Amendment 2026-11-23 — Remove has one order, and the administrative remove is last

Decision 7 listed what Remove takes and fixed no order. ADR-0063
decision 2 had put the key path first that day, and said a failure
after it leaves the identity listed, to be finished by Remove again.
The second review of InterWeave #751 found that false once the
administrative remove has committed (F7): the endpoint is gone, so the
identity is, and a connection lost after the commit, or a store
deletion that failed after it, left rows or a report of failure for an
identity no list showed. Ruled by architect-cto on ux-designer-01's
proposal: the key path, the platform key, the store's rows, then the
administrative remove last. Every failure before that remove commits
then leaves the endpoint configured, so the identity is listed, the
person is told what was removed, and Remove again completes; the one
state after a commit is a lost answer, which the client reads back
from `admin.endpoints.list`, as the add's is: the id absent is Remove
done, and the id present, or a list that does not answer, is a Remove
that did not finish. A remove of an id no entry holds is the daemon's
`EndpointUnknown` in the method's contract text, read as done once the
list confirms it.

One constraint comes with the order. The endpoint keeps its lease
until the remove commits, so a message could arrive after the store's
rows were deleted; were it stored and the remove then committed, the
store would hold rows of an identity no list shows. So the client
admits nothing to its store for that identity from the deletion until
Remove completes or the client next starts. That also covers ADR-0028's
overlay left ahead of the runtime: the remove is answered `Internal`,
the identity is still served until the next start and gone after it,
and nothing of it is left by then. How the client declines, and what
the sender is answered, is rust-ui-dev's to build inside the
contracts, and theirs to report if no mechanism holds it. Rejected:
the remove before the store's deletion, which leaves rows of an
identity no screen shows whenever the deletion fails. As this was
ruled, Remove was not built and `admin.endpoints.remove` was not on
`main`.

Propagation: decision 7; ADR-0063 decision 2 with its note, log row
and digest entry; this note; the log row; the digest entry. The design
notes (identities section 7 and its acceptance, contact-cards 5.2)
follow on #751.

### Amendment 2026-11-24 — An unfinished Remove needs no special case: rows follow the endpoint and the key

The third review of InterWeave #751 found the amendment of 2026-11-23
wrong in its clause for ADR-0028's overlay left ahead (F10), and
ux-designer-01 showed why in three parts, each read against the tree
before this ruling. The client could not tell that case from a write
that failed: the composition answered `Internal` for both. On the
desktop the client's start is not the daemon's, so a window that ended
"when the client next starts" ended while the daemon still served the
endpoint; rows arrived again, and at the daemon's start the endpoint
went and left rows of an identity no list showed, the state the note
above rejects. And `admin.endpoints.list` answered the runtime's view,
so while the overlay was ahead the read-back found the id present and
Remove again could not complete before the daemon started again.

Ruled by architect-cto on ux-designer-01's four-part proposal, with
one condition added to its third part:

1. The overlay left ahead is no case of its own. Every failure of the
   administrative remove, `Internal` included, is a Remove that did
   not finish.
2. The no-admission window is the running client's: it ends when
   Remove completes, when a list that answered does not hold the id,
   or with the client's process. No mark is stored, so the client
   still keeps no record that a Remove began (A 2026-11-04 for
   removals). After a restart the identity receives as any identity
   whose key is missing does, and part 3 is what leaves nothing.
3. At its start, after every reconnection to the daemon and whenever
   `admin.endpoints.list` answers, the client deletes the store rows
   of an identity whose EndpointId the list does not hold and whose
   key path holds no key file. Nothing is deleted on a list that
   failed. The condition on the key is the addition: rows are deleted
   because the identity cannot come back (an EndpointId is derived
   from its key, ADR-0067 decision 2), not because one list lacked an
   id. An endpoint taken out of the configuration by other hands, its
   key still on disk, can be configured again, and its rows are left.
   Remove takes the key path first, so every Remove this decision
   describes meets the condition.
4. While the overlay is ahead the list still holds the id, and Remove
   again ends not finished each time; the identity goes at the
   daemon's next start, by part 3 with nothing of it left.

The same review asked whether a remove the client stopped waiting for
commits before any later answer or never (F12). It does not, and
neither does the add: `LOCAL-IPC.md` (A 2026-10-23) says a lost admin
write may follow a later read, and the endpoint set's methods are not
ordered by connection. So the id present after a lost answer is a
Remove not known to have finished, and the words shown for it must
stay true if the identity goes a moment later. A remove applied late
is the remove the person confirmed; part 3 takes the identity from
the list at the next answer. What the add leans on instead is its own
refusal: an add applied after the client deleted the key file is
refused `key_missing`.

Rejected: a distinct answer for the overlay left ahead, which would
give the client a state it can do nothing with before the daemon
starts again; a stored mark that a Remove began, which part 3 makes
unnecessary; and deleting rows on the list alone, which turns an
edit of the configuration into the loss of an identity's
conversations while its key still exists. As this was ruled, Remove
was not built and `admin.endpoints.remove` was not on `main`.

The fourth review of #751 found part 4 not carried through (F10,
again). Decision 7's earlier clause still said a Remove that stops
before the remove commits "is finished by Remove again", the digest
entry said "Remove again finishes it", and ADR-0063 decision 2, which
this note had left unchanged, said Remove "stays offered and
completes". Each was true of every failure but the overlay left
ahead, where part 4 says Remove again ends not finished each time.
Each of the three now names that exception and what finishes the
Remove there, the daemon's next start. Nothing in the ruling changed.

Propagation: decision 7; this note; the log row; the digest entry.
ADR-0063 decision 2 was left unchanged as this was first written, and
gained the exception on the fourth review, with its note, log row and
digest entry. `LOCAL-IPC.md`'s remove text, not
on `main`, gains the reading of a remove while the overlay is ahead
and of one applied late, on its own supply. The design notes
(identities sections 1 and 7 and acceptance 13) follow on #751.

### Amendment 2026-11-24 — The add, remove and set_advertise methods are IPC 3.0 methods, not a minor (ii)

Decision 2 said the administrative methods that add a configured
endpoint, remove an added one and set an endpoint's `advertise` come
"under ADR-0017's minor rule". It was written on 2026-10-21, when the
IPC was major 2 and three new methods would have been a minor of it.
ADR-0067 decision 7 then moved the IPC to major 3 in the Stage 22
release, and `LOCAL-IPC.md` §Version negotiation and phases states the
major's rule: no production build speaks major 3 before that release
is tagged, so a method the stage adds is a 3.0 method and
`IPC_MAX_MINOR` is 0 at the release. ADR-0067's note of 2026-11-19
corrected the same phrase for the trust methods' endpoint dimension
and did not reach this record; the review of the Stage 22 batch found
the two texts apart (p2p-network-dev-01, 2026-11-24). Decision 2 now
names IPC 3.0 and the pre-release rule. What follows for a builder:
no client gates the three methods on a minor, a mirror that did would
refuse them to every 3.0 connection, and the release is not tagged
with any of them missing, since a build that spoke 3.0 without them
would leave them to arrive as 3.1. Nothing else a reader was told to
build changes: the methods, their capability and their overlay are as
decided. The history notes of ADR-0028 that call these methods "an
IPC minor under ADR-0017" stand as written, being what was true on
their day. Propagation: decision 2; this note; the log row; this
record's digest entry and ADR-0028's, which carried the same phrase;
plan §30 Activate.

### Amendment 2026-11-24 — The replaces record is written before the add and lives until either identity is removed

The note of 2026-11-22 said "the replaces record exists only while both
identities do and is deleted with the removed one", and decision 7 said
the record "is the store's while both exist". `STATE.md` §Logical
tables, written in the same batch, has the record written before the
administrative add of the new endpoint, for the reason that note itself
gave: a process that ends between the add and a later write would
otherwise leave an added identity nothing ties to the old one, and a
second replacement would make a third. So the record exists before the
new identity does, and after a process that ended before the add it
names an identity not yet added; the two texts disagreed on when it
exists (the Stage 22 batch's review, its finding F4 at 658dd3ee).
`STATE.md` was right, and decision 7 now says what it says: written
before the add, deleted in the transaction that removes either
identity, kept beside a first identity that is turned off and never
removed. What the earlier clause protected is kept, and is the bound
that matters: no row names a removed identity, so no record of a
removal exists (A 2026-11-04). A row whose new identity is not yet
added names no removed identity either; it is what the interrupted
replacement is finished from. The 2026-11-22 note stands as written,
being the record of its day; this one corrects its one clause.

Propagation: decision 7; this note; the log row; the digest entry.
`STATE.md` is unchanged.
