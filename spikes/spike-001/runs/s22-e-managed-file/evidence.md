# s22-e-managed-file

## Run

```json
{
  "run": "s22-e-managed-file",
  "managed_settings": {
    "path": "/etc/claude-code/managed-settings.json",
    "content": {
      "channelsEnabled": true,
      "allowedChannelPlugins": [
        {
          "plugin": "interweave",
          "marketplace": "interweave"
        }
      ]
    }
  },
  "setup": [
    {
      "command": [
        "claude",
        "plugin",
        "marketplace",
        "add",
        "$SCRATCH/marketplace"
      ],
      "exit": 0,
      "output": "Adding marketplace\u2026\u2714 Successfully added marketplace: interweave (declared in user settings)"
    },
    {
      "command": [
        "claude",
        "plugin",
        "install",
        "interweave@interweave"
      ],
      "exit": 0,
      "output": "Installing plugin \"interweave@interweave\"...\u2714 Successfully installed plugin: interweave@interweave (scope: user)\n2 userConfig options not yet set \u2014 run /plugin configure interweave@interweave in Claude Code, or pass --config KEY=VALUE."
    }
  ],
  "command": [
    "claude",
    "--debug-file",
    "$RAW/debug.txt",
    "--channels",
    "plugin:interweave@interweave"
  ],
  "sandbox": "bwrap: the host's /etc entries read-only, /etc/claude-code a scratch directory read-only",
  "steps": [
    {
      "channel_decision_at_s": 6.3,
      "registered": true
    }
  ],
  "wait_status": 2,
  "versions_in_debug_log": [
    "2.1.286"
  ],
  "channel_decision": "2026-10-23T04:53:48.457Z [DEBUG] MCP server \"plugin:interweave:interweave\": Channel notifications registered",
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
  "passed": true
}
```

## Claude Code debug log: the run's own server only

```text
2026-10-23T04:53:47.851Z [DEBUG] MCP server "plugin:interweave:interweave": Starting connection with timeout of 30000ms
2026-10-23T04:53:48.411Z [DEBUG] MCP server "plugin:interweave:interweave": Successfully connected (transport: stdio) in 567ms
2026-10-23T04:53:48.412Z [DEBUG] MCP server "plugin:interweave:interweave": Connection established with capabilities: {"hasTools":true,"hasPrompts":false,"hasResources":false,"hasResourceSubscribe":false,"serverVersion":{"name":"interweave","version":"0.0.0"},"protocolEra":"legacy","negotiatedProtocolVersion":"2025-11-25"}
2026-10-23T04:53:48.457Z [DEBUG] MCP server "plugin:interweave:interweave": Channel notifications registered
2026-10-23T04:53:52.278Z [DEBUG] MCP server "plugin:interweave:interweave": Sending SIGINT to MCP server process
2026-10-23T04:53:52.329Z [DEBUG] MCP server "plugin:interweave:interweave": MCP server process exited cleanly
```
