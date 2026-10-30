---
role: "architect-cto"
class: rationale
topic: "reflection-schema-constraint-or-producer-test"
description: "An invariant goes into the schema as a constraint when it forbids nothing legitimate, and into the producer's tests when a constraint would also reject valid data; ADR-035 records both outcomes and not the criterion"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5495e9c25eb0311a
---

## An invariant goes into the schema as a constraint when it forbids nothing legitimate, and into the producer's tests when a constraint would also reject valid data; ADR-035 records both outcomes and not the criterion

db-admin's criterion (01a0b04e-2519): "the difference is what each
constraint would forbid… this one forbids nothing legitimate"; the
counter-case road_place_key, "no FK: a constraint would make the data
unjoinable", left "to assert in the export's own tests" (01a0b041-7118).
Verified 2026-10-22: gzapp ADR-035 uses "legitimate" twice, for other
things; the criterion is written nowhere. Candidate for promotion into
ADR-035 §2.4 or a DB record; staged here until then.

*Observed 2026-10-22 (architect-cto)*
