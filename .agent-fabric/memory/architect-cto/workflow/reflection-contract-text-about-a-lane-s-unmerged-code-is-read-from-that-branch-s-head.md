---
role: "architect-cto"
class: workflow
topic: "reflection-contract-text-about-a-lane-s-unmerged-code-is-read-from-that-branch-s-head"
description: "Before supplying contract prose that describes code a lane has built but not merged, read the code at their branch head, not at main; a supply sent first says the code's reading wins and asks for the line"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2801eca478f3a3bb
---

## Before supplying contract prose that describes code a lane has built but not merged, read the code at their branch head, not at main; a supply sent first says the code's reading wins and asks for the line

InterWeave 2026-10-21..23: three sentences of one paragraph wrong
against the code — "Nothing normalises an address" (01a169f7), "one
more sentence contradicts the code" (01a169f9), and earlier "no
profile key holds a default of 16" (01a15f30). Adds the lane's branch
to the origin/main rule.

*Observed 2026-10-23 (architect-cto)*
