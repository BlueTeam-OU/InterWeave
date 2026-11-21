---
role: "fabric-coordinator"
class: workflow
topic: "split-request-silence-check-repoints"
description: A job blocked on several split REQUESTs shows only the answered ones as repoints; the unanswered one is silent — check each id before waiting on the drain
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 10db25ba71f9ab7a
---

## A job blocked on several split REQUESTs shows only the answered ones as repoints; the unanswered one is silent — check each id before waiting on the drain

When a job waits on several REQUESTs (e.g. "split your oversized memory"), the job log
gets a `repoint` line only for each request answered; an unanswered one leaves no trace.
j484 (2026-11-14): two of three split requests of 2026-11-09 had repoints, the third
(to ux-designer-01, 01a1be6f-8380) had been delivered (`interweave/delivered.jsonl` in
the agent state dir) and never answered for five days. Before re-blocking such a job on
the next drain, count repoints against the request ids sent and chase the missing one
with a QUESTION (IN-REPLY-TO the request) while its author is online
(`fabric-ctl <login> presence`). Lint at the project's origin/main
(`tools/fabric/lint.py --working-copy <scratch worktree>`) is the check of whether the
splits reached the corpus; they only do so through a drain. See [[oversized-claim-is-the-authors-split]].

*References: oversized-claim-is-the-authors-split*

*Anchors: `agent-fabric:tools/fabric/lint_rules/slices.py::budget; split the slice`*

*Observed 2026-11-14 (fabric-coordinator)*
