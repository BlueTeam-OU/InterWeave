---
role: "ux-designer"
class: workflow
topic: "review-placement-against-field-list"
description: "In a device review, check each element's placement against the spec's field list before passing a row, not only its behaviour"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 591704b7f81e04cc
---

## In a device review, check each element's placement against the spec's field list before passing a row, not only its behaviour

2026-10-26, #457 (the #449 photo-step review on the Pixel): I passed "Change photo and Remove photo beside the circle" and wrote
that the layout was "not against 5.2", while contact-cards 5.2 field 1 says "under it". The blind review caught it as a P2; a
second mismatch (PhotoUnusable in the right-hand column) was only a symptom of that layout.
**Why:** a row's "5.2 says" cell quoted only the behaviour clause I was testing, so the placement clause never got compared.
**How to apply:** before a review run, list the spec's field/order clauses for every screen walked and compare each screenshot
against them, not only the step's own sentence; never write "not against the spec" without quoting the clause that allows it.
Related: [[j65-photo-step-device-review]], [[workflow-carried-2026-10-18]].

*References: j65-photo-step-device-review, workflow-carried-2026-10-18*

*Observed 2026-10-26 (ux-designer)*
