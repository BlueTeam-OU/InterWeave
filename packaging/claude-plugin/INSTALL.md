<!-- SPDX-License-Identifier: Apache-2.0 -->
# Installing the Claude Code Channel bridge on a Linux host

This takes a Linux account that has nothing of InterWeave on it to a Claude Code session whose InterWeave channel is registered. It covers the transport daemon, the profile, its trust, and the plugin.

**Run every step as the account that will run Claude Code.** The bridge reads the profile's configuration and connects to the daemon's socket. Both live under that account's own home and runtime directory. Nothing is installed system-wide, and no step needs root.

What the plugin's steps rest on is measured in SPIKE-001's S19, S20 and S21 ([`spikes/spike-001/README.md`](../../spikes/spike-001/README.md)); the daemon's are `packaging/linux/README.md`'s. What a production host still lacks is in [What this does not give you](#what-this-does-not-give-you).

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

`profile` and `endpoint` are the plugin's two settings: the profile from step 3 and the endpoint its `config.yaml` declares. Unset, they are `default` and `claude`. They are kept in Claude Code's user settings, and `claude plugin configure interweave@interweave` shows or changes them.

The copy under `~/.local/share/interweave/claude-plugin` is what Claude Code runs: with a marketplace added from a directory, the plugin runs from that directory, not from a copy of its own (SPIKE-001 S20). Do not delete it. To upgrade, replace it with the newer archive's `claude-plugin/` and start a new session.

## 7. Start a session with the channel

```sh
claude --dangerously-load-development-channels plugin:interweave@interweave
```

Claude Code shows "WARNING: Loading development channels", naming `plugin:interweave@interweave`. Choose "1. I am using this for local development". Then:

- in the session, the bridge's `status` tool reports this profile's PeerId, the endpoint and a lease epoch;
- with `--debug-file <path>`, the debug log has `MCP server "plugin:interweave:interweave": Channel notifications registered`.

`--channels plugin:interweave@interweave` without the development flag is refused: the plugin is not on Claude Code's approved channels allowlist (SPIKE-001 S19). The flag is how this plugin is used today.

## Removing it

```sh
claude plugin uninstall interweave@interweave
claude plugin marketplace remove interweave
systemctl --user disable --now interweave-daemon@claude
./uninstall.sh                     # from the unpacked archive; keeps every profile
```

`uninstall.sh` leaves the profile, its key and its state under `~/.config/interweave` and `~/.local/share/interweave`. Remove them yourself if the profile is to go too.

## What this does not give you

- **A channel without the development flag.** `--channels` needs the plugin on Anthropic's remotely served allowlist, or an organisation's `allowedChannelPlugins` with `channelsEnabled: true`. Both are read from the 2.1.286 binary, not measured (SPIKE-001 fact 23), and neither is asked for or tried.
- **A headless session.** Channel events reach the model only in an interactive session (SPIKE-001 fact 4), and the development warning is answered by a person.
- **A channel after a bad start.** Claude Code starts the bridge once per session (SPIKE-001 fact 19). Once it runs, daemon trouble is a state: a daemon that is away, restarting, or holding the endpoint for another session is retried with a backoff up to 30 s. A wrong start is not retried: `XDG_RUNTIME_DIR` unset, a profile name that is not valid, or a usage error in `.mcp.json`'s `args` gives a session with no channel, whose `status` says `bridge refused` and why. Fix the cause and start a new session. A session started outside a login, from `cron` or `su` for example, has no `XDG_RUNTIME_DIR` unless the caller sets it.
- **More than one session per endpoint.** The endpoint takes one lease. A second session on the same account and profile gets "endpoint refused" from its bridge's `status` and keeps retrying. It never takes another endpoint. Run one profile, or one endpoint, per session.
- **Logs of the bridge's own.** The bridge writes its few lines to stderr. It has no log file and no log level. Claude Code keeps those lines as `Server stderr:` entries in `~/.cache/claude-cli-nodejs/<working directory>/mcp-logs-plugin-interweave-interweave/<timestamp>.jsonl`, one file per server start, which is where to look when the channel is not there (SPIKE-001 S21). Its lines carry no message content and no peer or endpoint identifier. The daemon logs to the account's journal (`journalctl --user -u interweave-daemon@<profile>`), at the profile's `observability.log_level`, and never logs a payload.
- **What a missed message was.** A daemon restart ends the bridge's lease. The bridge reconnects and joins its channels again, but nothing sent meanwhile is replayed, and a reply token from before the restart is refused. That a gap happened, and when, is reported: the bridge's `status` shows `lease_gap` with the lost and the new lease and the two instants, and in pull mode the first `receive` after it carries the same.
- **An upgrade Claude Code reports.** The plugin's version is `0.0.0` in every release, so `claude plugin update` reports nothing to update. That does not stop an upgrade: a replaced marketplace directory runs at the next session (SPIKE-001 S20).
