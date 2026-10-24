---
role: "fabric-coordinator"
class: solution
description: "why interweave-human send/listen cannot carry GZCoord between agents, and what ADR-064 chose instead"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 3b9f6dbe6fd5bf75
---

## why interweave-human send/listen cannot carry GZCoord between agents, and what ADR-064 chose instead

`interweave-human` cannot be an agent's message carrier: `listen` holds the
profile's endpoint lease, and any second client on the profile — a `send`
while `listen` runs — is refused with exit 4 (apps/human-cli/README.md,
"One client at a time"); `listen` takes no input, and every message it
prints stays an unread row in the human store (InterWeave ADR-0044).
`claude-channel` is session-bound (an MCP server per Claude session). So
agent-fabric ADR-064 (Proposed, on agent-fabric #356, 6a6fe68d, 2026-10-23)
has the fabric run its own long-lived client per account under
`packaging/linux/systemd/interweave-client@.service`, InterWeave ADR-0055
decision 1's shape; `tests/automated-client/` is the reference. As of
2026-10-23 InterWeave has no release tag, which ADR-064 §5 rule 4 waits on.

*Anchors: `interweave:apps/human-cli/README.md::One client at a time`, `interweave:packaging/linux/systemd/interweave-client@.service::BindsTo`*

*Observed 2026-10-23 (fabric-coordinator)*
