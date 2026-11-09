---
role: "p2p-network-dev"
class: solution
topic: "mdns-bounds-record-store-flakes"
description: "mdns_bounds record-store red was a test bug fixed by #386 (1056bae5), not noise; ingress global-bucket case red only under workspace load (2026-10-20)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 306c6a7ab3fa3984
  - e0b3eee2bdf1fec7
---

## mdns_bounds record-store red was a test bug fixed by #386 (1056bae5), not noise; ingress global-bucket case red only under workspace load (2026-10-20)

`crates/transport/libp2p/tests/mdns_bounds.rs`
`the_record_store_stops_at_the_providers_peer_bound_and_reports_what_it_evicts` failed about 1 in
10 on main and on a branch in 2026-10 (evicted+refused 256 vs delivered 230). It was NOT host
noise: the eviction phase's expectation read the cumulative `discovered_dropped()` count, which
included the fill's queue drops. #386 fixed it (1056bae5, "count the eviction phase's queue drops,
not the fill's"; 60 of 60 green): the test now subtracts the drops counted after the fill. A red
of this case on a tree that has 1056bae5 is a real failure, to be diagnosed, not rerun.

Still only an observation, not re-checked since 2026-10-20 (base 5121d46d):
`tests/direct-v2/tests/stage6_ingress_rate_limits.rs`
`the_global_bucket_bounds_peers_that_are_each_within_their_own` failed once in a full workspace
run (386 accepted) and passed alone. Before calling it a regression, run that binary 10 times on
the branch and on the base. Beside [[host-atspi-registry-empty]], the other known red.

*References: host-atspi-registry-empty*

*Observed 2026-10-24 (p2p-network-dev)*
