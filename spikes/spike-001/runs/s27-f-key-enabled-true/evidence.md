# s27-f-key-enabled-true

## Run

```json
{
  "run": "s27-f-key-enabled-true",
  "claude_version": "2.1.286",
  "where": "bwrap",
  "login": "key",
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
      "output": "Installing plugin \"interweave@interweave\"...\u2714 Successfully installed plugin: interweave@interweave (scope: user)\n3 userConfig options not yet set \u2014 run /plugin configure interweave@interweave in Claude Code, or pass --config KEY=VALUE."
    }
  ],
  "command": [
    "claude",
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--debug-file",
    "$RAW/debug.txt",
    "--channels",
    "plugin:interweave@interweave"
  ],
  "api_key": "sk-ant-api03-spike001-placeholder-not-a-key",
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
  "sandbox": "bwrap: the host's /etc entries read-only, /etc/claude-code a scratch directory read-only",
  "started_at": "2026-11-03T09:06:40.714Z",
  "steps": [
    {
      "channel_enable": {
        "subtype": "success",
        "request_id": "a9e1af715fc0"
      },
      "at_s": 5.0,
      "tries": 1
    }
  ],
  "exit_after_stdin_close": 0,
  "shutdown": {
    "signals_sent": [],
    "group_empty": true,
    "bwrap_wait_status": null
  },
  "channel_lines": [
    "[DEBUG] MCP server \"plugin:interweave:interweave\": Channel notifications registered"
  ],
  "policy_lines": [
    "[DEBUG] MDM settings load completed in 0ms",
    "[DEBUG] policyHelper: no helper configuration present at helper-pass time (remote managed settings eligible, no payload in cache); a payload landing later arms one only through a fetch cycle after preAction",
    "[DEBUG] [mcp-policy-cold-start] waiting on remote managed-settings load",
    "[DEBUG] Remote settings: Fetch attempt failed (http_401)",
    "[DEBUG] Remote settings: Fetch failed (http_401) and no cached settings",
    "[DEBUG] cc-plugin-sec-default@builtin seated outermost: this machine has managed settings",
    "[DEBUG] plugin cc-plugin-agents-md: no pluginConfigs[\"agents-md@builtin\" or \"cc-plugin-agents-md@builtin\"].options in user, --settings or managed settings (project settings are not read); every option is its default",
    "[DEBUG] Watching for changes in setting files $SCRATCH/config/settings.json, /etc/claude-code/managed-settings.json...",
    "[DEBUG] [3P telemetry] Waiting for remote managed settings fetch before telemetry init",
    "[DEBUG] [3P telemetry] Remote managed settings fetch settled, initializing telemetry"
  ],
  "stderr_head": [],
  "env_passed": [
    "CLAUDE_CONFIG_DIR",
    "DISABLE_AUTOUPDATER",
    "HOME",
    "LANG",
    "LOGNAME",
    "PATH",
    "SHELL",
    "TMPDIR",
    "USER",
    "XDG_RUNTIME_DIR",
    "ANTHROPIC_API_KEY"
  ],
  "passed": true
}
```

## Claude Code debug log: the run's own server only

```text
2026-11-03T09:06:42.301Z [DEBUG] MCP server "plugin:interweave:interweave": Starting connection with timeout of 30000ms
2026-11-03T09:06:42.395Z [DEBUG] MCP server "plugin:interweave:interweave": Successfully connected (transport: stdio) in 97ms
2026-11-03T09:06:42.397Z [DEBUG] MCP server "plugin:interweave:interweave": Connection established with capabilities: {"hasTools":true,"hasPrompts":false,"hasResources":false,"hasResourceSubscribe":false,"serverVersion":{"name":"interweave","version":"0.2.0"},"protocolEra":"legacy","negotiatedProtocolVersion":"2025-11-25"}
2026-11-03T09:06:45.719Z [DEBUG] MCP server "plugin:interweave:interweave": Channel notifications registered
2026-11-03T09:06:48.953Z [DEBUG] MCP server "plugin:interweave:interweave": Sending SIGINT to MCP server process
2026-11-03T09:06:48.958Z [DEBUG] MCP server "plugin:interweave:interweave": UNKNOWN connection closed after 6s (cleanly)
2026-11-03T09:06:48.959Z [DEBUG] MCP server "plugin:interweave:interweave": Cleared connection cache for reconnection
2026-11-03T09:06:49.006Z [DEBUG] MCP server "plugin:interweave:interweave": MCP server process exited cleanly
```

## Model output

```text

```
