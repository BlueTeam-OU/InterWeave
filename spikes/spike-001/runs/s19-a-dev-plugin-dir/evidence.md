# s19-a-dev-plugin-dir

Claude Code binary 2.1.286; versions named in the debug log: 2.1.286.

## Command

```text
claude --plugin-dir $MARKETPLACE/interweave --debug-file $RAW/debug.txt --dangerously-load-development-channels plugin:interweave@inline
```

## Setup (no session)

```text
$ claude plugin list   # exit 0
No plugins installed. Use `claude plugin install` to install a plugin.
```

## Screens answered

```json
[
  {
    "screen": "dev-warning",
    "option": "1",
    "at_s": 3.1
  }
]
```

Channel decision logged after: 10.6 s. Session wait status: 0.

Environment passed (names only): CLAUDE_CODE_OAUTH_TOKEN, CLAUDE_CONFIG_DIR, COLUMNS, DISABLE_AUTOUPDATER, HOME, LANG, LINES, LOGNAME, PATH, SHELL, TERM, TMPDIR, USER, XDG_RUNTIME_DIR.

## Debug-log lines naming a channel

```text
[DEBUG] MCP server "plugin:interweave:interweave": Channel notifications registered
```

## Screen, last lines

```text
78
────────────────────────────────────────────────────────────────────────────────
WARNING:Loadingdevelopmentchannels
--dangerously-load-development-channelsisforlocalchanneldevelopment
only.Donotusethisoptiontorunchannelsyouhavedownloadedoffthe
internet.
Pleaseuse--channelstorunalistofapprovedchannels.
Channels:plugin:interweave@inline
❯1.Iamusingthisforlocaldevelopment
2.Exit
Entertoconfirm·Esctocancel
[>0q[>4m[<u
 ▐▛███▛█Claude Codev2.1.286
▝▜██████▀Sonnet 5.5 · Claude API
 ▝▝   ▝▝ $SCRATCH/work
▎Auto mode is now Claude Code's default permission mode.
▎Auto mode lets Claude handle permission prompts automatically. Claude checks
▎each tool call for risky actions and prompt injection before executing, runs
▎the ones it assesses as lower-risk, and blocks the rest.
▎https://code.claude.com/docs/en/permission-modes
3 more notices hidden
◐ medium · /effort
────────────────────────────────────────────────────────────────────────────────
❯ Try "edit <filepath> to..."
────────────────────────────────────────────────────────────────────────────────
⏵⏵ auto mode on (shift+tab to cycle) · ← for agents[>0q
▂
▛[<u[>4m[>4m[<u78[>4m[<u78
```
