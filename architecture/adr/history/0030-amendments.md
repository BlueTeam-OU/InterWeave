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

### Amendment 2026-10-28 — An EndpointId is derived from the endpoint's key, and endpoint policy is the endpoint's own trust set

ADR-0067 (the owner's direction of 2026-10-28) makes every endpoint a
key pair whose EndpointId is derived from the public key exactly as the
PeerId is from the node's. The decision bullet that read "endpoint
identifiers are routing labels, not human/application identity proof"
now says the identifier names the endpoint's key, still peer-asserted
on the wire and still no authorization principal; the operational
sentence about "stable route names such as `human`" now describes local
aliases. Endpoint policy, which this record let narrow profile trust,
is the endpoint's own trusted set under ADR-0067 decision 6, and
"profile PeerTrustPolicy" in the narrowing rule is the admission set
the transport derives from the operator's list and those sets, the
rule bullet saying so. Propagation: the Status line; the decision
bullet; the narrowing rule bullet; Operational implications; this note; the log row; the
digest entry.

### Amendment 2026-11-19 — The wire majors are direct `3.0.0` and IPC `3` from the Stage 22 release

The Decision's last bullet read "implementation target becomes `/interweave/direct/2.0.0` and IPC major version 2", which was the wire from Stage 6 and Stage 13 until the Stage 22 release. ADR-0067 decision 7 moves both majors in that release, because every frame and every IPC document that carries an EndpointId carries a derived one from it and no older reader tolerates the value. The bullet keeps what this record set and names what it is now. Nothing else here changes: one exclusive lease per session, one resolved destination, the default on omission, policy that narrows and never widens. Propagation: the bullet; this note; the log row; the digest entry.
