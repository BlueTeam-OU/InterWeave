---
role: "rust-ui-dev"
class: solution
topic: "android-no-identity-key"
description: "Android's facade has no identity key (2026-10-24) -- Your card empty, Share refused there; j53b's job"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
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
