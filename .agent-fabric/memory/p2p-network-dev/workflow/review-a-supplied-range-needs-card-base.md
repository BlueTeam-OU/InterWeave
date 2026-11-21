---
role: "p2p-network-dev"
class: workflow
topic: "review-a-supplied-range-needs-card-base"
description: "reviewing a caller-folded supply — card_base for a base off main, save before next, word-diff for a one-line paragraph, AUTHORIZES only in the sender's header"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 92df216af739dd6a
---

## reviewing a caller-folded supply — card_base for a base off main, save before next, word-diff for a one-line paragraph, AUTHORIZES only in the sender's header

#640 merged 56d3709a0 on 2026-11-12: the device record
`packaging/android/evidence/2026-11-12-v0-3-0-update-in-place-from-v0-2-0.md`,
the README Validate-row sentence, and architect-cto's plan s22 supply (folded
as fast-forwards from its `for/` branches). Armed at 3 work on the owner's
"arm 640" plus architect-cto's AUTHORIZES (`fabric-pr arm --authorization <msg-id>`).

Still open by name: the same PeerId from the same store across an update runs
from v0.3.0 to its successor (s22 baseline rule); packaging/android/README.md's
"The release run (ADR-0057)" row lists only the v0.1.0/v0.2.0 runs (review risk,
not fixed).

**Why:** mechanics that cost a step this time.
**How to apply:**
- An AUTHORIZES line counts only in the SENDER's header (gzcoord-compose
  --authorizes); written in a body it grants nothing and gzcoord-send warns.
- A review of a supplied range whose base is not on main needs `card_base:`
  (the first round's base) in the request.
- `fabric-review next` embeds the full diff; a one-line plan paragraph made a
  45 KB brief — point the re-reviewer at `git diff --word-diff` instead.
- Run `fabric-review save <pr> <round>` before `next`, or it refuses.

*Anchors: `architecture/roadmap/BOTTOM-UP-IMPLEMENTATION-PLAN.md::The baseline rule for that half`*

*Observed 2026-11-12 (p2p-network-dev)*
