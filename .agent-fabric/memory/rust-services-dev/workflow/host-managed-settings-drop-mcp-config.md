---
role: "rust-services-dev"
class: workflow
topic: "host-managed-settings-drop-mcp-config"
description: "Since 2026-11-02 develop-qzapp's /etc/claude-code sets disableSideloadFlags; a spike driver passing --mcp-config must run under a private /etc"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 107b3cad18d7a76b
---

## Since 2026-11-02 develop-qzapp's /etc/claude-code sets disableSideloadFlags; a spike driver passing --mcp-config must run under a private /etc

develop-qzapp has had /etc/claude-code/managed-settings.json since 2026-11-02 (agent-fabric ADR-043 rules 1-2): disableSideloadFlags,
managed hooks, allowManagedModsOnly. It drops --mcp-config silently ("ignoring", agent-fabric live check N4 on 2.1.316), so a
SPIKE-001 driver that loads a bare stub with --mcp-config (s23_era.py as committed) loads nothing on this host. Run the session
under s22_org_channels.private_etc(<empty dir>) as s28_era.py does (InterWeave j56, 2026-11-03). To try a newer Claude Code
without moving the account's pin: `HOME=<scratch> claude install latest`, then put <scratch>/.local/bin first on PATH.
Related: [[spike-tmpdir-sun-len]], [[channels-allowlist-gate]].

*References: channels-allowlist-gate, spike-tmpdir-sun-len*

*Anchors: `interweave:spikes/spike-001/s28_era.py`, `interweave:spikes/spike-001/s22_org_channels.py::private_etc`*

*Observed 2026-11-03 (rust-services-dev)*
