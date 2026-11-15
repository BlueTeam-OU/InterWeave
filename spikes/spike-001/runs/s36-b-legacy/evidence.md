# s36-b-legacy

Model: claude-sonnet-5-5

## Run

```json
{
  "run": "s36-b-legacy",
  "tree": "07a7ac729",
  "tree_dirty": [],
  "tree_dirty_count": 0,
  "claude_binary_version": "2.1.325",
  "stub_era": "legacy",
  "negotiation": null,
  "nonce": "239ccd75",
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
      "at_s": 1.5
    },
    {
      "channel_decision_at_s": 3.6
    }
  ],
  "channel_decision": "2026-11-14T10:45:51.733Z [DEBUG] MCP server \"spike001\": Channel notifications registered",
  "shutdown": {
    "signals_sent": [
      "SIGINT to the session"
    ],
    "group_empty": true,
    "bwrap_wait_status": 0
  },
  "versions_in_debug_log": [
    "2.1.325"
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
2026-11-14T10:45:51.517Z [DEBUG] MCP server "spike001": Starting connection with timeout of 30000ms
2026-11-14T10:45:51.710Z [DEBUG] MCP server "spike001": Successfully connected (transport: stdio) in 283ms
2026-11-14T10:45:51.711Z [DEBUG] MCP server "spike001": Connection established with capabilities: {"hasTools":true,"hasPrompts":false,"hasResources":false,"hasResourceSubscribe":false,"serverVersion":{"name":"spike-001-s23","version":"0.0.0"},"protocolEra":"legacy","negotiatedProtocolVersion":"2025-11-25"}
2026-11-14T10:45:51.733Z [DEBUG] MCP server "spike001": Channel notifications registered
2026-11-14T10:45:54.748Z [DEBUG] MCP server "spike001": notifications/claude/channel: S23 239ccd75: a channel event on the legacy stub
2026-11-14T10:46:17.365Z [DEBUG] MCP server "spike001": Sending SIGINT to MCP server process
2026-11-14T10:46:17.415Z [DEBUG] MCP server "spike001": MCP server process exited cleanly
```

## Terminal: the development-channel warning (whitespace removed by capture)

```text
WARNING:Loadingdevelopmentchannels--dangerously-load-development-channelsisforlocalchanneldevelopmentonly.Donotusethisoptiontorunchannelsyouhavedownloadedofftheinternet.Pleaseuse--channelstorunalistofapprovedchannels.Channels:server:spike001❯1.Iamusingthisforlocaldevelopment2.ExitEntertoconfirm·Esctocancel
```

## Session transcript: injected messages, attachments, replies, tool calls

**injected/user:**

```text
<system-reminder>
<tools-used-before-summary>mcp__spike001__channel</tools-used-before-summary>
</system-reminder>
```

**injected/user:**

```text
<channel source="spike001" chat_id="s23" era="legacy">
S23 239ccd75: a channel event on the legacy stub
</channel>
```

**model:**

```text
I received a channel event from the `spike001` source (chat s23, legacy era): "S23 239ccd75: a channel event on the legacy stub". It doesn't contain a request or instruction, so there's nothing for me to act on.

If you want me to do something with it, such as investigate the legacy stub or look for related code in the working directory, tell me what you'd like.
```
