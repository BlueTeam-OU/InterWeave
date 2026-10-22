# s19-d-no-flag

Claude Code binary 2.1.286; versions named in the debug log: 2.1.286.

## Command

```text
claude --debug-file $RAW/debug.txt
```

## Setup (no session)

```text
$ claude plugin marketplace add $SCRATCH/marketplace   # exit 0
Adding marketplace…✔ Successfully added marketplace: interweave (declared in user settings)
```

```text
$ claude plugin install interweave@interweave   # exit 0
Installing plugin "interweave@interweave"...✔ Successfully installed plugin: interweave@interweave (scope: user)
```

```text
$ claude plugin list   # exit 0
Installed plugins:

  ❯ interweave@interweave
    Version: 0.0.0
    Scope: user
    Status: ✔ enabled
```

## Screens answered

```json
[]
```

Channel decision logged after: 9.2 s. Session wait status: 0.

Environment passed (names only): CLAUDE_CODE_OAUTH_TOKEN, CLAUDE_CONFIG_DIR, COLUMNS, DISABLE_AUTOUPDATER, HOME, LANG, LINES, LOGNAME, PATH, SHELL, TERM, TMPDIR, USER, XDG_RUNTIME_DIR.

## Debug-log lines naming a channel

```text
[DEBUG] MCP server "plugin:interweave:interweave": Channel notifications skipped: server plugin:interweave:interweave not in --channels list for this session
```

## Screen, last lines

```text
78
 ▐▛███▛█Claude Codev2.1.286
▝▜██████▀Sonnet 5.5 · Claude API
 ▝▝   ▝▝ $SCRATCH/work
▎Auto mode is now Claude Code's default permission mode.
▎Auto mode lets Claude handle permission prompts automatically. Claude checks
▎each tool call for risky actions and prompt injection before executing, runs
▎the ones it assesses as lower-risk, and blocks the rest.
▎https://code.claude.com/docs/en/permission-modes
2 more notices hidden
◐ medium · /effort
────────────────────────────────────────────────────────────────────────────────
❯ Try "how do I log an error?"
────────────────────────────────────────────────────────────────────────────────
⏵⏵ auto mode on (shift+tab to cycle) · ← for agents[>0q
  ▝▝
▝▝
  ▝▝
▝▝
  ▝▝
▗▟▄
 ▘
▝▝
 ▐
▝▀[<u[>4m[>4m[<u78[>4m[<u78
```
