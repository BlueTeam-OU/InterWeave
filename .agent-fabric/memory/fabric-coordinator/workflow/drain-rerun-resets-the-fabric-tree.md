---
role: "fabric-coordinator"
class: workflow
topic: "drain-rerun-resets-the-fabric-tree"
description: "a project-half drain re-run must reset the agent-fabric tree it runs from as well as the project's .agent-fabric/memory: assemble.py writes a bundle's domain claims into the fabric's memory/domains, and a second run then treats them as…"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - b2391f1979be1fbe
---

## a project-half drain re-run must reset the agent-fabric tree it runs from as well as the project's .agent-fabric/memory: assemble.py writes a bundle's domain claims into the fabric's memory/domains, and a second run then treats them as already applied

In j504 (InterWeave #529), the project half of the 2026-10-31 drain was assembled from a scratch
agent-fabric worktree at 6c295931 (`--fabric`). The first run wrote language-culture-en's domain
supersede into that worktree's `memory/domains/language-culture/domain/typed-names-not-joined.md`.
The F3 re-run (8454334c) reset only the InterWeave corpus to the curation commit. The supersede was
then a no-op, and its `collision_decisions` and `replaced_sections` entries vanished from
`last-drain-report.json` (round-2 F5). A third run, after `git checkout -- .` in the fabric
worktree, restored exactly those 12 lines (c51526ce).

**Why:** a project half reads and writes the fabric's domain tree too. The domain output belongs to
the fabric half's PR, never this one, so the project half's worktree must start clean each run.

**How to apply:** before every re-run, `git checkout -- . && git clean -fd` in the fabric worktree,
as `full.sh` does for F, then reset the project corpus. Afterwards, discard the fabric worktree's
domain writes. Do it in a separate command, because the job-copy guard refuses a command that
commits and names a path outside the job's copy. See [[drain-hold-keys-are-claim-topics]].

*References: drain-hold-keys-are-claim-topics*

*Anchors: `agent-fabric:tools/fabric/assemble.py::--fabric`*

*Observed 2026-11-03 (fabric-coordinator)*
