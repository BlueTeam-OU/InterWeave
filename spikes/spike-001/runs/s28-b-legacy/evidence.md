# s28-b-legacy

Model: claude-sonnet-5-5

## Run

```json
{
  "run": "s28-b-legacy",
  "claude_binary_version": "2.1.317",
  "stub_era": "legacy",
  "negotiation": null,
  "nonce": "52ffd342",
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
  "sandbox": "bwrap: the host's /etc entries read-only, /etc/claude-code an empty directory read-only",
  "screen_era_notice": null,
  "steps": [
    {
      "development_warning": true,
      "at_s": 2.1
    },
    {
      "channel_decision_at_s": 4.7
    }
  ],
  "channel_decision": "2026-11-03T08:21:22.529Z [DEBUG] MCP server \"spike001\": Channel notifications registered",
  "shutdown": {
    "signals_sent": [
      "SIGINT to the session"
    ],
    "group_empty": true,
    "bwrap_wait_status": 0
  },
  "versions_in_debug_log": [
    "2.1.317"
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
  "client_methods": [
    "server/discover",
    "initialize",
    "notifications/initialized",
    "tools/list"
  ],
  "initialize_asked": "2025-11-25",
  "protocol_era_lines": [
    "[DEBUG] MCP server \"spike001\": Connection established with capabilities: {\"hasTools\":true,\"hasPrompts\":false,\"hasResources\":false,\"hasResourceSubscribe\":false,\"serverVersion\":{\"name\":\"spike-001-s23\",\"version\":\"0.0.0\"},\"protocolEra\":\"legacy\",\"negotiatedProtocolVersion\":\"2025-11-25\"}"
  ],
  "passed": true
}
```

## Claude Code debug log: the run's own server only

```text
2026-11-03T08:21:22.184Z [DEBUG] MCP server "spike001": Starting connection with timeout of 30000ms
2026-11-03T08:21:22.505Z [DEBUG] MCP server "spike001": Successfully connected (transport: stdio) in 401ms
2026-11-03T08:21:22.507Z [DEBUG] MCP server "spike001": Connection established with capabilities: {"hasTools":true,"hasPrompts":false,"hasResources":false,"hasResourceSubscribe":false,"serverVersion":{"name":"spike-001-s23","version":"0.0.0"},"protocolEra":"legacy","negotiatedProtocolVersion":"2025-11-25"}
2026-11-03T08:21:22.529Z [DEBUG] MCP server "spike001": Channel notifications registered
2026-11-03T08:21:25.513Z [DEBUG] MCP server "spike001": notifications/claude/channel: S23 52ffd342: a channel event on the legacy stub
2026-11-03T08:21:48.381Z [DEBUG] MCP server "spike001": Sending SIGINT to MCP server process
```

## Terminal: the development-channel warning (whitespace removed by capture)

```text
WARNING:Loadingdevelopmentchannels--dangerously-load-development-channelsisforlocalchanneldevelopmentonly.Donotusethisoptiontorunchannelsyouhavedownloadedofftheinternet.Pleaseuse--channelstorunalistofapprovedchannels.Channels:server:spike001❯1.Iamusingthisforlocaldevelopment2.ExitEntertoconfirm·Esctocancel
```

## Session transcript: injected messages, attachments, replies, tool calls

**injected/user:**

```text
<channel source="spike001" chat_id="s23" era="legacy">
S23 52ffd342: a channel event on the legacy stub
</channel>
```

**model:**

```text
I received a channel event (S23 52ffd342, from the legacy stub), but it has no request or instructions in it. It's just an identifier and a label.

What would you like me to do? If you meant to send a task, please send it again with the details.
```
