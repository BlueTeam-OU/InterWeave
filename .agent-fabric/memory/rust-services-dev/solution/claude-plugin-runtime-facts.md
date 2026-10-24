---
role: "rust-services-dev"
class: solution
topic: "claude-plugin-runtime-facts"
description: "Claude Code 2.1.286 plugin runtime, measured CLI-only: userConfig defaults/--config reach argv; a directory marketplace runs in place; MCP stderr goes to ~/.cache/claude-cli-nodejs"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 37409311bd15cb28
---

## Claude Code 2.1.286 plugin runtime, measured CLI-only: userConfig defaults/--config reach argv; a directory marketplace runs in place; MCP stderr goes to ~/.cache/claude-cli-nodejs

Measured 2026-10-22 on Claude Code 2.1.286 with no session (InterWeave #371, SPIKE-001 S20/S21,
spikes/spike-001/runs/s20-plugin-config and s21-refused-start):
- A plugin's userConfig option `default` reaches `${user_config.KEY}` in .mcp.json args when nothing is set.
  `claude plugin install <p> --config KEY=VALUE` sets it, and the value is kept in settings.json pluginConfigs.
- `claude mcp get <server>` starts the server to check it. Point the plugin's bin at a wrapper that logs "$@"
  and it shows the argv Claude Code passes, with no session, model or credential.
- With a marketplace added from a directory, the plugin runs from that directory (CLAUDE_PLUGIN_ROOT is the
  marketplace's own plugin dir), equal-version rebuilds included. plugin update reports only a higher
  version, and the cache's per-version copies are not what runs.
- Claude Code keeps a plugin MCP server's stderr as "Server stderr:" entries in
  ~/.cache/claude-cli-nodejs/<cwd>/mcp-logs-plugin-<plugin>-<server>/<ts>.jsonl, connected or not.
- A stdio server that exits at start shows "Failed to connect" and is never restarted, so the bridge now
  serves a refusal instead (apps/claude-channel/src/refused.rs). See [[channels-allowlist-gate]].

*References: channels-allowlist-gate*

*Observed 2026-10-22 (rust-services-dev)*
