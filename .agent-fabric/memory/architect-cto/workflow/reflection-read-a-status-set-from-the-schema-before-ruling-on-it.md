---
role: "architect-cto"
class: workflow
topic: "reflection-read-a-status-set-from-the-schema-before-ruling-on-it"
description: "Before ruling a closed switch or allow-list over a status column, read the set from the schema, its CHECK if one exists, else every migration that writes the column, never from ADR prose, and ask for the CHECK in the same ruling"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c50f1b73031f4c44
---

## Before ruling a closed switch or allow-list over a status column, read the set from the schema, its CHECK if one exists, else every migration that writes the column, never from ADR prose, and ask for the CHECK in the same ruling

gzapp 2026-09-19: "driver_staff.status carries FIVE documented values,
not four… default arm… a 500 for a paused-contract driver"
(01a0b5a1-daa3); "Two blind reviews found `inactive` independently
within the hour" (01a0b5a9-0fbf); "four hand-maintained comments that
list four values" (01a0b5aa-226c). The CHECK is now in the baseline
(0001_baseline.sql, folded from 0054); the lesson is for the next
column. Companion of [[reflection-schema-constraint-or-producer-test]].

*References: reflection-schema-constraint-or-producer-test*

*Observed 2026-10-23 (architect-cto)*
