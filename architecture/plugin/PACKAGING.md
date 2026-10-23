# Packaging blueprint

No production plugin, daemon, or human client is created in this architecture repository.

## Future Claude plugin component

Conceptual contents:

```text
.claude-plugin/plugin.json
.mcp.json
skills/
bridge/
README.md
```

Bridge configuration must identify the transport profile, the local EndpointId to claim over IPC v2, and the delivery mode — `--delivery push|pull`, required; the Claude Code plugin's `.mcp.json` passes `push` (A 2026-10-07).

The manifest syntax, measured by SPIKE-001 against Claude Code 2.1.285 (facts 7, 8; A 2026-10-03 — this said "remains SPIKE-001"): `.claude-plugin/plugin.json` declares `"channels": [{"server": "<name>", "displayName": "…"}]`, where `<name>` is the server `.mcp.json` starts; with `author` present it passes `claude plugin validate --strict` and loads. Loaded through `--plugin-dir`, the plugin is inline, its server is named `plugin:<plugin>:<server>` and the channel flag value is `plugin:<plugin>@inline`; a bare `--mcp-config` server is named `<server>` and loaded as a channel by the flag value `server:<server>` (fact 7). Channel events reach the model only in an interactive session (fact 4). A local-marketplace install registers the channel under `--dangerously-load-development-channels`, accepted on its warning screen, as `--plugin-dir` does (facts 5–6; SPIKE-001 S19, 2026-10-22, Claude Code 2.1.286). Without that flag, `--channels plugin:interweave@interweave` registers it when the organisation's managed settings carry `allowedChannelPlugins` naming `{"plugin": "interweave", "marketplace": "interweave"}`, with `channelsEnabled: true`; no warning screen is shown, and a message reached the model and its reply the peer (SPIKE-001 S22, 2026-10-23, Claude Code 2.1.286, facts 29–33; on Linux the file is under `/etc/claude-code/`). Without that entry, `--channels` is refused (A 2026-10-23 — this said `--channels` with a published plugin stayed carried, the organisation path read from the binary and not tried). Still carried: Anthropic's remotely served allowlist, not asked for (plan §22), and the `channelsEnabled` refusal, which a claude.ai login applies only on a Team or Enterprise subscription (fact 32).

## Future transport component

```text
bin/interweave-transportd
bin/interweave-transportctl
config schema v2/docs
optional platform service integration
```

Daemon and bridge may ship together but remain independently versioned behind IPC v2.

## Future human client

The human client is a separate application package that consumes IPC v2; it does not bundle a second libp2p identity/runtime by default.

Conceptual pieces:

```text
human UI/TUI/CLI
IPC v2 data-plane adapter
application-local contacts + ADR-0044 retention store (pending outbound, unread inbound, receiver-kept inbound)
separate admin/settings adapter or privileged connection
```

The data-plane session claims a configured EndpointId such as `human`. Admin settings require separate capabilities and explicit local action.

## Skills / local configuration helpers

Helpers may initialize profiles/endpoints, display status, edit non-secret config, and guide allowlist changes. Remote Channel/network content is never sufficient authority for trust or endpoint-administration mutation.
