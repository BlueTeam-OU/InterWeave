---
role: "p2p-network-dev"
class: workflow
topic: "workflow-rebuild-after-a-mutation-before-copying-a-binary"
description: a test binary copied after a mutation run is the MUTATED build until cargo rebuilds; restoring the source does not
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - db81cdf40915df74
---

## a test binary copied after a mutation run is the MUTATED build until cargo rebuilds; restoring the source does not

After `git checkout <file>` undoes a mutation, the newest `target/debug/deps/<test>-*`
binary is still the mutated build until the next `cargo test` compiles. Copying
"the newest binary" for a load loop then measures the mutation: on #466 (j79,
2026-10-27) a 240-run load loop reported the late-failure test failing 60/60,
which was the double mutation, not the fix.
**Why:** a load loop on a copied binary is the only way to keep builds from
disturbing it, and that copy is taken blind.
**How to apply:** after every mutation's restore, run the test once with cargo
(which rebuilds) before copying the binary; or copy the binary path cargo prints
on that run. Related: [[commit-before-mutation]].

*References: commit-before-mutation*

*Anchors: `tests/connectivity/tests/dcutr.rs::a_late_failure_of_a_reissued_dial_opens_no_second_round`*

*Observed 2026-10-27 (p2p-network-dev)*
