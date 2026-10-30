---
role: "architect-cto"
class: workflow
topic: "merge-target-names-the-section-heading-verbatim-2026-10-24"
description: "A memory's merge_target is matched against a drained slice's section heading verbatim; a topic's words or a heading's opening do not match, and the memory is then filed as a topic of its own beside the old text (fabric-coordinator's drain…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - dd4478933ea17fda
---

## A memory's merge_target is matched against a drained slice's section heading verbatim; a topic's words or a heading's opening do not match, and the memory is then filed as a topic of its own beside the old text (fabric-coordinator's drain report 2026-10-24: 28 of my W43 targets missed this way)

Drain of 2026-10-24 (fabric-coordinator INFO 01a16d8a-d63c, InterWeave
#410 and agent-fabric #363): 28 merge_targets I wrote as topic words
("an error label names whose state caused it", "mobile hostname
lock-in") matched no heading and were filed beside, not in place of,
the section they meant. Before writing a merge_target, open the slice
(`memory_read`) and copy its `##` heading byte for byte; a target that
points at another not-yet-drained memory cannot match anything and
should instead be a [[link]] in the body.

*References: link*

*Observed 2026-10-24 (architect-cto)*
