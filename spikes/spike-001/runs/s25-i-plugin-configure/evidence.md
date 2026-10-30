# s25-i-plugin-configure

No session, no model, no credential.

## Run

```json
{
  "run": "s25-i-plugin-configure",
  "claude_version": "2.1.286",
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
      "output": "Adding marketplace…✔ Successfully added marketplace: interweave (declared in user settings)"
    },
    {
      "command": [
        "claude",
        "plugin",
        "install",
        "interweave@interweave"
      ],
      "exit": 0,
      "output": "Installing plugin \"interweave@interweave\"...✔ Successfully installed plugin: interweave@interweave (scope: user)\n3 userConfig options not yet set — run /plugin configure interweave@interweave in Claude Code, or pass --config KEY=VALUE."
    }
  ],
  "after_install": {
    "pluginConfigs": null,
    "mcp_get_status": [
      "Status: ✔ Connected"
    ],
    "bridge_argv": [
      "--profile default --endpoint claude --delivery push --log-level info"
    ]
  },
  "steps": {
    "bare-no-tty": {
      "command": [
        "claude",
        "plugin",
        "configure",
        "interweave@interweave"
      ],
      "stdin": "/dev/null",
      "exit": 0,
      "output": "Options for interweave (interweave@interweave):\n  ❯ profile — InterWeave profile (optional, not set)\n  ❯ endpoint — Bridge endpoint (optional, not set)\n  ❯ log_level — Bridge log level (optional, not set)\n\nSet values with /plugin configure <plugin> in Claude Code, or pipe a JSON object to `claude plugin configure <plugin> --values-stdin`.",
      "pluginConfigs": null,
      "mcp_get_status": [
        "Status: ✔ Connected"
      ],
      "bridge_argv": [
        "--profile default --endpoint claude --delivery push --log-level info"
      ]
    },
    "bare-tty": {
      "command": [
        "claude",
        "plugin",
        "configure",
        "interweave@interweave"
      ],
      "stdin": "a pseudo-terminal, nothing typed",
      "screen": "Options for interweave (interweave@interweave):\n  ❯ profile — InterWeave profile (optional, not set)\n  ❯ endpoint — Bridge endpoint (optional, not set)\n  ❯ log_level — Bridge log level (optional, not set)\n\nSet values with /plugin configure <plugin> in Claude Code, or pipe a JSON object to `claude plugin configure <plugin> --values-stdin`.",
      "exit": 0,
      "pluginConfigs": null,
      "mcp_get_status": [
        "Status: ✔ Connected"
      ],
      "bridge_argv": [
        "--profile default --endpoint claude --delivery push --log-level info"
      ]
    },
    "values-stdin": {
      "command": [
        "claude",
        "plugin",
        "configure",
        "interweave@interweave",
        "--values-stdin"
      ],
      "stdin": "{\"profile\": \"work\", \"endpoint\": \"bot\"}",
      "exit": 0,
      "output": "Configuration saved. Restart Claude Code to apply it.",
      "pluginConfigs": {
        "interweave@interweave": {
          "options": {
            "profile": "work",
            "endpoint": "bot"
          }
        }
      },
      "mcp_get_status": [
        "Status: ✔ Connected"
      ],
      "bridge_argv": [
        "--profile work --endpoint bot --delivery push --log-level info"
      ]
    }
  },
  "passed": true
}
```
