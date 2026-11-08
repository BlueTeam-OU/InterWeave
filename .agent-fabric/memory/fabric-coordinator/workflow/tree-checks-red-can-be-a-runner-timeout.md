---
role: "fabric-coordinator"
class: workflow
topic: "tree-checks-red-can-be-a-runner-timeout"
description: "an InterWeave required check reported \"fail\" by wait-merged or gh pr checks may be a 15-minute job timeout (conclusion cancelled), not a finding; re-run it, the arming survives"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 343f9ff589eaff59
---

## an InterWeave required check reported "fail" by wait-merged or gh pr checks may be a 15-minute job timeout (conclusion cancelled), not a finding; re-run it, the arming survives

On 2026-10-30 `fabric-pr wait-merged 516` exited 5 ("tree checks failed") and
`gh pr checks` showed `tree checks fail 20m0s` on BlueTeam-OU/InterWeave#516, a
memory-only drain PR. The job's real conclusion was `cancelled`: "The job has
exceeded the maximum execution time of 15m0s" (host under load), not a check
finding. `gh run view --log` returned "log not found" for it; the cause was
read with `gh api repos/<repo>/actions/jobs/<job-id>` (`.conclusion`, steps)
and the check-run annotations.

**Why:** both tools print a cancelled required job as a plain failure, so it
reads like a defect in the head and invites a fix commit — and any push
disarms the PR and voids an AUTHORIZES given for that head.

**How to apply:** before touching the branch for a red required check, read
the job's conclusion and annotations; if it is a timeout, `gh run rerun <run>
--failed`. A re-run is not a push: auto-merge stayed armed and #516 merged on
the re-run (f8717fb3). Related: [[drain-run-the-project-tree-checks-first]].

*References: drain-run-the-project-tree-checks-first*

*Anchors: `.github/workflows/ci.yml::tree checks`*

*Observed 2026-10-30 (fabric-coordinator)*
