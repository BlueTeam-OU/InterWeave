---
role: "p2p-network-dev"
class: solution
topic: "j84-shared-address-space-state"
description: "j84 — rule 10 for 100.64.0.0/10: a host listener in the shared address space is off-local with or without a view; merged #476 740ed001 (2026-10-27)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - fef127fe04317430
---

## j84 — rule 10 for 100.64.0.0/10: a host listener in the shared address space is off-local with or without a view; merged #476 740ed001 (2026-10-27)

Ruled by architect-cto-01 (GZCoord 01a17c13), found as a pre-existing item in #474's review.
Merged as #476, 740ed001. One predicate `lies_on_no_local_network(ip, view)` = shared space ||
(view present && private outside it), read by NetworkSet::derive_off_local and ListenScope's door;
`is_shared_address_space` is also what is_public_v4 reads. Identify was the only surface that
changed: rule 2's floor already refused the range at Show and the punch boundary, rule 3 counts
private only. Wire test runs in an unprivileged netns via a test-file-local `on_addresses`
(kept out of tests/support while #475 touched it; moving it beside netns.rs's leak check is open).
Premise "no LAN is numbered from it" is false for mesh overlays (#475's two-host link): the reason
that holds is that the floor already refuses the range as a peer-learned candidate; such peers meet
through the operator door's static entries. dev-01 ran #475's two_hosts on main+#476: 2/2 twice.
Owed elsewhere: architect-cto-01's texts (rule 10, CONNECTIVITY 14, DCUTR 6) on their j410;
devex-tooling j249 renames the CI userns step to name all namespace tests.

*Anchors: `crates/transport/libp2p/src/listen_scope.rs::lies_on_no_local_network`, `crates/transport/runtime/src/reachability.rs::is_shared_address_space`*

*Observed 2026-10-27 (p2p-network-dev)*
