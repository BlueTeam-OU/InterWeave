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
pair with a derived id and a transport trust set that is the union of
the identities' own. Decision 2's "configuring an endpoint" is
generating its key. Decision 7's rename changes the card and the local
alias, never the id, and a new identity on a freed alias has a new key
and id. Decision 8's automated endpoint is a key the deployment mints.
Decision 5 (what two identities share) and decision 9 (one identity at
a time) are unchanged; the Trust screen is the identity in front's
under decision 9. Propagation: the Status line; decisions 1, 2, 7, 8;
this note; the log row; the digest entry.
