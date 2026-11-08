---
role: "rust-ui-dev"
class: workflow
topic: "for-supply-close-as-delivered"
description: "close a for/ supply job with deliver, never done; a done job naming the for/ branch blocks fabric-jobs switch"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9e420ff32af298a3
---

## close a for/ supply job with deliver, never done; a done job naming the for/ branch blocks fabric-jobs switch

A job whose work is a for/ branch folded onto another agent's branch is `fabric-jobs deliver`, not `done`: the branch reaches main only through their PR. Closed `done` (j198, 2026-11-03), `switch` refused "branch … is not on origin/main: it is delivered, not done" — settle.py `_landed` counts replies only AFTER that check, so a reply beside the branch does not help, and any artifact text containing the branch name counts as the branch.

**Why:** `start` won't reopen a closed job; retract was the only way out.
**How to apply:** if it happens, `fabric-jobs retract <id> "<exact artifact text>"` for each artifact naming the branch (keep the reply; the range stays in the `no artifact` note), then switch.

*Observed 2026-11-03 (rust-ui-dev)*
