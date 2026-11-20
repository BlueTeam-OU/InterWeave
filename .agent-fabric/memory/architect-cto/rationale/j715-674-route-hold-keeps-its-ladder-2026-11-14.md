---
role: "architect-cto"
class: rationale
topic: "j715-674-route-hold-keeps-its-ladder-2026-11-14"
description: "ADR-0011 relay-hop route hold keeps 30 s doubling to 5 min after the peer retry went flat at 30 s; supply f3a45a34d for #674"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 58aa29ee25c11861
---

## ADR-0011 relay-hop route hold keeps 30 s doubling to 5 min after the peer retry went flat at 30 s; supply f3a45a34d for #674

Ruled 2026-11-14 for p2p-network-dev-01 (its REPLY 01a1db63-e6fb, my REPLY
01a1db67-f96f): the relay-hop route hold (ADR-0011 A 2026-10-21) keeps its
own ladder, 30 s doubling to 5 min on the peer's attempt count
(`route_hold_ms`, InterWeave #674 commit 8d6c5450a), and is not flattened
with the peer's retry (ADR-0011 A 2026-11-14, the owner's "30 seconds
maximum").

**Why:** the owner's bound is on the peer's retry. At each 30 s retry a held
circuit is refused and the peer's other routes are tried; only a peer whose
single route is a held circuit waits to the hold's end or until its relay
connects (`relay_reached`). A relay that stays down gains nothing from
being asked every 30 s.

**How to apply:** supply f3a45a34d on
`develop-qzapp/architect-cto-01/for/p2p-network-dev-01/j169-redial-30s`
(on top of 2ff6c8a6c, [[j714-redial-30s]]) carries the ADR-0011 body, the
history note, the digest and CONNECTIVITY.md section 12. Closes when
f3a45a34d is on main via #674. Owed after that: my arming word on #674
when p2p-network-dev-01 asks with the folded, reviewed head.

*References: j714-redial-30s*

*Observed 2026-11-14 (architect-cto)*
