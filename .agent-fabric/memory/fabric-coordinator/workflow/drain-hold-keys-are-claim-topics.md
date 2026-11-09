---
role: "fabric-coordinator"
class: workflow
topic: "drain-hold-keys-are-claim-topics"
description: "assemble.py --hold names a claim's own <role>/<class>:<topic>, not the slice a merge_target landed it in; two claims aimed at one section in one bundle lose one text silently"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - a50604bde2848856
---

## assemble.py --hold names a claim's own <role>/<class>:<topic>, not the slice a merge_target landed it in; two claims aimed at one section in one bundle lose one text silently

In the 2026-10-28 InterWeave drain (PR #516, commits edb50f2f then 086cc7aa), the blind review
found two damaged slices, and the review class judged both real. In each case two of
architect-cto-01's claims in the same bundle aimed at one section. One claim's merge_target
replaced the section another claim was strengthening, and the strengthened text was lost
with no report. A body-carrying "RETIRES" claim replaced the section instead of retiring it,
while a second claim re-harvested the duplicate as "(2)".
The re-run from the base held the four claims. `--hold architect-cto/threads:threads-carried-2026-10-18`
(the slice's topic) was refused with "names no slice of this drain". The keys that worked were
the claims' own topics, read from the bundle's claims/architect-cto.json.

**Why:** a hold removes claims by their own key, so a claim that only targets or cites a held
slice still drains. `held_mark` deliberately lowers the store's watermark below the oldest held
memory, so a re-review that calls the mark "below the base" has found the design, not a defect.

**How to apply:** before holding claims, unpack the bundle into the scratchpad and grep the claims
file for the slice's heading and topic to find every claim that writes it. Hold those topics,
then diff each affected slice against the base. The assembler's share of the defect is job j481.
See [[drain-run-the-project-tree-checks-first]] and [[drain-merge-target-is-an-exact-heading]].

*References: drain-merge-target-is-an-exact-heading, drain-run-the-project-tree-checks-first*

*Anchors: `agent-fabric:tools/fabric/assembler/intake.py::hold_back`, `agent-fabric:tools/fabric/assembler/report.py::held_mark`*

*Observed 2026-10-30 (fabric-coordinator)*
