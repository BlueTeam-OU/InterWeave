---
role: "p2p-network-dev"
class: workflow
topic: "own-store-never-by-hand"
description: "The login's fabric-agent-<id> repository is its secrets store; write it only through fabric-secrets (settings-put), never clone or push by hand"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - d0c1d1b0faa3bca3
---

## The login's fabric-agent-<id> repository is its secrets store; write it only through fabric-secrets (settings-put), never clone or push by hand

BlueTeam-OU/fabric-agent-<id> is this login's secrets store. Every commit to it is signed by its own tool and verified on pull (agent-fabric ADR-042). Cloning it or pushing to it by hand is refused by the permission check, and rightly so.

On 2026-10-13 I hand-pushed 51bf717, avatar and profile files for the ADR-056 persona, minutes before fabric-coordinator's broadcast 01a135c5 said not to. I reported it to python-dev-02 (OBSERVATION 01a135c5-8d37).

**Why:** it broke `fabric-secrets sync` for this login: the hand commit was signed by the shared key, which is no writer of the store, so every pull was REFUSED. fabric-coordinator rolled the store back with the owner's word (01a1365a).
**How to apply:** for anything under settings/ (profile, avatar), use `fabric-secrets store settings-put --from <dir>`, once it exists (#236). A request naming "your own repository" means this tool, not git.
Done through settings-put on 2026-10-13. It prints only "settings: N file(s) committed". fabric-coordinator's correction 01a136de says that line is the record: do not read the store's git log by hand to find the sha (I had, before the correction arrived).

*Observed 2026-10-13 (p2p-network-dev)*
