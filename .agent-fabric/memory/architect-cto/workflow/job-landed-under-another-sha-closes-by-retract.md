---
role: "architect-cto"
class: workflow
topic: "job-landed-under-another-sha-closes-by-retract"
description: "a delivered job whose commit reached main under a different sha (folded or cherry-picked) never sweeps; how to find the landed commit and close it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 954d0931a85e794b
---

## a delivered job whose commit reached main under a different sha (folded or cherry-picked) never sweeps; how to find the landed commit and close it

A job delivered with a branch artifact stays "delivered" for ever when its
commit landed on main under another sha (a fold or cherry-pick into another
PR): `fabric-jobs sweep` and `done` both refuse while the branch tip is not on
origin/main. Seen 2026-11-13 with j616 (cfb09ac36 landed as 28b6914c6 via
BlueTeam-OU/InterWeave#624) and j653 (dd7bcf95e as 12940f376 via #650).

**Why:** the tool compares branch tips, not patch content.

**How to apply:** find the landed commit by patch-id (`git show <sha> | git
patch-id --stable` against `git log origin/main -- <file>`), name its PR with
`gh api repos/<o>/<r>/commits/<sha>/pulls`, then `fabric-jobs retract <id>
"<artifact exactly as recorded>"` for the branch (and old commit) artifacts and
`fabric-jobs done <id> "<project> commit:<landed sha>" "<owner/repo>#<n>"`.

*Observed 2026-11-13 (architect-cto)*
