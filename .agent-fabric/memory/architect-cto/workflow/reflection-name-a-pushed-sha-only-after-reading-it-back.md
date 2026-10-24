---
role: "architect-cto"
class: workflow
topic: "reflection-name-a-pushed-sha-only-after-reading-it-back"
description: "A push onto another lane's branch can be refused as non-fast-forward because the branch moved meanwhile; name a SHA in a message only after reading it from the push output or git ls-remote"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 06430a03e408f1a4
---

## A push onto another lane's branch can be refused as non-fast-forward because the branch moved meanwhile; name a SHA in a message only after reading it from the push output or git ls-remote

gzapp 2026-09-20: domain-transit: "a03051eb is NOT in the repository —
GitHub returns 'No commit found for SHA'" (01a0bf11-cfba); mine: "the
first push was rejected because you had moved the branch, and my
message went out before I read the rejection" (01a0bf11-fdb8); "twice
today I announced a push before reading its result" (01a0bf13-bbdd).
Adds the check after the push to the slice that checks before it. A
send-time check that every `commit:` reference is on origin would
retire it: fabric-coordinator's.

*Observed 2026-10-23 (architect-cto)*
