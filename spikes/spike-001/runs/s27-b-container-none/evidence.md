# s27-b-container-none

## Run

```json
{
  "run": "s27-b-container-none",
  "claude_version": "2.1.286",
  "where": "container",
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
  "managed_settings": null,
  "image": {
    "name": "registry.fedoraproject.org/fedora:43",
    "digest": "sha256:d3fd3803b17d57bb219abebc265aaea197fb4952e650317b6a5d0abea5902ee9"
  },
  "container_run": {
    "exit": 0,
    "stderr": ""
  },
  "container_etc": [
    "1021",
    "0 755 directory /etc",
    "stat: cannot statx '/etc/claude-code': No such file or directory",
    "stat: cannot statx '/etc/claude-code/*': No such file or directory",
    "0",
    "0"
  ],
  "sandbox": "podman container of registry.fedoraproject.org/fedora:43: its own /etc, the policy file written by the container's root; the session as this login's uid (keep-id)",
  "started_at": "2026-11-03T09:20:52.516Z",
  "steps": [
    {
      "channel_enable": {
        "subtype": "error",
        "request_id": "62036ed901db",
        "error": "plugin interweave@interweave is not on the approved channels allowlist (use --dangerously-load-development-channels for local dev)"
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
  "channel_lines": [],
  "policy_lines": [
    "[DEBUG] MDM settings load completed in 0ms",
    "[DEBUG] Broken symlink or missing file encountered for settings.json at path: /etc/claude-code/managed-settings.json",
    "[DEBUG] policyHelper: no helper configuration present at helper-pass time (remote managed settings eligible, no payload in cache); a payload landing later arms one only through a fetch cycle after preAction",
    "[DEBUG] [mcp-policy-cold-start] waiting on remote managed-settings load",
    "[DEBUG] Remote settings: No settings found (404)",
    "[DEBUG] Broken symlink or missing file encountered for settings.json at path: /etc/claude-code/managed-settings.json",
    "[DEBUG] Remote settings: Saved to $SCRATCH/config/remote-settings.json",
    "[DEBUG] Remote settings: Saved empty sentinel (404 response)",
    "[DEBUG] Programmatic settings change notification for policySettings",
    "[DEBUG] Broken symlink or missing file encountered for settings.json at path: /etc/claude-code/managed-settings.json",
    "[DEBUG] cc-plugin-sec-default@builtin not seated: no managed settings and not a Team or Enterprise organization (none)",
    "[DEBUG] plugin cc-plugin-agents-md: no pluginConfigs[\"agents-md@builtin\" or \"cc-plugin-agents-md@builtin\"].options in user, --settings or managed settings (project settings are not read); every option is its default",
    "[DEBUG] Settings changed from policySettings, updating app state",
    "[DEBUG] Broken symlink or missing file encountered for settings.json at path: /etc/claude-code/managed-settings.json",
    "[DEBUG] Replacing all allow rules for destination 'policySettings' with 0 rule(s): []",
    "[DEBUG] Replacing all deny rules for destination 'policySettings' with 0 rule(s): []",
    "[DEBUG] Replacing all ask rules for destination 'policySettings' with 0 rule(s): []",
    "[DEBUG] [3P telemetry] Waiting for remote managed settings fetch before telemetry init",
    "[DEBUG] [3P telemetry] Remote managed settings fetch settled, initializing telemetry"
  ],
  "stderr_head": [],
  "env_passed": [
    "CLAUDE_CONFIG_DIR",
    "DISABLE_AUTOUPDATER",
    "HOME",
    "LANG",
    "PATH",
    "XDG_RUNTIME_DIR",
    "CLAUDE_CODE_OAUTH_TOKEN"
  ],
  "container_removed": {
    "exit": 0,
    "exists_after": false
  },
  "passed": true
}
```

## Claude Code debug log: the run's own server only

```text
2026-11-03T09:20:53.658Z [DEBUG] MCP server "plugin:interweave:interweave": Starting connection with timeout of 30000ms
2026-11-03T09:20:53.681Z [DEBUG] MCP server "plugin:interweave:interweave": Successfully connected (transport: stdio) in 26ms
2026-11-03T09:20:53.681Z [DEBUG] MCP server "plugin:interweave:interweave": Connection established with capabilities: {"hasTools":true,"hasPrompts":false,"hasResources":false,"hasResourceSubscribe":false,"serverVersion":{"name":"interweave","version":"0.2.0"},"protocolEra":"legacy","negotiatedProtocolVersion":"2025-11-25"}
2026-11-03T09:21:00.725Z [DEBUG] MCP server "plugin:interweave:interweave": Sending SIGINT to MCP server process
2026-11-03T09:21:00.726Z [DEBUG] MCP server "plugin:interweave:interweave": UNKNOWN connection closed after 7s (cleanly)
2026-11-03T09:21:00.727Z [DEBUG] MCP server "plugin:interweave:interweave": Cleared connection cache for reconnection
2026-11-03T09:21:00.775Z [DEBUG] MCP server "plugin:interweave:interweave": MCP server process exited cleanly
```

## Model output

```text

```
