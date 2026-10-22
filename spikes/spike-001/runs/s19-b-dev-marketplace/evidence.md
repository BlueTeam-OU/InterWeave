# s19-b-dev-marketplace

Claude Code version, as the session's debug log names it: 2.1.286.

## Command

```text
claude --debug-file $RAW/debug.txt --dangerously-load-development-channels plugin:interweave@interweave
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

dev-warning, answered with option 1 at 3.0 s:

```text
WARNING:Loadingdevelopmentchannels
--dangerously-load-development-channelsisforlocalchanneldevelopment
only.Donotusethisoptiontorunchannelsyouhavedownloadedoffthe
internet.
Pleaseuse--channelstorunalistofapprovedchannels.
Channels:plugin:interweave@interweave
❯1.Iamusingthisforlocaldevelopment
2.Exit
Entertoconfirm·Esctocancel
```

Channel decision logged after: 10.2 s. Session wait status: 0.

Environment passed (names only): CLAUDE_CODE_OAUTH_TOKEN, CLAUDE_CONFIG_DIR, COLUMNS, DISABLE_AUTOUPDATER, HOME, LANG, LINES, LOGNAME, PATH, SHELL, TERM, TMPDIR, USER, XDG_RUNTIME_DIR.

## Debug-log lines naming a channel

```text
[DEBUG] MCP server "plugin:interweave:interweave": Channel notifications registered
```

## Screen, last lines

```text
────────────────────────────────────────────────────────────────────────────────
WARNING:Loadingdevelopmentchannels
--dangerously-load-development-channelsisforlocalchanneldevelopment
only.Donotusethisoptiontorunchannelsyouhavedownloadedoffthe
internet.
Pleaseuse--channelstorunalistofapprovedchannels.
Channels:plugin:interweave@interweave
❯1.Iamusingthisforlocaldevelopment
2.Exit
Entertoconfirm·Esctocancel
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
❯ Try "fix typecheck errors"
────────────────────────────────────────────────────────────────────────────────
⏵⏵ auto mode on (shift+tab to cycle) · ← for agents
 ▐▂▂█
·▜██████·
~~
 ▐▟███▟█
▝▜███▀
 ▝▝   ▝▝
▛▛
▂▂
▛▛
```
