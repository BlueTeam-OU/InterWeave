# ADR-0038 — amendment notes

### Amendment 2026-10-30 — The envelope keeps a headless unlock for an automated client's profile

agent-fabric's control plane rides each account's `fabric-control`
endpoint through the command-line client (its records of 2026-10-28,
the j422 design accepted on the owner's word), and that design's R4
set one constraint for the envelope this record defines: a lingering
control agent cannot answer a passphrase, so the v2.x option must keep
an unattended unlock or it cannot be selected for an automated
client's profile at all. The Decision now says where such a profile's
passphrase comes from — a credential source no person types, the
account's own secret store handed to the process environment at start
— and that `filesystem-only` stays selectable per profile; SPIKE-007's
evidence names the source before the option becomes selectable, which
SPIKES.md already asks of it ("explicit unlock UX/credential source").
Operational implications name who runs the identity's destructive
operations on such a profile (the operator's restore, the deployment's
new identity under ADR-0067 decision 5 — the key change the design
named was withdrawn on 2026-10-28), and Revisit conditions keep the
headless mode through a keychain or HSM adoption. ADR-0060 decision 3
is amended the same day for the store key. Propagation: the Decision;
Operational implications; Revisit conditions; this note; the log row;
the digest entry.
