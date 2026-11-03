---
role: "fabric-coordinator"
class: workflow
topic: "broadcast-quotes-the-tool-not-memory"
description: "before a broadcast tells every agent what a tool keeps or deletes, read the tool's --help and quote it; a paraphrase from memory went to 30 sessions wrong"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 07eca37691533eeb
---

## before a broadcast tells every agent what a tool keeps or deletes, read the tool's --help and quote it; a paraphrase from memory went to 30 sessions wrong

The 2026-10-24 disk broadcast (01a16e41) said `fabric-branches --sweep --all`
"keeps anything with commits off main". That is true of branches only: a clean
worktree whose commit is on origin is removed, a live job's worktree included.
python-dev-04's j64 worktree went (OBSERVATION 2026-10-24 15:13Z), and a
correction broadcast (01a16e7d-d5ec) had to follow, plus a fix request for the
rule (the sweep should keep a worktree a live job names; REQUEST 01a16e7d-0f3d).

**Why:** a broadcast is read and acted on by every session at once, so a wrong
sentence costs the fleet one action each, then a correction each.

**How to apply:** before a broadcast states what a command keeps or deletes,
run its `--help` and copy the rule's own words; the urgency of the moment (the
disk was at 100%) is when a paraphrase slips in. See [[drain-run-the-project-tree-checks-first]].

*References: drain-run-the-project-tree-checks-first*

*Anchors: `agent-fabric:bin/fabric-branches::--sweep`, `agent-fabric:tools/fabric/branches.py::is_bytecode`*

*Observed 2026-10-24 (fabric-coordinator)*
