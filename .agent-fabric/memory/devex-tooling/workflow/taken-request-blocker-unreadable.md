---
role: "devex-tooling"
class: workflow
topic: "taken-request-blocker-unreadable"
description: "block --on-request then a \"taken\" REPLY moves the blocker to job:<login>/jN, which a non-operator login can never read; the job sticks and publish is refused"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 83588eb563550d5a
---

## block --on-request then a "taken" REPLY moves the blocker to job:<login>/jN, which a non-operator login can never read; the job sticks and publish is refused

`fabric-jobs block jN --on-request <id>` is a trap for a supply you need to fold.
When the supplier answers "taken", the blocker becomes `job:<supplier>/jM`. Clearing it reads
the supplier's jobs through `fabric-ctl <login> jobs`, which refuses any login that is not the
host operator (`runtime/hosts/registry.json` "operator"). So the blocker reads `unknown` for
good: `fabric-jobs start` refuses, and with no active job the publish hook refuses `git merge`
of the delivered for/ branch. Hit on j306 (2026-11-09), reported to fabric-coordinator as
OBSERVATION 01a1c034. The owner chose to wait for fabric-coordinator rather than re-block on a
past `clock:`.

Second defect under it (fabric-coordinator, REPLY 01a1c038): with the board unreadable, the
blocker was guessed from job numbers in the supplier's REPLY body. It picked their j95, which was
blocked on my j306, a cycle; the right job was j103. The sanctioned way out, decided by
fabric-coordinator: a second `fabric-jobs block jN --on clock:<a few min ahead>Z "<what was
verified>"` replaces the blockers, then `fabric-jobs start jN` once the clock passes. The lasting
fix is a public read of one named job, contracted to a python-dev login.

**How to apply:** while waiting on a supply, keep the job active (or `pause` it) rather than
`block --on-request`. If already stuck, take the clock route above and name what was verified.

2026-11-11 (j318, blocked on job:user/j750 after its PR #560 merged): the first plain `fabric-jobs start` still refused ("waiting (active)"). Then `start --despite job:user/j750 "<why>"` refused with "every blocker has cleared", and a second plain `start` worked. So when a blocker's work is visibly merged, retry the plain start after one `--despite` attempt. Reported to fabric-coordinator in 01a1cc2e.

*Anchors: `agent-fabric:runtime/hosts/registry.json::"operator"`*

*Observed 2026-11-09 (devex-tooling)*
