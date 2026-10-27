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

- Build: both accounts ran the installed v0.1.0 programs,
  `0.1.0+7196157887c2`, the tag's build. Each account runs the fleet's
  `gzcoord` profile on it too, untouched throughout: both `@gzcoord`
  units were active before and after.
- Release archive: the second account downloaded
  `interweave-0.1.0-x86_64-unknown-linux-gnu.tar.gz` and its sha256 was
  `77fcc622473016a5232bf10783c67581b8404cdeb910a9cd29855412c67e5115`,
  the published `.sha256`. It was not extracted and `install.sh` was
  not run, since its programs are the ones the account's `gzcoord`
  units use.
- Accounts: uid 1016 (`p2p-network-dev-01`) held the port; uid 1022
  (`p2p-network-dev-02`) tried it. Both are lingering and run their own
  `user@<uid>.service`.
- Profiles: `j125-hold` (uid 1016) and `j125-try` (uid 1022), one
  `config.yaml` shape for both: no allowed peer, one endpoint, no
  discovery provider, `listen: ["/ip4/127.0.0.1/tcp/47990"]`. Each
  ran as `interweave-daemon@<profile>`, started, never enabled, and
  was removed after the run with its config, state and data
  directories.
- Port 47990 was chosen as one no profile on the host uses. The
  `gzcoord` ports were left alone on purpose: had the rule failed, a
  second daemon there would have split the fleet's traffic.

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

- A fresh install from the archive in an empty account: `install.sh`
  was not run, by the reason above. The archive's hash was checked.
- Two accounts on two hosts, or a host other than this one (Fedora,
  this kernel).
- A long refusal: the counter reached 17 in about 90 s without the
  unit giving up. At `RestartSec=5` the unit stays under systemd's
  default start limit (5 starts in 10 s); nothing longer was watched.
- Two profiles of ONE account on one port. They share it silently, as
  ADR-0055 decision 5 says, pinned by
  `tests/automated-client`'s `two_daemons_of_one_account_share_a_port`.
