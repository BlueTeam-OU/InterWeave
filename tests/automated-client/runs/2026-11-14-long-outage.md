# A long outage between two hosts — 2026-11-14

`tests/automated-client/tests/two_hosts.rs`,
`after_a_long_outage_the_side_with_the_entry_reconnects_within_its_backoff`,
run by p2p-network-dev-01 on develop-qzapp at commit 29e4d38cd (the commit
that added the case), for `transport/libp2p/CONNECTIVITY.md` §14's
not-proved item "an outage long enough for the dial backoff, 30 s
doubling to 5 min, to delay the reconnect". The case is `#[ignore]`d:
it is an outage of eight minutes.

```sh
cargo build -p interweave-transportctl -p interweave-transport-daemon
cargo test -p interweave-automated-client-tests --test two_hosts -- \
  --exact after_a_long_outage_the_side_with_the_entry_reconnects_within_its_backoff \
  --ignored --test-threads 1
```

## What ran

Two daemons in two network namespaces joined by the overlay's link
(`interweave_test_support::e2e::netns`), the fleet's shape: only A holds
a static-bootstrap entry for B. Connected; then B's DAEMON stopped
(SIGTERM) with the link left up, so A's dials reach B's namespace and
are refused there, and nothing changes in A's network; 480 s later B
started again, same identity, same port. The figures are the case's own
record, written beside the test binary; the case asserts them (below).

## Output

The test's output and its record, as printed, with one substitution:
B's PeerId as `<B>`.

```text
running 1 test
test after_a_long_outage_the_side_with_the_entry_reconnects_within_its_backoff ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 753.22s

outage 480s, reconnected 272.041085466s after B's return
retry delays during the outage (ms): [30000, 60000, 120000, 240000, 300000]
reconnect refusals during the outage: 4
-- A's dial lines during the outage --
2026-11-14T13:47:46.625097Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T13:48:17.616259Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T13:49:17.654147Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T13:51:17.665663Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T13:55:17.679948Z DEBUG interweave::connectivity: dial failed peer="<B>" class="dial_failed"
2026-11-14T13:47:46.625161Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="discovery_reconnect" attempt=1 delay_ms=30000 peer_backoff=true
2026-11-14T13:48:17.616296Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="connection_manager" attempt=2 delay_ms=60000 peer_backoff=true
2026-11-14T13:49:17.654186Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="discovery_reconnect" attempt=3 delay_ms=120000 peer_backoff=true
2026-11-14T13:51:17.665771Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="discovery_reconnect" attempt=4 delay_ms=240000 peer_backoff=true
2026-11-14T13:55:17.679992Z DEBUG interweave::connectivity: retry scheduled peer="<B>" origin="discovery_reconnect" attempt=5 delay_ms=300000 peer_backoff=true
-- A's dial and connect lines after B's return --
2026-11-14T14:00:17.896448Z DEBUG interweave::connectivity: peer connected peer="<B>" path="direct"
```

## Findings

- **A redials through the outage on the backoff's schedule.** Its dials
  failed at 0, 31, 91, 211 and 451 s after the close, each scheduling
  the next at 30, 60, 120 and 240 s and then the 300 s ceiling. The
  first failure is the discovery round's redial on the close; later
  ones are the discovery round's or the scheduler's, whichever reaches
  the due retry first (the `origin` field). Four reconnect refusals in
  480 s: the discovery round is refused by the gate while the backoff
  holds, and logs it.
- **B's return lifts nothing.** B holds no entry for A and nothing in
  A's network changed, so the backoff A was in stood: the pair
  reconnected at 14:00:17.896, the retry the 13:55:17.680 failure
  scheduled 300 s later (0.2 s after it fell due), 272 s after B was
  back. The first run of the same case, before its assertions were
  tightened to these figures, read 271 s, with the same schedule.
- **So after an outage past the fourth step, a returned far daemon can
  wait up to the 5 min ceiling** to be redialled by the side with the
  entry, while both ends are up. Whether that is acceptable is the
  product's question, not this record's.

What the case asserts: the five delays exactly; the reconnect within a
second of the last failure plus 300 s, read from A's log timestamps;
and the time after B's return above the harness's 30 s patience
(`e2e::PATIENCE`) and within 330 s, the ceiling plus it.

## Not measured

A far end that comes back on another address or port; an outage that
ends while A is inside an earlier, shorter step (the wait is then that
step's remainder, by the same mechanism); a far end that returns and
dials A itself (an inbound lifts A's backoff, unit-tested in the
runtime, not run here); a real network outage rather than a stopped
daemon (a link that returns is a network addition, which lifts a
held-off peer's backoff, as `two_hosts_talk_over_the_overlay_and_redial_when_its_link_returns`
shows).
