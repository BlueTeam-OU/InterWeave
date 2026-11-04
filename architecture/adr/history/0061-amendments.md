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
