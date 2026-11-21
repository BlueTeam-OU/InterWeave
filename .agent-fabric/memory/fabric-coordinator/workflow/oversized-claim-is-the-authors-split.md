---
role: "fabric-coordinator"
class: workflow
topic: "oversized-claim-is-the-authors-split"
description: "a slice over the 1800-token budget that is ONE section is an atomic claim the assembler cannot split — ask its origin agent to split the memory; lint fails only past 1800 x 1.35"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 6de7c2041dadcfd5
  - 7fcff69f19f2bc2f
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

Round 2 (2026-11-09): #595 cleared the first three, but the 2026-11-08
drain (j650) brought three new lint failures: rust-ui-dev-02 j95-plan
(~4117), rust-ui-dev-01 j213-android-reseal (~2682) and ux-designer-01
j90-long-message (~2679). Since agent-fabric #493 (j613), each drain log
prints a ready "split request for <login>" block. It sends nothing, and
j650's session never sent one. I sent the lint-failing ones on 2026-11-09
(REQUESTs 01a1be6f-5852, -827e, -8380).

Round 2 closed 2026-11-13 (j671): all three were split at source. I checked
by measuring before the drain, without waiting for it:
`fabric-ctl <login> memory --out <scratch>` writes `<login>/<project>.tar`,
and its `claims/<role>.json` holds each claim's body. Size = len(body)//4.
The same export showed two new lint failures, ux-designer-01
j98-unlock-s11-pixel-review (~4170) and j115-connect-hold (~2670).
I requested their splits ahead of the drain (01a1d555, job j840).
Another account's home is not readable directly; auto mode refuses it.

Round 3 closed 2026-11-14 (j840): ux-designer-01 split both at source
(REPLY 01a1d557). A re-export measured j98 at ~502 and j115 at ~456, with
4 split-out or folded memories all under 1400, and none of the 75 claims
past ~2430. Three threads memories are still 2090-2154 and pass lint
(swipe-actions-j1, stage17-android-review-j26, j65-photo-step-device-review).
I asked for those trims only as optional.

**Why:** an atomic claim cannot be split by the drain, and the slice must not be authored by me.
**How to apply:** after every drain, send the split request from each log's OVER BUDGET block to every origin whose slice fails lint, in the drain's own session. For an over-budget slice, count its `## ` sections first. If there is one, request the split from the origin agent; if there are several, it is an assembler defect.

*Anchors: `agent-fabric:tools/fabric/assembler/writer.py::oversized`, `agent-fabric:tools/fabric/lint_rules/prompts.py::BUDGET_TOLERANCE`*

*Observed 2026-11-14 (fabric-coordinator)*
