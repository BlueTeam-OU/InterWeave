---
role: "architect-cto"
class: workflow
topic: "reflection-search-origin-s-unmerged-branches-before-writing-a-gap"
description: "A decision I authored can sit implemented on another lane's unmerged branch with no PR, pointed at by nothing on main or in the inbox; before writing a recorded gap or routing a job, list origin's for/ and feat/ branches and search them…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 06bb8dc19ca1b71b
---

## A decision I authored can sit implemented on another lane's unmerged branch with no PR, pointed at by nothing on main or in the inbox; before writing a recorded gap or routing a job, list origin's for/ and feat/ branches and search them for the decision's anchor

gzapp 2026-09-25: "STAND DOWN… the job already exists on backend-dev-
02's feat/served-chainage… I wrote #945's 'recorded gap' without
grepping origin for it" (01a0ddf9-3f78); "nothing in main or the inbox
pointed at an unmerged branch holding a decision I had authored. Two
lanes lost an hour" (01a0de11-25b5); the withdrawal to db-admin
(01a0ddf9-4103). Widens the cited-commit grep to origin's branches;
`fabric-pr gate --in-flight --path` is the instrument now.

*Observed 2026-10-23 (architect-cto)*
