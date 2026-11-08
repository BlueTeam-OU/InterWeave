---
role: "fabric-coordinator"
class: workflow
topic: "oversized-claim-is-the-authors-split"
description: "a slice over the 1800-token budget that is ONE section is an atomic claim the assembler cannot split — ask its origin agent to split the memory; lint fails only past 1800 x 1.35"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 6de7c2041dadcfd5
---

## a slice over the 1800-token budget that is ONE section is an atomic claim the assembler cannot split — ask its origin agent to split the memory; lint fails only past 1800 x 1.35

The assembler's budget split (agent-fabric tools/fabric/assembler/writer.py,
split_by_budget and write_role) groups claims into parts. A single claim over
the budget is written whole and listed in the drain report's `oversized_claims`.
Corpus lint (tools/fabric/lint_rules/slices.py) fails a slice only past
BUDGET_TOKENS 1800 x BUDGET_TOLERANCE 1.35 (~2430), so claims between those two
figures pass silently. j484 (2026-11-05): the 2026-10-31 InterWeave drain listed
16 oversized claims, and 3 failed lint (p2p-network-dev j94-stage-19-track-a,
rust-ui-dev j134-step5-inputs, ux-designer j39-add-peer-unknown-retries). The
architect-cto 7 had already been split by that drain. The fix was a REQUEST to
each origin login (the slice's `origin.agent`) to split its own memory. The
coordinator never edits the slice. The gap that nobody tells the author is j613. All three origins answered
2026-11-04 (memories split or shortened, no slice touched); rust-ui-dev's
j68-b4-plan/j68-b4c-plan have no source left, and j66/j69 device rows are
p2p-network-dev-02's (they pass lint). The fix shows only at the next drain
(j614), which j484 is blocked on: a split memory changes nothing on main
until a drain rewrites the slice.

**Why:** an atomic claim cannot be split by the drain, and the slice must not be authored by me.
**How to apply:** for an over-budget slice, count its `## ` sections first. If there is one, request the split from the origin agent; if there are several, it is an assembler defect.

*Anchors: `agent-fabric:tools/fabric/assembler/writer.py::oversized`, `agent-fabric:tools/fabric/lint_rules/prompts.py::BUDGET_TOLERANCE`*

*Observed 2026-11-04 (fabric-coordinator)*
