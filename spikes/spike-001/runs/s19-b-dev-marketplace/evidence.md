# s19-b-dev-marketplace

Claude Code binary 2.1.286; versions named in the debug log: 2.1.286.

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
[
  {
    "screen": "dev-warning",
    "option": "1",
    "at_s": 3.1
  }
]
```

Channel decision logged after: 10.2 s. Session wait status: 0.

Environment passed (names only): CLAUDE_CODE_OAUTH_TOKEN, CLAUDE_CONFIG_DIR, COLUMNS, DISABLE_AUTOUPDATER, HOME, LANG, LINES, LOGNAME, PATH, SHELL, TERM, TMPDIR, USER, XDG_RUNTIME_DIR.

## Debug-log lines naming a channel

```text
[DEBUG] MCP server "plugin:interweave:interweave": Channel notifications registered
```

## Screen, last lines

```text
▐█
▐
▝▝
█▌
█▌
▘ ▘
▐█
▐█
▝▝▝▝
█▌
▝█▛
▘▘ ▘▘
▐█
▝▜█▀
▝▝ ▝▝
 ▌
 ▝▛
 ▘▘▘▘
▐
 ▐
▝▝▝▝
 ▌
 ▌
 ▘ ▘
▐
▐
▝▝
▛▌
█▌
▘ ▘
▐▜▛█
▐█
▝▝▝▝
▛██▛▌
▝█▛
▘▘ ▘▘
▐█▜█
▝▜█▀
▝▝ ▝▝
▛ [<u[>4m[>4m[<u78[>4m[<u78
```
