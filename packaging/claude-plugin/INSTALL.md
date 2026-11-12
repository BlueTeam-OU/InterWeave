<!-- SPDX-License-Identifier: Apache-2.0 -->
# Installing the Claude Code Channel bridge on a Linux host

This takes a Linux account that has nothing of InterWeave on it to a Claude Code session whose InterWeave channel is registered. It covers the transport daemon, the profile, its trust, and the plugin.

**Run every step but step 7 as the account that will run Claude Code.** The bridge reads the profile's configuration and connects to the daemon's socket. Both live under that account's own home and runtime directory. Step 7 is the one setting that is host-wide: an administrator writes it as root, once per host. Nothing else is installed system-wide.

What the plugin's steps rest on is measured in SPIKE-001's S19 to S22 and S24 to S28 ([`spikes/spike-001/README.md`](../../spikes/spike-001/README.md)); the daemon's are `packaging/linux/README.md`'s. What a production host still lacks is in [What this does not give you](#what-this-does-not-give-you).

## 0. Before you start

- **Linux x86_64, with a systemd user manager for the account.** Log in to the account, or turn on lingering (`loginctl enable-linger <account>`, which an administrator runs). The daemon and the bridge both need `XDG_RUNTIME_DIR`, the directory the user manager provides; `echo "$XDG_RUNTIME_DIR"` must print a path. With it unset, the bridge cannot reach the daemon, and Claude Code does not start it again (SPIKE-001 fact 19): it serves the session without a channel, and its `status` tool reports `bridge refused` with the class `runtime_dir_unset`. There is no fallback directory, on purpose.
- **Claude Code**, installed for this account and signed in with this account's own credential. The session that loads the channel is an interactive one, or, once step 7 is in place, a non-interactive one (step 8).
- **The release archive** `interweave-<version>-x86_64-unknown-linux-gnu.tar.gz` and its `.sha256` file, from the project's release page. It holds the four programs (`transport-daemon`, `transportctl`, `human-desktop` and `interweave-human`, the command-line human client), the two user units, the install scripts, and `claude-plugin/`, the plugin marketplace with the bridge inside it.
- **One profile name and one TCP port per profile.** Below, the profile is `claude`, the port is `4001`, and the bridge's endpoint is `claude`. The plugin starts the bridge with `--profile default --endpoint claude` unless you change that (step 6).

## 1. Check and unpack the archive

```sh
sha256sum -c interweave-<version>-x86_64-unknown-linux-gnu.tar.gz.sha256
tar -xzf interweave-<version>-x86_64-unknown-linux-gnu.tar.gz
cd interweave-<version>-x86_64-unknown-linux-gnu
```

## 2. Install the daemon and its tools

```sh
./install.sh
```

This installs `transport-daemon`, `transportctl`, `human-desktop` and `interweave-human` into `~/.local/bin`, and the two unit templates into `~/.config/systemd/user`. It enables nothing, and it does not install the bridge: the bridge ships only inside the plugin (step 6). `packaging/linux/README.md` covers this step and uninstalling.

## 3. Create the profile

```sh
transportctl identity init --profile claude      # prints this profile's PeerId, once
```

Keep the printed PeerId: it is what other peers trust (step 4). The key never leaves this account.

`identity init` puts the key under `~/.local/share/interweave` and creates no configuration directory, so make that directory, readable by this account only:

```sh
mkdir -p -m 0700 ~/.config/interweave ~/.config/interweave/profiles ~/.config/interweave/profiles/claude
```

Write `~/.config/interweave/profiles/claude/config.yaml`. The bridge's endpoint is declared under `endpoints.entries`. In the file below, it is also the default endpoint, so a message a peer sends without naming an endpoint reaches the bridge. The example `architecture/config/examples/human-and-claude.yaml` shows the same endpoint beside a human one.

```yaml
schema_version: 2
profile: { name: claude }
transport:
  backend: libp2p
  listen: { addresses: ["/ip4/0.0.0.0/tcp/4001"] }
identity: {}
trust:
  policy: static-allowlist
  allowed_peers: []          # filled in step 4
endpoints:
  registration_policy: configured-only
  default_direct_endpoint: claude
  entries:
    - id: claude
      enabled: true
      advertise: true
      allowed_client_kinds: [claude-channel]
      inbound: inherit_profile_trust
      outbound: inherit_profile_trust
```

The configuration directory and the file must be this account's and not writable by anyone else. The daemon and the bridge both refuse a configuration that is. The port, `4001` here, must be free on the host: another profile, or another account's daemon, on the same port is a conflict (`packaging/linux/README.md`).

## 4. Trust the peers the bridge talks to

Trust is deny by default: with an empty allowlist, no message reaches the bridge and the bridge can send to no one. Either list the peers' PeerIds in `trust.allowed_peers` before the daemon starts, or add them to the running daemon once step 5 has started it. Before that, `transportctl` answers `no daemon is running for profile "<profile>"` and exits 3 (measured on the 0.3.0 archive, `evidence/2026-11-11-v0.3.0-install-run.md`):

```sh
transportctl --profile claude trust allow <PeerId>
transportctl --profile claude trust list
```

`trust allow` takes effect at once and is kept across restarts. Trust is per direction and per side. The other peer's daemon must also trust this profile's PeerId before anything it sends arrives here, and before a reply from here reaches it.

Trust does not say where a peer is. One side has to be told the other's address: `transportctl --profile <its profile> peers add /ip4/<address>/tcp/4001/p2p/<PeerId>` (or `interweave-human ... peers add`) on the peer, naming this host's address and this profile's port and PeerId. `peers add` refuses a loopback address as `special_use`, even with both profiles on one host, so name the host's own LAN address. That was measured with `interweave-human` on the 0.3.0 archive (`evidence/2026-11-11-v0.3.0-install-run.md`). That the daemon's admin path refuses every special-use range is read from `crates/transport/composition/src/runtime.rs`. Whether a loopback address written in `config.yaml` is refused was not checked. Once the peer is connected, the bridge replied over that connection (same record).

## 5. Start the daemon

```sh
systemctl --user daemon-reload
systemctl --user enable --now interweave-daemon@claude
transportctl --profile claude status
journalctl --user -u interweave-daemon@claude -n 20
```

`enable` makes the daemon start with the account's user manager, at login or at boot with lingering. A changed `config.yaml` is applied by `systemctl --user restart interweave-daemon@claude`. A restart ends the bridge's endpoint lease. The bridge reconnects by itself and joins its channels again, but nothing sent meanwhile is replayed.

## 6. Install the plugin

Copy the marketplace somewhere it stays, then add and install it at user scope:

```sh
mkdir -p ~/.local/share/interweave
cp -R claude-plugin ~/.local/share/interweave/claude-plugin
claude plugin marketplace add ~/.local/share/interweave/claude-plugin
claude plugin install interweave@interweave --config profile=claude --config endpoint=claude
claude plugin list
```

`profile` and `endpoint` are the plugin's two routing settings: the profile from step 3 and the endpoint its `config.yaml` declares. Unset, they are `default` and `claude`. A third, `log_level`, is how much the bridge writes to its own log: `error`, `warn`, `info` or `debug`, `info` unset (`--config log_level=debug` while you debug). With `log_level` unset, the install ends `1 userConfig option not yet set`, and `claude plugin configure` lists it as `not set`. The bridge then runs at `info`, and its `status` tool reports `info` under `log` (measured on the 0.3.0 archive with Claude Code 2.1.322, `evidence/2026-11-11-v0.3.0-install-run.md`). They are kept in Claude Code's user settings. `claude plugin configure interweave@interweave` shows them and changes nothing, on a terminal or not. To change them, pipe a JSON object to it with `--values-stdin`: the bridge's next start is given them (SPIKE-001 S25, fact 50):

```sh
echo '{"profile": "claude", "endpoint": "claude"}' | claude plugin configure interweave@interweave --values-stdin
```

The copy under `~/.local/share/interweave/claude-plugin` is what Claude Code runs: with a marketplace added from a directory, the plugin runs from that directory, not from a copy of its own (SPIKE-001 S20). Do not delete it. To upgrade, replace it with the newer archive's `claude-plugin/` and start a new session: the replaced directory is what the next session runs (SPIKE-001 S20). The plugin's version is the release's, so Claude Code then sees the upgrade:

```sh
claude plugin marketplace update interweave
claude plugin update interweave@interweave
```

The second command reports the plugin updated from the old version to the new one (SPIKE-001 S20, case B, measured with two hand-set versions).

## 7. Allow the channel for the organisation

Claude Code registers a channel without its development flag only when the channel's plugin is on an approved list. For this plugin, that list is the organisation's own, in Claude Code's managed settings on the host. An administrator writes it, as root, to `/etc/claude-code/managed-settings.d/interweave.json`:

```json
{
  "channelsEnabled": true,
  "allowedChannelPlugins": [
    { "plugin": "interweave", "marketplace": "interweave" }
  ]
}
```

The file must be readable by the account that runs Claude Code. Claude Code reads `/etc/claude-code/managed-settings.json` and every `*.json` in `/etc/claude-code/managed-settings.d/`. No environment variable moves that path (SPIKE-001 S22, fact 29).

- **The file applies to every account on the host.** Once `allowedChannelPlugins` is set, it replaces Anthropic's allowlist: a channel plugin it does not name is refused for every account (fact 31). If the host's managed settings already set `allowedChannelPlugins`, add this pair to that list. How two files' lists combine was not measured.
- **Keep both keys.** Read from the Claude Code binary, not measured as a whole (SPIKE-001 S22, fact 32): on a claude.ai Team or Enterprise subscription, or a login that is not claude.ai, Claude Code refuses every channel unless `channelsEnabled` is `true`; on other claude.ai subscriptions it does not read `channelsEnabled`, and the list alone decides. What is measured: an API-key login, with a placeholder key, was refused with the list alone, and with `channelsEnabled: false` (S27, fact 59; a valid Console key was not run, and is inferred to take the same branch); a claude.ai login was not refused with `false` (fact 58); the Team or Enterprise branch was not run.
- Server-managed settings and MDM policy rank above this file (fact 29). Neither was measured: the account the runs had has no server-managed policy, and, read from the binary and not measured, Claude Code reads MDM on Linux only under WSL (S27, facts 60 and 61).

## 8. Start a session with the channel

```sh
claude --channels plugin:interweave@interweave
```

No warning screen is shown (fact 30). Then:

- in the session, the bridge's `status` tool reports this profile's PeerId, the endpoint, a lease epoch, and under `log` the level and the path of the bridge's log file;
- with `--debug-file <path>`, the debug log has `MCP server "plugin:interweave:interweave": Channel notifications registered`.

Without step 7, the debug log has `Channel notifications skipped: plugin interweave@interweave is not on the approved channels allowlist (use --dangerously-load-development-channels for local dev)`, and the session has no channel (fact 33). With an `allowedChannelPlugins` that does not name the plugin, the log says it `is not on your org's approved channels list` (fact 31).

On a host without step 7, the development flag still registers the channel, for one account and with a person at the terminal:

```sh
claude --dangerously-load-development-channels plugin:interweave@interweave
```

Claude Code then shows "WARNING: Loading development channels", naming `plugin:interweave@interweave`. Choose "1. I am using this for local development" (SPIKE-001 S19).

### A peer that is a human client

The human clients, `interweave-human` and `human-desktop`, show only a message in their chat envelope. That was measured for `interweave-human`. For `human-desktop` it is read from `architecture/clients/human/HUMAN-CHAT.md` and the facade the two clients share. A message from one arrives with `content_type` `application/vnd.interweave-human-chat+json;v=2`, and its content is a JSON object. An answer that is plain text is still accepted: `reply` reports `remote transport accepted at endpoint human`, and the client shows nothing. `interweave-human status` counts it as neither unread nor undisplayable. An answer reaches the person only as such an object, `{"v":2,"kind":"text","app_message_id":"<32 lowercase hex digits>","text":"<the answer>","reply_to":"<the app_message_id answered>"}`, sent with that same `content_type`. Say so in the first user turn, beside the sentence in "Without a terminal" below. One run of each, on the 0.3.0 archive and Claude Code 2.1.322 (`evidence/2026-11-11-v0.3.0-install-run.md`): a plain answer was accepted and not shown, and an answer in the envelope was shown. That the client counts a plain answer as a message of unknown media type is `tests/desktop-e2e`'s `the_human_clients_facade_exchanges_with_the_bridge_plain_and_compressed`, which this run did not repeat.

The peer's endpoint must also be held when the answer arrives. A peer that sent with `interweave-human ... send` and then exited holds nothing, and `reply` fails with `RemoteEndpointUnavailable` (one run, same record). Keep a client running on the peer, `interweave-human ... listen` for example.

### Without a terminal

A program that drives Claude Code non-interactively gets the channel too, with step 7 in place:

```sh
claude -p --input-format stream-json --output-format stream-json --verbose --channels plugin:interweave@interweave \
  --allowedTools mcp__plugin_interweave_interweave__reply mcp__plugin_interweave_interweave__status
```

The program writes user turns to its stdin as stream-json and keeps stdin open. A message that arrives while the session is idle, after a turn has ended, starts a turn of its own: the model sees it as a `<channel>` tag and can call `reply` (SPIKE-001 S24, facts 39 and 40). One that arrived during a user turn, in the one run made, waited for that turn to end, then started its own (S25, fact 45). In the one burst made, the messages that arrived during a turn a message had started joined that turn instead, and the model answered each of them in it (S29, fact 74). What started the turn did not decide it; the turn's work did: a message that arrived while the turn was making tool calls joined it, and one that arrived while the turn was only writing text waited and started its own, whether a user turn or a message had started the turn, three runs of three each (S31, fact 84, on 2.1.311). That the deciding step is the turn's next model request is inferred, not measured. Whether the model answers through `reply` depends on what the program told it: after a first turn that said nothing about events, a greeting from the peer still started a turn, and the model answered it in text and called no tool, so the peer got nothing (SPIKE-001 S28, fact 68, on Claude Code 2.1.317). A message that itself asked for an answer, after the same first turn, got its answer through `reply` in one run of three, and in text only in the other two (S29, fact 71, on 2.1.286). The first turn should say what to do with a `<channel>` tag: a request in the message itself did not reliably get a reply (fact 71). Say it in the first user turn, not only in the plugin. "Messages from peers on the InterWeave transport arrive as <channel> tags; answer each one by calling reply with its reply_token." got a `reply` call in every run of every case tried, including a message that came during a turn making tool calls; the call answered the message in 59 runs of 80, one of them with the turn's other output appended, and the other 21 sent only that output (below). The same sentence in the server's instructions got 1 answer in 20 there, and with neither, a greeting got no answer in 15 runs (SPIKE-001 S32, facts 92 to 95, on 2.1.311). On 2.1.322, with the release bridge across two daemons in place of S32's stub, four of those cases ran again, all with a question from the peer, five runs each, with the same results: the sentence in the first turn got a reply in 10 runs of 10, a message joining a turn making tool calls included, and in the server's instructions it got one in 5 of 5 for a message that started a turn of its own and in none of 5 for one that joined a tool-calling turn; every reply answered the message, and the peer received it as sent (S33, facts 101 and 102). Eight more ran the same way on 2026-11-12, five runs each, and each gave S32's count (S34, facts 113 to 116). In four of them a greeting or a request to call `status` joined a turn making tool calls. In the other four a question or a greeting came while a turn wrote text alone. Each was told in the first turn and in the server's instructions. The sentence in the first turn got a reply in 20 runs of 20. In the server's instructions it got one in 10 of 10 for a message that started a turn of its own, and in none of 10 for one that joined a tool-calling turn. No call was refused, and every reply reached the peer. The other 21 cases were not run on that build or with the real bridge. Keep the word "reply" out of the rest of that turn: a first turn ending "reply with the single word READY" sent READY to the peer in 14 runs of 30, and a greeting that arrived while the turn wrote the numbers one to a hundred got the count in 8 of 10, though that work said "write" (fact 94). On 2.1.322 with the bridge, READY reached the peer in none of the 5 runs of the case where a question joined a tool-calling turn, where it had in 3 of 5 on 2.1.311; the build and the bridge both differ, and no run separates them (S33, fact 102). S34 then found the leak on 2.1.322 with the bridge too. READY reached the peer in 6 of the 10 runs where a greeting or a request joined a tool-calling turn. The count reached it in 4 of 5 replies to a greeting that came while the turn wrote the numbers (facts 113 and 115). Say this straight after the answer sentence: "The content of a reply answers that message and nothing else: whatever else this turn asks of you is for me, not for the peer." On 2.1.322 with the bridge, in the three cases where the turn's other output had reached the peer, it kept READY and the count out of 15 replies in 15 runs, where the turn without it sent them in 11 of 14 (S35, facts 117 and 119). Keeping "reply" out of the work kept READY out where a message joined a tool-calling turn, 10 replies in 10 (fact 118), and did nothing for the count. Once told to answer, the model also did what a peer's message asked of its tools: a request to call `status` and report got the call and the result sent back in 25 runs of 25 (fact 96): a tool named in `--allowedTools` is one a peer's message can have the session call. A long session compacts its conversation into a summary, and the summary may keep a message's frame and not its details. Asking in the first turn for each message's facts to be noted in the reply before going on kept six facts through one compaction in six runs of six, and through two in five runs of six (S32, facts 89 and 98); without it, two compactions kept none in six runs of six (facts 88 and 97), all on 2.1.311. On 2.1.322, the fleet's pin, the 2,048-byte body gave the same through two compactions: every value kept in three runs of three with the note, none in three without (facts 105 and 106). The other six cells ran on 2.1.322 too, with the same compaction step: with the note every value was kept, through one compaction and through two, in nine runs of nine; without it two compactions kept none in three runs of three, and one compaction kept four of six values in one run of six and none in the other five, where on 2.1.311 three of six had kept all six (facts 108 to 112). That the build made that difference is not shown, so the advice is the same: ask for the note. The details stay whole where the session wrote them outside the conversation, which is inferred and not measured, and in the session's transcript, whose path every summary names (fact 91). `--allowedTools` lets the model call the bridge's tools with nobody there to approve them. With it, `reply` went through in the `acceptEdits` and `dontAsk` permission modes, three runs of three each, and was denied in `plan`, three of three, so the bridge's allow list did not let it out there (S29, fact 70, on 2.1.286). Without it, `reply` is denied and the peer gets nothing in the `manual`, `acceptEdits`, `dontAsk` and `plan` permission modes (S25, fact 48; S26, fact 51), and with `--permission-prompts none` (fact 55). Without it, `reply` went through in three cases. In `auto`, which run h recorded on Claude Code 2.1.286 (fact 44), before S31 traced the mode to a served flag (fact 85, below), and on 2.1.311 and 2.1.317 a print session with no mode flag and the bridge's tools in `--allowedTools` reported `default` instead, with the same auto-mode gate line in its debug log (S28, fact 69; S29, fact 75). On 2.1.286 too, sessions that started late reported `default` (fact 75). Which of the two a session takes followed a feature flag served to Claude Code from outside the host: with the flag's cached value set true a session took `auto` on both builds, with the served value cached it took `default`, and with a fresh config directory it took `auto` on 2.1.286 and `default` on 2.1.311 (S31, fact 85); that each build reads the flag with a default of its own is read from the binaries, not measured. So do not count on `auto`. In `bypassPermissions`, which grants every tool, not only the bridge's (read from the help, not measured; fact 52). And when the program answers the permission prompt itself: started with `--permission-prompt-tool stdio`, the session writes a `can_use_tool` control request to stdout, and the program's `control_response` allows or denies the call (fact 53). `--permission-prompts host` alone sent the program no request (fact 54). Instead of `--channels`, the program can send the stream-json control request `channel_enable` naming `plugin:interweave:interweave`, which registers the channel on its own (fact 40). The development flag did nothing here: on the runs made without step 7, with `channel_enable` or the flag or both, the session had no channel (fact 41). Without step 7, `--channels` gives no channel either, and no line of its debug log, stderr or stdout matched the refusal patterns searched for (S25, fact 47). A print session with no flag or with the development flag that does not register a channel logs nothing about it, so there the only sign is a missing line in its debug log (fact 42); S25's run with `--channels` and no control request, which registered none, had no `Channel` line for the server (fact 47). Registered by `--channels`, the log has `Channel notifications re-registered after reconnect`; registered by `channel_enable`, it has `Channel notifications registered` first and then that reconnect line (fact 40, on Claude Code 2.1.286).

## Removing it

```sh
claude plugin uninstall interweave@interweave
claude plugin marketplace remove interweave
systemctl --user disable --now interweave-daemon@claude
./uninstall.sh                     # from the unpacked archive; keeps every profile
```

`uninstall.sh` stops and disables every `interweave-daemon@` and `interweave-client@` instance of the account, not only `claude`'s. When `XDG_RUNTIME_DIR` reaches no user manager, it looks the account's own manager up in `/run/user/<uid>`. So an account that runs another profile's daemon, or a client of one, loses those too, and has to enable them again. That is read from `packaging/linux/uninstall.sh`'s header, which `packaging/linux/README.md` also states, and it was seen once in the 0.3.0 run (`evidence/2026-11-11-v0.3.0-install-run.md`). It leaves the profile, its key and its state under `~/.config/interweave` and `~/.local/share/interweave`. Remove them yourself if the profile is to go too. Step 7's file stays too: an administrator removes `/etc/claude-code/managed-settings.d/interweave.json` once no account on the host uses the channel.

## What this does not give you

- **A channel without the development flag and without the organisation's setting.** The plugin is not on Anthropic's remotely served allowlist, and that has not been asked for. Without step 7, `--channels` is refused (SPIKE-001 S22, fact 33). Step 7's drop-in is measured presented to one session at its path. A `managed-settings.json` with step 7's content was also read from a container's own `/etc`, written there by its root (SPIKE-001 S27, fact 57); the drop-in was not tested there, and a file in a host's own `/etc` was not measured. On a claude.ai login, `channelsEnabled: false` did not refuse the channel (fact 58). On an API-key login with a placeholder key, a file without `channelsEnabled: true` refused it, even where `allowedChannelPlugins` named the plugin (fact 59; a valid Console key was not run). Step 7's file sets both. A policy served by Anthropic for an organisation, or set through MDM, was not measured (facts 60 and 61).
- **A headless session without step 7, or one whose input has ended.** A non-interactive session got the channel only through step 7 on the runs made. The development flag gave it nothing (SPIKE-001 S24, fact 41), and neither did `--channels` without step 7, with no line in the debug log, stderr or stdout matching the refusal patterns searched for (S25, fact 47). It answers a message that arrives after a finished turn with stdin open (fact 39), or, in the one run with stdin open during a user turn, once that turn ended (S25, fact 45); in the one burst during a turn a message had started, the later messages joined that turn (S29, fact 74); a message joins a turn that goes on to make tool calls and waits for one that only writes text, whatever started it (S31, fact 84). In the one run with stdin closed during a user turn, the session finished the turn and answered before it exited (fact 46). A message after stdin has closed and the last turn has ended is not delivered: closing stdin with no turn running stopped the bridge within 0.1 s and the session exited within a second (S28, fact 67, three runs on Claude Code 2.1.317). In one of the three the daemon accepted the send at the bridge's endpoint, in the same 0.1 s as the bridge's end, and it was still not delivered, so an accepted send does not show the session took it. A program that wants messages keeps stdin open. A plain `claude -p "<prompt>"` ends after its one turn, leaving a later message nowhere to go (fact 44).
- **A channel after a bad start.** Claude Code starts the bridge once per session (SPIKE-001 fact 19). Once it runs, daemon trouble is a state: a daemon that is away, restarting, or holding the endpoint for another session is retried with a backoff up to 30 s. A wrong start is not retried: `XDG_RUNTIME_DIR` unset, a profile name that is not valid, or a `profile`, `endpoint` or `log_level` setting that is not valid (step 6) gives a session with no channel, whose `status` says `bridge refused` and why. Fix the cause and start a new session. A session started outside a login, from `cron` or `su` for example, has no `XDG_RUNTIME_DIR` unless the caller sets it.
- **More than one session per endpoint.** The endpoint takes one lease. A second session on the same account and profile gets "endpoint refused" from its bridge's `status` and keeps retrying. It never takes another endpoint. Run one profile, or one endpoint, per session.
- **The bridge's log beyond its last 2 MiB, or in the journal.** The bridge writes its own lines -- its start, each session opened and lost, a refused open or re-join, a dropped message's class, its end -- to stderr and to `~/.local/state/interweave/profiles/<profile>/claude-channel/<endpoint>.log`, owner-only, at the `log_level` setting (step 6). Before a line would take that file past 1 MiB, the file is moved to `<endpoint>.log.1`, replacing the one there: what is older than the two files is gone. Two bridges on one endpoint (the second refused its lease) share the file. A start refused before it knew its profile and endpoint -- a usage error, `HOME` unset, a profile name that is not valid -- has no file, and a state directory that is not this account's alone gets none either: `status` says why under `log.file_error`, and stderr is the only copy. Claude Code keeps stderr as `Server stderr:` entries in `~/.cache/claude-cli-nodejs/<working directory>/mcp-logs-plugin-interweave-interweave/<timestamp>.jsonl`, one file per server start (SPIKE-001 S21). Neither carries message content, a reply token, a message id, or a peer's or endpoint's identifier. The bridge's lines do not reach the journal. The daemon logs to the account's journal (`journalctl --user -u interweave-daemon@<profile>`), at the profile's `observability.log_level`, and never logs a payload.
- **What a missed message was.** A daemon restart ends the bridge's lease. The bridge reconnects and joins its channels again, but nothing sent meanwhile is replayed, and a reply token from before the restart is refused. That a gap happened, and when, is reported: the bridge's `status` shows `lease_gap` with the lost and the new lease and the two instants, and in pull mode the first `receive` after it carries the same.
- **An update report between two builds of one release.** The plugin's version is the release version, and S20 measured `claude plugin update` reporting a higher one (case B) and not an equal one (case A); a lower one was not measured. A rebuild at the same version, a development build from a checkout included, reports nothing to update. The replaced directory still runs at the next session (SPIKE-001 S20, case A).
