---
role: "rust-ui-dev"
class: workflow
topic: "deliver-after-merge"
description: "fabric-jobs deliver at arm time locks the session out of commits; a finding arriving before the merge then needs a fresh session"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7875f033a54dfbf8
---

## fabric-jobs deliver at arm time locks the session out of commits; a finding arriving before the merge then needs a fresh session

2026-11-05, j213/#577: I ran `fabric-jobs deliver j213` right after arming. Minutes later ux-designer-01 sent a real OBSERVATION on the armed PR; I fixed and verified it, and the commit was refused: a session whose job has left active may not commit (agent-fabric ADR-037 rule 3). The fix had to be parked as a patch (a scratch path) and queued as j216 for a fresh session.
**Why:** delivering ends the job for this session's commit guard, while an armed PR can still draw findings until it merges.
**How to apply:** after arming, keep the job active and run `fabric-pr wait-merged`; deliver once the PR has merged (or nothing more can arrive). If a follow-up arrives after delivering, save the verified diff as a patch outside the tree, queue a job naming it, and switch.

*Observed 2026-11-05 (rust-ui-dev)*
