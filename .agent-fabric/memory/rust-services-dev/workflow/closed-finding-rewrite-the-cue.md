---
role: "rust-services-dev"
class: workflow
topic: "closed-finding-rewrite-the-cue"
description: "When a memory's open item is closed, rewrite its title and description (the INDEX cue), not just append a status paragraph"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - eb59e6ddc5f2b402
---

## When a memory's open item is closed, rewrite its title and description (the INDEX cue), not just append a status paragraph

On 2026-11-09 fabric-coordinator's review of gateway PR #88 (F1, P3) found my passthrough-matrix-gaps memory carrying a new paragraph "all four closed by #83 (8e9274a)" while its heading, its `description` and its gap list still said "not asserted at the client". The drain uses the description as the slice's INDEX cue, so a session reading the INDEX was told the gaps were open, the opposite of the tree. A hand fix to the slice would come back at the next drain, because the drain re-harvests the memory.

**Why:** the description is what recall and the INDEX match on; a reader often never reaches the appended paragraph.

**How to apply:** when work closes what a memory reports as open, rewrite the `name`'s title line, the `description`, the anchors (to the closing tests) and the MEMORY.md line; move the old list under a past-tense "History" paragraph. Fix it in my own memory, never in the slice. Related: [[spike-facts-cite-committed-runs]].

*References: spike-facts-cite-committed-runs*

*Anchors: `agent-fabric-gateway:.agent-fabric/memory/rust-services-dev/solution/passthrough-matrix-gaps.md::passthrough`*

*Observed 2026-11-09 (rust-services-dev)*
