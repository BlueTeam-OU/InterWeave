<!-- SPDX-License-Identifier: Apache-2.0 -->
# Host run: the gzcoord units' stop and SIGKILL restart, and a port another account holds, 2026-11-04

Plan §28 step 4's figures are a clean stop, the client stopping with
its daemon, the client starting with the daemon, and both units back
after a SIGKILL. They were measured by hand at f68b7b56 and no CI case
holds them. Its carried limit is the cross-account port refusal,
"until CI has a second account". This run measures all of them on the
fleet deployment's live units, in one account's own user manager. It is
job j144, asked by architect-cto in REQUEST 01a1a689.

Everything below is this account's own commands and their output,
printed by one script run at 13:28:58Z. Journal times are local (CET).
The home directory is shortened to `~`, PeerIds are cut after
`12D3KooW`, and the `ss` lines are cut after the unit name.

- Account: uid 1016 (`p2p-network-dev-01`), its own user manager
  (`systemd[2689]`), lingering on (`loginctl show-user` printed
  `Linger=yes`).
- Host: Fedora, kernel `6.17.9-1.qubes.fc37.x86_64`,
  `systemd 258 (258.11-1.fc43)`.
- Programs, the account's installed build, the ones the units run:
  `transport-daemon 0.2.0+32ade966cff8` and
  `transportctl 0.2.0+32ade966cff8`. The client is the deployment's
  program at `~/.local/libexec/interweave/client`. InterWeave ships no
  client.
- Units: `interweave-daemon@gzcoord` and `interweave-client@gzcoord`
  from `packaging/linux/systemd`, as installed in
  `~/.config/systemd/user/`. Both were `active` before the run.

## 1. A clean stop: 0.49 s, the client first

`systemctl --user stop interweave-daemon@gzcoord` returned after
0.513 s. The client is `BindsTo=` and `After=` its daemon, so systemd
stopped the client first, then the daemon:

```
14:28:58.652957 Stopping interweave-client@gzcoord.service - InterWeave automated client, profile gzcoord...
14:28:59.042557 Stopped interweave-client@gzcoord.service - InterWeave automated client, profile gzcoord.
14:28:59.044120 Stopping interweave-daemon@gzcoord.service - InterWeave transport daemon, profile gzcoord...
14:28:59.061940 transport-daemon[4049]: INFO transport_daemon::daemon: SIGTERM
14:28:59.061940 transport-daemon[4049]: INFO transport_daemon::daemon: stopping grace_ms=5000
14:28:59.114080 transport-daemon[4049]: INFO transport_daemon::daemon: stopped events_dropped=0 restart=false
14:28:59.146879 Stopped interweave-daemon@gzcoord.service - InterWeave transport daemon, profile gzcoord.
```

- The whole stop took 0.494 s, from the first `Stopping` to the last
  `Stopped`.
- The client took 0.390 s of that.
- The daemon took 0.103 s from `Stopping` to `Stopped`, 0.052 s of it
  from `SIGTERM` to `stopped`. Its unit's deactivating-to-inactive
  span read 0.102 s, `Result=success`.
- The client's `Result` was `success` too.

So the daemon's own stop is the 0.19 s order of f68b7b56's figure or
less. On this host the stop as a whole is longer: it waits for the
deployment's client to leave first.

## 1b. The start: the client with the daemon

`systemctl --user start interweave-daemon@gzcoord` named only the
daemon. The client came up after it, through its `WantedBy=`:

```
14:28:59.351736 Starting interweave-daemon@gzcoord.service - InterWeave transport daemon, profile gzcoord...
14:28:59.376806 Started interweave-daemon@gzcoord.service - InterWeave transport daemon, profile gzcoord.
14:28:59.392266 Starting interweave-client@gzcoord.service - InterWeave automated client, profile gzcoord...
14:28:59.402060 Started interweave-client@gzcoord.service - InterWeave automated client, profile gzcoord.
14:28:59.411138 transport-daemon[4126605]: INFO transport_daemon::daemon: starting profile="gzcoord" peer="12D3KooW…"
14:28:59.500605 transport-daemon[4126605]: INFO transport_daemon::daemon: serving data=/run/user/1016/interweave/gzcoord.sock
```

From the daemon's `Starting` line, the daemon was `Started` 0.025 s
later, the client 0.050 s, and the daemon was serving at 0.149 s. Every
figure here is from the journal lines quoted.

## 2. SIGKILL: both back in 5.3 s

`systemctl --user kill -s SIGKILL --kill-whom=main
interweave-daemon@gzcoord`:

```
14:29:03.491843 interweave-daemon@gzcoord.service: Sent signal SIGKILL to main process 4126605 (transport-daemo) on client request.
14:29:03.531072 interweave-daemon@gzcoord.service: Main process exited, code=killed, status=9/KILL
14:29:03.531245 interweave-daemon@gzcoord.service: Failed with result 'signal'.
14:29:03.754839 Stopping interweave-client@gzcoord.service - InterWeave automated client, profile gzcoord...
14:29:03.771764 Stopped interweave-client@gzcoord.service - InterWeave automated client, profile gzcoord.
14:29:08.643579 interweave-daemon@gzcoord.service: Scheduled restart job, restart counter is at 1.
14:29:08.696122 Starting interweave-daemon@gzcoord.service - InterWeave transport daemon, profile gzcoord...
14:29:08.716820 Started interweave-daemon@gzcoord.service - InterWeave transport daemon, profile gzcoord.
14:29:08.720739 Starting interweave-client@gzcoord.service - InterWeave automated client, profile gzcoord...
14:29:08.747629 transport-daemon[4129010]: INFO transport_daemon::daemon: starting profile="gzcoord" peer="12D3KooW…"
14:29:08.752211 Started interweave-client@gzcoord.service - InterWeave automated client, profile gzcoord.
14:29:08.765814 transport-daemon[4129010]: INFO transport_daemon::daemon: serving data=/run/user/1016/interweave/gzcoord.sock
```

- From the `Sent signal SIGKILL` line, the daemon was `Started` again
  5.225 s later, with `NRestarts=1`, the client 5.260 s, and the daemon
  was serving at 5.274 s. Every figure here is from the journal lines
  quoted.
- `BindsTo=` stopped the client 0.22 s after the daemon died.
  `Restart=on-failure` restarted the daemon `RestartSec=5` later. The
  client came back with it, both with new PIDs.
- Both were `active` after the run.

## 3. A port another account holds: refused, and the unit retries

The port was 47118, held by uid 1022's `gzcoord` daemon (the other
holder of this role, `p2p-network-dev-02`). A fresh profile `j144-try`
in this account was configured to listen on it:

```
$ ss -ltnHe 'sport = :47118'
LISTEN 0      1024   127.0.0.1:47118 0.0.0.0:* uid:1022 … interweave-daemon@gzcoord.service
```

```yaml
schema_version: 2
profile: { name: j144-try }
trust: { policy: static-allowlist, allowed_peers: [] }
endpoints:
  entries:
    - { id: probe, enabled: true, advertise: false, allowed_client_kinds: [probe] }
channels: { desired: [] }
discovery: { providers: [] }
transport: { listen: { addresses: ["/ip4/127.0.0.1/tcp/47118"] } }
observability: { log_level: info }
```

`transportctl --profile j144-try identity init` made its key. Then
`systemctl --user start interweave-daemon@j144-try` ran.

The profile trusts nobody, so a daemon that did take the port could
admit no peer. The script also watched the port for a socket of uid
1016 for 12 s, ready to stop the unit at once. None appeared: the only
listener was still uid 1022's. Each start exited 1 on `EADDRINUSE`, and
the unit retried every 5 s:

```
14:29:13.109812 transport-daemon[4130388]: INFO transport_daemon::daemon: starting profile="j144-try" peer="12D3KooW…"
14:29:13.125700 transport-daemon[4130388]: transport-daemon: the runtime: substrate: transport: Address already in use (os error 98)
14:29:13.133133 interweave-daemon@j144-try.service: Main process exited, code=exited, status=1/FAILURE
14:29:13.133500 interweave-daemon@j144-try.service: Failed with result 'exit-code'.
14:29:18.242961 interweave-daemon@j144-try.service: Scheduled restart job, restart counter is at 1.
14:29:18.264851 transport-daemon[4132071]: transport-daemon: the runtime: substrate: transport: Address already in use (os error 98)
14:29:18.310293 interweave-daemon@j144-try.service: Main process exited, code=exited, status=1/FAILURE
14:29:23.409214 interweave-daemon@j144-try.service: Scheduled restart job, restart counter is at 2.
14:29:23.487039 transport-daemon[4133585]: transport-daemon: the runtime: substrate: transport: Address already in use (os error 98)
14:29:23.501059 interweave-daemon@j144-try.service: Main process exited, code=exited, status=1/FAILURE
14:29:25.651925 Stopped interweave-daemon@j144-try.service - InterWeave transport daemon, profile j144-try.
```

```
Active: activating (auto-restart) (Result: exit-code) since Wed 2026-11-04 14:29:23 CET; 1s ago
```

## 3b. The control: the same profile on a free port listens

The same profile was then pointed at port 47990, which `ss` showed
free. The next start listened:

```
14:29:25.807183 transport-daemon[4134453]: INFO transport_daemon::daemon: starting profile="j144-try" peer="12D3KooW…"
14:29:25.910834 transport-daemon[4134453]: INFO transport_daemon::daemon: serving data=/run/user/1016/interweave/j144-try.sock
```

```
LISTEN 0      1024   127.0.0.1:47990 0.0.0.0:* uid:1016 … interweave-daemon@j144-try.service
```

So the refusal came from the port another uid held, not from the
profile, its key or the unit. The same daemon, configured the same way
except for the port, listened.

## Cleanup

The `j144-try` unit was stopped. Its config, state and data
directories were removed. What is left is the fleet's:

```
$ ls ~/.config/interweave/profiles/ ~/.local/state/interweave/profiles/ ~/.local/share/interweave/profiles/
~/.config/interweave/profiles/:
gzcoord

~/.local/share/interweave/profiles/:
gzcoord

~/.local/state/interweave/profiles/:
gzcoord
```

Both `gzcoord` units were `active` at the end, and uid 1022 still held
47118 alone.

## What this does not show

- **Whether the holder used `SO_REUSEPORT`.** `ss` does not print
  socket options. The holder is an InterWeave daemon, whose TCP
  transport listens with it (ADR-0055 decision 5). This account does
  not know the holder's build. It was not asked, and its version was
  not read.
- **The reverse direction.** Uid 1022 against a port uid 1016 holds is
  the 2026-10-27 record
  ([`2026-10-27-second-uid-port.md`](2026-10-27-second-uid-port.md)).
- **More than one run of each figure.** These are single samples on a
  loaded host. The SIGKILL figure is `RestartSec=5` plus the start.
- **A CI case.** None of this is one: it is a host record, as before.
- **What the deployment lost.** The `gzcoord` daemon was not serving
  for about 0.45 s across the stop and the start that followed it
  (`SIGTERM` at 14:28:59.062, `serving` at 14:28:59.501), and for 5.27
  s after the kill. What GZCoord
  traffic to this account was missed in those windows was not
  measured.
- **Reboot, lingering across logout, `enable`.** The units were only
  started and stopped here.
