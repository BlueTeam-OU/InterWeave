# s19-c-channels

Claude Code version, as the session's debug log names it: 2.1.286.

## Command

```text
claude --debug-file $RAW/debug.txt --channels plugin:interweave@interweave
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

## Screens answered

(none)

Channel decision logged after: 7.5 s. Session wait status: 0.

Environment passed (names only): CLAUDE_CODE_OAUTH_TOKEN, CLAUDE_CONFIG_DIR, COLUMNS, DISABLE_AUTOUPDATER, HOME, LANG, LINES, LOGNAME, PATH, SHELL, TERM, TMPDIR, USER, XDG_RUNTIME_DIR.

## Debug-log lines naming a channel

```text
[DEBUG] MCP server "plugin:interweave:interweave": Channel notifications skipped: plugin interweave@interweave is not on the approved channels allowlist (use --dangerously-load-development-channels for local dev)
```

## Screen, last lines

```text
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
❯ Try "how does <filepath> work?"
────────────────────────────────────────────────────────────────────────────────
⏵⏵ auto mode on (shift+tab to cycle) · ← for agents
 ▐▛▛█
·▜██████·
~~
▗▟▛███▛█▄
▜███▘
 ▝▝   ▝▝
 ▐
▝▀
 ▐▛▛█
·▜██████·
~~
▗▟▛███▛█▄
▜███▘
 ▝▝   ▝▝
 ▐
▝▀
```
