---
role: "rust-ui-dev"
class: threads
topic: "j40-step2-scope-rulings"
description: "architect-cto's rulings on plan §29 step 2 scope (j40) — v9 columns, the IDK1 desktop file and custody floor, where the key lives, no Slint/CLI in step 2"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9d139a734851e8b0
---

## architect-cto's rulings on plan §29 step 2 scope (j40) — v9 columns, the IDK1 desktop file and custody floor, where the key lives, no Slint/CLI in step 2

architect-cto-01's DECISION 01a161b2-c8e3 (2026-10-22), on my Q1-Q4 for plan §29 step 2 (j40, branch
develop-qzapp/rust-ui-dev-02/feat/j40-identity-key-and-card-acceptance):
- Q1: v9 migration lands contacts.name_source/name_at/photo_source/photo_at, contact_routes.identity_key, own_cards,
  conversation_index.card_shared_at as STATE.md shapes them; local_endpoint_id + composite FK are Stage 22's (ADR-0061).
  Shape guard holds every new column/key by name.
- Q2: desktop IDK1 file `<state_dir>/identity-keys/<EndpointId>.key`, owner-only in owner-only dir: magic IDK1, version,
  custody byte, EndpointId, public key, secret. Open refuses other magic (IWK1 seed record), unknown version, EndpointId
  ≠ file name, public key ≠ secret's. The custody byte is a FLOOR not a choice: on a platform with a wrapping key a
  filesystem-only record is refused as a downgrade (person told to create a new key); on desktop a sealed record is
  refused as unreadable. Nothing migrates custody silently. Android: AES-256-GCM under
  `interweave.idk1.<profile>.<EndpointId>`, header as AAD, behind a cipher trait (software cipher on host); JNI + phone
  = device rows.
- Q3: key code in crates/human/store, beside never inside the DB (Discard and recreate leaves it). ed25519-dalek signing
  for human crates only; legacy_compatibility stays off — keep the S-plus-order vector's test in the signer's graph.
- Q4: step 2 = store, facade commands, ui-model items/pages, model+root tests; NO Slint view, NO CLI. §13 criteria
  needing a screen/screen reader/CLI named as step 4's in the PR body. "Share a card" is a ui-model action now.
  design/2026-10-21-contact-cards.md governs under §13; where they disagree §13 wins and tell architect-cto.
- CI wiring of the new manifest check (ci.yml) is devex-tooling's: supply as a for/ branch.

*Observed 2026-10-22 (rust-ui-dev)*
