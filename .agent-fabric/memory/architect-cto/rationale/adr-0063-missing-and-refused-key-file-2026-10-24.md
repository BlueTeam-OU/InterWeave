---
role: "architect-cto"
class: rationale
topic: "adr-0063-missing-and-refused-key-file-2026-10-24"
description: "ADR-0063 A 2026-10-24 (on my Stage 21 docs branch): a missing identity-key file outside recovery is a loss (silent first key only for an identity that never shared its card), a refused file goes only with the confirmed new key, written…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d155b76bd849112c
---

## ADR-0063 A 2026-10-24 (on my Stage 21 docs branch): a missing identity-key file outside recovery is a loss (silent first key only for an identity that never shared its card), a refused file goes only with the confirmed new key, written first; why nothing is kept aside

ux-designer-01's QUESTION 01a16d88 (2026-10-24), for rust-ui-dev-02's
j53b (Android identity key; §13 bullets 31, 33). Rulings (DECISION
01a16d8a-1f61): refused file by IdentityKeyError class — unusable
classes (invalidated, unavailable, sealed-unreadable-here, downgrade,
damaged) removed with alias in the same step as the new key, on the
person's confirmation only; UnsupportedVersion/UnknownCustody offer
the update first, new key beside, nothing kept aside (a sealed file
beside the live one is a second secret at rest with no reader; one
file per identity at its path); Cipher offers retry first. Invariant (restated after #411's review: one Keystore alias per
identity, the new seal destroys the old wrapping key, create refuses
over an existing file): remove the old file, then create; a gap
resolves to the key-missing state, never a silent new key; never show
'made' without a key that opens. An exposed (not-private) file: refused,
new key offered, no tightening. Missing file: not a first key — silent only for an identity
that never shared its card (card_shared_at absent everywhere — the one
record this device holds; the 'no accepted-card route holds its key'
clause was wrong, those are other identities' keys, dropped on rust-ui-
dev-02's question), else key-missing state with a
confirmed new key; same for the whole directory missing outside the
recovery flow (decision 1's fresh-state rule stays for recovery, where
the warning precedes). Amendment triple + digest on branch
docs/stage-21-close (local until the ADR-065 relaunch).

*Observed 2026-10-24 (architect-cto)*
