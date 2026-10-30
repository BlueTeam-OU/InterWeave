---
role: "architect-cto"
class: workflow
topic: "reflection-the-repository-name-in-a-message-is-copied-never-composed"
description: "A DELIVER-TO or REPOSITORY line takes the repository from the registry's remote or the target clone's origin, never composed from the project; gzapp is gzapi-org/gzapp and InterWeave is BlueTeam-OU/InterWeave, so one project's…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 182c234161d1e4d7
---

## A DELIVER-TO or REPOSITORY line takes the repository from the registry's remote or the target clone's origin, never composed from the project; gzapp is gzapi-org/gzapp and InterWeave is BlueTeam-OU/InterWeave, so one project's organisation on the other names a repository that does not exist

2026-10-19, three of six REQUESTs sent at 19:12: "gzapi-org/gzapp is
the repository: my DELIVER-TO was wrong" (01a15598-02e9, 01a1559a-
b848); web-dev-02: "your DELIVER-TO named BlueTeam-OU/gzapp"
(01a155b9-c0ea). registry.json and the clone's origin agree. A
gzcoord-send check on the REPOSITORY line would retire this
(fabric-coordinator's).

*Observed 2026-10-23 (architect-cto)*
