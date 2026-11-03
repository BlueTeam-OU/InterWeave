---
role: "rust-ui-dev"
class: solution
topic: "android-no-identity-key"
description: "Android's identity key -- opened or made under the Keystore since #414 (2026-10-24); what is still missing there"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8c6522c6634c1331
  - ca1d3da5c324b546
---

## Android's facade has no identity key (2026-10-24) -- Your card empty, Share refused there; j53b's job

As of main a3583dd5 (2026-10-24), `crates/human/android-platform/src/service.rs` builds
`FacadeSide::new(client, wall_ms)` and never calls `set_identity_key`; only the desktop
(`apps/human-desktop/src/ipc.rs`, `app_core::identity_key`, FilesystemCustody) and the CLI open one.
So on Android: proposals, Accept and contact pages work; `UiModel::identity()` is None, Your card is
empty, Share is refused (`Failure::NoIdentityKey`). Found building j53's screens (PR from branch
develop-qzapp/rust-ui-dev-02/feat/j53-stage-21-close-step-4-s).

**Why:** step 2's Android half (ADR-0063 decision 2: Keystore alias `interweave.idk1.<profile>.<endpoint>`,
`identity_key::alias`) was never wired; human-client-ui.md section 13 bullets 31 and 33 depend on it.
**How to apply:** named as the next PR `develop-qzapp/rust-ui-dev-02/feat/j53b-android-identity-key`;
check service.rs on origin/main before assuming it is still missing. Related: [[j53-gate-map]].

*References: j53-gate-map*

*Observed 2026-10-24 (rust-ui-dev)*

## Android's identity key -- opened or made under the Keystore since #414 (2026-10-24); what is still missing there

Since BlueTeam-OU/InterWeave#414 (merge 9d1828f8, 2026-10-24), Android's service opens or makes
the identity's key under the Keystore at its own alias in both postures
(`crates/human/android-platform/src/service.rs` `own_key`, app-core `identity_key_under`), so Your
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
