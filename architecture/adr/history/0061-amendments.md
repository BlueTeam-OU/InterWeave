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

With ADR-0063 A 2026-10-21 (ii): Remove of an added identity deletes
its key file and its Keystore alias beside the endpoint, the
conversations, the contacts and the card, since the key is the
identity's; a counterparty keeps the contact it accepted.
