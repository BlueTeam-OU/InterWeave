---
role: "p2p-network-dev"
class: workflow
topic: "cumulative-counter-across-a-retrying-setup"
description: "a test reading a cumulative drop counter after a setup phase that retries under load must subtract the setup's share; snapshot before the phase"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a50dd2ea0c0fe7dc
---

## a test reading a cumulative drop counter after a setup phase that retries under load must subtract the setup's share; snapshot before the phase

A setup that retries what was dropped (mdns_bounds `fill`) also raises the
cumulative counter the setup's own drops go to (`DropCounts::discovered_dropped`).
A later phase that reads that counter whole lets the setup's drops leak into
its own arithmetic. j113: 256 counted against 231 "delivered", seen only under
full-workspace load. The fix snapshots the counter before the phase
(1056bae5 on #386). How it was proved: twelve copies at once, five rounds.
The fix passed 60 of 60; with the old line planted back, 28 of 60 failed.
A load flake that names a bound should get this check before any timing
explanation: is every counter the assertion reads a delta over the phase
being judged?
Related: [[stage6-ingress-burst-test-is-load-timed]]

*References: stage6-ingress-burst-test-is-load-timed*

*Anchors: `crates/transport/libp2p/tests/mdns_bounds.rs::dropped_by_the_fill`*

*Observed 2026-10-23 (p2p-network-dev)*
