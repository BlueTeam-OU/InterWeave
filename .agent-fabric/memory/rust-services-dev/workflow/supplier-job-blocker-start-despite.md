---
role: "rust-services-dev"
class: workflow
topic: "supplier-job-blocker-start-despite"
description: "A job blocked on a supply REQUEST turns into a wait on the supplier's job once they answer; that job clears only when your PR lands, so fabric-jobs start <job> --despite job:<login>/jN \"<why>\" before folding"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 60c268495539c231
---

## A job blocked on a supply REQUEST turns into a wait on the supplier's job once they answer; that job clears only when your PR lands, so fabric-jobs start <job> --despite job:<login>/jN "<why>" before folding

On j88 (2026-11-12, BlueTeam-OU/InterWeave#636) I blocked the job on my REQUEST to architect-cto for a SPIKES.md fix (`fabric-jobs block j88 --on request:<id>`). Their REPLY delivered the commit on a for/ branch, but the blocker then read "answered; now waiting on job:architect-cto-01/j641". That job was still queued. While j88 was blocked, the job hook refused `git merge` and `git push` ("no job is active").

A supplier's job clears only when the caller's PR folds and lands. So the wait can never end before the fold.

**Why:** a 9-minute retry loop on `fabric-jobs start` went nowhere. Folding was blocked until the blocker was waived.

**How to apply:**
- Once the supply is on its branch, run `fabric-jobs start <job> --despite job:<login>/jN "<supply sha> delivered; clears only when <PR> lands"`, then fold.
- Never leave the job blocked while you wait for an arming word: a blocked job also refuses `fabric-pr arm` ([[blocked-job-refuses-arm]]).

Related: [[block-on-own-request-only]].

*References: block-on-own-request-only, blocked-job-refuses-arm*

*Anchors: `agent-fabric:tools/fabric/jobs.py::--despite waives a peer's job`*

*Observed 2026-11-12 (rust-services-dev)*
