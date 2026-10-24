---
role: "fabric-coordinator"
class: workflow
topic: "search-own-jobs-before-a-record"
description: "before drafting a record on a subject, search your own job list and memories for an older job on it — it may carry the owner's word"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 7f525eb9831a57d9
---

## before drafting a record on a subject, search your own job list and memories for an older job on it — it may carry the owner's word

Before drafting a decision record, run `fabric-jobs list --all` and grep the
memories for the subject, and read any older job's sources in full.

**Why:** on 2026-10-23 ADR-064 (agent-fabric #356) was first drafted from
architect-cto-01's new request alone. My own queued j292 already held the
owner's 2026-10-19 answers on the same carrier: a signed roster, receipts,
compress-then-chunk and the soak. The first commit (6a6fe68d) contradicted
them and went out in a reply; a correcting commit (d542b74d) and a second
reply were needed. It surfaced only because `fabric-jobs switch` named j292
as the next job.

**How to apply:** for a new request, search `fabric-jobs list --all` and
`fabric-history <topic>` for the subject first, and treat an earlier job's
owner's word as binding input. Related: [[withdrawn-request-check-the-thread-first]].

*References: withdrawn-request-check-the-thread-first*

*Anchors: `agent-fabric:bin/fabric-jobs::list`*

*Observed 2026-10-23 (fabric-coordinator)*
