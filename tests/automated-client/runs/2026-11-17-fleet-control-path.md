# The daemon as the fleet's only control path — 2026-11-17

Run by p2p-network-dev-01 on develop-qzapp for architect-cto's REQUEST
01a1e8b4 (j174): thirty peers answering one `fabric-ctl all` on one
`fabric-control` endpoint, and how soon a peer is reached again after its
daemon restarts, on v0.3.0 and on main. Each run's own record is in
`2026-11-17-fleet-control-path/`, as the case wrote it, with scratch
paths and PeerIds replaced by placeholders.

## Programs

- **v0.3.0**: the released Linux archive,
  `interweave-0.3.0-x86_64-unknown-linux-gnu.tar.gz` from the v0.3.0
  release, its `.sha256` checked; `transport-daemon --version` prints
  `0.3.0+c0bbe40a9bec`.
- **main**: a release build (`cargo build --release -p
  interweave-transport-daemon -p interweave-transportctl`) at 2b27f3ab4.
  That commit is main at 773b2c685 plus test-only commits from this
  branch. `--version` prints `0.3.0+2b27f3ab4537`. The 25 commits main
  has gained since then touch no file under `crates/transport`,
  `crates/local`, `crates/config` or `apps/transport-daemon`. One fan-in
  run used the workspace's debug build as well
  (`fanin-main-debug.txt`). It is kept for its loss figure only; its
  timings are a debug build's.
- **The harness**: this branch's `tests/automated-client`, with
  `INTERWEAVE_PACKAGE_BIN` naming the `bin/` of each build. The client
  library and the IPC crates are the same at v0.3.0 and at main, so one
  harness drives both.

```sh
# fan-in, once per build (PKG = the archive's bin/, or target/release)
INTERWEAVE_PACKAGE_BIN=$PKG cargo test -p interweave-automated-client-tests \
  --test fan_in -- --ignored --nocapture --test-threads 1
# restart, once per build and variant
J174_B=silent|sends|entry J174_OUTAGE_S=0|100 [J174_DRAIN_MS=5 J174_DRAIN_LEAD_MS=300] \
INTERWEAVE_PACKAGE_BIN=$PKG cargo test -p interweave-automated-client-tests \
  --test two_hosts -- --exact a_restart_of_the_far_daemon_measured_from_its_return \
  --ignored --test-threads 1
```

The fan-in rounds and the four restart variants ran at 2b27f3ab4. The
eight repeats (`repeat-*`) ran at 69d3ae7bd, which only reads the drain
cadence from the environment. The drain runs (`drain-*`) ran at
2062778f7, where A's sends run on a task of their own across the
SIGTERM.

## 1. Fan-in: thirty sources, one endpoint

`tests/fan_in.rs`. 32 daemons on this host's loopback. One sink holds
`fabric-control` and `side`. Thirty sources each hold one static entry
for the sink, and a thirty-first sends to `side` twice a second, from
1 s before each round's window until 2 s after it. Each source sends k
direct messages of 32 768 bytes, one at a time, each answered before the
next. *Spread* places all 30·k sends evenly over 5 s, interleaved across
the sources. *Burst* sends back to back from the window's start. The
rounds run 20 s apart, so ADR-0026's buckets are full when each one
starts. Classes are what each sender's `send_direct` returned. "Lost" is
a message answered Accepted that the sink's client had not read within
30 s.

The sink's per-source pre-auth bucket was widened for this case (finding
F3).

| build | pace | k | sent | Accepted | Overloaded | senders refused | lost after Accepted | all read at | side: Overloaded / 17 |
|---|---|---|---|---|---|---|---|---|---|
| v0.3.0 | spread | 1 | 30 | 30 | 0 | 0 | 0 | 4.88 s | 0 |
| v0.3.0 | spread | 4 | 120 | 120 | 0 | 0 | 0 | 5.00 s | 0 |
| v0.3.0 | spread | 9 | 270 | 270 | 0 | 0 | 0 | 5.05 s | 0 |
| v0.3.0 | spread | 16 | 480 | 348 | 132 | 30 | 0 | 5.02 s | 3 |
| v0.3.0 | burst | 1 | 30 | 30 | 0 | 0 | 0 | 0.37 s | 0 |
| v0.3.0 | burst | 4 | 120 | 120 | 0 | 0 | 0 | 1.01 s | 0 |
| v0.3.0 | burst | 9 | 270 | 270 | 0 | 0 | 0 | 1.99 s | 0 |
| v0.3.0 | burst | 16 | 480 | 278 | 202 | 30 | 0 | 2.28 s | 1 |
| main | spread | 1 | 30 | 30 | 0 | 0 | 0 | 4.87 s | 0 |
| main | spread | 4 | 120 | 120 | 0 | 0 | 0 | 5.00 s | 0 |
| main | spread | 9 | 270 | 270 | 0 | 0 | 0 | 5.04 s | 0 |
| main | spread | 16 | 480 | 349 | 131 | 30 | 0 | 5.00 s | 4 |
| main | burst | 1 | 30 | 30 | 0 | 0 | 0 | 0.28 s | 0 |
| main | burst | 4 | 120 | 120 | 0 | 0 | 0 | 1.33 s | 0 |
| main | burst | 9 | 270 | 270 | 0 | 0 | 0 | 3.64 s | 0 |
| main | burst | 16 | 480 | 299 | 181 | 30 | 0 | 3.48 s | 1 |

"All read at" is measured from the window's start. Each run's record also
has the answer latency: p99 at most 323 ms on either release build.

- **The largest fan-in with no refusal is k = 9: 270 messages of 32 KiB
  from thirty peers inside 5 s**, on both builds and at both paces.
- **At k = 16, Overloaded and nothing else**, on both builds. Spread,
  about 350 of 480 are accepted: the global burst of 256 plus about 20 a
  second refilled over the 5 s (1200 a minute). Burst, 278–299 are
  accepted, since the refill has less time to act. No source reached its
  own bucket of 32. Every one of the thirty was refused 3 to 10 times.
- **Nothing answered Accepted was lost**, in any round of any build. No
  message was read without an Accepted, and no duplicate was read.
- **The other endpoint pays for the fan-in.** In each k = 16 round the
  `side` sender was answered Overloaded 1 to 4 times out of 17, though it
  sends two a second, far inside its own peer's bucket. The global
  bucket is the daemon's, not the endpoint's (ADR-0026). At k ≤ 9 it was
  never refused.

## 2. Reach after a restart

`tests/two_hosts.rs`, `a_restart_of_the_far_daemon_measured_from_its_return`.
Two daemons in two network namespaces joined by the overlay's link,
connected; A holds a static entry for B. B's daemon gets SIGTERM, stays
down `J174_OUTAGE_S` past its exit, and starts again with the same
identity and port. A's client sends to B's endpoint: through B's drain,
every 10 s through the outage, and every 250 ms from the moment B's
daemon serves until a send is accepted. "Reach" is the time from B
serving to A's first accepted send. B's client starts reopening its
session at B's exit and leased from 23 ms before to 122 ms after the
harness saw B serving. A negative figure means the lease came before the
harness's 50 ms poll of B's sockets.

The variants (`J174_B`):

- *silent*: B holds no entry for A and sends nothing.
- *sends*: B holds no entry, and its client sends to A as soon as it
  leases.
- *entry*: B holds an entry for A, so its daemon dials A as it starts,
  and its client sends to A as soon as it leases.

| build | variant | outage past exit | reach after B serves | A's backoffs (ms) |
|---|---|---|---|---|
| v0.3.0 | silent | 0 | 0.28 s | none |
| v0.3.0 | sends | 0 | 0.27 s (B's send: Accepted) | none |
| v0.3.0 | entry | 0 | 0 s | one 30 000, lifted by B's dial |
| v0.3.0 | silent | 100 s | **110.0 s** | 30 000, 60 000, 120 000 |
| main | silent | 0 | 0.29 s | none |
| main | sends | 0 | 30.1 s (B's send: PeerUnknown) | 30 000 |
| main | entry | 0 | 0 s | one 30 000, lifted by B's dial |
| main | silent | 100 s | **20.1 s** | 30 000 ×4 |

The quick restart, repeated eight times per build (*silent*, 0 s,
`repeat-*`). Reach was about 0.3 s (or 0 s) in 6 of 8 runs on v0.3.0 and
5 of 8 on main. The other runs took 29.9–30.2 s. In every drain run
below (12 of 12) it was 29.9–30.2 s.

- **Why a quick restart sometimes costs 30 s: A's own send dials B
  while B is stopping.** A send to a peer whose connection has just
  closed, and which is not yet held, dials it at once
  (`retry scheduled ... origin="manual"` in A's log, at the first send
  after the close). B is still stopping or not yet listening, so the
  dial fails and puts B under a 30 s backoff. Every send after that is
  refused at the gate (`dial failed class="PeerBackoff"`, answered
  PeerUnreachable at once) until the retry falls due. A redial on the
  close itself has the same race. Where nothing dials before B listens,
  reach is the next send. So on both builds the worst a 0.1–0.2 s
  restart costs is one backoff step, 30 s. It was paid in 6 of the 22
  quick restarts where A sent only from B's SIGTERM on, and in 12 of 12
  where A was already sending every 5 ms when the SIGTERM came.
- **Through a longer outage the builds part.** v0.3.0 doubles the backoff
  at each failed retry (30, 60, 120 s): after 100 s down, B was reached
  110 s after it returned. Main stays at 30 s: B was reached 20.1 s after
  it returned. v0.3.0's ceiling is 5 minutes, and an outage past about
  7.5 minutes waits up to 300 s; the 2026-11-14 record measured 272 s
  after a 480 s outage (`2026-11-14-long-outage.md`).
- **A's sends every 10 s through the outage did not lengthen the hold**
  on either build. Each was answered PeerUnreachable at once, and the
  backoff left on A's gate row counted down across it (the
  `A's hold left` column in `restart-*-silent-100s.txt`); only the
  retries moved it. The step at the *start* of an outage is different,
  as above: there a send is what dials.
- **B sending first reaches A only if B knows where A is.** With *entry*,
  B's start-up dial lifts A's backoff (an inbound from B resets it,
  `CONNECTIVITY.md` §12) and reach is 0 s on both builds. With *sends*
  and no entry, B's send to A succeeded when A's own dial had already
  connected (v0.3.0's run, B's send Accepted). It was PeerUnknown when it
  had not (main's run): a restarted daemon holds no address for a peer
  it has no entry for.

## 3. Refusals around B's drain and return

- **The drain is short.** SIGTERM to exit took 2–223 ms in these runs.
  The server closes the IPC sessions and releases the leases first, and
  then the runtime stops (the daemon's stop order, `daemon.rs`). From the
  SIGTERM, A's sends were answered PeerUnreachable, the connection
  already closed. In two runs they were answered RemoteEndpointUnavailable
  (main *sends* and repeat 1): the connection was still up, the lease
  already released.
- **A message answered Accepted just before B's SIGTERM can be lost.**
  In the drain runs (`drain-*`), A sent every 5 ms from 300 ms before the
  SIGTERM, and B's client read every 2 ms until the exit. In 6 of 12 runs
  (3 on v0.3.0, 3 on main), one of the 18–31 messages answered Accepted in
  those 300 ms never reached B's client. The same happened once in the
  first eight restart runs (main, silent 100 s: the one send answered
  Accepted, at the SIGTERM). This is not a breach of the contract:
  AcceptedV2 means bounded admission to the remote endpoint's queue, not
  delivery. But it is a loss after Accepted with no refusal to retry on,
  and the fleet's control link sends once. The mechanism is not verified
  at the line level. The record shows that the lost message was admitted
  while B's lease was held, and that B's session closed before its client
  read it.
- **RemoteEndpointUnavailable after B's return lasts until B's client
  leases.** B's client was retrying from B's exit. A send that reached B
  before the lease was answered RemoteEndpointUnavailable. In 12 of the
  16 quick restarts that reached B at once, A's first send at B's return
  (+0 ms) got that answer, and the next one, 260–290 ms later, was
  Accepted. In the other 4 the first send was Accepted. No send after
  the first was answered RemoteEndpointUnavailable. With the client
  re-opening at once, the window is under the 250 ms probe interval
  here.

## Not measured

- **Two hosts.** The fan-in ran on one host's loopback, all 32 daemons on
  its 10 cores, so the figures are what the sink admits with no network
  latency. The restart ran in two namespaces of one kernel.
- **The fan-in under the default pre-auth limits.** At the defaults the
  31st start from one address in a minute is refused before its handshake
  (F3), so the fan-in could not run there with every source on loopback.
- **A restart that changes B's address or port**, and a far end killed
  rather than stopped (no SIGTERM, no session close).
- **The fleet's actual configuration** of entries per account. A peer
  holding an entry for the coordinator is the *entry* row. One holding
  none is *silent* or *sends*.

## Findings, for architect-cto (proposals, nothing built)

- **F1. Accepted-then-lost at a stop.** A daemon that has been told to
  stop keeps admitting inbound direct messages into an endpoint queue
  whose session it is about to close. Options: refuse inbound direct from
  the stop's start (RemoteEndpointUnavailable or ShuttingDown, which a
  sender can retry), or deliver what the queue holds before the session's
  close.
- **F2. A send dials a peer that has just closed, and a failure costs
  30 s.** A peer that restarts in 0.1 s waits a full backoff step because
  the first send after the close dialled it mid-stop. Options (ADR-0011's
  ground): a shorter first step after a connection that closed cleanly,
  or no send-triggered dial for a short time after a clean close, leaving
  it to the scheduled retry.
- **F3. The pre-auth per-source bucket is a host's, and it holds 30
  starts a minute.** Every account on one host dials from the same
  address. On a two-host fleet the coordinator's daemon sees the other
  host's fifteen accounts in one bucket, and a mass restart that redials
  can pass 30 starts in a minute. Over the bucket, a start is refused
  before its handshake, and that peer then backs off. The limit is a
  profile setting (`transport.pre_auth.max_attempts_per_source_bucket_per_minute`,
  up to 600; this case set 120 on the sink). The coordinator's profile
  should carry a value fitting its fleet.
- **F4. 30 peers on one control endpoint fit inside ADR-0026 up to nine
  32 KiB parts each per 5 s.** At sixteen the global bucket refuses
  Overloaded, retryable under the same id, and the refusal spills onto
  every other endpoint of the daemon. No daemon change is needed for
  k ≤ 9. Past it, the choices are the sender's (retry Overloaded) or an
  ADR-0026 change, which is architect-cto's to weigh.
