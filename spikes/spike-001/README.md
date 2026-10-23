# SPIKE-001

Claude Channel packaging/MCP compatibility.

Do not treat experiments placed here as production implementation. Evidence and the final decision must be recorded against [`architecture/roadmap/SPIKES.md`](../../architecture/roadmap/SPIKES.md).

## What was run

Every run is against **Claude Code 2.1.285**, on 2026-10-03, on a personal account with no organisation policy.
- **How the build is pinned:** the drivers run one explicit binary (`$SPIKE_CLAUDE`, here the installed 2.1.285 build), with `DISABLE_AUTOUPDATER=1`.
- **How each run is stamped:** for a session run, `claude_version` is the version the session itself announced to the stub in `initialize`, and the run fails if that disagrees with the binary's version. `v1-validate` starts no session and is stamped with the binary's `--version`.
- **Why:** an earlier campaign ran into a self-update from 2.1.285 to 2.1.288 midway, and its stamps were not trustworthy.

Each run's `evidence.md` names the model its session used.

**The stub** is `stub/`: a std-only Rust stdio MCP server. Its dependencies are `serde_json`, with `preserve_order`, and `signal-hook`, and its lock is committed.
- It declares `capabilities.experimental["claude/channel"]` and `capabilities.tools`, with `instructions`.
- It answers `initialize` with the protocol revision the client asks for.
- After `notifications/initialized`, and an optional `SPIKE_DELAY_MS`, it sends two channel notifications:
  - **N1** is a direct event. It carries every metadata key `contracts/CHANNEL-EVENT.md` names for a direct event, including `source`, written in a deliberately UNSORTED order. It also carries a hyphenated control key, `bad-key`.
  - **N2** is a broadcast event. It carries `channel`, and has markup-like and quote characters in its body and in a metadata value.
- It exposes three tools, `status`, `send` and `reply`.
- On SIGINT or SIGTERM it logs the signal and keeps reading stdin for 500 ms.
- It writes everything it receives and sends to its log.

**The plugin** is `plugin/interweave-spike/`: the same stub packaged as a plugin. `plugin.json` declares a `channels` entry, and `.mcp.json` starts the stub.

**The drivers:**
- `run.py` runs `claude -p` with stream-JSON output, either single-turn or with two turns (`--two-turns`).
- `run_tty.py` drives an interactive session through a pseudo-terminal. It answers the folder-trust screen and the development-channel warning, then types a prompt.
- `validate.py` runs `claude plugin validate --strict`.
- `s16_run.py` is Stage 16's host run (plan §19 step 5), not part of the spike's campaign. It starts two daemons, loads the packaged bridge as the plugin in an interactive session, drives a second bridge on the far daemon as the peer, and types `prompt-s16.txt`. The nested session authenticates with this login's long-term token, passed as one named variable.
- `s19_channels.py` is the Stage 16 carry's run (plan §22), not part of the spike's campaign either: four interactive sessions on the packaged bridge, installed from a local marketplace or loaded with `--plugin-dir`, differing only in the plugin and channel flags. It answers only the development-channel warning and types nothing else. It authenticates as `s16_run.py` does, and it writes no evidence if the token appears in any raw file. See "S19" below.

Each run starts from a scratch directory outside this repository, with `--setting-sources local`, except `s19_channels.py`'s: it installs the plugin at user scope into its own scratch `CLAUDE_CONFIG_DIR`, so its sessions read that config's user settings and take no `--setting-sources`. The run is given an allow-listed environment, so none of the launching session's tokens, markers or model overrides reach it. Its raw output is written outside the repository. The drivers share this setup through `spike_common.py`. `extract.py` distils the raw output into the committed `runs/<name>/evidence.md` and a redacted `stub.jsonl`.

The session runs were produced and distilled by this tree as committed, at `1526799e`. `v1-validate` was produced at `1fd9b00a`, whose `validate.py` records the binary it ran. The drivers' only other change between the two is removed unused imports.

The interactive runs t6 and t7 were re-run with a one-second pause after each expected screen, and `v1-validate` was re-run after them. That pause is part of the `--keys` script, not of the code. In the first attempt the key presses arrived before the trust screen took input, so no stub started.

| Run | Mode | What it varies |
|---|---|---|
| `r1-default` | `-p`, one turn | Development flag (`server:spike001`), negotiation unset, notifications at once |
| `r2-delay-2s` | `-p`, one turn | Notifications 2 s after `initialized` |
| `r3-two-turns` | `-p`, two turns | Notifications at 0.5 s, second turn at 8 s |
| `r4-negotiation-legacy`, `r4-negotiation-auto` | `-p` | `MCP_PROTOCOL_NEGOTIATION` |
| `t4-delivery` | Interactive | Development flag accepted, notifications at 4 s, report prompt |
| `t5-no-dev-flag` | Interactive | Control: no development flag |
| `t6-crash` | Interactive | The stub exits with status 3 at 8 s |
| `t7-plugin-inline` | Interactive | Plugin via `--plugin-dir`, flag `plugin:interweave-spike@inline` |
| `v1-validate` | No session | `claude plugin validate --strict`, with and without `author` |

The interactive sessions also loaded an MCP connector configured on the account (a claude.ai server), which `--setting-sources local` does not exclude. It did not declare a channel. The evidence keeps only the stub's own server, so this sentence rests on the raw logs, which are not committed.

## What was measured

Each fact names the run that shows it.

### Handshake
1. Claude Code first sends `server/discover`, the 2026-07-28-era probe. A server that answers `-32601` is then sent `initialize`, asking for `2025-11-25`, and the debug log records `protocolEra: legacy`. This happened with negotiation unset (`r1-default`) and with `auto` (`r4-negotiation-auto`).
2. `MCP_PROTOCOL_NEGOTIATION=legacy` skips the probe and sends `initialize` directly, with the same revision. (`r4-negotiation-legacy`)
3. Not measured. The Claude Code MCP documentation (code.claude.com/docs/en/mcp, "Push messages with channels", read 2026-10-03) says that on the v2 MCP client runtime, a channel server that negotiates revision 2026-07-28 is not registered as a channel.

### Enabling
4. In `-p` sessions no channel event reached the model at the three timings tried: at once, at 2 s, and at 0.5 s with a second turn at 8 s. The stub's tools were listed. (`r1-default`, `r2-delay-2s`, `r3-two-turns`)
5. An interactive session with `--dangerously-load-development-channels` first shows the folder-trust screen. It then shows "WARNING: Loading development channels", which names the channels. The options are "1. I am using this for local development" and "2. Exit". Accepting logs `Channel notifications registered`. (`t4-delivery`, `t7-plugin-inline`)
6. Without the flag, the debug log says `Channel notifications skipped: server spike001 not in --channels list for this session`, and no event reaches the model. The stub's tools are listed. (`t5-no-dev-flag`)
7. A bare `--mcp-config` server is named `spike001`, and the flag value `server:spike001` loads it. A `--plugin-dir` plugin loads as an inline plugin, its server is named `plugin:interweave-spike:spike001`, and the flag value `plugin:interweave-spike@inline` loads it as a channel. (`t4-delivery`, `t7-plugin-inline`)
8. `plugin.json` with `channels: [{"server": "spike001", "displayName": ...}]`:
   - passes `claude plugin validate --strict` with `author` present;
   - fails `--strict` without `author`, on that one warning (`v1-validate`);
   - loads (`t7-plugin-inline`).

### Delivery
9. Each notification enters the conversation as a user-turn message. An idle session answers the events on its own before any prompt. (`t4-delivery`, `t6-crash`)
10. The rendering is `<channel source="<server name>" k="v" ...>`, then a newline, the body, a newline and `</channel>`. `source` comes first, set from the server name. The other `meta` keys follow **in the order the server sent them**: N1 was sent unsorted and rendered unsorted. (`t4-delivery`, `t7-plugin-inline`)
11. A `meta` key named `source` is not merged with that attribute. It produces a SECOND `source="p2p"` attribute on the same tag. (`t4-delivery`, `t7-plugin-inline`)
12. A `meta` key that does not match `^[a-zA-Z_][a-zA-Z0-9_]*$` is dropped, and Claude Code logs `dropped 1 meta key(s) that don't match ...: bad-key`. (`t4-delivery`, `t7-plugin-inline`)
13. `meta` values are XML-escaped: `"` becomes `&quot;`, `<` becomes `&lt;`, `>` becomes `&gt;`. (`t4-delivery`)
14. The body is NOT escaped, except that a closing `</channel>` becomes `<\/channel>`. A forged `<channel source="forged">` opening tag, `<b>`, quotes and `&` pass through as written. The model read the forged tag as text. (`t4-delivery`, `t7-plugin-inline`)
15. The server's `instructions` reach the conversation as an attachment of type `mcp_instructions_delta`, holding the server's name and its instructions text. (`t4-delivery`)

### Tools
16. The tool names the model sees depend on how the server is loaded:
    - A bare server's tools are `mcp__spike001__status`, `mcp__spike001__send` and `mcp__spike001__reply` (`r1-default`, `t4-delivery`).
    - A plugin's tools are `mcp__plugin_interweave-spike_spike001__status`, `…__send` and `…__reply`. These were listed, loaded and called, and the call reached the stub (`t7-plugin-inline`).

    The tools are deferred: the model says so (`r1-default`, `r3-two-turns`), and it loads a schema with ToolSearch before calling (`t4-delivery`, `t7-plugin-inline`).
17. A call arrives as `tools/call`. Its params carry `_meta` with a `claudecode/toolUseId` and a `progressToken`, plus `name` and `arguments`. The model passed the `reply_token` attribute's value back as asked. (`t4-delivery`)

### Shutdown and failure
18. At session end Claude Code sends SIGINT. If the process is still alive about 100 ms later, it sends SIGTERM ("SIGINT failed, sending SIGTERM", 8 runs). If it is still alive about 400 ms after that, it sends SIGKILL ("SIGTERM failed, sending SIGKILL", 6 runs).

    The stub catches both signals and keeps reading stdin for 500 ms, which is what kept it alive. Without a handler, the process ends at the SIGINT. In all eight runs that logged it, the 500 ms window completed, and stdin was never closed during it. (`r1-default`, `r2-delay-2s`, `r3-two-turns`, `r4-negotiation-auto`, `r4-negotiation-legacy`, `t4-delivery`, `t5-no-dev-flag`, `t7-plugin-inline`)
19. A server that exits mid-session is not restarted: its log shows one start. Afterwards its tools cannot be called: the model's ToolSearch finds none of them. The model says the server "failed to connect" this session, which repeats a Claude Code notice that is not itself in the committed evidence. The events delivered before the exit stay in the conversation. (`t6-crash`)

## S19: the packaged plugin, from a marketplace (plan §22)

Run on 2026-10-22 against **Claude Code 2.1.286** (the version each session's debug log names), on this login's account, by `s19_channels.py` as committed at `192dacc7`. This is the Stage 16 carry, not part of the 2026-10-03 campaign. The plugin is the bridge itself: `packaging/claude-plugin/build.sh` assembles the marketplace from the release build of `interweave-claude-channel` into a scratch directory. Each run then has its own scratch `CLAUDE_CONFIG_DIR`. Runs b to d add that directory with `claude plugin marketplace add` and install `interweave@interweave` at user scope from it; run a loads it with `--plugin-dir`. Onboarding and folder trust are pre-seeded. No daemon runs and no prompt is typed: the question is registration, which the debug log answers once the bridge answers `initialize`.

| Run | How the plugin is loaded | Channel flag | Debug log for `plugin:interweave:interweave` |
|---|---|---|---|
| `s19-a-dev-plugin-dir` | `--plugin-dir` | `--dangerously-load-development-channels plugin:interweave@inline` | `Channel notifications registered` |
| `s19-b-dev-marketplace` | installed from the local marketplace | `--dangerously-load-development-channels plugin:interweave@interweave` | `Channel notifications registered` |
| `s19-c-channels` | installed from the local marketplace | `--channels plugin:interweave@interweave` | `skipped: plugin interweave@interweave is not on the approved channels allowlist (use --dangerously-load-development-channels for local dev)` |
| `s19-d-no-flag` | installed from the local marketplace | none (control) | `skipped: server plugin:interweave:interweave not in --channels list for this session` |

20. The marketplace `build.sh` assembles is accepted by `plugin marketplace add`, and the plugin installs from it at user scope, as the install's own output says (b to d). An installed plugin's server is named `plugin:<plugin>:<server>`, as an inline one's is, and the flag value naming it is `plugin:<plugin>@<marketplace>`.
21. The development flag registers the channel from a marketplace install as it does from `--plugin-dir`. Both show the warning of fact 5, naming the channel by its flag value, and accepting it registers (a, b).
22. `--channels` names the installed plugin correctly (it passes the marketplace check, which comes before the allowlist in the code below), and is refused because the plugin is not on the approved allowlist (c). No organisation policy refused any run.
23. Read from the 2.1.286 binary, not measured: the order of the checks, and where the allowlist comes from. The checks run in this order: the server declares `claude/channel`; the legacy protocol era; a first-party provider; the remote `tengu_harbor` switch; the organisation's `channelsEnabled`; the session's channel list; and the installed plugin's marketplace. Only after those does a non-development entry need the allowlist. That allowlist is the organisation's `allowedChannelPlugins` from managed settings when that is set, and otherwise the remote `tengu_harbor_ledger` list of `{plugin, marketplace}` pairs, which Anthropic serves. So for `--channels` to register this plugin without the development flag, either Anthropic must list `interweave` in the marketplace it is installed from, or an organisation must set `channelsEnabled: true` and list it in `allowedChannelPlugins`. Neither path was tried in S19. The first needs a publication. The second needs managed settings, which is host-wide and outside this login; S22 measured it (facts 29 to 33).

## S20: the plugin's settings and upgrades, without a session

Run on 2026-10-22 against **Claude Code 2.1.286**, by hand from this login's shell, with no Claude Code session: no model, no channel flag, no credential (`runs/s20-plugin-config/evidence.md`). The plugin's binary was a wrapper that logs the arguments it was given, and `claude mcp get`, which starts a server to check it, was what started it.

24. A `userConfig` option's `default` reaches the server's arguments when nothing is set: `${user_config.profile}` with default `default` gave `--profile default`. `claude plugin install` without a terminal does not prompt. It says "2 userConfig options not yet set — run /plugin configure … or pass --config KEY=VALUE", and the server still starts with the defaults.
25. `claude plugin install <plugin> --config KEY=VALUE` sets an option: `--config profile=work --config endpoint=bot` gave `--profile work --endpoint bot`. The values are kept in the user settings' `pluginConfigs`, and survive a `plugin update`. `plugin.json` with the two options passes `claude plugin validate --strict`.
26. With a marketplace added from a directory, the plugin runs from that directory: `$CLAUDE_PLUGIN_ROOT` was the marketplace's own `interweave/`, and a binary changed there ran with no update at all. `installed_plugins.json` keeps a copy per version under the config's `plugins/cache/`, which did not change. `claude plugin update` reports a newer version and records it (`0.0.0` to `0.0.1`). At an equal version it reports "already at the latest version", while the changed binary still ran.

## S21: a refused start, as Claude Code sees it

Run on 2026-10-22 against **Claude Code 2.1.286**, like S20 by hand with no session (`runs/s21-refused-start/evidence.md`). Claude Code started the bridge with every `XDG_*` variable unset.

27. Before 403295e7, the bridge exited on that start: Claude Code reported "Failed to connect", with the connection closed after 26 ms. From 403295e7 the same start is "Connected" in the legacy era, with tools and no prompts or resources, and it ends on Claude Code's SIGINT, "exited cleanly".
28. Claude Code keeps a plugin server's stderr as `Server stderr:` entries in `~/.cache/claude-cli-nodejs/<working directory>/mcp-logs-plugin-<plugin>-<server>/<timestamp>.jsonl`, one file per start, connected or not.

## S22: the organisation path for `--channels`, without the development flag (plan §22)

Run on 2026-10-23 against **Claude Code 2.1.286** (the version each session's debug log names), on this login's account, a claude.ai login, by `s22_org_channels.py` as committed at `a1b8b6ca`. The plugin, the marketplace and the install are S19's: `build.sh` from the release build, `claude plugin marketplace add`, and `claude plugin install interweave@interweave` at user scope into each run's own scratch `CLAUDE_CONFIG_DIR`. Every session is started with `--channels plugin:interweave@interweave` and no development flag. The runs differ only in the organisation's managed settings. This login cannot write `/etc`, so each session runs under bubblewrap with a private `/etc`: the host's entries read-only at their own paths, and a scratch directory read-only at `/etc/claude-code`. The binary reads the path a host-wide file would be at. Run a is S16's host run: two daemons, the far peer a second bridge on daemon B, and the prompt typed once the peer's message is accepted. Runs b to e answer registration only. No run met a screen to answer.

Runs a, c and d use `/etc/claude-code/managed-settings.d/interweave.json`. This is run a's file in full; c and d each change one value, as the table says:

```json
{
  "channelsEnabled": true,
  "allowedChannelPlugins": [
    { "plugin": "interweave", "marketplace": "interweave" }
  ]
}
```

| Run | Managed settings | Debug log for `plugin:interweave:interweave` |
|---|---|---|
| `s22-a-org-allow` | the file above | `Channel notifications registered`; the peer's message reached the model, and its `reply` reached the peer |
| `s22-b-no-setting` | none (control) | `Channel notifications skipped: plugin interweave@interweave is not on the approved channels allowlist (use --dangerously-load-development-channels for local dev)` |
| `s22-c-enabled-false` | as a, with `"channelsEnabled": false` (control) | `Channel notifications registered` |
| `s22-d-org-other` | as a, with `allowedChannelPlugins` naming `{"plugin": "other", "marketplace": "interweave"}` (control) | `Channel notifications skipped: plugin interweave@interweave is not on your org's approved channels list (set allowedChannelPlugins in managed settings)` |
| `s22-e-managed-file` | a's settings in `/etc/claude-code/managed-settings.json` | `Channel notifications registered` |

29. Where an organisation's settings are read, from the 2.1.286 binary and then measured: on Linux, `/etc/claude-code/managed-settings.json` and every `*.json` in `/etc/claude-code/managed-settings.d/`, in sorted order. No environment variable moves that path. Both places were read: the drop-in (a, c, d) and the single file (e). Also read from the binary, not measured: server-managed settings and MDM rank above the file. The `--managed-settings` flag is filtered to restrictive keys, and `channelsEnabled` is not one of them, so the flag cannot turn channels on.
30. With the organisation's allow entry, `--channels plugin:interweave@interweave` registers the channel without the development flag, and no warning screen is shown (a, e). In a, the channel registered 3.8 s after the start. The peer's message reached the model as a `<channel>` tag, and the model called `reply` with its token. The far peer received `ack` 20.2 s after the start, and the bridge answered "remote transport accepted at endpoint claude". In its first turn, before the prompt, the model said it had no tool for replying, here and in an earlier run of a at `5ad6f58e`: `reply` is a deferred tool, found through `ToolSearch` once the prompt asked for it.
31. `allowedChannelPlugins` replaces Anthropic's allowlist. When it is set and does not name the plugin, the refusal names the organisation's list (d), not the remote allowlist of S19 and of b.
32. `channelsEnabled: false` did not refuse on this login (c). Read from the binary: for a claude.ai login, the gate reads `channelsEnabled` only when the subscription is Team or Enterprise. For any other login, it refuses whenever a policy exists without `channelsEnabled: true`, with "channels not enabled by org policy (set channelsEnabled: true in managed settings)". So on this account, `allowedChannelPlugins` alone decides. That refusal was not measured: it needs a Team or Enterprise subscription, or a login that is not claude.ai.
33. Without managed settings, the session is refused as S19's c was, with the same text (b).

## Where the architecture disagrees

The documents are listed by document and sentence. Amending them is architect-cto's (`architecture/plugin/`) and the contract owner's (`contracts/CHANNEL-EVENT.md`, under ADR-0049).

- **`contracts/CHANNEL-EVENT.md`**, "`source` | constant `p2p`": measured as a duplicate `source` attribute (fact 11).
- **`contracts/CHANNEL-EVENT.md`**, "the bridge ... never constructs channel markup by concatenating unescaped peer-controlled strings": this still holds, and is load-bearing, because Claude Code escapes only the closing tag in a body (fact 14).
- **`architecture/plugin/LIFECYCLE.md` §Shutdown**, "MCP stdin close/SIGTERM stops only the bridge": the first signal is SIGINT, with no stdin close (fact 18). A server still alive after it gets SIGTERM at about 100 ms and SIGKILL at about 500 ms. A bridge cannot count on a graceful window, so releasing its lease must not depend on the bridge doing work after the first signal.
- **`architecture/plugin/CLAUDE-CODE-CHANNEL.md` §Session behavior**, "If daemon is unavailable, bridge remains a functioning MCP server": right, and necessary. A bridge that exits is not restarted (fact 19).
- **`architecture/plugin/PACKAGING.md`**, "Exact Claude manifest syntax remains SPIKE-001": measured as `plugin.json` with a `channels` array naming the `.mcp.json` server, validated and loaded (facts 7 and 8).
- **`architecture/plugin/CLAUDE-CODE-CHANNEL.md` §Capability declaration**: the declaration as written is what the stub declared, and it registered (facts 5 and 7). Answering `server/discover` with `-32601` kept the stub in the legacy era, where it registered (facts 1 and 2). Whether a 2026-07-28-era answer would also register is not measured; the documentation says it would not (fact 3).

## Not established

- `--channels` with a plugin on Anthropic's allowlist. A plugin installed from a local marketplace is refused by `--channels` without an organisation's allow entry (S19, fact 22; S22, fact 33), and registers with one (S22, fact 30). A listing on Anthropic's allowlist was not asked for or tried.
- `userConfig` in a session started with a channel flag, which S20 did not run (it measured the argv `claude mcp get` starts the server with), and the interactive prompt `/plugin configure` shows.
- The `channelsEnabled` refusal (fact 32): it needs a Team or Enterprise subscription, or a login that is not claude.ai. Also not measured: a managed settings file written to the host's own `/etc`, rather than presented to one process; and server-managed or MDM policy, which rank above the file (fact 29).
- Permission relay, which the design does not declare.
- Size limits on `content` and `meta`, and behaviour under load.
- A server negotiating the 2026-07-28 revision (fact 3).
- Whether any non-interactive surface delivers channels.
- Any other `-p` timing than the three tried (fact 4).
