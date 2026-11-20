---
role: "p2p-network-dev"
class: solution
topic: "j168-hold-not-proved-items-landed"
description: "j168 landed (#672, 808488b4, 2026-11-14): crossing real dials read from ping traffic; long outage reconnects at the backoff's retry, up to 5 min after return"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d52d0ae454d0f692
---

## j168 landed (#672, 808488b4, 2026-11-14): crossing real dials read from ping traffic; long outage reconnects at the backoff's retry, up to 5 min after return

j168 (architect-cto REQUEST 01a1da16, two items of CONNECTIVITY.md §14's hold "Not proved" list). Merged as #672, 808488b4, on 2026-11-14. It was armed below the minimum on architect-cto's AUTHORIZES (REPLY 01a1da83) via `fabric-pr arm --authorization <id>`. Architect-cto writes the §14 wording themselves (their j708).

- **Crossing real dials.** `two_runtimes_dialling_each_other_at_once_both_hold_the_lower_peers_dial`.
  - **Why idle close can't prove it:** between two runtimes, gossipsub parks a stream on each end's FIRST connection, so both crossing connections stay open whatever the hold chose.
  - **How it is read instead:** only held connections are pinged (`relay_keepalive`). The test reads `ss -tni` per 4-tuple across the 15 s ping interval:
    - the lower peer's dial carries bytes and the higher peer's carries none;
    - `data_segs_out` per end: at least 4 per end with both ends pinging, against 3 with one end pinging (planted mutant).
  - **Each pinger** adds a 62-byte frame each way: 2 B noise length, 12 B yamux header, 32 B ping, 16 B tag.
  - **Untraced:** 2 baseline segments per end. The margin is one segment. If CI flakes, look there first.
  - **Mutations that fail it:** each end holds its own dial; both hold the higher's dial; only the lower end holds.
- **Long outage.** `after_a_long_outage_the_side_with_the_entry_reconnects_within_its_backoff`, `#[ignore]`d, about 12.5 min. The record is `tests/automated-client/runs/2026-11-14-long-outage.md`.
  - Far daemon stopped for 480 s with the link up. The side holding the entry dialled at 0, 31, 91, 211 and 451 s (30/60/120/240/300).
  - The pair reconnected when the 300 s retry fell due, 272 s after the far daemon returned. The far daemon's return lifts nothing: it holds no entry and the side with the entry sees no network change.
  - Architect-cto took the up-to-5-min wait to the owner as a product question.
- **Harness.** The netns inner run is killed at 600 s by default. `in_overlay_within(test, deadline)` gives one case more.
- **Carried, mine:** a short run record of the segment counts, owed the next time crossing_dials.rs is touched.

Related: [[j165-data-plane-hold-handover]], [[hold-choice-must-agree-at-both-ends]], [[gossipsub-extensions-stream-keeps-connections-alive]].

*References: gossipsub-extensions-stream-keeps-connections-alive, hold-choice-must-agree-at-both-ends, j165-data-plane-hold-handover*

*Anchors: `crates/transport/libp2p/tests/crossing_dials.rs`, `tests/automated-client/tests/two_hosts.rs::after_a_long_outage_the_side_with_the_entry_reconnects_within_its_backoff`, `tests/support/src/e2e/netns.rs::in_overlay_within`*

*Observed 2026-11-14 (p2p-network-dev)*
