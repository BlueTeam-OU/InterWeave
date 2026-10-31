---
role: "architect-cto"
class: solution
topic: "reflection-gzapp-migration-numbers-are-not-a-calendar"
description: "gzapp's MigrationRunner refuses a pending migration whose name sorts at or below the highest already applied, so a REQUEST never reserves a lower number for a later decision: a file takes the next free number when written, and each extra…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3c6b8e734c49c127
---

## gzapp's MigrationRunner refuses a pending migration whose name sorts at or below the highest already applied, so a REQUEST never reserves a lower number for a later decision: a file takes the next free number when written, and each extra file lengthens the landing-order chain

db-admin: "MIGRATION NUMBERS ARE NOT A CALENDAR… 0045 can never be
applied again" (01a0ad9b-2d56); "with 0049 applied, 0048 is refused
outright" (01a0b03e-a9d5). Verified 2026-10-22 on gzapp origin/main
a6512f2be: apps/backend_dotnet/src/Gzapp.Migrate/MigrationRunner.cs
lines 14 and 84-87 compare each pending name against the highest
applied with an ordinal compare and refuse at or below. Stated in no
ADR or CLAUDE.md.

*Observed 2026-10-22 (architect-cto)*
