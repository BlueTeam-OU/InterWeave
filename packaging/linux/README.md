# linux

Linux desktop/server packaging, service integration and permissions.

## systemd user units

Two templates, each instantiated on a profile name (plan §28 step 4,
ADR-0055 decision 6):

| unit | runs | after a failure |
|---|---|---|
| `interweave-daemon@<profile>.service` | `~/.local/bin/transport-daemon --profile <profile>` | restarted 5 s later; exit 2 (a usage error) is not restarted; exit 75 (a stop asked by SIGHUP) is restarted 5 s later and logged as a success |
| `interweave-client@<profile>.service` | `~/.local/libexec/interweave/client --profile <profile>` | restarted 5 s later |

Both are **user** units: they run as the account that installs them, from
that account's own binaries, with nothing installed system-wide. A host
with many profiles under many accounts runs one daemon unit per account.

The daemon is stopped with `SIGTERM`, which it answers by stopping within
its own shutdown grace. `SIGHUP` is a restart, not a reload: the daemon
stops exactly as on `SIGTERM` and its unit starts it again `RestartSec=`
(5 s) later. Nothing survives a restart -- every client's endpoint lease
ends and the new daemon issues fresh epochs -- and there is no
`ExecReload=`: a changed `config.yaml` is applied by `systemctl --user
restart interweave-daemon@<profile>`. A daemon whose listen port another
account holds fails its start and its unit retries it. Give each profile
its own port: two daemons of one account on one port both start, because
the TCP transport listens with `SO_REUSEPORT` (ADR-0055 decision 5).

The client unit starts after its daemon and is stopped whenever its daemon
stops (`After=`, `BindsTo=`). Enabling it also makes it start whenever its
daemon starts, its daemon's own restarts included (`WantedBy=` the daemon
instance). **InterWeave ships no automated client.** The program is the
deployment's: install it at `~/.local/libexec/interweave/client`, or keep it
elsewhere and override `ExecStart=` in a drop-in
(`systemctl --user edit interweave-client@<profile>`). What the transport
promises such a client, and what the client owes itself, is
`architecture/clients/automated/`.

## Installing and uninstalling

`install.sh` and `uninstall.sh` are run as the account that will run the
profile, and write nothing outside its home: the three executables
(`transport-daemon`, `transportctl`, `human-desktop`) into `~/.local/bin`,
the two templates into `${XDG_CONFIG_HOME:-~/.config}/systemd/user`.
From the release archive, whose `bin/` and `systemd/` sit beside the
scripts:

```sh
./install.sh                                          # or ./install.sh <dir holding the three executables>
transportctl identity init --profile <profile>        # once: mints the key, prints the PeerId
# write the profile's config.yaml (see below), then, to run it at login:
systemctl --user enable --now interweave-daemon@<profile>
systemctl --user enable --now interweave-client@<profile>   # if this profile has an automated client
```

**Installing enables nothing.** A profile's daemon starting with the
account's user manager -- at login, or at boot with lingering -- is the
person's explicit choice, made by `enable` above and undone by `disable`.
An upgrade is the same `install.sh` over the old one: each file is
replaced by a rename, so a running daemon keeps the binary it started
with until it is restarted.

`./uninstall.sh` stops and disables every `interweave-daemon@` and
`interweave-client@` instance of the account, then removes the two
templates and the three executables. **It never removes a profile**:
every profile's `config.yaml`, identity key, state, human store and logs
stay where they are, and a reinstall serves the same PeerId from the kept
key. A removed key is a lost PeerId, recoverable only from its recovery
mnemonic, so deleting a profile is your own act, by hand. A profile lives
in four directories, each `interweave/profiles/<profile>` under its XDG
root: the configuration under `$XDG_CONFIG_HOME` (`~/.config`), the key
under `$XDG_DATA_HOME` (`~/.local/share`), the state and the human
client's store under `$XDG_STATE_HOME` (`~/.local/state`), and the peer
cache under `$XDG_CACHE_HOME` (`~/.cache`).

Without `loginctl enable-linger <account>`, the account's units stop when
its last login session ends; a profile that must stay up enables lingering
(whether an account may do this for itself is the host's polkit policy).

**Never run one profile on two hosts at once**, as a home shared over NFS
would allow. The daemon's profile lock is `flock(2)`, which Linux carries
over NFS as a byte-range lock granted by the server's lock manager, so it
excludes a second host only when the server grants locks; a home mounted
`nolock` or `local_lock=flock` makes the lock host-local, and two hosts
would both serve one PeerId.

## The release archive

A release ships these files as
`interweave-<version>-x86_64-unknown-linux-gnu.tar.gz`, built by
`tools/ci/release_linux_archive.sh`: one directory holding `bin/` (the
three programs), `systemd/` (the two templates), `install.sh`,
`uninstall.sh`, this README, `LICENSE` and `THIRD-PARTY-NOTICES.md`, the
licence of every crate compiled into the programs. The archive has no
signature; its SHA-256 is on the release page and in the `.sha256` file
beside it, and the same sources and `SOURCE_DATE_EPOCH` give the same
bytes. The first build of it is `evidence/2026-10-20-release-archive.md`.

A deployment of many profiles renders each profile's `config.yaml` from its
own record of who is who and installs the units with its own tooling
instead (ADR-0055 decisions 4 and 5); nothing here has to be done by hand.
