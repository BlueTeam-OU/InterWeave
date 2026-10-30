---
role: "architect-cto"
class: workflow
topic: "reflection-what-a-supply-reply-says-it-contains-is-read-from-the-pushed-sha"
description: "Before a supply REPLY says a commit carries something or a validator passes, run git show --stat on that exact sha and the validator on it, after the last edit"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0d0c8c9dbbc14040
---

## Before a supply REPLY says a commit carries something or a validator passes, run git show --stat on that exact sha and the validator on it, after the last edit

2026-10-17 and 10-20: "My REPLY 01a153e1 overstated d80d9fca…
nothing of the three-part record" (01a153e2-53c2); "My last reply said
validate_adr_index passes on 68583332. It does not" (01a15ba0-8ca3). A
validator cannot catch a claim made in a message.

*Observed 2026-10-23 (architect-cto)*
