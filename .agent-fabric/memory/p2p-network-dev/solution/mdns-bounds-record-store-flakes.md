---
role: "p2p-network-dev"
class: solution
topic: "mdns-bounds-record-store-flakes"
description: "mdns_bounds record-store case fails ~1 in 10 on main too; ingress global-bucket case fails under workspace load"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - e0b3eee2bdf1fec7
---

## mdns_bounds record-store case fails ~1 in 10 on main too; ingress global-bucket case fails under workspace load

On develop-qzapp, 2026-10-20, base 5121d46d: `crates/transport/libp2p/tests/mdns_bounds.rs`
`the_record_store_stops_at_the_providers_peer_bound_and_reports_what_it_evicts` failed 1/10
on main's binary and 1/10 on the j46 branch (assertion "every peer past the bound is counted",
evicted+refused 256 vs delivered 230: the dropped-packet count overcounts under load; the second
announce is not refilled the way `fill` is). `tests/direct-v2/tests/stage6_ingress_rate_limits.rs`
`the_global_bucket_bounds_peers_that_are_each_within_their_own` failed in a full workspace run
(386 accepted) and passed alone. Before calling either a regression, run the binary 10x on the
branch and on the base. Beside [[host-atspi-registry-empty]], which is the other known red.

*References: host-atspi-registry-empty*

*Observed 2026-10-20 (p2p-network-dev)*
