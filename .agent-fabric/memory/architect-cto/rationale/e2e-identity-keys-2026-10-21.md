---
role: "architect-cto"
class: rationale
topic: "e2e-identity-keys-2026-10-21"
description: "Owner's direction 2026-10-21: peer-level (Noise) and end-to-end encryption are two matters; ADR-0063 (accepted) gives each user identity an Ed25519 key carried and proved by its card now; ADR-0064 (proposed) is the full E2E layer (sealed…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f8819dd69d7eeeb0
---

## Owner's direction 2026-10-21: peer-level (Noise) and end-to-end encryption are two matters; ADR-0063 (accepted) gives each user identity an Ed25519 key carried and proved by its card now; ADR-0064 (proposed) is the full E2E layer (sealed kind, ratchet, sender keys, multi-device by signed device list, opaque holder); both on IW #340

The owner, 2026-10-21 evening: "A matter is peer level encryption and
another should be the end2end encryption"; agreed with my reading
that E2E is a layer above the unchanged transport and asked for the
card field now plus a proposed ADR for the full layer. ADR-0063
(accepted): an Ed25519 identity key per user identity (ADR-0061's
EndpointId), generated with it, never derived from the node seed (so
an identity can live on a second node later), custody as the seed
(IWK1 under Keystore per identity on Android, filesystem floor on the
desktop, `<state_dir>/identity-keys/<EndpointId>.key`); the card gains
`identity_key` + `signature` over `interweave-card-v1 | name | surname
| sha256(photo) | PeerId | EndpointId`; a key change on a known route
is a proposal the person decides; the key encrypts nothing yet.
ADR-0064 (proposed): `sealed` kind, X3DH-like agreement + double
ratchet (SPIKE-013 picks the library), sender keys for channels,
device list signed by the identity key (ADR-0043 stands: no shared
PeerId), bounded opaque holder = the one offline store ADR-0020 can
admit, verification by full key, recovery only from the person's own
backup. Why the key now: every card exchanged without it would need
re-exchange when the layer lands. Open for the owner: accepting
ADR-0064; whether an identity key gets a phrase of its own.

**Landed:** ADR-0062/0063/0064 merged on IW #340 (e3a05d18, nine
rounds, 31 findings). Lessons from the rounds: (1) a key's custody
must name its own envelope and alias namespace — the seed's seam
retires other aliases and IWK1 re-derives the PeerId, so "as the seed"
is wrong; "as the store key" (ADR-0060 d3) is the pattern; (2) a
"canonical bytes" definition lives in the client contract alone, with
field count, u16be lengths, NFC and base64 form spelled out; (3) an
active schema with an unimplemented kind is tightened as a minor with
the reason in its description. rust-ui-dev-02 landed the card vectors
on #343 ahead of the parser; the 17 criteria + two rulings (Remove
deletes the key; recovery flow says key change) are on my #344.

**2026-10-22 ~03:45Z (step 2 rulings, rust-ui-dev-02 j40):** v9 lands
contacts' provenance columns, contact_routes.identity_key, own_cards,
conversation_index.card_shared_at (Stage 22 keeps local_endpoint_id);
the IDK1 file lives with crates/human/store beside the database (the
layering check keeps human crates off crates/transport); step 2 ships
store, facade, ui-model and tests, no Slint view or CLI (step 4's). New
rule promoted the same hour from the DECISION into ADR-0063 A
2026-10-22 (IW #354): the record's custody field is a floor, a
filesystem-only file refused at open where a wrapping key exists, no
silent re-sealing. Lesson: a condition I invent in a DECISION is
rationale; it goes into the record before the code conforms to it.

*Observed 2026-10-22 (architect-cto)*
