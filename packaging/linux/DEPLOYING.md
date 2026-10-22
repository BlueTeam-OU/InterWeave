# Deploying a daemon on a new host

From the four programs to a running `interweave-daemon@<profile>` unit
for one account, with that account's own privileges and nothing installed
system-wide. One account runs one profile here; a host with many profiles
runs one account per profile (ADR-0055 decision 5), and repeats this for
each. The recorded run is
[`evidence/2026-10-22-deployment-run.md`](evidence/2026-10-22-deployment-run.md),
in one account under a scratch home, not a second one.

Every command runs **as the account that will run the profile, in that
account's own login session** -- an SSH login or a console, which starts
its systemd user manager and sets `XDG_RUNTIME_DIR`. Not `sudo -u`,
`su -c` or cron: without `XDG_RUNTIME_DIR` the daemon, `transportctl`'s
daemon commands and the bridge refuse to start, since the IPC sockets
have no safe place to live (`NoRuntimeDir`).

## 0. What the host needs

- Linux x86_64 with systemd, and a user manager for the account
  (`systemctl --user status` answers).
- Lingering for the account, so its units keep running with nobody
  logged in and start at boot: `loginctl enable-linger` (the host's
  polkit policy decides whether an account may do this for itself;
  otherwise an administrator runs `loginctl enable-linger <account>`).
  `loginctl show-user "$USER" -p Linger` says `Linger=yes`.
- A TCP port for the profile that no other account on the host uses.
  Another account's daemon on the same port fails to start and its unit
  retries every 5 s; two profiles of one account on one port both start
  silently (`SO_REUSEPORT`), so give every profile its own.
- The XDG base directories at their defaults, or set where the user
  manager reads them. The paths below (`~/.config`, `~/.local/share`,
  `~/.local/state`, `~/.cache`) are the defaults. The user manager --
  and so the unit -- does not read a shell profile's exports: an
  `XDG_DATA_HOME` exported only in `~/.profile` sends `identity init`'s
  key where the daemon never looks, and the daemon exits naming the
  default path, restarted every 5 s. Leave `XDG_*_HOME` unset; a host
  that needs them sets the same values for the manager
  (`~/.config/environment.d/*.conf`) and for the shell the commands run
  in, which does not read that directory.
- No root, no display, no D-Bus session bus, no keyring and no terminal:
  none of the daemon, `transportctl identity init|show` or the bridge
  asks for one.

### The account's directories

The daemon and `transportctl` refuse a profile that another account
could change, before reading or writing anything in it. On an account
made by `useradd` with a user-private group (`USERGROUPS_ENAB yes` in
`/etc/login.defs`, as on the Fedora host the recorded run used) and a
home no other account can write to, nothing here needs doing: every directory below the home that is missing is
created owner-only (0700), the key and the state files 0600. The rules,
for an account set up otherwise:

- **Every directory above the profile**, up to `/` -- the home and the
  XDG roots included -- is owned by root or by the account, and no other
  account can write to it: not writable by others unless sticky (like
  `/tmp`), and group-writable only when the group is the account's own
  private group (named as the account, with no other members, no ACL on
  the directory). A 0775 home in a shared group such as `users`, or XDG
  roots under a directory another non-root account owns, are refused.
- **The state and data directories** of the profile
  (`~/.local/state/interweave/profiles/<profile>`,
  `~/.local/share/interweave/profiles/<profile>`) are owned by the
  account and owner-only (0700); one made by hand with `mkdir -p` at
  0755 is refused. Let `identity init` create them.
- **The config directory and `config.yaml`**: owned by root or the
  account and writable by no other account; `config.yaml` is a regular
  file, not a symbolic link (a dotfiles manager that links it is
  refused), at most 1 MiB.
- **The key** (`identity.key`) and every file in the state directory
  are owner-only (0600); one restored from a backup with wider modes, or
  owned by another uid, stops the daemon.
- **`$XDG_RUNTIME_DIR/interweave`**, the sockets' directory, is the
  account's and exactly 0700; the daemon creates it so.
- **A profile name** is 1 to 64 characters of `A-Z a-z 0-9 _ -`, not
  starting with a dot.

A refusal names the path and the rule it broke, and the daemon exits 1
(its unit retries every 5 s). Two wordings mislead, so read past them.
A directory "must be owner-only before key-equivalent material is
written into it" means, for a directory above the profile or the config
directory, that no other account may write to it (a 0755 home is fine),
and for the profile's state or data directory, literally 0700. And from
an `identity` command, "lock is held -- stop its daemon first" is
printed for any refusal of the profile's lock, a directory's mode
included -- the reason in brackets is the one that counts.

## 1. The programs

From the release archive of the version, which the release run builds
and the release page links:

```sh
sha256sum -c interweave-<version>-x86_64-unknown-linux-gnu.tar.gz.sha256   # beside the .tar.gz
tar -xzf interweave-<version>-x86_64-unknown-linux-gnu.tar.gz
cd interweave-<version>-x86_64-unknown-linux-gnu
./install.sh
```

Or from a release build of a named commit of the repository, on any
host with the toolchain (the build host needs fontconfig's development
files for `human-desktop`; the target host does not):

```sh
git checkout <commit>
cargo build --locked --release -p interweave-transport-daemon \
    -p interweave-transportctl -p interweave-human-desktop -p interweave-human-cli
packaging/linux/install.sh target/release
```

Either way `install.sh` puts `transport-daemon`, `transportctl`,
`human-desktop` and `interweave-human` in `~/.local/bin` and the two unit
templates in `~/.config/systemd/user`, and enables nothing; it refuses,
installing nothing, when any of the four is missing. `interweave-human`
is the command-line human client; `human-desktop` is the desktop client
and is unused on a server, installed because the archive carries it. Put `~/.local/bin` on `PATH` if it is
not.

## 2. The identity

```sh
transportctl --profile <profile> identity init
```

mints the profile's key at `~/.local/share/interweave/profiles/<profile>/identity.key`
(owner-only) and prints its PeerId -- the name every other profile
allows and dials. The unit does not create a key: a daemon started
without one exits naming the path. The key is backed up in step 4,
once `config.yaml` exists.

## 3. `config.yaml`

At `~/.config/interweave/profiles/<profile>/config.yaml`, owned by the
account and writable by nobody else (the daemon refuses a config
directory or file another account can change). The shape, for a profile
reached by other profiles on the same host and by the Claude bridge:

```yaml
schema_version: 2
profile: { name: <profile> }
trust:
  policy: static-allowlist
  allowed_peers: ["<PeerId of each profile this one talks to>"]
endpoints:
  default_direct_endpoint: claude
  entries:
    - { id: claude, enabled: true, advertise: false, allowed_client_kinds: [claude-channel] }
channels: { desired: [] }
discovery:
  providers:
    - type: static-bootstrap
      enabled: true
      priority: 5
      config:
        peers: ["/ip4/127.0.0.1/tcp/<their port>/p2p/<their PeerId>"]
transport: { listen: { addresses: ["/ip4/127.0.0.1/tcp/<this profile's port>"] } }
observability: { log_level: info }
```

- **Listen.** `127.0.0.1` keeps the profile on this host: other
  profiles here reach it, nothing off the host does. A profile other
  hosts must reach listens on an interface address instead
  (`/ip4/0.0.0.0/tcp/<port>` for all of them) and is reachable as far as
  the host's firewall allows; reaching it across NAT is AutoNAT, relay
  and hole punching, configured under `transport.connectivity` and not
  covered here. An empty `listen` list starts a daemon that accepts no
  connection.
- **Endpoints.** One row per local client. The bridge claims the row
  that admits `claude-channel`; a desktop client on the same profile
  needs a row admitting `human-client`. A client claims one row at a
  time, and a second client of the same row is refused while the first
  holds it.
- **Bootstrap.** Each `static-bootstrap` entry names a peer by address
  and PeerId (`/p2p/<PeerId>` is required). A loopback address is
  admitted here because the operator wrote it; one a peer advertises is
  not (ADR-0052). Entries written here stay the operator's: nothing at
  run time removes them. A running daemon also takes peers by address
  (ADR-0062): `transportctl --profile <profile> peers add
  <address>/p2p/<PeerId>`, `peers remove` and `peers addresses`, kept in
  the state directory's `bootstrap-overlay.json` and effective without
  a restart. It refuses a loopback or other special-use address, so a
  peer on this host stays an entry here; a peer on another host is
  better added at run time. If the provider is written here at all, write it `enabled:
  true`: a disabled one refuses every run-time add.
- **Trust.** `allowed_peers` is where trust starts. It changes at run
  time with `transportctl --profile <profile> trust allow|revoke <PeerId>`,
  which the daemon keeps in its state directory and never writes into
  this file.
- **Sockets** need no configuration: the data socket is
  `$XDG_RUNTIME_DIR/interweave/<profile>.sock` and the admin socket
  `<profile>.admin.sock` beside it, both owner-only. A profile name is
  at most 64 bytes, and under `/run/user/<uid>` every such name fits a
  socket address (108 bytes); only an unusually long
  `XDG_RUNTIME_DIR` makes the bind fail.

The examples under `architecture/config/examples/` listen on
`0.0.0.0:4001`; copied unchanged onto a host with more than one profile,
they all ask for the same port.

## 4. Back up the key, then start it

Back the key up as its recovery record before anything depends on this
PeerId:

```sh
transportctl --profile <profile> identity backup --to-file <new path>
```

The record is a secret; keep it off the host. This is the one window
for it: `backup` reads `config.yaml`, so it is refused before step 3,
and like every `identity` command it takes the profile's lock, so it is
refused while the daemon runs (stop it first:
`systemctl --user stop interweave-daemon@<profile>`).

```sh
systemctl --user daemon-reload
systemctl --user enable --now interweave-daemon@<profile>
systemctl --user status interweave-daemon@<profile>
transportctl --profile <profile> status
```

`enable` makes it start with the account's user manager -- at boot,
with lingering -- and is undone by `disable`. `status` exits 3 while the
daemon is not answering yet, naming which of "no daemon is running" and
"its admin socket does not answer" it found.

A changed `config.yaml` is applied by `systemctl --user restart
interweave-daemon@<profile>`; `SIGHUP` restarts it too. Neither keeps a
client's endpoint lease: every client reconnects.

## 5. Check it

```sh
transportctl --profile <profile> status
transportctl --profile <profile> peers list
transportctl --profile <profile> trust list
transportctl --profile <profile> endpoints list
```

`peers list` shows each allowed peer the daemon is connected to; a peer
in `allowed_peers` whose bootstrap entry is right and whose daemon runs
appears within seconds of both starting.

## 6. Logs

The daemon logs to standard error, which the user manager hands to the
journal:

```sh
journalctl --user -u interweave-daemon@<profile>          # -f to follow
```

The level is `observability.log_level` (`error`, `warn`, `info` -- the
default -- or `debug`); no environment variable changes it. Third-party
libraries' lines (libp2p and below) are capped at `warn`, and trust changes are logged at every
level. A refusal before the profile is loaded (a missing key, an
unreadable `config.yaml`) is a plain line on standard error, in the same
journal.

## 7. Next

- The Claude bridge on this profile: the plugin's own procedure
  (`packaging/claude-plugin/`), run by the Claude Code session of the
  same account, which gives it the same `XDG_RUNTIME_DIR`. Its
  `--profile` and `--endpoint` are this profile and the `claude` row.
  The plugin comes from the archive's `claude-plugin/` directory; from a
  source build, it is built by
  `cargo build --locked --release -p interweave-claude-channel` and
  `packaging/claude-plugin/build.sh <new directory> target/release/claude-channel`.
- An automated client: `interweave-client@<profile>` (`README.md`).
- Upgrading is `install.sh` again and a restart; uninstalling is
  `uninstall.sh`, which keeps every profile (`README.md`). A
  downgrade is not: a daemon records the profile format it wrote in
  the state directory, and an older build refuses a profile a newer
  one has raised, naming the newer format.

## What this procedure assumes, and where it stops

- **One host, one account per profile.** The profiles of different
  accounts are kept apart by their sockets, their directories and their
  ports; two accounts on one port is a failed start, retried.
- **The account's session.** Without lingering the units stop when the
  account's last session ends; without a user manager nothing here runs.
- **`After=network.target` does not order a user unit after the
  network**: a user manager started at boot may start the daemon before
  an interface address exists. A loopback listener does not depend on
  one; a daemon listening on an interface address that is not up yet
  fails its start and its unit retries every 5 s until it is.
- **Not a shared home.** One profile on two hosts at once, as an NFS
  home allows, is refused only where the NFS server grants locks
  (`README.md`).
- **Reaching other hosts** is not covered: a listen address on an
  interface, the firewall, and the connectivity block for NAT. What has
  been measured across hosts is SPIKE-004's record, with its limits.
