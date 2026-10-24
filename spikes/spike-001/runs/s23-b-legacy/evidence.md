# s23-b-legacy

Model: claude-sonnet-5-5

## Run

```json
{
  "run": "s23-b-legacy",
  "claude_binary_version": "2.1.286",
  "stub_era": "legacy",
  "negotiation": null,
  "nonce": "492dd650",
  "command": [
    "claude",
    "--mcp-config",
    "mcp.json",
    "--strict-mcp-config",
    "--debug-file",
    "$RAW/debug.txt",
    "--dangerously-load-development-channels",
    "server:spike001"
  ],
  "screen_era_notice": null,
  "steps": [
    {
      "development_warning": true,
      "at_s": 2.1
    },
    {
      "channel_decision_at_s": 5.9
    }
  ],
  "channel_decision": "2026-10-24T16:20:34.795Z [DEBUG] MCP server \"spike001\": Channel notifications registered",
  "versions_in_debug_log": [
    "2.1.286"
  ],
  "env_passed": [
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CONFIG_DIR",
    "COLUMNS",
    "DISABLE_AUTOUPDATER",
    "HOME",
    "LANG",
    "LINES",
    "LOGNAME",
    "PATH",
    "SHELL",
    "TERM",
    "TMPDIR",
    "USER",
    "XDG_RUNTIME_DIR"
  ],
  "event_reached_conversation": true,
  "passed": true
}
```

## Claude Code debug log: the run's own server only

```text
2026-10-24T16:20:33.904Z [DEBUG] MCP server "spike001": Starting connection with timeout of 30000ms
2026-10-24T16:20:34.704Z [DEBUG] MCP server "spike001": Successfully connected (transport: stdio) in 828ms
2026-10-24T16:20:34.707Z [DEBUG] MCP server "spike001": Connection established with capabilities: {"hasTools":true,"hasPrompts":false,"hasResources":false,"hasResourceSubscribe":false,"serverVersion":{"name":"spike-001-s23","version":"0.0.0"},"protocolEra":"legacy","negotiatedProtocolVersion":"2025-11-25"}
2026-10-24T16:20:34.795Z [DEBUG] MCP server "spike001": Channel notifications registered
2026-10-24T16:20:37.723Z [DEBUG] MCP server "spike001": notifications/claude/channel: S23 492dd650: a channel event on the legacy stub
2026-10-24T16:21:00.352Z [DEBUG] MCP server "spike001": Sending SIGINT to MCP server process
```

## Terminal: the development-channel warning (whitespace removed by capture)

```text
WARNING:Loadingdevelopmentchannels--dangerously-load-development-channelsisforlocalchanneldevelopmentonly.Donotusethisoptiontorunchannelsyouhavedownloadedofftheinternet.Pleaseuse--channelstorunalistofapprovedchannels.Channels:server:spike001❯1.Iamusingthisforlocaldevelopment2.ExitEntertoconfirm·Esctocancel
```

## Session transcript: injected messages, attachments, replies, tool calls

**injected/user:**

```text
<channel source="spike001" chat_id="s23" era="legacy">
S23 492dd650: a channel event on the legacy stub
</channel>
```

**model:**

```text
I received a channel event on the legacy stub (S23 492dd650), but it contains no actual request or task. There's nothing for me to act on yet.

What would you like me to do?
```
