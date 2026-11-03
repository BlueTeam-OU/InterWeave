---
role: "fabric-coordinator"
class: threads
topic: "adr-0067-roster-endpoint-ids-plan"
description: "InterWeave ADR-0067 (derived EndpointIds) lands in the fabric as an ADR-064 amendment, built only when Stage 22 opens (j458)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 216b2f0f5ae83a22
---

## InterWeave ADR-0067 (derived EndpointIds) lands in the fabric as an ADR-064 amendment, built only when Stage 22 opens (j458)

InterWeave ADR-0067 (#506, 2026-10-28) makes every EndpointId derived from an Ed25519 key. The fabric's answer (REPLY 01a18fc0 to architect-cto-01, 2026-10-31): an amendment of agent-fabric ADR-064 rules 2, 3, 12, not a new record. Build: roster.schema.json v2 (row `endpoint` label -> alias-to-EndpointId map, signed and countersigned), `render_config` in tools/fabric/gzcoord/carrier/roster.py writes key-file + alias entries, provisioning runs `transportctl endpoint init` per account beside `identity init`, trust stays static-allowlist (endpoint-less only), release.json pinned to the Stage 22 release with every account re-rendered at once.

**Why:** nothing to build until Stage 22 opens on the owner's word and `transportctl endpoint init` exists (absent from InterWeave main on 2026-10-31).
**How to apply:** j458 is blocked on that; when it clears, write the amendment first, then dispatch the roster/render change to a python-dev holder. Related: [[solution-carried-2026-10-24]].

*References: interweave-human-cli-is-not-a-carrier*

*Observed 2026-10-31 (fabric-coordinator)*
