---
role: "rust-ui-dev"
class: solution
topic: "android-no-identity-key"
description: "Android's identity key -- opened or made under the Keystore since #414 (2026-10-24); what is still missing there"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 16dc3d420f8c38a0
  - ca1d3da5c324b546
---

## Android's identity key -- opened or made under the Keystore since #414 (2026-10-24); what is still missing there

Since BlueTeam-OU/InterWeave#414 (merge 9d1828f8, 2026-10-24), Android's service opens or makes
the identity's key under the Keystore at its own alias in both postures
(`crates/human/android-platform/src/service.rs` `with_key_source`, which hands the facade an app-core
`KeySource` built on `identity_key_under`; `own_key` went in 1c26ab6c, #423), so Your
card shows a fingerprint and Share signs there. Before it, Android had no key at all (Share refused,
`Failure::NoIdentityKey`).

Since #423 (2026-10-24) a refused or missing key is shown on Your card and in the share sheet
by its kind, with Try again or Make a new key (design/2026-10-24-identity-key.md); the service
hands its facade a KeySource and keeps the kind in identity_key_refused for the log.

**Why:** ADR-0063 decision 2; human-client-ui.md section 13 bullets 31 and 33.
**How to apply:** none of this has run on the Pixel yet (a device row); check service.rs on
origin/main before relying on the details. Related: [[j54-progress]].

*References: j54-progress*

*Anchors: `crates/human/android-platform/src/service.rs::with_key_source`, `crates/human/android-platform/src/service.rs::identity_key_refused`*

*Observed 2026-10-24 (rust-ui-dev)*
