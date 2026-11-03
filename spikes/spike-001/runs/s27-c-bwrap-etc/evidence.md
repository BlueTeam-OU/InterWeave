# s27-c-bwrap-etc

## Run

```json
{
  "run": "s27-c-bwrap-etc",
  "claude_version": "2.1.286",
  "where": "bwrap",
  "login": "token",
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
  "started_at": "2026-11-03T09:06:04.748Z",
  "steps": [
    {
      "channel_enable": {
        "subtype": "success",
        "request_id": "bb4705b4b1f5"
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
    "[DEBUG] Remote settings: No settings found (404)",
    "[DEBUG] Remote settings: Saved to $SCRATCH/config/remote-settings.json",
    "[DEBUG] Remote settings: Saved empty sentinel (404 response)",
    "[DEBUG] Programmatic settings change notification for policySettings",
    "[DEBUG] cc-plugin-sec-default@builtin seated outermost: this machine has managed settings",
    "[DEBUG] plugin cc-plugin-agents-md: no pluginConfigs[\"agents-md@builtin\" or \"cc-plugin-agents-md@builtin\"].options in user, --settings or managed settings (project settings are not read); every option is its default",
    "[DEBUG] Settings changed from policySettings, updating app state",
    "[DEBUG] Replacing all allow rules for destination 'policySettings' with 0 rule(s): []",
    "[DEBUG] Replacing all deny rules for destination 'policySettings' with 0 rule(s): []",
    "[DEBUG] Replacing all ask rules for destination 'policySettings' with 0 rule(s): []",
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
    "CLAUDE_CODE_OAUTH_TOKEN"
  ],
  "passed": true
}
```

## Claude Code debug log: the run's own server only

```text
2026-11-03T09:06:06.146Z [DEBUG] MCP server "plugin:interweave:interweave": Starting connection with timeout of 30000ms
2026-11-03T09:06:06.212Z [DEBUG] MCP server "plugin:interweave:interweave": Successfully connected (transport: stdio) in 67ms
2026-11-03T09:06:06.214Z [DEBUG] MCP server "plugin:interweave:interweave": Connection established with capabilities: {"hasTools":true,"hasPrompts":false,"hasResources":false,"hasResourceSubscribe":false,"serverVersion":{"name":"interweave","version":"0.2.0"},"protocolEra":"legacy","negotiatedProtocolVersion":"2025-11-25"}
2026-11-03T09:06:09.753Z [DEBUG] MCP server "plugin:interweave:interweave": Channel notifications registered
2026-11-03T09:06:12.955Z [DEBUG] MCP server "plugin:interweave:interweave": Sending SIGINT to MCP server process
2026-11-03T09:06:12.958Z [DEBUG] MCP server "plugin:interweave:interweave": UNKNOWN connection closed after 6s (cleanly)
2026-11-03T09:06:12.959Z [DEBUG] MCP server "plugin:interweave:interweave": Cleared connection cache for reconnection
2026-11-03T09:06:13.006Z [DEBUG] MCP server "plugin:interweave:interweave": MCP server process exited cleanly
```

## Model output

```text

```
