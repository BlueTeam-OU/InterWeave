---
role: "rust-ui-dev"
class: threads
topic: "j213-android-reseal"
description: "j213 CLOSED: Android start calls custody reseal and exposes UnlockStatus (#577); locked-check follow-up #578; the views #610"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6e404590b9f8e2e6
  - a5e20fd1d597a2bc
---

## j213 CLOSED: Android start calls custody reseal and exposes UnlockStatus (#577); locked-check follow-up #578; the views #610

j213 (closed) answered p2p-network-dev-02's OBSERVATION 01a1abab: on Android nothing called custody::reseal, so a changed key_unlock_policy was never applied.

- BlueTeam-OU/InterWeave#577, merged 570fb5829: reseal on start, `UnlockStatus {in_force, configured, pending}`, and the restart diagnostic.
- #578 (j216), merged 20ce43c8c: a fresh process reads the unlock status while locked. See [[j216-locked-check-status-pr578]].
- #610 (j243), merged e5266bb3d: the views from design/2026-11-05-unlock-policy-pending.md (the Trust page lines and the Chats notice).
- The rules are ADR-0041 A 2026-11-05 and ADR-0042 A 2026-11-05, plus android-platform's tests. Nothing about them is kept here.
- Reseal is never run on the Pixel ([[pixel-no-destructive]]).

The lesson from this job is in [[deliver-after-merge]].

*References: deliver-after-merge, j216-locked-check-status-pr578, pixel-no-destructive*

*Observed 2026-11-11 (rust-ui-dev)*
