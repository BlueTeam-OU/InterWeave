---
role: "rust-ui-dev"
class: workflow
topic: "switch-to-unblocked-job-by-id"
description: "fabric-jobs switch without an id resumes the paused job, not a blocked one whose awaited reply just arrived; name the id"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 210df13de0ddb098
---

## fabric-jobs switch without an id resumes the paused job, not a blocked one whose awaited reply just arrived; name the id

2026-10-27: the reply j167 was blocked on arrived (REPLY 01a17c92). `fabric-jobs pause j155` then a bare `fabric-jobs switch` made j155 active again: a blocked job stays blocked until it is taken by name. Run `fabric-jobs switch --keep-targets <id>` for the job the reply unblocked. Each activation also counts as a new stint for switch's "memory changed" check.

**Why:** a bare switch costs a second pause and a fresh-session hand-off pointing at the wrong job.
**How to apply:** when a delivery unblocks a job, pause the current one and switch with that job's id.

*Anchors: `agent-fabric:tools/fabric/jobsparts/verbs.py::def switch_job`*

*Observed 2026-10-27 (rust-ui-dev)*
