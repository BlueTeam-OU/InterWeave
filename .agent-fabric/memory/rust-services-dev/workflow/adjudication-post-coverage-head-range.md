---
role: "rust-services-dev"
class: workflow
topic: "adjudication-post-coverage-head-range"
description: "Posting a finding-adjudication (range head..head) is refused when its coverage block names paths; empty the paths and say so"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - c48914faf09ffff6
---

## Posting a finding-adjudication (range head..head) is refused when its coverage block names paths; empty the paths and say so

A finding-adjudication brief must use range `<head>..<head>` and carry a `card_base`; `fabric-review brief` refuses any other range. Its reviewer still lists the files it read in the coverage block. `fabric-pr post-review` then refuses the report: "the coverage block names N path(s) outside the range". On PR #580 (2026-11-06) the fix was to replace the block with `{"paths": {}, "lenses": {...}}`, add a note under it naming what was read, and post. The judgements counted.

A re-review brief must leave out `previous_findings`: it reads the last posted review on its own. Post the adjudication before the re-review, so that the re-review picks it up. `fabric-pr post-review` must be the bare command, with nothing chained.

Related: [[review-brief-rounds-mechanics]].

*References: review-brief-rounds-mechanics*

*Anchors: `agent-fabric:tools/fabric/review_brief.py::finding-adjudication`*

*Observed 2026-11-06 (rust-services-dev)*
