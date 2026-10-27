<!-- SPDX-License-Identifier: Apache-2.0 -->
# Host run: another account's daemon refused a held port, 2026-10-27

ADR-0055 decision 5 rested on the kernel's documented `SO_REUSEPORT`
rule: two sockets share a port only under the same effective uid. So
another account's daemon is refused the port this one holds. This run
measures that between two accounts on one host, each with its own user
manager, using `DEPLOYING.md`'s unit and `config.yaml`. It is job j125,
asked by architect-cto in REQUEST 01a1722a. The second account was
another agent's, by fabric-coordinator's route (REPLY 01a17c3d: one
agent per account, no shell into another uid), the owner's choice.

Uid 1016's lines below are this account's own commands and their
output. Uid 1022's are as that account's agent reported them over
GZCoord (REPLY 01a17c44 and 01a17c46), quoted, not re-run here.

- Accounts: uid 1016 (`p2p-network-dev-01`) held the port; uid 1022
  (`p2p-network-dev-02`) tried it, each in its own user manager (uid
  1022's unit runs "under user@1022.service", its status said).
- Build: uid 1016 ran `~/.local/bin/transport-daemon --version`, which
  printed `transport-daemon 0.1.0+7196157887c2`. Uid 1022 reported
  `transportctl 0.1.0+7196157887c2`. Both are the v0.1.0 tag's build.
- Release archive, uid 1016's copy, checked against its published
  checksum file (run after the measurement, the same day):

  ```
  $ cat interweave-0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
  77fcc622473016a5232bf10783c67581b8404cdeb910a9cd29855412c67e5115  interweave-0.1.0-x86_64-unknown-linux-gnu.tar.gz
  $ sha256sum -c interweave-0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
  interweave-0.1.0-x86_64-unknown-linux-gnu.tar.gz: OK
  ```

  Uid 1022 reported its own download hashing to the same line. It did
  not extract the archive or run `install.sh`: its installed programs
  serve its `gzcoord` units.
- The configuration, uid 1016's file. Uid 1022 reported using it
  verbatim under its own profile name:

  ```yaml
  schema_version: 2
  profile: { name: j125-hold }
  trust: { policy: static-allowlist, allowed_peers: [] }
  endpoints:
    entries:
      - { id: probe, enabled: true, advertise: false, allowed_client_kinds: [probe] }
  channels: { desired: [] }
  discovery: { providers: [] }
  transport: { listen: { addresses: ["/ip4/127.0.0.1/tcp/47990"] } }
  observability: { log_level: info }
  ```

- Profiles `j125-hold` (uid 1016) and `j125-try` (uid 1022), each run as
  `interweave-daemon@<profile>` with `systemctl --user start`, never
  `enable`.
- Uid 1016's `j125-hold` config, state and data directories were
  removed after the run; what is left of uid 1016's profiles is the
  fleet's:

  ```
  $ ls ~/.config/interweave/profiles/ ~/.local/state/interweave/profiles/ ~/.local/share/interweave/profiles/
  ~/.config/interweave/profiles/:
  gzcoord

  ~/.local/share/interweave/profiles/:
  gzcoord

  ~/.local/state/interweave/profiles/:
  gzcoord
  ```

- Port 47990 was chosen as one no profile on the host uses. The
  `gzcoord` ports were left alone on purpose: had the rule failed, a
  second daemon there would have split the fleet's traffic. Uid 1016's
  `systemctl --user is-active interweave-daemon@gzcoord` printed
  `active` after `j125-hold` started and again after it stopped. Uid
  1022 reported its `@gzcoord` daemon and client active before and after.

## The refusal: uid 1022 against the port uid 1016 holds

`j125-hold` was listening first:

```
$ ss -ltnH 'sport = :47990'
LISTEN 0      1024   127.0.0.1:47990 0.0.0.0:*
```

`j125-try` was started at 07:29:24Z and read 30 s later. Each start
exits 1 and the unit retries, the restart counter reaching 5 within
the 30 s (local times are CET):

```
08:29:46 Scheduled restart job, restart counter is at 4.
08:29:47 INFO transport_daemon::daemon: starting profile="j125-try" peer="12D3KooW…"
08:29:47 transport-daemon: the runtime: substrate: transport: Address already in use (os error 98)
08:29:47 Main process exited, code=exited, status=1/FAILURE
08:29:52 Scheduled restart job, restart counter is at 5.
```

```
Active: activating (auto-restart) (Result: exit-code)
Process: … ExecStart=…/transport-daemon --profile j125-try (code=exited, status=1/FAILURE)
```

One socket on the port, owned by the holder:

```
$ ss -ltnHe 'sport = :47990'      # read by uid 1022
LISTEN 0 1024 127.0.0.1:47990 0.0.0.0:* uid:1016 … interweave-daemon@j125-hold.service
```

## The control: the port freed, the same daemon takes it

`j125-hold` was stopped at 07:30:55Z. The next retry of `j125-try`
bound the port:

```
08:30:51 transport-daemon: the runtime: substrate: transport: Address already in use (os error 98)
08:30:51 Main process exited, code=exited, status=1/FAILURE
08:30:56 Scheduled restart job, restart counter is at 17.
08:30:56 starting profile="j125-try" peer="12D3KooW…"
08:30:56 INFO transport_daemon::daemon: serving data=/run/user/1022/interweave/j125-try.sock
```

```
$ ss -ltnHe 'sport = :47990'
LISTEN 0 1024 127.0.0.1:47990 0.0.0.0:* uid:1022 … interweave-daemon@j125-try.service
```

So the refusal was the port held by the other uid, not a fault of the
profile or the unit: the same daemon, with the same configuration,
listened once the port was free.

## What this does not show

- Uid 1022's half re-run by this account: it is quoted as reported.
- Lingering for either account: not shown here.
- The cleanup of uid 1022's profile: it reported its directories
  removed, which is not re-checked here.

- A fresh install from the archive in an empty account: `install.sh`
  was not run, by the reason above. Both accounts' copies of the archive
  hashed to its published checksum (uid 1022's as it reported).
- Two accounts on two hosts, or a host other than this one (Fedora,
  this kernel).
- A long refusal: the counter reached 17 in about 90 s without the
  unit giving up. At `RestartSec=5` the unit stays under systemd's
  default start limit (5 starts in 10 s); nothing longer was watched.
- Two profiles of ONE account on one port. They share it silently, as
  ADR-0055 decision 5 says, pinned by
  `tests/automated-client`'s `two_daemons_of_one_account_share_a_port`.
