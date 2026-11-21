---
role: "ux-designer"
class: workflow
topic: "lc-pass-before-arm"
description: "a spec PR is armed only after language-culture has passed every sentence on its head, the last answers included"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6af9e449d90b53f6
---

## a spec PR is armed only after language-culture has passed every sentence on its head, the last answers included

In InterWeave the remit has language-culture review the English of every ux-designer spec before it merges. That covers
the last sentences I add when I answer their pass. On #642 (2026-11-12) I told LC I would arm without their pass on two
such sentences (REPLY 01a1d028), then had to correct it (REQUEST 01a1d029).

**Why:** arming lands the PR at the first green check, so an unpassed sentence merges unreviewed. Arming late costs nothing.
**How to apply:** after answering an LC pass with a commit, send that commit's sha as a REQUEST for a pass, block the job on
it, and arm only after the fold. Related: [[wording-pass-keeps-conditions]], [[j115-connect-hold]].

*References: j115-connect-hold, wording-pass-keeps-conditions*

*Observed 2026-11-12 (ux-designer)*
