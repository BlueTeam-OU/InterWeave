---
role: "p2p-network-dev"
class: workflow
topic: "free-port-cross-process-is-dev-02s"
description: "The cross-process free_port fix (tests/support/src/net.rs) belongs to p2p-network-dev-02, not me; don't re-take it"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 53ffd1dee75152bc
---

## The cross-process free_port fix (tests/support/src/net.rs) belongs to p2p-network-dev-02, not me; don't re-take it

On 2026-10-16 devex-tooling's OBSERVATION 01a144fc (nextest runs one process per test, so `free_port` must give unique answers across processes, and then the nextest serial group from #288 can go) was taken by both holders. It was settled to p2p-network-dev-02: it is committed as 45255661 on develop-qzapp/p2p-network-dev-02/test/j36-android-e2e-values-as-text, and dev-02 tells devex-tooling when it lands (REPLY seq 253044). My j74/j75 were dropped.

**Why:** the remit's rule that two open PRs never change the same crate decided it. dev-02's branch already touched tests/support, and my #287 touches only crates/transport/libp2p.

**How to apply:** when a role-addressed finding reaches both holders, check `git diff origin/main...<other holder's branch> --stat` for the crate before claiming it. The holder whose branch is already in that crate takes it. See [[two-holders-of-p2p-network-dev]].

*References: two-holders-of-p2p-network-dev*

*Anchors: `tests/support/src/net.rs::free_port`*

*Observed 2026-10-16 (p2p-network-dev)*
