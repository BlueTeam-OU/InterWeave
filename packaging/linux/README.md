# linux

Linux desktop/server packaging, service integration and permissions.

## systemd user units

Two templates, each instantiated on a profile name (plan §28 step 4,
ADR-0055 decision 6):

| unit | runs | after a failure |
|---|---|---|
| `interweave-daemon@<profile>.service` | `~/.local/bin/transport-daemon --profile <profile>` | restarted 5 s later; exit 2 (a usage error) is not restarted |
| `interweave-client@<profile>.service` | `~/.local/libexec/interweave/client --profile <profile>` | restarted 5 s later |

Both are **user** units: they run as the account that installs them, from
that account's own binaries, with nothing installed system-wide. A host
with many profiles under many accounts runs one daemon unit per account.

The daemon is stopped with `SIGTERM`, which it answers by stopping within
its own shutdown grace. A daemon whose listen port another account holds
fails its start and its unit retries it. Give each profile its own port:
two daemons of one account on one port both start, because the TCP
transport listens with `SO_REUSEPORT` (ADR-0055 decision 5).

The client unit starts after its daemon and is stopped whenever its daemon
stops (`After=`, `BindsTo=`). Enabling it also makes it start whenever its
daemon starts, its daemon's own restarts included (`WantedBy=` the daemon
instance). **InterWeave ships no automated client.** The program is the
deployment's: install it at `~/.local/libexec/interweave/client`, or keep it
elsewhere and override `ExecStart=` in a drop-in
(`systemctl --user edit interweave-client@<profile>`). What the transport
promises such a client, and what the client owes itself, is
`architecture/clients/automated/`.

## Installing them by hand

As the account that will run the profile:

```sh
install -Dm755 target/release/transport-daemon ~/.local/bin/transport-daemon
install -Dm755 target/release/transportctl ~/.local/bin/transportctl
install -Dm644 -t ~/.config/systemd/user packaging/linux/systemd/interweave-*@.service
systemctl --user daemon-reload
transportctl identity init --profile <profile>     # once: mints the key, prints the PeerId
# write the profile's config.yaml (see below), then:
systemctl --user enable --now interweave-daemon@<profile>
systemctl --user enable --now interweave-client@<profile>   # if this profile has an automated client
```

Without `loginctl enable-linger <account>`, the account's units stop when
its last login session ends; a profile that must stay up enables lingering
(whether an account may do this for itself is the host's polkit policy).

A deployment of many profiles renders each profile's `config.yaml` from its
own record of who is who and installs the units with its own tooling
instead (ADR-0055 decisions 4 and 5); nothing here has to be done by hand.
