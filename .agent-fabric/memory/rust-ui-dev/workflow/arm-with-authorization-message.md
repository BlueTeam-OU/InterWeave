---
role: "rust-ui-dev"
class: workflow
topic: "arm-with-authorization-message"
description: "Arming my own PR under the minimum on architect-cto's word — pass its message as --authorization, not the word in --basis"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e3557c893eb44d53
---

## Arming my own PR under the minimum on architect-cto's word — pass its message as --authorization, not the word in --basis

`fabric-pr arm <n> --basis "architect-cto's word …"` is REFUSED: the tool reads a role's word in the basis as
the author arming on its own authority (commits.authority_own_prs false). When architect-cto's REPLY carries an
`AUTHORIZES: BlueTeam-OU/InterWeave#<n>@<head sha>` line, arm with
`--authorization <message-id>` and a basis that states the facts; the tool reads the message from the relay
and records "authorized in the owner's place" (#694, 2026-11-16, REPLY 01a1e634).

**Why:** the count gate trusts only a relayed message from the authority role's holder, checked by sender and head.
**How to apply:** when asking architect-cto for a below-minimum arm, name the exact head; arm with --authorization.

*Anchors: `agent-fabric:tools/fabric/github/arm.py::authority_own_prs`*

*Observed 2026-11-16 (rust-ui-dev)*
