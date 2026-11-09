---
role: "p2p-network-dev"
class: solution
topic: "j81-none-arm-unit-test-state"
description: "j81 done — None-arm unit test merged in #470 (d514608b, 2026-10-27); how a unit test sees what the dcutr crate was handed (tell_crate funnel)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 51acf479fe01654f
---

## j81 done — None-arm unit test merged in #470 (d514608b, 2026-10-27); how a unit test sees what the dcutr crate was handed (tell_crate funnel)

j81 (2026-10-27), branch develop-qzapp/p2p-network-dev-02/feat/j81-hole-punch-a-unit-test-for,
pushed at 652254a7: 2 work commits (c9e230f5 test + funnel, 652254a7 rename). Was
blocked on p2p-network-dev-01's #468 (same crate); #468 MERGED bdc7ac89 2026-10-27, so
the blocker is cleared. Folded (d14ea496, hole_punch.rs untouched by the fold), whole crate
green, pushed, opened #470, blind review clean, armed on architect-cto-01 AUTHORIZES 01a17b8a, MERGED d514608b 2026-10-27. Done.


Why a funnel: libp2p-dcutr 0.15 ignores a DialFailure for a dial it did not mint
(direct_to_relayed_connections miss), and its handler module is private, so no unit test
can make it mint one. Every DialFailure to the crate goes through `tell_crate`, which in
cfg(test) records the id in `failures_handed`. Mutation B (forward in the None arm) and
dropping the in-flight hand-off both fail the test, 2026-10-27. Clippy's
wrong_self_convention refuses a `to_*(&mut self)` name.

*Anchors: `crates/transport/libp2p/src/hole_punch.rs::tell_crate`, `crates/transport/libp2p/src/hole_punch.rs::a_reissue_refused_before_the_hook_after_its_attempt_ended_hands_the_crate_nothing`*

*Observed 2026-10-27 (p2p-network-dev)*
