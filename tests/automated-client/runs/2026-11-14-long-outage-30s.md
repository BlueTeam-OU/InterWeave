# A long outage between two hosts, on the 30 s bound — 2026-11-14

`tests/automated-client/tests/two_hosts.rs`,
`after_a_long_outage_the_side_with_the_entry_reconnects_within_30_s_of_the_return`,
run by p2p-network-dev-01 on develop-qzapp at commit 41f94bcd6 (#674's
branch with ADR-0011 A 2026-11-14 built and its supply folded), the
same case and the same 480 s outage as `2026-11-14-long-outage.md`, on
the code that brings the peer retry to a flat 30 s and wakes the
scheduler at the earliest due retry. The case is `#[ignore]`d: it is an
outage of eight minutes.

```sh
cargo build -p interweave-transportctl -p interweave-transport-daemon
cargo test -p interweave-automated-client-tests --test two_hosts -- \
  --exact after_a_long_outage_the_side_with_the_entry_reconnects_within_30_s_of_the_return \
  --ignored --test-threads 1
```

## What ran

As in the first record: two daemons in two network namespaces joined
by the overlay's link, only A holding a static-bootstrap entry for B;
connected, then B's DAEMON stopped with the link up, so A's dials are
refused in B's namespace and nothing changes in A's network; 480 s
later B started again, same identity, same port.

## Output

The test's output and its record, as printed, with one substitution:
B's PeerId as `<B>`.

```text
running 1 test
test after_a_long_outage_the_side_with_the_entry_reconnects_within_30_s_of_the_return ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 481.52s


outage 480s, reconnected 955.297111ms after B's return
retry delays during the outage (ms): [30000, 30000, 30000, 30000, 30000, 30000, 30000, 30000, 30000, 30000, 30000, 30000, 30000, 30000, 30000, 30000]
reconnect refusals during the outage: 1
-- A's dial lines during the outage --
2026-11-14T18:52:07.025939Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:52:37.028471Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:53:07.030619Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:53:37.032111Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:54:07.034437Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:54:37.038081Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:55:07.040124Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:55:37.042373Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:56:07.052206Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:56:37.055586Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:57:07.061698Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:57:37.062472Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:58:07.063991Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:58:37.067961Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:59:07.072636Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:59:37.076327Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T18:52:07.025990Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="discovery_reconnect" attempt=1 delay_ms=30000 peer_backoff=true
2026-11-14T18:52:37.028513Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=2 delay_ms=30000 peer_backoff=true
2026-11-14T18:53:07.030672Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=3 delay_ms=30000 peer_backoff=true
2026-11-14T18:53:37.032152Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=4 delay_ms=30000 peer_backoff=true
2026-11-14T18:54:07.034488Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=5 delay_ms=30000 peer_backoff=true
2026-11-14T18:54:37.038154Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=6 delay_ms=30000 peer_backoff=true
2026-11-14T18:55:07.040177Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=7 delay_ms=30000 peer_backoff=true
2026-11-14T18:55:37.042427Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=8 delay_ms=30000 peer_backoff=true
2026-11-14T18:56:07.052239Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=9 delay_ms=30000 peer_backoff=true
2026-11-14T18:56:37.055654Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=10 delay_ms=30000 peer_backoff=true
2026-11-14T18:57:07.061785Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=11 delay_ms=30000 peer_backoff=true
2026-11-14T18:57:37.062523Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=12 delay_ms=30000 peer_backoff=true
2026-11-14T18:58:07.064028Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=13 delay_ms=30000 peer_backoff=true
2026-11-14T18:58:37.068014Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=14 delay_ms=30000 peer_backoff=true
2026-11-14T18:59:07.072690Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=15 delay_ms=30000 peer_backoff=true
2026-11-14T18:59:37.076366Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=16 delay_ms=30000 peer_backoff=true
-- A's dial and connect lines after B's return --
2026-11-14T19:00:07.100636Z DEBUG interweave::connectivity: peer connected peer="<B>" path="direct"
```

## Findings

- **A redials every 30 s through the outage, however many failed.**
  Sixteen failures, each `retry scheduled ... delay_ms=30000`, attempts
  1 to 16; consecutive failures 30.002 to 30.010 s apart, read from the
  timestamps above. The ladder this replaced made five dials in the
  same 480 s, the last at its 300 s ceiling.
- **The scheduler's dial is the one that runs.** After the first
  failure (the discovery round's reconnect on the close) every retry is
  `origin="connection_manager"`: the scheduler wakes at the due retry
  and dials before the discovery round's next second; one reconnect
  refusal in 480 s, against four on the old code.
- **The pair reconnected 0.96 s after B's return** (19:00:07.101, the
  retry the 18:59:37.076 failure scheduled, 30.024 s after it). B's
  return happened to land a second before that retry fell due, so this
  run shows the cadence and not the worst case. The worst case, a
  return just after a failure, waits the whole 30 s; it is pinned
  without a person by `crates/transport/libp2p/tests/reconnect.rs`'s
  `a_peer_down_past_several_steps_is_redialled_within_30_s_of_its_return`.

What the case asserts: at least fifteen retry delays, every one
30 000 ms; no gap between consecutive failures past 30.25 s; the
reconnect within a second of the last failure plus 30 s, read from A's
log; and the reconnect within 31 s of B's return.

## Not measured

As in the first record: a far end that returns on another address or
port, one that returns and dials A itself, and a real network outage
rather than a stopped daemon. And the cost on a phone of a dial every
30 s to each absent contact, carried by name in `CONNECTIVITY.md` §12.
