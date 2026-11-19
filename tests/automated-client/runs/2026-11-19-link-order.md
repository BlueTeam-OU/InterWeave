# The order a test link comes up in — 2026-11-19

`tests/support/src/e2e/netns.rs`'s `OVERLAY_UP`, the script that
makes the overlay's link at setup and again in `netns::overlay_up`,
measured by p2p-network-dev-02 on develop-qzapp with the case

- `the_side_that_learned_an_address_dials_it_on_a_send_and_retries`
  (`tests/automated-client/tests/two_hosts.rs`)

```sh
cargo build -p interweave-transportctl -p interweave-transport-daemon
cargo test -p interweave-automated-client-tests --test two_hosts -- \
  --exact <case> --test-threads 2
```

## Why it was measured

The case failed once under a workspace run at a9652379b, 2026-11-19
09:58 UTC (`two_hosts.rs:227` there, "A redialled once the link
returned within 30s"). A's log: once the link was made again, A's
redial by its entry and its discovery reconnect both failed within
30 ms of each other, the peer was held off 30 s, and the retry 30 s
later fell past the case's patience. The same tree passed alone.

The script on main 7b15b2cdc gave each end
`ip addr add … && ip link set … up`: A's address arrived while `w0`
was down, which the daemon reads as the network returning
(`network_added` lifts the backoff and the scheduler dials at once),
and a dial over a link that is not up has no route and fails at once.

## What ran

| tree | what was planted | runs | result |
|---|---|---|---|
| main 7b15b2cdc | `sleep 1` between A's `ip addr add 100.64.0.1/30 dev w0` and `ip link set w0 up` | 3 | 3 failed at "A redialled once the link returned within 30s", 37.5 s each; A's log as the field run's: two instant failures as the link returned, `reconnect refused … PeerBackoff`, the retry failing 30 s later |
| 1d7088d8b | `sleep 1` between B's address and A's (the far end ready a second before A's address arrives) | 3 | 3 passed, 10.7–11.1 s |
| 1d7088d8b | nothing | 3 | 3 passed, 7.8–8.0 s |
| 1d7088d8b | nothing; the whole `two_hosts` test file | 1 | 13 passed, 4 ignored (the long-outage and long-watch cases) |

The order at 1d7088d8b: both ends up, then B's address, then A's —
the side holding the entry, whose address arriving is the redial
these cases wait for. `second_link` follows the same order with B,
the side that redials there, last: its route goes in `onlink` before
its address.

## What it does not show

- No run under the full workspace load the field failure met; the
  planted `sleep 1` stands in for it. The order removes the window
  rather than narrowing it.
- `second_link`'s new order was run only through the `two_hosts` file
  as a whole, with nothing planted in it.
- No test pins the order. Swapping the steps back passes on an
  unloaded host, as the old order did; the first row above is the
  mutation to run against a later change to the script.
- B's early redial, when B holds an entry for A (the overlay's own
  case), now meets a link that is up and an address A has not taken
  yet: it waits on ARP, which A's address answers a process later or
  which gives up after about 3 s, a late failure the case's redial by
  A does not depend on. Not measured.
