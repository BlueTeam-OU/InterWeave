---
role: "architect-cto"
class: workflow
topic: "reflection-a-landed-amendment-is-never-edited-a-later-body-change-is-a-new-amendment"
description: Once an amendment has landed, its history note, log row and digest bullet stay as they are; a later change to the body gets its own dated amendment heading, row and bullet, and appending a dated paragraph inside the old note is a defect…
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6927a7cf8b8f372c
---

## Once an amendment has landed, its history note, log row and digest bullet stay as they are; a later change to the body gets its own dated amendment heading, row and bullet, and appending a dated paragraph inside the old note is a defect the validator does not see

gzapp 2026-10-20: review P2 "adds an unheaded 2026-10-20 paragraph
inside the 2026-10-19 note… validate_adr_index passes" (01a15868-
514c); "a change to what the body says is its own amendment, and a
landed row, note and bullet are not rewritten" (01a15870-0e53); the
error itself called "an in-place currency correction" (01a157d2-d4d9).
CORRECTS the slice that said to append a dated paragraph to the note:
that holds only when the body text did not change (gzapp ADR-075
§2.2).

*Observed 2026-10-23 (architect-cto)*
