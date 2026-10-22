# s20-plugin-config

Claude Code 2.1.286, 2026-10-22, run by hand from this login's shell with no Claude Code session: no model, no channel flag, no credential. `CLAUDE_CONFIG_DIR` was a scratch directory, with `DISABLE_AUTOUPDATER=1`. `$SCRATCH` stands for that scratch tree, which was removed afterwards.

The plugin was the bridge's own package, assembled by `packaging/claude-plugin/build.sh` from the release `claude-channel`. Two things were changed in the scratch copy only:

- `plugin.json` gained `userConfig` with `profile` (`default: "default"`) and `endpoint` (`default: "claude"`).
- `.mcp.json`'s `args` became `["--profile", "${user_config.profile}", "--endpoint", "${user_config.endpoint}", "--delivery", "push"]`.

The plugin's `bin/claude-channel` was a wrapper script. It appends a marker, `$CLAUDE_PLUGIN_ROOT` and its own arguments to `$SCRATCH/argv.log`, then runs the real bridge. `claude mcp get plugin:interweave:interweave` starts the server to check it ("Connected"), so the log shows the argv Claude Code passed.

## Configuration

```text
$ claude plugin validate --strict $SCRATCH/mp/interweave
✔ Validation passed
$ claude plugin marketplace add $SCRATCH/mp
✔ Successfully added marketplace: interweave (declared in user settings)
$ claude plugin install interweave@interweave            # no --config, stdin not a terminal
✔ Successfully installed plugin: interweave@interweave (scope: user)
2 userConfig options not yet set — run /plugin configure interweave@interweave in Claude Code, or pass --config KEY=VALUE.
$ claude mcp get plugin:interweave:interweave            # argv.log:
--profile default --endpoint claude --delivery push
$ claude plugin uninstall interweave@interweave
$ claude plugin install interweave@interweave --config profile=work --config endpoint=bot
✔ Successfully installed plugin: interweave@interweave (scope: user)
$ claude mcp get plugin:interweave:interweave            # argv.log:
--profile work --endpoint bot --delivery push
```

After the second install, the scratch `settings.json` held `"pluginConfigs": {"interweave@interweave": {"options": {"profile": "work", "endpoint": "bot"}}}`.

## Versions, with a directory marketplace

`installed_plugins.json` recorded the plugin's `installPath` as `$SCRATCH/cfg/plugins/cache/interweave/interweave/0.0.0`, keyed by version.

```text
# A: the marketplace's wrapper rebuilt (marker build-2), version still 0.0.0
$ claude plugin marketplace update interweave
✔ Successfully updated marketplace: interweave
$ claude plugin update interweave@interweave
✔ interweave is already at the latest version (0.0.0).
$ claude mcp get plugin:interweave:interweave            # argv.log:
build-2 --profile work --endpoint bot --delivery push
# the cache's 0.0.0 copy still held the first wrapper, with no marker

# B: the marketplace's plugin.json at version 0.0.1 (marker build-3)
$ claude plugin marketplace update interweave
$ claude plugin update interweave@interweave
✔ Plugin "interweave" updated from 0.0.0 to 0.0.1 for scope user. Restart to apply changes.
$ claude mcp get plugin:interweave:interweave            # argv.log:
build-3 --profile work --endpoint bot --delivery push
# the cache held 0.0.0 and 0.0.1; installed_plugins.json named 0.0.1; the options were kept

# C: the marketplace's wrapper changed again (marker build-4), with no update of any kind
$ claude mcp get plugin:interweave:interweave            # argv.log:
build-4 root=$SCRATCH/mp/interweave --profile work --endpoint bot --delivery push
# the cache's 0.0.1 copy still held build-3
```
