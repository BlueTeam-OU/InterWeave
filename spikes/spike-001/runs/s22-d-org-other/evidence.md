# s22-d-org-other

## Run

```json
{
  "run": "s22-d-org-other",
  "managed_settings": {
    "path": "/etc/claude-code/managed-settings.d/interweave.json",
    "content": {
      "channelsEnabled": true,
      "allowedChannelPlugins": [
        {
          "plugin": "other",
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
      "channel_decision_at_s": 3.8,
      "registered": false
    }
  ],
  "screen_prompt": null,
  "shutdown": {
    "signals_sent": [
      "SIGINT to the session"
    ],
    "group_empty": true,
    "bwrap_wait_status": 0
  },
  "versions_in_debug_log": [
    "2.1.286"
  ],
  "channel_decision": "2026-10-23T05:34:52.634Z [DEBUG] MCP server \"plugin:interweave:interweave\": Channel notifications skipped: plugin interweave@interweave is not on your org's approved channels list (set allowedChannelPlugins in managed settings)",
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
2026-10-23T05:34:52.025Z [DEBUG] MCP server "plugin:interweave:interweave": Starting connection with timeout of 30000ms
2026-10-23T05:34:52.598Z [DEBUG] MCP server "plugin:interweave:interweave": Successfully connected (transport: stdio) in 575ms
2026-10-23T05:34:52.599Z [DEBUG] MCP server "plugin:interweave:interweave": Connection established with capabilities: {"hasTools":true,"hasPrompts":false,"hasResources":false,"hasResourceSubscribe":false,"serverVersion":{"name":"interweave","version":"0.0.0"},"protocolEra":"legacy","negotiatedProtocolVersion":"2025-11-25"}
2026-10-23T05:34:52.634Z [DEBUG] MCP server "plugin:interweave:interweave": Channel notifications skipped: plugin interweave@interweave is not on your org's approved channels list (set allowedChannelPlugins in managed settings)
2026-10-23T05:34:56.628Z [DEBUG] MCP server "plugin:interweave:interweave": Sending SIGINT to MCP server process
2026-10-23T05:34:56.682Z [DEBUG] MCP server "plugin:interweave:interweave": MCP server process exited cleanly
```
