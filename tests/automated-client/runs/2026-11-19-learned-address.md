# The side that learned an address, between two namespaces — 2026-11-19

`tests/automated-client/tests/two_hosts.rs`, three cases run by
p2p-network-dev-02 on develop-qzapp at commit f4560a4cf (on main
3f9af2fd7; the mutations below on 8f15e5e3d, whose two drop and route
cases are this commit's but for a removed assertion that could not
fail), for the four items `CONNECTIVITY.md` §14 (A 2026-11-13)
named as not proved on the side that learned an address:

- `a_send_to_a_learned_address_the_network_drops_is_answered_at_the_send_horizon`
- `a_send_to_a_learned_address_that_is_routed_reconnects_the_pair`
- `the_side_that_learned_an_address_dials_nothing_through_a_long_watch_and_keeps_it`,
  `#[ignore]`d: two watches of 360 s.

```sh
cargo build -p interweave-transportctl -p interweave-transport-daemon
cargo test -p interweave-automated-client-tests --test two_hosts -- \
  --exact <case> --test-threads 1          # --ignored for the third
```

## What ran

The fleet's shape between two namespaces joined by the overlay's link:
only A holds a static-bootstrap entry for B, and each host holds a
private address on its loopback (A `10.99.0.1/24`, B `10.88.0.2/24`).
A's Identify advertises its own, and B, holding a private listener of
the family, puts it in its book (ADR-0052 rule 3); the overlay's
address is never learned. Connected, then the overlay's link deleted
and both ends closed on the address leaving, then B's 5 s watch read.

In the first two cases, a second link is then made between the
namespaces (`10.77.0.0/30`, `netns::second_link`) with B routing
`10.99.0.0/24` over it to A: its addresses did not exist while A's
Identify ran, so B's book still holds `10.99.0.1` alone for A, and A,
whose book holds its entry on the lost overlay and B's unrouted
`10.88.0.2`, has no route to B. In the first case B's own nft output
rule drops every packet to `10.99.0.1`, counted (`netns::drop_outbound`),
put in before the link. B's client then sends to A's.

In the third, B sends nothing: the 5 s watches of
`the_side_that_learned_an_address_dials_nothing_until_it_sends` are
each 360 s, past the retry ceiling (30 s), the Swarm's idle timeout
(60 s) and Identify's interval (5 min, the crate's default); then A's
private address is removed from A, and waited for until A's own
addresses (`admin.peers.own_addresses`) no longer list it, having
listed it, so the reconnect's Identify cannot advertise it to B again;
then the link returns, A redials by its entry, the second watch, the
link lost again, and the probe: B's send, answered `PeerUnreachable` while B's
book holds an address for A and `PeerUnknown` when it holds none.

## Output

Each case writes its figures beside the test binary, since the inner
run's output is not shown on a pass; as printed:

```text
the_side_that_learned_an_address_dials_nothing_through_a_long_watch_and_keeps_it ... ok (722.67s)
watch 360s each; dials [0, 0]; retry scheduled false; probe Err(PeerUnreachable) 722.18s after the first connection

a_send_to_a_learned_address_the_network_drops_is_answered_at_the_send_horizon ... ok (16.60s)
answer Err(Timeout) after 10.009s; drops Some(7); landed false

a_send_to_a_learned_address_that_is_routed_reconnects_the_pair ... ok (6.58s)
answer Ok(EndpointId("automation")) after 29.530ms; drops None; landed true
```

Earlier runs of the drop and route cases, on c24e65adf (the same cases
but for `netns::outbound_drops` reading only a rule ending in `drop`)
and on 8f15e5e3d, printed `answer
Err(PeerUnreachable) after 10.004s; drops Some(7)` and `answer
Err(Timeout) after 10.002s; drops Some(7)` for the first, `Ok` after
172.904 ms and 59.329 ms for the second. The long case ran on both
too, without the withdrawal, and printed the same as above (722.11 s,
722.02 s); those runs could not tell the first connection's entry from
one the reconnect's Identify learned again.

## Findings, per item

- **(1) A learned address the network drops silently.** B's send
  dialled it and was answered at the 10 s horizon: `Timeout` in two
  runs, `PeerUnreachable` in one, 10.002 s to 10.009 s after the
  send. Three timers are 10 s and start within milliseconds of
  each other: the IPC request's deadline (`Timeout`), the held send's
  horizon (`SEND_DIAL_HORIZON_MS`, `PeerUnreachable`) and the dial's
  handshake timeout (a failed dial, `PeerUnreachable`), so which answers
  is a race and either is the rule; neither came sooner. The rule
  dropped 7 packets to the learned address in each run: the dial
  reached it. The 7 is not broken down per dial or per socket; one
  connect's SYN and its retransmissions within 10 s would be four.
  The case asserts the answer is one of the two, at no less than 10 s
  and under 12 s, and the count above zero.
- **(2) A learned address that is reachable.** B's send dialled it, the
  dial connected, and the message was accepted by A's endpoint
  30–173 ms after the send and landed at A's client from B: the pair
  reconnected by the send, over the one route B's book held.
- **(3) How long a learned entry lives.** Not a constant: the address
  book has no age at which an entry leaves. In
  `crates/transport/runtime/src/connection_manager.rs` an entry leaves
  by an operator's removal (`forget_address`), a permanent dial failure
  (`record_permanent_failure`, `hand_over`), eviction when the peer's
  book is full (`learn_address`, `DEFAULT_MAX_ADDRESSES_PER_PEER` = 8,
  failed and quarantined entries first), its peer's retirement from
  trust (`MAX_RETIRED_BOOK_PEERS`), or the daemon's restart, the book
  being held in memory. The prune past `DEFAULT_IDLE_TTL_MS` (1 h)
  spares a book entry's state, pinned by
  `a_book_entrys_state_is_never_pruned_apart_from_it`; that the entry
  itself has no age limit has no test of its own. Measured: B's book
  still held what it learned on the first connection 722 s later,
  through a close, two 360 s watches, a reconnect whose Identify no
  longer advertised it, and a second close.
- **(4) A watch longer than 5 s.** B dialled nothing in 360 s after the
  close and nothing in 360 s after the link returned, and scheduled no
  retry: the 5 s watch of the existing case was not too short to see
  a timer-driven dial up to six minutes.

## Mutations

Each new case fails under its named mutation, run on 8f15e5e3d; the
three existing cases beside them, the controls, pass unchanged on
f4560a4cf, run with the cases above.

- **The drop turned into an accept** (`netns::drop_outbound`'s rule
  `counter drop` to `counter accept`): the drop case failed at its
  answer assertion, `Ok` after 43 ms, 35 packets matched.
- **The route removed** (`netns::second_link` without `ip route add`):
  the routed case failed at its answer assertion, `PeerUnreachable`
  after 5.8 ms, the unrouted address's answer.

Both were applied to the committed tree, each patch matched once, and
were reverted.

## Not shown, by name

One kernel: the drop is a filter in B's own namespace, not a middlebox
or a NAT's mapping on a real path, and there is no real latency or
loss. The send's dial horizon reads as reached by three equal timers;
which of them answers was not controlled. Twelve minutes is not the
hours a fleet host holds a learned entry; the code gives the entry no
age limit, so a longer watch would read the same code. A restart
forgetting what was learned is read from the code, not run.
