# ADR-0059 — amendment notes

### Amendment 2026-10-21 — The photo carries pixels and nothing else, a proposal has no Keep, the store records where a contact's name and photo came from, and a PeerId never ends under two contacts

ux-designer-01's specification of the card flows (#334,
`design/2026-10-21-contact-cards.md`, questions A1–A3 of its §12, the
same day the record was accepted) asked three things the record implied
but did not say. Ruled: (A3) a photo that leaves the device carries
pixels only — the sending client re-encodes it and strips every
metadata block (EXIF, XMP, GPS, thumbnails, the file name) before it
enters the card, and a receiver treats any metadata that arrives as
absent; decision 5 says so, HUMAN-CHAT.md §Contact card holds the rule
and gains the fixture case of a photo with a GPS block that the sender
strips. (A2) A proposal has no Keep: Accept is the only way a card
outlives the session, and an unaccepted proposal cannot be kept as
content; decision 3's "like any item the person did not keep" meant
that, and `human-client-ui.md` §3 now says it in those words. (A1)
Decision 3's "a local edit is never overwritten by a card" needs the
store to know where a value came from: STATE.md's `contacts` gains, for
the name and for the photo, the source (card or person) and when,
content-free columns in the §Logical tables review, added by the
migration plan §29 step 2 lands and listed in its exit gate's (d). A fourth, from the same specification's later question (its §6.3,
Move): decision 2's "one contact with two names, never two peers"
constrains both Move and Accept — moving a route moves every route of
the PeerId together, and accepting a card on a new route of a PeerId
that already has a route under a contact joins that contact — so one
peer is never shown under two names. None of the four changes what a
card is, who may send one or what a receiver does with it before
acceptance.

### Amendment 2026-10-21 — A card is one user identity's: one own card per EndpointId, a received card names the identity of its source endpoint, a contact is one remote identity, and the one-contact-per-PeerId rule is withdrawn (ii)

ADR-0061, the same day, made a user identity an EndpointId of the node
with its own card, several under one PeerId (the owner: "An EndpointId
should match a user identity … for any EndpointId he can add a
display_name and a photo"). Three sentences of this record read against
it and were changed in place. Decision 2's "as many cards as they like,
chosen per conversation": the cards were personas under one endpoint,
which a counterparty could not address; now one own card per identity,
keyed by EndpointId, and the conversation's identity decides which is
shared. Decision 1 and decision 3: a received card is still bound to the
authenticated `source_peer`, and the `source_endpoint` beside it now
names the identity the card presents, so a route of a PeerId that
already has another route under a contact is another identity and makes
a contact of its own, shown as sharing that node. Decision 4 and the
morning's amendment: "a PeerId never ends under two contacts" and "Move
moves every route of the PeerId together" are withdrawn — a contact
holds at most one route per PeerId, two routes of one PeerId are two
contacts, Move moves one route; grouping routes of one person's several
devices under one contact stays the person's act (ADR-0043). The
morning's other three rulings (pixels only, no Keep, provenance
columns) stand. The withdrawn clause lived on `main` for no release.
The same hour the owner added that two EndpointIds are two identities
even when one person holds them, and that no UX may let the person act
as one while believing they act as the other: a contact is therefore
held by the local identity that accepted it (decision 3), and the same
counterparty accepted by two identities is two contacts.

### Amendment 2026-10-21 — The card carries the identity's key and is signed by it (iii)

The owner, the same evening: peer-level encryption and end-to-end
encryption are two matters. ADR-0064 proposes the layer; ADR-0063
takes the one field that cannot wait, so that no card exchanged before
the layer has to be exchanged again: every user identity has an
Ed25519 key of its own, and the card carries its public key and a
signature over the card's canonical bytes bound to the PeerId and the
EndpointId it is sent from. Decision 1's "no identifier field" becomes
"no identifier a person chose": the key is the identity's cryptographic
name, the connection's `source_peer` stays what the card is bound to,
and a key that changes on a known route is a proposal the person
decides. The envelope schema is `active`, but no producer of a
`contact_card` exists on `main` (plan §29 step 1 is unbuilt), so the
card branch is tightened in place as a minor, 2.2.0 (ADR-0049).

### Amendment 2026-10-22 — The accepted photo is bytes in `contacts.photo`; `avatar_ref` is retired (iv)

Plan §29 step 2 found the contacts row with an `avatar_ref` TEXT column
and nothing to hold the accepted photo, which the contact page and the
conversation title show after a restart. Decision 5 already says a
photo is bytes in the envelope, never a reference, so the store keeps
the card's encoded bytes in `contacts.photo` (a BLOB within
HUMAN-CHAT.md's bound, decoded at display with the memory-safe decoder
at the fixed size, inside the store and so inside step 3's cipher and
the deletion property) and v9 drops `avatar_ref`. Rejected: a file under
`<state_dir>/human` keyed by hash (content outside the store's cipher
and deletion property); keeping no photo (defers the provenance
criterion). Propagation: the Consequences sentence; this note; the log
row; the digest entry; STATE.md's row.

### Amendment 2026-10-25 — The command-line client's record is its own README, and a route's deletion waits on Move (v)

Decision 9 said the client's name, argument shape and output format
would be "recorded in `architecture/clients/human/` when it lands".
The client landed as `interweave-human` (#368, #369, #404, #429, by
2026-10-24) with its record in `apps/human-cli/README.md` — every verb,
flag, exit code and output token a script may rely on — and nothing
under `architecture/clients/human/` names it; a reader following the
old sentence looked in an empty place. The decision now names the
README, and `human-client-ui.md` §13 names the verbs its criteria bind.
Batch 4's verification (architect-cto, 2026-10-25) also found that
`contacts route delete` refuses on every contact a person can have today:
the store's one writer of `contact_routes` makes a contact with one route
or updates a held one, and moving a route between contacts is plan §30
step 2's, so the verb's success path is reachable once Move lands; the
rule is recorded here so the limit is read as the stage's, not the
verb's. Propagation: this note; the log row; the digest entry; plan
§30 step 2 names Move and the route-delete control beside it.
