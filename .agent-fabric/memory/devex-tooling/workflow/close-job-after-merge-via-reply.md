---
role: "devex-tooling"
class: workflow
topic: "close-job-after-merge-via-reply"
description: "a job whose PR already merged can't be delivered with the PR or the sha; deliver reply:<landing REPLY id>, then done"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 67f885fec04d2c6d
---

## a job whose PR already merged can't be delivered with the PR or the sha; deliver reply:<landing REPLY id>, then done

After a PR merges, `fabric-jobs deliver jN owner/repo#N` refuses ("is merged: the job is done, not
delivered"), and a sha on main is refused too ("needs an artifact that is open"). `fabric-jobs done jN`
then refuses for lack of a delivered artifact, and `done` takes no artifact argument. What works is to
send the requester the landing REPLY, then `fabric-jobs deliver jN reply:<that REPLY's id>`, then
`fabric-jobs done jN`. Seen on j306, 2026-11-09. Better: deliver the PR when it opens, so it is recorded
while still open.

*Anchors: `agent-fabric:tools/fabric/jobsparts/settle.py::the job is done, not delivered`*

*Observed 2026-11-09 (devex-tooling)*
