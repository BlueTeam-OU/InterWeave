# The stop order and the loss ladder, measured again — 2026-11-18

Run by p2p-network-dev-01 on develop-qzapp for architect-cto's REQUEST
01a1ea5f (j176): the drain runs and the quick restarts of
`2026-11-17-fleet-control-path.md` §2 and §3, made again on the build
that carries F1 (the stop order, `LOCAL-IPC.md`'s shutdown order) and F2
(the loss ladder, ADR-0011 A 2026-11-17). Each run's own record is in
`2026-11-18-stop-order-and-ladder/`, as the case wrote it, with scratch
paths and PeerIds replaced by placeholders.

## Programs and harness

- **Daemons**: this branch's `target/release` (`transport-daemon`,
  `transportctl`), built at 50612ce47.
- **Harness**: `tests/two_hosts.rs`'s
  `a_restart_of_the_far_daemon_measured_from_its_return`, two daemons in
  two network namespaces over the overlay's link, as on 2026-11-17.
  The first series ran it as it was at 50612ce47. The second ran it with
  two additions, made after the first series and before the second:
  - `J174_HOLD_S`: how long the connection holds before B's `SIGTERM`.
    The 2026-11-17 runs stopped B about a second after connecting, so
    the connection lost was always younger than the ladder's 30 s floor.
  - After B's exit, the case reads what B's client library already held
    until its session reports the end, and prints a line of its own
    ("accepted and never read, counting what B's client held at the
    exit"). The line the 2026-11-17 record measured ("of them not read
    by B's client") is unchanged beside it.

```sh
J174_B=silent|sends J174_OUTAGE_S=0|100 [J174_DRAIN_MS=5] [J174_DRAIN_LEAD_MS=300] \
[J174_HOLD_S=35] INTERWEAVE_PACKAGE_BIN=$PWD/target/release \
cargo test -p interweave-automated-client-tests --test two_hosts -- --exact \
  a_restart_of_the_far_daemon_measured_from_its_return --ignored --test-threads 1
```

## 1. Loss after Accepted (F1)

- **The drain runs** (`drain-1`..`drain-12`; `J174_B=silent
  J174_OUTAGE_S=0 J174_DRAIN_MS=5 J174_DRAIN_LEAD_MS=300`, first series):
  **0 of 12 lost a message answered Accepted.** 351 messages were
  accepted before the `SIGTERM` across the twelve, every one read by B's
  client. On 2026-11-17, 6 of 12 lost one. SIGTERM to exit took 3–119 ms.
  No send was answered from B's drain in these runs: B's client reads,
  so the hand-over is a few milliseconds, and A's sends after the
  `SIGTERM` met the closed connection (`PeerUnreachable`).
- **The no-lead repeats** (`repeat-1`..`repeat-8`, first series; A sends
  from the `SIGTERM` alone, B's client reads only from it): 4 of 8 show
  one message "not read by B's client". Run again with the after-exit
  read (`repeat-read-1`..`repeat-read-8`): that line shows one in 1 of
  8 (B exited 2 ms after its `SIGTERM`), and **counting what B's client
  library held at B's exit, 0 of 8 lost one**. In each such run B exited
  within 2–9 ms of its `SIGTERM`; the message had been handed to B's
  client before the `close`, and the case had stopped asking for it at
  B's exit. So the earlier line is the harness's, not a
  loss.
- **Every other run** of both series lost nothing, by either line.
- The refusal itself (`shutting_down` answered from the stop's start)
  is held by `tests/automated_client.rs`'s
  `an_inbound_direct_after_the_stop_began_is_refused_shutting_down`,
  which keeps B in its hand-over with a client that does not read; these
  runs do not show it, for the reason above.

## 2. Reach after a quick restart (F2)

"Reach" is the time from B serving again to A's first accepted send,
as on 2026-11-17.

| runs | connection held before the stop | reach after B serves | A's first retry step |
|---|---|---|---|
| `repeat-read-1`..`8` | about 1 s | 0–0.28 s in 2 of 8; 29.9–30.1 s in 6 of 8 | 30 000 ms |
| `held-1`..`8` | 35 s | 0.26 s in 3 of 8; 1.01–1.04 s in 5 of 8 | 1 000 ms |
| `held-sends-1` | 35 s | 0.26 s | none |
| `held-silent-100s` (B down 100 s) | 35 s | 21.2 s | 1 000, 2 000, 4 000 ms, ... |
| `silent-100s` (first series) | about 1 s | 19.9 s | 30 000 ms |

- **After a connection that held, a quick restart costs at most the
  ladder's first step**: 1.0 s where A's first dial met B stopping, 0.26 s
  where it did not. On 2026-11-17 the same race cost 29.9–30.2 s.
- **A connection younger than the floor still costs 30 s** in the same
  race (6 of 8 here), as ruled: the ladder does not start. In this
  harness that is every case without `J174_HOLD_S`. The fleet's
  connection to a daemon normally holds far longer than 30 s.
- **Through a 100 s outage** the ladder is spent in its first 31 s and the
  30 s steps follow: B was reached 21.2 s after its return, against
  20.1 s on main on 2026-11-17. The ladder does not change a long outage.

## What this does not show

One kernel and one host, as `two_hosts.rs` says; no phone; no peer
that accepts and drops (the floor's own case is held by the unit and
socket tests, `loss_ladder.rs` and `crates/transport/libp2p/tests/
reconnect.rs`); and a far daemon stopping with a client that does not
read, which only `tests/automated_client.rs` exercises.
