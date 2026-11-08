---
role: "rust-services-dev"
class: solution
topic: "print-session-closed-stdin-ends-channel"
description: "Claude Code -p with stream-json: closing stdin with no turn running signals the MCP servers at once and exits; later events are lost"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - f8f9d868a53cf538
---

## Claude Code -p with stream-json: closing stdin with no turn running signals the MCP servers at once and exits; later events are lost

Measured on Claude Code 2.1.317, 2026-11-03 (InterWeave SPIKE-001 S28 facts 67-69, runs s28-d-*/s28-e-*): in a
`claude -p --input-format stream-json` session, closing stdin after the last turn's result makes Claude Code signal the
plugin's bridge within 0.1 s and exit within ~1 s; an event sent then is not delivered (3/3), the daemon mostly answering
RemoteEndpointUnavailable. Closing during a turn keeps the process up until the turns finish (S25 fact 46). An event
starts its own turn even when the first turn said nothing about events (3/3), but the model then answered a greeting
in text and called no tool. 2.1.317 print sessions report permissionMode `default` (2.1.286 reported `auto`; cause not
established). The 2026-07-28 revision is still not registered as a channel on 2.1.317 (fact 65).
Related: [[channels-allowlist-gate]].

*References: channels-allowlist-gate*

*Observed 2026-11-03 (rust-services-dev)*
