<!-- SPDX-License-Identifier: Apache-2.0 -->
# Installing the Claude Code Channel bridge on a Linux host

This takes a Linux account that has nothing of InterWeave on it to a Claude Code session whose InterWeave channel is registered. It covers the transport daemon, the profile, its trust, and the plugin.

**Run every step but step 7 as the account that will run Claude Code.** The bridge reads the profile's configuration and connects to the daemon's socket. Both live under that account's own home and runtime directory. Step 7 is the one setting that is host-wide: an administrator writes it as root, once per host. Nothing else is installed system-wide.

What the plugin's steps rest on is measured in SPIKE-001's S19, S20, S21 and S22 ([`spikes/spike-001/README.md`](../../spikes/spike-001/README.md)); the daemon's are `packaging/linux/README.md`'s. What a production host still lacks is in [What this does not give you](#what-this-does-not-give-you).

## 0. Before you start

- **Linux x86_64, with a systemd user manager for the account.** Log in to the account, or turn on lingering (`loginctl enable-linger <account>`, which an administrator runs). The daemon and the bridge both need `XDG_RUNTIME_DIR`, the directory the user manager provides; `echo "$XDG_RUNTIME_DIR"` must print a path. With it unset, the bridge cannot reach the daemon, and Claude Code does not start it again (SPIKE-001 fact 19): it serves the session without a channel, and its `status` tool reports `bridge refused` with the class `runtime_dir_unset`. There is no fallback directory, on purpose.
- **Claude Code**, installed for this account and signed in with this account's own credential. The session that loads the channel is an interactive one.
- **The release archive** `interweave-<version>-x86_64-unknown-linux-gnu.tar.gz` and its `.sha256` file, from the project's release page. It holds the three programs, the two user units, the install scripts, and `claude-plugin/`, the plugin marketplace with the bridge inside it.
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

This installs `transport-daemon`, `transportctl` and `human-desktop` into `~/.local/bin`, and the two unit templates into `~/.config/systemd/user`. It enables nothing, and it does not install the bridge: the bridge ships only inside the plugin (step 6). `packaging/linux/README.md` covers this step and uninstalling.

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

Trust is deny by default: with an empty allowlist, no message reaches the bridge and the bridge can send to no one. Either list the peers' PeerIds in `trust.allowed_peers` before the daemon starts, or add them to the running daemon:

```sh
transportctl --profile claude trust allow <PeerId>
transportctl --profile claude trust list
```

`trust allow` takes effect at once and is kept across restarts. Trust is per direction and per side. The other peer's daemon must also trust this profile's PeerId before anything it sends arrives here, and before a reply from here reaches it.

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

`profile` and `endpoint` are the plugin's two routing settings: the profile from step 3 and the endpoint its `config.yaml` declares. Unset, they are `default` and `claude`. A third, `log_level`, is how much the bridge writes to its own log: `error`, `warn`, `info` or `debug`, `info` unset (`--config log_level=debug` while you debug). They are kept in Claude Code's user settings, and `claude plugin configure interweave@interweave` shows or changes them.

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
- **Keep both keys.** On a claude.ai Team or Enterprise subscription, or a login that is not claude.ai, Claude Code refuses every channel unless `channelsEnabled` is `true`. On other claude.ai subscriptions it does not read `channelsEnabled`, and the list alone decides (fact 32).
- Server-managed settings and MDM policy rank above this file. Neither was measured (fact 29).

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

## Removing it

```sh
claude plugin uninstall interweave@interweave
claude plugin marketplace remove interweave
systemctl --user disable --now interweave-daemon@claude
./uninstall.sh                     # from the unpacked archive; keeps every profile
```

`uninstall.sh` leaves the profile, its key and its state under `~/.config/interweave` and `~/.local/share/interweave`. Remove them yourself if the profile is to go too. Step 7's file stays too: an administrator removes `/etc/claude-code/managed-settings.d/interweave.json` once no account on the host uses the channel.

## What this does not give you

- **A channel without the development flag and without the organisation's setting.** The plugin is not on Anthropic's remotely served allowlist, and that has not been asked for. Without step 7, `--channels` is refused (SPIKE-001 S22, fact 33). Step 7 is measured with the file presented to one session at its path, not with a file in a host's own `/etc`, and its `channelsEnabled` refusal is not measured (facts 29 and 32).
- **A headless session.** Channel events reach the model only in an interactive session (SPIKE-001 fact 4). With step 7 no warning screen is shown, but the session is still an interactive one; with the development flag instead, a person answers its warning.
- **A channel after a bad start.** Claude Code starts the bridge once per session (SPIKE-001 fact 19). Once it runs, daemon trouble is a state: a daemon that is away, restarting, or holding the endpoint for another session is retried with a backoff up to 30 s. A wrong start is not retried: `XDG_RUNTIME_DIR` unset, a profile name that is not valid, or a `profile`, `endpoint` or `log_level` setting that is not valid (step 6) gives a session with no channel, whose `status` says `bridge refused` and why. Fix the cause and start a new session. A session started outside a login, from `cron` or `su` for example, has no `XDG_RUNTIME_DIR` unless the caller sets it.
- **More than one session per endpoint.** The endpoint takes one lease. A second session on the same account and profile gets "endpoint refused" from its bridge's `status` and keeps retrying. It never takes another endpoint. Run one profile, or one endpoint, per session.
- **The bridge's log beyond its last 2 MiB, or in the journal.** The bridge writes its own lines -- its start, each session opened and lost, a refused open or re-join, a dropped message's class, its end -- to stderr and to `~/.local/state/interweave/profiles/<profile>/claude-channel/<endpoint>.log`, owner-only, at the `log_level` setting (step 6). Before a line would take that file past 1 MiB, the file is moved to `<endpoint>.log.1`, replacing the one there: what is older than the two files is gone. Two bridges on one endpoint (the second refused its lease) share the file. A start refused before it knew its profile and endpoint -- a usage error, `HOME` unset, a profile name that is not valid -- has no file, and a state directory that is not this account's alone gets none either: `status` says why under `log.file_error`, and stderr is the only copy. Claude Code keeps stderr as `Server stderr:` entries in `~/.cache/claude-cli-nodejs/<working directory>/mcp-logs-plugin-interweave-interweave/<timestamp>.jsonl`, one file per server start (SPIKE-001 S21). Neither carries message content, a reply token, a message id, or a peer's or endpoint's identifier. The bridge's lines do not reach the journal. The daemon logs to the account's journal (`journalctl --user -u interweave-daemon@<profile>`), at the profile's `observability.log_level`, and never logs a payload.
- **What a missed message was.** A daemon restart ends the bridge's lease. The bridge reconnects and joins its channels again, but nothing sent meanwhile is replayed, and a reply token from before the restart is refused. That a gap happened, and when, is reported: the bridge's `status` shows `lease_gap` with the lost and the new lease and the two instants, and in pull mode the first `receive` after it carries the same.
- **An update report between two builds of one release.** The plugin's version is the release version, and S20 measured `claude plugin update` reporting a higher one (case B) and not an equal one (case A); a lower one was not measured. A rebuild at the same version, a development build from a checkout included, reports nothing to update. The replaced directory still runs at the next session (SPIKE-001 S20, case A).
