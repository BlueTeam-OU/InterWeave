# s23-b-legacy

Model: claude-sonnet-5-5

## Run

```json
{
  "run": "s23-b-legacy",
  "claude_binary_version": "2.1.286",
  "stub_era": "legacy",
  "negotiation": null,
  "nonce": "16ab96d9",
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
      "at_s": 1.6
    },
    {
      "channel_decision_at_s": 4.8
    }
  ],
  "channel_decision": "2026-10-24T18:12:49.783Z [DEBUG] MCP server \"spike001\": Channel notifications registered",
  "shutdown": {
    "signals_sent": [
      "SIGINT to Claude Code"
    ],
    "group_empty": true,
    "bwrap_wait_status": null
  },
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
2026-10-24T18:12:49.293Z [DEBUG] MCP server "spike001": Starting connection with timeout of 30000ms
2026-10-24T18:12:49.725Z [DEBUG] MCP server "spike001": Successfully connected (transport: stdio) in 437ms
2026-10-24T18:12:49.725Z [DEBUG] MCP server "spike001": Connection established with capabilities: {"hasTools":true,"hasPrompts":false,"hasResources":false,"hasResourceSubscribe":false,"serverVersion":{"name":"spike-001-s23","version":"0.0.0"},"protocolEra":"legacy","negotiatedProtocolVersion":"2025-11-25"}
2026-10-24T18:12:49.783Z [DEBUG] MCP server "spike001": Channel notifications registered
2026-10-24T18:12:52.736Z [DEBUG] MCP server "spike001": notifications/claude/channel: S23 16ab96d9: a channel event on the legacy stub
2026-10-24T18:13:15.789Z [DEBUG] MCP server "spike001": Sending SIGINT to MCP server process
```

## Terminal: the development-channel warning (whitespace removed by capture)

```text
WARNING:Loadingdevelopmentchannels--dangerously-load-development-channelsisforlocalchanneldevelopmentonly.Donotusethisoptiontorunchannelsyouhavedownloadedofftheinternet.Pleaseuse--channelstorunalistofapprovedchannels.Channels:server:spike001❯1.Iamusingthisforlocaldevelopment2.ExitEntertoconfirm·Esctocancel
```

## Session transcript: injected messages, attachments, replies, tool calls

**injected/user:**

```text
<channel source="spike001" chat_id="s23" era="legacy">
S23 16ab96d9: a channel event on the legacy stub
</channel>
```

**model:**

```text
I got a channel event from the "spike001" channel (chat s23, legacy era). It reads "S23 16ab96d9: a channel event on the legacy stub". It doesn't contain a task or a question, so I haven't done anything with it.

What would you like me to do? If this event was meant to trigger something specific, tell me what.
```
