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

### Amendment 2026-11-20 — The add, remove and set_advertise methods are IPC 3.0, and a refusal is an answer

Decision 2 said the three administrative methods were p2p-network-dev's
to shape "under ADR-0017's minor rule", written when they were expected
to follow a released major. Stage 22 opened IPC major 3, which has no
production build before the stage's release, so ADR-0017's pre-release
rule applies and what the stage adds is 3.0 (`LOCAL-IPC.md` §Version
negotiation and phases, ruled 2026-11-19 for the trust dimension).
p2p-network-dev-02 proposed the shapes on 2026-11-20 (their j119) and
architect-cto ruled them the same day. Taken as proposed: the add takes
the key file's name and no public key, writes `advertise: false` and two
empty trust sets, needs the file present, and is safe to repeat; the
remove takes added endpoints alone, refuses the default, drops every
overlay member of the id, and leaves the key file to the client;
`set_advertise` syncs the overlay and updates the directory's answer
source before it answers. Changed from the proposal: an add's or a
remove's refusal is a named outcome in the result, as
`admin.peers.add_address`'s is, where the proposal asked for three new
error codes, so the closed error vocabulary stays as it was; the add has
no `enabled` member, so that a repeated add compares the alias and the
client kinds alone; `admin.endpoints.list`'s row gains `alias`, `source`
and `advertise`; add and remove are audited. Propagation: `LOCAL-IPC.md`
§Method catalogue and its capability list, `ENDPOINTS.md` §Configured
endpoint set, ADR-0067 decision 4, this note, the log row, the digest
entries.

This note and its two rows reached `main` after the amendment of
2026-11-24 (ii) below, which corrected decision 2's phrase on `main`
without it; the two agree, and that note's account of the phrase
still standing is of `main` as it was that day.

### Amendment 2026-11-20 — An added identity's alias is renamed by its own method, and the first identity's alias is the configuration's

Decision 7 says a rename changes "the display name on its card and its
local alias", and plan §30 step 1 puts the rename on the admin
connection, but the three administrative methods decision 2 names (add,
remove, set_advertise) give an alias no door: an added entry's alias is
in the endpoint overlay, which the daemon alone writes. Found by
architect-cto while ruling p2p-network-dev-02's shapes of those three
methods (their j119). Ruled: a fourth method of IPC 3.0,
`admin.endpoints.set_alias`, rewrites an added entry's alias with the
add's alias refusals. The first identity's alias is authored in
`config.yaml`, which no program rewrites (ADR-0067 A 2026-11-19), and an
alias delta in the overlay would leave the file's alias and the
effective one disagreeing for every reader of the file, so the method
refuses an authored entry and that identity's rename changes its card
alone. Not yet confirmed by the builders: sent to p2p-network-dev-02 and
rust-ui-dev with the ruling. Propagation: `LOCAL-IPC.md` §Method
catalogue and its capability list, `ENDPOINTS.md`'s alias paragraph,
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
removal exists (A 2026-11-04). A row whose new identity was never
added names no removed identity either, and no identity the node holds:
the client discards it at the next open and shows the old identity as
before, Replace offered again, so a record alone never shows as
unfinished (`design/2026-10-24-identity-key.md` §5.3; this change's
own review, its finding F3, found the first wording of this note
giving that row the finishing role). Only a record whose add happened
is what the replacement is finished from. So the record leaves in two
ways, with either identity's Remove and by that discard. The 2026-11-22 note stands as written,
being the record of its day; this one corrects its one clause.

Propagation: decision 7; this note; the log row; the digest entry.
`STATE.md` is unchanged; it does not state the discard, which is the
design's until the store change that builds the replacement states it
there.

### Amendment 2026-11-25 — A replaces record whose new identity the list lacks is settled by sending the add again

The note of 2026-11-24 had the client discard, at the next open, a
record "whose new identity was never added", and named no test for
never added. The client's view of what the node holds is
`admin.endpoints.list`, and `LOCAL-IPC.md` has two adds that list does
not show: one that left the overlay ahead of the runtime, answered
`Internal` and loaded at the daemon's next start, and one applied late,
after the client that sent it stopped waiting. A record discarded on
the list alone would in both leave an added identity nothing ties to
the old one, with Replace offered again: the third identity the record
exists to prevent (the review of this change at 29743549, its finding
F5). The key file does not tell the cases apart either, being sealed
before the add in all of them.

So the client reads nothing from the list's silence and sends the same
add again, the one question the contract answers without doubt:
`added` and `unchanged` both mean registered from the answer on, a late
copy of the first add then being `unchanged` too, and the state is the
one the design already has for a replacement stopped after its add,
finished from the record. Only `refused` discards. A refusal writes
nothing, but one of its reasons, `key_missing`, is decided before the
overlay is asked, so an entry left ahead can stand behind it; the
discard therefore empties the new key path and platform key, which
makes any add applied late `key_missing`, sends the administrative
remove, which takes an entry left ahead out of the overlay (`removed`)
and answers `EndpointUnknown` where none holds the id, and deletes the
record with the new identity's rows only on one of those two answers.
Anything else, `Internal` and no answer included, keeps the record for
the next open.

Three sentences close what that order would otherwise leave open (the
review of the first wording of this note at 051c83fb, its finding F7
and two of its risks). Replace pressed on an old identity a record
names settles that record and mints no second key, so two records
never name one old identity. The clean-up rule of 2026-11-24, which
deletes the rows of an identity the list lacks and whose key path is
empty, takes no replaces record and no rows of the new identity a
record names: after a discard that emptied the key path and whose remove did
not answer, that identity can still come back at the daemon's next
start. And no replacement is finished toward a new identity whose key
is missing, whether a discard emptied its path or the key was lost
after the add, which was reachable before this note: the identity is
listed as any whose key is missing is, the person removes it, and the
record goes in that Remove.

The first wording of this note, of the same day, discarded on the
list's silence after undoing the add, and never sent the add again, so
as to leave the old identity as the design shows it after a failure.
It was given up for this one: undoing an add the person asked for
needed every rule above and one more state, where sending it again
needs the contract's two answers. What the person sees changes in one
case, a Replace whose add failed or was cut: at the next open, or on
pressing Replace again, the replacement shows as unfinished with
Finish replacing, where the design's §5.3 shows the old identity as
before; the design's text is its owner's to bring to this
(`design/2026-10-24-identity-key.md` §5.3, unchanged here). The
replacement is not built (`STATE.md` §Logical tables), so no build did
otherwise. The 2026-11-24 note stands as the record of its day; this
one corrects its "discards it at the next open".

Propagation: decision 7; this note; the log row; the digest entry,
its replacement bullet and its Remove bullet's clean-up clause. Two
design texts state the rules this note changes and are their owner's
to bring to it, unchanged here: `design/2026-10-24-identity-key.md`
§5.3 (the discard at the next open, and Finish replacing offered
whatever the new identity's key) and `design/2026-11-04-identities.md`
§7 (the clean-up rule without its exception).
`LOCAL-IPC.md` is unchanged: the answers this leans on are its own
(`added` and `unchanged` registered from the answer on, `key_missing`
for a late add, `removed` for an entry left ahead, `EndpointUnknown`
for an id no entry holds).

### Amendment 2026-11-26 — An add whose outcome is unknown keeps its key under a pending-add record, settled by discard

Decision 7 said how a Remove and a replacement read the endpoint
methods' answers and said nothing of the add a person sends from the
form that creates an identity. The design filled the gap two ways the
contract does not carry (ux-designer's question on
`design/2026-11-04-identities.md` §6.2, from the third review of the
change that brought the design texts to the 2026-11-25 note). It read
the add's `Internal` as nothing created and deleted the key with the
form; and it deleted a key kept after a lost answer once a list that
answered lacked its id. `LOCAL-IPC.md` has an add that leaves the
overlay ahead of the runtime answered `Internal` as a failed write is,
absent from `admin.endpoints.list` and loaded at the daemon's next
start. On both paths that start listed an identity whose key the
client had deleted.

So the add's answers are read as the remove's already were. `added`
and `unchanged` are the identity. `refused` and `InvalidArgument` say
that nothing was created, but only where no earlier add of that key
ended unknown: `key_missing` is decided before the overlay is asked,
so an entry left ahead can stand behind a later refusal. Everything
else, `Internal`, `BackendUnavailable` and no answer included, is an
outcome the client does not know, and the key is kept. The method's
own `BackendUnavailable` says that nothing changed, but a call still
waiting when its connection ends is answered the same code
(`LOCAL-IPC.md` §Close), and the overlay may by then hold the entry.

A kept key is settled by discard and not, as a replaces record is, by
sending the add again. A replacement is finished toward because the
person asked for it and the old identity hangs on it; a form the person
left has nothing hanging on it, and sending its add again would need
its alias and client kinds stored beside the key. The discard is the
one the 2026-11-25 note gave a refused replacement: the key path and
the platform key first, which makes any add applied late `key_missing`;
then the administrative remove, `removed` taking an entry left ahead
out of the overlay and `EndpointUnknown` saying no entry holds the id.
A record whose key path is already empty goes on to the remove whether
its id is listed or not: the daemon may have started, with the entry
ahead, between the unlink and the remove. A form left after a known
refusal with no unknown add before it has nothing in the overlay to
take out, and deletes the key file, the platform key and then the record directly.

What the rule may touch is named by a record and never by absence. The
design recognised a kept key as a key file whose id no list holds; but
`endpoint-keys/` is not the human client's alone, a deployment minting
keys there with no daemon running (ADR-0067 decision 4), so that test
would delete another program's add in progress. The pending-add record
is content-free, names the new EndpointId, is written before the key
it names is sealed and before its platform key is made, and is deleted
when the id is an identity or the discard ends. The first text of this
note ordered the record before the add alone; a process ending between
the seal and the record would then have left a key file no record
names, which this same rule forbids deleting (the review of that text
at cadd2cf47, its finding F1). With the record first, a record whose
key was never sealed is settled as one whose key path is already
empty: nothing to unlink, a platform key made before the process ended
retired as any is, the remove answering `EndpointUnknown`. No alias and no client kinds are stored. A key that
Replace minted is named by no pending-add record, so this rule never
reaches it before or after its replaces record is written; and the
clean-up rule of 2026-11-24, whose test (the list lacks the id, the
key path is empty) a record in the middle of its discard meets, takes
no pending-add record, since the entry it waits to remove can still
come back at the daemon's next start.

Not decided here, and not verified: how the daemon orders an add
applied late against the discard's remove. If an add that had passed
its key check wrote its entry after the remove answered
`EndpointUnknown`, the entry would stand with its key gone and its
record deleted; the identity is then listed as any whose key is
missing is, and removed by the person, as the 2026-11-25 note leaves
the same case of a replacement. Neither the form nor the record is
built (`STATE.md` §Logical tables), so no build did otherwise.

Propagation: decision 7; this note; the log row; the digest entry, a
bullet of its own; `STATE.md` §Logical tables, the record. The design
texts that state the rules this note changes are their owner's and
arrive with it in one change: `design/2026-11-04-identities.md` D1
("cannot create an endpoint after the client deleted its key"), §6.2
(the kept key deleted on the list's silence, the form left after a
refusal) and `IdentityAddFailed` shown on `Internal`. How the form
holds the alias of the first add that ended unknown is the design's.
`LOCAL-IPC.md` is unchanged: every answer this leans on is its own.

### Amendment 2026-11-26 — The replaces record is written before the replacement's key is sealed, the pending-add record is bounded, and a store discard takes both (ii)

Four things, the first a gap both reviews of the pending-add text at
cadd2cf47 named as standing before it, the rest owed since that text
landed.

**The replaces record's order.** Decision 7 and `STATE.md` had the
record written before the administrative add, and the design's step
had the key sealed before the record. A Replace cut between the seal
and the record left a key file no record names: no settlement reaches
it, the pending-add rule of the same day forbids deleting a key no
record names, and a second Replace, finding no record on the old
identity, minted another. It is the gap the pending-add record had in
its first text, and it takes the same fix: the client generates the
key pair, knows the id from it, writes the record, and only then seals
the key, makes the platform key and sends the add; a record that cannot
be written seals nothing. A record whose key was never sealed needs no
rule of its own. The settlement of 2026-11-25 sends the same add, the
daemon answers `refused` with `key_missing`, and the discard finds the
key path empty, retires a platform key made before the process ended as
any is, and deletes the record on the remove's `EndpointUnknown`; the
old identity stands as before with Replace offered again. The
2026-11-24 note's "written before the add" stands as the record of its
day and is narrowed by this one.

**The pending-add record's bound.** At most 64 rows, the profile's
bound on endpoints, which is also the most identities a profile holds.
A write past it is a record that cannot be written, which the
2026-11-26 note already answers: no seal, no platform key, no add.
Nothing is evicted, because a row is the one thing that names its key
and an evicted row would leave the key file no rule may delete. The
replaces record takes no number of its own: one row at most names an
old identity, so the same 64 bound it.

**A discard of the store.** "Discard" in the two settlements is the
record's: key path, platform key, administrative remove. A discard of
the store (ADR-0060) takes both records with every other row and
deletes no key file. No settlement runs before it, and no mark of the
rows is kept outside the store: such a mark would be a second record of
what the store held, surviving the act meant to end it. What follows is
said rather than repaired. An id the endpoint list holds, then or after
the daemon's next start, is an identity whose card is recreated from
its alias, as decision 2 has it for every identity after a store
discard; a replacement in progress is forgotten, and the old identity
and the new stand as two identities, the person removing the one not
wanted; a key whose add never wrote stays in `endpoint-keys/` unused,
and the client does not scan for it, the directory being one a
deployment also mints keys in (ADR-0067 decision 4).

**"Read as that remove is".** The sentence opening the add's rule
followed the replacement's settlement, whose last-named remove is the
discard's, and a reviewer of the design change read it so. It now names
the administrative remove of an identity's Remove, whose failed or lost
answer is an outcome the client does not know, and says that the add is
settled the other way from a replacement's add.

Still not decided, and not verified, as the 2026-11-26 note left it:
how the daemon orders an add applied late against a discard's remove.
`LOCAL-IPC.md` says of the endpoint methods that none is ordered by
connection and that an add applied late after the key file is deleted
is `key_missing`; it does not say that an add past its key check cannot
write after a remove answered `EndpointUnknown`. Both settlements end
on that answer, so the residue is the same for both: an entry listed
with its key gone, removed by the person. A daemon interleaving test
would settle it; the daemon is p2p-network-dev's.

Propagation: decision 7; this note; the log row; the digest entry, its
replacement bullet and its pending-add bullet; `STATE.md` §Logical
tables, both records. Three design texts state the order or a case this
note changes and are their owner's to bring to it, unchanged here:
`design/2026-10-24-identity-key.md` §5.3 step 1 ("the key minted and
sealed at the new identity's key path, the record, the add") and
`design/2026-11-04-identities.md` §6.2 ("before or after Replace writes
its replaces record") and its run (g), a Replace stopped between the
seal and the record, which this order leaves no way to reach. Neither
record is built (`STATE.md` §Logical tables), so no build did
otherwise.
