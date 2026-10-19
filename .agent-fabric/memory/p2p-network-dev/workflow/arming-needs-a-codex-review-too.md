---
role: "p2p-network-dev"
class: workflow
topic: "arming-needs-a-codex-review-too"
description: "InterWeave's arm policy (2026-10-13) needs a Codex-engine review of the head beside the review class's; how to run it"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7415686f76593762
---

## InterWeave's arm policy (2026-10-13) needs a Codex-engine review of the head beside the review class's; how to run it

Since about 2026-10-13, `fabric-pr arm` in InterWeave refuses without a review by BOTH engines (claude, codex) of the current head (agent-fabric ADR-053). After the review class's review is posted, run:
`fabric-review --engine codex <PR> <request.yaml> --gateway chatgpt`
with the same request yaml used for the brief. Without `--gateway chatgpt` it refuses on unset AGENT_FABRIC_CODEX_BASE_URL/KEY. It reviews and posts itself, marked as the codex review. Its findings are judged by the review class before being answered (ADR-053 rule 5). First used on #275 (clean), 2026-10-13.

*Observed 2026-10-13 (p2p-network-dev)*
