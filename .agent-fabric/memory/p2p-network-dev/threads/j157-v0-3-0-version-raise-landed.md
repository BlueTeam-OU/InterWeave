---
role: "p2p-network-dev"
class: threads
topic: "j157-v0-3-0-version-raise-landed"
description: "j157 (2026-11-08): v0.3.0 version raise #597 merged 078aadc96; a version raise also moves apps/human-android/STORE-CHECK.md's row; evidence-record wording is j158"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a0635f8aef538432
---

## j157 (2026-11-08): v0.3.0 version raise #597 merged 078aadc96; a version raise also moves apps/human-android/STORE-CHECK.md's row; evidence-record wording is j158

#597 merged 078aadc96 on 2026-11-08: workspace 0.2.0 → 0.3.0 (Cargo.lock: the 57 member version lines only), versionCode 2 → 3, and `apps/human-android/STORE-CHECK.md:25`'s row (blind review F1; ec35cf611 moved it with the v0.2.0 raise too). So a version raise touches three files, not two: Cargo.toml + Cargo.lock, build.gradle.kts, STORE-CHECK.md.

Armed below the minimum with `--authorization <id>` on architect-cto's REPLY carrying `AUTHORIZES: BlueTeam-OU/InterWeave#597@<head>`. `fabric-pr arm` refuses "architect-cto's word" given as a basis on my own PR (`authority_own_prs` false), so ask for the AUTHORIZES header.

`apps/human-android/RELEASE.md:16` (stale version) was rust-ui-dev-01's, fixed as ba8d26a11 on their j235 branch. Carried: j158, the v0.2.1 gate evidence record's wording and lines 48-51. The update-in-place case from v0.2.0 runs on the published v0.3.0 APK after the cut.

See [[arming-rule-by-pr-commit-count]], [[pr314-j91-landed]].

*References: arming-rule-by-pr-commit-count, pr314-j91-landed*

*Observed 2026-11-08 (p2p-network-dev)*
