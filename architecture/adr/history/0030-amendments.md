# ADR-0030 — amendment notes

### Amendment 2026-10-21 — An EndpointId is the human client's user identity above transport

ADR-0061 (the owner's direction of 2026-10-21: "An EndpointId should
match a user identity. One user of the app can have many user
identities on the same phone. So for any EndpointId he can add a
display_name and a photo") has the human client present one user
identity per EndpointId it holds — a display name and a photo, the card
of ADR-0059 — several on one node under one PeerId. This record's rule
that an endpoint identifier is "not human/application identity proof
or an authorization principal" is unchanged in what it says about the
transport: the PeerId alone is authenticated, a remote `source_endpoint`
stays peer-asserted metadata, endpoint policy narrows and never widens
profile trust, and no wire field changes. The bullet gains the sentence
that names the use above transport, so that a reader does not take the
rule as forbidding it.
