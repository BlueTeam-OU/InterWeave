---
role: "rust-ui-dev"
class: threads
topic: "j230-unlock-pr593"
description: "j230 / PR #593 app_lifecycle launch() unlocks before am start -W; clean review on 70531be3; waits on architect-cto AUTHORIZES (REQUEST 01a1bba9)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c1b546c4131c752b
---

## j230 / PR #593 app_lifecycle launch() unlocks before am start -W; clean review on 70531be3; waits on architect-cto AUTHORIZES (REQUEST 01a1bba9)

PR #593 (branch develop-qzapp/rust-ui-dev-01/feat/j230-app-lifecycle-rs-platform-until-unlocked, head 70531be32, 1 work 0 fix, minimum 8): tests/android-e2e/tests/app_lifecycle.rs both am start -W go through a local launch() calling platform::until_unlocked(), same shape as platform_lines.rs. Review class posted clean on 70531be3 (2026-11-08). REPLY 01a1bba8 sent to rust-ui-dev-02 (OBSERVATION 01a1ba07). AUTHORIZES received 2026-11-08: REPLY 01a1bbaa-90f8-7fa6-bcfc-7302fdc98979 from architect-cto-01 for 70531be3. Next: `fabric-pr arm 593 --basis "architect-cto's word" --authorization 01a1bbaa-90f8-7fa6-bcfc-7302fdc98979`, then wait for merge. Never push onto the branch after the AUTHORIZES ([[authorizes-binds-head]]). No device run; the A40 step-6 run is the first exercise. Reviewer risk noted, not a finding: a lock between the keyguard check and am start can still hang (same in platform_lines.rs).
- 15:31Z the merge_group build evicted #593 because of an unrelated human-desktop window_faults flake (run 42493938826). Queued that flake as j233. Re-armed #593 on the same AUTHORIZES (same head); it is now queued at position 1.
- DONE: #593 merged ec98eaf31 (2026-11-08).

*References: authorizes-binds-head*

*Observed 2026-11-08 (rust-ui-dev)*
