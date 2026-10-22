# s21-refused-start

Claude Code 2.1.286, 2026-10-22. The run was by hand, with no Claude Code session: no model, no channel flag, no credential. `HOME` and `CLAUDE_CONFIG_DIR` were scratch directories (written `$SCRATCH` below, removed afterwards), and `DISABLE_AUTOUPDATER=1` was set. Every `XDG_*` variable was unset, so Claude Code started the bridge with no `XDG_RUNTIME_DIR`.

The plugin was `packaging/claude-plugin/build.sh`'s marketplace of the release `claude-channel`, installed at user scope with no `--config`. `claude mcp get plugin:interweave:interweave` started the server. The lines below are what Claude Code wrote to `$SCRATCH/home/.cache/claude-cli-nodejs/<working directory>/mcp-logs-plugin-interweave-interweave/<timestamp>.jsonl`, timestamps and session ids removed.

## The bridge before 403295e7: a bad start exits

```text
Status: ✘ Failed to connect
{"debug":"Starting connection with timeout of 30000ms"}
{"error":"Server stderr: claude-channel: the profile's paths: XDG_RUNTIME_DIR is unset; there is no safe substitute for the IPC socket directory\n"}
{"debug":"Connection failed after 26ms (CONNECTION_CLOSED): Connection closed"}
{"error":"Connection failed (CONNECTION_CLOSED): Connection closed"}
```

## The bridge at 403295e7: the refusal is served

```text
Status: ✔ Connected
{"debug":"Starting connection with timeout of 30000ms"}
{"error":"Server stderr: claude-channel: the profile's paths: XDG_RUNTIME_DIR is unset; there is no safe substitute for the IPC socket directory; serving the refusal\n"}
{"debug":"Successfully connected (transport: stdio) in 73ms"}
{"debug":"Connection established with capabilities: {\"hasTools\":true,\"hasPrompts\":false,\"hasResources\":false,\"hasResourceSubscribe\":false,\"serverVersion\":{\"name\":\"interweave\",\"version\":\"0.0.0\"},\"protocolEra\":\"legacy\", ...}"}
{"debug":"Sending SIGINT to MCP server process"}
{"debug":"UNKNOWN connection closed after 0s (cleanly)"}
{"debug":"MCP server process exited cleanly"}
```

The bridge's stderr reaches that log as `Server stderr:` lines, whether the server connects or not.
